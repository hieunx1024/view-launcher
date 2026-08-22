use view_launcher::launcher::LauncherEngine;
use view_launcher::config::Config;
use view_launcher::icon_resolver::IconResolver;

#[test]
fn test_live_search_int() {
    println!("=== Testing Launcher on Live System ===");
    let config = Config::load();
    let engine = LauncherEngine::new(config);
    let resolver = IconResolver::new();

    println!("Indexed {} apps from live system.", engine.apps.len());
    for app in &engine.apps {
        if app.name.to_lowercase().contains("int") {
            println!("  Found candidate app: name='{}', icon='{:?}', exec='{}'", app.name, app.icon, app.exec_or_path);
        }
    }

    println!("\n=== Running search('int') ===");
    let start = std::time::Instant::now();
    let results = engine.search("int");
    let dur = start.elapsed();
    println!("Search 'int' returned {} items in {:?}", results.len(), dur);

    for (i, (item, _indices)) in results.iter().enumerate() {
        let icon_start = std::time::Instant::now();
        let icon_res = resolver.resolve_icon(item.icon.as_deref(), &item.name, &item.exec_or_path);
        let icon_dur = icon_start.elapsed();
        println!("  Item #{}: '{}' (type: {:?}, icon: {:?}, resolved: {}, took: {:?})", 
            i, item.name, item.item_type, item.icon, icon_res.is_some(), icon_dur);
    }
}
