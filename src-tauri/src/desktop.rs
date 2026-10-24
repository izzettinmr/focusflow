use crate::core::*;
use crate::files::{atomic_write, csv_field, read_backup, write_export};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_notification::NotificationExt;
struct AppState {
    engine: Arc<Mutex<Engine>>,
    data_dir: PathBuf,
    sync: Arc<crate::sync::LanSync>,
    audio: crate::sound::Player,
}
fn tr(app: &tauri::AppHandle, text: &str) -> String {
    let state = app.state::<AppState>();
    let language = state
        .engine
        .lock()
        .ok()
        .and_then(|e| e.settings().ok())
        .map(|s| s.language)
        .unwrap_or_else(|| "tr".into());
    crate::i18n::translate(&language, text)
}
fn with_engine<T>(
    state: &tauri::State<AppState>,
    f: impl FnOnce(&mut Engine) -> Result<T, String>,
) -> Result<T, String> {
    let mut e = state
        .engine
        .lock()
        .map_err(|_| "Veri kilidine erişilemedi")?;
    f(&mut e)
}
#[tauri::command]
fn snapshot(state: tauri::State<AppState>) -> Result<Snapshot, String> {
    with_engine(&state, |e| e.snapshot())
}
#[tauri::command]
fn sync_status(state: tauri::State<AppState>) -> Result<crate::sync::Status, String> {
    state.sync.status()
}
#[tauri::command]
fn sync_configure(
    state: tauri::State<AppState>,
    action: String,
    code: Option<String>,
) -> Result<crate::sync::Status, String> {
    state.sync.configure(&action, code)
}
#[tauri::command]
fn start(
    state: tauri::State<AppState>,
    mode: String,
    kind: String,
    category_id: i64,
    minutes: u32,
) -> Result<(), String> {
    with_engine(&state, |e| {
        e.start(&mode, &kind, category_id, minutes, Stamp::now())
    })
}
#[tauri::command]
fn action(state: tauri::State<AppState>, action: String) -> Result<(), String> {
    with_engine(&state, |e| e.action(&action, Stamp::now()))
}
#[tauri::command]
fn save_settings(state: tauri::State<AppState>, settings: Settings) -> Result<(), String> {
    with_engine(&state, |e| e.save_settings(settings))
}
#[tauri::command]
async fn preview_sound(
    state: tauri::State<'_, AppState>,
    cue: String,
    volume: u32,
) -> Result<(), String> {
    let audio = state.audio.clone();
    tauri::async_runtime::spawn_blocking(move || audio.preview(&cue, volume))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn save_category(
    state: tauri::State<AppState>,
    id: Option<i64>,
    name: String,
    color: String,
) -> Result<(), String> {
    with_engine(&state, |e| e.save_category(id, name, color))
}
#[tauri::command]
fn archive_category(state: tauri::State<AppState>, id: i64) -> Result<(), String> {
    with_engine(&state, |e| e.archive_category(id))
}
#[tauri::command]
fn edit_session(
    state: tauri::State<AppState>,
    id: i64,
    category_id: i64,
    start: i64,
    end: i64,
    duration: i64,
) -> Result<(), String> {
    with_engine(&state, |e| {
        e.edit_session(id, category_id, start, end, duration)
    })
}
#[tauri::command]
fn delete_session(state: tauri::State<AppState>, id: i64) -> Result<(), String> {
    with_engine(&state, |e| e.delete_session(id))
}
#[tauri::command]
async fn export_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    format: String,
) -> Result<bool, String> {
    if !["json", "csv"].contains(&format.as_str()) {
        return Err("Geçersiz dışa aktarma biçimi".into());
    }
    let filename = format!(
        "FocusFlow-{}.{}",
        chrono::Local::now().format("%Y-%m-%d"),
        format
    );
    let Some(path) = app
        .dialog()
        .file()
        .set_title(tr(&app, "FocusFlow · Dışa aktar"))
        .add_filter(format.to_uppercase(), &[format.as_str()])
        .set_file_name(filename)
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    with_engine(&state, |e| {
        e.checkpoint(Stamp::now())?;
        let data = if format == "json" {
            serde_json::to_string_pretty(&e.backup()?).map_err(|e| e.to_string())?
        } else if format == "csv" {
            let cats = e.categories()?;
            let mut out =
                String::from("\u{feff}id,başlangıç,bitiş,mod,tür,ders,süre_saniye,durum\n");
            for s in e.sessions()? {
                let category = cats
                    .iter()
                    .find(|c| c.id == s.category_id)
                    .map(|c| c.name.as_str())
                    .unwrap_or("");
                let date = |v: i64| {
                    chrono::DateTime::from_timestamp_millis(v)
                        .map(|d| d.with_timezone(&chrono::Local).to_rfc3339())
                        .unwrap_or_default()
                };
                out.push_str(&format!(
                    "{},{},{},{},{},{},{:.3},{}\n",
                    s.id,
                    csv_field(&date(s.started_at)),
                    csv_field(&s.ended_at.map(date).unwrap_or_default()),
                    csv_field(&s.mode),
                    csv_field(&s.kind),
                    csv_field(category),
                    s.elapsed_ms as f64 / 1000.0,
                    csv_field(&s.status)
                ));
            }
            out
        } else {
            return Err("Geçersiz dışa aktarma biçimi".into());
        };
        write_export(&path, &state.data_dir, data.as_bytes())
    })?;
    Ok(true)
}
#[tauri::command]
async fn restore_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    if state.sync.enabled() {
        return Err("Geri yüklemeden önce Wi-Fi eşitlemesini kapatın.".into());
    }
    with_engine(&state, |e| {
        if e.active()?.is_some() {
            Err("Geri yüklemeden önce aktif oturumu bitirin.".into())
        } else {
            Ok(())
        }
    })?;
    let Some(path) = app
        .dialog()
        .file()
        .set_title(tr(&app, "FocusFlow · JSON yedeği seç"))
        .add_filter("FocusFlow JSON", &["json"])
        .blocking_pick_file()
    else {
        return Ok(false);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let b = read_backup(&path)?;
    if !app.dialog().message(tr(&app, "Mevcut veriler seçilen yedekle değiştirilecek. Önce mevcut verilerinizin otomatik güvenlik yedeği alınacak.")).title(tr(&app, "FocusFlow · Geri yüklemeyi onayla")).kind(MessageDialogKind::Warning).buttons(MessageDialogButtons::OkCancelCustom(tr(&app,"Geri yükle"),tr(&app,"Vazgeç"))).blocking_show(){return Ok(false)}
    with_engine(&state, |e| {
        if e.active()?.is_some() {
            return Err("Geri yüklemeden önce aktif oturumu bitirin.".into());
        }
        let before = serde_json::to_vec_pretty(&e.backup()?).map_err(|e| e.to_string())?;
        atomic_write(
            &state.data_dir.join(format!(
                "before-restore-{}.json",
                chrono::Local::now().timestamp_millis()
            )),
            &before,
        )?;
        e.restore(b, Stamp::now())
    })?;
    Ok(true)
}
#[tauri::command]
fn data_path(state: tauri::State<AppState>) -> String {
    state.data_dir.to_string_lossy().into_owned()
}
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}
#[tauri::command]
fn quit(app: tauri::AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    with_engine(&state, |e| e.action("pause", Stamp::now()))?;
    app.exit(0);
    Ok(())
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(&dir)?;
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
            }
            #[cfg(not(unix))]
            std::fs::create_dir_all(&dir)?;
            let engine = Arc::new(Mutex::new(
                Engine::open(&dir.join("focusflow.db")).map_err(std::io::Error::other)?,
            ));
            let sync = crate::sync::LanSync::new(engine.clone(), dir.clone(), true);
            let audio = crate::sound::Player::new(&dir).map_err(std::io::Error::other)?;
            app.manage(AppState {
                engine,
                data_dir: dir,
                sync,
                audio,
            });
            let show = MenuItem::with_id(app, "show", "FocusFlow'u aç", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Duraklat / devam et", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Kaydet ve çık", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause, &exit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().expect("uygulama simgesi").clone())
                .tooltip("FocusFlow · Odağını bul")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let state = app.state::<AppState>();
                        let result = with_engine(&state, |e| {
                            let a = e.active()?;
                            e.action(
                                if a.is_some_and(|s| s.status == "running") {
                                    "pause"
                                } else {
                                    "resume"
                                },
                                Stamp::now(),
                            )
                        });
                        if let Err(e) = result {
                            let _ = app
                                .notification()
                                .builder()
                                .title("FocusFlow")
                                .body(tr(app, &e))
                                .show();
                        }
                    }
                    "exit" => {
                        let state = app.state::<AppState>();
                        match with_engine(&state, |e| e.action("pause", Stamp::now())) {
                            Ok(()) => app.exit(0),
                            Err(e) => {
                                let _ = app
                                    .notification()
                                    .builder()
                                    .title(tr(app, "Kaydedilemedi"))
                                    .body(tr(app, &e))
                                    .show();
                            }
                        }
                    }
                    _ => {}
                })
                .build(app)?;
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut shown_language = String::new();
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let state = handle.state::<AppState>();
                    let language = state
                        .engine
                        .lock()
                        .ok()
                        .and_then(|e| e.settings().ok())
                        .map(|s| s.language)
                        .unwrap_or_else(|| "tr".into());
                    if shown_language != language {
                        let _ = show.set_text(crate::i18n::translate(&language, "FocusFlow'u aç"));
                        let _ = pause
                            .set_text(crate::i18n::translate(&language, "Duraklat / devam et"));
                        let _ = exit.set_text(crate::i18n::translate(&language, "Kaydet ve çık"));
                        shown_language = language.clone();
                    }
                    if let Ok(mut e) = state.engine.lock() {
                        if let Err(error) = e.checkpoint(Stamp::now()) {
                            e.notice = Some(format!(
                            "Kayıt hatası: {error}. Oturumu duraklatın; disk alanını kontrol edin."
                        ));
                        }
                        let cues = std::mem::take(&mut e.sounds);
                        if let Ok(settings) = e.settings() {
                            if settings.sound_enabled {
                                for cue in cues {
                                    state.audio.enqueue(cue, settings.sound_volume);
                                }
                            }
                        }
                        for body in e.notifications.drain(..) {
                            if let Err(error) = handle
                                .notification()
                                .builder()
                                .title("FocusFlow")
                                .body(crate::i18n::translate(&language, &body))
                                .show()
                            {
                                eprintln!("Bildirim gönderilemedi: {error}");
                            }
                        }
                    };
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            sync_status,
            sync_configure,
            start,
            action,
            save_settings,
            preview_sound,
            save_category,
            archive_category,
            edit_session,
            delete_session,
            export_file,
            restore_file,
            data_path,
            hide_window,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("FocusFlow başlatılamadı");
}
