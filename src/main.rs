use std::path::PathBuf;
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
use std::net::{TcpListener, TcpStream};
#[cfg(windows)]
use std::io::Write;

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
                                    let _ = ui.show();
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
fn handle_single_instance(_exit_trigger: Arc<AtomicBool>, _ui_handle: slint::Weak<AppWindow>) -> bool {
    // Port 42425 on localhost for Windows single instance
    if let Ok(mut stream) = TcpStream::connect("127.0.0.1:42425") {
        let _ = stream.write_all(b"toggle");
        return false;
    }

    if let Ok(listener) = TcpListener::bind("127.0.0.1:42425") {
        thread::spawn(move || {
            for _ in listener.incoming() {
                // Wake up window
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
    let results = engine.search(query);
    let count = results.len();

    ui.set_is_file_mode(is_file_mode);
    ui.set_has_results(count > 0);
    if is_file_mode {
        ui.set_mode_icon("󰉋".into());
        ui.set_mode_title("FILE SEARCH".into());
        ui.set_status_item_count(format!("{} files", count).into());
    } else {
        ui.set_mode_icon("󰀻".into());
        ui.set_mode_title("VIEW LAUNCHER".into());
        ui.set_status_item_count(format!("{} apps", count).into());
    }

    let mut current_items = Vec::new();
    let mut slint_items = Vec::new();

    let limit = (ui.get_cfg_max_results() as usize).clamp(4, 10);
    for (item, _indices) in results.into_iter().take(limit) {
        let slint_icon = if item.item_type == launcher::ItemType::App {
            icon_resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path)
        } else {
            None
        };
        let has_icon = slint_icon.is_some();
        let icon_img = slint_icon.unwrap_or_default();
        let category = item.get_category_tag().to_string();

        let item_type_str = match item.item_type {
            launcher::ItemType::App => "app",
            launcher::ItemType::File => "file",
            launcher::ItemType::Dir => "dir",
            launcher::ItemType::Calc => "calc",
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exit_trigger = Arc::new(AtomicBool::new(false));

    // 1. Create main Slint Window
    let ui = AppWindow::new()?;

    // 2. Single-Instance Check
    if !handle_single_instance(exit_trigger.clone(), ui.as_weak()) {
        return Ok(());
    }

    // 3. Load Config & Initialize Engine
    let config = Config::load();
    ui.set_cfg_show_icons(config.theme.show_icons.unwrap_or(true));
    ui.set_cfg_show_status_bar(config.theme.show_status_bar.unwrap_or(true));
    ui.set_cfg_enable_path_matching(config.search.enable_path_matching.unwrap_or(true));
    ui.set_cfg_max_results(config.search.max_results as i32);
    ui.set_cfg_max_depth(config.search.max_depth as i32);

    let engine = Arc::new(LauncherEngine::new(config));
    let icon_resolver = Arc::new(IconResolver::new());

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
            }
        });
    }

    // 5.1 Connect Move Up
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
            }
        });
    }

    // 5.2 Connect Move Down
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
            }
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
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_in_settings_mode(true);
                ui.set_settings_status("".into());
            }
        });
    }

    // 9.1 Connect Close Settings View
    {
        let ui_weak = ui.as_weak();
        ui.on_close_settings(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_in_settings_mode(false);
            }
        });
    }

    // 9.2 Connect Save Settings
    {
        let ui_weak = ui.as_weak();
        let engine = engine.clone();
        let icon_resolver = icon_resolver.clone();
        let current_results = current_results.clone();
        ui.on_save_settings(move |show_icons, show_status_bar, enable_path, max_results, max_depth| {
            let mut cfg = Config::load();
            cfg.theme.show_icons = Some(show_icons);
            cfg.theme.show_status_bar = Some(show_status_bar);
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
