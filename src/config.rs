use serde::Deserialize;
use std::fs;
use ratatui::style::Color;

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    pub query_color: String,
    pub selection_bg: String,
    pub selection_fg: String,
    pub app_badge_color: String,
    pub file_badge_color: String,
    pub border_color: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchConfig {
    pub max_depth: usize,
    pub ignored_dirs: Vec<String>,
    #[serde(alias = "disable_fcitx")]
    pub disable_ime: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub theme: ThemeConfig,
    pub search: SearchConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig {
                query_color: "cyan".to_string(),
                selection_bg: "#2d3748".to_string(), // sleek slate gray
                selection_fg: "white".to_string(),
                app_badge_color: "cyan".to_string(),
                file_badge_color: "yellow".to_string(),
                border_color: "#4a5568".to_string(),
            },
            search: SearchConfig {
                max_depth: 3,
                                ignored_dirs: vec![
                    ".git".to_string(),
                    ".cargo".to_string(),
                    ".cache".to_string(),
                    "node_modules".to_string(),
                    "target".to_string(),
                    "build".to_string(),
                    "dist".to_string(),
                ],
                disable_ime: Some(false),
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut config = Self::default();
        if let Some(mut config_path) = dirs::config_dir() {
            config_path.push("view-launcher");
            config_path.push("config.toml");
            
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(parsed) = toml::from_str::<Config>(&content) {
                        config = parsed;
                    }
                }
            }
        }
        config
    }
}

pub fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" => Color::Gray,
        "darkgray" | "dark_gray" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        hex if hex.starts_with('#') && hex.len() == 7 => {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[1..3], 16),
                u8::from_str_radix(&hex[3..5], 16),
                u8::from_str_radix(&hex[5..7], 16),
            ) {
                Color::Rgb(r, g, b)
            } else {
                Color::Reset
            }
        }
        _ => Color::Reset,
    }
}
