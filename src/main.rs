mod calc;
mod config;
mod history;
mod icons;
mod launcher;
mod ui;

use std::io::{stdout, Write};
use std::path::PathBuf;
use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use config::Config;
use launcher::{LauncherEngine, LauncherItem};
use ui::UiState;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

#[cfg(windows)]
use std::net::{TcpListener, TcpStream};

#[cfg(unix)]
fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("view-launcher.sock")
    } else {
        let uid = unsafe { libc_getuid() };
        PathBuf::from(format!("/tmp/view-launcher-{}.sock", uid))
    }
}

#[cfg(unix)]
unsafe fn libc_getuid() -> u32 {
    #[cfg(target_os = "linux")]
    {
        extern "C" {
            fn getuid() -> u32;
        }
        getuid()
    }
    #[cfg(not(target_os = "linux"))]
    {
        1000
    }
}

#[cfg(unix)]
fn handle_single_instance(exit_trigger: Arc<AtomicBool>) -> bool {
    let socket_path = get_socket_path();

    // 1. Try connecting to existing socket
    if let Ok(mut stream) = UnixStream::connect(&socket_path) {
        let _ = stream.write_all(b"toggle");
        return false;
    }

    // 2. Remove stale socket file if it exists
    let _ = std::fs::remove_file(&socket_path);

    // 3. Bind new Unix socket
    if let Ok(listener) = UnixListener::bind(&socket_path) {
        let path_clone = socket_path.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_ok() {
                    exit_trigger.store(true, Ordering::SeqCst);
                    break;
                }
            }
            let _ = std::fs::remove_file(&path_clone);
        });
    }

    true
}

#[cfg(windows)]
const LOCAL_PORT: u16 = 19428;

#[cfg(windows)]
fn handle_single_instance(exit_trigger: Arc<AtomicBool>) -> bool {
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", LOCAL_PORT)) {
        let _ = stream.write_all(b"toggle");
        return false;
    }
    
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
    cursor_pos: usize,
    results: Vec<(LauncherItem, Vec<usize>)>,
    selected_index: usize,
    engine: LauncherEngine,
    config: Config,
    should_quit: bool,
}

impl App {
    fn new(config: Config) -> Self {
        let engine = LauncherEngine::new(config.clone());
        let results = engine.search("");
        Self {
            input: String::new(),
            cursor_pos: 0,
            results,
            selected_index: 0,
            engine,
            config,
            should_quit: false,
        }
    }

    fn update_search(&mut self) {
        self.results = self.engine.search(&self.input);
        self.selected_index = 0; // Focus top match
    }

    fn delete_word_backward(&mut self) {
        let chars: Vec<char> = self.input.chars().collect();
        if self.cursor_pos == 0 || chars.is_empty() {
            return;
        }

        let mut new_pos = self.cursor_pos.min(chars.len());
        // 1. Skip spaces before cursor
        while new_pos > 0 && chars[new_pos - 1].is_whitespace() {
            new_pos -= 1;
        }
        // 2. Skip word characters
        while new_pos > 0 && !chars[new_pos - 1].is_whitespace() && chars[new_pos - 1] != '/' {
            new_pos -= 1;
        }
        if new_pos == self.cursor_pos && new_pos > 0 {
            new_pos -= 1;
        }

        let mut new_chars = Vec::new();
        new_chars.extend_from_slice(&chars[..new_pos]);
        if self.cursor_pos < chars.len() {
            new_chars.extend_from_slice(&chars[self.cursor_pos..]);
        }

        self.input = new_chars.into_iter().collect();
        self.cursor_pos = new_pos;
        self.update_search();
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

enum ActiveIme {
    None,
    Fcitx(&'static str),
    Ibus(String),
}

struct ImeGuard {
    active_ime: ActiveIme,
}

impl ImeGuard {
    fn new() -> Self {
        let mut active_ime = ActiveIme::None;
        #[cfg(target_os = "linux")]
        {
            let mut fcitx_cmd = "fcitx5-remote";
            let mut fcitx_output = std::process::Command::new("fcitx5-remote").output();
            if fcitx_output.is_err() {
                fcitx_cmd = "fcitx-remote";
                fcitx_output = std::process::Command::new("fcitx-remote").output();
            }

            if let Ok(out) = fcitx_output {
                let status = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if status == "2" {
                    active_ime = ActiveIme::Fcitx(fcitx_cmd);
                    let _ = std::process::Command::new(fcitx_cmd).arg("-c").status();

                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let _ = std::process::Command::new(fcitx_cmd).arg("-c").status();
                        std::thread::sleep(std::time::Duration::from_millis(150));
                        let _ = std::process::Command::new(fcitx_cmd).arg("-c").status();
                    });
                }
            }

            if matches!(active_ime, ActiveIme::None) {
                if let Ok(output) = std::process::Command::new("ibus").arg("engine").output() {
                    let engine = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if engine == "bamboo" || engine == "unikey" || engine == "bogo" || engine.contains("vietnamese") {
                        active_ime = ActiveIme::Ibus(engine);
                        let _ = std::process::Command::new("ibus").args(&["engine", "xkb:us::eng"]).status();
                    }
                }
            }
        }
        Self { active_ime }
    }
}

impl Drop for ImeGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        match &self.active_ime {
            ActiveIme::Fcitx(cmd) => {
                let _ = std::process::Command::new(*cmd).arg("-o").status();
            }
            ActiveIme::Ibus(original_engine) => {
                let _ = std::process::Command::new("ibus").args(&["engine", original_engine]).status();
            }
            ActiveIme::None => {}
        }
    }
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
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
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(get_socket_path());
        }
        original_hook(panic_info);
    }));

    // 2. Single Instance Lock check
    let exit_trigger = Arc::new(AtomicBool::new(false));
    if !handle_single_instance(exit_trigger.clone()) {
        return Ok(());
    }

    // 3. Load Configuration and Setup Engine
    let config = Config::load();
    let _ime_guard = if config.search.disable_ime.unwrap_or(false) {
        Some(ImeGuard::new())
    } else {
        None
    };
    let mut app = App::new(config);

    // 4. Initialize Terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    // 5. Main Event Loop
    while !app.should_quit {
        if exit_trigger.load(Ordering::SeqCst) {
            app.should_quit = true;
            break;
        }

        // Auto-refresh once background file scanning completes
        let has_files = app.results.iter().any(|(i, _)| matches!(i.item_type, launcher::ItemType::File | launcher::ItemType::Dir));
        let scan_completed = app.engine.shallow_files.read().map(|f| !f.is_empty()).unwrap_or(false);
        if !has_files && scan_completed && app.input.is_empty() {
            app.update_search();
        }

        terminal.draw(|f| {
            let state = UiState {
                input: &app.input,
                cursor_pos: app.cursor_pos,
                results: &app.results,
                selected_index: app.selected_index,
                theme: &app.config.theme,
            };
            ui::draw(f, &state);
        })?;

        // Process inputs
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }

                let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let has_alt = key.modifiers.contains(KeyModifiers::ALT);
                let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                match key.code {
                    KeyCode::Esc => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('c') if has_ctrl => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('u') if has_ctrl => {
                        app.input.clear();
                        app.cursor_pos = 0;
                        app.update_search();
                    }
                    KeyCode::Char('w') if has_ctrl => {
                        app.delete_word_backward();
                    }
                    KeyCode::Char('a') if has_ctrl => {
                        app.cursor_pos = 0;
                    }
                    KeyCode::Char('e') if has_ctrl => {
                        app.cursor_pos = app.input.chars().count();
                    }
                    KeyCode::Left => {
                        app.cursor_pos = app.cursor_pos.saturating_sub(1);
                    }
                    KeyCode::Right => {
                        let char_count = app.input.chars().count();
                        if app.cursor_pos < char_count {
                            app.cursor_pos += 1;
                        }
                    }
                    KeyCode::Home => {
                        app.cursor_pos = 0;
                    }
                    KeyCode::End => {
                        app.cursor_pos = app.input.chars().count();
                    }
                    KeyCode::Char('j') if has_ctrl => {
                        if !app.results.is_empty() {
                            app.selected_index = (app.selected_index + 1) % app.results.len();
                        }
                    }
                    KeyCode::Char('n') if has_ctrl => {
                        if !app.results.is_empty() {
                            app.selected_index = (app.selected_index + 1) % app.results.len();
                        }
                    }
                    KeyCode::Down => {
                        if !app.results.is_empty() {
                            app.selected_index = (app.selected_index + 1) % app.results.len();
                        }
                    }
                    KeyCode::Char('k') if has_ctrl => {
                        if !app.results.is_empty() {
                            if app.selected_index == 0 {
                                app.selected_index = app.results.len() - 1;
                            } else {
                                app.selected_index -= 1;
                            }
                        }
                    }
                    KeyCode::Char('p') if has_ctrl => {
                        if !app.results.is_empty() {
                            if app.selected_index == 0 {
                                app.selected_index = app.results.len() - 1;
                            } else {
                                app.selected_index -= 1;
                            }
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
                    KeyCode::PageDown | KeyCode::Char('d') if has_ctrl => {
                        if !app.results.is_empty() {
                            app.selected_index = (app.selected_index + 5).min(app.results.len() - 1);
                        }
                    }
                    KeyCode::PageUp => {
                        if !app.results.is_empty() {
                            app.selected_index = app.selected_index.saturating_sub(5);
                        }
                    }
                    KeyCode::Backspace => {
                        if has_alt {
                            app.delete_word_backward();
                        } else if app.cursor_pos > 0 {
                            let mut chars: Vec<char> = app.input.chars().collect();
                            if app.cursor_pos <= chars.len() {
                                chars.remove(app.cursor_pos - 1);
                                app.input = chars.into_iter().collect();
                                app.cursor_pos -= 1;
                                app.update_search();
                            }
                        }
                    }
                    KeyCode::Delete => {
                        let mut chars: Vec<char> = app.input.chars().collect();
                        if app.cursor_pos < chars.len() {
                            chars.remove(app.cursor_pos);
                            app.input = chars.into_iter().collect();
                            app.update_search();
                        }
                    }
                    KeyCode::Tab => {
                        if !app.results.is_empty() {
                            let selected_item = &app.results[app.selected_index].0;
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
                                    app.cursor_pos = app.input.chars().count();
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
                                    app.cursor_pos = app.input.chars().count();
                                    app.update_search();
                                }
                                launcher::ItemType::App => {
                                    app.input = selected_item.name.clone();
                                    app.cursor_pos = app.input.chars().count();
                                    app.update_search();
                                }
                                launcher::ItemType::Calc => {
                                    app.input = selected_item.exec_or_path.clone();
                                    app.cursor_pos = app.input.chars().count();
                                    app.update_search();
                                }
                            }
                        }
                    }
                    KeyCode::Char('t') if has_alt => {
                        // Alt + T: Open containing directory in Terminal
                        if !app.results.is_empty() {
                            let selected_item = &app.results[app.selected_index].0;
                            app.engine.open_in_terminal(selected_item);
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Char('c') if has_alt => {
                        // Alt + C: Copy path or result to clipboard
                        if !app.results.is_empty() {
                            let selected_item = &app.results[app.selected_index].0;
                            app.engine.copy_to_clipboard(&selected_item.exec_or_path);
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Enter => {
                        if !app.results.is_empty() {
                            let selected_item = &app.results[app.selected_index].0;
                            if has_shift {
                                app.engine.open_in_terminal(selected_item);
                            } else {
                                app.engine.launch(selected_item);
                            }
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Char(c) => {
                        let mut chars: Vec<char> = app.input.chars().collect();
                        let pos = app.cursor_pos.min(chars.len());
                        chars.insert(pos, c);
                        app.input = chars.into_iter().collect();
                        app.cursor_pos = pos + 1;
                        app.update_search();
                    }
                    _ => {}
                }
            }
        }
    }

    // 6. Restore Terminal State and cleanup
    cleanup_terminal(&mut terminal)?;

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(get_socket_path());
    }

    Ok(())
}
