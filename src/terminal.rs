use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color as AnsiColor, NamedColor, Processor};
use glyphon::{Attrs, Buffer, Color as GlyphonColor, Family, FontSystem, Shaping};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TermEventListener {
    pub pty_write_queue: Arc<Mutex<Vec<u8>>>,
    pub theme_bg: Arc<Mutex<[u8; 3]>>,
    pub theme_fg: Arc<Mutex<[u8; 3]>>,
    pub tab_title: Arc<Mutex<Option<String>>>,
}

impl TermEventListener {
    pub fn new(fg: [u8; 3], bg: [u8; 3]) -> Self {
        Self {
            pty_write_queue: Arc::new(Mutex::new(Vec::new())),
            theme_bg: Arc::new(Mutex::new(bg)),
            theme_fg: Arc::new(Mutex::new(fg)),
            tab_title: Arc::new(Mutex::new(None)),
        }
    }
}

impl EventListener for TermEventListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(text) => {
                if let Ok(mut q) = self.pty_write_queue.lock() {
                    q.extend_from_slice(text.as_bytes());
                }
            }
            Event::ColorRequest(index, formatter) => {
                let rgb = if index == NamedColor::Background as usize {
                    let bg = self.theme_bg.lock().unwrap_or_else(|e| e.into_inner());
                    alacritty_terminal::vte::ansi::Rgb {
                        r: bg[0],
                        g: bg[1],
                        b: bg[2],
                    }
                } else if index == NamedColor::Foreground as usize {
                    let fg = self.theme_fg.lock().unwrap_or_else(|e| e.into_inner());
                    alacritty_terminal::vte::ansi::Rgb {
                        r: fg[0],
                        g: fg[1],
                        b: fg[2],
                    }
                } else {
                    alacritty_terminal::vte::ansi::Rgb {
                        r: 128,
                        g: 128,
                        b: 128,
                    }
                };
                let resp = formatter(rgb);
                if let Ok(mut q) = self.pty_write_queue.lock() {
                    q.extend_from_slice(resp.as_bytes());
                }
            }
            Event::Title(title) => {
                if let Ok(mut t) = self.tab_title.lock() {
                    *t = Some(title);
                }
            }
            _ => {}
        }
    }
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
    event_listener: TermEventListener,
}

impl Terminal {
    #[allow(dead_code)]
    pub fn new(cols: u16, rows: u16) -> Self {
        Self::with_colors(cols, rows, [248, 248, 242], [39, 40, 34])
    }

    pub fn with_colors(cols: u16, rows: u16, fg: [u8; 3], bg: [u8; 3]) -> Self {
        let size = TerminalSize {
            cols: cols as usize,
            rows: rows as usize,
        };
        let config = Config::default();
        let event_listener = TermEventListener::new(fg, bg);
        let term = Term::new(config, &size, event_listener.clone());
        let processor = Processor::new();

        Self {
            term,
            processor,
            cols,
            rows,
            event_listener,
        }
    }

    pub fn set_theme_colors(&self, fg: [u8; 3], bg: [u8; 3]) {
        if let Ok(mut current_fg) = self.event_listener.theme_fg.lock() {
            *current_fg = fg;
        }
        if let Ok(mut current_bg) = self.event_listener.theme_bg.lock() {
            *current_bg = bg;
        }
    }

    pub fn take_pty_writes(&self) -> Vec<u8> {
        if let Ok(mut q) = self.event_listener.pty_write_queue.lock() {
            std::mem::take(&mut *q)
        } else {
            Vec::new()
        }
    }

    pub fn take_tab_title(&self) -> Option<String> {
        self.event_listener.tab_title.lock().ok()?.take()
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

    pub fn start_selection(&mut self, ty: SelectionType, point: Point, side: Side) {
        self.term.selection = Some(Selection::new(ty, point, side));
    }

    pub fn update_selection(&mut self, point: Point, side: Side) {
        if let Some(ref mut sel) = self.term.selection {
            sel.update(point, side);
        }
    }

    pub fn clear_selection(&mut self) -> bool {
        if self.term.selection.is_some() {
            self.term.selection = None;
            true
        } else {
            false
        }
    }

    pub fn has_selection(&self) -> bool {
        self.term.selection.as_ref().map_or(false, |s| !s.is_empty())
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string()
    }

    pub fn point_at_pos(
        &self,
        x: f64,
        y: f64,
        padding_x: f32,
        padding_y: f32,
        cell_w: f32,
        cell_h: f32,
    ) -> Option<(Point, Side)> {
        let grid = self.term.grid();
        let screen_lines = grid.screen_lines();
        let columns = grid.columns();
        if screen_lines == 0 || columns == 0 {
            return None;
        }
        let display_offset = grid.display_offset();

        let rel_x = (x as f32 - padding_x).max(0.0);
        let rel_y = (y as f32 - padding_y).max(0.0);

        let col_idx = (rel_x / cell_w.max(1.0)).floor() as usize;
        let col_idx = col_idx.min(columns.saturating_sub(1));

        let row_idx = (rel_y / cell_h.max(1.0)).floor() as usize;
        let row_idx = row_idx.min(screen_lines.saturating_sub(1));

        let in_cell_x = rel_x - (col_idx as f32 * cell_w);
        let side = if in_cell_x < (cell_w / 2.0) {
            Side::Left
        } else {
            Side::Right
        };

        let line = Line((row_idx as i32) - (display_offset as i32));
        Some((Point::new(line, Column(col_idx)), side))
    }

    /// Detects a web URL under the specified mouse coordinates (e.g. for Ctrl + Click opening).
    pub fn url_at_pos(
        &self,
        x: f64,
        y: f64,
        padding_x: f32,
        padding_y: f32,
        cell_w: f32,
        cell_h: f32,
    ) -> Option<String> {
        let (point, _) = self.point_at_pos(x, y, padding_x, padding_y, cell_w, cell_h)?;
        let grid = self.term.grid();
        let columns = grid.columns();
        let line = point.line;
        let col = point.column.0;

        let mut line_chars = Vec::with_capacity(columns);
        for c in 0..columns {
            line_chars.push(grid[line][Column(c)].c);
        }
        let line_text: String = line_chars.into_iter().collect();

        for prefix in &["https://", "http://"] {
            for (start_idx, _) in line_text.match_indices(prefix) {
                let remainder = &line_text[start_idx..];
                let len = remainder
                    .find(|c: char| {
                        c.is_whitespace()
                            || c == '"'
                            || c == '\''
                            || c == '`'
                            || c == '<'
                            || c == '>'
                            || c == ')'
                            || c == ']'
                            || c == '}'
                    })
                    .unwrap_or(remainder.len());

                let mut raw_url = &remainder[..len];
                raw_url = raw_url.trim_end_matches(|c| c == '.' || c == ',' || c == ';' || c == ':');
                let col_start = line_text[..start_idx].chars().count();
                let col_len = raw_url.chars().count();
                let col_end = col_start + col_len;

                if col >= col_start && col < col_end && raw_url.len() > prefix.len() {
                    return Some(raw_url.to_string());
                }
            }
        }
        None
    }

    /// Renders terminal grid directly into text buffer without injecting cursor characters,
    /// ensuring characters never shift or get displaced during navigation.
    /// Handles scrollback history, accurate cell background quads, selection highlights, and rich style attributes (bold, italic, underline).
    /// Returns (cursor_column, cursor_row, scroll_info, cell_quads).
    pub fn render_to_buffer(
        &self,
        font_system: &mut FontSystem,
        buffer: &mut Buffer,
        font_family: &str,
        default_color: GlyphonColor,
        theme_bg: GlyphonColor,
        selection_color: [f32; 4],
    ) -> (usize, usize, ScrollInfo, Vec<TerminalCellQuad>) {
        let grid = self.term.grid();
        let screen_lines = grid.screen_lines();
        let columns = grid.columns();
        let display_offset = grid.display_offset();
        let total_lines = grid.total_lines();
        let cursor_point = grid.cursor.point;
        let sel_range = self.term.selection.as_ref().and_then(|s| s.to_range(&self.term));

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

            // Scan line for URLs to automatically underline them
            let mut is_url_col = vec![false; columns];
            let mut line_chars = Vec::with_capacity(columns);
            for c in 0..columns {
                line_chars.push(grid[line][Column(c)].c);
            }
            let line_text: String = line_chars.into_iter().collect();
            for prefix in &["https://", "http://"] {
                for (start_idx, _) in line_text.match_indices(prefix) {
                    let remainder = &line_text[start_idx..];
                    let len = remainder
                        .find(|c: char| {
                            c.is_whitespace()
                                || c == '"'
                                || c == '\''
                                || c == '`'
                                || c == '<'
                                || c == '>'
                                || c == ')'
                                || c == ']'
                                || c == '}'
                        })
                        .unwrap_or(remainder.len());

                    let raw_url = remainder[..len].trim_end_matches(|c| c == '.' || c == ',' || c == ';' || c == ':');
                    let col_start = line_text[..start_idx].chars().count();
                    let col_len = raw_url.chars().count();

                    if raw_url.len() > prefix.len() && col_start < columns {
                        let col_end = (col_start + col_len).min(columns);
                        for c_idx in col_start..col_end {
                            is_url_col[c_idx] = true;
                        }
                    }
                }
            }

            for col_idx in 0..columns {
                let col = Column(col_idx);
                let cell = &grid[line][col];

                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }

                let is_selected = sel_range.as_ref().map_or(false, |r| r.contains(Point::new(line, col)));

                let is_inverse = cell.flags.contains(Flags::INVERSE);
                let raw_fg = if is_inverse { cell.bg } else { cell.fg };
                let raw_bg = if is_inverse { cell.fg } else { cell.bg };

                let mut cell_fg = ansi_to_glyphon_color(raw_fg, default_color, theme_bg);
                if is_selected {
                    cell_fg = GlyphonColor::rgb(255, 255, 255);
                } else if cell.flags.contains(Flags::DIM) {
                    cell_fg = GlyphonColor::rgb(
                        (cell_fg.r() as f32 * 0.66) as u8,
                        (cell_fg.g() as f32 * 0.66) as u8,
                        (cell_fg.b() as f32 * 0.66) as u8,
                    );
                }

                let bold = cell.flags.contains(Flags::BOLD);
                let italic = cell.flags.contains(Flags::ITALIC);
                let underline = cell.flags.contains(Flags::UNDERLINE) || is_url_col[col_idx];
                let strikethrough = cell.flags.contains(Flags::STRIKEOUT);

                let cell_bg_opt = ansi_to_rgba(raw_bg, default_color, theme_bg, false);
                let fg_rgba = [
                    (cell_fg.r() as f32) / 255.0,
                    (cell_fg.g() as f32) / 255.0,
                    (cell_fg.b() as f32) / 255.0,
                    1.0,
                ];

                // Background & decoration quads
                let (bg_color, has_bg) = if is_selected {
                    (selection_color, true)
                } else if let Some(bg) = cell_bg_opt {
                    (bg, true)
                } else {
                    ([0.0, 0.0, 0.0, 0.0], false)
                };
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
            buffer.set_text(font_system, "", base_attrs.color(default_color), Shaping::Advanced);
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

            buffer.set_rich_text(font_system, rich_spans, base_attrs.color(default_color), Shaping::Advanced);
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

pub fn ansi_to_glyphon_color(
    color: AnsiColor,
    default_fg: GlyphonColor,
    default_bg: GlyphonColor,
) -> GlyphonColor {
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
            NamedColor::BrightBlack => GlyphonColor::rgb(100, 100, 105),
            NamedColor::BrightRed => GlyphonColor::rgb(255, 115, 115),
            NamedColor::BrightGreen => GlyphonColor::rgb(120, 220, 160),
            NamedColor::BrightYellow => GlyphonColor::rgb(255, 235, 120),
            NamedColor::BrightBlue => GlyphonColor::rgb(120, 190, 255),
            NamedColor::BrightMagenta => GlyphonColor::rgb(225, 150, 250),
            NamedColor::BrightCyan => GlyphonColor::rgb(140, 230, 255),
            NamedColor::BrightWhite => GlyphonColor::rgb(255, 255, 255),
            NamedColor::Foreground => default_fg,
            NamedColor::Background => default_bg,
            _ => default_fg,
        },
        AnsiColor::Spec(rgb) => GlyphonColor::rgb(rgb.r, rgb.g, rgb.b),
        AnsiColor::Indexed(idx) => indexed_to_glyphon_color(idx, default_fg, default_bg),
    }
}

fn indexed_to_glyphon_color(
    idx: u8,
    default_fg: GlyphonColor,
    default_bg: GlyphonColor,
) -> GlyphonColor {
    match idx {
        0..=15 => ansi_to_glyphon_color(
            AnsiColor::Named(match idx {
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
            }),
            default_fg,
            default_bg,
        ),
        16..=231 => {
            const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];
            let i = (idx - 16) as usize;
            let b = CUBE[i % 6];
            let g = CUBE[(i / 6) % 6];
            let r = CUBE[(i / 36) % 6];
            GlyphonColor::rgb(r, g, b)
        }
        232..=255 => {
            let gray = (idx - 232) * 10 + 8;
            GlyphonColor::rgb(gray, gray, gray)
        }
    }
}

pub fn ansi_to_rgba(
    color: AnsiColor,
    default_fg: GlyphonColor,
    default_bg: GlyphonColor,
    is_fg: bool,
) -> Option<[f32; 4]> {
    match color {
        AnsiColor::Named(NamedColor::Background) => {
            if is_fg {
                Some([
                    (default_bg.r() as f32) / 255.0,
                    (default_bg.g() as f32) / 255.0,
                    (default_bg.b() as f32) / 255.0,
                    1.0,
                ])
            } else {
                None
            }
        }
        AnsiColor::Named(NamedColor::Foreground) => {
            Some([
                (default_fg.r() as f32) / 255.0,
                (default_fg.g() as f32) / 255.0,
                (default_fg.b() as f32) / 255.0,
                1.0,
            ])
        }
        AnsiColor::Named(named) => {
            let gc = ansi_to_glyphon_color(AnsiColor::Named(named), default_fg, default_bg);
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
            let gc = indexed_to_glyphon_color(idx, default_fg, default_bg);
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
        let default_color = GlyphonColor::rgb(220, 220, 220);
        let default_bg = GlyphonColor::rgb(30, 30, 30);
        let (_col, _row, _scroll, quads) = term.render_to_buffer(
            &mut font_system,
            &mut buffer,
            "Cascadia Code",
            default_color,
            default_bg,
            [0.25, 0.45, 0.85, 0.4],
        );

        assert!(!quads.is_empty(), "Should generate background/decoration quads");
        let first_quad = &quads[0];
        assert_eq!(first_quad.row, 0);
        assert_eq!(first_quad.col, 0);
        assert_eq!(first_quad.cols_count, 13); // "Error Message" length
        assert!(first_quad.underline);
        assert!(first_quad.color[0] > 0.5); // Red color
    }

    #[test]
    fn test_inverse_video() {
        let mut term = Terminal::new(80, 24);
        // Write inverse text (\x1b[7mInverse\x1b[27m)
        term.process_bytes(b"\x1b[7mInverse\x1b[27m\r\n");

        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, glyphon::Metrics::new(13.0, 19.0));
        let default_color = GlyphonColor::rgb(220, 220, 220);
        let default_bg = GlyphonColor::rgb(30, 30, 30);
        let (_col, _row, _scroll, quads) = term.render_to_buffer(
            &mut font_system,
            &mut buffer,
            "Cascadia Code",
            default_color,
            default_bg,
            [0.25, 0.45, 0.85, 0.4],
        );

        assert!(!quads.is_empty(), "Inverse video must generate a background quad");
        let first_quad = &quads[0];
        assert_eq!(first_quad.row, 0);
        assert_eq!(first_quad.col, 0);
        assert_eq!(first_quad.cols_count, 7);
        // Background of inverse cell must match default_color (220/255 = 0.86)
        assert!((first_quad.color[0] - 220.0 / 255.0).abs() < 0.05);
    }

    #[test]
    fn test_terminal_color_request_and_pty_write() {
        let mut term = Terminal::with_colors(80, 24, [248, 248, 242], [39, 40, 34]);
        // Query OSC 11 background: \x1b]11;?\x07
        term.process_bytes(b"\x1b]11;?\x07");
        let writes = term.take_pty_writes();
        let resp = String::from_utf8_lossy(&writes);
        assert!(
            resp.contains("11;rgb:"),
            "OSC 11 query must produce color response, got: {}",
            resp
        );

        // Query CPR cursor position report: \x1b[6n
        term.process_bytes(b"\x1b[6n");
        let cpr_writes = term.take_pty_writes();
        let cpr_resp = String::from_utf8_lossy(&cpr_writes);
        assert!(
            cpr_resp.contains("\x1b[1;1R"),
            "CPR query must respond with cursor position, got: {}",
            cpr_resp
        );
    }

    #[test]
    fn test_text_selection_and_copy() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"Hello World from Aero!\r\n");

        // Start selection on "Hello" (cols 0..5, line 0)
        let point = Point::new(Line(0), Column(0));
        term.start_selection(SelectionType::Simple, point, Side::Left);
        term.update_selection(Point::new(Line(0), Column(4)), Side::Right);

        assert!(term.has_selection());
        let selected_text = term.selection_text().expect("Should extract selected text");
        assert_eq!(selected_text.trim(), "Hello");

        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, glyphon::Metrics::new(13.0, 19.0));
        let default_color = GlyphonColor::rgb(220, 220, 220);
        let default_bg = GlyphonColor::rgb(30, 30, 30);
        let (_col, _row, _scroll, quads) = term.render_to_buffer(
            &mut font_system,
            &mut buffer,
            "Cascadia Code",
            default_color,
            default_bg,
            [0.25, 0.45, 0.85, 0.4],
        );

        // A quad for the selection highlight should be rendered
        let sel_quad = quads.iter().find(|q| q.row == 0 && q.col == 0);
        assert!(sel_quad.is_some(), "Selection quad should exist at row 0, col 0");
        let q = sel_quad.unwrap();
        assert_eq!(q.cols_count, 5);
        assert_eq!(q.color, [0.25, 0.45, 0.85, 0.4]);

        term.clear_selection();
        assert!(!term.has_selection());
    }

    #[test]
    fn test_vite_colors_and_link_underline() {
        let mut term = Terminal::new(80, 24);
        let bytes = b"  \x1b[32mVITE\x1b[39m \x1b[32mv8.2.0\x1b[39m  ready in \x1b[1m25417\x1b[22m ms\r\n  \x1b[32m\xe2\x9e\x9c\x1b[39m  \x1b[1mLocal:\x1b[22m   \x1b[36mhttp://localhost:5173/\x1b[39m\r\n";
        term.process_bytes(bytes);

        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, glyphon::Metrics::new(13.0, 19.0));
        let default_color = GlyphonColor::rgb(220, 220, 220);
        let default_bg = GlyphonColor::rgb(30, 30, 30);
        let (_col, _row, _scroll, quads) = term.render_to_buffer(
            &mut font_system,
            &mut buffer,
            "Cascadia Code",
            default_color,
            default_bg,
            [0.25, 0.45, 0.85, 0.4],
        );

        // Verify layout runs have solid colors (not scrambled)
        let runs: Vec<_> = buffer.layout_runs().collect();
        assert!(runs.len() >= 2);

        // Line 0: "VITE" glyphs must be green
        let line0_glyphs = runs[0].glyphs;
        // Glyphs 2..6 correspond to V, I, T, E
        for g in &line0_glyphs[2..6] {
            assert_eq!(
                g.color_opt,
                Some(GlyphonColor::rgb(50, 205, 120)),
                "VITE characters must be solid green"
            );
        }

        // Verify that quads with underline: true exist for the entire URL http://localhost:5173/ on line 1
        let total_underlined: usize = quads
            .iter()
            .filter(|q| q.row == 1 && q.underline)
            .map(|q| q.cols_count)
            .sum();
        assert_eq!(
            total_underlined, 22,
            "Total underlined cells must span the entire 22 characters of 'http://localhost:5173/'"
        );
    }

    #[test]
    fn test_url_detection() {
        let mut term = Terminal::new(80, 24);
        term.process_bytes(b"Visit https://github.com/carllosnc/aero for docs.\r\n");

        let padding_x = 0.0;
        let padding_y = 0.0;
        let cell_w = 10.0;
        let cell_h = 20.0;

        // Position over "https" (col 6, x = 65.0, y = 10.0)
        let detected = term.url_at_pos(65.0, 10.0, padding_x, padding_y, cell_w, cell_h);
        assert_eq!(
            detected.as_deref(),
            Some("https://github.com/carllosnc/aero")
        );

        // Position over "Visit" (col 2, x = 25.0)
        let none_detected = term.url_at_pos(25.0, 10.0, padding_x, padding_y, cell_w, cell_h);
        assert_eq!(none_detected, None);
    }

    #[test]
    fn test_url_underline_with_multibyte_prefix() {
        let mut term = Terminal::new(80, 24);
        // Simulate Astro dev server output with multi-byte '│' (3 bytes in UTF-8)
        term.process_bytes("│  Local   http://localhost:4321/\r\n".as_bytes());

        let mut font_system = FontSystem::new();
        let mut buffer = Buffer::new(&mut font_system, glyphon::Metrics::new(14.0, 20.0));
        let (_, _, _, quads) = term.render_to_buffer(
            &mut font_system,
            &mut buffer,
            "monospace",
            GlyphonColor::rgb(255, 255, 255),
            GlyphonColor::rgb(30, 30, 30),
            [0.2, 0.4, 0.8, 0.4],
        );

        // Find underlined quads on row 0
        let underlined_quads: Vec<_> = quads.into_iter().filter(|q| q.row == 0 && q.underline).collect();
        assert!(!underlined_quads.is_empty(), "Underlined quads must exist on row 0");

        // The URL 'http://localhost:4321/' starts at character column 11 ('h'), not byte offset 13!
        let first_quad = &underlined_quads[0];
        assert_eq!(first_quad.col, 11, "Underline must start at column 11 ('h')");

        let total_underlined: usize = underlined_quads.iter().map(|q| q.cols_count).sum();
        assert_eq!(total_underlined, 22, "Underline must span exactly the 22 characters of the URL");

        // Test click hit-testing at character column 11 ('h') and column 33 (after '/')
        let cell_w = 10.0;
        let cell_h = 20.0;
        let click_h = term.url_at_pos(115.0, 10.0, 0.0, 0.0, cell_w, cell_h);
        assert_eq!(click_h.as_deref(), Some("http://localhost:4321/"));

        let click_after = term.url_at_pos(335.0, 10.0, 0.0, 0.0, cell_w, cell_h);
        assert_eq!(click_after, None, "Click after the URL must not match");
    }
}
