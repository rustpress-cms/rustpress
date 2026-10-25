//! A few headers of a theme's `style.css`.

use std::path::Path;

/// `Text Domain:` of the theme in `dir` (its directory name without one).
pub fn text_domain(dir: &Path) -> String {
    let bytes = std::fs::read(dir.join("style.css")).unwrap_or_default();
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]).into_owned();
    head.lines()
        .find_map(|line| {
            let line = line.trim().trim_start_matches(['*', '/', '#', '@']).trim();
            let (key, value) = line.split_once(':')?;
            key.trim().eq_ignore_ascii_case("Text Domain").then(|| value.trim().to_owned())
        })
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}
