use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use view_launcher::AppWindow;

#[test]
fn test_settings_auto_save_triggers() {
    let ui_res = std::panic::catch_unwind(|| AppWindow::new());
    let ui = match ui_res {
        Ok(Ok(ui)) => ui,
        _ => return,
    };

    let saved = Arc::new(AtomicBool::new(false));
    let saved_clone = saved.clone();

    ui.on_save_settings(move |_icons, _bar, _path, _res, _depth, _auto, _compact, _mode, _opacity, _style| {
        saved_clone.store(true, Ordering::SeqCst);
    });

    // Invoke auto_save_settings
    ui.invoke_auto_save_settings();

    assert!(saved.load(Ordering::SeqCst), "auto_save_settings should trigger on_save_settings callback");
}
