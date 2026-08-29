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

pub fn execute_system_action(command: &str) {
    #[cfg(unix)]
    {
        let tokens: Vec<&str> = command.split_whitespace().collect();
        if !tokens.is_empty() {
            let _ = std::process::Command::new(tokens[0])
                .args(&tokens[1..])
                .spawn();
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
