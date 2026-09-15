use crate::launcher::{LauncherItem, ItemType};

pub fn get_system_actions() -> Vec<LauncherItem> {
    vec![
        LauncherItem::new(
            "Lock Screen".to_string(),
            "loginctl lock-session".to_string(),
            ItemType::System,
            Some("Lock the current session".to_string()),
            false,
            None,
        ),
        LauncherItem::new(
            "Restart / Reboot".to_string(),
            "systemctl reboot".to_string(),
            ItemType::System,
            Some("Reboot the operating system".to_string()),
            false,
            None,
        ),
        LauncherItem::new(
            "Shut Down / Power Off".to_string(),
            "systemctl poweroff".to_string(),
            ItemType::System,
            Some("Turn off the computer".to_string()),
            false,
            None,
        ),
        LauncherItem::new(
            "Suspend / Sleep".to_string(),
            "systemctl suspend".to_string(),
            ItemType::System,
            Some("Put computer into low-power sleep state".to_string()),
            false,
            None,
        ),
        LauncherItem::new(
            "Log Out".to_string(),
            "loginctl terminate-session self".to_string(),
            ItemType::System,
            Some("End the current user session".to_string()),
            false,
            None,
        ),
    ]
}

/// Resolves the current graphical login session's id, for `loginctl` commands that
/// need a session to act on. `loginctl ...  self` normally resolves "self" via the
/// calling process's own systemd cgroup, but that lookup fails with "Caller does not
/// belong to any known session" for a process not cleanly placed in a session scope -
/// which is exactly what happens to this app's daemon when it's started via a global
/// hotkey / desktop custom-keybinding rather than from an interactive shell inside
/// the session. Looking the session up explicitly sidesteps that.
#[cfg(unix)]
fn current_session_id() -> Option<String> {
    if let Ok(id) = std::env::var("XDG_SESSION_ID") {
        let id = id.trim();
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    let username = std::env::var("USER").ok()?;
    let output = std::process::Command::new("loginctl")
        .args(["show-user", &username, "-p", "Display", "--value"])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if id.is_empty() { None } else { Some(id) }
}

pub fn execute_system_action(command: &str) {
    #[cfg(unix)]
    {
        let resolved;
        let command = if command == "loginctl lock-session" {
            match current_session_id() {
                Some(id) => {
                    resolved = format!("loginctl lock-session {id}");
                    resolved.as_str()
                }
                None => command,
            }
        } else if command == "loginctl terminate-session self" {
            match current_session_id() {
                Some(id) => {
                    resolved = format!("loginctl terminate-session {id}");
                    resolved.as_str()
                }
                None => command,
            }
        } else {
            command
        };

        let tokens: Vec<&str> = command.split_whitespace().collect();
        if !tokens.is_empty() {
            match std::process::Command::new(tokens[0]).args(&tokens[1..]).spawn() {
                Ok(_) => eprintln!("[view-launcher] system action spawned: {command}"),
                Err(e) => eprintln!("[view-launcher] system action FAILED to spawn: {command}: {e}"),
            }
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let win_cmd = match command {
            "loginctl lock-session" => "rundll32.exe user32.dll,LockWorkStation",
            "systemctl reboot" => "shutdown /r /t 0",
            "systemctl poweroff" => "shutdown /s /t 0",
            "systemctl suspend" => "rundll32.exe powrprof.dll,SetSuspendState",
            "loginctl terminate-session self" => "shutdown /l",
            _ => command,
        };
        let mut cmd = std::process::Command::new("cmd");
        cmd.args(&["/C", win_cmd]);
        cmd.creation_flags(0x08000000);
        let _ = cmd.spawn();
    }
}
