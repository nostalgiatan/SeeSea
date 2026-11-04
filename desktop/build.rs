fn main() {
    // The page of the workbench may ask for these and nothing else: they draw its own title bar.
    let attrs = tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&["win_minimize", "win_toggle_maximize", "win_hide", "win_drag", "win_resize"]),
    );
    tauri_build::try_build(attrs).expect("failed to run the Tauri build script");
}
