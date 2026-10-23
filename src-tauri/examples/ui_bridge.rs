//! Test-only stdio transport. Gerçek Engine + geçici SQLite; uygulamaya dahil edilmez.
use focusflow_lib::core::*;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
fn main() {
    let path = std::env::args().nth(1).expect("geçici SQLite yolu");
    let mut e = Engine::open(std::path::Path::new(&path)).unwrap();
    for line in io::stdin().lock().lines() {
        let result = (|| -> Result<Value, String> {
            let v: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let a = &v["args"];
            e.checkpoint(Stamp::now())?;
            match v["cmd"].as_str().unwrap_or("") {
                "snapshot" => {
                    return serde_json::to_value(e.snapshot()?).map_err(|e| e.to_string())
                }
                "start" => e.start(
                    a["mode"].as_str().unwrap(),
                    a["kind"].as_str().unwrap(),
                    a["categoryId"].as_i64().unwrap(),
                    a["minutes"].as_u64().unwrap() as u32,
                    Stamp::now(),
                )?,
                "action" => e.action(a["action"].as_str().unwrap(), Stamp::now())?,
                "save_category" => e.save_category(
                    a["id"].as_i64(),
                    a["name"].as_str().unwrap().into(),
                    a["color"].as_str().unwrap().into(),
                )?,
                "archive_category" => e.archive_category(a["id"].as_i64().unwrap())?,
                "save_settings" => e.save_settings(
                    serde_json::from_value(a["settings"].clone()).map_err(|e| e.to_string())?,
                )?,
                "delete_session" => e.delete_session(a["id"].as_i64().unwrap())?,
                "edit_session" => e.edit_session(
                    a["id"].as_i64().unwrap(),
                    a["categoryId"].as_i64().unwrap(),
                    a["start"].as_i64().unwrap(),
                    a["end"].as_i64().unwrap(),
                    a["duration"].as_i64().unwrap(),
                )?,
                "data_path" => return Ok(json!(path)),
                _ => return Err("Bu test taşıyıcısı masaüstü eklentilerini kapsamaz".into()),
            }
            Ok(Value::Null)
        })();
        let out = match result {
            Ok(v) => json!({"ok":v}),
            Err(e) => json!({"error":e}),
        };
        println!("{out}");
        io::stdout().flush().unwrap();
    }
}
