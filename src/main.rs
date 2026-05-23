mod config;
mod launcher;
mod ui;

use std::io::{stdout, Write};
use std::time::Duration;
use std::thread;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use config::{Config, parse_color};
use launcher::{LauncherEngine, LauncherItem};
use ui::UiState;

const LOCAL_PORT: u16 = 19428;

fn handle_single_instance(exit_trigger: Arc<AtomicBool>) -> bool {
    // Check if another instance is already listening on our local port
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", LOCAL_PORT)) {
        // Send toggle command to the running instance
        let _ = stream.write_all(b"toggle");
        return false;
    }
    
    // Bind to the local port
    if let Ok(listener) = TcpListener::bind(("127.0.0.1", LOCAL_PORT)) {
        thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_ok() {
                    exit_trigger.store(true, Ordering::SeqCst);
                    break;
                }
            }
        });
    }
    
    true
}

struct App {
    input: String,
    results: Vec<LauncherItem>,
    selected_index: usize,
    engine: LauncherEngine,
    config: Config,
    should_quit: bool,
}

impl App {
    fn new(config: Config) -> Self {
        let engine = LauncherEngine::new(config.clone());
        let results = engine.search(""); // Initially show all/default
        Self {
            input: String::new(),
            results,
            selected_index: 0,
            engine,
            config,
            should_quit: false,
        }
    }

    fn update_search(&mut self) {
        self.results = self.engine.search(&self.input);
        self.selected_index = 0; // Always auto-focus on the top/most relevant match!
    }
}

#[cfg(target_os = "windows")]
fn auto_bootstrap_windows() {
    use std::os::windows::process::CommandExt;
    if let Ok(app_path) = std::env::current_exe() {
        if let Some(app_str) = app_path.to_str() {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let shortcut_path = format!(
                    r"{}\Microsoft\Windows\Start Menu\Programs\Startup\ViewLauncher.lnk",
                    appdata
                );

                if !std::path::Path::new(&shortcut_path).exists() {
                    let ps_script = format!(
                        "$WshShell = New-Object -ComObject WScript.Shell; \
                         $Shortcut = $WshShell.CreateShortcut('{}'); \
                         $Shortcut.TargetPath = 'wt.exe'; \
                         $Shortcut.Arguments = '-d . \"{}\"'; \
                         $Shortcut.Hotkey = 'Ctrl+Alt+Space'; \
                         $Shortcut.Save()",
                        shortcut_path, app_str
                    );

                    let _ = std::process::Command::new("powershell")
                        .args(&["-Command", &ps_script])
                        .creation_flags(0x08000000) // CREATE_NO_WINDOW
                        .status();
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    auto_bootstrap_windows();

    // 1. Setup panic hook to ensure terminal is restored if app crashes
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // 2. Single Instance Lock check
    let exit_trigger = Arc::new(AtomicBool::new(false));
    if !handle_single_instance(exit_trigger.clone()) {
        return Ok(());
    }

    // 3. Load Configuration and Setup Engine
    let config = Config::load();
    let mut app = App::new(config);

    // 3. Initialize Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Clear and draw initial frame
    terminal.clear()?;

    // 4. Main Event Loop
    while !app.should_quit {
        if exit_trigger.load(Ordering::SeqCst) {
            app.should_quit = true;
            break;
        }

        // Auto-refresh once background file scanning completes
        let has_files = app.results.iter().any(|i| matches!(i.item_type, launcher::ItemType::File | launcher::ItemType::Dir));
        let scan_completed = app.engine.shallow_files.read().map(|f| !f.is_empty()).unwrap_or(false);
        if !has_files && scan_completed && app.input.is_empty() {
            app.update_search();
        }

        terminal.draw(|f| {
            let state = UiState {
                input: &app.input,
                results: &app.results,
                selected_index: app.selected_index,
                query_color: parse_color(&app.config.theme.query_color),
                selection_bg: parse_color(&app.config.theme.selection_bg),
                selection_fg: parse_color(&app.config.theme.selection_fg),
                app_badge_color: parse_color(&app.config.theme.app_badge_color),
                file_badge_color: parse_color(&app.config.theme.file_badge_color),
                border_color: parse_color(&app.config.theme.border_color),
            };
            ui::draw(f, &state);
        })?;

        // Process inputs
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Ignore key releases to prevent double actions
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }

                // Check modifiers (like Ctrl+C, Ctrl+N, Ctrl+P)
                let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

                match key.code {
                    KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if has_ctrl => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('n') if has_ctrl => {
                        // Ctrl+N = Down
                        if !app.results.is_empty() {
                            app.selected_index = (app.selected_index + 1) % app.results.len();
                        }
                    }
                    KeyCode::Char('p') if has_ctrl => {
                        // Ctrl+P = Up
                        if !app.results.is_empty() {
                            if app.selected_index == 0 {
                                app.selected_index = app.results.len() - 1;
                            } else {
                                app.selected_index -= 1;
                            }
                        }
                    }
                    KeyCode::Down => {
                        if !app.results.is_empty() {
                            app.selected_index = (app.selected_index + 1) % app.results.len();
                        }
                    }
                    KeyCode::Up => {
                        if !app.results.is_empty() {
                            if app.selected_index == 0 {
                                app.selected_index = app.results.len() - 1;
                            } else {
                                app.selected_index -= 1;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                        app.update_search();
                    }
                    KeyCode::Tab => {
                        if !app.results.is_empty() {
                            let selected_item = &app.results[app.selected_index];
                            match selected_item.item_type {
                                launcher::ItemType::Dir => {
                                    let mut path = selected_item.exec_or_path.clone();
                                    if let Some(home) = dirs::home_dir() {
                                        let home_str = home.to_string_lossy().to_string();
                                        if path.starts_with(&home_str) {
                                            path = path.replacen(&home_str, "~", 1);
                                        }
                                    }
                                    if !path.ends_with('/') {
                                        path.push('/');
                                    }
                                    app.input = path;
                                    app.update_search();
                                }
                                launcher::ItemType::File => {
                                    let mut path = selected_item.exec_or_path.clone();
                                    if let Some(home) = dirs::home_dir() {
                                        let home_str = home.to_string_lossy().to_string();
                                        if path.starts_with(&home_str) {
                                            path = path.replacen(&home_str, "~", 1);
                                        }
                                    }
                                    app.input = path;
                                    app.update_search();
                                }
                                launcher::ItemType::App => {
                                    app.input = selected_item.name.clone();
                                    app.update_search();
                                }
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                        app.update_search();
                    }
                    KeyCode::Enter => {
                        if !app.results.is_empty() {
                            let selected_item = &app.results[app.selected_index];
                            app.engine.launch(selected_item);
                            app.should_quit = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // 5. Restore Terminal State
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
