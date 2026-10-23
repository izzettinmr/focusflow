use focusflow_lib::{core::*, i18n::translate, sync::ledger};
use rusqlite::Connection;
fn engine() -> Engine {
    Engine::from_connection(Connection::open_in_memory().unwrap(), Stamp::now()).unwrap()
}
#[test]
fn legacy_settings_and_backups_default_to_turkish() {
    let mut e = engine();
    let mut old = serde_json::to_value(e.backup().unwrap()).unwrap();
    old["settings"].as_object_mut().unwrap().remove("language");
    let backup: Backup = serde_json::from_value(old).unwrap();
    assert_eq!(backup.settings.language, "tr");
    e.restore(backup, Stamp::now()).unwrap();
    assert_eq!(e.settings().unwrap().language, "tr");
}
#[test]
fn language_persists_in_sqlite_and_backup_but_not_wifi_sync() {
    let mut a = engine();
    let mut b = engine();
    let mut settings = a.settings().unwrap();
    settings.language = "en".into();
    a.save_settings(settings).unwrap();
    let raw: String =
        a.db.query_row("SELECT value FROM settings WHERE id=1", [], |r| r.get(0))
            .unwrap();
    assert_eq!(
        serde_json::from_str::<Settings>(&raw).unwrap().language,
        "en"
    );
    ledger::apply(&mut b, ledger::prepare(&mut a).unwrap()).unwrap();
    assert_eq!(b.settings().unwrap().language, "tr");
    b.restore(a.backup().unwrap(), Stamp::now()).unwrap();
    assert_eq!(b.settings().unwrap().language, "en");
}
#[test]
fn invalid_language_is_rejected_without_changing_settings() {
    let mut e = engine();
    let mut s = e.settings().unwrap();
    s.language = "arbitrary".into();
    assert!(e.save_settings(s).is_err());
    let mut backup = e.backup().unwrap();
    backup.settings.language = "invalid".into();
    assert!(e.restore(backup, Stamp::now()).is_err());
    assert_eq!(e.settings().unwrap().language, "tr");
}
#[test]
fn native_notifications_and_composed_errors_are_translated() {
    assert_eq!(
        translate("en", "Günlük çalışma hedefine ulaştın!"),
        "You reached your daily study goal!"
    );
    assert_eq!(
        translate("tr", "Günlük çalışma hedefine ulaştın!"),
        "Günlük çalışma hedefine ulaştın!"
    );
    assert_eq!(
        translate("en", "Wi-Fi portu açılamadı: address already in use"),
        "Could not open the Wi-Fi port: address already in use"
    );
    assert_eq!(translate("en", "Vazgeç"), "Cancel");
}
