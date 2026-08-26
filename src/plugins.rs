/// Custom Script Plugin System for View Launcher (Rofi-like extensibility).
/// Loads executable scripts from ~/.config/view-launcher/plugins/ on demand.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use crate::launcher::{LauncherItem, ItemType};

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
}

/// Returns the path to the user plugins directory (~/.config/view-launcher/plugins/)
pub fn get_plugins_dir() -> PathBuf {
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(mut p) = dirs::config_dir() {
            p.push("view-launcher");
            p.push("plugins");
            let _ = fs::create_dir_all(&p);
            return p;
        }
        PathBuf::from("/tmp/view-launcher-plugins")
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(mut p) = dirs::config_dir() {
            p.push("view-launcher");
            p.push("plugins");
            let _ = fs::create_dir_all(&p);
            return p;
        }
        PathBuf::from(r"C:\ProgramData\view-launcher\plugins")
    }
}

/// Discovers all available custom plugin scripts in the plugins folder.
pub fn discover_plugins() -> Vec<PluginInfo> {
    let dir = get_plugins_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut plugins = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                if filename.starts_with('.') || filename.eq_ignore_ascii_case("README.md") {
                    continue;
                }

                let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                if !stem.is_empty() {
                    plugins.push(PluginInfo {
                        name: stem.to_lowercase(),
                        path,
                    });
                }
            }
        }
    }
    plugins
}

/// Runs a specific custom plugin script with the given query and parses its stdout.
pub fn execute_plugin(plugin: &PluginInfo, query: &str) -> Vec<LauncherItem> {
    let mut cmd = Command::new(&plugin.path);
    if !query.trim().is_empty() {
        cmd.arg(query.trim());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = match cmd.output() {
        Ok(out) => out,
        Err(_) => {
            return vec![LauncherItem::new(
                format!("Failed to execute @{}", plugin.name),
                plugin.path.to_string_lossy().to_string(),
                ItemType::Calc,
                Some(format!("Script path: {}", plugin.path.display())),
                false,
                None,
            )];
        }
    };

    if !output.status.success() && output.stdout.is_empty() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return vec![LauncherItem::new(
            format!("Error in plugin @{}", plugin.name),
            plugin.path.to_string_lossy().to_string(),
            ItemType::Calc,
            Some(if err_msg.is_empty() { "Script exited with error".to_string() } else { err_msg.trim().to_string() }),
            false,
            None,
        )];
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();

    for line in stdout_str.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Check if format is "Title | Subtitle | Icon | Exec"
        let parts: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
        if parts.len() >= 4 {
            items.push(LauncherItem::new(
                parts[0].to_string(),
                parts[3].to_string(),
                ItemType::App,
                Some(parts[1].to_string()),
                false,
                Some(parts[2].to_string()),
            ));
        } else if parts.len() == 3 {
            items.push(LauncherItem::new(
                parts[0].to_string(),
                parts[0].to_string(),
                ItemType::Calc,
                Some(parts[1].to_string()),
                false,
                Some(parts[2].to_string()),
            ));
        } else if parts.len() == 2 {
            items.push(LauncherItem::new(
                parts[0].to_string(),
                parts[0].to_string(),
                ItemType::Calc,
                Some(parts[1].to_string()),
                false,
                None,
            ));
        } else {
            items.push(LauncherItem::new(
                trimmed.to_string(),
                trimmed.to_string(),
                ItemType::Calc,
                Some(format!("Plugin @{}", plugin.name)),
                false,
                None,
            ));
        }
    }

    if items.is_empty() {
        items.push(LauncherItem::new(
            format!("No output from @{}", plugin.name),
            plugin.name.clone(),
            ItemType::Calc,
            Some(format!("Script executed: {}", plugin.path.display())),
            false,
            None,
        ));
    }

    items
}
