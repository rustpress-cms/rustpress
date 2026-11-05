//! The files of WordPress that pages read on every request (`formatting.php`, `script-loader.php`, style sheets,
//! `block.json`, `theme.json`, patterns, the emoji loader): read once and kept. A file is looked at again at most every
//! two seconds and read again when its modification time or size changed, so editing a theme in development still
//! shows at once.
//!
//! A CGI process lives for one request and would read all of them again each time; on the network file systems of
//! shared hosting every `open()` asks the file server. With [`use_snapshot`] the contents are also kept in one file
//! between processes: a file whose modification time and size still match is taken from there, which costs a `stat()`
//! (usually answered from the kernel's attribute cache) instead of an `open()` and a `read()`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RECHECK: Duration = Duration::from_secs(2);

static STATS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static STAT_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static READS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static READ_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn count<T>(calls: &std::sync::atomic::AtomicU64, nanos: &std::sync::atomic::AtomicU64, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    nanos.fetch_add(start.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
    out
}

/// What the file system cost this process so far: (`stat()` calls, their seconds, files read from disk, their
/// seconds). For `Server-Timing` in CGI mode, where a network file system answers every call.
pub fn io_totals() -> (u64, f64, u64, f64) {
    use std::sync::atomic::Ordering::Relaxed;
    (STATS.load(Relaxed), STAT_NANOS.load(Relaxed) as f64 / 1e9, READS.load(Relaxed), READ_NANOS.load(Relaxed) as f64 / 1e9)
}

/// What a file holds: UTF-8 text, other bytes, or nothing readable.
#[derive(Clone)]
enum Content {
    Text(Arc<str>),
    Bytes(Arc<[u8]>),
    Unreadable,
}

impl Content {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        match String::from_utf8(bytes) {
            Ok(text) => Content::Text(Arc::from(text)),
            Err(e) => Content::Bytes(Arc::from(e.into_bytes())),
        }
    }
}

struct Entry {
    checked: Instant,
    modified: Option<SystemTime>,
    /// `None`: the file does not exist (or is no file).
    size: Option<u64>,
    /// Read at the first [`read`]; `None` until then.
    content: Option<Content>,
}

fn cache() -> &'static Mutex<HashMap<PathBuf, Entry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Entry>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The entry of `path`, looked at again when it is older than [`RECHECK`]; with `content`, its content read as well.
fn entry<R>(path: &Path, content: bool, f: impl FnOnce(&Entry) -> R) -> R {
    let now = Instant::now();
    let mut map = cache().lock().expect("file cache");
    let fresh = map.get(path).is_some_and(|e| now.duration_since(e.checked) < RECHECK);
    if !fresh {
        let meta = count(&STATS, &STAT_NANOS, || std::fs::metadata(path)).ok().filter(std::fs::Metadata::is_file);
        let modified = meta.as_ref().and_then(|m| m.modified().ok());
        let size = meta.as_ref().map(std::fs::Metadata::len);
        let kept = map.get_mut(path).filter(|old| old.modified == modified && modified.is_some() && old.size == size).and_then(|old| old.content.take());
        map.insert(path.to_path_buf(), Entry { checked: now, modified, size, content: kept });
    }
    let entry = map.get_mut(path).expect("entry just inserted");
    if content && entry.content.is_none() {
        entry.content = Some(match entry.size {
            None => Content::Unreadable,
            Some(size) => load(path, entry.modified, size),
        });
    }
    f(entry)
}

/// The text of a file (UTF-8), `None` if it is missing or not text.
pub fn read(path: &Path) -> Option<Arc<str>> {
    entry(path, true, |e| match &e.content {
        Some(Content::Text(text)) => Some(text.clone()),
        _ => None,
    })
}

/// The bytes of a file, `None` if it is missing.
pub fn read_bytes(path: &Path) -> Option<Arc<[u8]>> {
    entry(path, true, |e| match &e.content {
        Some(Content::Text(text)) => Some(Arc::from(text.as_bytes())),
        Some(Content::Bytes(bytes)) => Some(bytes.clone()),
        _ => None,
    })
}

/// [`read`] as an owned string, empty when the file is missing (`file_get_contents()` of a file that should be there).
pub fn read_string(path: &Path) -> String {
    read(path).map(|text| text.to_string()).unwrap_or_default()
}

/// The size of a file in bytes, `None` if it is missing.
pub fn size(path: &Path) -> Option<u64> {
    entry(path, false, |e| e.size)
}

/// Whether `path` is a file (`is_file()`, looked at again at most every two seconds).
pub fn is_file(path: &Path) -> bool {
    size(path).is_some()
}

/// The contents kept between CGI processes (see [`use_snapshot`]).
struct Snapshot {
    file: PathBuf,
    entries: HashMap<PathBuf, Kept>,
    /// Something was read from disk that the snapshot did not have.
    changed: bool,
}

struct Kept {
    modified: (u64, u32),
    size: u64,
    content: Content,
}

fn snapshot() -> &'static Mutex<Option<Snapshot>> {
    static SNAPSHOT: Mutex<Option<Snapshot>> = Mutex::new(None);
    &SNAPSHOT
}

fn stamp(modified: Option<SystemTime>) -> Option<(u64, u32)> {
    let since = modified?.duration_since(UNIX_EPOCH).ok()?;
    Some((since.as_secs(), since.subsec_nanos()))
}

/// The content of `path` (a file of `size` bytes): from the snapshot when it still matches, else from disk.
fn load(path: &Path, modified: Option<SystemTime>, size: u64) -> Content {
    let mut snapshot = snapshot().lock().expect("file snapshot");
    let stamp = stamp(modified);
    if let (Some(snapshot), Some(stamp)) = (snapshot.as_ref(), stamp)
        && let Some(kept) = snapshot.entries.get(path).filter(|k| k.modified == stamp && k.size == size)
    {
        return kept.content.clone();
    }
    let content = count(&READS, &READ_NANOS, || std::fs::read(path)).map_or(Content::Unreadable, Content::from_bytes);
    // The configuration with the database password never goes into the snapshot.
    let secret = path.file_name().is_some_and(|name| name == "wp-config.php");
    if let (Some(snapshot), Some(stamp)) = (snapshot.as_mut(), stamp)
        && !secret
        && !matches!(content, Content::Unreadable)
    {
        snapshot.entries.insert(path.to_path_buf(), Kept { modified: stamp, size, content: content.clone() });
        snapshot.changed = true;
    }
    content
}

const MAGIC: &[u8] = b"rustpress-files 1\n";

/// Keeps the contents of the files read in `file` for the next process (CGI). Reads what an earlier process kept; a
/// missing or damaged file just starts empty.
pub fn use_snapshot(file: PathBuf) {
    let entries = count(&READS, &READ_NANOS, || std::fs::read(&file)).ok().and_then(|data| decode(&data)).unwrap_or_default();
    *snapshot().lock().expect("file snapshot") = Some(Snapshot { file, entries, changed: false });
}

/// Writes the snapshot back if this process read files it did not have. The file is replaced atomically (written
/// beside it, then renamed), so a process starting meanwhile reads either the old or the new one. Errors (a webspace
/// that is not writable) only cost the next process the reads.
pub fn save_snapshot() {
    let mut guard = snapshot().lock().expect("file snapshot");
    let Some(snapshot) = guard.as_mut().filter(|s| s.changed) else { return };
    let data = encode(&snapshot.entries);
    let temp = snapshot.file.with_extension(format!("tmp{}", std::process::id()));
    let written = std::fs::File::create(&temp).and_then(|mut f| f.write_all(&data)).and_then(|()| std::fs::rename(&temp, &snapshot.file));
    if let Err(error) = written {
        let _ = std::fs::remove_file(&temp);
        tracing::debug!(%error, file = %snapshot.file.display(), "file snapshot not written");
    }
    snapshot.changed = false;
}

fn encode(entries: &HashMap<PathBuf, Kept>) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    let kept: Vec<(&str, &Kept)> = entries.iter().filter_map(|(path, kept)| Some((path.to_str()?, kept))).collect();
    out.extend((kept.len() as u64).to_le_bytes());
    for (path, kept) in kept {
        let (kind, bytes): (u8, &[u8]) = match &kept.content {
            Content::Text(text) => (1, text.as_bytes()),
            Content::Bytes(bytes) => (2, bytes),
            Content::Unreadable => continue,
        };
        out.extend((path.len() as u64).to_le_bytes());
        out.extend(path.as_bytes());
        out.extend(kept.modified.0.to_le_bytes());
        out.extend(kept.modified.1.to_le_bytes());
        out.extend(kept.size.to_le_bytes());
        out.push(kind);
        out.extend((bytes.len() as u64).to_le_bytes());
        out.extend(bytes);
    }
    out
}

fn decode(data: &[u8]) -> Option<HashMap<PathBuf, Kept>> {
    let mut cursor = std::io::Cursor::new(data.strip_prefix(MAGIC)?);
    let word = |cursor: &mut std::io::Cursor<&[u8]>| -> Option<u64> {
        let mut b = [0u8; 8];
        cursor.read_exact(&mut b).ok()?;
        Some(u64::from_le_bytes(b))
    };
    let count = word(&mut cursor)?;
    let mut entries = HashMap::new();
    for _ in 0..count {
        let len = usize::try_from(word(&mut cursor)?).ok()?;
        let mut path = vec![0u8; len.min(data.len())];
        cursor.read_exact(&mut path).ok()?;
        let secs = word(&mut cursor)?;
        let mut nanos = [0u8; 4];
        cursor.read_exact(&mut nanos).ok()?;
        let size = word(&mut cursor)?;
        let mut kind = [0u8; 1];
        cursor.read_exact(&mut kind).ok()?;
        let len = usize::try_from(word(&mut cursor)?).ok()?;
        let mut bytes = vec![0u8; len.min(data.len())];
        cursor.read_exact(&mut bytes).ok()?;
        let content = match kind[0] {
            1 => Content::Text(Arc::from(String::from_utf8(bytes).ok()?)),
            2 => Content::Bytes(Arc::from(bytes)),
            _ => return None,
        };
        entries.insert(PathBuf::from(String::from_utf8(path).ok()?), Kept { modified: (secs, u32::from_le_bytes(nanos)), size, content });
    }
    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trip() {
        let mut entries = HashMap::new();
        entries.insert(PathBuf::from("/a/theme.json"), Kept { modified: (1_700_000_000, 5), size: 2, content: Content::Text(Arc::from("{}")) });
        entries.insert(PathBuf::from("/a/de_DE.mo"), Kept { modified: (1, 0), size: 3, content: Content::Bytes(Arc::from(vec![0xde, 0x12, 0xff])) });
        let decoded = decode(&encode(&entries)).expect("decodes");
        assert_eq!(decoded.len(), 2);
        let theme = &decoded[Path::new("/a/theme.json")];
        assert_eq!((theme.modified, theme.size), ((1_700_000_000, 5), 2));
        assert!(matches!(&theme.content, Content::Text(t) if &**t == "{}"));
        assert!(matches!(&decoded[Path::new("/a/de_DE.mo")].content, Content::Bytes(b) if **b == [0xde, 0x12, 0xff]));
        assert!(decode(b"something else").is_none());
        let mut cut = encode(&entries);
        cut.truncate(cut.len() - 1);
        assert!(decode(&cut).is_none());
    }
}
