use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    SetForegroundWindow, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOCOPYBITS, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SW_HIDE, SW_SHOW,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::platform::windows::WindowAttributesExtWindows;
use winit::window::{CursorIcon, Window, WindowAttributes, WindowId, WindowLevel};

use crate::config::{AppConfig, ThemePalette};
use crate::dwm::apply_dwm_effects;
use crate::hotkey::HotkeyManager;
use crate::input::handle_key_input;
use crate::pty::PtyManager;
use crate::renderer::{
    calculate_tab_geometries, GpuRenderer, TabVisualInfo, TAB_BAR_HEIGHT,
};
use crate::settings_ui::{SettingsUi, SettingsWindowState};
use crate::terminal::Terminal;

const BLINK_INTERVAL: Duration = Duration::from_millis(500);

// Smooth top-down slide animation durations (fluid entrance & snappy exit)
const OPEN_DURATION: Duration = Duration::from_millis(240);
const CLOSE_DURATION: Duration = Duration::from_millis(200);

// Bottom border resize grip thickness (pixels)
const RESIZE_BORDER_THICKNESS: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    Open,
    Opening,
    Closed,
    Closing,
}

// Fluid Quartic Deceleration for entrance: brisk start with soft, cushioned landing
fn ease_out_quart(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(4)
}

// Fluid Cubic In-Out for exit: gentle start, swift glide, and smooth settling
fn ease_in_out_cubic(x: f32) -> f32 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

pub struct Tab {
    pub id: usize,
    pub title: String,
    pub terminal: Arc<Mutex<Terminal>>,
    pub pty: PtyManager,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ToggleVisibility,
    PtyOutput { tab_id: usize, bytes: Vec<u8> },
}

pub struct DropApp {
    proxy: EventLoopProxy<AppEvent>,
    window: Option<Arc<Window>>,
    hwnd: Option<HWND>,
    renderer: Option<GpuRenderer>,
    text_buffer: Option<glyphon::Buffer>,
    cursor_buffer: Option<glyphon::Buffer>,
    tab_bar_buffer: Option<glyphon::Buffer>,
    tabs: Vec<Tab>,
    active_tab_index: usize,
    next_tab_id: usize,
    modifiers: ModifiersState,
    is_focused: bool,
    initialized: bool,
    cursor_visible: bool,
    last_blink: Instant,
    hotkey_manager: Option<HotkeyManager>,

    // App Configuration & Decoupled Settings Window
    pub config: AppConfig,
    pub cell_metrics: (f32, f32),
    pub settings_window: Option<SettingsWindowState>,
    pub pending_toggle_settings: bool,
    pub settings_ui: SettingsUi,

    // Monitor coordinates & drop-down geometry
    monitor_pos_x: i32,
    monitor_pos_y: i32,
    monitor_height: u32,
    drop_height: u32,
    last_animated_y: i32,

    // Mouse position & dragging state for border resize detection
    cursor_pos: PhysicalPosition<f64>,
    is_dragging_border: bool,
    scroll_pixel_accumulator: f32,

    // Slide animation state
    anim_state: AnimationState,
    anim_progress: f32, // 0.0 = fully retracted at top, 1.0 = fully dropped down
    anim_start_progress: f32,
    anim_start_time: Instant,
    anim_target_duration: Duration,
    last_toggle_time: Instant,
}

impl DropApp {
    pub fn new(proxy: EventLoopProxy<AppEvent>) -> Self {
        let config = AppConfig::load();
        let cell_metrics = config.cell_metrics();
        Self {
            proxy,
            window: None,
            hwnd: None,
            renderer: None,
            text_buffer: None,
            cursor_buffer: None,
            tab_bar_buffer: None,
            tabs: Vec::new(),
            active_tab_index: 0,
            next_tab_id: 1,
            modifiers: ModifiersState::default(),
            is_focused: true,
            initialized: false,
            cursor_visible: true,
            last_blink: Instant::now(),
            hotkey_manager: None,

            config,
            cell_metrics,
            settings_window: None,
            pending_toggle_settings: false,
            settings_ui: SettingsUi::new(),

            monitor_pos_x: 0,
            monitor_pos_y: 0,
            monitor_height: 1080,
            drop_height: 480,
            last_animated_y: -480,
            cursor_pos: PhysicalPosition::new(0.0, 0.0),
            is_dragging_border: false,
            scroll_pixel_accumulator: 0.0,

            anim_state: AnimationState::Closed,
            anim_progress: 0.0,
            anim_start_progress: 0.0,
            anim_start_time: Instant::now(),
            anim_target_duration: OPEN_DURATION,
            last_toggle_time: Instant::now() - Duration::from_secs(10),
        }
    }

    pub fn palette(&self) -> ThemePalette {
        self.config.current_theme()
    }

    pub fn update_cell_metrics(&mut self) {
        if let Some(renderer) = &mut self.renderer {
            self.cell_metrics =
                renderer.measure_cell_metrics(&self.config.font_family, self.config.font_size);
        } else {
            self.cell_metrics = self.config.cell_metrics();
        }
    }

    pub fn cell_metrics(&self) -> (f32, f32) {
        self.cell_metrics
    }

    pub fn grid_cols_rows(&self) -> (u16, u16) {
        let (cell_w, cell_h) = self.cell_metrics();
        let width = self
            .window
            .as_ref()
            .map(|w| w.inner_size().width)
            .unwrap_or(1920);
        let height = self.drop_height;
        let term_h = (height as f32 - TAB_BAR_HEIGHT - self.config.padding_y * 2.0).max(100.0);
        let cols = (((width as f32 - self.config.padding_x * 2.0) / cell_w).floor() as u16).max(40);
        let rows = ((term_h / cell_h).floor() as u16).max(5);
        (cols, rows)
    }

    pub fn apply_config(&mut self) {
        self.update_cell_metrics();
        let (cols, rows) = self.grid_cols_rows();
        let _width = self
            .window
            .as_ref()
            .map(|w| w.inner_size().width)
            .unwrap_or(1920);
        let height = self.drop_height;
        let palette = self.palette();

        if let Some(renderer) = &mut self.renderer {
            let term_h = (height as f32 - TAB_BAR_HEIGHT - self.config.padding_y * 2.0).max(100.0);
            let mut text_buf = renderer.create_buffer(
                self.config.font_size,
                self.config.padding_x,
                self.config.padding_y,
            );
            text_buf.set_size(
                &mut renderer.font_system,
                None,
                Some(term_h),
            );
            self.text_buffer = Some(text_buf);

            self.cursor_buffer = Some(renderer.create_cursor_buffer(
                &self.config.font_family,
                self.config.font_size,
                palette.cursor_color,
            ));
        }

        for tab in &mut self.tabs {
            tab.terminal.lock().unwrap().resize(cols, rows);
            let _ = tab.pty.resize(cols, rows);
        }

        self.update_tab_bar_buffer();
        let _ = self.config.save();

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub fn toggle_settings(&mut self, event_loop: Option<&ActiveEventLoop>) {
        if let Some(settings) = &mut self.settings_window {
            if settings.is_visible {
                settings.window.set_visible(false);
                settings.is_visible = false;
            } else {
                settings.window.set_visible(true);
                settings.is_visible = true;
                settings.window.focus_window();
                settings.window.request_redraw();
                if !settings.hwnd.0.is_null() {
                    unsafe {
                        let _ = SetForegroundWindow(settings.hwnd);
                    }
                }
            }
        } else if let Some(el) = event_loop {
            self.create_settings_window(el);
        } else {
            self.pending_toggle_settings = true;
        }
    }

    pub fn create_settings_window(&mut self, event_loop: &ActiveEventLoop) {
        let size = PhysicalSize::new(680, 520);
        let (pos_x, pos_y) = if let Some(monitor) = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next())
        {
            let mon_size = monitor.size();
            let mon_pos = monitor.position();
            let x = mon_pos.x + (mon_size.width as i32 - 680) / 2;
            let y = mon_pos.y + (mon_size.height as i32 - 520) / 2;
            (x, y)
        } else {
            (self.monitor_pos_x + 100, self.monitor_pos_y + 100)
        };

        let window_attrs = WindowAttributes::default()
            .with_title("Configurações // Settings")
            .with_inner_size(size)
            .with_min_inner_size(PhysicalSize::new(560, 420))
            .with_position(PhysicalPosition::new(pos_x, pos_y))
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(true)
            .with_resizable(true)
            .with_visible(true);

        match event_loop.create_window(window_attrs) {
            Ok(win) => {
                let win = Arc::new(win);
                match pollster::block_on(SettingsWindowState::new(win.clone())) {
                    Ok(state) => {
                        state.window.focus_window();
                        state.window.request_redraw();
                        if !state.hwnd.0.is_null() {
                            unsafe {
                                let _ = SetForegroundWindow(state.hwnd);
                            }
                        }
                        self.settings_window = Some(state);
                        tracing::info!("Decoupled Settings window created and opened");
                    }
                    Err(e) => {
                        tracing::error!("Failed to create Settings window GPU state: {:?}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to create Settings winit window: {:?}", e);
            }
        }
    }

    pub fn create_tab(&mut self, title: Option<String>) {
        let (cols, rows) = self.grid_cols_rows();
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let terminal = Arc::new(Mutex::new(Terminal::new(cols, rows)));
        let proxy = self.proxy.clone();
        match PtyManager::new(tab_id, cols, rows, proxy, |id, bytes| AppEvent::PtyOutput {
            tab_id: id,
            bytes,
        }) {
            Ok(pty) => {
                let tab = Tab {
                    id: tab_id,
                    title: title.unwrap_or_else(|| "pwsh".to_string()),
                    terminal,
                    pty,
                };
                self.tabs.push(tab);
                self.active_tab_index = self.tabs.len() - 1;
                self.update_tab_bar_buffer();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Err(e) => {
                tracing::error!("Failed to spawn PTY for new tab: {:?}", e);
            }
        }
    }

    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            // Spawn a fresh tab if all tabs were closed
            self.create_tab(None);
        } else {
            if self.active_tab_index >= self.tabs.len() {
                self.active_tab_index = self.tabs.len() - 1;
            }
            self.update_tab_bar_buffer();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    pub fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() && index != self.active_tab_index {
            self.active_tab_index = index;
            self.cursor_visible = true;
            self.last_blink = Instant::now();
            self.update_tab_bar_buffer();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            let next = (self.active_tab_index + 1) % self.tabs.len();
            self.switch_tab(next);
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            let prev = if self.active_tab_index == 0 {
                self.tabs.len().saturating_sub(1)
            } else {
                self.active_tab_index - 1
            };
            self.switch_tab(prev);
        }
    }

    pub fn handle_tab_bar_click(&mut self, click_x: f64, event_loop: Option<&ActiveEventLoop>) {
        let width = self
            .window
            .as_ref()
            .map(|w| w.inner_size().width)
            .unwrap_or(1920) as f32;
        let btn_w = 26.0;
        let plus_btn_x = (width - btn_w - 8.0).max(0.0);
        let set_btn_x = (plus_btn_x - btn_w - 6.0).max(0.0);

        // 1. Check if clicked on Settings [⚙] button
        if (click_x as f32) >= (set_btn_x - 3.0) && (click_x as f32) < (plus_btn_x - 3.0) {
            self.toggle_settings(event_loop);
            return;
        }

        // 2. Check if clicked on the [+] New Tab button on the far right
        if (click_x as f32) >= (plus_btn_x - 3.0) {
            self.create_tab(None);
            return;
        }

        // 3. Check individual tabs and close button
        let titles: Vec<&str> = self.tabs.iter().map(|t| t.title.as_str()).collect();
        let geoms = calculate_tab_geometries(&titles, 8.0);

        for geom in geoms {
            if (click_x as f32) >= geom.x && (click_x as f32) < geom.x + geom.width {
                if (click_x as f32) >= geom.close_x
                    && (click_x as f32) <= geom.close_x + geom.close_w
                {
                    self.close_tab(geom.index);
                } else {
                    self.switch_tab(geom.index);
                }
                return;
            }
        }
    }

    pub fn paste_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    if let Some(tab) = self.tabs.get(self.active_tab_index) {
                        tab.terminal.lock().unwrap().scroll_to_bottom();
                        let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
                        if let Err(e) = tab.pty.write(normalized.as_bytes()) {
                            tracing::error!("Failed to write clipboard to PTY: {:?}", e);
                        } else if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                }
            }
        }
    }

    pub fn update_tab_bar_buffer(&mut self) {
        let (renderer, tab_buffer) = match (&mut self.renderer, &mut self.tab_bar_buffer) {
            (Some(r), Some(b)) => (r, b),
            _ => return,
        };

        let base_attrs = glyphon::Attrs::new().family(glyphon::Family::Name("Cascadia Code"));
        let active_attrs = base_attrs.color(glyphon::Color::rgb(240, 246, 252)); // Crisp White
        let inactive_attrs = base_attrs.color(glyphon::Color::rgb(148, 163, 184)); // Muted Slate
        let active_close_attrs = base_attrs.color(glyphon::Color::rgb(148, 163, 184));
        let inactive_close_attrs = base_attrs.color(glyphon::Color::rgb(100, 116, 139));

        let mut spans: Vec<(String, glyphon::Attrs)> = Vec::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            let is_active = i == self.active_tab_index;
            let text_attrs = if is_active { active_attrs } else { inactive_attrs };
            let close_attrs = if is_active { active_close_attrs } else { inactive_close_attrs };

            spans.push((format!("   {}: {}  ", i + 1, tab.title), text_attrs));
            spans.push(("×".to_string(), close_attrs));
            spans.push(("   ".to_string(), text_attrs));
        }

        let span_refs: Vec<(&str, glyphon::Attrs)> = spans.iter().map(|(s, a)| (s.as_str(), *a)).collect();

        tab_buffer.set_rich_text(
            &mut renderer.font_system,
            span_refs,
            inactive_attrs,
            glyphon::Shaping::Basic,
        );
        tab_buffer.shape_until_scroll(&mut renderer.font_system, false);
    }

    pub fn resize_window_height(&mut self, new_height: u32) {
        let max_height = ((self.monitor_height as f64) * 0.95) as u32;
        let clamped = new_height.clamp(200, max_height);
        if self.drop_height == clamped {
            return;
        }
        self.drop_height = clamped;

        let (window, hwnd) = match (&self.window, self.hwnd) {
            (Some(w), Some(h)) => (w.clone(), h),
            _ => return,
        };
        let width = window.inner_size().width;

        // 1. Resize HWND immediately
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                None,
                self.monitor_pos_x,
                self.monitor_pos_y,
                width as i32,
                clamped as i32,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        let _ = window.request_inner_size(PhysicalSize::new(width, clamped));

        // 2. Immediately reconfigure GPU swapchain & buffers to prevent any texture stretching
        let (cell_w, cell_h) = self.cell_metrics();
        let term_h = (clamped as f32 - TAB_BAR_HEIGHT - self.config.padding_y * 2.0).max(100.0);
        let cols = (((width as f32 - self.config.padding_x * 2.0) / cell_w).floor() as u16).max(40);
        let rows = ((term_h / cell_h).floor() as u16).max(5);

        if let Some(renderer) = &mut self.renderer {
            renderer.resize(width, clamped);
            if let Some(buffer) = &mut self.text_buffer {
                buffer.set_size(
                    &mut renderer.font_system,
                    None,
                    Some(term_h),
                );
            }
            if let Some(tab_bar_buf) = &mut self.tab_bar_buffer {
                tab_bar_buf.set_size(
                    &mut renderer.font_system,
                    Some(width as f32),
                    Some(TAB_BAR_HEIGHT),
                );
            }
        }

        for tab in &mut self.tabs {
            tab.terminal.lock().unwrap().resize(cols, rows);
            let _ = tab.pty.resize(cols, rows);
        }

        self.update_tab_bar_buffer();

        window.request_redraw();
    }

    fn toggle_visibility(&mut self) {
        let (window, hwnd) = match (&self.window, self.hwnd) {
            (Some(w), Some(h)) => (w, h),
            _ => return,
        };

        // Debounce toggle calls within 150ms to prevent double-toggle between hook and local key event
        let now = Instant::now();
        if now.duration_since(self.last_toggle_time) < Duration::from_millis(150) {
            return;
        }
        self.last_toggle_time = now;

        match self.anim_state {
            AnimationState::Open | AnimationState::Opening => {
                // Trigger slide-up exit
                self.anim_state = AnimationState::Closing;
                self.anim_start_progress = self.anim_progress;
                self.anim_start_time = Instant::now();
                self.anim_target_duration = CLOSE_DURATION.mul_f32(self.anim_progress.max(0.05));
                tracing::info!(
                    "DROP sliding UP to close from progress {:.2}",
                    self.anim_progress
                );
            }
            AnimationState::Closed | AnimationState::Closing => {
                // Trigger slide-down entrance
                self.anim_state = AnimationState::Opening;
                self.anim_start_progress = self.anim_progress;
                self.anim_start_time = Instant::now();
                self.anim_target_duration =
                    OPEN_DURATION.mul_f32((1.0 - self.anim_progress).max(0.05));

                window.set_visible(true);
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                window.focus_window();
                self.is_focused = true;
                self.cursor_visible = true;
                self.last_blink = Instant::now();
                tracing::info!(
                    "DROP sliding DOWN to open from progress {:.2}",
                    self.anim_progress
                );
            }
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler<AppEvent> for DropApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let primary_monitor = event_loop
            .primary_monitor()
            .or_else(|| event_loop.available_monitors().next());

        let (width, height, pos_x, pos_y, mon_height) = if let Some(ref monitor) = primary_monitor {
            let size = monitor.size();
            let pos = monitor.position();
            let drop_height = ((size.height as f64) * 0.40) as u32;
            (size.width, drop_height, pos.x, pos.y, size.height)
        } else {
            (1920, 480, 0, 0, 1080)
        };

        self.monitor_pos_x = pos_x;
        self.monitor_pos_y = pos_y;
        self.monitor_height = mon_height;
        self.drop_height = height;
        let start_y = pos_y - height as i32;
        self.last_animated_y = start_y;

        let (cell_w, cell_h) = self.config.cell_metrics();
        let term_h = (height as f32 - TAB_BAR_HEIGHT - self.config.padding_y * 2.0).max(100.0);
        let cols = (((width as f32 - self.config.padding_x * 2.0) / cell_w).floor() as u16).max(40);
        let rows = ((term_h / cell_h).floor() as u16).max(5);

        tracing::info!(
            "Initializing DROP Terminal: {}x{} (Grid: {} cols x {} rows, Start Y: {})",
            width, height, cols, rows, start_y
        );

        let window_attrs = WindowAttributes::default()
            .with_title("DROP Terminal")
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_inner_size(PhysicalSize::new(width, height))
            .with_position(PhysicalPosition::new(pos_x, start_y))
            .with_skip_taskbar(false)
            .with_visible(false);

        match event_loop.create_window(window_attrs) {
            Ok(win) => {
                let win = Arc::new(win);
                let hwnd = match win.window_handle() {
                    Ok(handle) => match handle.as_raw() {
                        RawWindowHandle::Win32(win32) => {
                            HWND(win32.hwnd.get() as *mut std::ffi::c_void)
                        }
                        _ => {
                            tracing::error!("Unsupported window handle platform");
                            return;
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to get window handle: {:?}", e);
                        return;
                    }
                };

                apply_dwm_effects(hwnd);

                // Initialize GPU Render Pipeline (wgpu + glyphon)
                match pollster::block_on(GpuRenderer::new(win.clone(), width, height)) {
                    Ok(mut renderer) => {
                        tracing::info!("GPU Render Pipeline (wgpu + glyphon) initialized successfully");
                        let measured_metrics = renderer.measure_cell_metrics(
                            &self.config.font_family,
                            self.config.font_size,
                        );
                        let buffer = renderer.create_buffer(
                            self.config.font_size,
                            self.config.padding_x,
                            self.config.padding_y,
                        );
                        let palette = self.palette();
                        let cursor_buf = renderer.create_cursor_buffer(
                            &self.config.font_family,
                            self.config.font_size,
                            palette.cursor_color,
                        );
                        let tab_bar_buf = renderer.create_tab_bar_buffer();
                        self.text_buffer = Some(buffer);
                        self.cursor_buffer = Some(cursor_buf);
                        self.tab_bar_buffer = Some(tab_bar_buf);
                        self.renderer = Some(renderer);
                        self.cell_metrics = measured_metrics;
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize GPU renderer: {:?}", e);
                    }
                }

                // Initialize Initial Tab (PowerShell 7)
                let (cols, rows) = self.grid_cols_rows();
                let tab_id = 1;
                self.next_tab_id = 2;
                let terminal = Arc::new(Mutex::new(Terminal::new(cols, rows)));
                let proxy_clone = self.proxy.clone();
                match PtyManager::new(tab_id, cols, rows, proxy_clone, |id, bytes| AppEvent::PtyOutput {
                    tab_id: id,
                    bytes,
                }) {
                    Ok(pty) => {
                        tracing::info!("PTY session spawned successfully for Tab 1 with PowerShell 7");
                        self.tabs.push(Tab {
                            id: tab_id,
                            title: "pwsh".to_string(),
                            terminal,
                            pty,
                        });
                        self.active_tab_index = 0;
                    }
                    Err(e) => {
                        tracing::error!("Failed to spawn PTY session: {:?}", e);
                    }
                }
                self.update_tab_bar_buffer();

                // Register global hotkey
                let proxy_clone = self.proxy.clone();
                let hotkey = HotkeyManager::new(proxy_clone, || AppEvent::ToggleVisibility);
                self.hotkey_manager = Some(hotkey);

                self.hwnd = Some(hwnd);
                self.window = Some(win.clone());
                self.is_focused = true;
                self.cursor_visible = true;
                self.last_blink = Instant::now();

                // Render the first frame BEFORE making window visible, preventing any startup flicker/blur flash
                let palette = self.palette();
                if let (Some(renderer), Some(buffer)) = (
                    &mut self.renderer,
                    &mut self.text_buffer,
                ) {
                    if let Some(tab) = self.tabs.get(self.active_tab_index) {
                        let (cell_w, cell_h) = self.cell_metrics;
                        let (col, row, scroll_info, cell_quads) = tab
                            .terminal
                            .lock()
                            .unwrap()
                            .render_to_buffer(
                                &mut renderer.font_system,
                                buffer,
                                &self.config.font_family,
                                glyphon::Color::rgb(
                                    palette.fg_color[0],
                                    palette.fg_color[1],
                                    palette.fg_color[2],
                                ),
                            );
                        let cursor_info = if row < usize::MAX {
                            let cx = self.config.padding_x + (col as f32) * cell_w;
                            let cy = self.config.padding_y + (row as f32) * cell_h;
                            Some((cx, cy, cell_w, cell_h))
                        } else {
                            None
                        };
                        let titles: Vec<&str> = self.tabs.iter().map(|t| t.title.as_str()).collect();
                        let geoms = calculate_tab_geometries(&titles, 8.0);
                        let tabs_visual: Vec<TabVisualInfo> = geoms
                            .iter()
                            .map(|g| TabVisualInfo {
                                x: g.x,
                                width: g.width,
                                is_active: g.index == self.active_tab_index,
                                is_hovered: false,
                            })
                            .collect();
                        let tab_bar_ref = self.tab_bar_buffer.as_ref();
                        let _ = renderer.render_terminal(
                            &self.config,
                            &palette,
                            buffer,
                            tab_bar_ref,
                            &tabs_visual,
                            false,
                            false,
                            cursor_info,
                            Some(scroll_info),
                            &cell_quads,
                            cell_w,
                            cell_h,
                        );
                    }
                }

                // Start initial slide-down entrance animation
                self.anim_state = AnimationState::Opening;
                self.anim_progress = 0.0;
                self.anim_start_progress = 0.0;
                self.anim_start_time = Instant::now();
                self.anim_target_duration = OPEN_DURATION;

                win.set_visible(true);
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                win.focus_window();
            }
            Err(e) => {
                tracing::error!("Failed to create window: {:?}", e);
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::ToggleVisibility => {
                self.toggle_visibility();
                event_loop.set_control_flow(ControlFlow::Poll);
            }
            AppEvent::PtyOutput { tab_id, bytes } => {
                let mut should_redraw = false;
                if let Some(tab) = self.tabs.iter().find(|t| t.id == tab_id) {
                    tab.terminal.lock().unwrap().process_bytes(&bytes);
                    if let Some(active_tab) = self.tabs.get(self.active_tab_index) {
                        if active_tab.id == tab_id {
                            should_redraw = true;
                        }
                    }
                }
                if should_redraw {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.initialized {
            self.initialized = true;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }

        // Animate slide entrance or exit
        match self.anim_state {
            AnimationState::Opening => {
                let hwnd = match self.hwnd {
                    Some(h) => h,
                    None => return,
                };
                let now = Instant::now();
                let elapsed = now.duration_since(self.anim_start_time);
                let duration_secs = self.anim_target_duration.as_secs_f32();

                if duration_secs <= 0.0 || elapsed >= self.anim_target_duration {
                    self.anim_progress = 1.0;
                    self.anim_state = AnimationState::Open;
                    self.last_animated_y = self.monitor_pos_y;
                    unsafe {
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            self.monitor_pos_x,
                            self.monitor_pos_y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS,
                        );
                    }
                    if let Some(window) = &self.window {
                        window.focus_window();
                        window.request_redraw();
                    }
                    event_loop.set_control_flow(ControlFlow::WaitUntil(now + BLINK_INTERVAL));
                    return;
                } else {
                    let t = (elapsed.as_secs_f32() / duration_secs).clamp(0.0, 1.0);
                    let eased = ease_out_quart(t);
                    self.anim_progress =
                        self.anim_start_progress + (1.0 - self.anim_start_progress) * eased;

                    let y = self.monitor_pos_y
                        - ((1.0 - self.anim_progress) * self.drop_height as f32).round() as i32;
                    if y != self.last_animated_y {
                        self.last_animated_y = y;
                        unsafe {
                            let _ = SetWindowPos(
                                hwnd,
                                None,
                                self.monitor_pos_x,
                                y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS,
                            );
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    event_loop.set_control_flow(ControlFlow::Poll);
                    return;
                }
            }
            AnimationState::Closing => {
                let hwnd = match self.hwnd {
                    Some(h) => h,
                    None => return,
                };
                let now = Instant::now();
                let elapsed = now.duration_since(self.anim_start_time);
                let duration_secs = self.anim_target_duration.as_secs_f32();

                if duration_secs <= 0.0 || elapsed >= self.anim_target_duration {
                    self.anim_progress = 0.0;
                    self.anim_state = AnimationState::Closed;
                    let y = self.monitor_pos_y - self.drop_height as i32;
                    self.last_animated_y = y;
                    unsafe {
                        let _ = SetWindowPos(
                            hwnd,
                            None,
                            self.monitor_pos_x,
                            y,
                            0,
                            0,
                            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS,
                        );
                        let _ = ShowWindow(hwnd, SW_HIDE);
                    }
                    if let Some(window) = &self.window {
                        window.set_visible(false);
                    }
                    self.is_focused = false;
                    event_loop.set_control_flow(ControlFlow::Wait);
                    return;
                } else {
                    let t = (elapsed.as_secs_f32() / duration_secs).clamp(0.0, 1.0);
                    let eased = ease_in_out_cubic(t);
                    self.anim_progress = self.anim_start_progress * (1.0 - eased);

                    let y = self.monitor_pos_y
                        - ((1.0 - self.anim_progress) * self.drop_height as f32).round() as i32;
                    if y != self.last_animated_y {
                        self.last_animated_y = y;
                        unsafe {
                            let _ = SetWindowPos(
                                hwnd,
                                None,
                                self.monitor_pos_x,
                                y,
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS,
                            );
                        }
                        if let Some(window) = &self.window {
                            window.request_redraw();
                        }
                    }
                    event_loop.set_control_flow(ControlFlow::Poll);
                    return;
                }
            }
            _ => {}
        }

        if self.pending_toggle_settings {
            self.pending_toggle_settings = false;
            self.toggle_settings(Some(event_loop));
        }

        // 500ms cursor blink cycle when window is active and focused
        if self.anim_state == AnimationState::Open && self.is_focused {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_blink);

            if elapsed >= BLINK_INTERVAL {
                self.cursor_visible = !self.cursor_visible;
                self.last_blink = now;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                event_loop.set_control_flow(ControlFlow::WaitUntil(now + BLINK_INTERVAL));
            } else {
                event_loop
                    .set_control_flow(ControlFlow::WaitUntil(self.last_blink + BLINK_INTERVAL));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Handle events for decoupled Settings window
        let is_settings_window = self
            .settings_window
            .as_ref()
            .map(|s| s.window.id() == window_id)
            .unwrap_or(false);

        if is_settings_window {
            let mut should_hide = false;
            let mut config_changed = false;

            if let Some(settings) = &mut self.settings_window {
                let response = settings.egui_state.on_window_event(&settings.window, &event);
                if response.repaint {
                    settings.window.request_redraw();
                }

                match event {
                    WindowEvent::CloseRequested => {
                        should_hide = true;
                    }
                    WindowEvent::Resized(new_size) => {
                        settings.resize(new_size.width, new_size.height);
                        settings.window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        let mut is_open = true;
                        let config = &mut self.config;
                        let settings_ui = &mut self.settings_ui;

                        match settings.render(|ctx| {
                            settings_ui.show(ctx, config, &mut is_open)
                        }) {
                            Ok(changed) => {
                                config_changed = changed;
                            }
                            Err(e) => {
                                tracing::error!("Error rendering Settings window: {:?}", e);
                            }
                        }

                        if !is_open {
                            should_hide = true;
                        }
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                ref logical_key,
                                state,
                                ..
                            },
                        ..
                    } => {
                        if state.is_pressed() && *logical_key == Key::Named(NamedKey::Escape) {
                            should_hide = true;
                        }
                    }
                    _ => {}
                }
            }

            if should_hide {
                if let Some(settings) = &mut self.settings_window {
                    settings.window.set_visible(false);
                    settings.is_visible = false;
                }
            }

            if config_changed {
                self.apply_config();
            }

            return;
        }

        match event {
            WindowEvent::Focused(focused) => {
                self.is_focused = focused;
                if focused {
                    self.cursor_visible = true;
                    self.last_blink = Instant::now();
                } else {
                    self.cursor_visible = false;
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
                let width = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().width)
                    .unwrap_or(1920) as f64;
                let height = self.drop_height as f64;
                let tab_bar_top = height - TAB_BAR_HEIGHT as f64;

                let btn_w = 26.0;
                let plus_x = (width - btn_w - 8.0).max(0.0);
                let set_x = (plus_x - btn_w - 6.0).max(0.0);

                if self.is_dragging_border {
                    self.resize_window_height(position.y.round() as u32);
                    if let Some(window) = &self.window {
                        window.set_cursor(CursorIcon::NsResize);
                    }
                } else if self.anim_state == AnimationState::Open {
                    let is_on_bottom_border = position.y >= (height - RESIZE_BORDER_THICKNESS);
                    let is_on_plus_btn = position.y >= tab_bar_top
                        && position.x >= plus_x
                        && position.x <= (plus_x + btn_w);
                    let is_on_set_btn = position.y >= tab_bar_top
                        && position.x >= set_x
                        && position.x <= (set_x + btn_w);

                    if let Some(window) = &self.window {
                        if is_on_bottom_border {
                            window.set_cursor(CursorIcon::NsResize);
                        } else if is_on_plus_btn || is_on_set_btn {
                            window.set_cursor(CursorIcon::Pointer);
                            window.request_redraw();
                        } else if position.y >= tab_bar_top {
                            window.set_cursor(CursorIcon::Pointer);
                            window.request_redraw();
                        } else {
                            window.set_cursor(CursorIcon::Default);
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                if state == ElementState::Pressed {
                    if button == MouseButton::Right {
                        self.paste_clipboard();
                        return;
                    }
                    if button == MouseButton::Left {
                        let y = self.cursor_pos.y;
                        let height = self.drop_height as f64;
                        let is_on_bottom_border = y >= (height - RESIZE_BORDER_THICKNESS);

                        if is_on_bottom_border {
                            self.is_dragging_border = true;
                        } else if y >= (height - TAB_BAR_HEIGHT as f64) {
                            self.handle_tab_bar_click(self.cursor_pos.x, Some(event_loop));
                        }
                    }
                } else if button == MouseButton::Left {
                    self.is_dragging_border = false;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (_cell_w, cell_h) = self.cell_metrics;
                let lines = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_x, y) => (y * 3.0).round() as i32,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.scroll_pixel_accumulator += pos.y as f32;
                        let line_height = cell_h.max(12.0);
                        let l = (self.scroll_pixel_accumulator / line_height) as i32;
                        if l != 0 {
                            self.scroll_pixel_accumulator -= l as f32 * line_height;
                        }
                        l
                    }
                };
                if lines != 0 {
                    if let Some(tab) = self.tabs.get(self.active_tab_index) {
                        let is_alt = tab.terminal.lock().unwrap().is_alt_screen();
                        if is_alt {
                            let key = if lines > 0 { b"\x1b[A" } else { b"\x1b[B" };
                            let count = lines.abs();
                            let mut bytes = Vec::with_capacity(3 * count as usize);
                            for _ in 0..count {
                                bytes.extend_from_slice(key);
                            }
                            let _ = tab.pty.write(&bytes);
                        } else {
                            tab.terminal.lock().unwrap().scroll(lines);
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.drop_height = size.height;
                let (cell_w, cell_h) = self.cell_metrics();
                let term_h = (size.height as f32 - TAB_BAR_HEIGHT - self.config.padding_y * 2.0).max(100.0);
                let cols = (((size.width as f32 - self.config.padding_x * 2.0) / cell_w).floor() as u16).max(40);
                let rows = ((term_h / cell_h).floor() as u16).max(5);

                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                    if let Some(buffer) = &mut self.text_buffer {
                        buffer.set_size(
                            &mut renderer.font_system,
                            None,
                            Some(term_h),
                        );
                    }
                    if let Some(tab_bar_buf) = &mut self.tab_bar_buffer {
                        tab_bar_buf.set_size(
                            &mut renderer.font_system,
                            Some(size.width as f32),
                            Some(TAB_BAR_HEIGHT),
                        );
                    }
                }

                for tab in &mut self.tabs {
                    tab.terminal.lock().unwrap().resize(cols, rows);
                    let _ = tab.pty.resize(cols, rows);
                }

                self.update_tab_bar_buffer();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let palette = self.palette();

                if let (Some(renderer), Some(buffer)) = (
                    &mut self.renderer,
                    &mut self.text_buffer,
                ) {
                    if let Some(tab) = self.tabs.get(self.active_tab_index) {
                        let (cell_w, cell_h) = self.cell_metrics;
                        let (col, row, scroll_info, cell_quads) = tab
                            .terminal
                            .lock()
                            .unwrap()
                            .render_to_buffer(
                                &mut renderer.font_system,
                                buffer,
                                &self.config.font_family,
                                glyphon::Color::rgb(
                                    palette.fg_color[0],
                                    palette.fg_color[1],
                                    palette.fg_color[2],
                                ),
                            );

                        let cursor_info = if self.cursor_visible && self.is_focused && row < usize::MAX {
                            let cx = self.config.padding_x + (col as f32) * cell_w;
                            let cy = self.config.padding_y + (row as f32) * cell_h;
                            Some((cx, cy, cell_w, cell_h))
                        } else {
                            None
                        };

                        let width = renderer.config.width as f32;
                        let height = renderer.config.height as f32;
                        let tab_bar_top = height - TAB_BAR_HEIGHT;
                        let btn_w = 26.0;
                        let plus_x = (width - btn_w - 8.0).max(0.0);
                        let set_x = (plus_x - btn_w - 6.0).max(0.0);
                        let plus_hovered = self.cursor_pos.y as f32 >= tab_bar_top
                            && self.cursor_pos.x as f32 >= plus_x
                            && self.cursor_pos.x as f32 <= plus_x + btn_w;
                        let settings_hovered = self.cursor_pos.y as f32 >= tab_bar_top
                            && self.cursor_pos.x as f32 >= set_x
                            && self.cursor_pos.x as f32 <= set_x + btn_w;

                        let titles: Vec<&str> =
                            self.tabs.iter().map(|t| t.title.as_str()).collect();
                        let geoms = calculate_tab_geometries(&titles, 8.0);
                        let tabs_visual: Vec<TabVisualInfo> = geoms
                            .iter()
                            .map(|g| {
                                let hovered = self.cursor_pos.y as f32 >= tab_bar_top
                                    && self.cursor_pos.x as f32 >= g.x
                                    && (self.cursor_pos.x as f32) < g.x + g.width;
                                TabVisualInfo {
                                    x: g.x,
                                    width: g.width,
                                    is_active: g.index == self.active_tab_index,
                                    is_hovered: hovered,
                                }
                            })
                            .collect();

                        let tab_bar_ref = self.tab_bar_buffer.as_ref();
                        if let Err(e) = renderer.render_terminal(
                            &self.config,
                            &palette,
                            buffer,
                            tab_bar_ref,
                            &tabs_visual,
                            plus_hovered,
                            settings_hovered,
                            cursor_info,
                            Some(scroll_info),
                            &cell_quads,
                            cell_w,
                            cell_h,
                        ) {
                            tracing::error!("GPU frame render error: {:?}", e);
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        ref logical_key,
                        state,
                        ref text,
                        ref physical_key,
                        ..
                    },
                ..
            } => {
                // Keep cursor solidly visible while typing
                self.cursor_visible = true;
                self.last_blink = Instant::now();

                // Shift+Escape closes DROP application
                if *logical_key == Key::Named(NamedKey::Escape) && self.modifiers.shift_key() {
                    tracing::info!("Shift+Escape pressed, exiting DROP");
                    event_loop.exit();
                    return;
                }

                // Paste shortcuts: Ctrl+Shift+V, Ctrl+V, Shift+Insert
                let is_ctrl_v = self.modifiers.control_key()
                    && !self.modifiers.alt_key()
                    && match logical_key {
                        Key::Character(s) => s.eq_ignore_ascii_case("v"),
                        _ => false,
                    };
                let is_shift_insert = self.modifiers.shift_key()
                    && matches!(logical_key, Key::Named(NamedKey::Insert));

                if is_ctrl_v || is_shift_insert {
                    if state.is_pressed() {
                        self.paste_clipboard();
                    }
                    return;
                }

                // Ctrl + , : Open / Toggle Settings Modal
                if self.modifiers.control_key()
                    && !self.modifiers.shift_key()
                    && !self.modifiers.alt_key()
                {
                    let is_comma = match logical_key {
                        Key::Character(s) => s == ",",
                        _ => false,
                    } || matches!(physical_key, PhysicalKey::Code(KeyCode::Comma));

                    if is_comma {
                        if state.is_pressed() {
                            self.toggle_settings(Some(event_loop));
                        }
                        return;
                    }
                }

                // Ctrl + ': Toggle DROP open/close (swallow immediately to prevent typing into shell)
                if self.modifiers.control_key()
                    && !self.modifiers.shift_key()
                    && !self.modifiers.alt_key()
                {
                    let is_quote = match logical_key {
                        Key::Character(s) => s == "'" || s == "`" || s == "´" || s == "\"",
                        Key::Dead(ch) => {
                            matches!(ch, Some('\'') | Some('`') | Some('´') | Some('"') | None)
                        }
                        _ => false,
                    } || matches!(
                        physical_key,
                        PhysicalKey::Code(KeyCode::Quote) | PhysicalKey::Code(KeyCode::Backquote)
                    );

                    if is_quote {
                        if state.is_pressed() {
                            self.toggle_visibility();
                        }
                        return;
                    }
                }

                // Tab management keyboard shortcuts:
                // Ctrl+Shift+T: New Tab
                // Ctrl+Shift+W: Close Tab
                // Ctrl+Tab / Ctrl+PageDown: Next Tab
                // Ctrl+Shift+Tab / Ctrl+PageUp: Previous Tab
                // Alt+1..9: Switch to Tab 1..9
                if self.modifiers.control_key() {
                    if *logical_key == Key::Named(NamedKey::PageUp) {
                        if state.is_pressed() {
                            self.prev_tab();
                        }
                        return;
                    }
                    if *logical_key == Key::Named(NamedKey::PageDown) {
                        if state.is_pressed() {
                            self.next_tab();
                        }
                        return;
                    }

                    if self.modifiers.shift_key() {
                        if *logical_key == Key::Named(NamedKey::Tab) {
                            if state.is_pressed() {
                                self.prev_tab();
                            }
                            return;
                        }

                        if let Key::Character(ref ch) = logical_key {
                            if ch.eq_ignore_ascii_case("t") && state.is_pressed() {
                                self.create_tab(None);
                                return;
                            }
                            if ch.eq_ignore_ascii_case("w") && state.is_pressed() {
                                self.close_tab(self.active_tab_index);
                                return;
                            }
                        }
                    } else if *logical_key == Key::Named(NamedKey::Tab) {
                        if state.is_pressed() {
                            self.next_tab();
                        }
                        return;
                    }
                } else if self.modifiers.alt_key() {
                    if let Key::Character(ref s) = logical_key {
                        if let Ok(num) = s.parse::<usize>() {
                            if num >= 1 && num <= 9 && state.is_pressed() {
                                self.switch_tab(num - 1);
                                return;
                            }
                        }
                    }
                }

                // Scrollback navigation shortcuts:
                // Shift+PageUp: Scroll Page Up
                // Shift+PageDown: Scroll Page Down
                if self.modifiers.shift_key() {
                    if *logical_key == Key::Named(NamedKey::PageUp) && state.is_pressed() {
                        if let Some(tab) = self.tabs.get(self.active_tab_index) {
                            tab.terminal.lock().unwrap().scroll_page_up();
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                        return;
                    }
                    if *logical_key == Key::Named(NamedKey::PageDown) && state.is_pressed() {
                        if let Some(tab) = self.tabs.get(self.active_tab_index) {
                            tab.terminal.lock().unwrap().scroll_page_down();
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                        return;
                    }
                }

                // Keyboard height resize shortcuts:
                // Ctrl+Shift+Down: Expand height by 60px
                // Ctrl+Shift+Up: Shrink height by 60px
                // Ctrl+Shift+Enter: Toggle between 40% and 85% height
                if self.modifiers.control_key() && self.modifiers.shift_key() {
                    match logical_key {
                        Key::Named(NamedKey::ArrowDown) => {
                            if state.is_pressed() {
                                let new_h = self.drop_height + 60;
                                self.resize_window_height(new_h);
                            }
                            return;
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if state.is_pressed() {
                                let new_h = self.drop_height.saturating_sub(60);
                                self.resize_window_height(new_h);
                            }
                            return;
                        }
                        Key::Named(NamedKey::Enter) => {
                            if state.is_pressed() {
                                let default_h = (self.monitor_height as f64 * 0.40) as u32;
                                let large_h = (self.monitor_height as f64 * 0.85) as u32;
                                let new_h = if self.drop_height >= large_h - 30 {
                                    default_h
                                } else {
                                    large_h
                                };
                                self.resize_window_height(new_h);
                            }
                            return;
                        }
                        _ => {}
                    }
                }

                // Route keystrokes to active Tab PTY
                let mut routed_to_pty = false;
                if let Some(input_bytes) =
                    handle_key_input(logical_key, text.as_deref(), state, self.modifiers)
                {
                    if let Some(tab) = self.tabs.get(self.active_tab_index) {
                        tab.terminal.lock().unwrap().scroll_to_bottom();
                        if let Err(e) = tab.pty.write(&input_bytes) {
                            tracing::error!("Failed to write to active PTY: {:?}", e);
                        } else {
                            routed_to_pty = true;
                        }
                    }
                }

                // If input was routed to PTY, PtyOutput will trigger the redraw when terminal state updates.
                // Otherwise redraw immediately (e.g. to ensure solid cursor state).
                if !routed_to_pty {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CloseRequested => {
                tracing::info!("Close requested, exiting DROP");
                event_loop.exit();
            }
            _ => {}
        }
    }
}
