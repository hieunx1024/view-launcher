use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
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
const SVG_GEAR: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="12" cy="12" r="3" stroke="#98989D" stroke-width="1.8"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 01-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" stroke="#98989D" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_NAVIGATE: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M12 4L6 10H18L12 4Z" fill="#98989D"/><path d="M12 20L6 14H18L12 20Z" fill="#98989D"/></svg>"##;
const SVG_ENTER: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M19 6V13C19 14.1 18.1 15 17 15H6M6 15L10 11M6 15L10 19" stroke="#98989D" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;
const SVG_SEARCH: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="11" cy="11" r="7" stroke="#98989D" stroke-width="2.2"/><path d="M20 20L16 16" stroke="#98989D" stroke-width="2.2" stroke-linecap="round"/></svg>"##;
const SVG_BULB: &str = r##"<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M9 21h6M10 17h4M12 3a6 6 0 00-6 6c0 2.2 1.2 4.2 3 5.2V16a1 1 0 001 1h4a1 1 0 001-1v-1.8c1.8-1 3-3 3-5.2a6 6 0 00-6-6z" stroke="#FBBF24" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>"##;

/// High-performance thread-safe icon resolver with O(1) in-memory index and pre-decoding cache.
pub struct IconResolver {
    #[allow(dead_code)]
    icon_path_index: Arc<RwLock<HashMap<String, PathBuf>>>,
    image_cache: Arc<RwLock<HashMap<String, Option<slint::Image>>>>,
    file_type_cache: Arc<RwLock<HashMap<&'static str, slint::Image>>>,
    indexing_done: Arc<AtomicBool>,
}

impl IconResolver {
    pub fn new() -> Self {
        let path_index = Arc::new(RwLock::new(HashMap::new()));
        let image_cache: Arc<RwLock<HashMap<String, Option<slint::Image>>>> = Arc::new(RwLock::new(HashMap::new()));
        let indexing_done = Arc::new(AtomicBool::new(false));

        #[cfg(not(target_os = "windows"))]
        {
            let index_clone = path_index.clone();
            let indexing_done_clone = indexing_done.clone();

            std::thread::spawn(move || {
                let mut map: HashMap<String, (PathBuf, u8)> = HashMap::new();
                let mut search_roots = vec![
                    PathBuf::from("/usr/share/icons/Yaru"),
                    PathBuf::from("/usr/share/icons/hicolor"),
                    PathBuf::from("/usr/share/icons/Adwaita"),
                    PathBuf::from("/usr/share/icons/Humanity"),
                    PathBuf::from("/usr/share/icons/HighContrast"),
                    PathBuf::from("/usr/share/icons/Papirus"),
                    PathBuf::from("/usr/share/icons/breeze"),
                    PathBuf::from("/usr/share/pixmaps"),
                    PathBuf::from("/var/lib/snapd/desktop/icons"),
                    PathBuf::from("/var/lib/flatpak/exports/share/icons"),
                ];

                if let Some(home) = dirs::home_dir() {
                    search_roots.push(home.join(".local/share/icons"));
                    search_roots.push(home.join(".local/share/flatpak/exports/share/icons"));
                }

                if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
                    for dir in xdg_data_dirs.split(':') {
                        let p = PathBuf::from(dir).join("icons");
                        if !search_roots.contains(&p) {
                            search_roots.push(p);
                        }
                    }
                }

                for root in search_roots {
                    if !root.exists() {
                        continue;
                    }
                    for entry in walkdir::WalkDir::new(&root).max_depth(5).into_iter().flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                let ext_lower = ext.to_lowercase();
                                if ext_lower != "png" && ext_lower != "svg" && ext_lower != "xpm" && ext_lower != "ico" {
                                    continue;
                                }

                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                    let key = stem.to_lowercase();
                                    let path_str = path.to_string_lossy();

                                    // Priority score: Scalable SVG (5) > 256/128/64/48 PNG (4) > 32/24 PNG (3) > 16 PNG (2) > Symbolic SVG (1)
                                    let score = if path_str.contains("symbolic") {
                                        1
                                    } else if ext_lower == "svg" {
                                        5
                                    } else if path_str.contains("256x256") || path_str.contains("128x128") || path_str.contains("scalable") {
                                        5
                                    } else if path_str.contains("64x64") || path_str.contains("48x48") {
                                        4
                                    } else if path_str.contains("32x32") || path_str.contains("24x24") {
                                        3
                                    } else {
                                        2
                                    };

                                    let insert_key = |map: &mut HashMap<String, (PathBuf, u8)>, k: String, p: PathBuf, s: u8| {
                                        if let Some((_, old_score)) = map.get(&k) {
                                            if s > *old_score {
                                                map.insert(k, (p, s));
                                            }
                                        } else {
                                            map.insert(k, (p, s));
                                        }
                                    };

                                    // 1. Exact stem (e.g. "org.gnome.texteditor", "firefox")
                                    insert_key(&mut map, key.clone(), path.to_path_buf(), score);

                                    // 2. Strip "-symbolic" if present
                                    if key.ends_with("-symbolic") {
                                        let non_sym = key.trim_end_matches("-symbolic").to_string();
                                        insert_key(&mut map, non_sym, path.to_path_buf(), 1);
                                    }

                                    // 3. If reverse domain (e.g. "com.mattjakeman.extensionmanager"), also index last segment ("extensionmanager")
                                    if let Some(last_seg) = key.split('.').last() {
                                        if last_seg.len() > 2 && last_seg != key {
                                            insert_key(&mut map, last_seg.to_string(), path.to_path_buf(), score);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let final_map: HashMap<String, PathBuf> = map.into_iter().map(|(k, (p, _))| (k, p)).collect();

                if let Ok(mut lock) = index_clone.write() {
                    *lock = final_map;
                }

                indexing_done_clone.store(true, Ordering::SeqCst);
            });
        }

        #[cfg(target_os = "windows")]
        {
            indexing_done.store(true, Ordering::SeqCst);
        }

        Self {
            icon_path_index: path_index,
            image_cache,
            file_type_cache: Arc::new(RwLock::new(HashMap::new())),
            indexing_done,
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

        let slint_img = Self::rasterize_svg_bytes(svg_str.as_bytes(), 64)?;

        if let Ok(mut guard) = self.file_type_cache.write() {
            guard.insert(category_key, slint_img.clone());
        }

        Some(slint_img)
    }

    pub fn get_gear_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_GEAR.as_bytes(), 48).unwrap_or_default()
    }

    pub fn get_nav_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_NAVIGATE.as_bytes(), 36).unwrap_or_default()
    }

    pub fn get_enter_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_ENTER.as_bytes(), 36).unwrap_or_default()
    }

    pub fn get_search_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_SEARCH.as_bytes(), 48).unwrap_or_default()
    }

    pub fn get_bulb_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_BULB.as_bytes(), 36).unwrap_or_default()
    }

    pub fn get_folder_icon(&self) -> slint::Image {
        Self::rasterize_svg_bytes(SVG_FOLDER.as_bytes(), 72).unwrap_or_default()
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
        let key = if let Some(hint) = icon_hint {
            if !hint.is_empty() { hint } else if !exec_or_path.is_empty() { exec_or_path } else { app_name }
        } else if !exec_or_path.is_empty() {
            exec_or_path
        } else {
            app_name
        };
        if key.is_empty() {
            return None;
        }

        // 1. Instant Cache Hit (only return cached None if indexing has completed)
        if let Ok(guard) = self.image_cache.read() {
            if let Some(cached) = guard.get(key) {
                if let Some(ref img) = *cached {
                    return Some(img.clone());
                } else if self.indexing_done.load(Ordering::Relaxed) {
                    return None;
                }
            }
        }

        // 2. Direct Image File Check
        let is_supported_image = |p: &Path| -> bool {
            p.extension().map_or(false, |ext| {
                let ext_str = ext.to_string_lossy();
                ext_str.eq_ignore_ascii_case("ico") 
                    || ext_str.eq_ignore_ascii_case("png") 
                    || ext_str.eq_ignore_ascii_case("svg")
                    || ext_str.eq_ignore_ascii_case("jpg")
                    || ext_str.eq_ignore_ascii_case("jpeg")
                    || ext_str.eq_ignore_ascii_case("webp")
                    || ext_str.eq_ignore_ascii_case("bmp")
                    || ext_str.eq_ignore_ascii_case("gif")
            })
        };

        let mut slint_img = None;

        if let Some(hint) = icon_hint {
            let p = Path::new(hint);
            if p.exists() && is_supported_image(p) {
                slint_img = Self::load_downscaled_icon(p);
            }
        }

        if slint_img.is_none() && !exec_or_path.is_empty() {
            let p = Path::new(exec_or_path);
            if p.exists() && is_supported_image(p) {
                slint_img = Self::load_downscaled_icon(p);
            }
        }

        // 3. Platform specific native resolution
        #[cfg(target_os = "windows")]
        if slint_img.is_none() {
            let candidate = if let Some(hint) = icon_hint {
                if Path::new(hint).exists() {
                    Some(hint)
                } else {
                    None
                }
            } else {
                None
            }.or_else(|| {
                if Path::new(exec_or_path).exists() {
                    Some(exec_or_path)
                } else {
                    None
                }
            });

            if let Some(target_path) = candidate {
                slint_img = Self::extract_windows_icon(target_path);
            }
        }

        #[cfg(not(target_os = "windows"))]
        if slint_img.is_none() {
            let resolved_path = self.find_icon_path(icon_hint, app_name, exec_or_path);
            slint_img = resolved_path.and_then(|p| Self::load_downscaled_icon(&p));
        }

        // 4. Store in cache (only store None if background indexing has completed)
        if let Ok(mut guard) = self.image_cache.write() {
            if slint_img.is_some() || self.indexing_done.load(Ordering::Relaxed) {
                guard.insert(key.to_string(), slint_img.clone());
            }
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
            let thumb = dyn_img.thumbnail(96, 96).to_rgba8();
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
        let mut pixmap = tiny_skia::Pixmap::new(96, 96)?;
        let size = tree.size();
        let sx = 96.0 / size.width();
        let sy = 96.0 / size.height();
        let scale = sx.min(sy);
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        
        let mut pixel_buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(96, 96);
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

    #[allow(dead_code)]
    fn find_icon_path(&self, #[allow(unused_variables)] icon_hint: Option<&str>, #[allow(unused_variables)] app_name: &str, #[allow(unused_variables)] exec_or_path: &str) -> Option<PathBuf> {
        #[cfg(not(target_os = "windows"))]
        {
            // 1. Direct path
            if let Some(hint) = icon_hint {
                let p = PathBuf::from(hint);
                if p.exists() {
                    return Some(p);
                }
            }

            if let Ok(index) = self.icon_path_index.read() {
                // 2. Check icon hint variations
                if let Some(hint) = icon_hint {
                    let h_lower = hint.to_lowercase();
                    // a. Exact lowercase
                    if let Some(path) = index.get(&h_lower) {
                        return Some(path.clone());
                    }
                    // b. Strip extension (.png, .svg, .xpm, .ico)
                    let h_no_ext = h_lower.trim_end_matches(".png").trim_end_matches(".svg").trim_end_matches(".xpm").trim_end_matches(".ico");
                    if let Some(path) = index.get(h_no_ext) {
                        return Some(path.clone());
                    }
                    // c. Strip -symbolic
                    if h_no_ext.ends_with("-symbolic") {
                        let h_no_sym = h_no_ext.trim_end_matches("-symbolic");
                        if let Some(path) = index.get(h_no_sym) {
                            return Some(path.clone());
                        }
                    }
                    // d. Reverse-domain last segment (e.g. "com.mattjakeman.ExtensionManager" -> "extensionmanager")
                    if let Some(last_seg) = h_no_ext.split('.').last() {
                        if last_seg.len() > 2 && last_seg != h_no_ext {
                            if let Some(path) = index.get(last_seg) {
                                return Some(path.clone());
                            }
                        }
                    }
                }

                // 3. Check app name variations
                let name_key = app_name.to_lowercase();
                if let Some(path) = index.get(&name_key) {
                    return Some(path.clone());
                }
                // App name without spaces (e.g. "Text Editor" -> "texteditor", "Extension Manager" -> "extensionmanager")
                let name_compact: String = name_key.chars().filter(|c| c.is_alphanumeric()).collect();
                if !name_compact.is_empty() && name_compact != name_key {
                    if let Some(path) = index.get(&name_compact) {
                        return Some(path.clone());
                    }
                }

                // 4. Check executable name variations
                if let Some(bin) = Path::new(exec_or_path).file_name().and_then(|f| f.to_str()) {
                    let bin_clean = bin.split_whitespace().next().unwrap_or("").to_lowercase();
                    if let Some(path) = index.get(&bin_clean) {
                        return Some(path.clone());
                    }
                    // Executable without '-' or '_' (e.g. "gnome-text-editor" -> "texteditor" or "gnometexteditor")
                    if let Some(last_bin) = bin_clean.split('-').last() {
                        if last_bin.len() > 2 && last_bin != bin_clean {
                            if let Some(path) = index.get(last_bin) {
                                return Some(path.clone());
                            }
                        }
                    }
                }
            }

            None
        }

        #[cfg(target_os = "windows")]
        {
            let is_supported_image = |p: &Path| -> bool {
                p.extension().map_or(false, |ext| {
                    let ext_str = ext.to_string_lossy();
                    ext_str.eq_ignore_ascii_case("ico") 
                        || ext_str.eq_ignore_ascii_case("png") 
                        || ext_str.eq_ignore_ascii_case("svg")
                        || ext_str.eq_ignore_ascii_case("jpg")
                        || ext_str.eq_ignore_ascii_case("jpeg")
                        || ext_str.eq_ignore_ascii_case("webp")
                        || ext_str.eq_ignore_ascii_case("bmp")
                        || ext_str.eq_ignore_ascii_case("gif")
                })
            };

            if let Some(hint) = icon_hint {
                let p = PathBuf::from(hint);
                if p.exists() && is_supported_image(&p) {
                    return Some(p);
                }
            }
            let p = PathBuf::from(exec_or_path);
            if p.exists() && is_supported_image(&p) {
                return Some(p);
            }
            None
        }
    }

    /// Extracts a high-quality icon directly from Windows Shell (.lnk, .exe, .url, .msc, .cpl, etc.)
    #[cfg(target_os = "windows")]
    pub fn extract_windows_icon(path_str: &str) -> Option<slint::Image> {
        use windows_sys::Win32::UI::Shell::{
            SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{PrivateExtractIconsW, HICON};

        let clean_path = path_str.trim().trim_matches('"');
        let wide_path: Vec<u16> = clean_path.encode_utf16().chain(std::iter::once(0)).collect();

        // 1. If it is an executable, DLL, or .ico, try PrivateExtractIconsW first (crisp 48x48 icon)
        let is_binary_or_ico = clean_path.ends_with(".exe")
            || clean_path.ends_with(".EXE")
            || clean_path.ends_with(".ico")
            || clean_path.ends_with(".ICO")
            || clean_path.ends_with(".dll")
            || clean_path.ends_with(".DLL");

        if is_binary_or_ico {
            let mut hicon: HICON = std::ptr::null_mut();
            let mut icon_id = 0;
            let count = unsafe {
                PrivateExtractIconsW(
                    wide_path.as_ptr(),
                    0,
                    48,
                    48,
                    &mut hicon,
                    &mut icon_id,
                    1,
                    0,
                )
            };
            if count > 0 && !hicon.is_null() {
                if let Some(img) = unsafe { Self::hicon_to_slint_image(hicon) } {
                    return Some(img);
                }
            }
        }

        // 2. Try SHGetFileInfoW (resolves .lnk shortcuts, .url, .msc, documents, folders, etc.)
        let mut shfi: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let res = unsafe {
            SHGetFileInfoW(
                wide_path.as_ptr(),
                0,
                &mut shfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            )
        };

        if res != 0 && !shfi.hIcon.is_null() {
            if let Some(img) = unsafe { Self::hicon_to_slint_image(shfi.hIcon) } {
                return Some(img);
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    unsafe fn hicon_to_slint_image(hicon: windows_sys::Win32::UI::WindowsAndMessaging::HICON) -> Option<slint::Image> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetIconInfo, DestroyIcon, ICONINFO};
        use windows_sys::Win32::Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, DeleteObject, GetObjectW, GetDIBits,
            BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        };

        if hicon.is_null() {
            return None;
        }

        unsafe {
            let mut icon_info: ICONINFO = std::mem::zeroed();
            if GetIconInfo(hicon, &mut icon_info) == 0 {
                DestroyIcon(hicon);
                return None;
            }

            let hbm_color = icon_info.hbmColor;
            let hbm_mask = icon_info.hbmMask;

            let hbm_target = if !hbm_color.is_null() { hbm_color } else { hbm_mask };
            if hbm_target.is_null() {
                if !hbm_mask.is_null() { DeleteObject(hbm_mask as _); }
                DestroyIcon(hicon);
                return None;
            }

            let mut bm: BITMAP = std::mem::zeroed();
            if GetObjectW(
                hbm_target as _,
                std::mem::size_of::<BITMAP>() as i32,
                &mut bm as *mut _ as *mut _,
            ) == 0 {
                if !hbm_color.is_null() { DeleteObject(hbm_color as _); }
                if !hbm_mask.is_null() { DeleteObject(hbm_mask as _); }
                DestroyIcon(hicon);
                return None;
            }

            let width = bm.bmWidth;
            let height = if !hbm_color.is_null() { bm.bmHeight } else { bm.bmHeight / 2 };

            if width <= 0 || height <= 0 {
                if !hbm_color.is_null() { DeleteObject(hbm_color as _); }
                if !hbm_mask.is_null() { DeleteObject(hbm_mask as _); }
                DestroyIcon(hicon);
                return None;
            }

            let hdc = CreateCompatibleDC(std::ptr::null_mut());
            if hdc.is_null() {
                if !hbm_color.is_null() { DeleteObject(hbm_color as _); }
                if !hbm_mask.is_null() { DeleteObject(hbm_mask as _); }
                DestroyIcon(hicon);
                return None;
            }

            let mut bi: BITMAPINFOHEADER = std::mem::zeroed();
            bi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.biWidth = width;
            bi.biHeight = -height; // Negative for top-down DIB
            bi.biPlanes = 1;
            bi.biBitCount = 32;
            bi.biCompression = BI_RGB;

            let mut bgra_buf = vec![0u8; (width * height * 4) as usize];

            if !hbm_color.is_null() {
                GetDIBits(
                    hdc,
                    hbm_color as _,
                    0,
                    height as u32,
                    bgra_buf.as_mut_ptr() as *mut _,
                    &mut bi as *mut _ as *mut BITMAPINFO,
                    DIB_RGB_COLORS,
                );
            }

            // Check if 32-bit color buffer already has non-zero alpha channel
            let mut has_valid_alpha = false;
            if !hbm_color.is_null() {
                for chunk in bgra_buf.chunks_exact(4) {
                    if chunk[3] != 0 {
                        has_valid_alpha = true;
                        break;
                    }
                }
            }

            // If no valid alpha channel, use mask bitmap to compute transparency
            if !has_valid_alpha && !hbm_mask.is_null() {
                let mask_height_total = if !hbm_color.is_null() { height } else { height * 2 };
                let mut mask_bi = bi;
                mask_bi.biHeight = -mask_height_total;
                let mut mask_buf = vec![0u8; (width * mask_height_total * 4) as usize];

                GetDIBits(
                    hdc,
                    hbm_mask as _,
                    0,
                    mask_height_total as u32,
                    mask_buf.as_mut_ptr() as *mut _,
                    &mut mask_bi as *mut _ as *mut BITMAPINFO,
                    DIB_RGB_COLORS,
                );

                if !hbm_color.is_null() {
                    // Color icon with separate 1-bit mask
                    for (i, mask_chunk) in mask_buf.chunks_exact(4).enumerate() {
                        let is_transparent = mask_chunk[0] != 0 || mask_chunk[1] != 0 || mask_chunk[2] != 0;
                        let dest_idx = i * 4;
                        bgra_buf[dest_idx + 3] = if is_transparent { 0 } else { 255 };
                    }
                } else {
                    // Monochrome icon: upper half is AND mask, lower half is XOR mask
                    let pixels_per_half = (width * height) as usize;
                    for i in 0..pixels_per_half {
                        let and_val = mask_buf[i * 4];
                        let xor_val = mask_buf[(pixels_per_half + i) * 4];
                        let is_transparent = and_val != 0;
                        let color = if xor_val != 0 { 255 } else { 0 };
                        let dest_idx = i * 4;
                        bgra_buf[dest_idx] = color;
                        bgra_buf[dest_idx + 1] = color;
                        bgra_buf[dest_idx + 2] = color;
                        bgra_buf[dest_idx + 3] = if is_transparent { 0 } else { 255 };
                    }
                }
            }

            DeleteDC(hdc);
            if !hbm_color.is_null() { DeleteObject(hbm_color as _); }
            if !hbm_mask.is_null() { DeleteObject(hbm_mask as _); }
            DestroyIcon(hicon);

            // Convert BGRA to RGBA in Slint SharedPixelBuffer
            let mut pixel_buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width as u32, height as u32);
            let dest = pixel_buf.make_mut_slice();
            for (src_chunk, dest_pixel) in bgra_buf.chunks_exact(4).zip(dest.iter_mut()) {
                *dest_pixel = slint::Rgba8Pixel {
                    r: src_chunk[2], // BGRA -> RGBA
                    g: src_chunk[1],
                    b: src_chunk[0],
                    a: src_chunk[3],
                };
            }

            Some(slint::Image::from_rgba8(pixel_buf))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_resolver_basic() {
        let resolver = IconResolver::new();
        let start = std::time::Instant::now();
        while !resolver.indexing_done.load(Ordering::SeqCst) && start.elapsed().as_secs() < 3 {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }

        // 1. Built-in file type icon resolution
        assert!(resolver.resolve_file_type_icon(Path::new("test.rs"), ItemType::File).is_some());
        assert!(resolver.resolve_file_type_icon(Path::new("photo.png"), ItemType::File).is_some());
        assert!(resolver.resolve_file_type_icon(Path::new("doc.pdf"), ItemType::File).is_some());
        assert!(resolver.resolve_file_type_icon(Path::new("archive.zip"), ItemType::File).is_some());
        assert!(resolver.resolve_file_type_icon(Path::new("folder"), ItemType::Dir).is_some());

        // 2. Built-in UI helper icons
        let _ = resolver.get_gear_icon();
        let _ = resolver.get_nav_icon();
        let _ = resolver.get_enter_icon();
        let _ = resolver.get_search_icon();
        let _ = resolver.get_bulb_icon();
        let _ = resolver.get_folder_icon();

        // 3. Workspace asset icon resolution
        let asset_svg = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("view-launcher.svg");
        if asset_svg.exists() {
            let svg_icon = resolver.resolve_icon(asset_svg.to_str(), "View Launcher", "view-launcher");
            assert!(svg_icon.is_some(), "Asset SVG should resolve when file exists");
        }

        let asset_png = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("view-launcher.png");
        if asset_png.exists() {
            let png_icon = resolver.resolve_icon(asset_png.to_str(), "View Launcher", "view-launcher");
            assert!(png_icon.is_some(), "Asset PNG should resolve when file exists");
        }

        // 4. Non-existent app icon gracefully handles None
        let _ = resolver.resolve_icon(Some("non.existent.random_app_xyz_987"), "RandomAppXYZ", "random-app-xyz");

        // 5. System app resolution if available on live system
        let p_ext = resolver.find_icon_path(Some("com.mattjakeman.ExtensionManager"), "Extension Manager", "extension-manager");
        println!("Extension Manager icon path: {:?}", p_ext);
        if p_ext.is_some() {
            let ext_icon = resolver.resolve_icon(Some("com.mattjakeman.ExtensionManager"), "Extension Manager", "extension-manager");
            assert!(ext_icon.is_some(), "Extension Manager icon should resolve when path is found");
        }

        let p_text = resolver.find_icon_path(Some("org.gnome.TextEditor"), "Text Editor", "gnome-text-editor");
        println!("Text Editor icon path: {:?}", p_text);
        if p_text.is_some() {
            let text_icon = resolver.resolve_icon(Some("org.gnome.TextEditor"), "Text Editor", "gnome-text-editor");
            assert!(text_icon.is_some(), "Text Editor icon should resolve when path is found");
        }
    }
}
