use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GeneralConfig {
    #[serde(default)]
    pub autostart: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_mode")]
    pub mode: String,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
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
    #[serde(default = "default_true")]
    pub compact_empty_view: Option<bool>,
}

fn default_theme_mode() -> String { "dark".to_string() }
fn default_opacity() -> f32 { 0.95 }
fn default_query_color() -> String { "#7aa2f7".to_string() }
fn default_selection_bg() -> String { "#283457".to_string() }
fn default_selection_fg() -> String { "#ffffff".to_string() }
fn default_border_color() -> String { "#3b4261".to_string() }
fn default_highlight_color() -> String { "#ff9e64".to_string() }
fn default_true() -> Option<bool> { Some(true) }

impl ThemeConfig {
    pub fn is_dark(&self) -> bool {
        match self.mode.to_lowercase().as_str() {
            "light" => false,
            "system" => is_system_dark_mode(),
            _ => true,
        }
    }
}

pub fn is_system_dark_mode() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("reg")
            .args(&["query", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", "/v", "AppsUseLightTheme"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("0x0") {
                return true;
            } else if text.contains("0x1") {
                return false;
            }
        }
        true
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(output) = std::process::Command::new("gsettings")
            .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if text.contains("prefer-dark") || text.contains("dark") {
                return true;
            } else if text.contains("prefer-light") {
                return false;
            }
        }
        if let Ok(theme_out) = std::process::Command::new("gsettings")
            .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
        {
            let theme_text = String::from_utf8_lossy(&theme_out.stdout).to_lowercase();
            if theme_text.contains("dark") {
                return true;
            }
        }
        true
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            mode: default_theme_mode(),
            opacity: default_opacity(),
            query_color: default_query_color(),
            selection_bg: default_selection_bg(),
            selection_fg: default_selection_fg(),
            border_color: default_border_color(),
            highlight_color: default_highlight_color(),
            show_icons: Some(true),
            show_status_bar: Some(true),
            compact_empty_view: Some(true),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchPathConfig {
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
    pub max_depth: Option<usize>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_depth() -> usize { 2 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub paths: Vec<SearchPathConfig>,
    #[serde(default = "default_true")]
    pub enable_path_matching: Option<bool>,
    #[serde(default)]
    pub ignored_dirs: Vec<String>,
    #[serde(default)]
    pub ignored_extensions: Vec<String>,
}

fn default_max_results() -> usize { 50 }

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: default_max_results(),
            max_depth: default_depth(),
            paths: Vec::new(),
            enable_path_matching: Some(true),
            ignored_dirs: Vec::new(),
            ignored_extensions: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomAppConfig {
    pub name: String,
    pub exec: String,
    pub description: Option<String>,
    #[serde(default)]
    pub terminal: bool,
    pub icon: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
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

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub apps: AppsConfig,
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
                if let Ok(parsed) = toml::from_str(&content) {
                    config = parsed;
                }
            }
        } else {
            // Write default config to disk if it doesn't exist
            if let Some(parent) = config_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(toml_str) = toml::to_string_pretty(&config) {
                let _ = fs::write(&config_path, toml_str);
            }
        }
        
        config.general.autostart = is_autostart_enabled();
        config
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path();
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let toml_str = toml::to_string_pretty(self)?;
        fs::write(config_path, toml_str)?;
        let _ = set_autostart(self.general.autostart);
        Ok(())
    }
}

/// Checks if autostart entry exists for current platform
pub fn is_autostart_enabled() -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(mut path) = dirs::config_dir() {
            path.push("autostart");
            path.push("view-launcher.desktop");
            return path.exists();
        }
        false
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(mut path) = dirs::config_dir() {
            path.push("Microsoft");
            path.push("Windows");
            path.push("Start Menu");
            path.push("Programs");
            path.push("Startup");
            path.push("view-launcher.bat");
            return path.exists();
        }
        false
    }
}

/// Sets or removes system autostart entry for current platform
pub fn set_autostart(enabled: bool) -> Result<(), std::io::Error> {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(mut path) = dirs::config_dir() {
            path.push("autostart");
            if enabled {
                fs::create_dir_all(&path)?;
                path.push("view-launcher.desktop");
                let desktop_entry = "[Desktop Entry]\nType=Application\nName=View Launcher\nComment=Minimalist, high-performance GUI desktop app & file launcher in Rust\nExec=view-launcher\nIcon=view-launcher\nTerminal=false\nHidden=false\nNoDisplay=false\nX-GNOME-Autostart-enabled=true\n";
                fs::write(&path, desktop_entry)?;
            } else {
                path.push("view-launcher.desktop");
                if path.exists() {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(mut path) = dirs::config_dir() {
            path.push("Microsoft");
            path.push("Windows");
            path.push("Start Menu");
            path.push("Programs");
            path.push("Startup");
            path.push("view-launcher.bat");
            if enabled {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let bat_content = "@start \"\" \"view-launcher.exe\"\n";
                fs::write(&path, bat_content)?;
            } else if path.exists() {
                let _ = fs::remove_file(&path);
            }
        }
    }
    Ok(())
}

/// Automatically registers global shortcut on supported desktop environments (GNOME, etc.)
pub fn setup_global_shortcut() {
    #[cfg(not(target_os = "windows"))]
    {
        let is_gnome = std::env::var("XDG_CURRENT_DESKTOP")
            .map(|d| d.to_lowercase().contains("gnome") || d.to_lowercase().contains("ubuntu"))
            .unwrap_or(false);

        if is_gnome {
            // Ensure GNOME automatically centers new windows in the middle of the screen
            let _ = std::process::Command::new("gsettings")
                .args(&["set", "org.gnome.mutter", "center-new-windows", "true"])
                .status();

            let output = std::process::Command::new("gsettings")
                .args(&["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"])
                .output();

            if let Ok(out) = output {
                let current = String::from_utf8_lossy(&out.stdout);
                let path = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/view-launcher/";
                if !current.contains("view-launcher") {
                    let mut list: Vec<String> = current
                        .trim()
                        .trim_start_matches('@')
                        .trim_start_matches("as")
                        .trim()
                        .trim_matches(|c| c == '[' || c == ']')
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty() && s != "''")
                        .collect();
                    list.push(format!("'{}'", path));
                    let new_val = format!("[{}]", list.join(", "));
                    let _ = std::process::Command::new("gsettings")
                        .args(&["set", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings", &new_val])
                        .status();
                    let _ = std::process::Command::new("gsettings")
                        .args(&["set", &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path), "name", "View Launcher"])
                        .status();
                    let _ = std::process::Command::new("gsettings")
                        .args(&["set", &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path), "command", "view-launcher"])
                        .status();
                    let _ = std::process::Command::new("gsettings")
                        .args(&["set", &format!("org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}", path), "binding", "<Control><Alt>space"])
                        .status();
                }
            }
        }
    }
}
