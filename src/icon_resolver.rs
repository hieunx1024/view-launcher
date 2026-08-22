use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use crate::launcher::LauncherItem;

/// High-performance thread-safe icon resolver with O(1) in-memory index and pre-decoding cache.
pub struct IconResolver {
    icon_path_index: Arc<RwLock<HashMap<String, PathBuf>>>,
    image_cache: Arc<RwLock<HashMap<String, Option<slint::Image>>>>,
}

impl IconResolver {
    pub fn new() -> Self {
        let mut path_index = HashMap::new();

        #[cfg(not(target_os = "windows"))]
        {
            let search_prefixes = [
                "/usr/share/icons/hicolor/48x48/apps",
                "/usr/share/icons/hicolor/scalable/apps",
                "/usr/share/icons/hicolor/64x64/apps",
                "/usr/share/icons/hicolor/32x32/apps",
                "/usr/share/icons/hicolor/128x128/apps",
                "/usr/share/icons/hicolor/256x256/apps",
                "/usr/share/icons/Yaru/48x48/apps",
                "/usr/share/icons/Yaru/scalable/apps",
                "/usr/share/icons/Adwaita/48x48/apps",
                "/usr/share/icons/Adwaita/scalable/apps",
                "/usr/share/pixmaps",
            ];

            for prefix in &search_prefixes {
                if let Ok(entries) = std::fs::read_dir(prefix) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let key = stem.to_lowercase();
                            if !path_index.contains_key(&key) {
                                path_index.insert(key, path);
                            }
                        }
                    }
                }
            }

            if let Some(home) = dirs::home_dir() {
                let user_icons = home.join(".local/share/icons/hicolor/48x48/apps");
                if let Ok(entries) = std::fs::read_dir(user_icons) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            path_index.insert(stem.to_lowercase(), path);
                        }
                    }
                }
            }
        }

        Self {
            icon_path_index: Arc::new(RwLock::new(path_index)),
            image_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Preloads and decodes icons for all apps in the background.
    pub fn preload_icons(&self, apps: &[LauncherItem]) {
        for app in apps {
            let _ = self.resolve_icon(app.icon.as_deref(), &app.name, &app.exec_or_path);
        }
    }

    /// Resolves and loads the native system icon into a `slint::Image` instantly.
    pub fn resolve_icon(&self, icon_hint: Option<&str>, app_name: &str, exec_or_path: &str) -> Option<slint::Image> {
        let key = icon_hint.unwrap_or(app_name);
        if key.is_empty() {
            return None;
        }

        // 1. Instant Cache Hit
        if let Ok(guard) = self.image_cache.read() {
            if let Some(cached) = guard.get(key) {
                return cached.clone();
            }
        }

        // 2. Fast O(1) Path Lookup
        let resolved_path = self.find_icon_path(icon_hint, app_name, exec_or_path);
        let slint_img = resolved_path.and_then(|p| slint::Image::load_from_path(&p).ok());

        // 3. Store in cache
        if let Ok(mut guard) = self.image_cache.write() {
            guard.insert(key.to_string(), slint_img.clone());
        }

        slint_img
    }

    fn find_icon_path(&self, icon_hint: Option<&str>, app_name: &str, exec_or_path: &str) -> Option<PathBuf> {
        #[cfg(not(target_os = "windows"))]
        {
            // Direct absolute path
            if let Some(hint) = icon_hint {
                let p = PathBuf::from(hint);
                if p.is_absolute() && p.exists() {
                    return Some(p);
                }
            }

            if let Ok(index) = self.icon_path_index.read() {
                // Check icon hint
                if let Some(hint) = icon_hint {
                    let k = hint.to_lowercase();
                    if let Some(path) = index.get(&k) {
                        return Some(path.clone());
                    }
                }

                // Check app name
                let name_key = app_name.to_lowercase();
                if let Some(path) = index.get(&name_key) {
                    return Some(path.clone());
                }

                // Check executable name
                if let Some(bin) = Path::new(exec_or_path).file_name().and_then(|f| f.to_str()) {
                    let bin_clean = bin.split_whitespace().next().unwrap_or("").to_lowercase();
                    if let Some(path) = index.get(&bin_clean) {
                        return Some(path.clone());
                    }
                }
            }

            None
        }

        #[cfg(target_os = "windows")]
        {
            let p = PathBuf::from(exec_or_path);
            if p.exists() && (p.extension().map_or(false, |ext| ext == "ico" || ext == "png")) {
                Some(p)
            } else {
                None
            }
        }
    }
}
