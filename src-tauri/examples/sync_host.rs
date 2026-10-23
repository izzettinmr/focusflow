//! Isolated native LAN fixture. Never accepts the personal application directory.
use focusflow_lib::{core::*, sync::LanSync};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};
fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("temporary fixture directory"),
    );
    assert!(
        dir.starts_with(std::env::temp_dir())
            && dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("focusflow-sync-")
    );
    std::fs::create_dir_all(&dir).unwrap();
    let mut e = Engine::open(&dir.join("fixture.db")).unwrap();
    if e.sessions().unwrap().is_empty() {
        let now = Stamp::now();
        let start = Stamp {
            wall: now.wall - 30_000,
            mono: now.mono - 30_000,
        };
        e.start("stopwatch", "work", 1, 0, start).unwrap();
        e.action(
            "finish",
            Stamp {
                wall: start.wall + 2000,
                mono: start.mono + 2000,
            },
        )
        .unwrap();
    }
    let engine = Arc::new(Mutex::new(e));
    let sync = LanSync::new(engine.clone(), dir, true);
    if sync.status().unwrap().mode != "host" {
        sync.configure("host", None).unwrap();
    }
    for line in io::stdin().lock().lines() {
        let r = (|| -> Result<Value, String> {
            let request: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            match request["cmd"].as_str().unwrap_or("") {
                "status" => serde_json::to_value(sync.status()?).map_err(|e| e.to_string()),
                "snapshot" => serde_json::to_value(engine.lock().unwrap().snapshot()?)
                    .map_err(|e| e.to_string()),
                "rename" => {
                    engine.lock().unwrap().save_category(
                        Some(1),
                        "WiFi Physics".into(),
                        "#a99bff".into(),
                    )?;
                    Ok(json!(true))
                }
                "off" => {
                    serde_json::to_value(sync.configure("off", None)?).map_err(|e| e.to_string())
                }
                _ => Err("Unknown fixture command".into()),
            }
        })();
        println!(
            "{}",
            match r {
                Ok(v) => json!({"ok":v}),
                Err(e) => json!({"error":e}),
            }
        );
        io::stdout().flush().unwrap();
    }
}
