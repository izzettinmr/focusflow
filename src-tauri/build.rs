#[cfg(feature = "desktop")]
fn main() {
    if std::env::var_os("CARGO_FEATURE_DESKTOP").is_some() {
        tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "snapshot",
                "sync_status",
                "sync_configure",
                "start",
                "action",
                "save_settings",
                "save_category",
                "archive_category",
                "edit_session",
                "delete_session",
                "export_file",
                "restore_file",
                "data_path",
                "hide_window",
                "quit",
            ]),
        ))
        .expect("Tauri izinleri oluşturulamadı");
    }
}
#[cfg(not(feature = "desktop"))]
fn main() {}
