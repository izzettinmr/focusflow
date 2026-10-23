use focusflow_lib::{
    core::*,
    sync::ledger::{self, Content},
};
use rusqlite::Connection;
fn engine() -> Engine {
    let mut e =
        Engine::from_connection(Connection::open_in_memory().unwrap(), Stamp::now()).unwrap();
    e.set_timing_policy(TimingPolicy::Mobile);
    e
}
fn stamp(offset: i64) -> Stamp {
    Stamp {
        wall: 1_792_670_400_000 + offset,
        mono: offset,
    }
}
fn finish(e: &mut Engine, offset: i64, duration: i64) {
    e.start("stopwatch", "work", 1, 0, stamp(offset)).unwrap();
    e.action("finish", stamp(offset + duration)).unwrap();
}
fn exchange(a: &mut Engine, b: &mut Engine) {
    let data = ledger::prepare(a).unwrap();
    ledger::apply(b, data).unwrap();
    let data = ledger::prepare(b).unwrap();
    ledger::apply(a, data).unwrap();
}
#[test]
fn two_devices_merge_once_and_repeat_without_double_counting() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 40 * 60_000);
    finish(&mut b, 41 * 60_000, 35 * 60_000);
    exchange(&mut a, &mut b);
    for _ in 0..4 {
        exchange(&mut a, &mut b);
    }
    for e in [&a, &b] {
        assert_eq!(e.sessions().unwrap().len(), 2);
        assert_eq!(e.categories().unwrap().len(), 7);
        assert_eq!(
            e.daily()
                .unwrap()
                .iter()
                .map(|d| d.duration_ms)
                .sum::<i64>(),
            75 * 60_000
        );
    }
}
#[test]
fn legacy_backup_on_both_devices_is_not_duplicated() {
    let mut a = engine();
    finish(&mut a, 0, 1200);
    let mut b = engine();
    b.restore(a.backup().unwrap(), stamp(5000)).unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(a.sessions().unwrap().len(), 1);
    assert_eq!(b.sessions().unwrap().len(), 1);
}
#[test]
fn active_sessions_do_not_leave_their_device() {
    let mut a = engine();
    let mut b = engine();
    a.start("stopwatch", "work", 1, 0, stamp(0)).unwrap();
    a.action("pause", stamp(1000)).unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(a.active().unwrap().unwrap().status, "paused");
    assert!(b.sessions().unwrap().is_empty());
}
#[test]
fn edits_and_deletions_propagate_without_resurrection() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 1000);
    exchange(&mut a, &mut b);
    let id = a.sessions().unwrap()[0].id;
    a.edit_session(id, 5, stamp(0).wall, stamp(1000).wall, 1000)
        .unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(b.sessions().unwrap()[0].category_id, 5);
    a.delete_session(id).unwrap();
    exchange(&mut a, &mut b);
    exchange(&mut a, &mut b);
    assert!(a.sessions().unwrap().is_empty());
    assert!(b.sessions().unwrap().is_empty());
}
#[test]
fn overlapping_remote_study_is_rejected_atomically() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 2000);
    finish(&mut b, 1000, 2000);
    let before = serde_json::to_value(b.backup().unwrap()).unwrap();
    let remote = ledger::prepare(&mut a).unwrap();
    assert!(ledger::apply(&mut b, remote)
        .unwrap_err()
        .contains("çakışan"));
    assert_eq!(before, serde_json::to_value(b.backup().unwrap()).unwrap());
}
#[test]
fn malformed_payload_does_not_change_local_study() {
    let mut a = engine();
    finish(&mut a, 0, 1000);
    let mut b = engine();
    let mut remote = ledger::prepare(&mut a).unwrap();
    if let Some(Content::Session { elapsed, .. }) = remote
        .records
        .iter_mut()
        .find(|r| r.kind == "session")
        .unwrap()
        .content
        .as_mut()
    {
        *elapsed = i64::MAX;
    }
    assert!(ledger::apply(&mut b, remote).is_err());
    assert!(b.sessions().unwrap().is_empty());
}
#[test]
fn category_changes_and_archiving_are_shared() {
    let mut a = engine();
    let mut b = engine();
    exchange(&mut a, &mut b);
    a.save_category(Some(1), "Physics notes".into(), "#123456".into())
        .unwrap();
    a.archive_category(2).unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(b.categories().unwrap()[0].name, "Physics notes");
    assert!(b.categories().unwrap()[1].archived);
}
#[test]
fn local_settings_are_preserved() {
    let mut a = engine();
    let mut b = engine();
    let mut s = b.settings().unwrap();
    s.goal_minutes = 60;
    b.save_settings(s).unwrap();
    exchange(&mut a, &mut b);
    assert_eq!(b.settings().unwrap().goal_minutes, 60);
    assert_eq!(a.settings().unwrap().goal_minutes, 240);
}
#[test]
fn deleting_then_reusing_a_local_id_does_not_reuse_remote_identity() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 1000);
    exchange(&mut a, &mut b);
    let id = a.sessions().unwrap()[0].id;
    a.delete_session(id).unwrap();
    finish(&mut a, 5000, 2000);
    exchange(&mut a, &mut b);
    exchange(&mut a, &mut b);
    for e in [&a, &b] {
        let records = e.sessions().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].elapsed_ms, 2000);
    }
}
#[test]
fn an_untouched_default_course_does_not_overwrite_a_customized_course_on_first_sync() {
    let mut a = engine();
    finish(&mut a, 0, 1000);
    let mut b = engine();
    b.restore(a.backup().unwrap(), stamp(3000)).unwrap();
    a.save_category(Some(1), "My Physics".into(), "#123456".into())
        .unwrap();
    exchange(&mut a, &mut b);
    for e in [&a, &b] {
        assert_eq!(e.categories().unwrap().len(), 7);
        assert_eq!(
            e.categories()
                .unwrap()
                .iter()
                .find(|c| c.id == 1)
                .unwrap()
                .name,
            "My Physics"
        );
        assert_eq!(e.sessions().unwrap().len(), 1);
    }
}
#[test]
fn a_lost_reply_can_be_retried_without_duplicating_a_committed_transfer() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 1000);
    finish(&mut b, 5000, 1000);
    let outgoing = ledger::prepare(&mut a).unwrap();
    ledger::apply(&mut b, outgoing).unwrap();
    // The reply is lost after the host commits; the client retransmits on reconnect.
    exchange(&mut a, &mut b);
    assert_eq!(a.sessions().unwrap().len(), 2);
    assert_eq!(b.sessions().unwrap().len(), 2);
}
#[test]
fn incoming_work_cannot_extend_into_a_local_active_timer() {
    let mut a = engine();
    let mut b = engine();
    a.start("stopwatch", "work", 1, 0, stamp(0)).unwrap();
    finish(&mut b, 1000, 1000);
    let incoming = ledger::prepare(&mut b).unwrap();
    assert!(ledger::apply(&mut a, incoming)
        .unwrap_err()
        .contains("aktif oturum"));
    assert_eq!(a.sessions().unwrap().len(), 1);
    assert_eq!(a.active().unwrap().unwrap().elapsed_ms, 0);
}
#[test]
fn future_dated_peer_history_is_rejected() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 1000);
    let mut incoming = ledger::prepare(&mut a).unwrap();
    let future = Stamp::now().wall + 120_000;
    if let Some(Content::Session {
        start,
        end,
        segments,
        ..
    }) = incoming
        .records
        .iter_mut()
        .find(|r| r.kind == "session")
        .unwrap()
        .content
        .as_mut()
    {
        *start = future;
        *end = future + 1000;
        segments[0].start = future;
        segments[0].end = future + 1000;
    }
    assert!(ledger::apply(&mut b, incoming)
        .unwrap_err()
        .contains("saati ileride"));
    assert!(b.sessions().unwrap().is_empty());
}
#[test]
fn every_accepted_session_field_is_persisted_consistently() {
    let mut a = engine();
    let mut b = engine();
    finish(&mut a, 0, 1000);
    exchange(&mut a, &mut b);
    let mut incoming = ledger::prepare(&mut a).unwrap();
    let record = incoming
        .records
        .iter_mut()
        .find(|r| r.kind == "session")
        .unwrap();
    record.version += 1;
    if let Some(Content::Session { mode, target, .. }) = record.content.as_mut() {
        *mode = "pomodoro".into();
        *target = 60_000;
    }
    ledger::apply(&mut b, incoming).unwrap();
    let verified = b.backup().unwrap();
    assert_eq!(verified.sessions[0].mode, "pomodoro");
    assert_eq!(verified.sessions[0].target_ms, 60_000);
}

// Documents a reproduced trust-boundary weakness; not a mitigation test.
#[test]
fn pentest_paired_peer_can_advance_clock_beyond_accepted_version_range() {
    let mut a = engine();
    let mut b = engine();
    let mut incoming = ledger::prepare(&mut a).unwrap();
    incoming.records[0].version = 4_102_444_800_000;
    ledger::apply(&mut b, incoming).unwrap();
    b.save_category(Some(1), "Local edit after attack".into(), "#123456".into())
        .unwrap();
    let outgoing = ledger::prepare(&mut b).unwrap();
    assert!(outgoing
        .records
        .iter()
        .any(|r| r.version > 4_102_444_800_000));
    assert!(ledger::apply(&mut a, outgoing).is_err());
}

#[test]
fn pentest_invalid_sync_metadata_is_rejected_atomically() {
    let mut source = engine();
    finish(&mut source, 0, 1000);
    let template = ledger::prepare(&mut source).unwrap();
    for case in 0..9 {
        let mut target = engine();
        let before = serde_json::to_value(target.backup().unwrap()).unwrap();
        let mut payload = template.clone();
        match case {
            0 => payload.protocol = u32::MAX,
            1 => payload.records[0].uid = "../../etc/passwd".into(),
            2 => payload.records[0].writer = "'; DROP TABLE sessions;--".into(),
            3 => payload.records[0].version = i64::MAX,
            4 => payload.records[0].version = -1,
            5 => payload.records[0].kind = "settings".into(),
            6 => payload.records.push(payload.records[0].clone()),
            7 => {
                payload.records[0].content = None;
                payload.records[0].version = Stamp::now().wall;
            }
            _ => payload.records[0].writer = "a".repeat(65),
        }
        assert!(ledger::apply(&mut target, payload).is_err(), "case {case}");
        assert_eq!(
            before,
            serde_json::to_value(target.backup().unwrap()).unwrap()
        );
        assert_eq!(
            target
                .db
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
    }
}
