//! Android JNI transport. All accounting and validation remain in the shared Engine.
use focusflow_lib::{
    core::*,
    files::{atomic_write, csv_field},
};
use jni::{
    objects::{JClass, JString},
    sys::jstring,
    JNIEnv,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

struct Store {
    engine: Arc<Mutex<Engine>>,
    dir: PathBuf,
    sync: Arc<focusflow_lib::sync::LanSync>,
}
static STORE: OnceLock<Mutex<Option<Store>>> = OnceLock::new();
fn arg<T: DeserializeOwned>(a: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(
        a.get(name)
            .cloned()
            .ok_or_else(|| format!("Eksik alan: {name}"))?,
    )
    .map_err(|_| format!("Geçersiz alan: {name}"))
}
fn value<T: serde::Serialize>(v: T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}
fn export(e: &Engine, format: &str) -> Result<String, String> {
    if format == "json" {
        return serde_json::to_string_pretty(&e.backup()?).map_err(|e| e.to_string());
    }
    if format != "csv" {
        return Err("Geçersiz dışa aktarma biçimi".into());
    }
    let cats = e.categories()?;
    let mut out = String::from("\u{feff}id,başlangıç,bitiş,mod,tür,ders,süre_saniye,durum\n");
    for s in e.sessions()? {
        let date = |n| {
            chrono::DateTime::from_timestamp_millis(n)
                .map(|d| d.with_timezone(&chrono::Local).to_rfc3339())
                .unwrap_or_default()
        };
        let category = cats
            .iter()
            .find(|c| c.id == s.category_id)
            .map(|c| c.name.as_str())
            .unwrap_or("");
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
    Ok(out)
}
fn parse_backup(a: &Value) -> Result<Backup, String> {
    let raw: String = arg(a, "data")?;
    if raw.len() > 100_000_000 {
        return Err("Yedek 100 MB sınırını aşıyor.".into());
    }
    let backup: Backup = serde_json::from_str(&raw).map_err(|e| format!("Yedek okunamadı: {e}"))?;
    validate_backup(&backup)?;
    Ok(backup)
}
fn dispatch(store: &mut Store, cmd: &str, a: Value) -> Result<Value, String> {
    if cmd == "sync_status" {
        return value(store.sync.status()?);
    }
    if cmd == "sync_configure" {
        return value(store.sync.configure(
            &arg::<String>(&a, "action")?,
            a.get("code").and_then(|v| v.as_str()).map(String::from),
        )?);
    }
    if ["validate_restore", "restore_data"].contains(&cmd) && store.sync.enabled() {
        return Err("Geri yüklemeden önce Wi-Fi eşitlemesini kapatın.".into());
    }
    let mut guard = store
        .engine
        .lock()
        .map_err(|_| "Kayıt motoru kilidi hatası")?;
    let e = &mut *guard;
    e.checkpoint(Stamp::now())?;
    match cmd {
        "snapshot" => return value(e.snapshot()?),
        "start" => e.start(
            &arg::<String>(&a, "mode")?,
            &arg::<String>(&a, "kind")?,
            arg(&a, "categoryId")?,
            arg(&a, "minutes")?,
            Stamp::now(),
        )?,
        "action" => e.action(&arg::<String>(&a, "action")?, Stamp::now())?,
        "save_settings" => e.save_settings(arg(&a, "settings")?)?,
        "save_category" => e.save_category(
            a.get("id")
                .filter(|v| !v.is_null())
                .map(|_| arg(&a, "id"))
                .transpose()?,
            arg(&a, "name")?,
            arg(&a, "color")?,
        )?,
        "archive_category" => e.archive_category(arg(&a, "id")?)?,
        "edit_session" => e.edit_session(
            arg(&a, "id")?,
            arg(&a, "categoryId")?,
            arg(&a, "start")?,
            arg(&a, "end")?,
            arg(&a, "duration")?,
        )?,
        "delete_session" => e.delete_session(arg(&a, "id")?)?,
        "data_path" => return value(store.dir.to_string_lossy()),
        // These commands are called only by native Android code, never exposed by the JS allowlist.
        "service_tick" => {
            return Ok(
                json!({"audio_settings":e.settings()?,"sounds":std::mem::take(&mut e.sounds),"language":e.settings()?.language,"active":e.active()?,"messages":std::mem::take(&mut e.notifications),"notice":e.notice}),
            )
        }
        "export_data" => return value(export(e, &arg::<String>(&a, "format")?)?),
        "validate_restore" => {
            parse_backup(&a)?;
            if e.active()?.is_some() {
                return Err("Geri yüklemeden önce aktif oturumu bitirin.".into());
            }
        }
        "restore_data" => {
            let backup = parse_backup(&a)?;
            if e.active()?.is_some() {
                return Err("Geri yüklemeden önce aktif oturumu bitirin.".into());
            }
            let before = serde_json::to_vec_pretty(&e.backup()?).map_err(|e| e.to_string())?;
            atomic_write(
                &store.dir.join(format!(
                    "before-restore-{}.json",
                    chrono::Local::now().timestamp_millis()
                )),
                &before,
            )?;
            e.restore(backup, Stamp::now())?;
        }
        _ => return Err("Bilinmeyen komut".into()),
    }
    Ok(Value::Null)
}
fn response(f: impl FnOnce() -> Result<Value, String>) -> String {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(v)) => json!({"ok":true,"value":v}).to_string(),
        Ok(Err(e)) => json!({"ok":false,"error":e}).to_string(),
        Err(_) => {
            json!({"ok":false,"error":"Kayıt motoru hatası. Uygulamayı yeniden açın."}).to_string()
        }
    }
}
#[no_mangle]
pub extern "system" fn Java_com_focusflow_mobile_NativeBridge_initialize(
    mut env: JNIEnv,
    _: JClass,
    path: JString,
) -> jstring {
    let out = response(|| {
        let path: String = env.get_string(&path).map_err(|e| e.to_string())?.into();
        let mut slot = STORE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "Veri kilidi hatası")?;
        if slot.is_none() {
            let path = PathBuf::from(path);
            let mut engine = Engine::open(&path)?;
            engine.set_timing_policy(TimingPolicy::Mobile);
            let dir: PathBuf = path.parent().ok_or("Veri dizini yok")?.into();
            let engine = Arc::new(Mutex::new(engine));
            let sync = focusflow_lib::sync::LanSync::new(engine.clone(), dir.clone(), false);
            *slot = Some(Store { engine, dir, sync });
        }
        Ok(Value::Null)
    });
    env.new_string(out)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_com_focusflow_mobile_NativeBridge_call(
    mut env: JNIEnv,
    _: JClass,
    command: JString,
    args: JString,
) -> jstring {
    let out = response(|| {
        let cmd: String = env.get_string(&command).map_err(|e| e.to_string())?.into();
        let args: String = env.get_string(&args).map_err(|e| e.to_string())?.into();
        let a = serde_json::from_str(&args).map_err(|e| format!("Geçersiz istek: {e}"))?;
        let mut guard = STORE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "Veri kilidi hatası")?;
        dispatch(
            guard.as_mut().ok_or("Kayıt motoru henüz açılmadı")?,
            &cmd,
            a,
        )
    });
    env.new_string(out)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> Store {
        let mut e = Engine::from_connection(
            rusqlite::Connection::open_in_memory().unwrap(),
            Stamp::now(),
        )
        .unwrap();
        e.set_timing_policy(TimingPolicy::Mobile);
        let engine = Arc::new(Mutex::new(e));
        let dir =
            std::env::temp_dir().join(format!("focusflow-android-rpc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sync = focusflow_lib::sync::LanSync::new(engine.clone(), dir.clone(), false);
        Store { engine, dir, sync }
    }
    #[test]
    fn malformed_requests_are_errors() {
        let mut s = store();
        for (cmd, a) in [
            ("start", json!({})),
            (
                "start",
                json!({"mode":"pomodoro","kind":"work","categoryId":1,"minutes":-1}),
            ),
            ("save_category", json!({"name":3,"color":"#ffffff"})),
            ("arbitrary_file", json!({"path":"/etc/passwd"})),
        ] {
            assert!(dispatch(&mut s, cmd, a).is_err());
        }
    }
}
