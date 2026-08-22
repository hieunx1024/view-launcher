use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeConfig {
    #[serde(default = "default_query_color")]
    pub query_color: String,
    #[serde(default = "default_selection_bg")]
    pub selection_bg: String,
    #[serde(default = "default_selection_fg")]
    pub selection_fg: String,
    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,
    #[serde(default = "default_true")]
    pub show_icons: Option<bool>,
    #[serde(default = "default_true")]
    pub show_status_bar: Option<bool>,
}

fn default_query_color() -> String { "#7aa2f7".to_string() }
fn default_selection_bg() -> String { "#283457".to_string() }
fn default_selection_fg() -> String { "#ffffff".to_string() }
fn default_border_color() -> String { "#3b4261".to_string() }
fn default_highlight_color() -> String { "#ff9e64".to_string() }
fn default_true() -> Option<bool> { Some(true) }

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            query_color: default_query_color(),
            selection_bg: default_selection_bg(),
            selection_fg: default_selection_fg(),
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
    #[serde(default = "default_depth")]
    pub depth: usize,
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_depth() -> usize { 2 }

#[derive(Debug, Deserialize, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default)]
    pub paths: Vec<SearchPathConfig>,
    #[serde(default = "default_true")]
    pub enable_path_matching: Option<bool>,
}

fn default_max_results() -> usize { 50 }

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: default_max_results(),
            paths: Vec::new(),
            enable_path_matching: Some(true),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct CustomAppConfig {
    pub name: String,
    pub exec: String,
    #[serde(default)]
    pub terminal: bool,
    pub icon: Option<String>,
    pub category: Option<String>,
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
    pub fn get_config_path() -> PathBuf {
        if let Some(mut config_path) = dirs::config_dir() {
            config_path.push("view-launcher");
            config_path.push("config.toml");
            config_path
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load() -> Self {
        let mut config = Self::default();
        let config_path = Self::get_config_path();
        
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(parsed) = toml::from_str::<Config>(&content) {
                    config = parsed;
                }
            }
        }
        config
    }
}
