use std::sync::Arc;
use glyphon::{
    Buffer, Cache, Color, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextBounds,
    TextRenderer, Viewport,
};
use wgpu::util::DeviceExt;
use winit::window::Window;
use crate::config::{AppConfig, ThemePalette};

pub const TAB_BAR_HEIGHT: f32 = 30.0;
pub const TAB_CHAR_WIDTH: f32 = 7.6172;

const RECT_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

struct Uniforms {
    screen_size: vec2<f32>,
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let x = (model.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let y = 1.0 - (model.position.y / uniforms.screen_size.y) * 2.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

impl RectVertex {
    pub fn new(x: f32, y: f32, color: [f32; 4]) -> Self {
        Self {
            position: [x, y],
            color,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub screen_size: [f32; 2],
    pub _padding: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct TabVisualInfo {
    pub x: f32,
    pub width: f32,
    pub is_active: bool,
    pub is_hovered: bool,
}

#[derive(Clone, Debug)]
pub struct TabGeometry {
    pub index: usize,
    pub x: f32,
    pub width: f32,
    pub close_x: f32,
    pub close_w: f32,
}

pub fn calculate_tab_geometries(tab_titles: &[&str], start_x: f32) -> Vec<TabGeometry> {
    let mut geoms = Vec::with_capacity(tab_titles.len());
    let mut cur_x = start_x;
    for (i, title) in tab_titles.iter().enumerate() {
        let text = format!("   {}: {}  ×   ", i + 1, title);
        let char_count = text.chars().count();
        let width = char_count as f32 * TAB_CHAR_WIDTH;
        let close_x = cur_x + (char_count as f32 - 4.5) * TAB_CHAR_WIDTH;
        let close_w = 3.5 * TAB_CHAR_WIDTH;
        geoms.push(TabGeometry {
            index: i,
            x: cur_x,
            width,
            close_x,
            close_w,
        });
        cur_x += width;
    }
    geoms
}

pub fn push_rect(vertices: &mut Vec<RectVertex>, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
    let p0 = RectVertex::new(x, y, color);
    let p1 = RectVertex::new(x + w, y, color);
    let p2 = RectVertex::new(x, y + h, color);
    let p3 = RectVertex::new(x + w, y + h, color);

    vertices.push(p0);
    vertices.push(p1);
    vertices.push(p2);
    vertices.push(p1);
    vertices.push(p3);
    vertices.push(p2);
}

pub struct GpuRenderer {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: Viewport,
    pub atlas: TextAtlas,
    pub text_renderer: TextRenderer,

    // Fast 2D UI Rectangles pipeline (Tab Bar background, active blue line, [+] button)
    pub rect_pipeline: wgpu::RenderPipeline,
    pub rect_bind_group: wgpu::BindGroup,
    pub rect_uniform_buf: wgpu::Buffer,
    pub rect_vertex_buf: wgpu::Buffer,
    pub rect_vertex_capacity: usize,
}

impl GpuRenderer {
    pub async fn new(window: Arc<Window>, width: u32, height: u32) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("DROP GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let alpha_mode = caps.alpha_modes[0];

        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            tracing::info!("Using PresentMode::Mailbox (Ultra-low latency, 0 tearing)");
            wgpu::PresentMode::Mailbox
        } else if caps.present_modes.contains(&wgpu::PresentMode::Immediate) {
            tracing::info!("Using PresentMode::Immediate (Immediate presentation)");
            wgpu::PresentMode::Immediate
        } else {
            tracing::info!("Using PresentMode::Fifo (V-Sync)");
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, wgpu::MultisampleState::default(), None);

        // Rectangles pipeline initialization
        let uniform_data = Uniforms {
            screen_size: [width as f32, height as f32],
            _padding: [0.0, 0.0],
        };
        let rect_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("DROP Rect Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let rect_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("DROP Rect Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let rect_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("DROP Rect Bind Group"),
            layout: &rect_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: rect_uniform_buf.as_entire_binding(),
            }],
        });

        let rect_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("DROP Rect Pipeline Layout"),
                bind_group_layouts: &[&rect_bind_group_layout],
                push_constant_ranges: &[],
            });

        let rect_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DROP Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
        });

        let rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("DROP Rect Pipeline"),
            layout: Some(&rect_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &rect_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 8,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &rect_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        let rect_vertex_capacity = 8192;
        let rect_vertex_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DROP Rect Vertex Buffer"),
            size: (rect_vertex_capacity * std::mem::size_of::<RectVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            rect_pipeline,
            rect_bind_group,
            rect_uniform_buf,
            rect_vertex_buf,
            rect_vertex_capacity,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            let uniform_data = Uniforms {
                screen_size: [width as f32, height as f32],
                _padding: [0.0, 0.0],
            };
            self.queue.write_buffer(
                &self.rect_uniform_buf,
                0,
                bytemuck::cast_slice(&[uniform_data]),
            );
        }
    }

    pub fn measure_cell_metrics(&mut self, font_family: &str, font_size: f32) -> (f32, f32) {
        let resolved = resolve_monospace_font(&self.font_system, font_family);
        let cell_h = (font_size * 1.4667).round().max(16.0);
        let mut buffer = Buffer::new(&mut self.font_system, glyphon::Metrics::new(font_size, cell_h));
        buffer.set_size(&mut self.font_system, Some(1000.0), Some(cell_h + 10.0));
        buffer.set_text(
            &mut self.font_system,
            "MMMMMMMMMM", // 10 characters
            glyphon::Attrs::new().family(glyphon::Family::Name(&resolved)),
            glyphon::Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut advance_10 = 0.0;
        if let Some(line) = buffer.lines.first() {
            if let Some(layout) = line.layout_opt() {
                for run in layout {
                    for glyph in &run.glyphs {
                        advance_10 += glyph.w;
                    }
                }
            }
        }
        let cell_w = if advance_10 > 0.0 {
            advance_10 / 10.0
        } else {
            font_size * 0.58594
        };
        (cell_w, cell_h)
    }

    pub fn create_buffer(&mut self, font_size: f32, _padding_x: f32, padding_y: f32) -> Buffer {
        let cell_h = (font_size * 1.4667).round();
        let mut buffer = Buffer::new(&mut self.font_system, glyphon::Metrics::new(font_size, cell_h));
        let term_h = (self.config.height as f32 - TAB_BAR_HEIGHT - padding_y * 2.0).max(100.0);
        buffer.set_size(
            &mut self.font_system,
            None, // Unlimited width: prevents cosmic-text from soft-wrapping terminal lines
            Some(term_h),
        );
        buffer
    }

    pub fn create_tab_bar_buffer(&mut self) -> Buffer {
        let mut buffer = Buffer::new(&mut self.font_system, glyphon::Metrics::new(13.0, 18.0));
        buffer.set_size(
            &mut self.font_system,
            Some(self.config.width as f32),
            Some(TAB_BAR_HEIGHT),
        );
        buffer
    }

    pub fn create_cursor_buffer(
        &mut self,
        font_family: &str,
        font_size: f32,
        cursor_color: [u8; 3],
    ) -> Buffer {
        let resolved = resolve_monospace_font(&self.font_system, font_family);
        let cell_h = (font_size * 1.4667).round();
        let mut buffer = Buffer::new(&mut self.font_system, glyphon::Metrics::new(font_size, cell_h));
        buffer.set_size(&mut self.font_system, Some(30.0), Some(cell_h + 4.0));
        buffer.set_text(
            &mut self.font_system,
            "|",
            glyphon::Attrs::new()
                .family(glyphon::Family::Name(&resolved))
                .color(Color::rgb(cursor_color[0], cursor_color[1], cursor_color[2])),
            glyphon::Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        buffer
    }

    pub fn render_terminal(
        &mut self,
        config: &AppConfig,
        palette: &ThemePalette,
        buffer: &Buffer,
        tab_bar: Option<&Buffer>,
        tabs: &[TabVisualInfo],
        plus_btn_hovered: bool,
        settings_btn_hovered: bool,
        cursor: Option<(f32, f32, f32, f32)>,
        scroll_info: Option<crate::terminal::ScrollInfo>,
        cell_quads: &[crate::terminal::TerminalCellQuad],
        cell_w: f32,
        cell_h: f32,
    ) -> anyhow::Result<()> {
        let width = self.config.width as f32;
        let height = self.config.height as f32;
        let tab_bar_top = height - TAB_BAR_HEIGHT;
        let term_bottom = tab_bar_top as i32;

        let mut rect_vertices: Vec<RectVertex> = Vec::with_capacity(512);

        // 1. Cell backgrounds and text decorations (underlines, strikethroughs)
        // Rendered first and strictly clipped to tab_bar_top so terminal cells never occlude the tab bar
        for q in cell_quads {
            let qx = config.padding_x + (q.col as f32) * cell_w;
            let qy = config.padding_y + (q.row as f32) * cell_h;
            if qy >= tab_bar_top {
                continue;
            }
            let qw = (q.cols_count as f32) * cell_w;
            let qh = cell_h.min(tab_bar_top - qy);
            if qh <= 0.0 {
                continue;
            }

            if q.color[3] > 0.0 {
                push_rect(&mut rect_vertices, qx, qy, qw, qh, q.color);
            }
            if q.underline {
                let uy = qy + qh - 2.0;
                if uy + 1.5 <= tab_bar_top {
                    push_rect(&mut rect_vertices, qx, uy, qw, 1.5, q.fg_color);
                }
            }
            if q.strikethrough {
                let sy = qy + (qh * 0.5).round();
                if sy + 1.5 <= tab_bar_top {
                    push_rect(&mut rect_vertices, qx, sy, qw, 1.5, q.fg_color);
                }
            }
        }

        // 2. Bottom Tab Bar full-width background panel
        push_rect(
            &mut rect_vertices,
            0.0,
            tab_bar_top,
            width,
            TAB_BAR_HEIGHT,
            palette.tab_bar_bg,
        );

        // 3. Top separator line spanning full width
        push_rect(
            &mut rect_vertices,
            0.0,
            tab_bar_top,
            width,
            1.0,
            palette.border,
        );

        // 4. Tab cards
        for tab in tabs {
            if tab.is_active {
                // Active tab card background
                push_rect(
                    &mut rect_vertices,
                    tab.x,
                    tab_bar_top + 1.0,
                    tab.width,
                    TAB_BAR_HEIGHT - 1.0,
                    palette.tab_active_bg,
                );
                // Active top accent stripe (2.5px height)
                push_rect(
                    &mut rect_vertices,
                    tab.x,
                    tab_bar_top,
                    tab.width,
                    2.5,
                    palette.accent,
                );
                // Right separator
                push_rect(
                    &mut rect_vertices,
                    tab.x + tab.width - 1.0,
                    tab_bar_top + 3.0,
                    1.0,
                    TAB_BAR_HEIGHT - 6.0,
                    palette.border,
                );
            } else {
                let bg_color = if tab.is_hovered {
                    [
                        palette.tab_bar_bg[0] + 0.03,
                        palette.tab_bar_bg[1] + 0.035,
                        palette.tab_bar_bg[2] + 0.045,
                        1.0,
                    ]
                } else {
                    palette.tab_bar_bg
                };
                push_rect(
                    &mut rect_vertices,
                    tab.x,
                    tab_bar_top + 1.0,
                    tab.width,
                    TAB_BAR_HEIGHT - 1.0,
                    bg_color,
                );
                // Right separator
                push_rect(
                    &mut rect_vertices,
                    tab.x + tab.width - 1.0,
                    tab_bar_top + 4.0,
                    1.0,
                    TAB_BAR_HEIGHT - 8.0,
                    [palette.border[0], palette.border[1], palette.border[2], 0.6],
                );
            }
        }

        // 5. Guake-style Tab Bar buttons:
        // [+] New Tab button anchored to far right
        let btn_w = 26.0;
        let btn_h = 22.0;
        let plus_x = (width - btn_w - 8.0).max(0.0);
        let plus_y = tab_bar_top + 4.0;

        let (plus_bg, plus_border, plus_color) = if plus_btn_hovered {
            (
                [0.22, 0.27, 0.35, 1.0], // Hover background
                [0.38, 0.45, 0.58, 1.0], // Hover border
                [1.0, 1.0, 1.0, 1.0],    // White plus
            )
        } else {
            (
                [0.14, 0.16, 0.21, 1.0], // Default background
                [0.25, 0.29, 0.37, 1.0], // Default border
                palette.accent,          // Theme accent plus
            )
        };

        push_rect(&mut rect_vertices, plus_x, plus_y, btn_w, btn_h, plus_bg);
        push_rect(&mut rect_vertices, plus_x, plus_y, btn_w, 1.0, plus_border);
        push_rect(&mut rect_vertices, plus_x, plus_y + btn_h - 1.0, btn_w, 1.0, plus_border);
        push_rect(&mut rect_vertices, plus_x, plus_y, 1.0, btn_h, plus_border);
        push_rect(&mut rect_vertices, plus_x + btn_w - 1.0, plus_y, 1.0, btn_h, plus_border);

        // Centered '+' cross icon
        let cx = plus_x + btn_w / 2.0;
        let cy = plus_y + btn_h / 2.0;
        push_rect(&mut rect_vertices, cx - 5.0, cy - 1.0, 10.0, 2.0, plus_color);
        push_rect(&mut rect_vertices, cx - 1.0, cy - 5.0, 2.0, 10.0, plus_color);

        // [⚙] Settings button placed right next to [+]
        let set_x = (plus_x - btn_w - 6.0).max(0.0);
        let set_y = tab_bar_top + 4.0;

        let (set_bg, set_border, set_color) = if settings_btn_hovered {
            (
                [0.22, 0.27, 0.35, 1.0],
                [0.38, 0.45, 0.58, 1.0],
                [1.0, 1.0, 1.0, 1.0],
            )
        } else {
            (
                [0.14, 0.16, 0.21, 1.0],
                [0.25, 0.29, 0.37, 1.0],
                [0.65, 0.72, 0.82, 1.0],
            )
        };

        push_rect(&mut rect_vertices, set_x, set_y, btn_w, btn_h, set_bg);
        push_rect(&mut rect_vertices, set_x, set_y, btn_w, 1.0, set_border);
        push_rect(&mut rect_vertices, set_x, set_y + btn_h - 1.0, btn_w, 1.0, set_border);
        push_rect(&mut rect_vertices, set_x, set_y, 1.0, btn_h, set_border);
        push_rect(&mut rect_vertices, set_x + btn_w - 1.0, set_y, 1.0, btn_h, set_border);

        // Crisply centered settings sliders icon
        let scx = set_x + btn_w / 2.0;
        let scy = set_y + btn_h / 2.0;
        push_rect(&mut rect_vertices, scx - 5.0, scy - 3.5, 10.0, 1.5, set_color);
        push_rect(&mut rect_vertices, scx - 3.0, scy - 5.0, 3.0, 4.5, set_color);
        push_rect(&mut rect_vertices, scx - 5.0, scy + 2.5, 10.0, 1.5, set_color);
        push_rect(&mut rect_vertices, scx + 0.5, scy + 1.0, 3.0, 4.5, set_color);

        let text_area = TextArea {
            buffer,
            left: config.padding_x,
            top: config.padding_y,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: self.config.width as i32,
                bottom: term_bottom,
            },
            default_color: Color::rgb(palette.fg_color[0], palette.fg_color[1], palette.fg_color[2]),
            custom_glyphs: &[],
        };

        // Crisp cursor quad drawn with 2D rect pipeline (Bar or Block)
        if let Some((cx, cy, cw, ch)) = cursor {
            let cursor_color = [
                (palette.cursor_color[0] as f32) / 255.0,
                (palette.cursor_color[1] as f32) / 255.0,
                (palette.cursor_color[2] as f32) / 255.0,
                1.0,
            ];
            if config.cursor_style == "block" {
                push_rect(
                    &mut rect_vertices,
                    cx,
                    cy,
                    cw,
                    ch,
                    [cursor_color[0], cursor_color[1], cursor_color[2], 0.75],
                );
            } else {
                push_rect(&mut rect_vertices, cx, cy, 2.5, ch, cursor_color);
            }
        }

        // Sleek scrollbar indicator when scrolled or scrollback history exists
        if let Some(scroll) = scroll_info {
            if scroll.total_lines > scroll.screen_lines {
                let term_top = config.padding_y;
                let track_height = (tab_bar_top - term_top).max(20.0);
                let total = scroll.total_lines as f32;
                let visible = scroll.screen_lines as f32;
                let thumb_h = ((visible / total) * track_height).clamp(16.0, track_height);

                let max_offset = (scroll.total_lines.saturating_sub(scroll.screen_lines)) as f32;
                let scroll_ratio = if max_offset > 0.0 {
                    (scroll.display_offset as f32 / max_offset).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let thumb_y = term_top + (track_height - thumb_h) * (1.0 - scroll_ratio);
                let scrollbar_x = (width - 5.0).max(0.0);

                // Show stronger opacity when scrolled up into history, subtle hint when at bottom
                let alpha = if scroll.display_offset > 0 { 0.45 } else { 0.15 };
                push_rect(&mut rect_vertices, scrollbar_x, thumb_y, 3.0, thumb_h, [1.0, 1.0, 1.0, alpha]);
            }
        }

        let mut areas = Vec::with_capacity(3);
        areas.push(text_area);

        if let Some(tab_bar_buf) = tab_bar {
            areas.push(TextArea {
                buffer: tab_bar_buf,
                left: 8.0,
                top: tab_bar_top + 6.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: tab_bar_top as i32,
                    right: (set_x - 6.0) as i32, // Never overlap settings or [+] buttons
                    bottom: self.config.height as i32,
                },
                default_color: Color::rgb(148, 163, 184),
                custom_glyphs: &[],
            });
        }

        self.render_frame(&areas, &rect_vertices, palette.bg)
    }

    pub fn render_frame(
        &mut self,
        text_areas: &[TextArea],
        rect_vertices: &[RectVertex],
        clear_color: [f32; 4],
    ) -> anyhow::Result<()> {
        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture()?
            }
            Err(e) => return Err(e.into()),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.viewport.update(
            &self.queue,
            Resolution {
                width: self.config.width,
                height: self.config.height,
            },
        );

        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas.iter().cloned(),
            &mut self.swash_cache,
        )?;

        // Update vertex buffer for 2D UI quads
        if !rect_vertices.is_empty() {
            if rect_vertices.len() > self.rect_vertex_capacity {
                self.rect_vertex_capacity = (rect_vertices.len() * 2).next_power_of_two().max(8192);
                self.rect_vertex_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("DROP Rect Vertex Buffer Expanded"),
                    size: (self.rect_vertex_capacity * std::mem::size_of::<RectVertex>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            self.queue.write_buffer(
                &self.rect_vertex_buf,
                0,
                bytemuck::cast_slice(rect_vertices),
            );
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("DROP Render Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("DROP Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // 1. Draw 2D Quads (Tab Bar background, active Cyan top stripe, [+] button, backdrop)
            if !rect_vertices.is_empty() {
                pass.set_pipeline(&self.rect_pipeline);
                pass.set_bind_group(0, &self.rect_bind_group, &[]);
                pass.set_vertex_buffer(0, self.rect_vertex_buf.slice(..));
                pass.draw(0..rect_vertices.len() as u32, 0..1);
            }

            // 2. Draw Text (Terminal text, cursor, tab text)
            self.text_renderer
                .render(&self.atlas, &self.viewport, &mut pass)?;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub fn resolve_monospace_font(font_system: &glyphon::FontSystem, preferred: &str) -> String {
    let query = glyphon::fontdb::Query {
        families: &[
            glyphon::fontdb::Family::Name(preferred),
            glyphon::fontdb::Family::Name("Cascadia Code"),
            glyphon::fontdb::Family::Name("Cascadia Mono"),
            glyphon::fontdb::Family::Name("Consolas"),
            glyphon::fontdb::Family::Monospace,
        ],
        ..Default::default()
    };
    if let Some(id) = font_system.db().query(&query) {
        if let Some(face) = font_system.db().face(id) {
            if let Some((family_name, _)) = face.families.first() {
                return family_name.clone();
            }
        }
    }
    "Consolas".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_tab_geometries() {
        let titles = ["pwsh", "cmd", "wsl"];
        let geoms = calculate_tab_geometries(&titles, 10.0);

        assert_eq!(geoms.len(), 3);
        assert_eq!(geoms[0].index, 0);
        assert_eq!(geoms[0].x, 10.0);
        assert!(geoms[0].width > 0.0);
        assert!(geoms[0].close_x >= geoms[0].x);
        assert!(geoms[0].close_x + geoms[0].close_w <= geoms[0].x + geoms[0].width);

        // Next tab starts where previous tab ends
        assert_eq!(geoms[1].index, 1);
        assert_eq!(geoms[1].x, geoms[0].x + geoms[0].width);

        assert_eq!(geoms[2].index, 2);
        assert_eq!(geoms[2].x, geoms[1].x + geoms[1].width);
    }

    #[test]
    fn test_push_rect_vertex_generation() {
        let mut vertices = Vec::new();
        push_rect(&mut vertices, 50.0, 100.0, 200.0, 40.0, [1.0, 0.5, 0.2, 0.8]);

        // Each rect pushes 6 vertices (2 triangles)
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [50.0, 100.0]);
        assert_eq!(vertices[0].color, [1.0, 0.5, 0.2, 0.8]);
        assert_eq!(vertices[1].position, [250.0, 100.0]);
        assert_eq!(vertices[2].position, [50.0, 140.0]);
    }

    #[test]
    fn test_rect_vertex_capacity_math() {
        let count: usize = 2106;
        let capacity = (count * 2).next_power_of_two().max(8192);
        assert!(capacity >= count);
        let buffer_size = (capacity * std::mem::size_of::<RectVertex>()) as u64;
        let data_size = (count * std::mem::size_of::<RectVertex>()) as u64;
        assert!(buffer_size >= data_size);
    }
}
