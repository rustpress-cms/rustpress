//! The plugins installed under `wp-content/plugins` and `mu-plugins` (`get_plugins()`, `get_mu_plugins()`).

use std::path::Path;

/// `get_file_data()`: the value of `Header: value` in the first 8 KiB of a file.
pub fn file_header(text: &str, header: &str) -> String {
    let lower = header.to_lowercase();
    for line in text.replace('\r', "\n").lines() {
        let trimmed = line.trim_start_matches(|c: char| matches!(c, ' ' | '\t' | '/' | '*' | '#' | '@'));
        let trimmed = trimmed.strip_prefix("<?php").map(str::trim_start).unwrap_or(trimmed);
        let Some((key, value)) = trimmed.split_once(':') else { continue };
        if key.trim().to_lowercase() == lower && key.len() == key.trim_end().len() {
            let value = value.split("*/").next().unwrap_or(value).split("?>").next().unwrap_or(value);
            return value.trim().to_owned();
        }
    }
    String::new()
}


#[derive(Debug, Clone, Default)]
pub struct PluginInfo {
    /// `akismet/akismet.php` or `hello.php`.
    pub file: String,
    pub name: String,
    pub plugin_uri: String,
    pub description: String,
    pub author: String,
    pub author_uri: String,
    pub version: String,
    pub text_domain: String,
    pub requires_wp: String,
    pub requires_php: String,
    /// `Network: true`.
    pub network: bool,
}

impl PluginInfo {
    /// The directory of the plugin, or the title of a single-file plugin as a slug.
    pub fn slug(&self) -> String {
        match self.file.split_once('/') {
            Some((dir, _)) => dir.to_owned(),
            None => crate::slug::sanitize_title(&self.name),
        }
    }
}

/// `wp_kses()` with the tags of plugin descriptions: links, `abbr`, `acronym`, `code`, `em` and `strong`.
pub fn kses_description(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            rest = "";
            break;
        };
        let tag = &rest[start + 1..start + end];
        rest = &rest[start + end + 1..];
        let closing = tag.starts_with('/');
        let body = tag.trim_start_matches('/').trim_end_matches('/').trim();
        let name = body.split_whitespace().next().unwrap_or_default().to_lowercase();
        let allowed: &[&str] = match name.as_str() {
            "a" => &["href", "title"],
            "abbr" | "acronym" => &["title"],
            "code" | "em" | "strong" => &[],
            _ => continue,
        };
        if closing {
            out.push_str(&format!("</{name}>"));
            continue;
        }
        out.push('<');
        out.push_str(&name);
        let attributes = body[name.len()..].trim();
        let mut remaining = attributes;
        while let Some(eq) = remaining.find('=') {
            let key = remaining[..eq].trim().to_lowercase();
            let after = remaining[eq + 1..].trim_start();
            let (value, tail) = match after.chars().next() {
                Some(q @ ('"' | '\'')) => {
                    let inner = &after[1..];
                    let close = inner.find(q).unwrap_or(inner.len());
                    (&inner[..close], inner.get(close + 1..).unwrap_or_default())
                }
                _ => {
                    let close = after.find(char::is_whitespace).unwrap_or(after.len());
                    (&after[..close], &after[close..])
                }
            };
            if allowed.contains(&key.as_str()) {
                out.push_str(&format!(" {key}=\"{}\"", value.replace('"', "&quot;")));
            }
            remaining = tail.trim_start();
        }
        out.push('>');
    }
    out.push_str(rest);
    out
}

fn read_plugin(path: &Path, file: String) -> Option<PluginInfo> {
    let bytes = std::fs::read(path).ok()?;
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]).into_owned();
    let name = file_header(&head, "Plugin Name");
    if name.is_empty() {
        return None;
    }
    Some(PluginInfo {
        file,
        name,
        plugin_uri: file_header(&head, "Plugin URI"),
        description: file_header(&head, "Description"),
        author: file_header(&head, "Author"),
        author_uri: file_header(&head, "Author URI"),
        version: file_header(&head, "Version"),
        text_domain: file_header(&head, "Text Domain"),
        requires_wp: file_header(&head, "Requires at least"),
        requires_php: file_header(&head, "Requires PHP"),
        network: file_header(&head, "Network").eq_ignore_ascii_case("true"),
    })
}

/// `get_plugins()`: the main files of the plugins, by name.
pub fn installed(wp_root: &Path) -> Vec<PluginInfo> {
    let dir = wp_root.join("wp-content").join("plugins");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else { return out };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_file() && name.ends_with(".php") && name != "index.php" {
            out.extend(read_plugin(&path, name));
        } else if path.is_dir()
            && let Ok(inner) = std::fs::read_dir(&path)
        {
            for file in inner.flatten() {
                let fname = file.file_name().to_string_lossy().into_owned();
                if file.path().is_file() && fname.ends_with(".php") {
                    out.extend(read_plugin(&file.path(), format!("{name}/{fname}")));
                }
            }
        }
    }
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}

/// `get_mu_plugins()`.
pub fn must_use(wp_root: &Path) -> Vec<PluginInfo> {
    let dir = wp_root.join("wp-content").join("mu-plugins");
    let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut out: Vec<PluginInfo> = entries
        .flatten()
        .filter(|e| e.path().is_file() && e.file_name().to_string_lossy().ends_with(".php"))
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            // A file without a header still counts: WordPress lists it under its file name.
            let info = read_plugin(&e.path(), file.clone());
            info.or(Some(PluginInfo { file: file.clone(), name: file, ..Default::default() }))
        })
        .collect();
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}
