use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Thread-safe icon resolver that locates and caches native system PNG / SVG icons.
pub struct IconResolver {
    cache: Arc<RwLock<HashMap<String, Option<slint::Image>>>>,
}

impl IconResolver {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolves and loads the native system icon into a `slint::Image`.
    pub fn resolve_icon(&self, icon_hint: Option<&str>, app_name: &str, exec_or_path: &str) -> Option<slint::Image> {
        let key = icon_hint.unwrap_or(app_name);
        if key.is_empty() {
            return None;
        }

        // Check cache first
        if let Ok(guard) = self.cache.read() {
            if let Some(cached) = guard.get(key) {
                return cached.clone();
            }
        }

        // Find icon path on disk
        let resolved_path = self.find_icon_path(icon_hint, app_name, exec_or_path);
        let slint_img = resolved_path.and_then(|p| slint::Image::load_from_path(&p).ok());

        // Cache result
        if let Ok(mut guard) = self.cache.write() {
            guard.insert(key.to_string(), slint_img.clone());
        }

        slint_img
    }

    fn find_icon_path(&self, icon_hint: Option<&str>, app_name: &str, exec_or_path: &str) -> Option<PathBuf> {
        #[cfg(not(target_os = "windows"))]
        {
            // 1. Direct path in icon_hint
            if let Some(hint) = icon_hint {
                let p = PathBuf::from(hint);
                if p.is_absolute() && p.exists() {
                    return Some(p);
                }
            }

            // 2. Search XDG icon directories for icon_hint, then app_name, then binary name
            let mut candidates = Vec::new();
            if let Some(hint) = icon_hint {
                candidates.push(hint.to_string());
            }
            candidates.push(app_name.to_lowercase());
            
            // Extract executable binary name
            if let Some(bin) = Path::new(exec_or_path).file_name().and_then(|f| f.to_str()) {
                candidates.push(bin.split_whitespace().next().unwrap_or("").to_lowercase());
            }

            let search_prefixes = vec![
                "/usr/share/icons/hicolor/48x48/apps",
                "/usr/share/icons/hicolor/scalable/apps",
                "/usr/share/icons/hicolor/64x64/apps",
                "/usr/share/icons/hicolor/128x128/apps",
                "/usr/share/icons/hicolor/256x256/apps",
                "/usr/share/icons/hicolor/32x32/apps",
                "/usr/share/icons/Yaru/48x48/apps",
                "/usr/share/icons/Yaru/scalable/apps",
                "/usr/share/icons/Adwaita/48x48/apps",
                "/usr/share/icons/Adwaita/scalable/apps",
                "/usr/share/pixmaps",
            ];

            let home_icons = dirs::home_dir().map(|mut h| {
                h.push(".local/share/icons/hicolor/48x48/apps");
                h
            });

            for candidate in &candidates {
                if candidate.is_empty() {
                    continue;
                }

                // Check standard extensions
                let extensions = ["png", "svg", "xpm", "jpg"];

                for prefix in &search_prefixes {
                    for ext in &extensions {
                        let path = PathBuf::from(format!("{}/{}.{}", prefix, candidate, ext));
                        if path.exists() {
                            return Some(path);
                        }
                    }
                }

                if let Some(ref home_dir) = home_icons {
                    for ext in &extensions {
                        let path = home_dir.join(format!("{}.{}", candidate, ext));
                        if path.exists() {
                            return Some(path);
                        }
                    }
                }
            }

            None
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, if exec_or_path points to an existing file with image or ico
            let p = PathBuf::from(exec_or_path);
            if p.exists() && (p.extension().map_or(false, |ext| ext == "ico" || ext == "png")) {
                Some(p)
            } else {
                None
            }
        }
    }
}
