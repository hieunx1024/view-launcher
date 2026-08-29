use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;
use rayon::prelude::*;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use crate::config::Config;
use crate::history::HistoryManager;
use crate::calc;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
unsafe extern "C" {
    fn setsid() -> i32;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemType {
    App,
    File,
    Dir,
    Calc,
    System,
    Window,
    Clipboard,
    Dmenu,
    Theme,
}

#[derive(Debug, Clone)]
pub struct LauncherItem {
    pub name: String,
    pub normalized_name: String,
    pub exec_or_path: String,
    pub item_type: ItemType,
    pub description: Option<String>,
    pub terminal: bool,
    pub icon: Option<String>,
}

impl LauncherItem {
    pub fn new(
        name: String,
        exec_or_path: String,
        item_type: ItemType,
        description: Option<String>,
        terminal: bool,
        icon: Option<String>,
    ) -> Self {
        let normalized_name = remove_vietnamese_accents(&name);
        Self {
            name,
            normalized_name,
            exec_or_path,
            item_type,
            description,
            terminal,
            icon,
        }
    }
    pub fn get_category_tag(&self) -> &'static str {
        match self.item_type {
            ItemType::Theme => "Theme",
            ItemType::Calc => "Calc",
            ItemType::Dir => "Folder",
            ItemType::File => "File",
            ItemType::System => "System",
            ItemType::Window => "Window",
            ItemType::Clipboard => "Clipboard",
            ItemType::Dmenu => "Item",
            ItemType::App => {
                let lower = format!("{} {} {}", self.name.to_lowercase(), self.exec_or_path.to_lowercase(), self.icon.as_deref().unwrap_or(""));
                if lower.contains("idea") || lower.contains("pycharm") || lower.contains("clion") || lower.contains("webstorm") || lower.contains("code") || lower.contains("studio") || lower.contains("nvim") || lower.contains("vim") {
                    "IDE"
                } else if lower.contains("dbeaver") || lower.contains("datagrip") || lower.contains("database") || lower.contains("mysql") || lower.contains("postgres") || lower.contains("redis") || lower.contains("mongo") {
                    "Database"
                } else if lower.contains("firefox") || lower.contains("chrome") || lower.contains("brave") || lower.contains("edge") || lower.contains("browser") {
                    "Browser"
                } else if lower.contains("viber") || lower.contains("telegram") || lower.contains("discord") || lower.contains("slack") || lower.contains("teams") || lower.contains("chat") {
                    "Chat"
                } else if lower.contains("spotify") || lower.contains("vlc") || lower.contains("music") || lower.contains("video") || lower.contains("obs") || lower.contains("gimp") || lower.contains("blender") {
                    "Media"
                } else if lower.contains("writer") || lower.contains("calc") || lower.contains("office") || lower.contains("word") || lower.contains("excel") || lower.contains("notes") || lower.contains("pdf") {
                    "Office"
                } else if lower.contains("terminal") || lower.contains("system") || lower.contains("monitor") || lower.contains("disk") || lower.contains("driver") || lower.contains("setting") || lower.contains("network") {
                    "System"
                } else {
                    "App"
                }
            }
        }
    }
}

pub fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

pub fn open_config_file() {
    let config_path = crate::config::Config::get_config_path();
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&config_path)
            .spawn();
    }
    #[cfg(target_os = "windows")]
    {
        #[cfg(windows)]
        use std::os::windows::process::CommandExt;
        #[allow(unused_mut)]
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(&["/C", "start", "", &config_path.to_string_lossy()]);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        let _ = cmd.spawn();
    }
}

pub struct LauncherEngine {
    pub apps: Vec<LauncherItem>,
    pub custom_apps: Vec<LauncherItem>,
    pub pinned_apps: Vec<String>,
    pub hidden_apps: Vec<String>,
    pub shallow_files: Arc<RwLock<Vec<LauncherItem>>>,
    pub history: Arc<RwLock<HistoryManager>>,
    pub clipboard: Arc<crate::clipboard::ClipboardManager>,
    matcher: SkimMatcherV2,
    config: Config,
}

impl LauncherEngine {
    pub fn new(config: Config) -> Self {
        let history = Arc::new(RwLock::new(HistoryManager::load()));
        let clipboard = Arc::new(crate::clipboard::ClipboardManager::new());
        let mut engine = Self {
            apps: Vec::new(),
            custom_apps: Vec::new(),
            pinned_apps: config.apps.pinned.clone(),
            hidden_apps: config.apps.hidden.clone(),
            shallow_files: Arc::new(RwLock::new(Vec::new())),
            history,
            clipboard,
            matcher: SkimMatcherV2::default(),
            config: config.clone(),
        };

        // Load custom apps from config
        for custom in &config.apps.custom {
            engine.custom_apps.push(LauncherItem::new(
                custom.name.clone(),
                custom.exec.clone(),
                ItemType::App,
                custom.description.clone(),
                custom.terminal,
                custom.icon.clone(),
            ));
        }

        engine.index_apps();
        
        let shallow_files_clone = engine.shallow_files.clone();
        let config_clone = config.clone();
        std::thread::spawn(move || {
            let mut files = Vec::new();
            Self::index_files_impl(&config_clone, &mut files);
            if let Ok(mut lock) = shallow_files_clone.write() {
                *lock = files;
            }
        });
        
        engine
    }

    /// Indexes all standard Linux .desktop application entries + extra paths from config.
    #[cfg(not(target_os = "windows"))]
    fn index_apps(&mut self) {
        let mut paths = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            PathBuf::from("/var/lib/snapd/desktop/applications"),
            PathBuf::from("/var/lib/flatpak/exports/share/applications"),
            dirs::home_dir().map(|mut h| {
                h.push(".local/share/applications");
                h
            }).unwrap_or_default(),
            dirs::home_dir().map(|mut h| {
                h.push(".local/share/flatpak/exports/share/applications");
                h
            }).unwrap_or_default(),
        ];

        // Parse standard $XDG_DATA_DIRS (e.g. Ubuntu snap / flatpak / desktop entries)
        if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir_str in xdg_data_dirs.split(':') {
                let trimmed = dir_str.trim();
                if !trimmed.is_empty() {
                    let mut p = PathBuf::from(trimmed);
                    if p.file_name().map_or(true, |f| f != "applications") {
                        p.push("applications");
                    }
                    if !paths.contains(&p) {
                        paths.push(p);
                    }
                }
            }
        }

        // Add extra desktop paths from config
        for extra in &self.config.apps.extra_desktop_paths {
            let expanded = expand_tilde(extra);
            let p = PathBuf::from(expanded);
            if !paths.contains(&p) {
                paths.push(p);
            }
        }

        for path in paths {
            if !path.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let file_path = entry.path();
                    if file_path.extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(app) = self.parse_desktop_file(&file_path) {
                            // Check if app is in hidden list
                            let is_hidden = self.hidden_apps.iter().any(|h| {
                                app.name.eq_ignore_ascii_case(h) || app.exec_or_path.contains(h)
                            });

                            if !is_hidden && !self.apps.iter().any(|item| item.name == app.name) {
                                self.apps.push(app);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Indexes all standard Windows shortcut entries (.lnk and .url) from Start Menu and Desktop.
    #[cfg(target_os = "windows")]
    fn index_apps(&mut self) {
        let mut paths = Vec::new();
        if let Some(mut path) = dirs::config_dir() {
            path.push("Microsoft");
            path.push("Windows");
            path.push("Start Menu");
            path.push("Programs");
            paths.push(path);
        }
        paths.push(PathBuf::from(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs"));
        if let Some(desktop) = dirs::desktop_dir() {
            paths.push(desktop);
        }
        if let Ok(public_dir) = std::env::var("PUBLIC") {
            let p = PathBuf::from(public_dir).join("Desktop");
            if p.exists() && !paths.contains(&p) {
                paths.push(p);
            }
        }

        for path in paths {
            if !path.exists() {
                continue;
            }
            for entry in WalkDir::new(path).into_iter().flatten() {
                let file_path = entry.path();
                let is_app_shortcut = file_path.extension().map_or(false, |ext| {
                    ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("url")
                });

                if is_app_shortcut {
                    let name = file_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let exec = file_path.to_string_lossy().to_string();
                    
                    let is_hidden = self.hidden_apps.iter().any(|h| name.eq_ignore_ascii_case(h));
                    if !is_hidden && !self.apps.iter().any(|item| item.name.eq_ignore_ascii_case(&name)) {
                        self.apps.push(LauncherItem::new(
                            name,
                            exec.clone(),
                            ItemType::App,
                            Some("Windows App".to_string()),
                            false,
                            Some(exec),
                        ));
                    }
                }
            }
        }
    }

    /// Parses a Linux .desktop entry file line by line to extract the core fields.
    #[allow(dead_code)]
    fn parse_desktop_file(&self, path: &Path) -> Option<LauncherItem> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        
        let mut name = String::new();
        let mut exec = String::new();
        let mut comment = None;
        let mut icon_hint = None;
        let mut is_app = false;
        let mut no_display = false;
        let mut hidden = false;
        let mut terminal = false;
        let mut in_desktop_entry = false;

        for line in reader.lines().flatten() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_desktop_entry = trimmed == "[Desktop Entry]";
                continue;
            }

            if !in_desktop_entry {
                continue;
            }

            if let Some(idx) = trimmed.find('=') {
                let key = trimmed[..idx].trim();
                let val = trimmed[idx + 1..].trim();

                match key {
                    "Type" => {
                        if val == "Application" {
                            is_app = true;
                        }
                    }
                    "Name" => {
                        if name.is_empty() {
                            name = val.to_string();
                        }
                    }
                    "Exec" => {
                        let tokens: Vec<&str> = val.split_whitespace().collect();
                        let mut cleaned_tokens = Vec::new();
                        for token in tokens {
                            // Skip any placeholder starting with '%'
                            if token.starts_with('%') {
                                continue;
                            }
                            cleaned_tokens.push(token);
                        }
                        if let Some(&"--") = cleaned_tokens.last() {
                            cleaned_tokens.pop();
                        }
                        exec = cleaned_tokens.join(" ");
                    }
                    "Comment" => {
                        comment = Some(val.to_string());
                    }
                    "Icon" => {
                        if icon_hint.is_none() && !val.is_empty() {
                            icon_hint = Some(val.to_string());
                        }
                    }
                    "NoDisplay" => {
                        if val == "true" {
                            no_display = true;
                        }
                    }
                    "Hidden" => {
                        if val == "true" {
                            hidden = true;
                        }
                    }
                    "Terminal" => {
                        if val == "true" {
                            terminal = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if is_app && !name.is_empty() && !exec.is_empty() && !no_display && !hidden {
            Some(LauncherItem::new(
                name,
                exec,
                ItemType::App,
                comment,
                terminal,
                icon_hint,
            ))
        } else {
            None
        }
    }

    /// Indexes files based on configured `search.paths` or defaults.
    fn index_files_impl(config: &Config, files: &mut Vec<LauncherItem>) {
        let search_paths = if config.search.paths.is_empty() {
            vec![crate::config::SearchPathConfig {
                path: "~".to_string(),
                depth: config.search.max_depth,
                max_depth: Some(config.search.max_depth),
                exclude: Vec::new(),
            }]
        } else {
            config.search.paths.clone()
        };

        let ignored_dirs = &config.search.ignored_dirs;
        let ignored_exts = &config.search.ignored_extensions;

        for search_entry in search_paths {
            let expanded_path_str = expand_tilde(&search_entry.path);
            let root_path = PathBuf::from(expanded_path_str);
            if !root_path.exists() {
                continue;
            }

            let max_depth = search_entry.max_depth.unwrap_or(config.search.max_depth);

            for entry in WalkDir::new(&root_path)
                .max_depth(max_depth)
                .into_iter()
                .filter_entry(|e| {
                    if let Some(name) = e.file_name().to_str() {
                        !ignored_dirs.iter().any(|ignored| name == ignored) && !name.starts_with('.')
                    } else {
                        false
                    }
                })
                .flatten()
            {
                if files.len() >= 5000 {
                    break;
                }
                let path = entry.path();
                if path == root_path {
                    continue;
                }

                // Check ignored extensions
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_dot = format!(".{}", ext.to_lowercase());
                    if ignored_exts.iter().any(|ig| ig.eq_ignore_ascii_case(&ext_dot) || ig.eq_ignore_ascii_case(ext)) {
                        continue;
                    }
                }

                let path_str = path.to_string_lossy().to_string();
                let name = entry.file_name().to_string_lossy().to_string();
                
                let item_type = if entry.file_type().is_dir() {
                    ItemType::Dir
                } else {
                    ItemType::File
                };

                let desc = path.parent().map(|p| {
                    let s = p.to_string_lossy().to_string();
                    if let Some(home) = dirs::home_dir() {
                        let home_str = home.to_string_lossy().to_string();
                        if s.starts_with(&home_str) {
                            return s.replacen(&home_str, "~", 1);
                        }
                    }
                    s
                });

                files.push(LauncherItem::new(
                    name,
                    path_str,
                    item_type,
                    desc,
                    false,
                    None,
                ));
            }
        }
    }

    /// Resolves dynamic path searching (e.g. typing `~/Downloads/` directly lists Downloads contents)
    pub fn resolve_path_search(&self, input: &str) -> Option<(PathBuf, String)> {
        if !input.contains('/') && input != "~" {
            return None;
        }

        let home = dirs::home_dir()?;
        let expanded = if input.starts_with("~/") {
            input.replacen("~/", &format!("{}/", home.to_string_lossy()), 1)
        } else if input == "~" {
            format!("{}/", home.to_string_lossy())
        } else {
            input.to_string()
        };

        let path = PathBuf::from(&expanded);
        
        if expanded.ends_with('/') {
            if path.is_dir() {
                Some((path, String::new()))
            } else {
                None
            }
        } else if let Some(parent) = path.parent() {
            if parent.is_dir() {
                let filter = path.file_name()?.to_string_lossy().to_string();
                Some((parent.to_path_buf(), filter))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Scans a specific directory recursively up to `max_depth` for filtering sub-items.
    pub fn scan_dir_recursive(&self, dir: &Path, max_depth: usize) -> Vec<LauncherItem> {
        let mut items = Vec::new();
        let ignored_dirs = &self.config.search.ignored_dirs;
        let ignored_exts = &self.config.search.ignored_extensions;

        for entry in WalkDir::new(dir)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                if let Some(name) = e.file_name().to_str() {
                    !ignored_dirs.iter().any(|ignored| name == ignored) && !name.starts_with('.')
                } else {
                    false
                }
            })
            .flatten()
        {
            if items.len() >= 1000 {
                break;
            }
            let path = entry.path();
            if path == dir {
                continue;
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_dot = format!(".{}", ext.to_lowercase());
                if ignored_exts.iter().any(|ig| ig.eq_ignore_ascii_case(&ext_dot) || ig.eq_ignore_ascii_case(ext)) {
                    continue;
                }
            }

            let path_str = path.to_string_lossy().to_string();
            let name = entry.file_name().to_string_lossy().to_string();
            let item_type = if entry.file_type().is_dir() {
                ItemType::Dir
            } else {
                ItemType::File
            };

            let desc = path.parent().map(|p| {
                let s = p.to_string_lossy().to_string();
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy().to_string();
                    if s.starts_with(&home_str) {
                        return s.replacen(&home_str, "~", 1);
                    }
                }
                s
            });

            items.push(LauncherItem::new(
                name,
                path_str,
                item_type,
                desc,
                false,
                None,
            ));
        }
        items
    }

    /// Scans a specific directory on-the-fly for quick sub-folder traversal.
    pub fn scan_dir_on_the_fly(&self, dir: &Path) -> Vec<LauncherItem> {
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let path_str = path.to_string_lossy().to_string();
                
                if path.is_dir() {
                    dirs.push(LauncherItem::new(
                        name,
                        path_str,
                        ItemType::Dir,
                        Some(dir.to_string_lossy().to_string()),
                        false,
                        None,
                    ));
                } else {
                    files.push(LauncherItem::new(
                        name,
                        path_str,
                        ItemType::File,
                        Some(dir.to_string_lossy().to_string()),
                        false,
                        None,
                    ));
                }
            }
        }
        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        dirs.extend(files);
        dirs
    }

    /// Performs high-performance fuzzy matching and ranking of items.
    /// Mặc định: Chỉ tìm ứng dụng & tính toán (cực nhanh, không lẫn file).
    /// Khi bắt đầu bằng `@f ` hoặc `@file `: Chuyển sang chế độ tìm kiếm file & thư mục.
    pub fn search(&self, query: &str) -> Vec<(LauncherItem, Vec<usize>)> {
        let trimmed_query = query.trim();
        let query_normalized = remove_vietnamese_accents(trimmed_query);
        let shallow_files_guard = self.shallow_files.read().unwrap_or_else(|e| e.into_inner());
        let history_guard = self.history.read().unwrap_or_else(|e| e.into_inner());

        let mut results = Vec::new();

        // 1. Window Switcher Mode (@w or @win)
        let is_win_mode = trimmed_query.starts_with("@w ") || trimmed_query.starts_with("@win ")
            || trimmed_query == "@w" || trimmed_query == "@win";
        if is_win_mode {
            let win_query = if trimmed_query.starts_with("@win ") {
                trimmed_query[5..].trim()
            } else if trimmed_query.starts_with("@w ") {
                trimmed_query[3..].trim()
            } else {
                ""
            };
            let win_query_normalized = remove_vietnamese_accents(win_query);
            let windows = crate::window_switcher::get_open_windows(&self.apps);
            let items: Vec<LauncherItem> = windows.into_iter().map(|w| {
                let icon = self.apps.iter().find(|a| a.name == w.title || a.exec_or_path == w.id.replace("app:", "")).and_then(|a| a.icon.clone());
                LauncherItem::new(
                    w.title,
                    w.id,
                    ItemType::Window,
                    Some(w.class_name),
                    false,
                    icon,
                )
            }).collect();

            if win_query.is_empty() {
                for item in items {
                    results.push((item, Vec::new()));
                }
                return results;
            }

            let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = items
                .par_iter()
                .filter_map(|item| {
                    let match_res = self.match_precomputed(
                        &item.name,
                        &item.normalized_name,
                        win_query,
                        &win_query_normalized,
                    );
                    let final_res = if match_res.is_some() {
                        match_res
                    } else if let Some(ref desc) = item.description {
                        self.match_item(desc, win_query)
                    } else {
                        None
                    };

                    final_res.map(|(score, indices)| ((item.clone(), indices), score))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            for (item_with_indices, _) in matches {
                results.push(item_with_indices);
            }
            return results;
        }

        // 2. Clipboard History Mode (@c or @clip)
        let is_clip_mode = trimmed_query.starts_with("@c ") || trimmed_query.starts_with("@clip ")
            || trimmed_query == "@c" || trimmed_query == "@clip";
        if is_clip_mode {
            let clip_query = if trimmed_query.starts_with("@clip ") {
                trimmed_query[6..].trim()
            } else if trimmed_query.starts_with("@c ") {
                trimmed_query[3..].trim()
            } else {
                ""
            };
            let clip_query_normalized = remove_vietnamese_accents(clip_query);
            let history = self.clipboard.get_history();
            let items: Vec<LauncherItem> = history.into_iter().map(|text| {
                let one_line = text.lines().next().unwrap_or("").trim().to_string();
                let snippet = if one_line.len() > 70 {
                    format!("{}...", &one_line[..70])
                } else if one_line.is_empty() {
                    "Clipboard text".to_string()
                } else {
                    one_line
                };
                LauncherItem::new(
                    snippet,
                    text.clone(),
                    ItemType::Clipboard,
                    Some(format!("{} chars", text.len())),
                    false,
                    None,
                )
            }).collect();

            if clip_query.is_empty() {
                for item in items {
                    results.push((item, Vec::new()));
                }
                return results;
            }

            let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = items
                .par_iter()
                .filter_map(|item| {
                    let match_res = self.match_precomputed(
                        &item.name,
                        &item.normalized_name,
                        clip_query,
                        &clip_query_normalized,
                    );
                    let final_res = if match_res.is_some() {
                        match_res
                    } else {
                        self.match_item(&item.exec_or_path, clip_query)
                    };
                    final_res.map(|(score, indices)| ((item.clone(), indices), score))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            for (item_with_indices, _) in matches {
                results.push(item_with_indices);
            }
            return results;
        }

        // 3. System / Power Actions (@sys or @power or keywords)
        let is_sys_mode = trimmed_query.starts_with("@sys ") || trimmed_query.starts_with("@power ")
            || trimmed_query == "@sys" || trimmed_query == "@power";
        let is_sys_keyword = ["lock", "shutdown", "power off", "restart", "reboot", "sleep", "suspend", "logout", "log out"]
            .iter().any(|&k| trimmed_query.eq_ignore_ascii_case(k));

        if is_sys_mode || (is_sys_keyword && !trimmed_query.is_empty()) {
            let sys_query = if trimmed_query.starts_with("@power ") {
                trimmed_query[7..].trim()
            } else if trimmed_query.starts_with("@sys ") {
                trimmed_query[5..].trim()
            } else if is_sys_mode {
                ""
            } else {
                trimmed_query
            };
            let sys_query_normalized = remove_vietnamese_accents(sys_query);
            let actions = crate::system_actions::get_system_actions();

            if sys_query.is_empty() {
                for item in actions {
                    results.push((item, Vec::new()));
                }
                return results;
            }

            let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = actions
                .into_iter()
                .filter_map(|item| {
                    let (score, indices) = self.match_precomputed(
                        &item.name,
                        &item.normalized_name,
                        sys_query,
                        &sys_query_normalized,
                    )?;
                    Some(((item, indices), score + 300))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            for (item_with_indices, _) in matches {
                results.push(item_with_indices);
            }
            if is_sys_mode {
                return results;
            }
        }

        // 4. Chế độ tìm kiếm File chuyên biệt khi gõ tiền tố @f hoặc @file
        let is_file_mode = trimmed_query.starts_with("@f ") || trimmed_query.starts_with("@file ")
            || trimmed_query == "@f" || trimmed_query == "@file";

        if is_file_mode {
            let file_query = if trimmed_query.starts_with("@file ") {
                trimmed_query[6..].trim()
            } else if trimmed_query.starts_with("@f ") {
                trimmed_query[3..].trim()
            } else {
                ""
            };
            let file_query_normalized = remove_vietnamese_accents(file_query);

            // Nếu chỉ gõ "@f" hoặc "@f ", hiển thị danh sách các file/thư mục mới nhất
            if file_query.is_empty() {
                for file in shallow_files_guard.iter().take(50) {
                    results.push((file.clone(), Vec::new()));
                }
                return results;
            }

            // Duyệt theo đường dẫn trực tiếp trong file mode (nếu có)
            if let Some((dir, filter)) = self.resolve_path_search(file_query) {
                if filter.is_empty() {
                    let dir_items = self.scan_dir_on_the_fly(&dir);
                    for item in dir_items {
                        results.push((item, Vec::new()));
                    }
                    return results;
                }

                let dir_items = self.scan_dir_recursive(&dir, 3);
                let filter_normalized = remove_vietnamese_accents(&filter);
                let enable_path_matching = self.config.search.enable_path_matching.unwrap_or(true);
                let mut matched: Vec<((LauncherItem, Vec<usize>), i64)> = dir_items
                    .par_iter()
                    .filter_map(|item| {
                        let match_res = self.match_precomputed(
                            &item.name,
                            &item.normalized_name,
                            &filter,
                            &filter_normalized,
                        );
                        let final_res = if match_res.is_some() {
                            match_res
                        } else if enable_path_matching {
                            self.match_item(&item.exec_or_path, &filter)
                        } else {
                            None
                        };

                        final_res.map(|(score, indices)| ((item.clone(), indices), score))
                    })
                    .collect();

                matched.sort_by(|a, b| b.1.cmp(&a.1));
                for ((item, indices), _) in matched {
                    results.push((item, indices));
                }
                return results;
            }

            // Tìm kiếm mờ song song (Rayon) trong toàn bộ Files & Directories
            let enable_path_matching = self.config.search.enable_path_matching.unwrap_or(true);
            let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = shallow_files_guard
                .par_iter()
                .filter_map(|file| {
                    let match_res = self.match_precomputed(
                        &file.name,
                        &file.normalized_name,
                        file_query,
                        &file_query_normalized,
                    );
                    let final_res = if match_res.is_some() {
                        match_res
                    } else if enable_path_matching {
                        self.match_item(&file.exec_or_path, file_query)
                    } else {
                        None
                    };

                    final_res.map(|(score, indices)| {
                        let boost = history_guard.get_boost(&file.exec_or_path);
                        ((file.clone(), indices), score + boost)
                    })
                })
                .collect();

            matches.sort_by(|a, b| b.1.cmp(&a.1));
            for (item_with_indices, _) in matches {
                results.push(item_with_indices);
            }
            return results;
        }

        // 5. Chế độ tìm kiếm Emoji (@emoji, @e, hoặc bắt đầu bằng dấu ':')
        let is_emoji_mode = trimmed_query.starts_with("@emoji")
            || trimmed_query.starts_with("@e ")
            || trimmed_query == "@e"
            || trimmed_query.starts_with(':');

        if is_emoji_mode {
            let emoji_query = if trimmed_query.starts_with("@emoji ") {
                trimmed_query[7..].trim()
            } else if trimmed_query.starts_with("@e ") {
                trimmed_query[3..].trim()
            } else if trimmed_query.starts_with(':') {
                trimmed_query[1..].trim().trim_end_matches(':')
            } else {
                ""
            };
            let emoji_items = crate::emoji::search_emojis(emoji_query);
            for item in emoji_items {
                results.push((item, Vec::new()));
            }
            return results;
        }

        // 6. Chế độ Custom Script Plugins (~/.config/view-launcher/plugins/)
        if trimmed_query.starts_with('@') && !is_win_mode && !is_clip_mode && !is_sys_mode && !is_file_mode && !is_emoji_mode {
            let plugins = crate::plugins::discover_plugins();
            let after_at = &trimmed_query[1..];
            let mut parts = after_at.splitn(2, ' ');
            let target_plugin = parts.next().unwrap_or("").to_lowercase();
            let plugin_query = parts.next().unwrap_or("");

            if let Some(plugin) = plugins.iter().find(|p| p.name == target_plugin) {
                let items = crate::plugins::execute_plugin(plugin, plugin_query);
                for item in items {
                    results.push((item, Vec::new()));
                }
                return results;
            } else if target_plugin.is_empty() {
                // Liệt kê danh sách tất cả các chế độ & plugin tùy chỉnh
                results.push((
                    LauncherItem::new(
                        "@w - Window Switcher".to_string(),
                        "@w".to_string(),
                        ItemType::Calc,
                        Some("Switch between active running windows".to_string()),
                        false,
                        None,
                    ),
                    Vec::new(),
                ));
                results.push((
                    LauncherItem::new(
                        "@c - Clipboard History".to_string(),
                        "@c".to_string(),
                        ItemType::Calc,
                        Some("Browse and paste clipboard history".to_string()),
                        false,
                        None,
                    ),
                    Vec::new(),
                ));
                results.push((
                    LauncherItem::new(
                        "@sys - System Actions".to_string(),
                        "@sys".to_string(),
                        ItemType::Calc,
                        Some("Lock, Sleep, Restart, Shutdown".to_string()),
                        false,
                        None,
                    ),
                    Vec::new(),
                ));
                results.push((
                    LauncherItem::new(
                        "@f - File Search".to_string(),
                        "@f".to_string(),
                        ItemType::Calc,
                        Some("Search files and browse directories".to_string()),
                        false,
                        None,
                    ),
                    Vec::new(),
                ));
                results.push((
                    LauncherItem::new(
                        "@emoji - Emoji Picker".to_string(),
                        "@emoji".to_string(),
                        ItemType::Calc,
                        Some("Search emojis in English or Vietnamese (:fire)".to_string()),
                        false,
                        None,
                    ),
                    Vec::new(),
                ));
                results.push((
                    LauncherItem::new(
                        "@theme - Switch Theme & Opacity".to_string(),
                        "@theme".to_string(),
                        ItemType::Calc,
                        Some("Dark, Light, System, or Opacity (e.g. @theme 95%)".to_string()),
                        false,
                        None,
                    ),
                    Vec::new(),
                ));
                for p in plugins {
                    results.push((
                        LauncherItem::new(
                            format!("@{} - Custom Script Plugin", p.name),
                            format!("@{}", p.name),
                            ItemType::Calc,
                            Some(format!("Run script: {}", p.path.display())),
                            false,
                            None,
                        ),
                        Vec::new(),
                    ));
                }
                return results;
            }
        }

        // 7. Chế độ chuyển đổi Theme & Opacity (@theme, @mode, @opacity)
        let is_theme_mode = trimmed_query.starts_with("@theme")
            || trimmed_query.starts_with("@mode")
            || trimmed_query.starts_with("@opacity");

        if is_theme_mode {
            let theme_query = if trimmed_query.starts_with("@theme ") {
                trimmed_query[7..].trim()
            } else if trimmed_query.starts_with("@mode ") {
                trimmed_query[6..].trim()
            } else if trimmed_query.starts_with("@opacity ") {
                trimmed_query[9..].trim()
            } else if trimmed_query == "@theme" || trimmed_query == "@mode" || trimmed_query == "@opacity" {
                ""
            } else {
                trimmed_query
            };

            let theme_items = vec![
                LauncherItem::new(
                    "Dark Theme".to_string(),
                    "theme:dark".to_string(),
                    ItemType::Theme,
                    Some("Switch to sleek dark palette with high contrast".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "Light Theme".to_string(),
                    "theme:light".to_string(),
                    ItemType::Theme,
                    Some("Switch to clean, high-visibility light palette".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "System Theme (Auto)".to_string(),
                    "theme:system".to_string(),
                    ItemType::Theme,
                    Some("Automatically match OS dark/light mode".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "Window Opacity: 100% (Solid)".to_string(),
                    "theme:opacity:100".to_string(),
                    ItemType::Theme,
                    Some("Set window opacity to 100% (opaque)".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "Window Opacity: 95% (Frosted Glass - Recommended)".to_string(),
                    "theme:opacity:95".to_string(),
                    ItemType::Theme,
                    Some("Set window opacity to 95% (modern glassmorphism)".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "Window Opacity: 90% (Glass)".to_string(),
                    "theme:opacity:90".to_string(),
                    ItemType::Theme,
                    Some("Set window opacity to 90%".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "Window Opacity: 80% (Translucent)".to_string(),
                    "theme:opacity:80".to_string(),
                    ItemType::Theme,
                    Some("Set window opacity to 80%".to_string()),
                    false,
                    None,
                ),
                LauncherItem::new(
                    "Window Opacity: 70% (High Transparency)".to_string(),
                    "theme:opacity:70".to_string(),
                    ItemType::Theme,
                    Some("Set window opacity to 70%".to_string()),
                    false,
                    None,
                ),
            ];

            if theme_query.is_empty() {
                for item in theme_items {
                    results.push((item, Vec::new()));
                }
                return results;
            }

            let norm_q = remove_vietnamese_accents(theme_query);
            let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = theme_items
                .into_iter()
                .filter_map(|item| {
                    let (score, indices) = self.match_precomputed(
                        &item.name,
                        &item.normalized_name,
                        theme_query,
                        &norm_q,
                    )?;
                    Some(((item, indices), score + 300))
                })
                .collect();
            matches.sort_by(|a, b| b.1.cmp(&a.1));
            for (item_with_indices, _) in matches {
                results.push(item_with_indices);
            }
            return results;
        }

        // 8. Chuyển đổi thông minh Offline (Unit & Currency Converter)
        if let Some(conv) = calc::evaluate_conversion(trimmed_query) {
            results.push((
                LauncherItem::new(
                    conv.title.clone(),
                    conv.value_to_copy.clone(),
                    ItemType::Calc,
                    Some(conv.subtitle),
                    false,
                    None,
                ),
                Vec::new(),
            ));
        }

        // 8. Chế độ mặc định: Tính toán nhanh nếu là biểu thức toán
        if let Some(calc_val) = calc::evaluate(trimmed_query) {
            let formatted = calc::format_result(calc_val);
            results.push((
                LauncherItem::new(
                    formatted.clone(),
                    formatted.clone(),
                    ItemType::Calc,
                    Some("Press Enter to copy result to clipboard".to_string()),
                    false,
                    None,
                ),
                Vec::new(),
            ));
        }

        // 3. Duyệt đường dẫn trực tiếp nếu bắt đầu bằng '/' hoặc '~'
        if trimmed_query.starts_with('/') || trimmed_query.starts_with('~') {
            if let Some((dir, filter)) = self.resolve_path_search(trimmed_query) {
                if filter.is_empty() {
                    let dir_items = self.scan_dir_on_the_fly(&dir);
                    for item in dir_items {
                        results.push((item, Vec::new()));
                    }
                    return results;
                }

                let dir_items = self.scan_dir_recursive(&dir, 3);
                let filter_normalized = remove_vietnamese_accents(&filter);
                let enable_path_matching = self.config.search.enable_path_matching.unwrap_or(true);
                let mut matched: Vec<((LauncherItem, Vec<usize>), i64)> = dir_items
                    .par_iter()
                    .filter_map(|item| {
                        let match_res = self.match_precomputed(
                            &item.name,
                            &item.normalized_name,
                            &filter,
                            &filter_normalized,
                        );
                        let final_res = if match_res.is_some() {
                            match_res
                        } else if enable_path_matching {
                            self.match_item(&item.exec_or_path, &filter)
                        } else {
                            None
                        };

                        final_res.map(|(score, indices)| ((item.clone(), indices), score))
                    })
                    .collect();

                matched.sort_by(|a, b| b.1.cmp(&a.1));
                for ((item, indices), _) in matched {
                    results.push((item, indices));
                }
                return results;
            }
        }

        // 4. Khi chưa gõ: Hiển thị ứng dụng đã ghim (Pinned) -> Ứng dụng tùy biến -> Ứng dụng hệ thống
        if trimmed_query.is_empty() {
            let mut default_apps = Vec::new();

            // Pinned apps first
            for pinned in &self.pinned_apps {
                if let Some(app) = self.apps.iter().chain(self.custom_apps.iter()).find(|a| a.name.eq_ignore_ascii_case(pinned)) {
                    if !default_apps.iter().any(|(item, _): &(LauncherItem, _)| item.name == app.name) {
                        default_apps.push((app.clone(), Vec::new()));
                    }
                }
            }

            // Custom apps next
            for custom in &self.custom_apps {
                if !default_apps.iter().any(|(item, _): &(LauncherItem, _)| item.name == custom.name) {
                    default_apps.push((custom.clone(), Vec::new()));
                }
            }

            // Standard apps
            for app in &self.apps {
                if !default_apps.iter().any(|(item, _): &(LauncherItem, _)| item.name == app.name) {
                    default_apps.push((app.clone(), Vec::new()));
                }
            }

            return default_apps;
        }

        // 5. Tìm kiếm mờ đa luồng trong Ứng dụng (Rayon Parallel Matching)
        let mut custom_matches: Vec<((LauncherItem, Vec<usize>), i64)> = self
            .custom_apps
            .par_iter()
            .filter_map(|custom| {
                let (score, indices) = self.match_precomputed(
                    &custom.name,
                    &custom.normalized_name,
                    trimmed_query,
                    &query_normalized,
                )?;
                let boost = history_guard.get_boost(&custom.name) + 150;
                Some(((custom.clone(), indices), score + boost))
            })
            .collect();

        let mut app_matches: Vec<((LauncherItem, Vec<usize>), i64)> = self
            .apps
            .par_iter()
            .filter_map(|app| {
                let (score, indices) = self.match_precomputed(
                    &app.name,
                    &app.normalized_name,
                    trimmed_query,
                    &query_normalized,
                )?;
                let boost = history_guard.get_boost(&app.name) + 100;
                Some(((app.clone(), indices), score + boost))
            })
            .collect();

        custom_matches.append(&mut app_matches);
        custom_matches.sort_by(|a, b| b.1.cmp(&a.1));

        for (item_with_indices, _) in custom_matches {
            results.push(item_with_indices);
        }

        results
    }

    #[inline]
    fn match_precomputed(
        &self,
        name: &str,
        name_normalized: &str,
        query: &str,
        query_normalized: &str,
    ) -> Option<(i64, Vec<usize>)> {
        let (score_orig, indices_orig) = self.matcher.fuzzy_indices(name, query).unwrap_or((0, Vec::new()));
        let (score_accent, indices_accent) = if !query_normalized.is_empty() {
            self.matcher.fuzzy_indices(name_normalized, query_normalized).unwrap_or((0, Vec::new()))
        } else {
            (0, Vec::new())
        };

        let (base_score, indices) = if score_orig >= score_accent && score_orig > 0 {
            (score_orig, indices_orig)
        } else if score_accent > 0 {
            (score_accent, indices_accent)
        } else {
            return None;
        };

        let bonus = Self::calculate_tier_bonus(name_normalized, query_normalized);
        Some((base_score + bonus, indices))
    }

    #[inline]
    fn calculate_tier_bonus(name_normalized: &str, query_normalized: &str) -> i64 {
        if query_normalized.is_empty() {
            return 0;
        }

        // Tier 1: Exact prefix ("image viewer" starts with "image") -> +350 bonus
        if name_normalized.starts_with(query_normalized) {
            return 350;
        }

        // Tier 2: Word-boundary prefix ("gnome image editor" has word starting with "image") -> +200 bonus
        if name_normalized.split_whitespace().any(|word| word.starts_with(query_normalized)) {
            return 200;
        }

        // Tier 3: Consecutive substring ("fastimage" contains "image") -> +100 bonus
        if name_normalized.contains(query_normalized) {
            return 100;
        }

        0
    }

    fn match_item(&self, text: &str, query: &str) -> Option<(i64, Vec<usize>)> {
        let query_normalized = remove_vietnamese_accents(query);
        let text_normalized = remove_vietnamese_accents(text);
        self.match_precomputed(text, &text_normalized, query, &query_normalized)
    }

    /// Spawns the application or executes item action safely.
    pub fn launch(&self, item: &LauncherItem) {
        if item.item_type == ItemType::Calc || item.item_type == ItemType::Clipboard {
            self.clipboard.copy_to_clipboard(&item.exec_or_path);
            return;
        }

        if item.item_type == ItemType::Window {
            crate::window_switcher::focus_window(&item.exec_or_path);
            return;
        }

        if item.item_type == ItemType::System {
            crate::system_actions::execute_system_action(&item.exec_or_path);
            return;
        }

        if item.item_type == ItemType::Dmenu {
            println!("{}", item.name);
            std::process::exit(0);
        }

        // Record history
        let key = if item.item_type == ItemType::App { &item.name } else { &item.exec_or_path };
        if let Ok(mut h) = self.history.write() {
            h.record_launch(key);
        }

        #[cfg(not(target_os = "windows"))]
        {
            match item.item_type {
                ItemType::App => {
                    let tokens: Vec<&str> = item.exec_or_path.split_whitespace().collect();
                    if tokens.is_empty() { return; }

                    if item.terminal {
                        let term = find_terminal_emulator();
                        let mut cmd = Command::new(&term);
                        cmd.arg("-e").args(&tokens);
                        unsafe {
                            cmd.stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .stdin(std::process::Stdio::null())
                                .pre_exec(|| {
                                    setsid();
                                    Ok(())
                                })
                                .spawn()
                                .ok();
                        }
                    } else {
                        let mut cmd = Command::new(tokens[0]);
                        if tokens.len() > 1 {
                            cmd.args(&tokens[1..]);
                        }
                        unsafe {
                            cmd.stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null())
                                .stdin(std::process::Stdio::null())
                                .pre_exec(|| {
                                    setsid();
                                    Ok(())
                                })
                                .spawn()
                                .ok();
                        }
                    }
                }
                ItemType::File | ItemType::Dir => {
                    unsafe {
                        Command::new("xdg-open")
                            .arg(&item.exec_or_path)
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .stdin(std::process::Stdio::null())
                            .pre_exec(|| {
                                setsid();
                                Ok(())
                            })
                            .spawn()
                            .ok();
                    }
                }
                ItemType::Calc | ItemType::Clipboard | ItemType::Window | ItemType::System | ItemType::Dmenu | ItemType::Theme => {}
            }
        }

        #[cfg(target_os = "windows")]
        {
            #[cfg(windows)]
            use std::os::windows::process::CommandExt;
            #[allow(unused_mut)]
            let mut cmd = Command::new("cmd");
            cmd.args(&["/C", "start", "", &item.exec_or_path])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            let _ = cmd.spawn();
        }
    }

    /// Opens the containing directory of the item in terminal.
    pub fn open_in_terminal(&self, item: &LauncherItem) {
        let dir = match item.item_type {
            ItemType::Dir => PathBuf::from(&item.exec_or_path),
            ItemType::File => Path::new(&item.exec_or_path).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
            ItemType::App | ItemType::Calc | ItemType::Clipboard | ItemType::Window | ItemType::System | ItemType::Dmenu | ItemType::Theme => return,
        };

        #[cfg(not(target_os = "windows"))]
        {
            let term = find_terminal_emulator();
            unsafe {
                Command::new(&term)
                    .current_dir(&dir)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .stdin(std::process::Stdio::null())
                    .pre_exec(|| {
                        setsid();
                        Ok(())
                    })
                    .spawn()
                    .ok();
            }
        }

        #[cfg(target_os = "windows")]
        {
            #[cfg(windows)]
            use std::os::windows::process::CommandExt;
            #[allow(unused_mut)]
            let mut cmd = Command::new("cmd");
            cmd.args(&["/C", "start", "wt.exe", "-d", &dir.to_string_lossy()]);
            #[cfg(windows)]
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            let _ = cmd.spawn();
        }
    }

    /// Copies path or result string to clipboard.
    pub fn copy_to_clipboard(&self, text: &str) {
        #[cfg(not(target_os = "windows"))]
        {
            use std::io::Write;
            // Try wl-copy (Wayland) first
            if let Ok(mut child) = Command::new("wl-copy").stdin(std::process::Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return;
            }

            // Fallback to xclip (X11)
            if let Ok(mut child) = Command::new("xclip").args(&["-selection", "clipboard"]).stdin(std::process::Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
                return;
            }

            // Fallback to xsel
            if let Ok(mut child) = Command::new("xsel").args(&["--clipboard", "--input"]).stdin(std::process::Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }

        #[cfg(target_os = "windows")]
        {
            #[cfg(windows)]
            use std::os::windows::process::CommandExt;
            use std::io::Write;
            #[allow(unused_mut)]
            let mut cmd = Command::new("clip");
            cmd.stdin(std::process::Stdio::piped());
            #[cfg(windows)]
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            if let Ok(mut child) = cmd.spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }
    }
}

fn expand_tilde(path_str: &str) -> String {
    if path_str.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path_str.replacen("~/", &format!("{}/", home.to_string_lossy()), 1);
        }
    } else if path_str == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    path_str.to_string()
}

#[cfg(not(target_os = "windows"))]
fn find_terminal_emulator() -> String {
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.trim().is_empty() {
            return term;
        }
    }
    
    let common_terminals = vec![
        "kitty",
        "alacritty",
        "wezterm",
        "ghostty",
        "foot",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
    ];
    
    for term in common_terminals {
        if which_binary(term) {
            return term.to_string();
        }
    }
    
    "xterm".to_string()
}

#[cfg(not(target_os = "windows"))]
fn which_binary(name: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let p = Path::new(dir).join(name);
            if p.exists() && p.is_file() {
                return true;
            }
        }
    }
    false
}

/// Helper function to strip Vietnamese accents and convert to lowercase for accent-insensitive search.
pub fn remove_vietnamese_accents(s: &str) -> String {
    s.chars().map(|c| {
        match c {
            'á' | 'à' | 'ả' | 'ã' | 'ạ' | 'ă' | 'ắ' | 'ằ' | 'ẳ' | 'ẵ' | 'ặ' | 'â' | 'ấ' | 'ầ' | 'ẩ' | 'ẫ' | 'ậ' |
            'Á' | 'À' | 'Ả' | 'Ã' | 'Ạ' | 'Ă' | 'Ắ' | 'Ằ' | 'Ẳ' | 'Ẵ' | 'Ặ' | 'Â' | 'Ấ' | 'Ầ' | 'Ẩ' | 'Ẫ' | 'Ậ' => 'a',
            'é' | 'è' | 'ẻ' | 'ẽ' | 'ẹ' | 'ê' | 'ế' | 'ề' | 'ể' | 'ễ' | 'ệ' |
            'É' | 'È' | 'Ẻ' | 'Ẽ' | 'Ẹ' | 'Ê' | 'Ế' | 'Ề' | 'Ể' | 'Ễ' | 'Ệ' => 'e',
            'í' | 'ì' | 'ỉ' | 'ĩ' | 'ị' |
            'Í' | 'Ì' | 'Ỉ' | 'Ĩ' | 'Ị' => 'i',
            'ó' | 'ò' | 'ỏ' | 'õ' | 'ọ' | 'ô' | 'ố' | 'ồ' | 'ổ' | 'ỗ' | 'ộ' | 'ơ' | 'ớ' | 'ờ' | 'ở' | 'ỡ' | 'ợ' |
            'Ó' | 'Ò' | 'Ỏ' | 'Õ' | 'Ọ' | 'Ô' | 'Ố' | 'Ồ' | 'Ổ' | 'Ỗ' | 'Ộ' | 'Ơ' | 'Ớ' | 'Ờ' | 'Ở' | 'Ỡ' | 'Ợ' => 'o',
            'ú' | 'ù' | 'ủ' | 'ũ' | 'ụ' | 'ư' | 'ứ' | 'ừ' | 'ử' | 'ữ' | 'ự' |
            'Ú' | 'Ù' | 'Ủ' | 'Ũ' | 'Ụ' | 'Ư' | 'Ứ' | 'Ừ' | 'Ử' | 'Ữ' | 'Ự' => 'u',
            'ý' | 'ỳ' | 'ỷ' | 'ỹ' | 'ỵ' |
            'Ý' | 'Ỳ' | 'Ỷ' | 'Ỹ' | 'Ỵ' => 'y',
            'đ' | 'Đ' => 'd',
            _ => c.to_ascii_lowercase(),
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_vietnamese_accents() {
        assert_eq!(remove_vietnamese_accents("Tải xuống"), "tai xuong");
        assert_eq!(remove_vietnamese_accents("Học tập"), "hoc tap");
        assert_eq!(remove_vietnamese_accents("Đường dẫn"), "duong dan");
        assert_eq!(remove_vietnamese_accents("Lập trình Rust"), "lap trinh rust");
    }

    #[test]
    fn test_resolve_path_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        assert!(engine.resolve_path_search("downloads").is_none());
        if let Some(home) = dirs::home_dir() {
            let mut home_str = home.to_string_lossy().to_string();
            if !home_str.ends_with('/') && !home_str.ends_with('\\') {
                home_str.push('/');
            }
            assert!(engine.resolve_path_search(&home_str).is_some());
            assert!(engine.resolve_path_search("~").is_some());
            assert!(engine.resolve_path_search("~/").is_some());
        }
    }

    #[test]
    fn test_file_mode_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let results = engine.search("@f");
        assert!(results.iter().all(|(item, _)| item.item_type == ItemType::File || item.item_type == ItemType::Dir));
    }

    #[test]
    fn test_settings_save_and_load() {
        let mut config = Config::default();
        config.theme.show_icons = Some(false);
        config.search.max_results = 25;
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.theme.show_icons, Some(false));
        assert_eq!(deserialized.search.max_results, 25);
    }

    #[test]
    fn test_search_and_icon_performance() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let resolver = crate::icon_resolver::IconResolver::new();
        
        let start = std::time::Instant::now();
        let results = engine.search("int");
        let elapsed_search = start.elapsed();
        assert!(elapsed_search.as_millis() < 500, "Search took too long: {:?}", elapsed_search);

        // Cold icon resolution
        for (item, _) in results.iter().take(20) {
            let _ = resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path);
        }
        let elapsed_cold = start.elapsed();
        assert!(elapsed_cold.as_millis() < 5000, "Cold search + icon took too long: {:?}", elapsed_cold);

        // Instant cached icon lookup test (O(1) memory cache)
        let cache_start = std::time::Instant::now();
        for (item, _) in results.iter().take(20) {
            let _ = resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path);
        }
        let elapsed_cached = cache_start.elapsed();
        assert!(elapsed_cached.as_millis() < 50, "Cached icon lookup took too long: {:?}", elapsed_cached);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(2450), "2.4 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_directory_search_and_scan() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let current_dir = std::env::current_dir().unwrap();
        let current_dir_str = current_dir.to_string_lossy();
        
        let on_the_fly = engine.scan_dir_on_the_fly(&current_dir);
        assert!(!on_the_fly.is_empty(), "Current dir should not be empty");
        assert!(on_the_fly.iter().any(|item| item.name == "Cargo.toml" || item.name == "src"));

        let recursive = engine.scan_dir_recursive(&current_dir, 3);
        assert!(!recursive.is_empty(), "Recursive scan should find files");
        assert!(recursive.iter().any(|item| item.name == "main.rs" || item.name == "launcher.rs"));

        let results = engine.search(&format!("@f {}/", current_dir_str));
        assert!(!results.is_empty());
    }

    #[test]
    fn test_system_mode_and_keywords() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        
        let sys_results = engine.search("@sys");
        assert!(!sys_results.is_empty());
        assert!(sys_results.iter().all(|(item, _)| item.item_type == ItemType::System));

        let lock_results = engine.search("lock");
        assert!(lock_results.iter().any(|(item, _)| item.item_type == ItemType::System && item.name.contains("Lock")));

        let reboot_results = engine.search("restart");
        assert!(reboot_results.iter().any(|(item, _)| item.item_type == ItemType::System && item.name.contains("Restart")));
    }

    #[test]
    fn test_window_mode_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let win_results = engine.search("@w");
        assert!(win_results.iter().all(|(item, _)| item.item_type == ItemType::Window));
    }

    #[test]
    fn test_clipboard_mode_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let clip_results = engine.search("@c");
        assert!(clip_results.iter().all(|(item, _)| item.item_type == ItemType::Clipboard));
    }

    #[test]
    fn test_emoji_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        
        let e1 = engine.search("@emoji fire");
        assert!(!e1.is_empty());
        assert!(e1.iter().any(|(item, _)| item.name.contains("🔥")));

        let e2 = engine.search(":rocket");
        assert!(!e2.is_empty());
        assert!(e2.iter().any(|(item, _)| item.name.contains("🚀")));
    }

    #[test]
    fn test_unit_and_currency_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);

        let conv1 = engine.search("100 usd in vnd");
        assert!(!conv1.is_empty());
        assert!(conv1.iter().any(|(item, _)| item.name.contains("VND")));

        let conv2 = engine.search("37 c to f");
        assert!(!conv2.is_empty());
        assert!(conv2.iter().any(|(item, _)| item.name.contains("98.6 °F")));
    }

    #[test]
    fn test_custom_plugins_overview() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);

        let modes = engine.search("@");
        assert!(!modes.is_empty());
        assert!(modes.iter().any(|(item, _)| item.name.contains("@w")));
        assert!(modes.iter().any(|(item, _)| item.name.contains("@c")));
        assert!(modes.iter().any(|(item, _)| item.name.contains("@sys")));
        assert!(modes.iter().any(|(item, _)| item.name.contains("@emoji")));
    }

    #[test]
    fn test_prefix_and_tiered_ranking() {
        let mut config = Config::default();
        config.apps.custom.push(crate::config::CustomAppConfig {
            name: "Extension Manager".to_string(),
            exec: "extension-manager".to_string(),
            description: None,
            terminal: false,
            icon: None,
            category: None,
        });
        config.apps.custom.push(crate::config::CustomAppConfig {
            name: "Image Viewer".to_string(),
            exec: "image-viewer".to_string(),
            description: None,
            terminal: false,
            icon: None,
            category: None,
        });
        let engine = LauncherEngine::new(config);

        let results = engine.search("image");
        assert!(results.len() >= 2);
        // Image Viewer must rank at #0 above Extension Manager
        assert_eq!(results[0].0.name, "Image Viewer");
    }

    #[test]
    fn test_theme_mode_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);

        let results = engine.search("@theme");
        assert!(!results.is_empty());
        assert!(results.iter().any(|(item, _)| item.exec_or_path == "theme:dark"));
        assert!(results.iter().any(|(item, _)| item.exec_or_path == "theme:light"));
        assert!(results.iter().any(|(item, _)| item.exec_or_path == "theme:system"));
        assert!(results.iter().any(|(item, _)| item.exec_or_path == "theme:opacity:95"));

        let dark_match = engine.search("@theme dark");
        assert!(!dark_match.is_empty());
        assert_eq!(dark_match[0].0.exec_or_path, "theme:dark");

        let opacity_match = engine.search("@opacity 80");
        assert!(!opacity_match.is_empty());
        assert_eq!(opacity_match[0].0.exec_or_path, "theme:opacity:80");
    }

    #[test]
    fn test_theme_config_and_is_dark() {
        let mut theme = crate::config::ThemeConfig::default();
        assert_eq!(theme.mode, "dark");
        assert_eq!(theme.opacity, 0.95);
        assert!(theme.is_dark());

        theme.mode = "light".to_string();
        assert!(!theme.is_dark());

        theme.mode = "dark".to_string();
        assert!(theme.is_dark());
    }

    #[test]
    fn test_system_dark_mode_live() {
        let is_dark = crate::config::is_system_dark_mode();
        println!(">>> Current is_system_dark_mode(): {}", is_dark);
    }
}



