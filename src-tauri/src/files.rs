//! Yerel dosya sınırları: normal, sınırlı boyutlu yedekler ve özel atomik çıktılar.
use crate::core::{validate_backup, Backup};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};
const MAX_BACKUP_BYTES: u64 = 100_000_000;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
pub fn csv_field(s: &str) -> String {
    let safe = if s.trim_start().starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_owned()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Tam bir dosya yolu seçilmeli.".into());
    }
    let parent = path.parent().ok_or("Geçersiz hedef dizin")?;
    let name = path
        .file_name()
        .ok_or("Geçersiz dosya adı")?
        .to_string_lossy();
    let tmp = parent.join(format!(
        ".{name}.focusflow-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut f = options.open(&tmp).map_err(|e| e.to_string())?;
    let result = (|| {
        f.write_all(data)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result.map_err(|e| e.to_string())
}
pub fn write_export(path: &Path, data_dir: &Path, data: &[u8]) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Tam bir dosya yolu seçilmeli.".into());
    }
    let protected = std::fs::canonicalize(data_dir).map_err(|e| e.to_string())?;
    let parent = std::fs::canonicalize(path.parent().ok_or("Geçersiz hedef dizin")?)
        .map_err(|e| e.to_string())?;
    let resolved = parent.join(path.file_name().ok_or("Geçersiz dosya adı")?);
    // . ve .. bileşenleri, üst dizin bağları, SQLite yan dosyaları ve iç önbellekler.
    if resolved.starts_with(&protected)
        || std::fs::canonicalize(path).is_ok_and(|p| p.starts_with(&protected))
    {
        return Err("Uygulamanın iç veri dizinine dışa aktarılamaz. Başka bir dizin seçin.".into());
    }
    atomic_write(&resolved, data)
}
pub fn read_backup(path: &Path) -> Result<Backup, String> {
    let raw = read_regular(path, MAX_BACKUP_BYTES)?;
    let b: Backup = serde_json::from_slice(&raw).map_err(|e| format!("Yedek okunamadı: {e}"))?;
    validate_backup(&b)?;
    Ok(b)
}
pub fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    // FIFO ve cihazların açılışta veya okumada süresiz bekletmesini engelle.
    #[cfg(unix)]
    options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    let file = options.open(path).map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("Yalnızca normal JSON yedek dosyaları okunabilir.".into());
    }
    if meta.len() > limit {
        return Err("Dosya boyut sınırını aşıyor.".into());
    }
    let mut raw = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() as u64 > limit {
        return Err("Dosya boyut sınırını aşıyor.".into());
    }
    Ok(raw)
}
