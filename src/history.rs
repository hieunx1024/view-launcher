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

#[derive(Debug)]
pub struct HistoryManager {
    entries: HashMap<String, HistoryEntry>,
    cache_path: Option<PathBuf>,
}

impl HistoryManager {
    pub fn load() -> Self {
        let cache_path = Self::get_cache_path();
        let mut entries = HashMap::new();

        if let Some(ref path) = cache_path {
            if path.exists() {
                if let Ok(data) = fs::read_to_string(path) {
                    if let Ok(parsed) = serde_json::from_str::<HashMap<String, HistoryEntry>>(&data) {
                        entries = parsed;
                    }
                }
            }
        }

        Self {
            entries,
            cache_path,
        }
    }

    fn get_cache_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir().map(|mut p| {
                p.push("view-launcher");
                p.push("history.json");
                p
            })
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::cache_dir().map(|mut p| {
                p.push("view-launcher");
                p.push("history.json");
                p
            })
        }
    }

    pub fn record_launch(&mut self, key: &str) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let entry = self.entries.entry(key.to_string()).or_insert(HistoryEntry {
            count: 0,
            last_used: now,
        });

        entry.count = entry.count.saturating_add(1);
        entry.last_used = now;

        self.save();
    }

    pub fn get_boost(&self, key: &str) -> i64 {
        if let Some(entry) = self.entries.get(key) {
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
            if let Ok(json) = serde_json::to_string_pretty(&self.entries) {
                let _ = fs::write(path, json);
            }
        }
    }
}
