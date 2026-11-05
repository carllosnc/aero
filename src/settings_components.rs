use egui::{
    vec2, Align, Align2, Color32, Frame, Layout, Margin, Response, RichText, Rounding, Sense, Shadow, Stroke, Ui,
};

use crate::config::ThemePalette;
use crate::icons;
use crate::settings_ui::SettingsTab;

/// Premium Neutral Carbon / Zinc Design Tokens
/// Inspired by Linear, Raycast, Vercel & modern neutral dark interfaces.
#[allow(dead_code)]
pub mod tokens {
    use egui::Color32;

    // Canvas & Surface Backgrounds (Pure Neutral Zinc / Carbon)
    pub const BG_WINDOW: Color32 = Color32::from_rgb(14, 14, 16);       // #0e0e10 - Deep Carbon Base
    pub const BG_SURFACE: Color32 = Color32::from_rgb(20, 20, 23);      // #141417 - Neutral Inset Surface
    pub const BG_CARD: Color32 = Color32::from_rgb(24, 24, 27);         // #18181b - Zinc 900 Card
    pub const BG_INPUT: Color32 = Color32::from_rgb(18, 18, 20);        // #121214 - Inset Sunken Input
    pub const BG_TERMINAL_BAR: Color32 = Color32::from_rgb(26, 26, 30); // #1a1a1e - Neutral Terminal Header
    pub const BG_HOVER: Color32 = Color32::from_rgb(39, 39, 42);        // #27272a - Zinc 800 Hover / Active

    // Neutral Hairline Borders & Dividers
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(39, 39, 42);   // #27272a - Zinc 800 1px Hairline
    pub const BORDER_LIGHT: Color32 = Color32::from_rgb(63, 63, 70);    // #3f3f46 - Zinc 700 Elevated Border
    pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(113, 113, 122);// #71717a - Zinc 500 Focus Ring

    // Pure Neutral Typography & Content Hierarchy
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 244, 245); // #f4f4f5 - Zinc 100 Crisp White
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(161, 161, 170);// #a1a1aa - Zinc 400 Clean Silver
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(113, 113, 122);   // #71717a - Zinc 500 Graphite
    pub const TEXT_CODE: Color32 = Color32::from_rgb(228, 228, 231);    // #e4e4e7 - Zinc 200 Code / Chips

    // Refined Minimal Semantic Accents
    pub const ACCENT_PRIMARY: Color32 = Color32::from_rgb(244, 244, 245); // Pure High-Contrast White
    pub const ACCENT_INDIGO: Color32 = Color32::from_rgb(129, 140, 248);  // Indigo #818cf8
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);     // Sky Cyan #38bdf8
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(74, 222, 128);    // Emerald / Green 400 (#4ade80)
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(96, 165, 250);     // Sky / Blue 400 (#60a5fa)
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(192, 132, 252);  // Violet / Purple 400 (#c084fc)
    pub const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);    // Amber 400 (#fbbf24)
    pub const ACCENT_RED: Color32 = Color32::from_rgb(248, 113, 113);     // Rose 400 (#f87171)
}

/// Configure high-quality Inter & JetBrains Mono fonts into egui
pub fn setup_custom_fonts(fonts: &mut egui::FontDefinitions) {
    // 1. Embed Inter Regular, Medium and SemiBold
    fonts.font_data.insert(
        "Inter-Regular".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-Medium".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Medium.ttf"
        ))),
    );
    fonts.font_data.insert(
        "Inter-SemiBold".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );

    // 2. Embed JetBrains Mono
    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono.ttf"
        ))),
    );

    // 3. Make Inter the default proportional UI font
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        family.insert(0, "Inter-Regular".to_owned());
        family.insert(1, "Inter-Medium".to_owned());
        family.insert(2, "Inter-SemiBold".to_owned());
    }

    // 4. Make JetBrains Mono the default monospace/code font
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.insert(0, "JetBrainsMono".to_owned());
    }
}

/// 1. Window Title Bar Component
/// Displays application identity (Aero icon & breadcrumb) plus OS window controls.
#[allow(dead_code)]
pub fn window_title_bar(ui: &mut Ui, title: &str, subtitle: &str, on_close: &mut bool) {
    let titlebar_frame = Frame {
        inner_margin: Margin::symmetric(14.0_f32, 8.0_f32),
        outer_margin: Margin::ZERO,
        rounding: Rounding::ZERO,
        shadow: Shadow::NONE,
        fill: tokens::BG_SURFACE,
        stroke: Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
    };

    titlebar_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // App Identity with Terminal Prompt Icon
            let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
            ui.painter().rect_filled(rect, Rounding::same(4.0), tokens::ACCENT_INDIGO);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                ">",
                egui::FontId::monospace(11.0),
                Color32::WHITE,
            );

            ui.add_space(4.0);

            // Breadcrumb: Configurações // Settings
            ui.label(
                RichText::new(title)
                    .size(12.0)
                    .color(tokens::TEXT_PRIMARY)
                    .strong(),
            );
            ui.label(
                RichText::new("//")
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            );
            ui.label(
                RichText::new(subtitle)
                    .size(11.5)
                    .color(tokens::TEXT_SECONDARY),
            );

            // Right-aligned Window Controls
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let close_btn = egui::Button::new(
                    RichText::new(egui_phosphor::regular::X)
                        .size(12.0)
                        .color(tokens::TEXT_SECONDARY),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .rounding(Rounding::same(4.0))
                .min_size(vec2(24.0, 20.0));

                let resp = ui.add(close_btn);
                if resp.hovered() {
                    ui.painter().rect_filled(
                        resp.rect,
                        Rounding::same(4.0),
                        Color32::from_rgb(225, 29, 72),
                    );
                }
                if resp.clicked() {
                    *on_close = true;
                }
            });
        });
    });
}

/// 2. Header Section Component
/// Renders the main window title, pulsing version badge, and subtitle.
pub fn header_section(ui: &mut Ui, title: &str, badge_text: &str, description: &str) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new(title)
                    .size(20.0)
                    .color(tokens::TEXT_PRIMARY)
                    .strong(),
            );
            ui.add_space(8.0);

            // Refined Neutral Version Badge
            let badge_frame = Frame {
                inner_margin: Margin::symmetric(8.0_f32, 3.0_f32),
                outer_margin: Margin::ZERO,
                rounding: Rounding::same(12.0_f32),
                shadow: Shadow::NONE,
                fill: tokens::BG_CARD,
                stroke: Stroke::new(1.0_f32, tokens::BORDER_LIGHT),
            };
            badge_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(5.0, 0.0);
                    // Glowing Emerald Dot
                    let (dot_rect, _) = ui.allocate_exact_size(vec2(6.0, 6.0), Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 3.0, tokens::ACCENT_GREEN);

                    ui.label(
                        RichText::new(badge_text)
                            .size(11.0)
                            .color(tokens::TEXT_CODE)
                            .strong(),
                    );
                });
            });
        });

        ui.add_space(3.0);
        ui.label(
            RichText::new(description)
                .size(12.0)
                .color(tokens::TEXT_SECONDARY),
        );
    });
}

/// 3. Segmented Navigation Tab Bar Component
/// Renders the 4 navigation tabs (Aparência, Tipografia, Espaçamento, Shell)
/// with Phosphor vector icons, pill shapes, and subtle active borders.
pub fn tab_navigation(ui: &mut Ui, current_tab: SettingsTab) -> Option<SettingsTab> {
    let mut selected = None;

    let container_frame = Frame {
        inner_margin: Margin::same(4.0_f32),
        outer_margin: Margin::symmetric(0.0_f32, 2.0_f32),
        rounding: Rounding::same(8.0_f32),
        shadow: Shadow::NONE,
        fill: tokens::BG_SURFACE,
        stroke: Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
    };

    container_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
            let tabs = [
                (
                    SettingsTab::Appearance,
                    icons::tabs::APPEARANCE,
                    "Aparência",
                ),
                (
                    SettingsTab::Typography,
                    icons::tabs::TYPOGRAPHY,
                    "Tipografia",
                ),
                (
                    SettingsTab::Layout,
                    icons::tabs::SPACING,
                    "Espaçamento",
                ),
                (
                    SettingsTab::Shell,
                    icons::tabs::SHELL,
                    "Shell",
                ),
            ];

            for (tab, icon, label) in tabs {
                let is_active = current_tab == tab;
                let bg = if is_active {
                    tokens::BG_HOVER
                } else {
                    tokens::BG_CARD
                };
                let text_color = if is_active {
                    tokens::TEXT_PRIMARY
                } else {
                    tokens::TEXT_SECONDARY
                };
                let stroke = if is_active {
                    Stroke::new(1.0_f32, tokens::BORDER_LIGHT)
                } else {
                    Stroke::new(1.0_f32, tokens::BORDER_SUBTLE)
                };

                let btn = egui::Button::new(
                    RichText::new(format!("{}  {}", icon, label))
                        .size(12.0)
                        .color(text_color)
                        .strong(),
                )
                .fill(bg)
                .stroke(stroke)
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(120.0, 30.0));

                if ui.add(btn).clicked() {
                    selected = Some(tab);
                }
            }
        });
    });

    selected
}

/// 4. Section Header Component
/// Standardized section title + subtitle used across all setting tabs.
pub fn section_header(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(title)
                .size(14.5)
                .color(tokens::TEXT_PRIMARY)
                .strong(),
        );
        ui.label(
            RichText::new(subtitle)
                .size(11.5)
                .color(tokens::TEXT_SECONDARY),
        );
    });
}

/// 5. Reusable Container Card Component
/// Used to house distinct settings groups with 1px border and subtle dark background.
pub fn card_container<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let card_frame = Frame {
        inner_margin: Margin::same(16.0_f32),
        outer_margin: Margin::symmetric(0.0_f32, 4.0_f32),
        rounding: Rounding::same(10.0_f32),
        shadow: Shadow::NONE,
        fill: tokens::BG_CARD,
        stroke: Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
    };

    card_frame.show(ui, add_contents).inner
}

/// 6. Search Filter Input Component
/// Styled search input with magnifying glass and clear trigger.
pub fn search_input(ui: &mut Ui, text: &mut String, placeholder: &str, width: f32) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(icons::actions::SEARCH)
                .size(13.0)
                .color(tokens::TEXT_MUTED),
        );
        let resp = ui.add(
            egui::TextEdit::singleline(text)
                .hint_text(placeholder)
                .desired_width(width),
        );
        if resp.changed() {
            changed = true;
        }
        if !text.is_empty() {
            let clear_btn = egui::Button::new(
                RichText::new(icons::actions::CLEAR)
                    .size(11.0)
                    .color(tokens::TEXT_MUTED),
            )
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(vec2(16.0, 16.0));

            if ui.add(clear_btn).clicked() {
                text.clear();
                changed = true;
            }
        }
    });
    changed
}

/// 7. Metadata Pill Badge Component
pub fn pill_badge(ui: &mut Ui, text: &str, bg: Color32, fg: Color32, border: Color32) {
    Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .size(10.5)
                    .color(fg)
                    .strong(),
            );
        });
}

/// 8. Color Swatch Component with hover tooltip and border
pub fn color_swatch(ui: &mut Ui, rgb: [u8; 3], size: f32, rounding: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    ui.painter().rect_filled(rect, Rounding::same(rounding), color);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(rounding),
        Stroke::new(1.0_f32, Color32::from_white_alpha(35)),
    );

    let hex_tooltip = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
    resp.on_hover_text(hex_tooltip)
}

/// 9. Color Swatches Row Component
pub fn color_swatches_row(ui: &mut Ui, colors: &[[u8; 3]], swatch_size: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = vec2(5.0, 0.0);
        for &c in colors {
            color_swatch(ui, c, swatch_size, 5.0);
        }
    });
}

/// 10. High-Fidelity Mini Terminal Preview Card
/// Directly matches the Stitch design screen:
/// Traffic lights, window title `terminal preview • zsh (80x24)`,
/// syntax-highlighted prompt, compiling status, finished indicator, and pulsing cursor.
pub fn terminal_preview(
    ui: &mut Ui,
    theme: &ThemePalette,
    command: &str,
    status_output: &[(&str, Color32)],
) {
    let preview_bg = Color32::from_rgb(
        (theme.bg[0] * 255.0) as u8,
        (theme.bg[1] * 255.0) as u8,
        (theme.bg[2] * 255.0) as u8,
    );
    let preview_fg = Color32::from_rgb(
        theme.fg_color[0],
        theme.fg_color[1],
        theme.fg_color[2],
    );
    let accent_color = Color32::from_rgb(
        (theme.accent[0] * 255.0) as u8,
        (theme.accent[1] * 255.0) as u8,
        (theme.accent[2] * 255.0) as u8,
    );

    let terminal_frame = Frame {
        inner_margin: Margin::ZERO,
        outer_margin: Margin::symmetric(0.0_f32, 2.0_f32),
        rounding: Rounding::same(8.0_f32),
        shadow: Shadow::NONE,
        fill: preview_bg,
        stroke: Stroke::new(1.0_f32, tokens::BORDER_LIGHT),
    };

    terminal_frame.show(ui, |ui| {
        ui.vertical(|ui| {
            // Header Bar with Window Traffic Lights & Shell Info
            let bar_frame = Frame {
                inner_margin: Margin::symmetric(10.0_f32, 6.0_f32),
                outer_margin: Margin::ZERO,
                rounding: Rounding {
                    nw: 7.0,
                    ne: 7.0,
                    sw: 0.0,
                    se: 0.0,
                },
                shadow: Shadow::NONE,
                fill: tokens::BG_TERMINAL_BAR,
                stroke: Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
            };

            bar_frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Traffic Lights (Red, Yellow, Green)
                    let (red_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(red_rect.center(), 4.0, Color32::from_rgb(255, 85, 85));

                    let (yel_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(yel_rect.center(), 4.0, Color32::from_rgb(241, 250, 140));

                    let (grn_rect, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().circle_filled(grn_rect.center(), 4.0, Color32::from_rgb(80, 250, 123));

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("terminal preview • zsh")
                            .size(11.0)
                            .color(tokens::TEXT_MUTED),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new("80x24")
                                .monospace()
                                .size(10.0)
                                .color(tokens::TEXT_MUTED),
                        );
                    });
                });
            });

            // Terminal Shell Body
            let body_frame = Frame {
                inner_margin: Margin::symmetric(14.0_f32, 12.0_f32),
                outer_margin: Margin::ZERO,
                rounding: Rounding {
                    nw: 0.0,
                    ne: 0.0,
                    sw: 7.0,
                    se: 7.0,
                },
                shadow: Shadow::NONE,
                fill: preview_bg,
                stroke: Stroke::NONE,
            };

            body_frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    // 1. Prompt Line
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        ui.label(
                            RichText::new("user@aero")
                                .monospace()
                                .size(12.0)
                                .color(accent_color)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(":")
                                .monospace()
                                .size(12.0)
                                .color(tokens::ACCENT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("~")
                                .monospace()
                                .size(12.0)
                                .color(tokens::ACCENT_CYAN),
                        );
                        ui.label(
                            RichText::new("$ ")
                                .monospace()
                                .size(12.0)
                                .color(tokens::TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new(command)
                                .monospace()
                                .size(12.0)
                                .color(preview_fg)
                                .strong(),
                        );
                    });

                    ui.add_space(4.0);

                    // 2. Command Output Lines (indented with subtle border)
                    ui.horizontal(|ui| {
                        // Left vertical accent bar
                        let (bar_rect, _) = ui.allocate_exact_size(vec2(2.0, 36.0), Sense::hover());
                        ui.painter().rect_filled(bar_rect, Rounding::same(1.0), tokens::BORDER_LIGHT);

                        ui.vertical(|ui| {
                            for &(text, color) in status_output {
                                ui.label(
                                    RichText::new(text)
                                        .monospace()
                                        .size(11.5)
                                        .color(color),
                                );
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // 3. Second Prompt Line with Pulsing Cursor
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        ui.label(
                            RichText::new("user@aero")
                                .monospace()
                                .size(12.0)
                                .color(accent_color)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(":")
                                .monospace()
                                .size(12.0)
                                .color(tokens::ACCENT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("~")
                                .monospace()
                                .size(12.0)
                                .color(tokens::ACCENT_CYAN),
                        );
                        ui.label(
                            RichText::new("$ ")
                                .monospace()
                                .size(12.0)
                                .color(tokens::TEXT_MUTED),
                        );

                        // Block cursor
                        let (cursor_rect, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                        ui.painter().rect_filled(
                            cursor_rect,
                            Rounding::same(1.0),
                            accent_color,
                        );
                    });
                });
            });
        });
    });
}

/// 11. Footer Section Component
/// Renders status persistence ("Salvo automaticamente em drop_config.json"),
/// "Restaurar Padrões" secondary button, and "Fechar (Esc)" primary button.
pub fn footer_section(
    ui: &mut Ui,
    config_file: &str,
    mut on_restore_defaults: impl FnMut(),
    on_close: &mut bool,
) -> bool {
    let mut config_changed = false;

    let footer_frame = Frame {
        inner_margin: Margin::symmetric(14.0_f32, 10.0_f32),
        outer_margin: Margin::ZERO,
        rounding: Rounding::ZERO,
        shadow: Shadow::NONE,
        fill: tokens::BG_SURFACE,
        stroke: Stroke::new(1.0_f32, tokens::BORDER_SUBTLE),
    };

    footer_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            // Auto-save Status Indicator
            ui.label(
                RichText::new(icons::status::SUCCESS)
                    .size(13.0)
                    .color(tokens::ACCENT_GREEN),
            );
            ui.label(
                RichText::new("Salvo automaticamente em")
                    .size(11.5)
                    .color(tokens::TEXT_SECONDARY),
            );

            // Code Chip
            pill_badge(
                ui,
                config_file,
                tokens::BG_INPUT,
                tokens::TEXT_CODE,
                tokens::BORDER_SUBTLE,
            );

            // Action Buttons aligned to the right
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Primary: Fechar (Esc) - High contrast solid button
                let close_btn = egui::Button::new(
                    RichText::new(format!("{}  Fechar (Esc)", icons::actions::CLOSE))
                        .size(12.0)
                        .color(tokens::BG_WINDOW)
                        .strong(),
                )
                .fill(tokens::TEXT_PRIMARY)
                .stroke(Stroke::new(1.0_f32, tokens::TEXT_PRIMARY))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(105.0, 30.0));

                if ui.add(close_btn).clicked() {
                    *on_close = true;
                }

                // Secondary: Restaurar Padrões - Neutral card button
                let reset_btn = egui::Button::new(
                    RichText::new(format!(
                        "{}  Restaurar Padrões",
                        icons::actions::RESET
                    ))
                    .size(11.5)
                    .color(tokens::TEXT_SECONDARY),
                )
                .fill(tokens::BG_CARD)
                .stroke(Stroke::new(1.0_f32, tokens::BORDER_SUBTLE))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(140.0, 30.0));

                if ui.add(reset_btn).clicked() {
                    on_restore_defaults();
                    config_changed = true;
                }
            });
        });
    });

    config_changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokens_consistency() {
        assert_ne!(tokens::BG_WINDOW, tokens::TEXT_PRIMARY);
        assert_eq!(tokens::BG_WINDOW, Color32::from_rgb(14, 14, 16));
        assert_eq!(tokens::TEXT_PRIMARY, Color32::from_rgb(244, 244, 245));
    }

    #[test]
    fn test_components_in_egui_context() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut close = false;
                window_title_bar(ui, "Configurações", "Settings", &mut close);
                header_section(ui, "Title", "Badge", "Description");
                let tab = tab_navigation(ui, SettingsTab::Appearance);
                assert!(tab.is_none());
                section_header(ui, "Section", "Subtitle");

                card_container(ui, |ui| {
                    pill_badge(ui, "Test", tokens::BG_INPUT, tokens::TEXT_PRIMARY, tokens::BORDER_SUBTLE);
                    color_swatches_row(ui, &[[255, 0, 0], [0, 255, 0]], 16.0);
                });

                let mut search = String::from("cat");
                let _ = search_input(ui, &mut search, "Hint", 100.0);

                let mut dummy_reset = false;
                let _ = footer_section(ui, "config.json", || dummy_reset = true, &mut close);
            });
        });
    }
}

