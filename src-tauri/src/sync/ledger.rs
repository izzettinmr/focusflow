//! Stable identities and an atomic, idempotent merge of completed study records.
use crate::core::{validate_backup, Category, Engine, Segment, Session};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};

pub type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn random_id() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(err)?;
    Ok(hex(&bytes))
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn identity(prefix: &str, value: &str) -> String {
    format!("{prefix}{}", hex(&Sha256::digest(value.as_bytes())))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Slice {
    pub start: i64,
    pub end: i64,
    pub duration: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Content {
    Category {
        name: String,
        color: String,
        archived: bool,
    },
    Session {
        category: String,
        mode: String,
        kind: String,
        start: i64,
        end: i64,
        status: String,
        target: i64,
        elapsed: i64,
        segments: Vec<Slice>,
    },
}
impl Content {
    fn kind(&self) -> &str {
        match self {
            Self::Category { .. } => "category",
            Self::Session { .. } => "session",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub uid: String,
    pub kind: String,
    pub version: i64,
    pub writer: String,
    pub content: Option<Content>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub protocol: u32,
    pub records: Vec<Record>,
}
#[derive(Clone)]
struct Item {
    record: Record,
    local: Option<i64>,
}
pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS sync_state(id INTEGER PRIMARY KEY CHECK(id=1), device TEXT NOT NULL, clock INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS sync_items(uid TEXT PRIMARY KEY, kind TEXT NOT NULL, local_id INTEGER, version INTEGER NOT NULL, writer TEXT NOT NULL, data TEXT NOT NULL, UNIQUE(kind,local_id));").map_err(err)?;
    if db
        .query_row("SELECT COUNT(*) FROM sync_state", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(err)?
        == 0
    {
        db.execute("INSERT INTO sync_state VALUES(1,?1,0)", [random_id()?])
            .map_err(err)?;
    }
    Ok(())
}
fn items(db: &Connection) -> Result<BTreeMap<String, Item>> {
    let mut q = db
        .prepare("SELECT uid,kind,local_id,version,writer,data FROM sync_items")
        .map_err(err)?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(err)?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (uid, kind, local, version, writer, data) = row.map_err(err)?;
        out.insert(
            uid.clone(),
            Item {
                record: Record {
                    uid,
                    kind,
                    version,
                    writer,
                    content: serde_json::from_str(&data).map_err(err)?,
                },
                local,
            },
        );
    }
    Ok(out)
}
fn put(db: &Connection, item: &Item) -> Result<()> {
    let r = &item.record;
    db.execute("INSERT INTO sync_items VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(uid) DO UPDATE SET local_id=excluded.local_id,version=excluded.version,writer=excluded.writer,data=excluded.data WHERE sync_items.local_id IS NOT excluded.local_id OR sync_items.version!=excluded.version OR sync_items.writer!=excluded.writer OR sync_items.data!=excluded.data",params![r.uid,r.kind,item.local,r.version,r.writer,serde_json::to_string(&r.content).map_err(err)?]).map_err(err)?;
    Ok(())
}
/// Observe local edits before each exchange; active sessions never leave their device.
pub fn prepare(e: &mut Engine) -> Result<Bundle> {
    let mut map = items(&e.db)?;
    let (device, mut clock) =
        e.db.query_row("SELECT device,clock FROM sync_state WHERE id=1", [], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(err)?;
    let categories = e.categories()?;
    let sessions = e.sessions()?;
    let segments = e.segments()?;
    let local_uids: BTreeMap<(String, i64), String> = map
        .values()
        .filter_map(|i| {
            i.local
                .map(|id| ((i.record.kind.clone(), id), i.record.uid.clone()))
        })
        .collect();
    let mut pieces: BTreeMap<i64, Vec<Slice>> = BTreeMap::new();
    for p in &segments {
        pieces.entry(p.session_id).or_default().push(Slice {
            start: p.start_ms,
            end: p.end_ms,
            duration: p.duration_ms,
        });
    }
    let mut observed = Vec::new();
    let mut category_ids = BTreeMap::new();
    for c in categories {
        let uid = local_uids
            .get(&("category".into(), c.id))
            .cloned()
            .unwrap_or_else(|| {
                identity(
                    "c-",
                    &if (1..=7).contains(&c.id) {
                        format!("focusflow-default-{}", c.id)
                    } else {
                        c.name.trim().to_lowercase()
                    },
                )
            });
        let uid = if map.get(&uid).is_some_and(|i| i.local != Some(c.id)) {
            format!("c-{}", random_id()?)
        } else {
            uid
        };
        category_ids.insert(c.id, uid.clone());
        observed.push((
            uid,
            c.id,
            Content::Category {
                name: c.name,
                color: c.color,
                archived: c.archived,
            },
        ));
    }
    for s in &sessions {
        if !["completed", "early"].contains(&s.status.as_str()) {
            continue;
        }
        let content = Content::Session {
            category: category_ids
                .get(&s.category_id)
                .ok_or("Ders kimliği yok")?
                .clone(),
            mode: s.mode.clone(),
            kind: s.kind.clone(),
            start: s.started_at,
            end: s.ended_at.ok_or("Bitiş tarihi yok")?,
            status: s.status.clone(),
            target: s.target_ms,
            elapsed: s.elapsed_ms,
            segments: pieces.remove(&s.id).unwrap_or_default(),
        };
        let uid = local_uids
            .get(&("session".into(), s.id))
            .cloned()
            .unwrap_or(identity(
                "s-",
                &serde_json::to_string(&content).map_err(err)?,
            ));
        let uid = if map.get(&uid).is_some_and(|i| i.local != Some(s.id)) {
            format!("s-{}", random_id()?)
        } else {
            uid
        };
        observed.push((uid, s.id, content));
    }
    for (uid, id, content) in observed {
        if map
            .get(&uid)
            .is_some_and(|i| i.record.content.as_ref() == Some(&content))
        {
            continue;
        }
        let default_names = [
            "Physics I",
            "Physics II",
            "Linear Algebra",
            "Programming for Engineers",
            "Medical Biology",
            "English",
            "Other",
        ];
        let default_colors = [
            "#a99bff", "#7fb8f5", "#e9b86d", "#67d8bf", "#c69aff", "#f29cb8", "#9ba5ba",
        ];
        let initial_default = !map.contains_key(&uid)
            && (1..=7).contains(&id)
            && matches!(&content,Content::Category{name,color,archived:false} if name==default_names[id as usize-1]&&color==default_colors[id as usize-1]);
        let version = if initial_default {
            1
        } else {
            clock = clock
                .checked_add(1)
                .ok_or("Sürüm sayacı doldu")?
                .max(chrono::Local::now().timestamp_millis());
            clock
        };
        map.insert(
            uid.clone(),
            Item {
                record: Record {
                    uid,
                    kind: content.kind().into(),
                    version,
                    writer: device.clone(),
                    content: Some(content),
                },
                local: Some(id),
            },
        );
    }
    let session_ids: HashSet<i64> = sessions.iter().map(|s| s.id).collect();
    for item in map.values_mut() {
        let exists = match item.record.kind.as_str() {
            "category" => category_ids.keys().any(|id| Some(*id) == item.local),
            _ => item.local.is_some_and(|id| session_ids.contains(&id)),
        };
        if item.local.is_some() && !exists {
            clock = clock
                .checked_add(1)
                .ok_or("Sürüm sayacı doldu")?
                .max(chrono::Local::now().timestamp_millis());
            item.local = None;
            item.record.content = None;
            item.record.version = clock;
            item.record.writer = device.clone();
        }
    }
    let tx = e.db.transaction().map_err(err)?;
    for item in map.values() {
        put(&tx, item)?;
    }
    tx.execute(
        "UPDATE sync_state SET clock=?1 WHERE id=1 AND clock!=?1",
        [clock],
    )
    .map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(Bundle {
        protocol: 1,
        records: map.into_values().map(|i| i.record).collect(),
    })
}
fn valid_id(s: &str) -> bool {
    (16..=70).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_hexdigit() || c == b'-' || c == b's')
}
/// Validate a candidate combined ledger before touching any local study row.
pub fn apply(e: &mut Engine, remote: Bundle) -> Result<usize> {
    if remote.protocol != 1 || remote.records.len() > 200_000 {
        return Err("Desteklenmeyen veya çok büyük eşitleme verisi".into());
    }
    prepare(e)?;
    let mut map = items(&e.db)?;
    let mut seen = HashSet::new();
    let mut changed = 0;
    let mut changed_uids = HashSet::new();
    let active_start = e.active()?.map(|s| s.started_at);
    for r in remote.records {
        if !valid_id(&r.uid)
            || r.writer.len() != 64
            || !r.writer.bytes().all(|c| c.is_ascii_hexdigit())
            || !(1..=4_102_444_800_000).contains(&r.version)
            || !["category", "session"].contains(&r.kind.as_str())
            || r.content.as_ref().is_some_and(|c| c.kind() != r.kind)
            || !seen.insert(r.uid.clone())
        {
            return Err("Geçersiz eşitleme kaydı".into());
        }
        if let Some(old) = map.get(&r.uid) {
            if old.record.kind != r.kind {
                return Err("Kayıt türü değiştirilemez".into());
            }
            if (old.record.version, &old.record.writer) >= (r.version, &r.writer) {
                continue;
            }
            if old.record.content != r.content {
                changed += 1;
                changed_uids.insert(r.uid.clone());
            }
        } else if r.content.is_some() {
            changed += 1;
            changed_uids.insert(r.uid.clone());
        }
        if let Some(Content::Session { end, .. }) = &r.content {
            if *end > chrono::Local::now().timestamp_millis() + 60_000 {
                return Err(
                    "Diğer cihazın saati ileride; saatini düzeltip yeniden eşitleyin.".into(),
                );
            }
            if changed_uids.contains(&r.uid) && active_start.is_some_and(|start| *end > start) {
                return Err("Gelen kayıt aktif oturumun zaman aralığında. Önce aktif oturumu bitirip yeniden eşitleyin.".into());
            }
        }
        let local = map.get(&r.uid).and_then(|i| i.local);
        map.insert(r.uid.clone(), Item { record: r, local });
    }
    let mut backup = e.backup()?;
    let mut next_category = backup.categories.iter().map(|c| c.id).max().unwrap_or(0);
    let mut next_session = backup.sessions.iter().map(|s| s.id).max().unwrap_or(0);
    let mut categories: BTreeMap<i64, Category> =
        backup.categories.drain(..).map(|c| (c.id, c)).collect();
    let mut sessions: BTreeMap<i64, Session> =
        backup.sessions.drain(..).map(|s| (s.id, s)).collect();
    let mut pieces: BTreeMap<i64, Vec<Segment>> = BTreeMap::new();
    for p in backup.segments.drain(..) {
        pieces.entry(p.session_id).or_default().push(p);
    }
    let mut category_ids = BTreeMap::new();
    for (uid, item) in map.iter_mut().filter(|(_, i)| i.record.kind == "category") {
        if let Some(Content::Category {
            name,
            color,
            archived,
        }) = &item.record.content
        {
            let id = match item.local {
                Some(id) => id,
                None => {
                    next_category = next_category.checked_add(1).ok_or("Ders kimliği doldu")?;
                    next_category
                }
            };
            categories.insert(
                id,
                Category {
                    id,
                    name: name.clone(),
                    color: color.clone(),
                    archived: *archived,
                },
            );
            item.local = Some(id);
            category_ids.insert(uid.clone(), id);
        } else {
            return Err("Dersler silinmek yerine arşivlenmeli".into());
        }
    }
    for item in map.values_mut().filter(|i| i.record.kind == "session") {
        if let Some(id) = item.local {
            if sessions
                .get(&id)
                .is_some_and(|s| ["running", "paused"].contains(&s.status.as_str()))
            {
                return Err("Aktif oturum eşitlemeyle değiştirilemez".into());
            }
            sessions.remove(&id);
            pieces.remove(&id);
        }
        if let Some(Content::Session {
            category,
            mode,
            kind,
            start,
            end,
            status,
            target,
            elapsed,
            segments,
        }) = &item.record.content
        {
            if !["completed", "early"].contains(&status.as_str()) {
                return Err("Yalnızca bitmiş oturumlar eşitlenir".into());
            }
            let id = match item.local {
                Some(id) => id,
                None => {
                    next_session = next_session.checked_add(1).ok_or("Oturum kimliği doldu")?;
                    next_session
                }
            };
            sessions.insert(
                id,
                Session {
                    id,
                    category_id: *category_ids
                        .get(category)
                        .ok_or("Eşitlenen oturumun dersi bulunamadı")?,
                    mode: mode.clone(),
                    kind: kind.clone(),
                    started_at: *start,
                    ended_at: Some(*end),
                    status: status.clone(),
                    target_ms: *target,
                    elapsed_ms: *elapsed,
                },
            );
            pieces.insert(
                id,
                segments
                    .iter()
                    .map(|p| Segment {
                        session_id: id,
                        start_ms: p.start,
                        end_ms: p.end,
                        duration_ms: p.duration,
                    })
                    .collect(),
            );
            item.local = Some(id);
        } else {
            item.local = None;
        }
    }
    backup.categories = categories.into_values().collect();
    backup.sessions = sessions.into_values().collect();
    backup.segments = pieces.into_values().flatten().collect();
    // A deleted/archived course cannot orphan an active study, or remove the last course.
    if e.active()?.is_some_and(|s| {
        backup
            .categories
            .iter()
            .find(|c| c.id == s.category_id)
            .is_none_or(|c| c.archived)
    }) {
        return Err("Aktif oturumun dersi diğer cihazda arşivlenmiş. Önce oturumu bitirin.".into());
    }
    validate_backup(&backup).map_err(|why|format!("Eşitleme uygulanmadı: {why}. Aynı saatlerdeki çalışmaları Geçmiş ekranından düzeltip yeniden deneyin."))?;
    let max_clock = map.values().map(|i| i.record.version).max().unwrap_or(0);
    let session_rows: BTreeMap<i64, &Session> = backup.sessions.iter().map(|s| (s.id, s)).collect();
    let mut segment_rows: BTreeMap<i64, Vec<&Segment>> = BTreeMap::new();
    for p in &backup.segments {
        segment_rows.entry(p.session_id).or_default().push(p);
    }
    let tx = e.db.transaction().map_err(err)?;
    // Upsert instead of restore: active timer, settings and recovery state stay local.
    for c in backup.categories.iter().filter(|c| {
        map.values().any(|i| {
            i.record.kind == "category"
                && i.local == Some(c.id)
                && changed_uids.contains(&i.record.uid)
        })
    }) {
        tx.execute("INSERT INTO categories(id,name,color,archived) VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET name=excluded.name,color=excluded.color,archived=excluded.archived",params![c.id,c.name,c.color,c.archived]).map_err(err)?;
    }
    for item in map
        .values()
        .filter(|i| i.record.kind == "session" && changed_uids.contains(&i.record.uid))
    {
        if let Some(id) = item.local {
            let s = session_rows.get(&id).ok_or("Oturum eşlemesi kayıp")?;
            tx.execute("INSERT INTO sessions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET mode=excluded.mode,kind=excluded.kind,category_id=excluded.category_id,started_at=excluded.started_at,ended_at=excluded.ended_at,status=excluded.status,target_ms=excluded.target_ms,elapsed_ms=excluded.elapsed_ms",params![s.id,s.mode,s.kind,s.category_id,s.started_at,s.ended_at,s.status,s.target_ms,s.elapsed_ms]).map_err(err)?;
            tx.execute("DELETE FROM segments WHERE session_id=?1", [id])
                .map_err(err)?;
            for p in segment_rows.get(&id).into_iter().flatten() {
                tx.execute("INSERT INTO segments(session_id,start_ms,end_ms,duration_ms) VALUES(?1,?2,?3,?4)",params![id,p.start_ms,p.end_ms,p.duration_ms]).map_err(err)?;
            }
        } else if item.record.content.is_none() {
            // Use the previous mapping when an incoming tombstone clears local_id.
            tx.execute("DELETE FROM sessions WHERE id=(SELECT local_id FROM sync_items WHERE uid=?1) AND status NOT IN ('running','paused')",[&item.record.uid]).map_err(err)?;
        }
    }
    for item in map.values() {
        put(&tx, item)?;
    }
    tx.execute(
        "UPDATE sync_state SET clock=?1 WHERE id=1 AND clock<?1",
        [max_clock],
    )
    .map_err(err)?;
    tx.commit().map_err(err)?;
    Ok(changed)
}
/// Called in the same transaction as history deletion, before SQLite can reuse an ID.
pub fn record_deletion(db: &Connection, id: i64) -> Result<()> {
    let (device, clock) = db
        .query_row("SELECT device,clock FROM sync_state WHERE id=1", [], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(err)?;
    let version = clock
        .checked_add(1)
        .ok_or("Sürüm sayacı doldu")?
        .max(chrono::Local::now().timestamp_millis());
    db.execute("UPDATE sync_items SET local_id=NULL,data='null',version=?1,writer=?2 WHERE kind='session' AND local_id=?3",params![version,device,id]).map_err(err)?;
    db.execute("UPDATE sync_state SET clock=?1 WHERE id=1", [version])
        .map_err(err)?;
    Ok(())
}
