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
    #[serde(default = "default_shell")]
    pub shell: String,
}

fn default_cursor_style() -> String {
    "bar".to_string()
}

fn default_shell() -> String {
    "pwsh".to_string()
}

#[derive(Debug, Clone)]
pub struct ThemePalette {
    pub name: &'static str,
    pub is_light: bool,
    pub description: &'static str,
    pub bg: [f32; 4],
    pub tab_bar_bg: [f32; 4],
    pub tab_active_bg: [f32; 4],
    pub accent: [f32; 4],
    pub border: [f32; 4],
    pub fg_color: [u8; 3],
    pub cursor_color: [u8; 3],
    pub preview_colors: [[u8; 3]; 5],
}

pub const THEMES: &[ThemePalette] = &[
    ThemePalette {
        name: "Dark Slate",
        is_light: false,
        description: "Moderno e minimalista com tons ardósia",
        bg: [0.07, 0.08, 0.10, 1.0],
        tab_bar_bg: [0.095, 0.105, 0.135, 1.0],
        tab_active_bg: [0.145, 0.165, 0.215, 1.0],
        accent: [0.22, 0.74, 0.97, 1.0], // Cyan #38bdf8
        border: [0.18, 0.21, 0.27, 1.0],
        fg_color: [225, 235, 245],
        cursor_color: [80, 220, 255],
        preview_colors: [
            [18, 20, 26],
            [225, 235, 245],
            [56, 189, 248],
            [52, 211, 153],
            [129, 140, 248],
        ],
    },
    ThemePalette {
        name: "Catppuccin",
        is_light: false,
        description: "Paleta pastel moderna com azul lavanda suave",
        bg: [0.12, 0.12, 0.18, 1.0], // #1e1e2e
        tab_bar_bg: [0.09, 0.09, 0.14, 1.0],
        tab_active_bg: [0.19, 0.20, 0.27, 1.0],
        accent: [0.54, 0.70, 0.98, 1.0], // Lavender / Mauve #89b4fa
        border: [0.27, 0.28, 0.36, 1.0],
        fg_color: [205, 214, 244],
        cursor_color: [245, 224, 220],
        preview_colors: [
            [30, 30, 46],
            [205, 214, 244],
            [203, 166, 247],
            [166, 227, 161],
            [137, 180, 250],
        ],
    },
    ThemePalette {
        name: "Dracula",
        is_light: false,
        description: "Contraste gótico vibrante com roxo, rosa e ciano",
        bg: [0.16, 0.16, 0.21, 1.0], // #282a36
        tab_bar_bg: [0.13, 0.14, 0.18, 1.0],
        tab_active_bg: [0.27, 0.28, 0.35, 1.0],
        accent: [1.0, 0.47, 0.77, 1.0], // Pink #ff79c6
        border: [0.38, 0.39, 0.48, 1.0],
        fg_color: [248, 248, 242],
        cursor_color: [139, 233, 253],
        preview_colors: [
            [40, 42, 54],
            [248, 248, 242],
            [189, 147, 249],
            [80, 250, 123],
            [255, 121, 198],
        ],
    },
    ThemePalette {
        name: "Tokyo Night",
        is_light: false,
        description: "Azuis profundos inspirados nas noites de Tóquio",
        bg: [0.10, 0.11, 0.15, 1.0], // #1a1b26
        tab_bar_bg: [0.09, 0.09, 0.13, 1.0],
        tab_active_bg: [0.15, 0.17, 0.23, 1.0],
        accent: [0.48, 0.64, 0.97, 1.0], // #7aa2f7
        border: [0.24, 0.26, 0.36, 1.0],
        fg_color: [192, 202, 245],
        cursor_color: [122, 162, 247],
        preview_colors: [
            [26, 27, 38],
            [192, 202, 245],
            [122, 162, 247],
            [158, 206, 106],
            [125, 207, 255],
        ],
    },
    ThemePalette {
        name: "One Dark",
        is_light: false,
        description: "Visual clássico do Atom e VS Code",
        bg: [0.16, 0.17, 0.20, 1.0], // #282c34
        tab_bar_bg: [0.13, 0.14, 0.17, 1.0],
        tab_active_bg: [0.22, 0.24, 0.29, 1.0],
        accent: [0.38, 0.69, 0.94, 1.0], // #61afef
        border: [0.28, 0.31, 0.36, 1.0],
        fg_color: [171, 178, 191],
        cursor_color: [97, 175, 239],
        preview_colors: [
            [40, 44, 52],
            [171, 178, 191],
            [97, 175, 239],
            [152, 195, 121],
            [229, 192, 123],
        ],
    },
    ThemePalette {
        name: "Monokai",
        is_light: false,
        description: "Alto contraste clássico com tons amarelos e esmeralda",
        bg: [0.15, 0.16, 0.13, 1.0], // #272822
        tab_bar_bg: [0.12, 0.12, 0.10, 1.0],
        tab_active_bg: [0.24, 0.24, 0.20, 1.0],
        accent: [0.65, 0.89, 0.18, 1.0], // #a6e22e
        border: [0.31, 0.31, 0.26, 1.0],
        fg_color: [248, 248, 242],
        cursor_color: [230, 219, 116],
        preview_colors: [
            [39, 40, 34],
            [248, 248, 242],
            [166, 226, 46],
            [102, 217, 239],
            [249, 38, 114],
        ],
    },
    ThemePalette {
        name: "Nord",
        is_light: false,
        description: "Elegância ártica em tons azul-gelo nórdicos",
        bg: [0.18, 0.20, 0.25, 1.0], // #2e3440
        tab_bar_bg: [0.15, 0.17, 0.21, 1.0],
        tab_active_bg: [0.26, 0.29, 0.35, 1.0],
        accent: [0.53, 0.75, 0.82, 1.0], // Frost cyan #88c0d0
        border: [0.30, 0.34, 0.42, 1.0],
        fg_color: [236, 239, 244],
        cursor_color: [143, 188, 187],
        preview_colors: [
            [46, 52, 64],
            [236, 239, 244],
            [136, 192, 208],
            [163, 190, 140],
            [129, 161, 193],
        ],
    },
    ThemePalette {
        name: "Gruvbox Dark",
        is_light: false,
        description: "Cores terrosas e acolhedoras estilo retrô groove",
        bg: [0.157, 0.157, 0.157, 1.0], // #282828
        tab_bar_bg: [0.114, 0.122, 0.125, 1.0],
        tab_active_bg: [0.235, 0.219, 0.212, 1.0],
        accent: [0.98, 0.74, 0.18, 1.0], // Yellow #fabd2f
        border: [0.314, 0.286, 0.271, 1.0],
        fg_color: [235, 219, 178],
        cursor_color: [250, 189, 47],
        preview_colors: [
            [40, 40, 40],
            [235, 219, 178],
            [250, 189, 47],
            [184, 187, 38],
            [131, 165, 152],
        ],
    },
    ThemePalette {
        name: "Rose Pine",
        is_light: false,
        description: "Visual aconchegante com tons rosé, pinho e lavanda",
        bg: [0.098, 0.090, 0.141, 1.0], // #191724
        tab_bar_bg: [0.078, 0.071, 0.114, 1.0],
        tab_active_bg: [0.165, 0.153, 0.235, 1.0],
        accent: [0.92, 0.74, 0.73, 1.0], // Rose #ebbcba
        border: [0.247, 0.224, 0.337, 1.0],
        fg_color: [224, 222, 244],
        cursor_color: [235, 188, 186],
        preview_colors: [
            [25, 23, 36],
            [224, 222, 244],
            [235, 188, 186],
            [156, 207, 216],
            [196, 167, 231],
        ],
    },
    ThemePalette {
        name: "Kanagawa",
        is_light: false,
        description: "Inspirado na icônica pintura A Grande Onda de Kanagawa",
        bg: [0.122, 0.122, 0.157, 1.0], // #1f1f28
        tab_bar_bg: [0.094, 0.094, 0.125, 1.0],
        tab_active_bg: [0.173, 0.173, 0.224, 1.0],
        accent: [0.49, 0.61, 0.85, 1.0], // Wave blue #7e9cd8
        border: [0.278, 0.278, 0.357, 1.0],
        fg_color: [220, 215, 186],
        cursor_color: [126, 156, 216],
        preview_colors: [
            [31, 31, 40],
            [220, 215, 186],
            [126, 156, 216],
            [152, 187, 108],
            [228, 104, 116],
        ],
    },
    ThemePalette {
        name: "Cyberpunk 2077",
        is_light: false,
        description: "Neon amarelo e ciano futurista de alta energia",
        bg: [0.051, 0.047, 0.114, 1.0], // #0d0c1d
        tab_bar_bg: [0.035, 0.031, 0.078, 1.0],
        tab_active_bg: [0.141, 0.125, 0.259, 1.0],
        accent: [0.98, 0.93, 0.04, 1.0], // Neon yellow #fcee0a
        border: [0.0, 0.94, 1.0, 0.6],
        fg_color: [255, 255, 255],
        cursor_color: [0, 240, 255],
        preview_colors: [
            [13, 12, 29],
            [255, 255, 255],
            [252, 238, 10],
            [0, 240, 255],
            [255, 0, 128],
        ],
    },
    ThemePalette {
        name: "Solarized Dark",
        is_light: false,
        description: "Paleta harmônica projetada por Ethan Schoonover",
        bg: [0.00, 0.169, 0.212, 1.0], // #002b36
        tab_bar_bg: [0.00, 0.129, 0.165, 1.0],
        tab_active_bg: [0.027, 0.212, 0.259, 1.0],
        accent: [0.149, 0.545, 0.824, 1.0], // Blue #268bd2
        border: [0.055, 0.278, 0.337, 1.0],
        fg_color: [131, 148, 150],
        cursor_color: [38, 139, 210],
        preview_colors: [
            [0, 43, 54],
            [131, 148, 150],
            [38, 139, 210],
            [133, 153, 0],
            [42, 161, 152],
        ],
    },
    ThemePalette {
        name: "GitHub Dark",
        is_light: false,
        description: "O clássico tema escuro oficial do GitHub",
        bg: [0.051, 0.067, 0.090, 1.0], // #0d1117
        tab_bar_bg: [0.035, 0.047, 0.067, 1.0],
        tab_active_bg: [0.125, 0.153, 0.196, 1.0],
        accent: [0.345, 0.651, 1.0, 1.0], // Blue #58a6ff
        border: [0.188, 0.227, 0.286, 1.0],
        fg_color: [201, 209, 217],
        cursor_color: [88, 166, 255],
        preview_colors: [
            [13, 17, 23],
            [201, 209, 217],
            [88, 166, 255],
            [63, 185, 80],
            [210, 153, 34],
        ],
    },
    ThemePalette {
        name: "Solarized Light",
        is_light: true,
        description: "Contraste diurno suave em bege para leitura prolongada",
        bg: [0.992, 0.965, 0.890, 1.0], // #fdf6e3
        tab_bar_bg: [0.933, 0.910, 0.835, 1.0],
        tab_active_bg: [0.867, 0.843, 0.765, 1.0],
        accent: [0.149, 0.545, 0.824, 1.0],
        border: [0.776, 0.749, 0.678, 1.0],
        fg_color: [101, 123, 131],
        cursor_color: [38, 139, 210],
        preview_colors: [
            [253, 246, 227],
            [101, 123, 131],
            [38, 139, 210],
            [133, 153, 0],
            [181, 137, 0],
        ],
    },
    ThemePalette {
        name: "GitHub Light",
        is_light: true,
        description: "Branco limpo com alto contraste oficial do GitHub",
        bg: [1.0, 1.0, 1.0, 1.0],
        tab_bar_bg: [0.96, 0.97, 0.98, 1.0],
        tab_active_bg: [0.91, 0.93, 0.95, 1.0],
        accent: [0.035, 0.412, 0.855, 1.0], // Blue #0969da
        border: [0.82, 0.85, 0.88, 1.0],
        fg_color: [36, 41, 47],
        cursor_color: [9, 105, 218],
        preview_colors: [
            [255, 255, 255],
            [36, 41, 47],
            [9, 105, 218],
            [26, 127, 55],
            [154, 103, 0],
        ],
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
            shell: "pwsh".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.cursor_style, "bar");
        assert_eq!(cfg.shell, "pwsh");
        assert_eq!(cfg.theme, "Dark Slate");
        assert!(cfg.padding_x > 0.0);
        assert!(cfg.padding_y > 0.0);
        assert!(cfg.font_size > 0.0);
    }

    #[test]
    fn test_config_json_serde_with_defaults() {
        let json_str = r#"{
            "padding_x": 16.0,
            "padding_y": 12.0,
            "font_family": "Fira Code",
            "font_size": 16.0,
            "theme": "Dracula"
        }"#;
        let cfg: AppConfig = serde_json::from_str(json_str).expect("Should deserialize with defaults");
        assert_eq!(cfg.padding_x, 16.0);
        assert_eq!(cfg.theme, "Dracula");
        assert_eq!(cfg.cursor_style, "bar");
        assert_eq!(cfg.shell, "pwsh");
    }

    #[test]
    fn test_current_theme_resolution_and_fallback() {
        let mut cfg = AppConfig::default();
        cfg.theme = "dracula".to_string();
        let theme = cfg.current_theme();
        assert_eq!(theme.name, "Dracula");

        cfg.theme = "catppuccin".to_string();
        let theme = cfg.current_theme();
        assert_eq!(theme.name, "Catppuccin");

        cfg.theme = "NonExistentTheme123".to_string();
        let fallback = cfg.current_theme();
        assert_eq!(fallback.name, "Dark Slate");
    }

    #[test]
    fn test_cell_metrics_calculation() {
        let mut cfg = AppConfig::default();
        cfg.font_size = 15.0;
        let (w, h) = cfg.cell_metrics();
        assert!((w - (15.0_f32 * 0.58594_f32)).abs() < 0.001);
        assert!((h - (15.0_f32 * 1.4667_f32).round()).abs() < 0.001);
        assert!(h >= 16.0);
    }
}
