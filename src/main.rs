#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[allow(unused_imports)]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use slint::ComponentHandle;
use view_launcher::config::Config;
use view_launcher::icon_resolver::IconResolver;
use view_launcher::launcher::{self, LauncherEngine, LauncherItem, open_config_file};
use view_launcher::{AppWindow, LauncherItemData};
use i_slint_backend_winit::WinitWindowAccessor;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(unix)]
use std::io::Write;

#[cfg(windows)]
use std::net::{TcpListener, TcpStream, SocketAddr};
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use std::io::{Read, Write};

#[cfg(unix)]
fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("view-launcher.sock")
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
        PathBuf::from(format!("/tmp/view-launcher-{}.sock", user))
    }
}

#[cfg(unix)]
fn handle_single_instance(exit_trigger: Arc<AtomicBool>, ui_handle: slint::Weak<AppWindow>) -> bool {
    let socket_path = get_socket_path();

    // 1. Try connecting to existing socket to toggle window
    if let Ok(mut stream) = UnixStream::connect(&socket_path) {
        let _ = stream.write_all(b"toggle");
        return false;
    }

    // 2. Remove stale socket file if it exists
    let _ = std::fs::remove_file(&socket_path);

    // 3. Bind new Unix socket listener
    if let Ok(listener) = UnixListener::bind(&socket_path) {
        let path_clone = socket_path.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_ok() {
                    let _ = slint::invoke_from_event_loop({
                        let ui_weak = ui_handle.clone();
                        let exit_flag = exit_trigger.clone();
                        move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                if ui.window().is_visible() {
                                    exit_flag.store(true, Ordering::SeqCst);
                                    let _ = ui.hide();
                                } else {
                                    ui.set_search_text("".into());
                                    ui.set_is_expanded(false);
                                    let _ = ui.show();
                                    ui.invoke_focus_search();
                                }
                            }
                        }
                    });
                }
            }
            let _ = std::fs::remove_file(&path_clone);
        });
    }

    true
}

#[cfg(windows)]
fn handle_single_instance(exit_trigger: Arc<AtomicBool>, ui_handle: slint::Weak<AppWindow>) -> bool {
    let addr: SocketAddr = "127.0.0.1:42425".parse().unwrap();
    // 1. Try connecting with a fast timeout (50ms) to toggle existing instance
    if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(50)) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(50)));
        let _ = stream.write_all(b"toggle");
        return false;
    }

    // 2. Bind TCP listener on localhost
    if let Ok(listener) = TcpListener::bind(addr) {
        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let mut buf = [0u8; 6];
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                    let _ = stream.read(&mut buf);

                    let _ = slint::invoke_from_event_loop({
                        let ui_weak = ui_handle.clone();
                        let exit_flag = exit_trigger.clone();
                        move || {
                            if let Some(ui) = ui_weak.upgrade() {
                                if ui.window().is_visible() {
                                    exit_flag.store(true, Ordering::SeqCst);
                                    let _ = ui.hide();
                                } else {
                                    ui.set_search_text("".into());
                                    ui.set_is_expanded(false);
                                    let _ = ui.show();
                                    ui.invoke_focus_search();
                                }
                            }
                        }
                    });
                }
            }
        });
    }

    true
}

fn populate_items(
    ui: &AppWindow,
    engine: &LauncherEngine,
    icon_resolver: &IconResolver,
    query: &str,
) -> Vec<LauncherItem> {
    let trimmed = query.trim();
    let is_file_mode = trimmed.starts_with("@f") || trimmed.starts_with("@file");
    let is_win_mode = trimmed.starts_with("@w") || trimmed.starts_with("@win");
    let is_clip_mode = trimmed.starts_with("@c") || trimmed.starts_with("@clip");
    let is_sys_mode = trimmed.starts_with("@sys") || trimmed.starts_with("@power");

    let mode_badge = if is_file_mode {
        "Files"
    } else if is_win_mode {
        "Windows"
    } else if is_clip_mode {
        "Clipboard"
    } else if is_sys_mode {
        "System"
    } else {
        ""
    };
    ui.set_mode_badge_text(mode_badge.into());
    ui.set_is_file_mode(is_file_mode);

    let results = engine.search(query);
    let count = results.len();
    ui.set_has_results(count > 0);

    let (mode_title, item_count_suffix) = if is_file_mode {
        ("FILE SEARCH", "files")
    } else if is_win_mode {
        ("WINDOWS", "windows")
    } else if is_clip_mode {
        ("CLIPBOARD", "items")
    } else if is_sys_mode {
        ("SYSTEM", "actions")
    } else {
        ("APPLICATIONS", "apps")
    };
    ui.set_mode_title(mode_title.into());
    ui.set_status_item_count(format!("{} {}", count, item_count_suffix).into());

    let mut current_items = Vec::new();
    let mut slint_items = Vec::new();

    let file_query = if trimmed.starts_with("@file ") {
        trimmed[6..].trim()
    } else if trimmed.starts_with("@f ") {
        trimmed[3..].trim()
    } else if is_file_mode {
        ""
    } else {
        trimmed
    };

    let is_dir_view = if let Some((dir, filter)) = engine.resolve_path_search(file_query) {
        filter.is_empty() && dir.is_dir()
    } else {
        false
    };

    if count == 0 {
        if is_dir_view {
            ui.set_empty_state_title("Folder is empty".into());
            ui.set_empty_state_subtitle("Press Backspace to go back to parent folder".into());
            ui.set_is_empty_folder(true);
        } else if is_win_mode {
            ui.set_empty_state_title("No matching windows found".into());
            ui.set_empty_state_subtitle("Make sure target application window is running".into());
            ui.set_is_empty_folder(false);
        } else if is_clip_mode {
            ui.set_empty_state_title("Clipboard is empty".into());
            ui.set_empty_state_subtitle("Copy some text to see it appear in clipboard history".into());
            ui.set_is_empty_folder(false);
        } else if !trimmed.is_empty() {
            ui.set_empty_state_title(format!("No results for '{}'", trimmed).into());
            ui.set_empty_state_subtitle("Try searching for an app, file, or math command".into());
            ui.set_is_empty_folder(false);
        } else {
            ui.set_empty_state_title("No applications found".into());
            ui.set_empty_state_subtitle("Try searching for an app, file, or math command".into());
            ui.set_is_empty_folder(false);
        }
    } else {
        ui.set_is_empty_folder(false);
    }

    const MAX_COMPUTE_RESULTS: usize = 50;
    for (item, _indices) in results.into_iter().take(MAX_COMPUTE_RESULTS) {
        let (slint_icon, category) = match item.item_type {
            launcher::ItemType::App => {
                let icon = icon_resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path);
                (icon, item.get_category_tag().to_string())
            }
            launcher::ItemType::Dir => {
                let icon = icon_resolver.resolve_file_type_icon(Path::new(&item.exec_or_path), launcher::ItemType::Dir);
                (icon, "Folder".to_string())
            }
            launcher::ItemType::File => {
                let icon = icon_resolver.resolve_file_type_icon(Path::new(&item.exec_or_path), launcher::ItemType::File);
                let size_str = std::fs::metadata(&item.exec_or_path)
                    .map(|m| launcher::format_file_size(m.len()))
                    .unwrap_or_else(|_| "File".to_string());
                (icon, size_str)
            }
            launcher::ItemType::Calc => {
                (None, "Calc".to_string())
            }
            launcher::ItemType::Window => {
                let icon = icon_resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path);
                (icon, item.description.clone().unwrap_or_else(|| "Window".to_string()))
            }
            launcher::ItemType::Clipboard => {
                (None, item.description.clone().unwrap_or_else(|| "Clipboard".to_string()))
            }
            launcher::ItemType::System => {
                (None, "System".to_string())
            }
            launcher::ItemType::Dmenu => {
                (None, "".to_string())
            }
        };

        let has_icon = slint_icon.is_some();
        let icon_img = slint_icon.unwrap_or_default();

        let item_type_str = match item.item_type {
            launcher::ItemType::App => "app",
            launcher::ItemType::File => "file",
            launcher::ItemType::Dir => "dir",
            launcher::ItemType::Calc => "calc",
            launcher::ItemType::Window => "win",
            launcher::ItemType::Clipboard => "clip",
            launcher::ItemType::System => "sys",
            launcher::ItemType::Dmenu => "dmenu",
        };

        slint_items.push(LauncherItemData {
            name: item.name.clone().into(),
            category: category.into(),
            item_type: item_type_str.into(),
            has_icon,
            icon: icon_img,
            exec_or_path: item.exec_or_path.clone().into(),
        });

        current_items.push(item);
    }

    let model = std::rc::Rc::new(slint::VecModel::from(slint_items));
    ui.set_items(model.into());

    current_items
}

fn ensure_selection_visible(ui: &AppWindow, selected_index: i32) {
    let item_h: f32 = 51.0;
    let sel = selected_index as f32;
    let max_v = ui.get_cfg_max_results().clamp(4, 10) as f32;
    let cur_offset = -ui.get_scroll_viewport_y() / item_h;
    
    if sel < cur_offset {
        ui.set_scroll_viewport_y(-sel * item_h);
    } else if sel >= cur_offset + max_v {
        ui.set_scroll_viewport_y(-(sel - max_v + 1.0) * item_h);
    }
}

fn run_dmenu_mode(prompt: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let raw_lines: Vec<String> = stdin.lock().lines().filter_map(|l| l.ok()).collect();
    if raw_lines.is_empty() {
        return Ok(());
    }

    let ui = AppWindow::new()?;
    ui.set_mode_title(prompt.to_string().into());
    ui.set_mode_badge_text("dmenu".into());
    ui.set_is_file_mode(false);
    ui.set_cfg_show_icons(false);
    ui.set_cfg_show_status_bar(true);
    ui.set_cfg_max_results(8);

    let icon_resolver = IconResolver::new();
    ui.set_settings_icon(icon_resolver.get_gear_icon());
    ui.set_nav_icon(icon_resolver.get_nav_icon());
    ui.set_enter_icon(icon_resolver.get_enter_icon());
    ui.set_search_icon(icon_resolver.get_search_icon());
    ui.set_folder_icon(icon_resolver.get_folder_icon());

    ui.window().with_winit_window(|winit_window| {
        winit_window.set_transparent(true);
        winit_window.set_decorations(false);
        if let Some(monitor) = winit_window.current_monitor().or_else(|| winit_window.primary_monitor()) {
            let screen_size = monitor.size();
            let scale_factor = monitor.scale_factor();
            let window_width = (680.0 * scale_factor) as u32;
            let window_height = (600.0 * scale_factor) as u32;
            let pos_x = screen_size.width.saturating_sub(window_width) / 2;
            let pos_y = screen_size.height.saturating_sub(window_height) / 2;
            winit_window.set_outer_position(i_slint_backend_winit::winit::dpi::PhysicalPosition::new(pos_x as i32, pos_y as i32));
        }
    });

    let current_filtered = std::rc::Rc::new(std::cell::RefCell::new(raw_lines.clone()));

    let update_dmenu_view = {
        let ui_weak = ui.as_weak();
        let raw_lines = raw_lines.clone();
        let current_filtered = current_filtered.clone();
        move |query: &str| {
            let Some(ui) = ui_weak.upgrade() else { return; };
            let trimmed = query.trim();
            let filtered: Vec<String> = if trimmed.is_empty() {
                raw_lines.clone()
            } else {
                let norm_query = launcher::remove_vietnamese_accents(trimmed);
                let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
                use fuzzy_matcher::FuzzyMatcher;
                let mut matches: Vec<(String, i64)> = raw_lines
                    .iter()
                    .filter_map(|line| {
                        let norm_line = launcher::remove_vietnamese_accents(line);
                        let s1 = matcher.fuzzy_match(line, trimmed).unwrap_or(0);
                        let s2 = matcher.fuzzy_match(&norm_line, &norm_query).unwrap_or(0);
                        let max_s = s1.max(s2);
                        if max_s > 0 { Some((line.clone(), max_s)) } else { None }
                    })
                    .collect();
                matches.sort_by(|a, b| b.1.cmp(&a.1));
                matches.into_iter().map(|(l, _)| l).collect()
            };

            *current_filtered.borrow_mut() = filtered.clone();
            ui.set_has_results(!filtered.is_empty());
            ui.set_status_item_count(format!("{} items", filtered.len()).into());
            ui.set_selected_index(0);
            ui.set_scroll_viewport_y(0.0);

            let slint_items: Vec<LauncherItemData> = filtered
                .iter()
                .take(50)
                .map(|l| LauncherItemData {
                    name: l.clone().into(),
                    category: "".into(),
                    item_type: "dmenu".into(),
                    has_icon: false,
                    icon: slint::Image::default(),
                    exec_or_path: l.clone().into(),
                })
                .collect();
            ui.set_items(std::rc::Rc::new(slint::VecModel::from(slint_items)).into());
        }
    };

    update_dmenu_view("");

    {
        let update_view = update_dmenu_view.clone();
        ui.on_search_text_changed(move |text| {
            update_view(text.as_str());
        });
    }

    {
        let current_filtered = current_filtered.clone();
        ui.on_item_activated(move |idx| {
            let list = current_filtered.borrow();
            if let Some(item) = list.get(idx as usize) {
                println!("{}", item);
                std::process::exit(0);
            }
        });
    }

    {
        let ui_weak = ui.as_weak();
        let current_filtered = current_filtered.clone();
        ui.on_move_up(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let cur = ui.get_selected_index();
                if cur > 0 {
                    let next = cur - 1;
                    ui.set_selected_index(next);
                    ensure_selection_visible(&ui, next);
                }
            }
        });
        let ui_weak = ui.as_weak();
        ui.on_move_down(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let total = current_filtered.borrow().len() as i32;
                let cur = ui.get_selected_index();
                if cur + 1 < total {
                    let next = cur + 1;
                    ui.set_selected_index(next);
                    ensure_selection_visible(&ui, next);
                }
            }
        });
    }

    ui.on_close_window(|| {
        std::process::exit(1);
    });

    ui.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let is_dmenu = args.iter().any(|a| a == "--dmenu" || a == "-d");
    if is_dmenu {
        let mut prompt = "Select:".to_string();
        let mut i = 1;
        while i < args.len() {
            if (args[i] == "-p" || args[i] == "--prompt") && i + 1 < args.len() {
                prompt = args[i + 1].clone();
                i += 1;
            }
            i += 1;
        }
        return run_dmenu_mode(&prompt);
    }

    let exit_trigger = Arc::new(AtomicBool::new(false));

    // 1. Create main Slint Window
    let ui = AppWindow::new()?;

    // 2. Single-Instance Check
    if !handle_single_instance(exit_trigger.clone(), ui.as_weak()) {
        return Ok(());
    }

    // Setup global shortcut on desktop environments (GNOME, etc.)
    view_launcher::config::setup_global_shortcut();

    // 3. Load Config & Initialize Engine
    let config = Config::load();
    ui.set_cfg_autostart(view_launcher::config::is_autostart_enabled());
    ui.set_cfg_show_icons(config.theme.show_icons.unwrap_or(true));
    ui.set_cfg_show_status_bar(config.theme.show_status_bar.unwrap_or(true));
    ui.set_cfg_compact_empty(config.theme.compact_empty_view.unwrap_or(true));
    ui.set_cfg_enable_path_matching(config.search.enable_path_matching.unwrap_or(true));
    ui.set_cfg_max_results(config.search.max_results as i32);
    ui.set_cfg_max_depth(config.search.max_depth as i32);

    let engine = Arc::new(LauncherEngine::new(config));
    let icon_resolver = Arc::new(IconResolver::new());

    // Set UI action icons
    ui.set_settings_icon(icon_resolver.get_gear_icon());
    ui.set_nav_icon(icon_resolver.get_nav_icon());
    ui.set_enter_icon(icon_resolver.get_enter_icon());
    ui.set_search_icon(icon_resolver.get_search_icon());
    ui.set_folder_icon(icon_resolver.get_folder_icon());

    // Window properties & Icon
    let app_icon_opt = image::load_from_memory(include_bytes!("../assets/view-launcher.png"))
        .ok()
        .and_then(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            i_slint_backend_winit::winit::window::Icon::from_rgba(rgba.into_raw(), width, height).ok()
        });

    ui.window().with_winit_window(move |winit_window| {
        winit_window.set_title("View Launcher");
        winit_window.set_transparent(true);
        winit_window.set_decorations(false);
        if let Some(icon) = app_icon_opt {
            winit_window.set_window_icon(Some(icon));
        }
    });

    let center_window = |ui: &AppWindow| {
        ui.window().with_winit_window(|winit_window| {
            let monitor_opt = winit_window
                .current_monitor()
                .or_else(|| winit_window.primary_monitor())
                .or_else(|| winit_window.available_monitors().next());

            if let Some(monitor) = monitor_opt {
                let screen_size = monitor.size();
                let scale_factor = monitor.scale_factor();
                let window_width = (680.0 * scale_factor) as u32;
                let window_height = (600.0 * scale_factor) as u32;
                let pos_x = screen_size.width.saturating_sub(window_width) / 2;
                let pos_y = screen_size.height.saturating_sub(window_height) / 2;
                winit_window.set_outer_position(i_slint_backend_winit::winit::dpi::PhysicalPosition::new(pos_x as i32, pos_y as i32));
            }
        });
    };

    center_window(&ui);

    // Re-center when Wayland surface configure event resolves
    let center_timer = slint::Timer::default();
    {
        let ui_weak = ui.as_weak();
        center_timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(60),
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    center_window(&ui);
                }
            },
        );
    }

    // 4. Initial Population
    let current_results = Arc::new(std::sync::RwLock::new(populate_items(
        &ui,
        &engine,
        &icon_resolver,
        "",
    )));

    // Preload icons on startup (takes <15ms with O(1) index)
    icon_resolver.preload_icons(&engine.apps);

    // 5. Connect Search Text Changed
    {
        let engine = engine.clone();
        let icon_resolver = icon_resolver.clone();
        let current_results = current_results.clone();
        let ui_weak = ui.as_weak();

        ui.on_search_text_changed(move |text| {
            if let Some(ui) = ui_weak.upgrade() {
                let items = populate_items(&ui, &engine, &icon_resolver, &text);
                if let Ok(mut lock) = current_results.write() {
                    *lock = items;
                }
                ui.set_selected_index(0);
                ui.set_scroll_viewport_y(0.0);
            }
        });
    }

    // 5.1 Connect Move Up with Auto-Scroll
    {
        let ui_weak = ui.as_weak();
        let current_results = current_results.clone();
        ui.on_move_up(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let total = if let Ok(lock) = current_results.read() {
                    lock.len() as i32
                } else {
                    0
                };
                if total == 0 { return; }
                let cur = ui.get_selected_index();
                let next = if cur <= 0 { total - 1 } else { cur - 1 };
                ui.set_selected_index(next);
                ensure_selection_visible(&ui, next);
            }
        });
    }

    // 5.2 Connect Move Down with Auto-Scroll
    {
        let ui_weak = ui.as_weak();
        let current_results = current_results.clone();
        ui.on_move_down(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let total = if let Ok(lock) = current_results.read() {
                    lock.len() as i32
                } else {
                    0
                };
                if total == 0 { return; }
                let cur = ui.get_selected_index();
                let next = if cur + 1 >= total { 0 } else { cur + 1 };
                ui.set_selected_index(next);
                ensure_selection_visible(&ui, next);
            }
        });
    }

    // 5.3 Connect Tab Pressed (Navigate Into Directory)
    {
        let engine = engine.clone();
        let icon_resolver = icon_resolver.clone();
        let current_results = current_results.clone();
        let ui_weak = ui.as_weak();

        ui.on_tab_pressed(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let search_text = ui.get_search_text().to_string();
                let is_file_mode = search_text.starts_with("@f") || search_text.starts_with("@file");
                let is_path_mode = search_text.starts_with('/') || search_text.starts_with('~');
                if is_file_mode || is_path_mode {
                    let sel_idx = ui.get_selected_index() as usize;
                    let item_opt = {
                        if let Ok(lock) = current_results.read() {
                            lock.get(sel_idx).cloned()
                        } else {
                            None
                        }
                    };

                    if let Some(item) = item_opt {
                        if item.item_type == launcher::ItemType::Dir {
                            let mut path = item.exec_or_path;
                            if let Some(home) = dirs::home_dir() {
                                let home_str = home.to_string_lossy().to_string();
                                if path.starts_with(&home_str) {
                                    path = format!("~{}", &path[home_str.len()..]);
                                }
                            }
                            if !path.ends_with('/') {
                                path.push('/');
                            }
                            let prefix = if search_text.starts_with("@file ") || search_text == "@file" {
                                "@file "
                            } else if search_text.starts_with("@f ") || search_text == "@f" {
                                "@f "
                            } else {
                                ""
                            };
                            let new_search_text = format!("{}{}", prefix, path);
                            ui.set_search_text(new_search_text.clone().into());
                            ui.invoke_move_cursor_to_end();
                            let items = populate_items(&ui, &engine, &icon_resolver, &new_search_text);
                            if let Ok(mut lock) = current_results.write() {
                                *lock = items;
                            }
                            ui.set_selected_index(0);
                            ui.set_scroll_viewport_y(0.0);
                        }
                    }
                }
            }
        });
    }

    // 5.4 Connect Handle Backspace (Step Out to Parent Directory)
    {
        let engine = engine.clone();
        let icon_resolver = icon_resolver.clone();
        let current_results = current_results.clone();
        let ui_weak = ui.as_weak();

        ui.on_handle_backspace(move || -> bool {
            if let Some(ui) = ui_weak.upgrade() {
                let search_text = ui.get_search_text().to_string();
                let is_file_mode = search_text.starts_with("@f ") || search_text.starts_with("@file ");
                let is_path_mode = search_text.starts_with('/') || search_text.starts_with('~');
                if (is_file_mode || is_path_mode) && search_text.ends_with('/') {
                    let prefix_len = if search_text.starts_with("@file ") {
                        6
                    } else if search_text.starts_with("@f ") {
                        3
                    } else {
                        0
                    };
                    let path_part = &search_text[prefix_len..];
                    let trimmed_path = path_part.trim_end_matches('/');
                    if let Some(pos) = trimmed_path.rfind('/') {
                        let parent_path = &trimmed_path[..=pos];
                        let new_search_text = format!("{}{}", &search_text[..prefix_len], parent_path);
                        ui.set_search_text(new_search_text.clone().into());
                        ui.invoke_move_cursor_to_end();
                        let items = populate_items(&ui, &engine, &icon_resolver, &new_search_text);
                        if let Ok(mut lock) = current_results.write() {
                            *lock = items;
                        }
                        ui.set_selected_index(0);
                        ui.set_scroll_viewport_y(0.0);
                        return true;
                    } else {
                        let new_search_text = format!("{}", &search_text[..prefix_len]);
                        ui.set_search_text(new_search_text.clone().into());
                        ui.invoke_move_cursor_to_end();
                        let items = populate_items(&ui, &engine, &icon_resolver, &new_search_text);
                        if let Ok(mut lock) = current_results.write() {
                            *lock = items;
                        }
                        ui.set_selected_index(0);
                        ui.set_scroll_viewport_y(0.0);
                        return true;
                    }
                }
            }
            false
        });
    }

    // 6. Connect Item Activated (Enter key)
    {
        let engine = engine.clone();
        let current_results = current_results.clone();
        let ui_weak = ui.as_weak();

        ui.on_item_activated(move |idx| {
            let idx = idx as usize;
            let item_opt = {
                if let Ok(lock) = current_results.read() {
                    lock.get(idx).cloned()
                } else {
                    None
                }
            };

            if let Some(item) = item_opt {
                if let Some(ui) = ui_weak.upgrade() {
                    let _ = ui.hide();
                }
                engine.launch(&item);
                std::process::exit(0);
            }
        });
    }

    // 7. Connect Open In Terminal (Alt+T)
    {
        let engine = engine.clone();
        let current_results = current_results.clone();
        let ui_weak = ui.as_weak();

        ui.on_open_in_terminal(move |idx| {
            let idx = idx as usize;
            let item_opt = {
                if let Ok(lock) = current_results.read() {
                    lock.get(idx).cloned()
                } else {
                    None
                }
            };

            if let Some(item) = item_opt {
                if let Some(ui) = ui_weak.upgrade() {
                    let _ = ui.hide();
                }
                engine.open_in_terminal(&item);
                std::process::exit(0);
            }
        });
    }

    // 8. Connect Copy Path (Alt+C)
    {
        let engine = engine.clone();
        let current_results = current_results.clone();

        ui.on_copy_path(move |idx| {
            let idx = idx as usize;
            let item_opt = {
                if let Ok(lock) = current_results.read() {
                    lock.get(idx).cloned()
                } else {
                    None
                }
            };

            if let Some(item) = item_opt {
                engine.copy_to_clipboard(&item.exec_or_path);
            }
        });
    }

    // 9. Connect Open Settings View (Ctrl+,)
    {
        let ui_weak = ui.as_weak();
        ui.on_open_config(move || {
            let start = std::time::Instant::now();
            if std::env::var("VIEW_LAUNCHER_DEBUG").is_ok() || std::env::var("RUST_LOG").is_ok() {
                eprintln!("[DEBUG] on_open_config ENTER at {:?}", start);
            }
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_cfg_autostart(view_launcher::config::is_autostart_enabled());
                ui.set_in_settings_mode(true);
                ui.set_settings_status("".into());
                ui.invoke_focus_settings();
            }
            if std::env::var("VIEW_LAUNCHER_DEBUG").is_ok() || std::env::var("RUST_LOG").is_ok() {
                eprintln!("[DEBUG] on_open_config EXIT in {:?}", start.elapsed());
            }
        });
    }

    // 9.1 Connect Close Settings View
    {
        let ui_weak = ui.as_weak();
        ui.on_close_settings(move || {
            let start = std::time::Instant::now();
            if std::env::var("VIEW_LAUNCHER_DEBUG").is_ok() || std::env::var("RUST_LOG").is_ok() {
                eprintln!("[DEBUG] on_close_settings ENTER at {:?}", start);
            }
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_in_settings_mode(false);
                ui.invoke_focus_search();
            }
            if std::env::var("VIEW_LAUNCHER_DEBUG").is_ok() || std::env::var("RUST_LOG").is_ok() {
                eprintln!("[DEBUG] on_close_settings EXIT in {:?}", start.elapsed());
            }
        });
    }

    // 9.2 Connect Save Settings
    {
        let ui_weak = ui.as_weak();
        let engine = engine.clone();
        let icon_resolver = icon_resolver.clone();
        let current_results = current_results.clone();
        ui.on_save_settings(move |show_icons, show_status_bar, enable_path, max_results, max_depth, autostart, compact_empty| {
            let mut cfg = Config::load();
            cfg.general.autostart = autostart;
            cfg.theme.show_icons = Some(show_icons);
            cfg.theme.show_status_bar = Some(show_status_bar);
            cfg.theme.compact_empty_view = Some(compact_empty);
            cfg.search.enable_path_matching = Some(enable_path);
            cfg.search.max_results = max_results as usize;
            cfg.search.max_depth = max_depth as usize;
            let _ = cfg.save();

            if let Some(ui) = ui_weak.upgrade() {
                ui.set_settings_status("✔ Configuration saved to ~/.config/view-launcher/config.toml".into());
                let items = populate_items(&ui, &engine, &icon_resolver, &ui.get_search_text());
                if let Ok(mut lock) = current_results.write() {
                    *lock = items;
                }
            }
        });
    }

    // 9.3 Connect Open Raw config.toml in Editor
    ui.on_open_raw_config(move || {
        open_config_file();
    });

    // 10. Connect Close Window (Esc)
    {
        let ui_weak = ui.as_weak();
        ui.on_close_window(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let _ = ui.hide();
            }
            std::process::exit(0);
        });
    }

    // 10.1 Native Window Dragging on Wayland / X11 / Windows
    {
        let ui_weak = ui.as_weak();
        ui.on_start_window_drag(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.window().with_winit_window(|winit_window| {
                    let _ = winit_window.drag_window();
                });
            }
        });
    }

    // 11. Run Slint Event Loop
    ui.run()?;

    // Cleanup socket on exit
    #[cfg(unix)]
    {
        let socket_path = get_socket_path();
        let _ = std::fs::remove_file(&socket_path);
    }

    Ok(())
}
