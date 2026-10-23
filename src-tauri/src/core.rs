//! Tek oturum, atomik kontrol noktaları ve takvim günlerine bölünen çalışma defteri.
use chrono::{Datelike, Local, TimeZone};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub archived: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub mode: String,
    pub kind: String,
    pub category_id: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub status: String,
    pub target_ms: i64,
    pub elapsed_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub session_id: i64,
    pub start_ms: i64,
    pub end_ms: i64,
    pub duration_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_language")]
    pub language: String,
    pub work_minutes: u32,
    pub short_minutes: u32,
    pub long_minutes: u32,
    pub goal_minutes: u32,
    pub long_every: u32,
}
fn default_language() -> String {
    "tr".into()
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            work_minutes: 25,
            short_minutes: 5,
            long_minutes: 15,
            goal_minutes: 240,
            long_every: 4,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayEntry {
    pub day: String,
    pub session_id: i64,
    pub duration_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub categories: Vec<Category>,
    pub sessions: Vec<Session>,
    pub days: Vec<DayEntry>,
    pub settings: Settings,
    pub recovery: bool,
    pub notice: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    pub version: u32,
    pub categories: Vec<Category>,
    pub sessions: Vec<Session>,
    pub segments: Vec<Segment>,
    pub settings: Settings,
}
#[derive(Debug, Clone, Copy)]
pub struct Stamp {
    pub wall: i64,
    pub mono: i64,
}
impl Stamp {
    pub fn now() -> Self {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // Telefonda ekran kapalıyken de çalışma sürer. Masaüstünde askı hariç tutulur.
        #[cfg(target_os = "android")]
        let clock = libc::CLOCK_BOOTTIME;
        #[cfg(not(target_os = "android"))]
        let clock = libc::CLOCK_MONOTONIC;
        unsafe {
            libc::clock_gettime(clock, &mut ts);
        }
        Self {
            wall: Local::now().timestamp_millis(),
            mono: ts.tv_sec * 1000 + ts.tv_nsec / 1_000_000,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimingPolicy {
    Desktop,
    // Only within the same living process, with a boot clock. Reopening still pauses.
    Mobile,
}
pub struct Engine {
    pub db: Connection,
    last: Stamp,
    pub recovery: bool,
    pub notice: Option<String>,
    pub notifications: Vec<String>,
    timing_policy: TimingPolicy,
}
impl Engine {
    pub fn open(path: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let f = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)
                .map_err(err)?;
            if !f.metadata().map_err(err)?.is_file() {
                return Err("Veritabanı normal bir dosya olmalı.".into());
            }
            f.set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(err)?;
        }
        let e = Self::from_connection(Connection::open(path).map_err(err)?, Stamp::now())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for suffix in ["-wal", "-shm"] {
                let sidecar = path.with_file_name(format!(
                    "{}{}",
                    path.file_name()
                        .ok_or("Geçersiz veritabanı yolu")?
                        .to_string_lossy(),
                    suffix
                ));
                if let Ok(meta) = std::fs::symlink_metadata(&sidecar) {
                    if meta.is_file() {
                        std::fs::set_permissions(sidecar, std::fs::Permissions::from_mode(0o600))
                            .map_err(err)?;
                    }
                }
            }
        }
        Ok(e)
    }
    pub fn from_connection(db: Connection, now: Stamp) -> Result<Self> {
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000;
        CREATE TABLE IF NOT EXISTS categories(id INTEGER PRIMARY KEY, name TEXT NOT NULL, color TEXT NOT NULL, archived INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE IF NOT EXISTS sessions(id INTEGER PRIMARY KEY, mode TEXT NOT NULL CHECK(mode IN ('pomodoro','stopwatch')), kind TEXT NOT NULL CHECK(kind IN ('work','break')), category_id INTEGER NOT NULL REFERENCES categories(id), started_at INTEGER NOT NULL, ended_at INTEGER, status TEXT NOT NULL CHECK(status IN ('running','paused','completed','early')), target_ms INTEGER NOT NULL, elapsed_ms INTEGER NOT NULL DEFAULT 0);
        CREATE UNIQUE INDEX IF NOT EXISTS one_active ON sessions((1)) WHERE status IN ('running','paused');
        CREATE TABLE IF NOT EXISTS segments(id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE, start_ms INTEGER NOT NULL, end_ms INTEGER NOT NULL, duration_ms INTEGER NOT NULL CHECK(duration_ms>0 AND end_ms>start_ms));
        CREATE INDEX IF NOT EXISTS segment_session ON segments(session_id);
        CREATE TABLE IF NOT EXISTS settings(id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);
        PRAGMA user_version=1;").map_err(err)?;
        let recovered = db
            .execute(
                "UPDATE sessions SET status='paused' WHERE status='running'",
                [],
            )
            .map_err(err)?
            > 0;
        if db
            .query_row("SELECT COUNT(*) FROM categories", [], |r| {
                r.get::<_, i64>(0)
            })
            .map_err(err)?
            == 0
        {
            for (i, name) in [
                "Physics I",
                "Physics II",
                "Linear Algebra",
                "Programming for Engineers",
                "Medical Biology",
                "English",
                "Other",
            ]
            .iter()
            .enumerate()
            {
                let colors = [
                    "#a99bff", "#7fb8f5", "#e9b86d", "#67d8bf", "#c69aff", "#f29cb8", "#9ba5ba",
                ];
                db.execute(
                    "INSERT INTO categories(name,color) VALUES(?1,?2)",
                    params![name, colors[i]],
                )
                .map_err(err)?;
            }
        }
        db.execute(
            "INSERT OR IGNORE INTO settings VALUES(1,?1)",
            [serde_json::to_string(&Settings::default()).map_err(err)?],
        )
        .map_err(err)?;
        crate::sync::ledger::init(&db)?;
        Ok(Self {
            db,
            last: now,
            recovery: recovered,
            notice: None,
            notifications: vec![],
            timing_policy: TimingPolicy::Desktop,
        })
    }
    pub fn set_timing_policy(&mut self, policy: TimingPolicy) {
        self.timing_policy = policy;
    }
    pub fn settings(&self) -> Result<Settings> {
        let raw: String = self
            .db
            .query_row("SELECT value FROM settings WHERE id=1", [], |r| r.get(0))
            .map_err(err)?;
        serde_json::from_str(&raw).map_err(err)
    }
    pub fn save_settings(&self, s: Settings) -> Result<()> {
        if !["tr", "en"].contains(&s.language.as_str())
            || !(1..=1440).contains(&s.work_minutes)
            || !(1..=240).contains(&s.short_minutes)
            || !(1..=240).contains(&s.long_minutes)
            || !(1..=1440).contains(&s.goal_minutes)
            || !(1..=12).contains(&s.long_every)
        {
            return Err("Süre ayarları izin verilen aralıkta olmalı.".into());
        }
        self.db
            .execute(
                "UPDATE settings SET value=?1 WHERE id=1",
                [serde_json::to_string(&s).map_err(err)?],
            )
            .map_err(err)?;
        Ok(())
    }
    pub fn sessions(&self) -> Result<Vec<Session>> {
        let mut q=self.db.prepare("SELECT id,mode,kind,category_id,started_at,ended_at,status,target_ms,elapsed_ms FROM sessions ORDER BY started_at DESC,id DESC").map_err(err)?;
        let rows = q
            .query_map([], |r| {
                Ok(Session {
                    id: r.get(0)?,
                    mode: r.get(1)?,
                    kind: r.get(2)?,
                    category_id: r.get(3)?,
                    started_at: r.get(4)?,
                    ended_at: r.get(5)?,
                    status: r.get(6)?,
                    target_ms: r.get(7)?,
                    elapsed_ms: r.get(8)?,
                })
            })
            .map_err(err)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)
    }
    pub fn active(&self) -> Result<Option<Session>> {
        Ok(self
            .sessions()?
            .into_iter()
            .find(|s| s.status == "running" || s.status == "paused"))
    }
    pub fn categories(&self) -> Result<Vec<Category>> {
        let mut q = self
            .db
            .prepare("SELECT id,name,color,archived FROM categories ORDER BY id")
            .map_err(err)?;
        let rows = q
            .query_map([], |r| {
                Ok(Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                    archived: r.get(3)?,
                })
            })
            .map_err(err)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)
    }
    pub fn segments(&self) -> Result<Vec<Segment>> {
        let mut q = self
            .db
            .prepare(
                "SELECT session_id,start_ms,end_ms,duration_ms FROM segments ORDER BY start_ms,id",
            )
            .map_err(err)?;
        let rows = q
            .query_map([], |r| {
                Ok(Segment {
                    session_id: r.get(0)?,
                    start_ms: r.get(1)?,
                    end_ms: r.get(2)?,
                    duration_ms: r.get(3)?,
                })
            })
            .map_err(err)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(err)
    }
    pub fn checkpoint(&mut self, now: Stamp) -> Result<()> {
        let previous = self.last;
        let Some(s) = self.active()? else {
            self.last = now;
            return Ok(());
        };
        if s.status != "running" {
            self.last = now;
            return Ok(());
        }
        let dt = now.mono - previous.mono;
        let wall = now.wall - previous.wall;
        // Askı, saat sıçraması veya 10 saniyeden uzun işlem durması: belirsiz aralığı sayma.
        if dt < 0
            || (self.timing_policy == TimingPolicy::Desktop && dt > 10_000)
            || (wall - dt).abs() > 2500
            || wall < 0
        {
            self.db
                .execute("UPDATE sessions SET status='paused' WHERE id=?1", [s.id])
                .map_err(err)?;
            self.notice=Some("Uyku, saat değişikliği veya uzun kesinti algılandı. Oturum güvenli biçimde duraklatıldı; belirsiz süre eklenmedi.".into());
            self.last = now;
            return Ok(());
        }
        let accepted = if s.target_ms > 0 {
            dt.min((s.target_ms - s.elapsed_ms).max(0))
        } else {
            dt
        };
        let done = s.target_ms > 0 && s.elapsed_ms + accepted >= s.target_ms;
        let end = previous.wall + accepted;
        let before_today = self.work_today(previous.wall)?;
        let tx = self.db.transaction().map_err(err)?;
        if accepted > 0 {
            let merged=tx.execute("UPDATE segments SET end_ms=?1,duration_ms=duration_ms+?2 WHERE id=(SELECT id FROM segments WHERE session_id=?3 AND end_ms=?4 ORDER BY id DESC LIMIT 1)",params![end,accepted,s.id,previous.wall]).map_err(err)?;
            if merged == 0 {
                tx.execute("INSERT INTO segments(session_id,start_ms,end_ms,duration_ms) VALUES(?1,?2,?3,?4)",params![s.id,previous.wall,end,accepted]).map_err(err)?;
            }
        }
        tx.execute(
            "UPDATE sessions SET elapsed_ms=elapsed_ms+?1,status=?2,ended_at=?3 WHERE id=?4",
            params![
                accepted,
                if done { "completed" } else { "running" },
                if done { Some(end) } else { None },
                s.id
            ],
        )
        .map_err(err)?;
        tx.commit().map_err(err)?;
        self.last = Stamp {
            wall: previous.wall + dt,
            mono: now.mono,
        };
        if done {
            self.notifications.push(
                if s.kind == "break" {
                    "Mola bitti. Yeni bir odak oturumuna hazırsın."
                } else {
                    "Pomodoro tamamlandı. Emeğine sağlık! Bir mola ver."
                }
                .into(),
            );
        }
        let goal = self.settings()?.goal_minutes as i64 * 60_000;
        if s.kind == "work" && before_today < goal && self.work_today(now.wall)? >= goal {
            self.notifications
                .push("Günlük çalışma hedefine ulaştın!".into());
        }
        Ok(())
    }
    fn safe_anchor(&self, now: Stamp) -> Result<Stamp> {
        let latest: i64 = self
            .db
            .query_row("SELECT COALESCE(MAX(end_ms),0) FROM segments", [], |r| {
                r.get(0)
            })
            .map_err(err)?;
        if latest - now.wall > 2500 {
            return Err("Sistem saati son kayıttan geride. Çift sayımı önlemek için sistem saatini düzeltin ve tekrar deneyin.".into());
        }
        Ok(Stamp {
            wall: now.wall.max(latest),
            mono: now.mono,
        })
    }
    pub fn start(
        &mut self,
        mode: &str,
        kind: &str,
        category_id: i64,
        minutes: u32,
        now: Stamp,
    ) -> Result<()> {
        let now = self.safe_anchor(now)?;
        if self.active()?.is_some() {
            return Err("Önce mevcut oturumu bitirin veya iptal edin.".into());
        }
        if !["pomodoro", "stopwatch"].contains(&mode)
            || !["work", "break"].contains(&kind)
            || (mode == "stopwatch" && kind != "work")
            || (mode == "pomodoro" && !(1..=1440).contains(&minutes))
        {
            return Err("Geçersiz çalışma ayarı.".into());
        }
        let exists: bool = self
            .db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND archived=0)",
                [category_id],
                |r| r.get(0),
            )
            .map_err(err)?;
        if !exists {
            return Err("Geçerli bir ders seçin.".into());
        }
        self.db.execute("INSERT INTO sessions(mode,kind,category_id,started_at,status,target_ms) VALUES(?1,?2,?3,?4,'running',?5)",params![mode,kind,category_id,now.wall,if mode=="pomodoro" {minutes as i64*60_000}else{0}]).map_err(err)?;
        self.last = now;
        self.recovery = false;
        self.notice = None;
        Ok(())
    }
    pub fn action(&mut self, action: &str, now: Stamp) -> Result<()> {
        self.checkpoint(now)?;
        let Some(s) = self.active()? else {
            return Ok(());
        };
        match action {
            "pause" => {
                self.db
                    .execute("UPDATE sessions SET status='paused' WHERE id=?1", [s.id])
                    .map_err(err)?;
            }
            "resume" => {
                let now = self.safe_anchor(now)?;
                self.db
                    .execute("UPDATE sessions SET status='running' WHERE id=?1", [s.id])
                    .map_err(err)?;
                self.last = now;
                self.recovery = false;
                self.notice = None;
            }
            "finish" => {
                self.recovery = false;
                self.notice = None;
                self.db.execute("UPDATE sessions SET status='early',ended_at=?1 WHERE id=?2",params![now.wall.max(s.started_at).max(self.db.query_row("SELECT COALESCE(MAX(end_ms),0) FROM segments WHERE session_id=?1",[s.id],|r|r.get::<_,i64>(0)).map_err(err)?),s.id]).map_err(err)?;
            }
            "cancel" => {
                self.recovery = false;
                self.notice = None;
                self.db
                    .execute("DELETE FROM sessions WHERE id=?1", [s.id])
                    .map_err(err)?;
            }
            _ => return Err("Bilinmeyen işlem.".into()),
        }
        Ok(())
    }
    pub fn daily(&self) -> Result<Vec<DayEntry>> {
        let work: std::collections::HashSet<i64> = self
            .sessions()?
            .iter()
            .filter(|s| s.kind == "work")
            .map(|s| s.id)
            .collect();
        let mut map = BTreeMap::<(String, i64), i64>::new();
        for s in self.segments()? {
            if work.contains(&s.session_id) {
                for (day, ms) in split_days(&s)? {
                    *map.entry((day, s.session_id)).or_default() += ms;
                }
            }
        }
        Ok(map
            .into_iter()
            .map(|((day, session_id), duration_ms)| DayEntry {
                day,
                session_id,
                duration_ms,
            })
            .collect())
    }
    fn work_today(&self, now: i64) -> Result<i64> {
        let day = Local
            .timestamp_millis_opt(now)
            .single()
            .ok_or("Geçersiz tarih")?
            .format("%Y-%m-%d")
            .to_string();
        Ok(self
            .daily()?
            .iter()
            .filter(|d| d.day == day)
            .map(|d| d.duration_ms)
            .sum())
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        Ok(Snapshot {
            categories: self.categories()?,
            sessions: self.sessions()?,
            days: self.daily()?,
            settings: self.settings()?,
            recovery: self.recovery,
            notice: self.notice.clone(),
        })
    }
    pub fn save_category(&self, id: Option<i64>, name: String, color: String) -> Result<()> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 80
            || color.len() != 7
            || !color.starts_with('#')
            || !color[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Ders adı (1–80 karakter) ve renk gerekli.".into());
        }
        if let Some(id) = id {
            self.db
                .execute(
                    "UPDATE categories SET name=?1,color=?2 WHERE id=?3",
                    params![name, color, id],
                )
                .map_err(err)?;
        } else {
            self.db
                .execute(
                    "INSERT INTO categories(name,color) VALUES(?1,?2)",
                    params![name, color],
                )
                .map_err(err)?;
        }
        Ok(())
    }
    pub fn archive_category(&self, id: i64) -> Result<()> {
        if self.active()?.is_some_and(|s| s.category_id == id) {
            return Err("Aktif oturumun dersini silemezsiniz.".into());
        }
        let count: i64 = self
            .db
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE archived=0",
                [],
                |r| r.get(0),
            )
            .map_err(err)?;
        if count <= 1 {
            return Err("En az bir ders kalmalı.".into());
        }
        // Geçmiş kayıtların ders adını korumak için mantıksal silme.
        self.db
            .execute("UPDATE categories SET archived=1 WHERE id=?1", [id])
            .map_err(err)?;
        Ok(())
    }
    pub fn edit_session(
        &mut self,
        id: i64,
        category_id: i64,
        start: i64,
        end: i64,
        duration: i64,
    ) -> Result<()> {
        let session = self
            .sessions()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("Oturum bulunamadı")?;
        if ["running", "paused"].contains(&session.status.as_str()) {
            return Err("Aktif oturum düzenlenemez.".into());
        }
        let span = end.checked_sub(start).ok_or("Tarih aralığı geçersiz.")?;
        if start < 0
            || end <= start
            || duration <= 0
            || duration > span
            || end > Local::now().timestamp_millis() + 60_000
            || span > 31 * 86_400_000
        {
            return Err(
                "Tarih ve süre tutarsız. Çalışma süresi başlangıç/bitiş aralığını aşamaz.".into(),
            );
        }
        let overlap:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id!=?1 AND started_at<?3 AND COALESCE(ended_at,9223372036854775807)>?2)",params![id,start,end],|r|r.get(0)).map_err(err)?;
        if overlap {
            return Err("Bu zaman aralığı başka bir oturumla çakışıyor.".into());
        }
        let tx = self.db.transaction().map_err(err)?;
        tx.execute("UPDATE sessions SET category_id=?1,started_at=?2,ended_at=?3,elapsed_ms=?4 WHERE id=?5",params![category_id,start,end,duration,id]).map_err(err)?;
        // Sadece kategori değişmişse orijinal duraklamalar ve gün dağılımı korunur.
        if start != session.started_at
            || Some(end) != session.ended_at
            || duration != session.elapsed_ms
        {
            tx.execute("DELETE FROM segments WHERE session_id=?1", [id])
                .map_err(err)?;
            tx.execute(
                "INSERT INTO segments(session_id,start_ms,end_ms,duration_ms) VALUES(?1,?2,?3,?4)",
                params![id, start, end, duration],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)?;
        Ok(())
    }
    pub fn delete_session(&self, id: i64) -> Result<()> {
        let tx = self.db.unchecked_transaction().map_err(err)?;
        let removable:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sessions WHERE id=?1 AND status NOT IN ('running','paused'))",[id],|r|r.get(0)).map_err(err)?;
        if removable {
            crate::sync::ledger::record_deletion(&tx, id)?;
        }
        tx.execute(
            "DELETE FROM sessions WHERE id=?1 AND status NOT IN ('running','paused')",
            [id],
        )
        .map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(())
    }
    pub fn backup(&self) -> Result<Backup> {
        let b = Backup {
            version: 1,
            categories: self.categories()?,
            sessions: self.sessions()?,
            segments: self.segments()?,
            settings: self.settings()?,
        };
        validate_backup(&b)?;
        Ok(b)
    }
    pub fn restore(&mut self, b: Backup, now: Stamp) -> Result<()> {
        if self.active()?.is_some() {
            return Err("Geri yüklemeden önce aktif oturumu bitirin.".into());
        }
        validate_backup(&b)?;
        let tx = self.db.transaction().map_err(err)?;
        tx.execute_batch("DELETE FROM sync_items; DELETE FROM segments; DELETE FROM sessions; DELETE FROM categories;")
            .map_err(err)?;
        for c in &b.categories {
            tx.execute(
                "INSERT INTO categories VALUES(?1,?2,?3,?4)",
                params![c.id, c.name, c.color, c.archived],
            )
            .map_err(err)?;
        }
        for s in &b.sessions {
            tx.execute(
                "INSERT INTO sessions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    s.id,
                    s.mode,
                    s.kind,
                    s.category_id,
                    s.started_at,
                    s.ended_at,
                    if s.status == "running" {
                        "paused"
                    } else {
                        &s.status
                    },
                    s.target_ms,
                    s.elapsed_ms
                ],
            )
            .map_err(err)?;
        }
        for s in &b.segments {
            tx.execute(
                "INSERT INTO segments(session_id,start_ms,end_ms,duration_ms) VALUES(?1,?2,?3,?4)",
                params![s.session_id, s.start_ms, s.end_ms, s.duration_ms],
            )
            .map_err(err)?;
        }
        tx.execute(
            "UPDATE settings SET value=?1 WHERE id=1",
            [serde_json::to_string(&b.settings).map_err(err)?],
        )
        .map_err(err)?;
        tx.commit().map_err(err)?;
        self.last = now;
        self.recovery = true;
        Ok(())
    }
}
pub fn split_days(s: &Segment) -> Result<Vec<(String, i64)>> {
    if s.end_ms <= s.start_ms || s.duration_ms < 0 {
        return Err("Geçersiz süre parçası".into());
    }
    let span = s
        .end_ms
        .checked_sub(s.start_ms)
        .ok_or("Süre aralığı taşması")?;
    let mut cur = s.start_ms;
    let mut assigned = 0;
    let mut out = vec![];
    while cur < s.end_ms {
        let date = Local
            .timestamp_millis_opt(cur)
            .single()
            .ok_or("Geçersiz tarih")?
            .date_naive();
        let next = date.succ_opt().ok_or("Tarih aralığı aşıldı")?;
        let midnight = Local
            .with_ymd_and_hms(next.year(), next.month(), next.day(), 0, 0, 0)
            .earliest()
            .ok_or("Yerel gece yarısı çözülemedi")?
            .timestamp_millis();
        let end = midnight.min(s.end_ms);
        let cumulative = ((end - s.start_ms) as i128 * s.duration_ms as i128 / span as i128) as i64;
        out.push((date.to_string(), cumulative - assigned));
        assigned = cumulative;
        cur = end;
    }
    Ok(out)
}
pub fn validate_backup(b: &Backup) -> Result<()> {
    if b.version != 1
        || b.categories.is_empty()
        || !b.categories.iter().any(|c| !c.archived)
        || b.sessions.len() > 500_000
        || b.segments.len() > 5_000_000
    {
        return Err("Desteklenmeyen veya aşırı büyük yedek.".into());
    }
    let mut ids = std::collections::HashSet::new();
    for c in &b.categories {
        if !ids.insert(c.id)
            || c.name.trim().is_empty()
            || c.name.chars().count() > 80
            || c.color.len() != 7
            || !c.color.starts_with('#')
            || !c.color[1..].chars().all(|ch| ch.is_ascii_hexdigit())
        {
            return Err("Geçersiz ders kaydı.".into());
        }
    }
    let mut sessions = std::collections::HashMap::new();
    let mut active = 0;
    for s in &b.sessions {
        if sessions.insert(s.id, s).is_some()
            || !ids.contains(&s.category_id)
            || !["pomodoro", "stopwatch"].contains(&s.mode.as_str())
            || !["work", "break"].contains(&s.kind.as_str())
            || !["running", "paused", "early", "completed"].contains(&s.status.as_str())
            || (s.mode == "stopwatch" && (s.kind != "work" || s.target_ms != 0))
            || s.elapsed_ms < 0
            || s.target_ms < 0
            || s.started_at < 0
            || s.started_at > 4_102_444_800_000
        {
            return Err("Geçersiz oturum kaydı.".into());
        }
        if ["running", "paused"].contains(&s.status.as_str()) {
            active += 1;
            if s.ended_at.is_some() {
                return Err("Aktif oturum bitiş tarihi içeremez.".into());
            }
        } else if s
            .ended_at
            .is_none_or(|e| e < s.started_at || e > 4_102_444_800_000)
        {
            return Err("Geçersiz bitiş tarihi.".into());
        }
    }
    if active > 1 {
        return Err("Yedekte birden fazla aktif oturum var.".into());
    }
    let mut totals = BTreeMap::<i64, i64>::new();
    let mut ranges = vec![];
    for p in &b.segments {
        let s = sessions.get(&p.session_id).ok_or("Sahipsiz süre parçası")?;
        let span = p
            .end_ms
            .checked_sub(p.start_ms)
            .ok_or("Süre aralığı taşması")?;
        if p.duration_ms <= 0
            || p.end_ms <= p.start_ms
            || p.duration_ms > span
            || p.start_ms < s.started_at
            || p.end_ms > s.ended_at.unwrap_or(4_102_444_800_000)
            || span > 31 * 86_400_000
        {
            return Err("Geçersiz süre parçası.".into());
        }
        let entry = totals.entry(p.session_id).or_default();
        *entry = entry.checked_add(p.duration_ms).ok_or("Süre taşması")?;
        ranges.push((p.start_ms, p.end_ms));
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|w| w[0].1 > w[1].0) {
        return Err("Yedekte çakışan süreler var.".into());
    }
    for s in &b.sessions {
        if totals.get(&s.id).copied().unwrap_or(0) != s.elapsed_ms {
            return Err("Oturum ve süre defteri toplamı eşleşmiyor.".into());
        }
    }
    let s = &b.settings;
    if !["tr", "en"].contains(&s.language.as_str())
        || !(1..=1440).contains(&s.work_minutes)
        || !(1..=1440).contains(&s.goal_minutes)
        || !(1..=240).contains(&s.short_minutes)
        || !(1..=240).contains(&s.long_minutes)
        || !(1..=12).contains(&s.long_every)
    {
        return Err("Geçersiz yedek ayarları.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base() -> Stamp {
        Stamp {
            wall: Local
                .with_ymd_and_hms(2026, 10, 20, 12, 0, 0)
                .single()
                .unwrap()
                .timestamp_millis(),
            mono: 1_000_000,
        }
    }
    fn at(t: Stamp, ms: i64) -> Stamp {
        Stamp {
            wall: t.wall + ms,
            mono: t.mono + ms,
        }
    }
    fn engine() -> Engine {
        Engine::from_connection(Connection::open_in_memory().unwrap(), base()).unwrap()
    }
    #[test]
    fn mobile_screen_off_counts_once_and_pause_excludes_time() {
        let mut e = engine();
        e.set_timing_policy(TimingPolicy::Mobile);
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        let later = at(base(), 3_600_000);
        e.checkpoint(later).unwrap();
        e.checkpoint(later).unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 3_600_000);
        e.action("pause", later).unwrap();
        e.checkpoint(at(later, 3_600_000)).unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 3_600_000);
    }
    #[test]
    fn mobile_sleep_clamps_pomodoro_to_target_and_notifies_once() {
        let mut e = engine();
        e.set_timing_policy(TimingPolicy::Mobile);
        e.start("pomodoro", "work", 1, 25, base()).unwrap();
        e.checkpoint(at(base(), 3_600_000)).unwrap();
        e.checkpoint(at(base(), 3_601_000)).unwrap();
        let s = &e.sessions().unwrap()[0];
        assert_eq!(s.elapsed_ms, 25 * 60_000);
        assert_eq!(s.ended_at, Some(base().wall + 25 * 60_000));
        assert_eq!(s.status, "completed");
        assert_eq!(e.notifications.len(), 1);
    }
    #[test]
    fn mobile_clock_jump_still_pauses_and_reopen_does_not_add_offline_time() {
        let mut e = engine();
        e.set_timing_policy(TimingPolicy::Mobile);
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(Stamp {
            wall: base().wall + 100_000,
            mono: base().mono + 1000,
        })
        .unwrap();
        assert_eq!(e.active().unwrap().unwrap().status, "paused");
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 0);
        e.action("resume", at(base(), 100_000)).unwrap();
        e.checkpoint(at(base(), 105_000)).unwrap();
        let mut reopened = Engine::from_connection(e.db, at(base(), 500_000)).unwrap();
        reopened.set_timing_policy(TimingPolicy::Mobile);
        reopened.checkpoint(at(base(), 600_000)).unwrap();
        assert_eq!(reopened.active().unwrap().unwrap().status, "paused");
        assert_eq!(reopened.active().unwrap().unwrap().elapsed_ms, 5000);
    }
    fn advance(e: &mut Engine, t: &mut Stamp, ms: i64) {
        let mut left = ms;
        while left > 0 {
            let n = left.min(10_000);
            *t = at(*t, n);
            e.checkpoint(*t).unwrap();
            left -= n;
        }
    }
    #[test]
    fn shared_ledger_40_plus_35_plus_50_is_125_minutes() {
        let mut e = engine();
        let mut t = base();
        e.start("pomodoro", "work", 1, 40, t).unwrap();
        advance(&mut e, &mut t, 40 * 60_000);
        e.start("stopwatch", "work", 1, 0, t).unwrap();
        advance(&mut e, &mut t, 35 * 60_000);
        e.action("finish", t).unwrap();
        e.start("pomodoro", "work", 1, 50, t).unwrap();
        advance(&mut e, &mut t, 50 * 60_000);
        assert_eq!(
            e.daily()
                .unwrap()
                .iter()
                .map(|d| d.duration_ms)
                .sum::<i64>(),
            125 * 60_000
        );
        assert_eq!(e.sessions().unwrap().len(), 3);
        assert_eq!(e.segments().unwrap().len(), 3);
    }
    #[test]
    fn pause_time_excluded_and_repeated_commands_idempotent() {
        let mut e = engine();
        let t = base();
        e.start("stopwatch", "work", 1, 0, t).unwrap();
        e.action("pause", at(t, 5000)).unwrap();
        e.action("pause", at(t, 6000)).unwrap();
        e.action("resume", at(t, 60_000)).unwrap();
        e.action("resume", at(t, 60_000)).unwrap();
        e.action("finish", at(t, 65_000)).unwrap();
        e.action("finish", at(t, 65_000)).unwrap();
        assert_eq!(e.sessions().unwrap()[0].elapsed_ms, 10_000);
        assert_eq!(e.segments().unwrap().len(), 2);
    }
    #[test]
    fn only_one_active_even_paused_or_other_mode() {
        let mut e = engine();
        e.start("pomodoro", "work", 1, 25, base()).unwrap();
        assert!(e.start("stopwatch", "work", 1, 0, base()).is_err());
        e.action("pause", base()).unwrap();
        assert!(e.start("stopwatch", "work", 1, 0, base()).is_err());
        assert!(e.db.execute("INSERT INTO sessions(mode,kind,category_id,started_at,status,target_ms) VALUES('stopwatch','work',1,0,'running',0)",[]).is_err());
    }
    #[test]
    fn completed_pomodoro_exactly_caps_target() {
        let mut e = engine();
        let mut t = base();
        e.start("pomodoro", "work", 1, 1, t).unwrap();
        advance(&mut e, &mut t, 65_000);
        let s = &e.sessions().unwrap()[0];
        assert_eq!(s.elapsed_ms, 60_000);
        assert_eq!(s.status, "completed");
        assert_eq!(s.ended_at, Some(base().wall + 60_000));
        assert_eq!(e.notifications.len(), 1);
    }
    #[test]
    fn break_never_counts_as_work() {
        let mut e = engine();
        let mut t = base();
        e.start("pomodoro", "break", 1, 1, t).unwrap();
        advance(&mut e, &mut t, 60_000);
        assert!(e.daily().unwrap().is_empty());
        assert_eq!(e.sessions().unwrap()[0].elapsed_ms, 60_000);
    }
    #[test]
    fn early_finish_records_actual_duration() {
        let mut e = engine();
        e.start("pomodoro", "work", 1, 25, base()).unwrap();
        e.action("finish", at(base(), 3123)).unwrap();
        let s = &e.sessions().unwrap()[0];
        assert_eq!(s.elapsed_ms, 3123);
        assert_eq!(s.status, "early");
    }
    #[test]
    fn cancel_removes_session_and_segments_atomically() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(at(base(), 5000)).unwrap();
        e.action("cancel", at(base(), 6000)).unwrap();
        assert!(e.sessions().unwrap().is_empty());
        assert!(e.segments().unwrap().is_empty());
        assert!(e.daily().unwrap().is_empty());
    }
    #[test]
    fn reopen_keeps_checkpoint_and_recovers_paused() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(at(base(), 7000)).unwrap();
        let mut reopened = Engine::from_connection(e.db, at(base(), 86_400_000)).unwrap();
        assert!(reopened.recovery);
        assert_eq!(reopened.active().unwrap().unwrap().status, "paused");
        reopened.checkpoint(at(base(), 86_405_000)).unwrap();
        assert_eq!(reopened.active().unwrap().unwrap().elapsed_ms, 7000);
        reopened.action("resume", at(base(), 86_405_000)).unwrap();
        reopened.action("finish", at(base(), 86_407_000)).unwrap();
        assert_eq!(reopened.sessions().unwrap()[0].elapsed_ms, 9000);
    }
    #[test]
    fn suspend_excluded_and_session_paused() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(at(base(), 1000)).unwrap();
        e.checkpoint(Stamp {
            wall: base().wall + 3_601_000,
            mono: base().mono + 2000,
        })
        .unwrap();
        let a = e.active().unwrap().unwrap();
        assert_eq!(a.elapsed_ms, 1000);
        assert_eq!(a.status, "paused");
        assert!(e.notice.is_some());
    }
    #[test]
    fn backward_clock_jump_excluded() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(Stamp {
            wall: base().wall - 60_000,
            mono: base().mono + 1000,
        })
        .unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 0);
        assert_eq!(e.active().unwrap().unwrap().status, "paused");
    }
    #[test]
    fn long_process_stall_is_conservatively_paused() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(at(base(), 30_000)).unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 0);
        assert_eq!(e.active().unwrap().unwrap().status, "paused");
    }
    #[test]
    fn midnight_splits_exactly_and_preserves_total() {
        let start = Local
            .with_ymd_and_hms(2026, 10, 20, 23, 59, 55)
            .single()
            .unwrap()
            .timestamp_millis();
        let seg = Segment {
            session_id: 1,
            start_ms: start,
            end_ms: start + 15_000,
            duration_ms: 15_000,
        };
        let days = split_days(&seg).unwrap();
        assert_eq!(
            days,
            vec![("2026-10-20".into(), 5000), ("2026-10-21".into(), 10_000)]
        );
    }
    #[test]
    fn edited_duration_is_prorated_without_rounding_loss() {
        let start = Local
            .with_ymd_and_hms(2026, 10, 20, 23, 59, 55)
            .single()
            .unwrap()
            .timestamp_millis();
        let days = split_days(&Segment {
            session_id: 1,
            start_ms: start,
            end_ms: start + 15_000,
            duration_ms: 7001,
        })
        .unwrap();
        assert_eq!(days.iter().map(|d| d.1).sum::<i64>(), 7001);
        assert_eq!(days[0].1, 2333);
    }
    #[test]
    fn backup_roundtrip_and_invalid_backup_is_rejected() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.action("finish", at(base(), 5000)).unwrap();
        let backup = e.backup().unwrap();
        validate_backup(&backup).unwrap();
        let mut dest = engine();
        dest.restore(backup.clone(), base()).unwrap();
        assert_eq!(dest.sessions().unwrap()[0].elapsed_ms, 5000);
        let mut bad = backup.clone();
        bad.sessions[0].elapsed_ms += 1;
        assert!(dest.restore(bad, base()).is_err());
        assert_eq!(dest.sessions().unwrap()[0].elapsed_ms, 5000);
        let mut bad = backup;
        bad.segments.push(bad.segments[0].clone());
        assert!(validate_backup(&bad).is_err());
    }
    #[test]
    fn goal_notification_only_on_crossing() {
        let mut e = engine();
        let mut settings = e.settings().unwrap();
        settings.goal_minutes = 1;
        e.save_settings(settings).unwrap();
        let mut t = base();
        e.start("stopwatch", "work", 1, 0, t).unwrap();
        advance(&mut e, &mut t, 120_000);
        assert_eq!(
            e.notifications
                .iter()
                .filter(|s| s.contains("hedefine"))
                .count(),
            1
        );
    }
    #[test]
    fn category_deletion_preserves_history() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        assert!(e.archive_category(1).is_err());
        e.action("finish", at(base(), 1000)).unwrap();
        e.archive_category(1).unwrap();
        assert!(
            e.categories()
                .unwrap()
                .iter()
                .find(|c| c.id == 1)
                .unwrap()
                .archived
        );
        assert_eq!(e.sessions().unwrap()[0].category_id, 1);
        assert!(e
            .start("stopwatch", "work", 1, 0, at(base(), 2000))
            .is_err());
    }
    #[test]
    fn failed_checkpoint_rolls_back_and_retry_does_not_double_count() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.db.execute_batch("CREATE TRIGGER fail_update BEFORE UPDATE ON sessions BEGIN SELECT RAISE(ABORT, 'disk error'); END;").unwrap();
        assert!(e.checkpoint(at(base(), 1000)).is_err());
        assert!(e.segments().unwrap().is_empty());
        e.db.execute_batch("DROP TRIGGER fail_update").unwrap();
        e.checkpoint(at(base(), 2000)).unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 2000);
    }
    #[test]
    fn duplicate_tick_does_not_add_time() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.checkpoint(at(base(), 1234)).unwrap();
        e.checkpoint(at(base(), 1234)).unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 1234);
    }
    #[test]
    fn disk_persistence_survives_connection_close() {
        let path = std::env::temp_dir().join(format!("focusflow-test-{}.db", std::process::id()));
        {
            let mut e = Engine::open(&path).unwrap();
            let t = Stamp::now();
            e.start("stopwatch", "work", 1, 0, t).unwrap();
            e.checkpoint(at(t, 1234)).unwrap();
        }
        let e = Engine::open(&path).unwrap();
        assert_eq!(e.active().unwrap().unwrap().elapsed_ms, 1234);
        assert_eq!(e.active().unwrap().unwrap().status, "paused");
        drop(e);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn history_edit_rejects_overlapping_sessions() {
        let mut e = engine();
        e.start("stopwatch", "work", 1, 0, base()).unwrap();
        e.action("finish", at(base(), 1000)).unwrap();
        e.start("stopwatch", "work", 1, 0, at(base(), 2000))
            .unwrap();
        e.action("finish", at(base(), 3000)).unwrap();
        let sessions = e.sessions().unwrap();
        assert!(e
            .edit_session(sessions[1].id, 1, base().wall, base().wall + 2500, 2000)
            .is_err());
        e.edit_session(sessions[1].id, 2, base().wall, base().wall + 1000, 1000)
            .unwrap();
        assert_eq!(e.segments().unwrap()[0].duration_ms, 1000);
    }
    #[test]
    fn clock_jitter_keeps_backup_valid_and_never_overlaps() {
        let mut e = engine();
        let t = base();
        e.start("stopwatch", "work", 1, 0, t).unwrap();
        e.checkpoint(Stamp {
            wall: t.wall + 999,
            mono: t.mono + 1000,
        })
        .unwrap();
        e.action(
            "pause",
            Stamp {
                wall: t.wall + 1999,
                mono: t.mono + 2000,
            },
        )
        .unwrap();
        e.action(
            "resume",
            Stamp {
                wall: t.wall + 1999,
                mono: t.mono + 2000,
            },
        )
        .unwrap();
        e.action(
            "finish",
            Stamp {
                wall: t.wall + 2999,
                mono: t.mono + 3000,
            },
        )
        .unwrap();
        assert_eq!(e.sessions().unwrap()[0].elapsed_ms, 3000);
        e.backup().unwrap();
        assert!(e
            .start(
                "stopwatch",
                "work",
                1,
                0,
                Stamp {
                    wall: t.wall - 60_000,
                    mono: t.mono + 4000
                }
            )
            .is_err());
    }
}
