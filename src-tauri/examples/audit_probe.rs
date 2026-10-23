//! Yalnızca geçici veriler için: güvenlik ve sabit veri hacmiyle bellek yükü.
use focusflow_lib::{core::*, files::read_backup};
use std::path::Path;
fn stamp(t: Stamp, ms: i64) -> Stamp {
    Stamp {
        wall: t.wall + ms,
        mono: t.mono + ms,
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("--read") => {
            let r = read_backup(Path::new(&a[2]));
            println!("{:?}", r.map(|_| ()));
        }
        Some("--seed") => {
            let path = Path::new(&a[2]);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut e = Engine::open(path).unwrap();
            let count: usize = a.get(3).unwrap().parse().unwrap();
            let mut t = Stamp::now();
            t.wall -= 86_400_000;
            for i in 0..count {
                e.start(
                    if i % 2 == 0 { "pomodoro" } else { "stopwatch" },
                    "work",
                    (i % 7 + 1) as i64,
                    25,
                    t,
                )
                .unwrap();
                t = stamp(t, 1000);
                e.action("finish", t).unwrap();
                t = stamp(t, 1000);
                e.notifications.clear();
            }
            println!("{} geçici oturum oluşturuldu", count);
        }
        Some("--soak") => {
            let count: usize = a[2].parse().unwrap();
            let path = Path::new(&a[3]);
            let mut e = Engine::open(path).unwrap();
            let mut t = Stamp::now();
            t.wall -= 86_400_000;
            for i in 0..count {
                e.start(
                    if i % 2 == 0 { "pomodoro" } else { "stopwatch" },
                    "work",
                    1,
                    25,
                    t,
                )
                .unwrap();
                t = stamp(t, 1000);
                e.action("pause", t).unwrap();
                t = stamp(t, 1000);
                e.action("resume", t).unwrap();
                t = stamp(t, 1000);
                e.action("finish", t).unwrap();
                let id = e.sessions().unwrap()[0].id;
                let _ = e.snapshot().unwrap();
                e.delete_session(id).unwrap();
                e.notifications.clear();
                if i % 250 == 0 {
                    let status = std::fs::read_to_string("/proc/self/status").unwrap();
                    let rss = status
                        .lines()
                        .find(|l| l.starts_with("VmRSS:"))
                        .unwrap_or("");
                    println!("{{\"cycle\":{i},\"rss\":{rss:?}}}");
                }
            }
            assert!(e.sessions().unwrap().is_empty());
            assert!(e.segments().unwrap().is_empty());
            assert_eq!(
                e.db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
            println!("soak tamamlandı: {count} döngü; kayıt sayısı=0; SQLite=ok");
        }
        _ => panic!("--read PATH | --seed DB COUNT | --soak CYCLES DB"),
    }
}
