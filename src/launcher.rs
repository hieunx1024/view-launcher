use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;
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
}

#[derive(Debug, Clone)]
pub struct LauncherItem {
    pub name: String,
    pub exec_or_path: String,
    pub item_type: ItemType,
    pub description: Option<String>,
    pub terminal: bool,
    pub icon: Option<String>,
}

pub struct LauncherEngine {
    pub apps: Vec<LauncherItem>,
    pub custom_apps: Vec<LauncherItem>,
    pub pinned_apps: Vec<String>,
    pub hidden_apps: Vec<String>,
    pub shallow_files: Arc<RwLock<Vec<LauncherItem>>>,
    pub history: Arc<RwLock<HistoryManager>>,
    matcher: SkimMatcherV2,
    config: Config,
}

impl LauncherEngine {
    pub fn new(config: Config) -> Self {
        let history = Arc::new(RwLock::new(HistoryManager::load()));
        let mut engine = Self {
            apps: Vec::new(),
            custom_apps: Vec::new(),
            pinned_apps: config.apps.pinned.clone(),
            hidden_apps: config.apps.hidden.clone(),
            shallow_files: Arc::new(RwLock::new(Vec::new())),
            history,
            matcher: SkimMatcherV2::default(),
            config: config.clone(),
        };

        // Load custom apps from config
        for custom in &config.apps.custom {
            engine.custom_apps.push(LauncherItem {
                name: custom.name.clone(),
                exec_or_path: custom.exec.clone(),
                item_type: ItemType::App,
                description: custom.description.clone(),
                terminal: custom.terminal.unwrap_or(false),
                icon: custom.icon.clone(),
            });
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
            dirs::home_dir().map(|mut h| {
                h.push(".local/share/applications");
                h
            }).unwrap_or_default(),
        ];

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

    /// Indexes all standard Windows shortcut entries.
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

        for path in paths {
            if !path.exists() {
                continue;
            }
            for entry in WalkDir::new(path).into_iter().flatten() {
                let file_path = entry.path();
                if file_path.extension().map_or(false, |ext| ext == "lnk") {
                    let name = file_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let exec = file_path.to_string_lossy().to_string();
                    
                    let is_hidden = self.hidden_apps.iter().any(|h| name.eq_ignore_ascii_case(h));
                    if !is_hidden {
                        self.apps.push(LauncherItem {
                            name,
                            exec_or_path: exec,
                            item_type: ItemType::App,
                            description: Some("Windows Shortcut".to_string()),
                            terminal: false,
                            icon: None,
                        });
                    }
                }
            }
        }
    }

    /// Parses a Linux .desktop entry file line by line to extract the core fields.
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
            Some(LauncherItem {
                name,
                exec_or_path: exec,
                item_type: ItemType::App,
                description: comment,
                terminal,
                icon: icon_hint,
            })
        } else {
            None
        }
    }

    /// Indexes files based on configured `search.paths` or defaults.
    fn index_files_impl(config: &Config, files: &mut Vec<LauncherItem>) {
        let search_paths = if config.search.paths.is_empty() {
            vec![crate::config::SearchPathConfig {
                path: "~".to_string(),
                max_depth: Some(config.search.max_depth),
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

                files.push(LauncherItem {
                    name,
                    exec_or_path: path_str,
                    item_type,
                    description: desc,
                    terminal: false,
                    icon: None,
                });
            }
        }
    }

    /// Resolves dynamic path searching (e.g. typing `~/Downloads/` directly lists Downloads contents)
    pub fn resolve_path_search(&self, input: &str) -> Option<(PathBuf, String)> {
        if !input.contains('/') {
            return None;
        }

        let home = dirs::home_dir()?;
        let expanded = if input.starts_with("~/") {
            input.replacen("~/", &format!("{}/", home.to_string_lossy()), 1)
        } else if input == "~" {
            home.to_string_lossy().to_string()
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

    /// Scans a specific directory on-the-fly for quick sub-folder traversal.
    pub fn scan_dir_on_the_fly(&self, dir: &Path) -> Vec<LauncherItem> {
        let mut items = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let path_str = path.to_string_lossy().to_string();
                
                let item_type = if path.is_dir() {
                    ItemType::Dir
                } else {
                    ItemType::File
                };

                items.push(LauncherItem {
                    name,
                    exec_or_path: path_str,
                    item_type,
                    description: Some(dir.to_string_lossy().to_string()),
                    terminal: false,
                    icon: None,
                });
            }
        }
        items
    }

    /// Performs high-performance fuzzy matching and ranking of items.
    /// Mặc định: Chỉ tìm ứng dụng & tính toán (cực nhanh, không lẫn file).
    /// Khi bắt đầu bằng `@f ` hoặc `@file `: Chuyển sang chế độ tìm kiếm file & thư mục.
    pub fn search(&self, query: &str) -> Vec<(LauncherItem, Vec<usize>)> {
        let trimmed_query = query.trim();
        let shallow_files_guard = self.shallow_files.read().unwrap_or_else(|e| e.into_inner());
        let history_guard = self.history.read().unwrap_or_else(|e| e.into_inner());

        let mut results = Vec::new();

        // 1. Chế độ tìm kiếm File chuyên biệt khi gõ tiền tố @f hoặc @file
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

            // Nếu chỉ gõ "@f" hoặc "@f ", hiển thị danh sách các file/thư mục mới nhất
            if file_query.is_empty() {
                for file in shallow_files_guard.iter().take(50) {
                    results.push((file.clone(), Vec::new()));
                }
                return results;
            }

            // Duyệt theo đường dẫn trực tiếp trong file mode (nếu có)
            if let Some((dir, filter)) = self.resolve_path_search(file_query) {
                let dir_items = self.scan_dir_on_the_fly(&dir);
                if filter.is_empty() {
                    for item in dir_items {
                        results.push((item, Vec::new()));
                    }
                    return results;
                }

                let mut matched = Vec::new();
                for item in dir_items {
                    if let Some((final_score, final_indices)) = self.match_item(&item.name, &filter) {
                        matched.push(((item, final_indices), final_score));
                    }
                }
                matched.sort_by(|a, b| b.1.cmp(&a.1));
                for ((item, indices), _) in matched {
                    results.push((item, indices));
                }
                return results;
            }

            // Tìm kiếm mờ trong toàn bộ Files & Directories
            let enable_path_matching = self.config.search.enable_path_matching.unwrap_or(true);
            let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = Vec::new();

            for file in shallow_files_guard.iter() {
                let match_res = self.match_item(&file.name, file_query);
                let final_res = if match_res.is_some() {
                    match_res
                } else if enable_path_matching {
                    self.match_item(&file.exec_or_path, file_query)
                } else {
                    None
                };

                if let Some((score, indices)) = final_res {
                    let boost = history_guard.get_boost(&file.exec_or_path);
                    matches.push(((file.clone(), indices), score + boost));
                }
            }

            matches.sort_by(|a, b| b.1.cmp(&a.1));
            for (item_with_indices, _) in matches {
                results.push(item_with_indices);
            }
            return results;
        }

        // 2. Chế độ mặc định: Tính toán nhanh nếu là biểu thức toán
        if let Some(calc_val) = calc::evaluate(trimmed_query) {
            let formatted = calc::format_result(calc_val);
            results.push((
                LauncherItem {
                    name: format!("= {}", formatted),
                    exec_or_path: formatted.clone(),
                    item_type: ItemType::Calc,
                    description: Some("Press Enter to copy result to clipboard".to_string()),
                    terminal: false,
                    icon: None,
                },
                Vec::new(),
            ));
        }

        // 3. Duyệt đường dẫn trực tiếp nếu bắt đầu bằng '/' hoặc '~'
        if trimmed_query.starts_with('/') || trimmed_query.starts_with('~') {
            if let Some((dir, filter)) = self.resolve_path_search(trimmed_query) {
                let dir_items = self.scan_dir_on_the_fly(&dir);
                if filter.is_empty() {
                    for item in dir_items {
                        results.push((item, Vec::new()));
                    }
                    return results;
                }

                let mut matched = Vec::new();
                for item in dir_items {
                    if let Some((final_score, final_indices)) = self.match_item(&item.name, &filter) {
                        matched.push(((item, final_indices), final_score));
                    }
                }
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

        // 5. Tìm kiếm mờ trong Ứng dụng (Chỉ tìm App - Cực nhanh & Không rác file)
        let mut matches: Vec<((LauncherItem, Vec<usize>), i64)> = Vec::new();

        // 5.1 Custom Apps
        for custom in &self.custom_apps {
            if let Some((score, indices)) = self.match_item(&custom.name, trimmed_query) {
                let boost = history_guard.get_boost(&custom.name) + 150;
                matches.push(((custom.clone(), indices), score + boost));
            }
        }

        // 5.2 Standard Apps
        for app in &self.apps {
            if let Some((score, indices)) = self.match_item(&app.name, trimmed_query) {
                let boost = history_guard.get_boost(&app.name) + 100;
                matches.push(((app.clone(), indices), score + boost));
            }
        }

        // Sắp xếp theo độ khớp và lịch sử sử dụng
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        for (item_with_indices, _) in matches {
            results.push(item_with_indices);
        }

        results
    }

    fn match_item(&self, text: &str, query: &str) -> Option<(i64, Vec<usize>)> {
        let (score_orig, indices_orig) = self.matcher.fuzzy_indices(text, query).unwrap_or((0, Vec::new()));
        let (score_accent, indices_accent) = {
            let text_stripped = remove_vietnamese_accents(text);
            let query_stripped = remove_vietnamese_accents(query);
            self.matcher.fuzzy_indices(&text_stripped, &query_stripped).unwrap_or((0, Vec::new()))
        };

        if score_orig >= score_accent && score_orig > 0 {
            Some((score_orig, indices_orig))
        } else if score_accent > 0 {
            Some((score_accent, indices_accent))
        } else {
            None
        }
    }

    /// Spawns the application or opens file safely.
    pub fn launch(&self, item: &LauncherItem) {
        if item.item_type == ItemType::Calc {
            self.copy_to_clipboard(&item.exec_or_path);
            return;
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
                                    unsafe { setsid(); }
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
                                    unsafe { setsid(); }
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
                                unsafe { setsid(); }
                                Ok(())
                            })
                            .spawn()
                            .ok();
                    }
                }
                ItemType::Calc => {}
            }
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(&["/C", "start", "", &item.exec_or_path])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .spawn();
        }
    }

    /// Opens the containing directory of the item in terminal.
    pub fn open_in_terminal(&self, item: &LauncherItem) {
        let dir = match item.item_type {
            ItemType::Dir => PathBuf::from(&item.exec_or_path),
            ItemType::File => Path::new(&item.exec_or_path).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")),
            ItemType::App | ItemType::Calc => return,
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
                        unsafe { setsid(); }
                        Ok(())
                    })
                    .spawn()
                    .ok();
            }
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(&["/C", "start", "wt.exe", "-d", &dir.to_string_lossy()])
                .spawn();
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
            use std::io::Write;
            if let Ok(mut child) = Command::new("clip").stdin(std::process::Stdio::piped()).spawn() {
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
        }
    }

    #[test]
    fn test_file_mode_search() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let results = engine.search("@f");
        assert!(results.iter().all(|(item, _)| item.item_type == ItemType::File || item.item_type == ItemType::Dir));
    }
}


