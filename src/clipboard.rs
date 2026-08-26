use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use arboard::Clipboard;

#[derive(Clone)]
pub struct ClipboardManager {
    history: Arc<RwLock<VecDeque<String>>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let manager = Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
        };
        manager.start_monitoring();
        manager
    }

    fn start_monitoring(&self) {
        let history = self.history.clone();
        thread::spawn(move || {
            let mut last_text = String::new();
            loop {
                thread::sleep(Duration::from_millis(600));
                if let Ok(mut cb) = Clipboard::new() {
                    if let Ok(text) = cb.get_text() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && trimmed != last_text {
                            last_text = trimmed.to_string();
                            if let Ok(mut lock) = history.write() {
                                lock.retain(|item| item != &last_text);
                                lock.push_front(last_text.clone());
                                if lock.len() > 100 {
                                    lock.pop_back();
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn get_history(&self) -> Vec<String> {
        self.history.read().map(|h| h.iter().cloned().collect()).unwrap_or_default()
    }

    pub fn copy_to_clipboard(&self, text: &str) {
        if let Ok(mut cb) = Clipboard::new() {
            let _ = cb.set_text(text);
        }
    }
}
