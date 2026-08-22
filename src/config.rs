use serde::Deserialize;
use std::fs;
use ratatui::style::Color;

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    #[serde(default = "default_query_color")]
    pub query_color: String,
    #[serde(default = "default_selection_bg")]
    pub selection_bg: String,
    #[serde(default = "default_selection_fg")]
    pub selection_fg: String,
    #[serde(default = "default_app_badge_color")]
    pub app_badge_color: String,
    #[serde(default = "default_file_badge_color")]
    pub file_badge_color: String,
    #[serde(default = "default_dir_badge_color")]
    pub dir_badge_color: String,
    #[serde(default = "default_calc_badge_color")]
    pub calc_badge_color: String,
    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,
    #[serde(default = "default_true")]
    pub show_icons: Option<bool>,
    #[serde(default = "default_true")]
    pub show_status_bar: Option<bool>,
}

fn default_query_color() -> String { "cyan".to_string() }
fn default_selection_bg() -> String { "#2d3748".to_string() }
fn default_selection_fg() -> String { "white".to_string() }
fn default_app_badge_color() -> String { "cyan".to_string() }
fn default_file_badge_color() -> String { "yellow".to_string() }
fn default_dir_badge_color() -> String { "green".to_string() }
fn default_calc_badge_color() -> String { "magenta".to_string() }
fn default_border_color() -> String { "#4a5568".to_string() }
fn default_highlight_color() -> String { "#f6e05e".to_string() } // gold / yellow highlight
fn default_true() -> Option<bool> { Some(true) }

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            query_color: default_query_color(),
            selection_bg: default_selection_bg(),
            selection_fg: default_selection_fg(),
            app_badge_color: default_app_badge_color(),
            file_badge_color: default_file_badge_color(),
            dir_badge_color: default_dir_badge_color(),
            calc_badge_color: default_calc_badge_color(),
            border_color: default_border_color(),
            highlight_color: default_highlight_color(),
            show_icons: Some(true),
            show_status_bar: Some(true),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchPathConfig {
    pub path: String,
    pub max_depth: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub paths: Vec<SearchPathConfig>,
    #[serde(default = "default_ignored_dirs")]
    pub ignored_dirs: Vec<String>,
    #[serde(default = "default_ignored_extensions")]
    pub ignored_extensions: Vec<String>,
    #[serde(default = "default_true")]
    pub enable_path_matching: Option<bool>,
    #[serde(alias = "disable_fcitx", default)]
    pub disable_ime: Option<bool>,
}

fn default_max_depth() -> usize { 3 }
fn default_ignored_dirs() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".cargo".to_string(),
        ".cache".to_string(),
        "node_modules".to_string(),
        "target".to_string(),
        "build".to_string(),
        "dist".to_string(),
        ".venv".to_string(),
    ]
}
fn default_ignored_extensions() -> Vec<String> {
    vec![
        ".tmp".to_string(),
        ".o".to_string(),
        ".lock".to_string(),
        ".log".to_string(),
    ]
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: default_max_depth(),
            paths: vec![
                SearchPathConfig {
                    path: "~".to_string(),
                    max_depth: Some(2),
                }
            ],
            ignored_dirs: default_ignored_dirs(),
            ignored_extensions: default_ignored_extensions(),
            enable_path_matching: Some(true),
            disable_ime: Some(false),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomAppConfig {
    pub name: String,
    pub exec: String,
    pub description: Option<String>,
    #[serde(default)]
    pub terminal: Option<bool>,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppsConfig {
    #[serde(default)]
    pub pinned: Vec<String>,
    #[serde(default)]
    pub hidden: Vec<String>,
    #[serde(default)]
    pub extra_desktop_paths: Vec<String>,
    #[serde(default)]
    pub custom: Vec<CustomAppConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub apps: AppsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            search: SearchConfig::default(),
            apps: AppsConfig::default(),
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
