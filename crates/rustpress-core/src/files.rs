//! The files of WordPress that pages read on every request (`formatting.php`, `script-loader.php`, style sheets,
//! the emoji loader): read once and kept. A file is looked at again at most every two seconds and read again when
//! its modification time changed, so editing a theme in development still shows at once.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

const RECHECK: Duration = Duration::from_secs(2);

struct Entry {
    checked: Instant,
    modified: Option<SystemTime>,
    /// `None`: the file does not exist (or cannot be read).
    text: Option<Arc<str>>,
    size: Option<u64>,
}

fn cache() -> &'static Mutex<HashMap<PathBuf, Entry>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Entry>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn entry<R>(path: &Path, f: impl FnOnce(&Entry) -> R) -> R {
    let now = Instant::now();
    {
        let map = cache().lock().expect("file cache");
        if let Some(entry) = map.get(path).filter(|e| now.duration_since(e.checked) < RECHECK) {
            return f(entry);
        }
    }
    let meta = std::fs::metadata(path).ok();
    let modified = meta.as_ref().and_then(|m| m.modified().ok());
    let mut map = cache().lock().expect("file cache");
    let fresh = match map.get(path) {
        Some(old) if old.modified == modified && modified.is_some() => Entry { checked: now, modified, text: old.text.clone(), size: old.size },
        _ => {
            let text = meta.as_ref().filter(|m| m.is_file()).and_then(|_| std::fs::read_to_string(path).ok()).map(Arc::from);
            Entry { checked: now, modified, text, size: meta.as_ref().filter(|m| m.is_file()).map(std::fs::Metadata::len) }
        }
    };
    let result = f(&fresh);
    map.insert(path.to_path_buf(), fresh);
    result
}

/// The text of a file (UTF-8), `None` if it is missing or not text.
pub fn read(path: &Path) -> Option<Arc<str>> {
    entry(path, |e| e.text.clone())
}

/// [`read`] as an owned string, empty when the file is missing (`file_get_contents()` of a file that should be there).
pub fn read_string(path: &Path) -> String {
    read(path).map(|text| text.to_string()).unwrap_or_default()
}

/// The size of a file in bytes, `None` if it is missing.
pub fn size(path: &Path) -> Option<u64> {
    entry(path, |e| e.size)
}

/// Whether `path` is a file (`is_file()`, looked at again at most every two seconds).
pub fn is_file(path: &Path) -> bool {
    size(path).is_some()
}
