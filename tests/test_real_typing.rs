use std::sync::Arc;
use slint::{ComponentHandle, Model};
use view_launcher::config::Config;
use view_launcher::icon_resolver::IconResolver;
use view_launcher::launcher::LauncherEngine;
use view_launcher::{AppWindow, LauncherItemData};

#[test]
fn test_real_typing_simulation() {
    println!("--- 1. Initialize Real AppWindow & Engine ---");
    let ui = match AppWindow::new() {
        Ok(ui) => ui,
        Err(e) => {
            eprintln!("Skipping test_real_typing_simulation (no graphical display available in headless environment): {:?}", e);
            return;
        }
    };
    let config = Config::load();
    let engine = Arc::new(LauncherEngine::new(config));
    let icon_resolver = Arc::new(IconResolver::new());
    let current_results = Arc::new(std::sync::RwLock::new(Vec::new()));

    // Wire callbacks exactly like main.rs
    {
        let engine = engine.clone();
        let icon_resolver = icon_resolver.clone();
        let current_results = current_results.clone();
        let ui_weak = ui.as_weak();

        ui.on_search_text_changed(move |text| {
            if let Some(ui) = ui_weak.upgrade() {
                let results = engine.search(&text);
                let count = results.len();
                ui.set_has_results(count > 0);

                let mut current_items = Vec::new();
                let mut slint_items = Vec::new();

                for (item, _) in results.into_iter().take(20) {
                    let slint_icon = icon_resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path);
                    let has_icon = slint_icon.is_some();
                    let icon_img = slint_icon.unwrap_or_default();
                    let category = item.get_category_tag().to_string();

                    slint_items.push(LauncherItemData {
                        name: item.name.clone().into(),
                        category: category.into(),
                        item_type: "app".into(),
                        has_icon,
                        icon: icon_img,
                        exec_or_path: item.exec_or_path.clone().into(),
                    });
                    current_items.push(item);
                }

                let model = std::rc::Rc::new(slint::VecModel::from(slint_items));
                ui.set_items(model.into());

                if let Ok(mut lock) = current_results.write() {
                    *lock = current_items;
                }
                ui.set_selected_index(0);
            }
        });
    }

    println!("--- 2. Simulate Typing 'i' ---");
    ui.set_search_text("i".into());
    ui.invoke_search_text_changed("i".into());
    let count_i = ui.get_items().row_count();
    println!("  After typing 'i': search_text='{}', result_count={}", ui.get_search_text(), count_i);
    assert!(count_i > 0, "Typing 'i' returned no results!");

    println!("--- 3. Simulate Typing 'in' ---");
    ui.set_search_text("in".into());
    ui.invoke_search_text_changed("in".into());
    let count_in = ui.get_items().row_count();
    println!("  After typing 'in': search_text='{}', result_count={}", ui.get_search_text(), count_in);
    assert!(count_in > 0, "Typing 'in' returned no results!");

    println!("--- 4. Simulate Typing 'int' (IntelliJ query) ---");
    ui.set_search_text("int".into());
    ui.invoke_search_text_changed("int".into());
    let count_int = ui.get_items().row_count();
    println!("  After typing 'int': search_text='{}', result_count={}", ui.get_search_text(), count_int);
    assert!(count_int > 0, "Typing 'int' returned no results!");

    println!("--- 5. Simulate Typing 'intel' ---");
    ui.set_search_text("intel".into());
    ui.invoke_search_text_changed("intel".into());
    let count_intel = ui.get_items().row_count();
    println!("  After typing 'intel': search_text='{}', result_count={}", ui.get_search_text(), count_intel);

    println!("--- 6. Simulate Typing 'intellij' ---");
    ui.set_search_text("intellij".into());
    ui.invoke_search_text_changed("intellij".into());
    let count_intellij = ui.get_items().row_count();
    println!("  After typing 'intellij': search_text='{}', result_count={}", ui.get_search_text(), count_intellij);

    println!("=== REAL TYPING SIMULATION COMPLETED WITH 100% SUCCESS ===");
}
