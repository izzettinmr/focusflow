use focusflow_lib::core::*;
use rusqlite::Connection;
fn engine() -> Engine {
    let mut e = Engine::from_connection(Connection::open_in_memory().unwrap(), stamp(0)).unwrap();
    e.set_timing_policy(TimingPolicy::Mobile);
    e
}
fn stamp(ms: i64) -> Stamp {
    Stamp {
        wall: 1_792_670_400_000 + ms,
        mono: ms,
    }
}
#[test]
fn old_backups_default_to_enabled_audio_and_invalid_volume_is_atomic() {
    let mut e = engine();
    let mut backup = serde_json::to_value(e.backup().unwrap()).unwrap();
    backup["settings"]
        .as_object_mut()
        .unwrap()
        .remove("sound_enabled");
    backup["settings"]
        .as_object_mut()
        .unwrap()
        .remove("sound_volume");
    e.restore(serde_json::from_value(backup).unwrap(), stamp(0))
        .unwrap();
    assert!(e.settings().unwrap().sound_enabled);
    assert_eq!(e.settings().unwrap().sound_volume, 80);
    let mut settings = e.settings().unwrap();
    settings.sound_enabled = false;
    settings.sound_volume = 101;
    assert!(e.save_settings(settings).is_err());
    let mut backup = e.backup().unwrap();
    backup.settings.sound_volume = u32::MAX;
    assert!(e.restore(backup, stamp(0)).is_err());
    assert!(e.settings().unwrap().sound_enabled);
}
#[test]
fn completion_emits_once_regardless_of_repeated_checkpoints() {
    let mut e = engine();
    e.start("pomodoro", "work", 1, 1, stamp(0)).unwrap();
    assert_eq!(std::mem::take(&mut e.sounds), vec![SoundCue::Start]);
    assert!(e.start("pomodoro", "work", 1, 1, stamp(0)).is_err());
    assert!(e.sounds.is_empty());
    e.checkpoint(stamp(60_000)).unwrap();
    assert_eq!(std::mem::take(&mut e.sounds), vec![SoundCue::Complete]);
    e.checkpoint(stamp(60_001)).unwrap();
    e.checkpoint(stamp(90_000)).unwrap();
    assert!(e.sounds.is_empty());
    assert_eq!(e.sessions().unwrap()[0].elapsed_ms, 60_000);
}
#[test]
fn paused_time_resume_and_early_finish_do_not_emit_completion() {
    let mut e = engine();
    e.start("pomodoro", "work", 1, 1, stamp(0)).unwrap();
    e.sounds.clear();
    e.action("pause", stamp(1000)).unwrap();
    e.checkpoint(stamp(100_000)).unwrap();
    assert!(e.sounds.is_empty());
    e.action("resume", stamp(100_000)).unwrap();
    assert!(e.sounds.is_empty());
    e.action("finish", stamp(101_000)).unwrap();
    assert!(e.sounds.is_empty());
    assert_eq!(e.sessions().unwrap()[0].elapsed_ms, 2000);
}
#[test]
fn breaks_have_cues_stopwatch_cancel_restore_do_not_replay_them() {
    let mut e = engine();
    e.start("pomodoro", "break", 1, 1, stamp(0)).unwrap();
    assert_eq!(std::mem::take(&mut e.sounds), vec![SoundCue::Start]);
    e.checkpoint(stamp(60_000)).unwrap();
    assert_eq!(std::mem::take(&mut e.sounds), vec![SoundCue::Complete]);
    e.start("stopwatch", "work", 1, 0, stamp(61_000)).unwrap();
    e.action("cancel", stamp(62_000)).unwrap();
    assert!(e.sounds.is_empty());
    let backup = e.backup().unwrap();
    let mut other = engine();
    other.restore(backup, stamp(70_000)).unwrap();
    assert!(other.sounds.is_empty());
}
