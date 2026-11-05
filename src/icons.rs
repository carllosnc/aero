//! Aero Icon Package
//!
//! A structured, categorized semantic icon set powered by Phosphor Icons for egui.
//! Phosphor provides over 1,200 scalable vector glyphs rendered crisply via TrueType vector fonts.

#![allow(dead_code)]

/// Navigation & Tab Icons
pub mod tabs {
    pub const APPEARANCE: &str = egui_phosphor::regular::PALETTE;
    pub const TYPOGRAPHY: &str = egui_phosphor::regular::TEXT_AA;
    pub const SPACING: &str = egui_phosphor::regular::FRAME_CORNERS;
    pub const SHELL: &str = egui_phosphor::regular::TERMINAL_WINDOW;
    pub const SHORTCUTS: &str = egui_phosphor::regular::KEYBOARD;
    pub const GENERAL: &str = egui_phosphor::regular::GEAR_SIX;
}

/// Action & Interactive Icons
pub mod actions {
    pub const CLOSE: &str = egui_phosphor::regular::X;
    pub const RESET: &str = egui_phosphor::regular::ARROWS_COUNTER_CLOCKWISE;
    pub const SEARCH: &str = egui_phosphor::regular::MAGNIFYING_GLASS;
    pub const CLEAR: &str = egui_phosphor::regular::X_CIRCLE;
    pub const SAVE: &str = egui_phosphor::regular::FLOPPY_DISK;
    pub const CHECK: &str = egui_phosphor::regular::CHECK;
    pub const COPY: &str = egui_phosphor::regular::COPY;
    pub const CARET_DOWN: &str = egui_phosphor::regular::CARET_DOWN;
    pub const CARET_RIGHT: &str = egui_phosphor::regular::CARET_RIGHT;
    pub const ARROW_RIGHT: &str = egui_phosphor::regular::ARROW_RIGHT;
    pub const EXTERNAL_LINK: &str = egui_phosphor::regular::ARROW_SQUARE_OUT;
    pub const PLUS: &str = egui_phosphor::regular::PLUS;
    pub const MINUS: &str = egui_phosphor::regular::MINUS;
}

/// Status & Feedback Icons
pub mod status {
    pub const SUCCESS: &str = egui_phosphor::regular::CHECK_CIRCLE;
    pub const WARNING: &str = egui_phosphor::regular::WARNING_CIRCLE;
    pub const ERROR: &str = egui_phosphor::regular::X_CIRCLE;
    pub const INFO: &str = egui_phosphor::regular::INFO;
    pub const SPARKLE: &str = egui_phosphor::regular::SPARKLE;
    pub const ACTIVITY: &str = egui_phosphor::regular::ACTIVITY;
    pub const DOT: &str = egui_phosphor::regular::CIRCLE;
}

/// System, Shell & Terminal Icons
pub mod system {
    pub const TERMINAL: &str = egui_phosphor::regular::TERMINAL;
    pub const WINDOW: &str = egui_phosphor::regular::APP_WINDOW;
    pub const MONITOR: &str = egui_phosphor::regular::MONITOR;
    pub const FOLDER: &str = egui_phosphor::regular::FOLDER_SIMPLE;
    pub const FILE_CODE: &str = egui_phosphor::regular::FILE_CODE;
    pub const CODE: &str = egui_phosphor::regular::CODE;
    pub const CPU: &str = egui_phosphor::regular::CPU;
    pub const SUN: &str = egui_phosphor::regular::SUN;
    pub const MOON: &str = egui_phosphor::regular::MOON;
    pub const CURSOR: &str = egui_phosphor::regular::CURSOR_TEXT;
    pub const SLIDERS: &str = egui_phosphor::regular::SLIDERS_HORIZONTAL;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icons_non_empty() {
        assert!(!tabs::APPEARANCE.is_empty());
        assert!(!tabs::TYPOGRAPHY.is_empty());
        assert!(!tabs::SPACING.is_empty());
        assert!(!tabs::SHELL.is_empty());
        assert!(!actions::CLOSE.is_empty());
        assert!(!actions::RESET.is_empty());
        assert!(!actions::SEARCH.is_empty());
        assert!(!status::SUCCESS.is_empty());
        assert!(!system::TERMINAL.is_empty());
    }
}
