use std::sync::Arc;
use windows::Win32::Foundation::HWND;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window as WinitWindow;

use crate::config::{AppConfig, THEMES};
use crate::dwm::apply_dwm_effects;
use crate::settings_components::{
    card_container, color_swatches_row, footer_section, header_section, pill_badge,
    search_input, section_header, setup_custom_fonts, tab_navigation, terminal_preview, tokens,
};
use egui::{
    vec2, Align, Color32, Frame, Layout, Margin, RichText, Rounding, ScrollArea,
    Sense, Shadow, Slider, Stroke, Ui,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Typography,
    Layout,
    Shell,
}

#[derive(Debug, Clone)]
pub struct DetectedShell {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub tag: String,
    pub is_available: bool,
}

pub struct SettingsUi {
    pub selected_tab: SettingsTab,
    pub theme_search: String,
    pub font_search: String,
    pub shell_search: String,
    pub system_fonts: Vec<(String, bool)>, // (family_name, is_monospace)
    pub detected_shells: Vec<DetectedShell>,
}

impl SettingsUi {
    pub fn new() -> Self {
        // Enumerate all system fonts installed on the user's OS via fontdb
        let mut db = glyphon::fontdb::Database::new();
        db.load_system_fonts();
        let mut system_fonts = Vec::new();
        for face in db.faces() {
            if let Some((name, _)) = face.families.first() {
                if !system_fonts.iter().any(|(n, _): &(String, bool)| n == name) {
                    system_fonts.push((name.clone(), face.monospaced));
                }
            }
        }

        // Sort monospace first, then alphabetically
        system_fonts.sort_by(|a, b| {
            match (a.1, b.1) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
            }
        });

        if system_fonts.is_empty() {
            system_fonts = vec![
                ("Cascadia Code".to_string(), true),
                ("Consolas".to_string(), true),
                ("Fira Code".to_string(), true),
                ("JetBrains Mono".to_string(), true),
                ("Courier New".to_string(), true),
            ];
        }

        let detected_shells = detect_system_shells();

        Self {
            selected_tab: SettingsTab::Appearance,
            theme_search: String::new(),
            font_search: String::new(),
            shell_search: String::new(),
            system_fonts,
            detected_shells,
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

        let frame = Frame {
            inner_margin: Margin::ZERO,
            outer_margin: Margin::ZERO,
            rounding: Rounding::ZERO,
            shadow: Shadow::NONE,
            fill: tokens::BG_WINDOW,
            stroke: Stroke::NONE,
        };

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            // Main Content Area with padding
            let content_frame = Frame {
                inner_margin: Margin::symmetric(22.0_f32, 16.0_f32),
                outer_margin: Margin::ZERO,
                rounding: Rounding::ZERO,
                shadow: Shadow::NONE,
                fill: Color32::TRANSPARENT,
                stroke: Stroke::NONE,
            };

            content_frame.show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 10.0);

                // Header Section with Title, Pulse Badge and Subtitle
                header_section(
                    ui,
                    "Preferências do Terminal",
                    "Aero v0.1.0",
                    "Personalize o tema visual, todas as fontes do sistema, espaçamento e interpretador de comandos.",
                );

                ui.add_space(4.0);

                // Segmented Navigation Tab Bar
                if let Some(tab) = tab_navigation(ui, self.selected_tab) {
                    self.selected_tab = tab;
                }

                ui.add_space(3.0);
                ui.separator();
                ui.add_space(3.0);

                // Scrollable View for Tab Content
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
                            SettingsTab::Shell => {
                                self.render_shell_tab(ui, config, &mut config_changed);
                            }
                        }
                    });
            });

            // 3. Persistent Footer Bar
            let reset_changed = footer_section(
                ui,
                "drop_config.json",
                || {
                    *config = AppConfig::default();
                },
                &mut should_close,
            );
            if reset_changed {
                config_changed = true;
            }
        });

        if should_close {
            *is_open = false;
        }

        config_changed
    }

    fn render_appearance_tab(
        &mut self,
        ui: &mut Ui,
        config: &mut AppConfig,
        config_changed: &mut bool,
    ) {
        // Section Header
        section_header(
            ui,
            "Paleta e Tema de Cores",
            "Selecione entre mais de 15 temas de alto contraste e paletas consagradas.",
        );
        ui.add_space(4.0);

        // Search & Select ComboBox row
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Tema:")
                    .size(13.0)
                    .color(tokens::TEXT_PRIMARY)
                    .strong(),
            );

            search_input(ui, &mut self.theme_search, "Filtrar temas...", 150.0);

            let current_theme = config.theme.clone();
            let query = self.theme_search.trim().to_lowercase();
            let matching_count = THEMES
                .iter()
                .filter(|t| t.name.to_lowercase().contains(&query))
                .count();

            egui::ComboBox::from_id_salt("theme_selector")
                .selected_text(
                    RichText::new(&current_theme)
                        .color(tokens::TEXT_PRIMARY)
                        .strong(),
                )
                .width(220.0)
                .show_ui(ui, |ui| {
                    for theme in THEMES {
                        if !query.is_empty() && !theme.name.to_lowercase().contains(&query) {
                            continue;
                        }

                        let is_current = config.theme == theme.name;
                        let label_text = format!(
                            "{} {}",
                            if theme.is_light { "☀️" } else { "🌙" },
                            theme.name
                        );

                        ui.horizontal(|ui| {
                            let resp = ui.selectable_label(is_current, label_text);
                            if resp.clicked() {
                                config.theme = theme.name.to_string();
                                *config_changed = true;
                            }
                            for c in &theme.preview_colors[..3] {
                                draw_u8_swatch(ui, *c, 10.0);
                            }
                        });
                    }
                });

            ui.label(
                RichText::new(format!("({} disponíveis)", matching_count))
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            );
        });

        ui.add_space(8.0);

        // Active Theme Showcase & Terminal Preview Card
        let active_theme = THEMES
            .iter()
            .find(|t| t.name == config.theme)
            .unwrap_or(&THEMES[0]);

        card_container(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(active_theme.name)
                                .size(16.0)
                                .color(tokens::TEXT_PRIMARY)
                                .strong(),
                        );
                        let (badge_text, bg_badge, fg_badge, border_badge) = if active_theme.is_light {
                            (
                                "Light Theme",
                                Color32::from_rgba_premultiplied(234, 179, 8, 35),
                                Color32::from_rgb(253, 224, 71),
                                Color32::from_rgba_premultiplied(234, 179, 8, 90),
                            )
                        } else {
                            (
                                "Dark Theme",
                                Color32::from_rgba_premultiplied(189, 147, 249, 35),
                                Color32::from_rgb(216, 180, 254),
                                Color32::from_rgba_premultiplied(189, 147, 249, 90),
                            )
                        };
                        pill_badge(ui, badge_text, bg_badge, fg_badge, border_badge);
                    });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(active_theme.description)
                            .size(11.5)
                            .color(tokens::TEXT_SECONDARY),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    color_swatches_row(ui, &active_theme.preview_colors, 20.0);
                });
            });

            ui.add_space(10.0);

            // High-fidelity terminal emulator preview matching Stitch design
            terminal_preview(
                ui,
                active_theme,
                "cargo check --release",
                &[
                    (
                        "Compiling aero v0.1.0 (d:/Projects/open/aero)",
                        tokens::TEXT_MUTED,
                    ),
                    (
                        "Finished `release` profile [optimized] in 0.42s",
                        tokens::ACCENT_GREEN,
                    ),
                ],
            );
        });
    }

    fn render_typography_tab(
        &mut self,
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
            RichText::new("Selecione entre todas as fontes instaladas no sistema operacional ou filtre por fontes mono.")
                .size(11.5)
                .color(Color32::from_rgb(140, 150, 170)),
        );
        ui.add_space(6.0);

        // Search & ComboBox
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} Fonte:", egui_phosphor::regular::TEXT_AA))
                    .size(13.0)
                    .color(Color32::from_rgb(210, 220, 235))
                    .strong(),
            );

            ui.add(
                egui::TextEdit::singleline(&mut self.font_search)
                    .hint_text("Filtrar fontes do sistema...")
                    .desired_width(170.0),
            );

            let query = self.font_search.trim().to_lowercase();
            let current_font = config.font_family.clone();

            egui::ComboBox::from_id_salt("font_selector")
                .selected_text(RichText::new(&current_font).color(Color32::WHITE).strong())
                .width(240.0)
                .show_ui(ui, |ui| {
                    let mut count = 0;
                    for (font_name, is_mono) in &self.system_fonts {
                        if !query.is_empty() && !font_name.to_lowercase().contains(&query) {
                            continue;
                        }
                        if count > 120 {
                            ui.label(
                                RichText::new("... (refine a busca para mais resultados)")
                                    .size(11.0)
                                    .color(Color32::from_rgb(120, 130, 150)),
                            );
                            break;
                        }
                        count += 1;

                        let is_current = config.font_family == *font_name;
                        let display_label = if *is_mono {
                            format!("{} [Mono]", font_name)
                        } else {
                            font_name.clone()
                        };

                        if ui.selectable_label(is_current, display_label).clicked() {
                            config.font_family = font_name.clone();
                            *config_changed = true;
                        }
                    }
                });
        });

        ui.add_space(8.0);

        // Typography Live Code Preview Box
        let code_box = Frame {
            inner_margin: Margin::same(12.0_f32),
            rounding: Rounding::same(6.0_f32),
            fill: Color32::from_rgb(20, 24, 32),
            stroke: Stroke::new(1.0_f32, Color32::from_rgb(38, 45, 60)),
            ..Default::default()
        };

        code_box.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Pré-visualização de Código (Fonte: {} | {:.0}px)", config.font_family, config.font_size))
                        .size(11.0)
                        .color(Color32::from_rgb(110, 125, 150)),
                );
            });
            ui.add_space(4.0);

            ui.label(
                RichText::new("fn main() {\n    let items = vec![\"aero\", \"fast\", \"gpu\"];\n    println!(\"Running at 120 FPS! => {:?}\", items);\n    // != <= >= == && || ->\n}")
                    .size(config.font_size.clamp(11.0, 18.0))
                    .color(Color32::from_rgb(190, 215, 245)),
            );
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        // Font Size Slider + Presets
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
            Slider::new(&mut config.font_size, 10.0..=24.0)
                .step_by(1.0)
                .suffix(" px")
                .show_value(false),
        );
        if (config.font_size - old_size).abs() > 0.01 {
            *config_changed = true;
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Presets rápidos:")
                    .size(12.0)
                    .color(Color32::from_rgb(130, 140, 160)),
            );
            for sz in [12.0, 13.0, 14.0, 15.0, 16.0, 18.0] {
                let is_sel = (config.font_size - sz).abs() < 0.5;
                let btn = egui::Button::new(
                    RichText::new(format!("{:.0}px", sz))
                        .size(11.5)
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

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

        // Cursor style
        ui.label(
            RichText::new("Estilo do Cursor")
                .size(13.5)
                .color(Color32::from_rgb(230, 235, 245))
                .strong(),
        );
        ui.horizontal(|ui| {
            for (style_key, style_label) in [("bar", "Barra Vertical (|)"), ("block", "Bloco Sólido (█)")] {
                let is_sel = config.cursor_style == style_key;
                let btn = egui::Button::new(
                    RichText::new(style_label)
                        .size(12.0)
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
        &mut self,
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
                .size(11.5)
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
                        .size(11.5)
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

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);

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
                        .size(11.5)
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

    fn render_shell_tab(
        &mut self,
        ui: &mut Ui,
        config: &mut AppConfig,
        config_changed: &mut bool,
    ) {
        ui.label(
            RichText::new("Shell e Interpretador de Comandos")
                .size(14.0)
                .color(Color32::from_rgb(230, 235, 245))
                .strong(),
        );
        ui.label(
            RichText::new("Selecione entre os shells instalados no seu sistema ou especifique um executável customizado.")
                .size(11.5)
                .color(Color32::from_rgb(140, 150, 170)),
        );
        ui.add_space(8.0);

        // Find current shell display name
        let current_shell_info = self
            .detected_shells
            .iter()
            .find(|s| s.executable.eq_ignore_ascii_case(&config.shell) || s.id.eq_ignore_ascii_case(&config.shell))
            .map(|s| s.name.clone())
            .unwrap_or_else(|| config.shell.clone());

        // ComboBox for Shell Selection
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} Shell Padrão:", egui_phosphor::regular::TERMINAL_WINDOW))
                    .size(13.0)
                    .color(Color32::from_rgb(210, 220, 235))
                    .strong(),
            );

            ui.add(
                egui::TextEdit::singleline(&mut self.shell_search)
                    .hint_text("Filtrar shells...")
                    .desired_width(140.0),
            );

            let query = self.shell_search.trim().to_lowercase();

            egui::ComboBox::from_id_salt("shell_selector")
                .selected_text(RichText::new(&current_shell_info).color(Color32::WHITE).strong())
                .width(260.0)
                .show_ui(ui, |ui| {
                    for shell in &self.detected_shells {
                        if !query.is_empty()
                            && !shell.name.to_lowercase().contains(&query)
                            && !shell.executable.to_lowercase().contains(&query)
                        {
                            continue;
                        }

                        let is_active = config.shell.eq_ignore_ascii_case(&shell.executable)
                            || config.shell.eq_ignore_ascii_case(&shell.id);

                        let avail_icon = if shell.is_available { "✓" } else { "○" };
                        let label_text = format!("{} {} ({})", avail_icon, shell.name, shell.tag);

                        if ui.selectable_label(is_active, label_text).clicked() {
                            config.shell = shell.executable.clone();
                            *config_changed = true;
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // List detected shells with details
        for shell in &self.detected_shells {
            let is_sel = config.shell.eq_ignore_ascii_case(&shell.executable)
                || config.shell.eq_ignore_ascii_case(&shell.id);

            let bg = if is_sel {
                Color32::from_rgb(32, 42, 60)
            } else {
                Color32::from_rgb(22, 26, 36)
            };
            let stroke = if is_sel {
                Stroke::new(1.2_f32, Color32::from_rgb(60, 130, 246))
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(35, 41, 55))
            };

            let frame = Frame {
                inner_margin: Margin::symmetric(12.0_f32, 8.0_f32),
                outer_margin: Margin::symmetric(0.0_f32, 2.0_f32),
                rounding: Rounding::same(6.0_f32),
                fill: bg,
                stroke,
                ..Default::default()
            };

            let resp = frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&shell.name)
                                    .size(13.0)
                                    .color(if is_sel { Color32::WHITE } else { Color32::from_rgb(205, 215, 230) })
                                    .strong(),
                            );
                            if shell.is_available {
                                Frame::none()
                                    .fill(Color32::from_rgb(28, 60, 42))
                                    .rounding(Rounding::same(3.0))
                                    .inner_margin(Margin::symmetric(4.0, 1.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Disponível").size(10.0).color(Color32::from_rgb(80, 220, 120)));
                                    });
                            } else {
                                Frame::none()
                                    .fill(Color32::from_rgb(45, 45, 55))
                                    .rounding(Rounding::same(3.0))
                                    .inner_margin(Margin::symmetric(4.0, 1.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Não detectado").size(10.0).color(Color32::from_rgb(140, 145, 160)));
                                    });
                            }
                        });
                        ui.label(
                            RichText::new(format!("Executável: {} • {}", shell.executable, shell.tag))
                                .size(11.0)
                                .color(Color32::from_rgb(130, 140, 160)),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if is_sel {
                            ui.label(
                                RichText::new(format!("{} Ativo", egui_phosphor::regular::CHECK))
                                    .size(12.0)
                                    .color(Color32::from_rgb(60, 140, 255))
                                    .strong(),
                            );
                        }
                    });
                });
            }).response;

            if resp.interact(Sense::click()).clicked() && !is_sel {
                config.shell = shell.executable.clone();
                *config_changed = true;
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);

        ui.label(
            RichText::new("Comando ou Executável Customizado:")
                .size(13.0)
                .color(Color32::from_rgb(220, 230, 245))
                .strong(),
        );

        let mut custom_shell = config.shell.clone();
        let edit = ui.add(
            egui::TextEdit::singleline(&mut custom_shell)
                .hint_text("ex: pwsh.exe, nu.exe, C:\\msys64\\usr\\bin\\zsh.exe")
                .min_size(vec2(ui.available_width(), 26.0)),
        );
        if edit.changed() {
            config.shell = custom_shell;
            *config_changed = true;
        }
    }
}

pub fn draw_u8_swatch(ui: &mut Ui, rgb: [u8; 3], size: f32) {
    let (rect, _response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    ui.painter().rect_filled(rect, Rounding::same(3.0), color);
    ui.painter().rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0_f32, Color32::from_white_alpha(35)));
}

fn check_executable_exists(name: &str) -> bool {
    let path = std::path::Path::new(name);
    if path.is_file() {
        return true;
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for p in std::env::split_paths(&path_var) {
            if p.join(name).is_file() {
                return true;
            }
        }
    }
    false
}

fn find_git_bash() -> Option<String> {
    let candidates = [
        r"C:\Program Files\Git\bin\bash.exe",
        r"C:\Program Files (x86)\Git\bin\bash.exe",
    ];
    for path in &candidates {
        if std::path::Path::new(path).is_file() {
            return Some((*path).to_string());
        }
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let p = std::path::Path::new(&local_app_data).join(r"Programs\Git\bin\bash.exe");
        if p.is_file() {
            return Some(p.to_string_lossy().to_string());
        }
    }
    if check_executable_exists("bash.exe") {
        return Some("bash.exe".to_string());
    }
    None
}

pub fn detect_system_shells() -> Vec<DetectedShell> {
    let mut shells = Vec::new();

    // 1. PowerShell 7
    let pwsh_available = check_executable_exists("pwsh.exe");
    shells.push(DetectedShell {
        id: "pwsh".to_string(),
        name: "PowerShell 7 (pwsh)".to_string(),
        executable: "pwsh.exe".to_string(),
        tag: "Moderno & Rápido".to_string(),
        is_available: pwsh_available,
    });

    // 2. Windows PowerShell
    let ps_available = check_executable_exists("powershell.exe");
    shells.push(DetectedShell {
        id: "powershell".to_string(),
        name: "Windows PowerShell 5.1".to_string(),
        executable: "powershell.exe".to_string(),
        tag: "Nativo do Windows".to_string(),
        is_available: ps_available,
    });

    // 3. Command Prompt
    let cmd_available = check_executable_exists("cmd.exe");
    shells.push(DetectedShell {
        id: "cmd".to_string(),
        name: "Prompt de Comando (CMD)".to_string(),
        executable: "cmd.exe".to_string(),
        tag: "Clássico".to_string(),
        is_available: cmd_available,
    });

    // 4. Git Bash
    let git_bash = find_git_bash();
    let bash_available = git_bash.is_some();
    shells.push(DetectedShell {
        id: "bash".to_string(),
        name: "Git Bash".to_string(),
        executable: git_bash.unwrap_or_else(|| "bash.exe".to_string()),
        tag: "Ambiente Unix / MinGW".to_string(),
        is_available: bash_available,
    });

    // 5. WSL
    let wsl_available = check_executable_exists("wsl.exe");
    shells.push(DetectedShell {
        id: "wsl".to_string(),
        name: "WSL (Linux)".to_string(),
        executable: "wsl.exe".to_string(),
        tag: "Subsistema Linux".to_string(),
        is_available: wsl_available,
    });

    // 6. Nushell
    let nu_available = check_executable_exists("nu.exe");
    if nu_available {
        shells.push(DetectedShell {
            id: "nu".to_string(),
            name: "Nushell".to_string(),
            executable: "nu.exe".to_string(),
            tag: "Estruturado / Moderno".to_string(),
            is_available: true,
        });
    }

    shells
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
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        setup_custom_fonts(&mut fonts);
        egui_ctx.set_fonts(fonts);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = tokens::BG_WINDOW;
        visuals.window_fill = tokens::BG_WINDOW;
        visuals.extreme_bg_color = tokens::BG_INPUT;
        visuals.widgets.noninteractive.bg_fill = tokens::BG_CARD;
        visuals.widgets.inactive.bg_fill = tokens::BG_SURFACE;
        visuals.widgets.hovered.bg_fill = tokens::BG_HOVER;
        visuals.widgets.active.bg_fill = tokens::BORDER_LIGHT;
        visuals.widgets.open.bg_fill = tokens::BG_CARD;
        egui_ctx.set_visuals(visuals);

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
                            r: 14.0 / 255.0,
                            g: 14.0 / 255.0,
                            b: 16.0 / 255.0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_system_shells() {
        let shells = detect_system_shells();
        assert!(!shells.is_empty(), "Must detect at least 1 shell");
        // On Windows, cmd or powershell should always be available
        #[cfg(target_os = "windows")]
        {
            let has_any_available = shells.iter().any(|s| s.is_available);
            assert!(has_any_available, "At least one shell should be available on Windows");
        }
    }

    #[test]
    fn test_settings_ui_initialization() {
        let ui = SettingsUi::new();
        assert!(!ui.system_fonts.is_empty(), "System fonts list should not be empty");
        assert!(!ui.detected_shells.is_empty(), "Detected shells should not be empty");
        assert_eq!(ui.selected_tab, SettingsTab::Appearance);
    }
}
