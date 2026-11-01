use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub padding_x: f32,
    pub padding_y: f32,
    pub font_family: String,
    pub font_size: f32,
    pub theme: String,
    #[serde(default = "default_cursor_style")]
    pub cursor_style: String,
}

fn default_cursor_style() -> String {
    "bar".to_string()
}

#[derive(Debug, Clone)]
pub struct ThemePalette {
    pub name: &'static str,
    pub bg: [f32; 4],
    pub tab_bar_bg: [f32; 4],
    pub tab_active_bg: [f32; 4],
    pub accent: [f32; 4],
    pub border: [f32; 4],
    pub fg_color: [u8; 3],
    pub cursor_color: [u8; 3],
}

pub const THEMES: &[ThemePalette] = &[
    ThemePalette {
        name: "Dark Slate",
        bg: [0.07, 0.08, 0.10, 1.0],
        tab_bar_bg: [0.095, 0.105, 0.135, 1.0],
        tab_active_bg: [0.145, 0.165, 0.215, 1.0],
        accent: [0.22, 0.74, 0.97, 1.0], // Cyan #38bdf8
        border: [0.18, 0.21, 0.27, 1.0],
        fg_color: [225, 235, 245],
        cursor_color: [80, 220, 255],
    },
    ThemePalette {
        name: "Catppuccin",
        bg: [0.12, 0.12, 0.18, 1.0], // #1e1e2e
        tab_bar_bg: [0.09, 0.09, 0.14, 1.0],
        tab_active_bg: [0.19, 0.20, 0.27, 1.0],
        accent: [0.54, 0.70, 0.98, 1.0], // Lavender / Mauve #89b4fa
        border: [0.27, 0.28, 0.36, 1.0],
        fg_color: [205, 214, 244],
        cursor_color: [245, 224, 220],
    },
    ThemePalette {
        name: "Dracula",
        bg: [0.16, 0.16, 0.21, 1.0], // #282a36
        tab_bar_bg: [0.13, 0.14, 0.18, 1.0],
        tab_active_bg: [0.27, 0.28, 0.35, 1.0],
        accent: [1.0, 0.47, 0.77, 1.0], // Pink #ff79c6
        border: [0.38, 0.39, 0.48, 1.0],
        fg_color: [248, 248, 242],
        cursor_color: [139, 233, 253],
    },
    ThemePalette {
        name: "Tokyo Night",
        bg: [0.10, 0.11, 0.15, 1.0], // #1a1b26
        tab_bar_bg: [0.09, 0.09, 0.13, 1.0],
        tab_active_bg: [0.15, 0.17, 0.23, 1.0],
        accent: [0.48, 0.64, 0.97, 1.0], // #7aa2f7
        border: [0.24, 0.26, 0.36, 1.0],
        fg_color: [192, 202, 245],
        cursor_color: [122, 162, 247],
    },
    ThemePalette {
        name: "One Dark",
        bg: [0.16, 0.17, 0.20, 1.0], // #282c34
        tab_bar_bg: [0.13, 0.14, 0.17, 1.0],
        tab_active_bg: [0.22, 0.24, 0.29, 1.0],
        accent: [0.38, 0.69, 0.94, 1.0], // #61afef
        border: [0.28, 0.31, 0.36, 1.0],
        fg_color: [171, 178, 191],
        cursor_color: [97, 175, 239],
    },
    ThemePalette {
        name: "Monokai",
        bg: [0.15, 0.16, 0.13, 1.0], // #272822
        tab_bar_bg: [0.12, 0.12, 0.10, 1.0],
        tab_active_bg: [0.24, 0.24, 0.20, 1.0],
        accent: [0.65, 0.89, 0.18, 1.0], // #a6e22e
        border: [0.31, 0.31, 0.26, 1.0],
        fg_color: [248, 248, 242],
        cursor_color: [230, 219, 116],
    },
    ThemePalette {
        name: "Nord",
        bg: [0.18, 0.20, 0.25, 1.0], // #2e3440
        tab_bar_bg: [0.15, 0.17, 0.21, 1.0],
        tab_active_bg: [0.26, 0.29, 0.35, 1.0],
        accent: [0.53, 0.75, 0.82, 1.0], // Frost cyan #88c0d0
        border: [0.30, 0.34, 0.42, 1.0],
        fg_color: [236, 239, 244],
        cursor_color: [143, 188, 187],
    },
];

#[allow(dead_code)]
pub const FONTS: &[&str] = &[
    "Cascadia Code",
    "Consolas",
    "Fira Code",
    "JetBrains Mono",
    "Segoe UI Mono",
];

#[allow(dead_code)]
pub const FONT_SIZES: &[f32] = &[12.0, 13.0, 14.0, 15.0, 16.0, 18.0];
#[allow(dead_code)]
pub const PADDING_X_PRESETS: &[f32] = &[8.0, 16.0, 24.0, 32.0, 40.0];
#[allow(dead_code)]
pub const PADDING_Y_PRESETS: &[f32] = &[8.0, 12.0, 18.0, 24.0, 30.0];

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            padding_x: 24.0,
            padding_y: 18.0,
            font_family: "Cascadia Code".to_string(),
            font_size: 15.0,
            theme: "Dark Slate".to_string(),
            cursor_style: "bar".to_string(),
        }
    }
}

impl AppConfig {
    fn config_path() -> PathBuf {
        PathBuf::from("drop_config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    tracing::info!("Loaded configuration from {:?}", path);
                    return cfg;
                }
            }
        }
        let default = Self::default();
        let _ = default.save();
        default
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn current_theme(&self) -> ThemePalette {
        for t in THEMES {
            if t.name.eq_ignore_ascii_case(&self.theme) {
                return t.clone();
            }
        }
        THEMES[0].clone()
    }

    pub fn cell_metrics(&self) -> (f32, f32) {
        let w = self.font_size * 0.58594;
        let h = (self.font_size * 1.4667).round().max(16.0);
        (w, h)
    }
}
