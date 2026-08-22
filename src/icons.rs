use std::path::Path;
use crate::launcher::ItemType;

/// Determines the best Nerd Font glyph and semantic category for any application, file, or directory.
pub fn get_icon(
    item_type: &ItemType,
    name: &str,
    path_or_exec: &str,
    icon_hint: Option<&str>,
    show_icons: bool,
) -> (&'static str, &'static str) {
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
            let lower_icon = icon_hint.unwrap_or("").to_lowercase();

            // Combined search target string
            let target = format!("{} {} {}", lower_name, lower_exec, lower_icon);

            // 1. Browsers
            if target.contains("firefox") {
                ("󰈹  ", "app_browser")
            } else if target.contains("chrome") || target.contains("chromium") || target.contains("brave") || target.contains("google-chrome") {
                ("󰊯  ", "app_browser")
            } else if target.contains("edge") || target.contains("msedge") {
                ("󰇩  ", "app_browser")
            } else if target.contains("opera") || target.contains("vivaldi") || target.contains("tor-browser") || target.contains("zen") {
                ("󰈹  ", "app_browser")
            }
            // 2. IDEs & Editors
            else if target.contains("idea") || target.contains("intellij") {
                ("  ", "app_code")
            } else if target.contains("pycharm") {
                ("󱤓  ", "app_code")
            } else if target.contains("clion") {
                ("  ", "app_code")
            } else if target.contains("webstorm") || target.contains("phpstorm") || target.contains("rubymine") {
                ("  ", "app_code")
            } else if target.contains("android-studio") || target.contains("android studio") {
                ("󰀲  ", "app_code")
            } else if target.contains("code") || target.contains("vscode") || target.contains("visual studio") || target.contains("vscodium") {
                ("󰨞  ", "app_code")
            } else if target.contains("nvim") || target.contains("neovim") || target.contains("vim") || target.contains("gvim") {
                ("  ", "app_code")
            } else if target.contains("sublime") {
                ("  ", "app_code")
            } else if target.contains("emacs") {
                ("  ", "app_code")
            } else if target.contains("gedit") || target.contains("kate") || target.contains("mousepad") || target.contains("notepad") || target.contains("text editor") {
                ("  ", "app_code")
            }
            // 3. Database Tools
            else if target.contains("dbeaver") || target.contains("datagrip") || target.contains("database") || target.contains("mysql") || target.contains("postgres") || target.contains("sqlite") || target.contains("redis") || target.contains("mongo") || target.contains("robo3t") || target.contains("compass") || target.contains("dbeaver-ce") {
                ("󰆼  ", "app_db")
            }
            // 4. Terminals
            else if target.contains("terminal") || target.contains("kitty") || target.contains("alacritty") || target.contains("wezterm") || target.contains("ghostty") || target.contains("konsole") || target.contains("tilix") || target.contains("terminator") || target.contains("foot") || target.contains("urxvt") || target.contains("xterm") {
                ("  ", "app_terminal")
            }
            // 5. Chat & Communication
            else if target.contains("viber") {
                ("󰍡  ", "app_chat")
            } else if target.contains("telegram") {
                ("󰭹  ", "app_chat")
            } else if target.contains("discord") {
                ("󰭹  ", "app_chat")
            } else if target.contains("slack") {
                ("󰒱  ", "app_chat")
            } else if target.contains("whatsapp") {
                ("󰖣  ", "app_chat")
            } else if target.contains("teams") {
                ("󰊻  ", "app_chat")
            } else if target.contains("skype") {
                ("󰒯  ", "app_chat")
            } else if target.contains("zoom") || target.contains("meet") {
                ("󰍫  ", "app_chat")
            } else if target.contains("signal") || target.contains("wechat") || target.contains("element") {
                ("󰭹  ", "app_chat")
            }
            // 6. Media & Audio & Video
            else if target.contains("spotify") || target.contains("music") || target.contains("rhythmbox") || target.contains("audacious") {
                ("󰓇  ", "app_media")
            } else if target.contains("vlc") || target.contains("mpv") || target.contains("video") || target.contains("celluloid") || target.contains("totem") {
                ("󰕼  ", "app_media")
            } else if target.contains("obs") || target.contains("kazam") || target.contains("screen recorder") {
                ("󰑋  ", "app_media")
            } else if target.contains("audacity") || target.contains("ardour") || target.contains("reaper") {
                ("󰎆  ", "app_media")
            } else if target.contains("gimp") || target.contains("photoshop") || target.contains("krita") || target.contains("inkscape") || target.contains("blender") || target.contains("figma") || target.contains("draw") {
                ("󰽉  ", "app_graphics")
            } else if target.contains("image") || target.contains("photo") || target.contains("viewer") || target.contains("loupe") || target.contains("eog") || target.contains("shotwell") || target.contains("gwenview") {
                ("󰋩  ", "app_graphics")
            }
            // 7. Office & Documents & Notes
            else if target.contains("writer") || target.contains("word") {
                ("󰈬  ", "app_office")
            } else if target.contains("calc") || target.contains("excel") || target.contains("spreadsheet") {
                ("󰈛  ", "app_office")
            } else if target.contains("impress") || target.contains("powerpoint") || target.contains("presentation") {
                ("󰈧  ", "app_office")
            } else if target.contains("libreoffice") || target.contains("office") || target.contains("wps") {
                ("󰏆  ", "app_office")
            } else if target.contains("pdf") || target.contains("evince") || target.contains("document viewer") || target.contains("okular") {
                ("󰈦  ", "app_office")
            } else if target.contains("obsidian") || target.contains("notion") || target.contains("logseq") || target.contains("joplin") || target.contains("notes") {
                ("󱞁  ", "app_office")
            } else if target.contains("mail") || target.contains("thunderbird") || target.contains("evolution") || target.contains("outlook") {
                ("󰇮  ", "app_office")
            } else if target.contains("calendar") {
                ("󰃭  ", "app_office")
            }
            // 8. DevOps & Developer Tools
            else if target.contains("docker") || target.contains("podman") || target.contains("kubernetes") || target.contains("container") {
                ("󰡨  ", "app_code")
            } else if target.contains("git") || target.contains("github") || target.contains("gitlab") || target.contains("gitkraken") {
                ("  ", "app_code")
            } else if target.contains("postman") || target.contains("insomnia") || target.contains("hoppscotch") {
                ("󱘲  ", "app_code")
            } else if target.contains("wireshark") || target.contains("nmap") {
                ("󰙀  ", "app_system")
            } else if target.contains("filezilla") || target.contains("winscp") || target.contains("putty") {
                ("󰛳  ", "app_system")
            }
            // 9. System, Hardware & Utilities
            else if target.contains("calculator") || target.contains("kcalc") || target.contains("galculator") {
                ("󰃬  ", "app_calc")
            } else if target.contains("system monitor") || target.contains("task manager") || target.contains("htop") || target.contains("btop") || target.contains("systemmonitor") {
                ("󰓅  ", "app_system")
            } else if target.contains("disk") || target.contains("baobab") || target.contains("gparted") || target.contains("storage") || target.contains("drive") {
                ("󰋊  ", "app_system")
            } else if target.contains("driver") || target.contains("hardware") || target.contains("additional drivers") {
                ("󰘚  ", "app_system")
            } else if target.contains("log") || target.contains("logs") || target.contains("event") {
                ("󰌱  ", "app_system")
            } else if target.contains("input method") || target.contains("ibus") || target.contains("fcitx") || target.contains("keyboard") {
                ("󰌌  ", "app_system")
            } else if target.contains("font") || target.contains("fonts") || target.contains("character map") {
                ("󰬬  ", "app_system")
            } else if target.contains("password") || target.contains("keyring") || target.contains("seahorse") || target.contains("bitwarden") || target.contains("keepass") {
                ("󰷡  ", "app_security")
            } else if target.contains("clock") || target.contains("time") || target.contains("alarm") {
                ("󰥔  ", "app_system")
            } else if target.contains("network") || target.contains("wifi") || target.contains("wireless") || target.contains("ethernet") {
                ("󰖩  ", "app_system")
            } else if target.contains("bluetooth") {
                ("󰂯  ", "app_system")
            } else if target.contains("sound") || target.contains("audio") || target.contains("volume") || target.contains("pavucontrol") {
                ("󰕾  ", "app_system")
            } else if target.contains("camera") || target.contains("webcam") || target.contains("cheese") {
                ("󰄀  ", "app_system")
            } else if target.contains("print") || target.contains("cups") {
                ("󰐪  ", "app_system")
            } else if target.contains("archive") || target.contains("file-roller") || target.contains("ark") || target.contains("7z") || target.contains("zip") {
                ("  ", "app_system")
            } else if target.contains("software") || target.contains("app center") || target.contains("store") || target.contains("synaptic") || target.contains("pamac") {
                ("󰏓  ", "app_system")
            } else if target.contains("setting") || target.contains("control") || target.contains("tweak") || target.contains("preference") {
                ("󰒓  ", "app_settings")
            } else if target.contains("info") || target.contains("help") || target.contains("texinfo") || target.contains("doc") {
                ("󰂺  ", "app_system")
            } else if target.contains("file") || target.contains("nautilus") || target.contains("dolphin") || target.contains("thunar") || target.contains("nemo") || target.contains("explorer") {
                ("󰉋  ", "app_system")
            } else if target.contains("lock") {
                ("󰌾  ", "app_lock")
            } else if target.contains("power") || target.contains("reboot") || target.contains("shutdown") || target.contains("logout") || target.contains("restart") {
                ("󰜉  ", "app_power")
            }
            // 10. Games
            else if target.contains("steam") || target.contains("lutris") || target.contains("heroic") || target.contains("game") || target.contains("retroarch") || target.contains("minecraft") {
                ("󰊴  ", "app_game")
            }
            // Fallback default app icon
            else {
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
