use focusflow_lib::{core::*, files::*};
use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn temp() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "focusflow-security-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn engine() -> Engine {
    Engine::from_connection(Connection::open_in_memory().unwrap(), Stamp::now()).unwrap()
}
fn finished() -> Engine {
    let mut e = engine();
    let t = Stamp::now();
    e.start("stopwatch", "work", 1, 0, t).unwrap();
    e.action(
        "finish",
        Stamp {
            wall: t.wall + 1000,
            mono: t.mono + 1000,
        },
    )
    .unwrap();
    e
}
#[test]
fn direct_database_export_is_denied() {
    let p = temp();
    std::fs::write(p.join("focusflow.db"), b"database-canary").unwrap();
    assert!(write_export(&p.join("focusflow.db"), &p, b"overwrite").is_err());
    assert_eq!(
        std::fs::read(p.join("focusflow.db")).unwrap(),
        b"database-canary"
    );
}
#[test]
fn dotdot_database_export_is_denied() {
    let p = temp();
    std::fs::create_dir(p.join("child")).unwrap();
    std::fs::write(p.join("focusflow.db"), b"canary").unwrap();
    assert!(write_export(&p.join("child/../focusflow.db"), &p, b"overwrite").is_err());
}
#[cfg(unix)]
#[test]
fn parent_symlink_database_export_is_denied() {
    let p = temp();
    let alias = temp().join("alias");
    std::os::unix::fs::symlink(&p, &alias).unwrap();
    std::fs::write(p.join("focusflow.db"), b"canary").unwrap();
    assert!(write_export(&alias.join("focusflow.db"), &p, b"overwrite").is_err());
}
#[test]
fn sqlite_sidecars_are_protected() {
    let p = temp();
    for name in [
        "focusflow.db-wal",
        "focusflow.db-shm",
        "focusflow.db-journal",
    ] {
        std::fs::write(p.join(name), b"canary").unwrap();
        assert!(
            write_export(&p.join(name), &p, b"overwrite").is_err(),
            "{name}"
        );
    }
}
#[cfg(unix)]
#[test]
fn exports_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let p = temp().join("backup.json");
    atomic_write(&p, b"private-canary").unwrap();
    assert_eq!(
        std::fs::metadata(p).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[cfg(unix)]
#[test]
fn database_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let p = temp().join("data.db");
    let _e = Engine::open(&p).unwrap();
    assert_eq!(
        std::fs::metadata(p).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[cfg(unix)]
#[test]
fn destination_symlink_does_not_overwrite_victim() {
    let p = temp();
    let victim = p.join("victim.txt");
    std::fs::write(&victim, b"keep").unwrap();
    let output = p.join("export.json");
    std::os::unix::fs::symlink(&victim, &output).unwrap();
    atomic_write(&output, b"export").unwrap();
    assert_eq!(std::fs::read(victim).unwrap(), b"keep");
}
#[test]
fn sql_injection_is_stored_as_text() {
    let e = engine();
    e.save_category(None, "'; DROP TABLE sessions;--".into(), "#aabbcc".into())
        .unwrap();
    assert_eq!(e.categories().unwrap().len(), 8);
    assert_eq!(e.sessions().unwrap().len(), 0);
}
#[test]
fn csv_formula_payloads_are_neutralized() {
    for text in [
        "=1+1",
        "+SUM(1,1)",
        "-1+2",
        "@SUM(1,1)",
        "\t=1+1",
        " \r\n=1+1",
    ] {
        assert!(csv_field(text).starts_with("\"'"), "{text:?}");
    }
    assert_eq!(csv_field("a,b\"c"), "\"a,b\"\"c\"");
}
#[test]
fn negative_history_timestamp_is_rejected() {
    let mut e = finished();
    let id = e.sessions().unwrap()[0].id;
    assert!(e.edit_session(id, 1, -60000, -30000, 30000).is_err());
}
#[test]
fn extreme_edit_does_not_panic() {
    let mut e = finished();
    let id = e.sessions().unwrap()[0].id;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.edit_session(id, 1, i64::MIN, i64::MAX, 1000)
    }));
    assert!(result.is_ok(), "extreme edit panicked");
    assert!(result.unwrap().is_err());
}
#[test]
fn extreme_backup_does_not_panic() {
    let mut b = finished().backup().unwrap();
    b.segments[0].start_ms = i64::MIN;
    b.segments[0].end_ms = i64::MAX;
    let result = std::panic::catch_unwind(|| validate_backup(&b));
    assert!(result.is_ok(), "extreme backup panicked");
    assert!(result.unwrap().is_err());
}
#[test]
fn unsupported_mode_kind_in_backup_is_rejected() {
    let mut b = finished().backup().unwrap();
    b.sessions[0].mode = "stopwatch".into();
    b.sessions[0].kind = "break".into();
    assert!(validate_backup(&b).is_err());
}
#[test]
fn malformed_backup_preserves_database() {
    let mut e = finished();
    let original = e.backup().unwrap();
    for variant in 0..6 {
        let mut b = original.clone();
        match variant {
            0 => b.version = 999,
            1 => b.sessions[0].elapsed_ms += 1,
            2 => b.segments[0].session_id = 999,
            3 => b.categories.push(b.categories[0].clone()),
            4 => b.sessions[0].status = "invalid".into(),
            _ => b.categories.clear(),
        };
        assert!(e.restore(b, Stamp::now()).is_err());
        assert_eq!(
            e.sessions().unwrap()[0].elapsed_ms,
            original.sessions[0].elapsed_ms
        );
        assert_eq!(
            e.db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
    }
}
#[test]
fn invalid_category_colors_are_rejected() {
    let e = engine();
    for color in ["red", "#abc", "#abcdef;url(x)", "#ééé"] {
        assert!(e.save_category(None, "Test".into(), color.into()).is_err());
    }
}
#[test]
fn invalid_foreign_key_edit_rolls_back() {
    let mut e = finished();
    let s = e.sessions().unwrap()[0].clone();
    assert!(e
        .edit_session(s.id, 999, s.started_at, s.ended_at.unwrap(), s.elapsed_ms)
        .is_err());
    assert_eq!(e.sessions().unwrap()[0].category_id, s.category_id);
}
#[test]
fn deeply_nested_json_is_rejected() {
    let p = temp().join("deep.json");
    std::fs::write(&p, format!("{}0{}", "[".repeat(256), "]".repeat(256))).unwrap();
    assert!(read_backup(&p).is_err());
}
#[test]
fn oversized_backup_is_rejected_before_read() {
    let p = temp().join("huge.json");
    let f = std::fs::File::create(&p).unwrap();
    f.set_len(100_000_001).unwrap();
    assert!(read_backup(&p).is_err());
}
#[test]
fn active_session_cannot_be_deleted_via_history() {
    let mut e = engine();
    e.start("stopwatch", "work", 1, 0, Stamp::now()).unwrap();
    let id = e.active().unwrap().unwrap().id;
    e.delete_session(id).unwrap();
    assert!(e.active().unwrap().is_some());
}

#[cfg(unix)]
#[test]
fn fifo_backup_is_rejected_without_waiting() {
    use std::ffi::CString;
    let p = temp().join("named-pipe.json");
    let c = CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let start = std::time::Instant::now();
    assert!(read_backup(&p).is_err());
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
}
#[cfg(unix)]
#[test]
fn backup_symlink_is_rejected() {
    let p = temp();
    let real = p.join("real.json");
    std::fs::write(
        &real,
        serde_json::to_vec(&engine().backup().unwrap()).unwrap(),
    )
    .unwrap();
    std::os::unix::fs::symlink(&real, p.join("link.json")).unwrap();
    assert!(read_backup(&p.join("link.json")).is_err());
}
#[test]
fn fuzz_backup_numbers_never_panic() {
    let original = finished().backup().unwrap();
    let extremes = [i64::MIN, -1, 0, 1, 1_000, i64::MAX, 4_102_444_800_001];
    let mut seed = 0x3141592653589793u64;
    for i in 0..20_000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = extremes[(seed as usize) % extremes.len()];
        let mut b = original.clone();
        match i % 9 {
            0 => b.segments[0].start_ms = n,
            1 => b.segments[0].end_ms = n,
            2 => b.segments[0].duration_ms = n,
            3 => b.sessions[0].started_at = n,
            4 => b.sessions[0].ended_at = Some(n),
            5 => b.sessions[0].elapsed_ms = n,
            6 => b.sessions[0].target_ms = n,
            7 => b.sessions[0].category_id = n,
            _ => b.segments[0].session_id = n,
        };
        assert!(
            std::panic::catch_unwind(|| validate_backup(&b)).is_ok(),
            "fuzz case {i}"
        );
    }
}
