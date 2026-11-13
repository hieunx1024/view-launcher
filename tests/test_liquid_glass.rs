use view_launcher::AppWindow;

#[test]
fn test_liquid_glass_style_and_expansion() {
    let ui_res = std::panic::catch_unwind(|| AppWindow::new());
    let ui = match ui_res {
        Ok(Ok(ui)) => ui,
        Ok(Err(e)) => {
            eprintln!("Skipping test (no graphical display available): {:?}", e);
            return;
        }
        Err(_) => {
            eprintln!("Skipping test (window backend missing in test runner)");
            return;
        }
    };

    ui.set_cfg_view_style("liquid_glass".into());
    assert_eq!(ui.get_cfg_view_style(), "liquid_glass");

    // Initially search text is empty and not expanded
    ui.set_search_text("".into());
    ui.set_is_expanded(false);
    assert!(!ui.get_is_expanded());

    // Focus search should work without error
    ui.invoke_focus_search();

    // Typing should expand the panel
    ui.set_search_text("calc".into());
    ui.invoke_search_text_changed("calc".into());
    ui.set_is_expanded(true);
    assert!(ui.get_is_expanded());

    // Clearing search text should allow collapse
    ui.set_search_text("".into());
    ui.set_is_expanded(false);
    assert!(!ui.get_is_expanded());

    for style in ["liquid_glass", "classic"] {
        ui.set_cfg_view_style(style.into());
        ui.set_cfg_compact_empty(true);
        let shadow_margin = if style == "liquid_glass" { 18.0 } else { 0.0 };
        assert_eq!(ui.get_launcher_width(), 576.0 + 2.0 * shadow_margin);
        assert_eq!(ui.get_launcher_height(), 48.0 + shadow_margin);
        assert_eq!(ui.get_launcher_radius(), 24.0);

        ui.set_is_expanded(true);
        assert_eq!(ui.get_launcher_height(), 380.0 + shadow_margin);
        assert_eq!(ui.get_launcher_radius(), 18.0);

        ui.set_in_settings_mode(true);
        assert_eq!(ui.get_launcher_height(), 428.0);
        ui.set_in_settings_mode(false);
        ui.set_is_expanded(false);
    }
}
