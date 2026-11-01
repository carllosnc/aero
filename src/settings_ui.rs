use std::sync::Arc;
use windows::Win32::Foundation::HWND;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window as WinitWindow;

use crate::config::{AppConfig, THEMES};
use crate::dwm::apply_dwm_effects;
use egui::{
    vec2, Align, Color32, Frame, Layout, Margin, RichText, Rounding, ScrollArea,
    Sense, Shadow, Slider, Stroke, Ui,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Typography,
    Layout,
}

pub struct SettingsUi {
    pub selected_tab: SettingsTab,
}

impl SettingsUi {
    pub fn new() -> Self {
        Self {
            selected_tab: SettingsTab::Appearance,
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        config: &mut AppConfig,
        is_open: &mut bool,
    ) -> bool {
        let mut config_changed = false;
        let mut should_close = false;

        // Modern Fluent Dark Styling for the Window Body
        let frame = Frame {
            inner_margin: Margin::same(20.0_f32),
            outer_margin: Margin::ZERO,
            rounding: Rounding::ZERO,
            shadow: Shadow::NONE,
            fill: Color32::from_rgb(22, 25, 34), // Elevated dark obsidian
            stroke: Stroke::NONE,
        };

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 12.0);

            // 1. Header Bar with Title & Subtitle
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading(
                        RichText::new("Preferências do Terminal")
                            .size(20.0)
                            .color(Color32::from_rgb(240, 244, 250))
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Personalize o tema visual, tipografia e espaçamento.")
                            .size(12.5)
                            .color(Color32::from_rgb(130, 142, 165)),
                    );
                });
            });

            ui.add_space(4.0);

            // 2. Segmented Navigation Tabs
            ui.horizontal(|ui| {
                let tabs = [
                    (SettingsTab::Appearance, "🎨  Aparência"),
                    (SettingsTab::Typography, "🔤  Tipografia"),
                    (SettingsTab::Layout, "📐  Espaçamento"),
                ];

                for (tab, label) in tabs {
                    let is_active = self.selected_tab == tab;
                    let bg = if is_active {
                        Color32::from_rgb(42, 48, 68)
                    } else {
                        Color32::from_rgb(28, 32, 44)
                    };
                    let text_color = if is_active {
                        Color32::from_rgb(255, 255, 255)
                    } else {
                        Color32::from_rgb(160, 172, 195)
                    };

                    let btn = egui::Button::new(
                        RichText::new(label).size(13.5).color(text_color).strong(),
                    )
                    .fill(bg)
                    .rounding(Rounding::same(6.0_f32))
                    .min_size(vec2(130.0, 32.0));

                    if ui.add(btn).clicked() {
                        self.selected_tab = tab;
                    }
                }
            });

            ui.separator();
            ui.add_space(4.0);

            // 3. Tab Contents inside Scrollable View
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(ui.available_height() - 48.0)
                .show(ui, |ui| {
                    match self.selected_tab {
                        SettingsTab::Appearance => {
                            self.render_appearance_tab(ui, config, &mut config_changed);
                        }
                        SettingsTab::Typography => {
                            self.render_typography_tab(ui, config, &mut config_changed);
                        }
                        SettingsTab::Layout => {
                            self.render_layout_tab(ui, config, &mut config_changed);
                        }
                    }
                });

            ui.add_space(4.0);
            ui.separator();

            // 4. Footer Bar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Salvo em drop_config.json")
                        .size(11.5)
                        .color(Color32::from_rgb(100, 112, 135)),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(
                            RichText::new("Fechar")
                                .size(13.0)
                                .color(Color32::from_rgb(230, 235, 245)),
                        )
                        .clicked()
                    {
                        should_close = true;
                    }

                    if ui
                        .button(
                            RichText::new("Restaurar Padrões")
                                .size(12.5)
                                .color(Color32::from_rgb(180, 190, 205)),
                        )
                        .clicked()
                    {
                        *config = AppConfig::default();
                        config_changed = true;
                    }
                });
            });
        });

        if should_close {
            *is_open = false;
        }

        config_changed
    }

    fn render_appearance_tab(
        &self,
        ui: &mut Ui,
        config: &mut AppConfig,
        config_changed: &mut bool,
    ) {
        ui.label(
            RichText::new("Temas de Cores Disponíveis")
                .size(14.0)
                .color(Color32::from_rgb(230, 235, 245))
                .strong(),
        );
        ui.label(
            RichText::new("Selecione um tema para atualizar instantaneamente o terminal e a barra de abas.")
                .size(12.0)
                .color(Color32::from_rgb(140, 150, 170)),
        );
        ui.add_space(6.0);

        for theme in THEMES {
            let is_selected = config.theme == theme.name;
            let bg_color = if is_selected {
                Color32::from_rgb(32, 42, 60)
            } else {
                Color32::from_rgb(24, 28, 38)
            };
            let border_stroke = if is_selected {
                Stroke::new(1.5_f32, Color32::from_rgb(60, 130, 246)) // Vibrant Blue accent
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(40, 46, 62))
            };

            let card = Frame {
                inner_margin: Margin::symmetric(14.0_f32, 10.0_f32),
                outer_margin: Margin::symmetric(0.0_f32, 3.0_f32),
                rounding: Rounding::same(8.0_f32),
                shadow: Shadow::NONE,
                fill: bg_color,
                stroke: border_stroke,
            };

            card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(theme.name)
                                .size(14.0)
                                .color(if is_selected {
                                    Color32::from_rgb(255, 255, 255)
                                } else {
                                    Color32::from_rgb(210, 218, 232)
                                })
                                .strong(),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if is_selected {
                            ui.label(
                                RichText::new("✓ Ativo")
                                    .size(12.0)
                                    .color(Color32::from_rgb(90, 180, 255))
                                    .strong(),
                            );
                        } else if ui
                            .button(
                                RichText::new("Aplicar")
                                    .size(12.0)
                                    .color(Color32::from_rgb(180, 190, 210)),
                            )
                            .clicked()
                        {
                            config.theme = theme.name.to_string();
                            *config_changed = true;
                        }

                        // Theme Preview Swatches
                        draw_swatch(ui, [theme.accent[0], theme.accent[1], theme.accent[2]]);
                        draw_swatch(ui, [theme.tab_active_bg[0], theme.tab_active_bg[1], theme.tab_active_bg[2]]);
                        draw_swatch(ui, [theme.bg[0], theme.bg[1], theme.bg[2]]);
                    });
                });
            });
        }
    }

    fn render_typography_tab(
        &self,
        ui: &mut Ui,
        config: &mut AppConfig,
        config_changed: &mut bool,
    ) {
        ui.label(
            RichText::new("Família da Fonte")
                .size(14.0)
                .color(Color32::from_rgb(230, 235, 245))
                .strong(),
        );
        ui.label(
            RichText::new("Selecione entre as fontes de programação instaladas no sistema.")
                .size(12.0)
                .color(Color32::from_rgb(140, 150, 170)),
        );
        ui.add_space(6.0);

        let fonts = [
            ("Cascadia Code", "Padrão moderna do Windows Terminal com suporte a ligaduras"),
            ("Consolas", "Clássica e altamente legível em qualquer resolução"),
            ("Fira Code", "Visual icônico adorado por desenvolvedores"),
            ("JetBrains Mono", "Desenhada especificamente para reduzir fadiga visual"),
        ];

        for (font_name, font_desc) in fonts {
            let is_sel = config.font_family == font_name;
            let bg_color = if is_sel {
                Color32::from_rgb(32, 42, 60)
            } else {
                Color32::from_rgb(24, 28, 38)
            };
            let stroke = if is_sel {
                Stroke::new(1.5_f32, Color32::from_rgb(60, 130, 246))
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(40, 46, 62))
            };

            let card = Frame {
                inner_margin: Margin::symmetric(14.0_f32, 10.0_f32),
                outer_margin: Margin::symmetric(0.0_f32, 3.0_f32),
                rounding: Rounding::same(8.0_f32),
                shadow: Shadow::NONE,
                fill: bg_color,
                stroke,
            };

            card.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(font_name)
                                .size(14.0)
                                .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(210, 220, 235) })
                                .strong(),
                        );
                        ui.label(
                            RichText::new(font_desc)
                                .size(11.5)
                                .color(Color32::from_rgb(130, 140, 160)),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if is_sel {
                            ui.label(
                                RichText::new("✓ Selecionada")
                                    .size(12.0)
                                    .color(Color32::from_rgb(90, 180, 255))
                                    .strong(),
                            );
                        } else if ui
                            .button(
                                RichText::new("Escolher")
                                    .size(12.0)
                                    .color(Color32::from_rgb(180, 190, 210)),
                            )
                            .clicked()
                        {
                            config.font_family = font_name.to_string();
                            *config_changed = true;
                        }
                    });
                });
            });
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        // Font Size Slider
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Tamanho da Fonte:")
                    .size(13.5)
                    .color(Color32::from_rgb(220, 230, 245))
                    .strong(),
            );
            ui.label(
                RichText::new(format!("{:.1} pt", config.font_size))
                    .size(14.0)
                    .color(Color32::from_rgb(80, 200, 255))
                    .strong(),
            );
        });

        let old_size = config.font_size;
        ui.add(
            Slider::new(&mut config.font_size, 11.0..=22.0)
                .step_by(1.0)
                .suffix(" px")
                .show_value(false),
        );
        if (config.font_size - old_size).abs() > 0.01 {
            *config_changed = true;
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Atalhos rápidos:")
                    .size(12.0)
                    .color(Color32::from_rgb(130, 140, 160)),
            );
            for sz in [12.0, 13.0, 14.0, 15.0, 16.0, 18.0] {
                let is_sel = (config.font_size - sz).abs() < 0.5;
                let btn = egui::Button::new(
                    RichText::new(format!("{:.0}px", sz))
                        .size(12.0)
                        .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(160, 170, 190) }),
                )
                .fill(if is_sel { Color32::from_rgb(45, 60, 90) } else { Color32::from_rgb(28, 32, 44) })
                .rounding(Rounding::same(4.0));

                if ui.add(btn).clicked() && !is_sel {
                    config.font_size = sz;
                    *config_changed = true;
                }
            }
        });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label(
            RichText::new("Estilo do Cursor")
                .size(14.0)
                .color(Color32::from_rgb(230, 235, 245))
                .strong(),
        );
        ui.horizontal(|ui| {
            for (style_key, style_label) in [("bar", "Barra Vertical (|)"), ("block", "Bloco Sólido (█)")] {
                let is_sel = config.cursor_style == style_key;
                let btn = egui::Button::new(
                    RichText::new(style_label)
                        .size(12.5)
                        .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(180, 195, 215) }),
                )
                .fill(if is_sel { Color32::from_rgb(32, 42, 60) } else { Color32::from_rgb(20, 24, 33) })
                .rounding(Rounding::same(5.0));

                if ui.add(btn).clicked() && !is_sel {
                    config.cursor_style = style_key.to_string();
                    *config_changed = true;
                }
            }
        });
    }

    fn render_layout_tab(
        &self,
        ui: &mut Ui,
        config: &mut AppConfig,
        config_changed: &mut bool,
    ) {
        ui.label(
            RichText::new("Espaçamento e Margens do Terminal")
                .size(14.0)
                .color(Color32::from_rgb(230, 235, 245))
                .strong(),
        );
        ui.label(
            RichText::new("Ajuste as margens entre o texto do terminal e as bordas da tela.")
                .size(12.0)
                .color(Color32::from_rgb(140, 150, 170)),
        );
        ui.add_space(10.0);

        // Padding X
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Padding Horizontal (X):")
                    .size(13.5)
                    .color(Color32::from_rgb(220, 230, 245))
                    .strong(),
            );
            ui.label(
                RichText::new(format!("{:.0} px", config.padding_x))
                    .size(13.5)
                    .color(Color32::from_rgb(80, 200, 255))
                    .strong(),
            );
        });

        let old_px = config.padding_x;
        ui.add(
            Slider::new(&mut config.padding_x, 4.0..=50.0)
                .step_by(2.0)
                .suffix(" px")
                .show_value(false),
        );
        if (config.padding_x - old_px).abs() > 0.01 {
            *config_changed = true;
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Presets X:")
                    .size(12.0)
                    .color(Color32::from_rgb(130, 140, 160)),
            );
            for px in [8.0, 16.0, 24.0, 32.0, 40.0] {
                let is_sel = (config.padding_x - px).abs() < 1.0;
                let btn = egui::Button::new(
                    RichText::new(format!("{:.0}px", px))
                        .size(12.0)
                        .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(160, 170, 190) }),
                )
                .fill(if is_sel { Color32::from_rgb(45, 60, 90) } else { Color32::from_rgb(28, 32, 44) })
                .rounding(Rounding::same(4.0));

                if ui.add(btn).clicked() && !is_sel {
                    config.padding_x = px;
                    *config_changed = true;
                }
            }
        });

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(8.0);

        // Padding Y
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Padding Vertical (Y):")
                    .size(13.5)
                    .color(Color32::from_rgb(220, 230, 245))
                    .strong(),
            );
            ui.label(
                RichText::new(format!("{:.0} px", config.padding_y))
                    .size(13.5)
                    .color(Color32::from_rgb(80, 200, 255))
                    .strong(),
            );
        });

        let old_py = config.padding_y;
        ui.add(
            Slider::new(&mut config.padding_y, 4.0..=50.0)
                .step_by(2.0)
                .suffix(" px")
                .show_value(false),
        );
        if (config.padding_y - old_py).abs() > 0.01 {
            *config_changed = true;
        }

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Presets Y:")
                    .size(12.0)
                    .color(Color32::from_rgb(130, 140, 160)),
            );
            for py in [8.0, 12.0, 18.0, 24.0, 30.0] {
                let is_sel = (config.padding_y - py).abs() < 1.0;
                let btn = egui::Button::new(
                    RichText::new(format!("{:.0}px", py))
                        .size(12.0)
                        .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(160, 170, 190) }),
                )
                .fill(if is_sel { Color32::from_rgb(45, 60, 90) } else { Color32::from_rgb(28, 32, 44) })
                .rounding(Rounding::same(4.0));

                if ui.add(btn).clicked() && !is_sel {
                    config.padding_y = py;
                    *config_changed = true;
                }
            }
        });
    }
}

fn draw_swatch(ui: &mut Ui, rgb: [f32; 3]) {
    let (rect, _response) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
    let color = Color32::from_rgb(
        (rgb[0] * 255.0) as u8,
        (rgb[1] * 255.0) as u8,
        (rgb[2] * 255.0) as u8,
    );
    ui.painter().rect_filled(rect, Rounding::same(4.0), color);
    ui.painter().rect_stroke(rect, Rounding::same(4.0), Stroke::new(1.0_f32, Color32::from_white_alpha(40)));
}

/// Fully decoupled native OS window state for Settings
pub struct SettingsWindowState {
    pub window: Arc<WinitWindow>,
    pub hwnd: HWND,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub egui_ctx: egui::Context,
    pub egui_state: egui_winit::State,
    pub egui_renderer: egui_wgpu::Renderer,
    pub is_visible: bool,
}

impl SettingsWindowState {
    pub async fn new(window: Arc<WinitWindow>) -> anyhow::Result<Self> {
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
            .ok_or_else(|| anyhow::anyhow!("Failed to find GPU adapter for Settings window"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("DROP Settings GPU Device"),
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

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

        let hwnd = match window.window_handle() {
            Ok(handle) => match handle.as_raw() {
                RawWindowHandle::Win32(win32) => HWND(win32.hwnd.get() as *mut std::ffi::c_void),
                _ => HWND(std::ptr::null_mut()),
            },
            Err(_) => HWND(std::ptr::null_mut()),
        };

        if !hwnd.0.is_null() {
            apply_dwm_effects(hwnd);
        }

        Ok(Self {
            window,
            hwnd,
            surface,
            device,
            queue,
            config,
            egui_ctx,
            egui_state,
            egui_renderer,
            is_visible: true,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(
        &mut self,
        mut ui_callback: impl FnMut(&egui::Context) -> bool,
    ) -> anyhow::Result<bool> {
        let output = match self.surface.get_current_texture() {
            Ok(tex) => tex,
            Err(wgpu::SurfaceError::Outdated) => {
                let size = self.window.inner_size();
                self.resize(size.width, size.height);
                self.surface.get_current_texture()?
            }
            Err(e) => return Err(e.into()),
        };
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let mut config_changed = false;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            config_changed = ui_callback(ctx);
        });

        self.egui_state.handle_platform_output(&self.window, full_output.platform_output.clone());

        let clipped_primitives = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.device, &self.queue, *id, delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("DROP Settings Command Encoder"),
        });

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("DROP Settings Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 22.0 / 255.0,
                            g: 25.0 / 255.0,
                            b: 34.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer
                .render(&mut pass.forget_lifetime(), &clipped_primitives, &screen_descriptor);
        }

        self.queue.submit(Some(encoder.finish()));
        output.present();

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        Ok(config_changed)
    }
}
