use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Processor};
use glyphon::{Attrs, Buffer, Color as GlyphonColor, Family, FontSystem, Shaping};

struct TermEventListener;
impl EventListener for TermEventListener {
    fn send_event(&self, _event: Event) {}
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollInfo {
    pub display_offset: usize,
    pub total_lines: usize,
    pub screen_lines: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalCellQuad {
    pub col: usize,
    pub row: usize,
    pub cols_count: usize,
    pub color: [f32; 4],
    pub underline: bool,
    pub strikethrough: bool,
    pub fg_color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanAttrs {
    pub color: GlyphonColor,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for TerminalSize {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Terminal {
    term: Term<TermEventListener>,
    processor: Processor,
    pub cols: u16,
    pub rows: u16,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let size = TerminalSize {
            cols: cols as usize,
            rows: rows as usize,
        };
        let config = Config::default();
        let term = Term::new(config, &size, TermEventListener);
        let processor = Processor::new();

        Self {
            term,
            processor,
            cols,
            rows,
        }
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.processor.advance(&mut self.term, byte);
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        let size = TerminalSize {
            cols: cols as usize,
            rows: rows as usize,
        };
        self.term.resize(size);
    }

    pub fn scroll(&mut self, delta: i32) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::Delta(delta));
    }

    pub fn scroll_page_up(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::PageUp);
    }

    pub fn scroll_page_down(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::PageDown);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::Bottom);
    }

    pub fn is_alt_screen(&self) -> bool {
        self.term.mode().contains(alacritty_terminal::term::TermMode::ALT_SCREEN)
    }

    /// Renders terminal grid directly into text buffer without injecting cursor characters,
    /// ensuring characters never shift or get displaced during navigation.
    /// Handles scrollback history, accurate cell background quads, and rich style attributes (bold, italic, underline).
    /// Returns (cursor_column, cursor_row, scroll_info, cell_quads).
    pub fn render_to_buffer(
        &self,
        font_system: &mut FontSystem,
        buffer: &mut Buffer,
        font_family: &str,
        default_color: GlyphonColor,
    ) -> (usize, usize, ScrollInfo, Vec<TerminalCellQuad>) {
        let grid = self.term.grid();
        let screen_lines = grid.screen_lines();
        let columns = grid.columns();
        let display_offset = grid.display_offset();
        let total_lines = grid.total_lines();
        let cursor_point = grid.cursor.point;

        // Calculate cursor position relative to the visible screen lines
        let cursor_screen_row = cursor_point.line.0 + display_offset as i32;
        let cursor_col = cursor_point.column.0;
        let cursor_row = if cursor_screen_row >= 0 && (cursor_screen_row as usize) < screen_lines {
            cursor_screen_row as usize
        } else {
            usize::MAX
        };

        let mut spans: Vec<(String, SpanAttrs)> = Vec::with_capacity(screen_lines * 4);
        let mut cell_quads: Vec<TerminalCellQuad> = Vec::new();

        for line_idx in 0..screen_lines {
            // Index line from grid taking display_offset (scrollback) into account
            let line = Line((line_idx as i32) - (display_offset as i32));
            let mut line_spans: Vec<(String, SpanAttrs)> = Vec::new();
            let mut current_span_attrs = SpanAttrs {
                color: default_color,
                bold: false,
                italic: false,
            };
            let mut current_span_text = String::new();
            let mut current_quad: Option<TerminalCellQuad> = None;

            for col_idx in 0..columns {
                let col = Column(col_idx);
                let cell = &grid[line][col];

                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }

                let is_inverse = cell.flags.contains(Flags::INVERSE);
                let raw_fg = if is_inverse { cell.bg } else { cell.fg };
                let raw_bg = if is_inverse { cell.fg } else { cell.bg };

                let mut cell_fg = ansi_to_glyphon_color(raw_fg, default_color);
                if cell.flags.contains(Flags::DIM) {
                    cell_fg = GlyphonColor::rgb(
                        (cell_fg.r() as f32 * 0.66) as u8,
                        (cell_fg.g() as f32 * 0.66) as u8,
                        (cell_fg.b() as f32 * 0.66) as u8,
                    );
                }

                let bold = cell.flags.contains(Flags::BOLD);
                let italic = cell.flags.contains(Flags::ITALIC);
                let underline = cell.flags.contains(Flags::UNDERLINE);
                let strikethrough = cell.flags.contains(Flags::STRIKEOUT);

                let cell_bg_opt = ansi_to_rgba(raw_bg, default_color, false);
                let fg_rgba = [
                    (cell_fg.r() as f32) / 255.0,
                    (cell_fg.g() as f32) / 255.0,
                    (cell_fg.b() as f32) / 255.0,
                    1.0,
                ];

                // Background & decoration quads
                let has_bg = cell_bg_opt.is_some();
                let bg_color = cell_bg_opt.unwrap_or([0.0, 0.0, 0.0, 0.0]);
                if has_bg || underline || strikethrough {
                    if let Some(ref mut q) = current_quad {
                        if q.color == bg_color && q.underline == underline && q.strikethrough == strikethrough && q.fg_color == fg_rgba {
                            q.cols_count += 1;
                        } else {
                            cell_quads.push(*q);
                            current_quad = Some(TerminalCellQuad {
                                col: col_idx,
                                row: line_idx,
                                cols_count: 1,
                                color: bg_color,
                                underline,
                                strikethrough,
                                fg_color: fg_rgba,
                            });
                        }
                    } else {
                        current_quad = Some(TerminalCellQuad {
                            col: col_idx,
                            row: line_idx,
                            cols_count: 1,
                            color: bg_color,
                            underline,
                            strikethrough,
                            fg_color: fg_rgba,
                        });
                    }
                } else if let Some(q) = current_quad.take() {
                    cell_quads.push(q);
                }

                // Text character
                let ch = if cell.c == '\0' || cell.flags.contains(Flags::HIDDEN) {
                    ' '
                } else {
                    cell.c
                };

                let attrs = SpanAttrs {
                    color: cell_fg,
                    bold,
                    italic,
                };

                if current_span_text.is_empty() {
                    current_span_attrs = attrs;
                } else if attrs != current_span_attrs {
                    line_spans.push((std::mem::take(&mut current_span_text), current_span_attrs));
                    current_span_attrs = attrs;
                }

                current_span_text.push(ch);
            }

            if let Some(q) = current_quad.take() {
                cell_quads.push(q);
            }

            if !current_span_text.is_empty() {
                line_spans.push((current_span_text, current_span_attrs));
            }

            // Trim trailing empty spaces on this row
            trim_trailing_spaces(&mut line_spans);

            // Add newline \n to the end of the line
            if line_idx + 1 < screen_lines {
                if let Some((last_str, _)) = line_spans.last_mut() {
                    last_str.push('\n');
                } else {
                    line_spans.push((
                        String::from("\n"),
                        SpanAttrs {
                            color: default_color,
                            bold: false,
                            italic: false,
                        },
                    ));
                }
            }

            spans.extend(line_spans);
        }

        let resolved = crate::renderer::resolve_monospace_font(font_system, font_family);
        let base_attrs = Attrs::new().family(Family::Name(&resolved));

        if spans.is_empty() {
            buffer.set_text(font_system, "", base_attrs.color(default_color), Shaping::Basic);
        } else {
            let rich_spans: Vec<(&str, Attrs)> = spans
                .iter()
                .map(|(text, attrs)| {
                    let mut a = base_attrs.color(attrs.color);
                    if attrs.bold {
                        a = a.weight(glyphon::Weight::BOLD);
                    }
                    if attrs.italic {
                        a = a.style(glyphon::Style::Italic);
                    }
                    (text.as_str(), a)
                })
                .collect();

            buffer.set_rich_text(font_system, rich_spans, base_attrs.color(default_color), Shaping::Basic);
        }

        buffer.shape_until_scroll(font_system, false);

        let scroll_info = ScrollInfo {
            display_offset,
            total_lines,
            screen_lines,
        };

        (cursor_col, cursor_row, scroll_info, cell_quads)
    }
}

fn trim_trailing_spaces(spans: &mut Vec<(String, SpanAttrs)>) {
    while let Some((last_text, _)) = spans.last_mut() {
        let trimmed_len = last_text.trim_end_matches(' ').len();
        if trimmed_len == 0 {
            spans.pop();
        } else {
            last_text.truncate(trimmed_len);
            break;
        }
    }
}

pub fn ansi_to_glyphon_color(color: AnsiColor, default_color: GlyphonColor) -> GlyphonColor {
    match color {
        AnsiColor::Named(named) => match named {
            NamedColor::Black => GlyphonColor::rgb(30, 30, 35),
            NamedColor::Red => GlyphonColor::rgb(240, 90, 90),
            NamedColor::Green => GlyphonColor::rgb(50, 205, 120),
            NamedColor::Yellow => GlyphonColor::rgb(250, 215, 90),
            NamedColor::Blue => GlyphonColor::rgb(80, 160, 255),
            NamedColor::Magenta => GlyphonColor::rgb(200, 120, 230),
            NamedColor::Cyan => GlyphonColor::rgb(90, 215, 250),
            NamedColor::White => GlyphonColor::rgb(240, 240, 240),
            NamedColor::BrightBlack => GlyphonColor::rgb(140, 140, 140),
            NamedColor::BrightRed => GlyphonColor::rgb(255, 115, 115),
            NamedColor::BrightGreen => GlyphonColor::rgb(120, 220, 160),
            NamedColor::BrightYellow => GlyphonColor::rgb(255, 235, 120),
            NamedColor::BrightBlue => GlyphonColor::rgb(120, 190, 255),
            NamedColor::BrightMagenta => GlyphonColor::rgb(225, 150, 250),
            NamedColor::BrightCyan => GlyphonColor::rgb(140, 230, 255),
            NamedColor::BrightWhite => GlyphonColor::rgb(255, 255, 255),
            NamedColor::Foreground => default_color,
            _ => default_color,
        },
        AnsiColor::Spec(rgb) => GlyphonColor::rgb(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(idx) => indexed_to_glyphon_color(idx, default_color),
    }
}

fn indexed_to_glyphon_color(idx: u8, default_color: GlyphonColor) -> GlyphonColor {
    match idx {
        0..=15 => ansi_to_glyphon_color(AnsiColor::Named(match idx {
            0 => NamedColor::Black,
            1 => NamedColor::Red,
            2 => NamedColor::Green,
            3 => NamedColor::Yellow,
            4 => NamedColor::Blue,
            5 => NamedColor::Magenta,
            6 => NamedColor::Cyan,
            7 => NamedColor::White,
            8 => NamedColor::BrightBlack,
            9 => NamedColor::BrightRed,
            10 => NamedColor::BrightGreen,
            11 => NamedColor::BrightYellow,
            12 => NamedColor::BrightBlue,
            13 => NamedColor::BrightMagenta,
            14 => NamedColor::BrightCyan,
            _ => NamedColor::BrightWhite,
        }), default_color),
        16..=231 => {
            let mut i = idx - 16;
            let b = (i % 6) * 51;
            i /= 6;
            let g = (i % 6) * 51;
            i /= 6;
            let r = (i % 6) * 51;
            GlyphonColor::rgb(r, g, b)
        }
        232..=255 => {
            let gray = (idx - 232) * 10 + 8;
            GlyphonColor::rgb(gray, gray, gray)
        }
    }
}

pub fn ansi_to_rgba(color: AnsiColor, default_color: GlyphonColor, is_fg: bool) -> Option<[f32; 4]> {
    match color {
        AnsiColor::Named(NamedColor::Background) => {
            if is_fg {
                None
            } else {
                None
            }
        }
        AnsiColor::Named(NamedColor::Foreground) => {
            if is_fg {
                Some([
                    (default_color.r() as f32) / 255.0,
                    (default_color.g() as f32) / 255.0,
                    (default_color.b() as f32) / 255.0,
                    1.0,
                ])
            } else {
                None
            }
        }
        AnsiColor::Named(named) => {
            let gc = ansi_to_glyphon_color(AnsiColor::Named(named), default_color);
            Some([
                (gc.r() as f32) / 255.0,
                (gc.g() as f32) / 255.0,
                (gc.b() as f32) / 255.0,
                1.0,
            ])
        }
        AnsiColor::Spec(rgb) => Some([
            (rgb.r as f32) / 255.0,
            (rgb.g as f32) / 255.0,
            (rgb.b as f32) / 255.0,
            1.0,
        ]),
        AnsiColor::Indexed(idx) => {
            let gc = indexed_to_glyphon_color(idx, default_color);
            Some([
                (gc.r() as f32) / 255.0,
                (gc.g() as f32) / 255.0,
                (gc.b() as f32) / 255.0,
                1.0,
            ])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_quads_and_decorations() {
        let mut term = Terminal::new(80, 24);
        // Write text with red background (\x1b[41m) and underline (\x1b[4m)
        term.process_bytes(b"\x1b[41m\x1b[4mError Message\x1b[0m\r\n");

        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, glyphon::Metrics::new(13.0, 19.0));
        let (_col, _row, _scroll, quads) = term.render_to_buffer(
            &mut font_system,
            &mut buffer,
            "Cascadia Code",
            GlyphonColor::rgb(220, 220, 220),
        );

        assert!(!quads.is_empty(), "Should generate background/decoration quads");
        let first_quad = &quads[0];
        assert_eq!(first_quad.row, 0);
        assert_eq!(first_quad.col, 0);
        assert_eq!(first_quad.cols_count, 13); // "Error Message" length
        assert!(first_quad.underline);
        assert!(first_quad.color[0] > 0.5); // Red color
    }
}
