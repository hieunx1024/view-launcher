use std::path::Path;
use crate::launcher::ItemType;

pub fn get_icon(item_type: &ItemType, name: &str, path_or_exec: &str, show_icons: bool) -> (&'static str, &'static str) {
    if !show_icons {
        return match item_type {
            ItemType::App => ("[App] ", "app"),
            ItemType::File => ("[File] ", "file"),
            ItemType::Dir => ("[Dir]  ", "dir"),
            ItemType::Calc => ("[Calc] ", "calc"),
        };
    }

    match item_type {
        ItemType::Calc => ("󰃬  ", "calc"),
        ItemType::App => {
            let lower_name = name.to_lowercase();
            let lower_exec = path_or_exec.to_lowercase();

            if lower_name.contains("firefox") || lower_exec.contains("firefox") {
                ("󰈹  ", "app_browser")
            } else if lower_name.contains("chrome") || lower_name.contains("chromium") || lower_name.contains("brave") {
                ("󰊯  ", "app_browser")
            } else if lower_name.contains("terminal") || lower_name.contains("kitty") || lower_name.contains("alacritty") || lower_name.contains("wezterm") || lower_name.contains("ghostty") {
                ("  ", "app_terminal")
            } else if lower_name.contains("code") || lower_name.contains("visual studio") {
                ("󰨞  ", "app_code")
            } else if lower_name.contains("nvim") || lower_name.contains("neovim") || lower_name.contains("vim") {
                ("  ", "app_code")
            } else if lower_name.contains("spotify") || lower_name.contains("music") {
                ("󰓇  ", "app_media")
            } else if lower_name.contains("vlc") || lower_name.contains("mpv") || lower_name.contains("video") {
                ("󰕼  ", "app_media")
            } else if lower_name.contains("discord") || lower_name.contains("telegram") || lower_name.contains("slack") {
                ("󰭹  ", "app_chat")
            } else if lower_name.contains("settings") || lower_name.contains("control") {
                ("󰒓  ", "app_settings")
            } else if lower_name.contains("lock") {
                ("󰌾  ", "app_lock")
            } else if lower_name.contains("power") || lower_name.contains("reboot") || lower_name.contains("shutdown") {
                ("󰜉  ", "app_power")
            } else {
                ("󰀻  ", "app")
            }
        }
        ItemType::Dir => ("  ", "dir"),
        ItemType::File => {
            let ext = Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match ext.as_str() {
                "rs" => ("  ", "file_code"),
                "py" => ("  ", "file_code"),
                "js" | "mjs" | "cjs" => ("  ", "file_code"),
                "ts" | "tsx" | "jsx" => ("  ", "file_code"),
                "go" => ("  ", "file_code"),
                "c" | "h" => ("  ", "file_code"),
                "cpp" | "hpp" | "cc" => ("  ", "file_code"),
                "java" | "kt" => ("  ", "file_code"),
                "sh" | "bash" | "zsh" | "fish" => ("  ", "file_code"),
                "html" | "css" | "scss" => ("󰌝  ", "file_code"),
                "json" | "toml" | "yaml" | "yml" => ("  ", "file_config"),
                "md" | "txt" | "org" => ("  ", "file_text"),
                "pdf" => ("  ", "file_pdf"),
                "png" | "jpg" | "jpeg" | "svg" | "gif" | "webp" => ("󰋩  ", "file_image"),
                "mp4" | "mkv" | "avi" | "mov" => ("󰕧  ", "file_media"),
                "mp3" | "flac" | "wav" | "ogg" => ("󰎆  ", "file_media"),
                "zip" | "tar" | "gz" | "7z" | "rar" | "xz" => ("  ", "file_archive"),
                _ => ("  ", "file"),
            }
        }
    }
}
