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
            let search_roots = [
                "/usr/share/icons/Yaru",
                "/usr/share/icons/hicolor",
                "/usr/share/icons/Adwaita",
                "/usr/share/icons/Humanity",
                "/usr/share/icons/HighContrast",
                "/usr/share/pixmaps",
            ];

            for root in &search_roots {
                for entry in walkdir::WalkDir::new(root).max_depth(4).into_iter().flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let key = stem.to_lowercase();
                            if !path_index.contains_key(&key) {
                                path_index.insert(key, path.to_path_buf());
                            }
                        }
                    }
                }
            }

            if let Some(home) = dirs::home_dir() {
                let user_icons = home.join(".local/share/icons");
                for entry in walkdir::WalkDir::new(user_icons).max_depth(4).into_iter().flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let key = stem.to_lowercase();
                            if !path_index.contains_key(&key) {
                                path_index.insert(key, path.to_path_buf());
                            }
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
        let slint_img = resolved_path.and_then(|p| Self::load_downscaled_icon(&p));

        // 3. Store in cache
        if let Ok(mut guard) = self.image_cache.write() {
            guard.insert(key.to_string(), slint_img.clone());
        }

        slint_img
    }

    fn load_downscaled_icon(path: &Path) -> Option<slint::Image> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext.eq_ignore_ascii_case("svg") {
                if let Some(img) = Self::rasterize_svg(path) {
                    return Some(img);
                }
            }
        }

        if let Ok(dyn_img) = image::open(path) {
            let thumb = dyn_img.thumbnail(48, 48).to_rgba8();
            let (w, h) = thumb.dimensions();
            let mut pixel_buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
            let raw_slice = thumb.into_raw();
            
            let dest = pixel_buf.make_mut_slice();
            for (src_chunk, dest_pixel) in raw_slice.chunks_exact(4).zip(dest.iter_mut()) {
                *dest_pixel = slint::Rgba8Pixel {
                    r: src_chunk[0],
                    g: src_chunk[1],
                    b: src_chunk[2],
                    a: src_chunk[3],
                };
            }
            return Some(slint::Image::from_rgba8(pixel_buf));
        }

        None
    }

    fn rasterize_svg(path: &Path) -> Option<slint::Image> {
        let svg_data = std::fs::read(path).ok()?;
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(&svg_data, &opt).ok()?;
        let mut pixmap = tiny_skia::Pixmap::new(48, 48)?;
        let size = tree.size();
        let sx = 48.0 / size.width();
        let sy = 48.0 / size.height();
        let scale = sx.min(sy);
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        
        let mut pixel_buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(48, 48);
        let raw_slice = pixmap.data();
        let dest = pixel_buf.make_mut_slice();
        for (src_chunk, dest_pixel) in raw_slice.chunks_exact(4).zip(dest.iter_mut()) {
            *dest_pixel = slint::Rgba8Pixel {
                r: src_chunk[0],
                g: src_chunk[1],
                b: src_chunk[2],
                a: src_chunk[3],
            };
        }
        Some(slint::Image::from_rgba8(pixel_buf))
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
