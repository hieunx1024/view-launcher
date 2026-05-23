use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use crate::config::Config;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
unsafe extern "C" {
    fn setsid() -> i32;
}

#[derive(Debug, Clone)]
pub enum ItemType {
    App,
    File,
    Dir,
}

#[derive(Debug, Clone)]
pub struct LauncherItem {
    pub name: String,
    pub exec_or_path: String,
    pub item_type: ItemType,
    pub description: Option<String>,
    pub terminal: bool,
}

pub struct LauncherEngine {
    pub apps: Vec<LauncherItem>,
    pub shallow_files: Arc<RwLock<Vec<LauncherItem>>>,
    matcher: SkimMatcherV2,
}

impl LauncherEngine {
    pub fn new(config: Config) -> Self {
        let mut engine = Self {
            apps: Vec::new(),
            shallow_files: Arc::new(RwLock::new(Vec::new())),
            matcher: SkimMatcherV2::default(),
        };
        engine.index_apps();
        
        let shallow_files_clone = engine.shallow_files.clone();
        let config_clone = config.clone();
        std::thread::spawn(move || {
            let mut files = Vec::new();
            Self::index_shallow_home_impl(&config_clone, &mut files);
            if let Ok(mut lock) = shallow_files_clone.write() {
                *lock = files;
            }
        });
        
        engine
    }

    /// Indexes all standard Linux .desktop application entries.
    #[cfg(not(target_os = "windows"))]
    fn index_apps(&mut self) {
        let paths = vec![
            PathBuf::from("/usr/share/applications"),
            dirs::home_dir().map(|mut h| {
                h.push(".local/share/applications");
                h
            }).unwrap_or_default(),
        ];

        for path in paths {
            if !path.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let file_path = entry.path();
                    if file_path.extension().map_or(false, |ext| ext == "desktop") {
                        if let Some(app) = self.parse_desktop_file(&file_path) {
                            // Avoid duplicates by name
                            if !self.apps.iter().any(|item| item.name == app.name) {
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
                    self.apps.push(LauncherItem {
                        name,
                        exec_or_path: exec,
                        item_type: ItemType::App,
                        description: Some("Windows Shortcut".to_string()),
                        terminal: false,
                    });
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
                        // If the last token is "--", remove it since we removed file arguments
                        if let Some(&"--") = cleaned_tokens.last() {
                            cleaned_tokens.pop();
                        }
                        exec = cleaned_tokens.join(" ");
                    }
                    "Comment" => {
                        comment = Some(val.to_string());
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
            })
        } else {
            None
        }
    }

    /// Fast shallow scan of the Home directory (depth 1 or 2) to get immediate files.
    fn index_shallow_home_impl(config: &Config, files: &mut Vec<LauncherItem>) {
        let Some(home) = dirs::home_dir() else { return; };
        
        let max_depth = config.search.max_depth;
        let ignored_dirs = &config.search.ignored_dirs;

        for entry in WalkDir::new(&home)
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
            if path == home {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();
            let name = entry.file_name().to_string_lossy().to_string();
            
            let item_type = if entry.file_type().is_dir() {
                ItemType::Dir
            } else {
                ItemType::File
            };

            files.push(LauncherItem {
                name,
                exec_or_path: path_str,
                item_type,
                description: Some(path.parent().map_or("".to_string(), |p| p.to_string_lossy().to_string())),
                terminal: false,
            });
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
                });
            }
        }
        items
    }

    /// Performs high-performance fuzzy matching and ranking of items.
    pub fn search(&self, query: &str) -> Vec<LauncherItem> {
        let shallow_files_guard = self.shallow_files.read().unwrap_or_else(|e| e.into_inner());

        if query.is_empty() {
            // By default, list all apps, then files
            let mut default_list = self.apps.clone();
            default_list.extend(shallow_files_guard.iter().take(50).cloned());
            return default_list;
        }

        // Check if query is a dynamic path search
        if let Some((dir, filter)) = self.resolve_path_search(query) {
            let dir_items = self.scan_dir_on_the_fly(&dir);
            if filter.is_empty() {
                return dir_items;
            }
            
            // Fuzzy match the directory contents
            let mut matched = Vec::new();
            for item in dir_items {
                let score_orig = self.matcher.fuzzy_match(&item.name, &filter);
                let score_accent = {
                    let name_stripped = remove_vietnamese_accents(&item.name);
                    let filter_stripped = remove_vietnamese_accents(&filter);
                    self.matcher.fuzzy_match(&name_stripped, &filter_stripped)
                };
                let final_score = match (score_orig, score_accent) {
                    (Some(s1), Some(s2)) => Some(s1.max(s2)),
                    (Some(s), None) | (None, Some(s)) => Some(s),
                    (None, None) => None,
                };
                if let Some(score) = final_score {
                    matched.push((item, score));
                }
            }
            matched.sort_by(|a, b| b.1.cmp(&a.1));
            return matched.into_iter().map(|(item, _)| item).collect();
        }

        // Normal search: search applications and pre-indexed files
        let mut matches = Vec::new();

        // 1. Search apps
        for app in &self.apps {
            let score_orig = self.matcher.fuzzy_match(&app.name, query);
            let score_accent = {
                let name_stripped = remove_vietnamese_accents(&app.name);
                let query_stripped = remove_vietnamese_accents(query);
                self.matcher.fuzzy_match(&name_stripped, &query_stripped)
            };
            let final_score = match (score_orig, score_accent) {
                (Some(s1), Some(s2)) => Some(s1.max(s2)),
                (Some(s), None) | (None, Some(s)) => Some(s),
                (None, None) => None,
            };
            if let Some(score) = final_score {
                matches.push((app.clone(), score + 100)); // Boost applications slightly
            }
        }

        // 2. Search files
        for file in shallow_files_guard.iter() {
            let score_orig = self.matcher.fuzzy_match(&file.name, query);
            let score_accent = {
                let name_stripped = remove_vietnamese_accents(&file.name);
                let query_stripped = remove_vietnamese_accents(query);
                self.matcher.fuzzy_match(&name_stripped, &query_stripped)
            };
            let final_score = match (score_orig, score_accent) {
                (Some(s1), Some(s2)) => Some(s1.max(s2)),
                (Some(s), None) | (None, Some(s)) => Some(s),
                (None, None) => None,
            };
            if let Some(score) = final_score {
                matches.push((file.clone(), score));
            }
        }

        // Sort by fuzzy match score descending
        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches.into_iter().map(|(item, _)| item).collect()
    }

    /// Spawns the detached GUI application or opens files via xdg-open.
    #[cfg(not(target_os = "windows"))]
    pub fn launch(&self, item: &LauncherItem) {
        let res = match item.item_type {
            ItemType::App => {
                let exec_cmd = if item.terminal {
                    let term = find_terminal_emulator();
                    format!("{} -e {}", term, item.exec_or_path)
                } else {
                    item.exec_or_path.clone()
                };

                // We use standard pre_exec Unix Extension in Rust to execute setsid()
                // in the child process immediately after fork but before exec. This is the 
                // native systems-level way to create a new session, fully detaching the process
                // from the controlling terminal, ensuring it continues running after the terminal exits.
                unsafe {
                    Command::new("sh")
                        .arg("-c")
                        .arg(format!("exec {}", exec_cmd))
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .stdin(std::process::Stdio::null())
                        .pre_exec(|| {
                            setsid();
                            Ok(())
                        })
                        .spawn()
                        .map(|_| ())
                }
            }
            ItemType::File | ItemType::Dir => {
                // Open file or directory using system default via xdg-open in a detached session
                unsafe {
                    Command::new("sh")
                        .arg("-c")
                        .arg(format!("exec xdg-open '{}'", item.exec_or_path))
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .stdin(std::process::Stdio::null())
                        .pre_exec(|| {
                            setsid();
                            Ok(())
                        })
                        .spawn()
                        .map(|_| ())
                }
            }
        };

        if let Err(e) = res {
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/view-launcher.log")
            {
                use std::io::Write;
                let _ = writeln!(file, "[ERROR] Failed to spawn {}: {:?}", item.exec_or_path, e);
            }
        }
    }

    /// Spawns the detached GUI application or opens files via cmd /C start on Windows.
    #[cfg(target_os = "windows")]
    pub fn launch(&self, item: &LauncherItem) {
        let _ = Command::new("cmd")
            .args(&["/C", "start", "", &item.exec_or_path])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .spawn();
    }
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
    fn test_search_yazi() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        let results = engine.search("yazi");
        println!("\n=== SEARCH RESULTS FOR 'YAZI' ===");
        for (i, item) in results.iter().enumerate() {
            println!("{}: [{:?}] {} (Exec: {}) [Terminal: {}]", i + 1, item.item_type, item.name, item.exec_or_path, item.terminal);
        }
        println!("=================================\n");
        assert!(!results.is_empty(), "Should find yazi application or files");
        assert!(results[0].terminal, "Yazi desktop entry should specify Terminal=true");
    }

    #[test]
    fn test_resolve_path_search_no_slash() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        assert!(engine.resolve_path_search("downloads").is_none());
    }

    #[test]
    fn test_resolve_path_search_with_slash() {
        let config = Config::default();
        let engine = LauncherEngine::new(config);
        
        // Verify path parsing for temporary path
        let res = engine.resolve_path_search("/tmp/testfile");
        assert!(res.is_some());
        let (dir, filter) = res.unwrap();
        assert_eq!(dir, PathBuf::from("/tmp"));
        assert_eq!(filter, "testfile");
    }

    #[test]
    fn test_remove_vietnamese_accents() {
        assert_eq!(remove_vietnamese_accents("Tải xuống"), "tai xuong");
        assert_eq!(remove_vietnamese_accents("Học tập"), "hoc tap");
        assert_eq!(remove_vietnamese_accents("Đường dẫn"), "duong dan");
        assert_eq!(remove_vietnamese_accents("Lập trình Rust"), "lap trinh rust");
    }
}

