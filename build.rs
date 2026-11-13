fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec![manifest_dir.join("ui")]);
    slint_build::compile_with_config("ui/app_window.slint", config).expect("Slint build failed");
}
