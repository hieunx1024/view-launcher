use std::process::Command;
use crate::launcher::LauncherItem;

#[cfg(target_os = "linux")]
use std::collections::HashSet;
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
unsafe extern "C" {
    fn setsid() -> i32;
}

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt as WindowsCommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone)]
pub struct WindowItem {
    pub id: String,
    pub title: String,
    pub class_name: String,
}

pub fn get_open_windows(_known_apps: &[LauncherItem]) -> Vec<WindowItem> {
    let mut windows = Vec::new();

    // 1. Try Hyprland (Wayland)
    #[cfg(unix)]
    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        if let Ok(output) = Command::new("hyprctl").args(&["clients", "-j"]).output() {
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            let title = item["title"].as_str().unwrap_or("").to_string();
                            let class_name = item["class"].as_str().unwrap_or("").to_string();
                            let address = item["address"].as_str().unwrap_or("").to_string();
                            if !title.is_empty() && !address.is_empty() {
                                windows.push(WindowItem {
                                    id: format!("hypr:{}", address),
                                    title,
                                    class_name,
                                });
                            }
                        }
                        if !windows.is_empty() {
                            return windows;
                        }
                    }
                }
            }
        }
    }

    // 2. Try Sway (Wayland)
    #[cfg(unix)]
    if std::env::var("SWAYSOCK").is_ok() {
        if let Ok(output) = Command::new("swaymsg").args(&["-t", "get_tree"]).output() {
            if let Ok(json_str) = String::from_utf8(output.stdout) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    collect_sway_windows(&val, &mut windows);
                    if !windows.is_empty() {
                        return windows;
                    }
                }
            }
        }
    }

    // 3. Try wmctrl (X11 & XWayland)
    #[cfg(unix)]
    if let Ok(output) = Command::new("wmctrl").args(&["-l", "-x"]).output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let id = parts[0].to_string();
                let class_name = parts[2].to_string();
                let title = parts[4..].join(" ");
                if !title.is_empty() && !title.eq_ignore_ascii_case("Desktop") && !class_name.contains("gnome-shell") {
                    windows.push(WindowItem {
                        id: format!("wmctrl:{}", id),
                        title,
                        class_name,
                    });
                }
            }
        }
        if !windows.is_empty() {
            return windows;
        }
    }

    // 4. Linux / GNOME Wayland fallback: Scan running GUI processes & match against known apps
    #[cfg(target_os = "linux")]
    {
        let running_bins = get_user_running_binaries();
        let mut seen = HashSet::new();

        for app in _known_apps {
            let tokens = crate::launcher::parse_command_line(&app.exec_or_path);
            let bin_target = tokens.iter()
                .find(|t| *t != "env" && !t.contains('='))
                .or_else(|| tokens.first());
            let bin_name = bin_target
                .and_then(|p| Path::new(p).file_name())
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if bin_name.is_empty() || bin_name == "view-launcher" {
                continue;
            }

            // Exclude common background daemons
            if is_background_daemon(&bin_name) || is_background_daemon(&app.name.to_lowercase()) {
                continue;
            }

            let matches_binary = running_bins.contains(&bin_name)
                || running_bins.iter().any(|r| (r.len() >= 4 && bin_name.contains(r)) || (bin_name.len() >= 4 && r.contains(&bin_name)));

            if matches_binary {
                if seen.insert(app.name.clone()) {
                    windows.push(WindowItem {
                        id: format!("app:{}", app.exec_or_path),
                        title: app.name.clone(),
                        class_name: "Running".to_string(),
                    });
                }
            }
        }
    }

    // 5. Windows 10/11 native active window switcher (<1ms via Win32 EnumWindows)
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextW, GetWindowTextLengthW, GetWindowThreadProcessId,
            IsWindowVisible, GetWindowLongW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
        };

        unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                let list = &mut *(lparam as *mut Vec<WindowItem>);

                if IsWindowVisible(hwnd) == 0 {
                    return 1;
                }

                // Filter out tool windows or overlay utility popups
                let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
                if (ex_style & WS_EX_TOOLWINDOW) != 0 {
                    return 1;
                }

                let len = GetWindowTextLengthW(hwnd);
                if len > 0 {
                    let mut title_buf = vec![0u16; (len + 1) as usize];
                    let read_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), len + 1);
                    if read_len > 0 {
                        let title = String::from_utf16_lossy(&title_buf[..read_len as usize]).trim().to_string();
                        if !title.is_empty() && title != "View Launcher" {
                            let mut pid: u32 = 0;
                            GetWindowThreadProcessId(hwnd, &mut pid);
                            list.push(WindowItem {
                                id: format!("winhwnd:{}", hwnd as isize),
                                title,
                                class_name: format!("PID: {}", pid),
                            });
                        }
                    }
                }
                1
            }
        }

        unsafe {
            let mut win_list: Vec<WindowItem> = Vec::new();
            EnumWindows(Some(enum_windows_callback), &mut win_list as *mut _ as LPARAM);
            windows.extend(win_list);
        }
    }

    windows
}

#[cfg(target_os = "linux")]
fn is_background_daemon(name: &str) -> bool {
    const DAEMONS: &[&str] = &[
        "systemd", "dbus", "ibus", "tracker", "portal", "pipewire", "wireplumber",
        "pulseaudio", "gnome-shell", "gnome-session", "gsd-", "evolution-alarm",
        "gcr-ssh", "snap", "view-launcher", "python", "sh", "bash", "xdg-",
        "at-spi", "dconf", "gvfs", "agent", "daemon", "service", "appindicators"
    ];
    DAEMONS.iter().any(|&d| name.contains(d))
}

#[cfg(target_os = "linux")]
fn get_user_running_binaries() -> HashSet<String> {
    let mut bins = HashSet::new();
    let Ok(entries) = fs::read_dir("/proc") else { return bins; };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.chars().all(|c| c.is_ascii_digit()) {
            let pid_path = entry.path();
            if let Ok(comm) = fs::read_to_string(pid_path.join("comm")) {
                let trimmed = comm.trim().to_lowercase();
                if !trimmed.is_empty() {
                    bins.insert(trimmed);
                }
            }
            if let Ok(cmdline) = fs::read(pid_path.join("cmdline")) {
                let text = String::from_utf8_lossy(&cmdline);
                for token in text.split('\0') {
                    if let Some(file) = Path::new(token).file_name() {
                        let file_str = file.to_string_lossy().to_lowercase();
                        if !file_str.is_empty() {
                            bins.insert(file_str);
                        }
                    }
                }
            }
        }
    }
    bins
}

#[allow(dead_code)]
fn collect_sway_windows(node: &serde_json::Value, list: &mut Vec<WindowItem>) {
    if let Some(nodes) = node["nodes"].as_array() {
        for n in nodes {
            collect_sway_windows(n, list);
        }
    }
    if let Some(fnodes) = node["floating_nodes"].as_array() {
        for n in fnodes {
            collect_sway_windows(n, list);
        }
    }
    if let Some(name) = node["name"].as_str() {
        if let Some(id) = node["id"].as_i64() {
            let app_id = node["app_id"].as_str().unwrap_or(node["window_properties"]["class"].as_str().unwrap_or(""));
            if !name.is_empty() && (node["type"].as_str() == Some("con") || node["type"].as_str() == Some("floating_con")) {
                list.push(WindowItem {
                    id: format!("sway:{}", id),
                    title: name.to_string(),
                    class_name: app_id.to_string(),
                });
            }
        }
    }
}

pub fn focus_window(id: &str) {
    if let Some(addr) = id.strip_prefix("hypr:") {
        let _ = Command::new("hyprctl")
            .args(&["dispatch", "focuswindow", &format!("address:{}", addr)])
            .spawn();
        return;
    }

    if let Some(con_id) = id.strip_prefix("sway:") {
        let _ = Command::new("swaymsg")
            .args(&[&format!("[con_id={}] focus", con_id)])
            .spawn();
        return;
    }

    if let Some(win_id) = id.strip_prefix("wmctrl:") {
        if let Ok(_) = Command::new("wmctrl").args(&["-i", "-a", win_id]).spawn() {
            return;
        }
        let _ = Command::new("xdotool").args(&["windowactivate", win_id]).spawn();
        return;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(hwnd_str) = id.strip_prefix("winhwnd:") {
            if let Ok(hwnd_val) = hwnd_str.parse::<isize>() {
                use windows_sys::Win32::Foundation::HWND;
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    SetForegroundWindow, ShowWindow, IsIconic, BringWindowToTop, SW_RESTORE,
                };
                let hwnd = hwnd_val as HWND;
                unsafe {
                    if IsIconic(hwnd) != 0 {
                        ShowWindow(hwnd, SW_RESTORE);
                    }
                    BringWindowToTop(hwnd);
                    SetForegroundWindow(hwnd);
                }
                return;
            }
        }

        if let Some(pid) = id.strip_prefix("winpid:") {
            let script = format!("$w = (New-Object -ComObject WScript.Shell); $w.AppActivate({})", pid);
            let _ = Command::new("powershell")
                .args(&["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            return;
        }

        if let Some(exec) = id.strip_prefix("app:") {
            let _ = Command::new("cmd")
                .args(&["/C", "start", "", exec])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
            return;
        }
    }

    #[cfg(unix)]
    if let Some(exec) = id.strip_prefix("app:") {
        let tokens = crate::launcher::parse_command_line(exec);
        if !tokens.is_empty() {
            let mut cmd = Command::new(&tokens[0]);
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
}
