use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use crate::launcher::{LauncherItem, ItemType};

const SVG_FOLDER: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" fill="#EBCB8B" stroke="#D08770" stroke-width="1.2"/></svg>"##;
const SVG_CODE: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="3" y="3" width="18" height="18" rx="4" fill="#A78BFA22" stroke="#A78BFA" stroke-width="1.5"/><path d="M9 8.5L5.5 12 9 15.5M15 8.5l3.5 3.5L15 15.5M13 7l-2 10" stroke="#A78BFA" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_DOCUMENT: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M6 3h8l5 5v13a1 1 0 01-1 1H6a1 1 0 01-1-1V4a1 1 0 011-1z" fill="#7AA2F722" stroke="#7AA2F7" stroke-width="1.5"/><path d="M14 3v5h5M9 12h6M9 16h4" stroke="#7AA2F7" stroke-width="1.5" stroke-linecap="round"/></svg>"##;
const SVG_IMAGE: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="3" y="3" width="18" height="18" rx="4" fill="#2AC3DE22" stroke="#2AC3DE" stroke-width="1.5"/><circle cx="8.5" cy="8.5" r="1.5" fill="#2AC3DE"/><path d="M21 15l-5-5L5 21" stroke="#2AC3DE" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_PDF: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M6 3h8l5 5v13a1 1 0 01-1 1H6a1 1 0 01-1-1V4a1 1 0 011-1z" fill="#F7768E22" stroke="#F7768E" stroke-width="1.5"/><path d="M14 3v5h5" stroke="#F7768E" stroke-width="1.5"/><text x="6.5" y="16" fill="#F7768E" font-size="6" font-weight="bold" font-family="sans-serif">PDF</text></svg>"##;
const SVG_ARCHIVE: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="4" y="4" width="16" height="16" rx="3" fill="#FBBF2422" stroke="#FBBF24" stroke-width="1.5"/><path d="M10 4v16M14 4v16M10 7h4M10 10h4M10 13h4M10 16h4" stroke="#FBBF24" stroke-width="1.2"/></svg>"##;
const SVG_AUDIO: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="9" fill="#BB9AF722" stroke="#BB9AF7" stroke-width="1.5"/><path d="M10 15a2 2 0 102-2V7h4" stroke="#BB9AF7" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_VIDEO: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="3" y="4" width="18" height="16" rx="3" fill="#7DCFFF22" stroke="#7DCFFF" stroke-width="1.5"/><path d="M10 9l5 3-5 3V9z" fill="#7DCFFF"/></svg>"##;
const SVG_GENERIC: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M6 3h8l5 5v13a1 1 0 01-1 1H6a1 1 0 01-1-1V4a1 1 0 011-1z" fill="#9AA5CE22" stroke="#9AA5CE" stroke-width="1.5"/><path d="M14 3v5h5" stroke="#9AA5CE" stroke-width="1.5"/></svg>"##;
const SVG_GEAR: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="3" stroke="#7DCFFF" stroke-width="1.8"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" stroke="#7DCFFF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_NAVIGATE: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 4L6 10H18L12 4Z" fill="#7AA2F7"/><path d="M12 20L6 14H18L12 20Z" fill="#7AA2F7"/></svg>"##;
const SVG_ENTER: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M19 6V13C19 14.1 18.1 15 17 15H6M6 15L10 11M6 15L10 19" stroke="#7AA2F7" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_SEARCH: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="11" cy="11" r="7" stroke="#7AA2F7" stroke-width="2.2"/><path d="M20 20L16 16" stroke="#7AA2F7" stroke-width="2.2" stroke-linecap="round"/></svg>"##;
const SVG_BULB: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M9 21h6M10 17h4M12 3a6 6 0 00-6 6c0 2.2 1.2 4.2 3 5.2V16a1 1 0 001 1h4a1 1 0 001-1v-1.8c1.8-1 3-3 3-5.2a6 6 0 00-6-6z" stroke="#FBBF24" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// High-performance thread-safe icon resolver with O(1) in-memory index and pre-decoding cache.
pub struct IconResolver {
    icon_path_index: Arc<RwLock<HashMap<String, PathBuf>>>,
    image_cache: Arc<RwLock<HashMap<String, Option<slint::Image>>>>,
    file_type_cache: Arc<RwLock<HashMap<&'static str, slint::Image>>>,
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
            file_type_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Resolves file type icons (.rs, .png, .pdf, folder, etc.) with instant caching.
    pub fn resolve_file_type_icon(&self, path: &Path, item_type: ItemType) -> Option<slint::Image> {
        let category_key = match item_type {
            ItemType::Dir => "folder",
            ItemType::File => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                match ext.as_str() {
                    "txt" | "md" | "log" | "rtf" | "csv" | "tsv" | "doc" | "docx" | "odt" => "document",
                    "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" => "image",
                    "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "toml" | "json" | "c" | "cpp" | "h" | "hpp" | "go" | "html" | "css" | "sh" | "bash" | "zsh" | "yaml" | "yml" | "sql" | "java" | "kt" | "lua" | "vue" | "svelte" => "code",
                    "pdf" => "pdf",
                    "zip" | "tar" | "gz" | "7z" | "rar" | "xz" | "bz2" | "zst" | "iso" => "archive",
                    "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => "audio",
                    "mp4" | "mkv" | "mov" | "avi" | "webm" | "flv" | "wmv" => "video",
                    _ => "generic",
                }
            }
            _ => return None,
        };

        // 1. Instant Cache Hit
        if let Ok(guard) = self.file_type_cache.read() {
            if let Some(img) = guard.get(category_key) {
                return Some(img.clone());
            }
        }

        // 2. Render SVG for type
        let svg_str = match category_key {
            "folder" => SVG_FOLDER,
            "document" => SVG_DOCUMENT,
            "image" => SVG_IMAGE,
            "code" => SVG_CODE,
            "pdf" => SVG_PDF,
            "archive" => SVG_ARCHIVE,
            "audio" => SVG_AUDIO,
            "video" => SVG_VIDEO,
            _ => SVG_GENERIC,
        };

        let slint_img = Self::rasterize_svg_bytes(svg_str.as_bytes(), 32)?;

        if let Ok(mut guard) = self.file_type_cache.write() {
            guard.insert(category_key, slint_img.clone());
        }

        Some(slint_img)
    }

    pub fn get_gear_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_GEAR.as_bytes(), 20).unwrap_or_default()
    }

    pub fn get_nav_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_NAVIGATE.as_bytes(), 14).unwrap_or_default()
    }

    pub fn get_enter_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_ENTER.as_bytes(), 14).unwrap_or_default()
    }

    pub fn get_search_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_SEARCH.as_bytes(), 18).unwrap_or_default()
    }

    pub fn get_bulb_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_BULB.as_bytes(), 14).unwrap_or_default()
    }

    pub fn rasterize_svg_bytes(svg_data: &[u8], size_px: u32) -> Option<slint::Image> {
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(svg_data, &opt).ok()?;
        let mut pixmap = tiny_skia::Pixmap::new(size_px, size_px)?;
        let size = tree.size();
        let sx = size_px as f32 / size.width();
        let sy = size_px as f32 / size.height();
        let scale = sx.min(sy);
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        
        let mut pixel_buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(size_px, size_px);
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
