use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub count: u32,
    pub last_used: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct HistoryData {
    #[serde(default)]
    pub entries: HashMap<String, HistoryEntry>,
    /// Raw shell commands run via the "!" launcher mode, oldest first, capped at
    /// `SHELL_HISTORY_CAP` entries. Separate from `entries` because it's an ordered
    /// recall log (like a real shell's history), not a ranking-boost table.
    #[serde(default)]
    pub shell_commands: Vec<String>,
}

const SHELL_HISTORY_CAP: usize = 200;

#[derive(Debug)]
pub struct HistoryManager {
    data: HistoryData,
    cache_path: Option<PathBuf>,
}

impl HistoryManager {
    /// An in-memory-only manager with no backing file (`save()` becomes a no-op since
    /// it already guards on `cache_path.is_some()`). Used by tests so they don't write
    /// throwaway data into the real `~/.cache/view-launcher/history.toml`.
    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self { data: HistoryData::default(), cache_path: None }
    }

    pub fn load() -> Self {
        let cache_path = Self::get_cache_path();
        let mut data = HistoryData::default();

        if let Some(ref path) = cache_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(parsed) = toml::from_str::<HistoryData>(&content) {
                        data = parsed;
                    }
                }
            }
        }

        Self {
            data,
            cache_path,
        }
    }

    fn get_cache_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir().map(|mut p| {
                p.push("view-launcher");
                p.push("history.toml");
                p
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::cache_dir().map(|mut p| {
                p.push("view-launcher");
                p.push("history.toml");
                p
            })
        }
    }

    /// Appends a command run via the "!" shell mode to the recall history (skipping
    /// immediate repeats, like most shells' `HISTCONTROL=ignoredups`), trimming the
    /// oldest entries once `SHELL_HISTORY_CAP` is exceeded.
    pub fn record_shell_command(&mut self, command: &str) {
        if command.trim().is_empty() {
            return;
        }
        if self.data.shell_commands.last().map(|s| s.as_str()) == Some(command) {
            return;
        }
        self.data.shell_commands.push(command.to_string());
        let len = self.data.shell_commands.len();
        if len > SHELL_HISTORY_CAP {
            self.data.shell_commands.drain(0..len - SHELL_HISTORY_CAP);
        }
        self.save();
    }

    /// Oldest-first list of past "!" shell commands, for Up/Down recall.
    pub fn shell_commands(&self) -> &[String] {
        &self.data.shell_commands
    }

    pub fn record_launch(&mut self, key: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = self.data.entries.entry(key.to_string()).or_insert(HistoryEntry {
            count: 0,
            last_used: now,
        });

        entry.count = entry.count.saturating_add(1);
        entry.last_used = now;

        self.save();
    }

    pub fn get_boost(&self, key: &str) -> i64 {
        if let Some(entry) = self.data.entries.get(key) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let age_secs = now.saturating_sub(entry.last_used);
            
            // Recency boost
            let recency_boost = if age_secs < 3600 {
                60 // Within last hour
            } else if age_secs < 86400 {
                40 // Within last 24 hours
            } else if age_secs < 86400 * 7 {
                20 // Within last week
            } else {
                5
            };

            // Frequency boost (capped at 100)
            let freq_boost = (entry.count as i64 * 10).min(100);

            recency_boost + freq_boost
        } else {
            0
        }
    }

    fn save(&self) {
        if let Some(ref path) = self.cache_path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(content) = toml::to_string_pretty(&self.data) {
                let _ = fs::write(path, content);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_command_history_order_and_dedup() {
        let mut h = HistoryManager::in_memory();
        assert!(h.shell_commands().is_empty());

        h.record_shell_command("ls -la");
        h.record_shell_command("echo hi");
        // Immediate repeat of the last command is ignored (like ignoredups).
        h.record_shell_command("echo hi");
        h.record_shell_command("git status");

        assert_eq!(h.shell_commands(), &["ls -la", "echo hi", "git status"]);
    }

    #[test]
    fn test_shell_command_history_ignores_blank() {
        let mut h = HistoryManager::in_memory();
        h.record_shell_command("");
        h.record_shell_command("   ");
        assert!(h.shell_commands().is_empty());
    }

    #[test]
    fn test_shell_command_history_caps_length() {
        let mut h = HistoryManager::in_memory();
        for i in 0..(SHELL_HISTORY_CAP + 10) {
            h.record_shell_command(&format!("cmd{i}"));
        }
        assert_eq!(h.shell_commands().len(), SHELL_HISTORY_CAP);
        // Oldest entries were trimmed; the most recent one is still there.
        assert_eq!(h.shell_commands().last().unwrap(), &format!("cmd{}", SHELL_HISTORY_CAP + 9));
        assert!(!h.shell_commands().contains(&"cmd0".to_string()));
    }
}
