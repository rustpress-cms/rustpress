//! `rustpress check`: tells whether RustPress can serve a site, before it takes the site over.
//!
//! It reads the active theme and plugins from the database and looks at the theme's files: a block theme
//! (`templates/index.html`, as `wp_is_block_theme()` checks) runs as it is, a classic theme only when it has a
//! Rust port, and a plugin only when a native module stands in for it. Nothing is written.

use std::fmt::Write;
use std::path::Path;

use crate::Config;

/// How well RustPress covers one part of the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    /// Runs as in WordPress.
    Yes,
    /// Runs, with a difference the site owner should know about.
    Note,
    /// Does not run; the site would look or behave differently.
    No,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub what: String,
    pub support: Support,
    pub detail: String,
}

/// Plugins that need no module because RustPress does their job itself or because they only matter to PHP.
const NOT_NEEDED: &[(&str, &str)] = &[
    ("wp-mail-smtp/wp_mail_smtp.php", "not needed: RustPress sends mail itself; set RUSTPRESS_SMTP_URL"),
    ("wp-super-cache/wp-cache.php", "not needed: a page cache for PHP"),
    ("w3-total-cache/w3-total-cache.php", "not needed: a page cache for PHP"),
    ("wp-fastest-cache/wpFastestCache.php", "not needed: a page cache for PHP"),
    ("litespeed-cache/litespeed-cache.php", "not needed: a page cache for PHP"),
    ("hello.php", "not needed: it only shows a line in wp-admin"),
];

/// Classic themes with a Rust port (keep in step with `rustpress_theme::classic`).
const PORTED_THEMES: &[&str] = &["twentyseventeen"];

/// Checks the theme, a child theme's parent and the active plugins of the site.
pub async fn check(config: &Config) -> Result<Vec<Item>, String> {
    rustpress_plugins::register();
    let db = rustpress_db::Db::connect(&config.database_url, &config.table_prefix).await.map_err(|e| format!("cannot connect to the database: {e}"))?;
    let options = db.options_named(&["template", "stylesheet", "active_plugins"]).await.map_err(|e| e.to_string())?;
    let option = |name: &str| options.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or_default();
    let mut items = Vec::new();

    if config.wp_config_path().and_then(|p| std::fs::read_to_string(p).ok()).is_some_and(|text| is_multisite(&text)) {
        items.push(Item { what: "Multisite".into(), support: Support::No, detail: "a network of sites is not supported".into() });
    }

    let themes = config.wp_root.join("wp-content/themes");
    let (template, stylesheet) = (option("template"), option("stylesheet"));
    items.push(theme_item(&themes, &template));
    if !stylesheet.is_empty() && stylesheet != template {
        let mut child = theme_item(&themes, &stylesheet);
        child.what = format!("Child theme {stylesheet}");
        items.push(child);
    }

    let active = rustpress_phpser::unserialize(option("active_plugins").as_bytes()).unwrap_or(rustpress_phpser::Value::Null);
    let files: Vec<String> = active.as_array().unwrap_or_default().iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect();
    if files.is_empty() {
        items.push(Item { what: "Plugins".into(), support: Support::Yes, detail: "none active".into() });
    }
    let plugins = config.wp_root.join("wp-content/plugins");
    for file in files {
        items.push(plugin_item(&plugins, &file));
    }
    Ok(items)
}

fn theme_item(themes: &Path, slug: &str) -> Item {
    let what = format!("Theme {slug}");
    let dir = themes.join(slug);
    if slug.is_empty() || !dir.is_dir() {
        return Item { what, support: Support::No, detail: format!("not found in {}", themes.display()) };
    }
    if dir.join("templates/index.html").is_file() || dir.join("block-templates/index.html").is_file() {
        return Item { what, support: Support::Yes, detail: "block theme, runs as it is".into() };
    }
    if PORTED_THEMES.contains(&slug) {
        return Item { what, support: Support::Yes, detail: "classic theme with a native port".into() };
    }
    Item { what, support: Support::No, detail: "classic PHP theme without a port; its PHP templates do not run".into() }
}

fn plugin_item(plugins: &Path, file: &str) -> Item {
    let installed = plugin_version(&plugins.join(file));
    let what = format!("Plugin {file}{}", installed.as_deref().map(|v| format!(" {v}")).unwrap_or_default());
    if let Some(module) = rustpress_core::native::registered().iter().find(|m| m.file() == file) {
        return match installed {
            Some(v) if v != module.version() => {
                Item { what, support: Support::Note, detail: format!("native module, written for version {}; differences are possible", module.version()) }
            }
            _ => Item { what, support: Support::Yes, detail: "native module".into() },
        };
    }
    if let Some((_, why)) = NOT_NEEDED.iter().find(|(f, _)| *f == file) {
        return Item { what, support: Support::Yes, detail: (*why).into() };
    }
    Item { what, support: Support::No, detail: "PHP plugin without a native module; RustPress ignores it".into() }
}

/// The `Version:` header of a plugin's main file, as `get_plugin_data()` reads it from the first 8 KiB.
fn plugin_version(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]).into_owned();
    head.lines().find_map(|line| {
        let line = line.trim_start_matches(|c: char| c.is_whitespace() || c == '*' || c == '#' || c == '@');
        line.strip_prefix("Version:").map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
    })
}

/// Whether `wp-config.php` turns on Multisite (`define( 'MULTISITE', true );`).
fn is_multisite(wp_config: &str) -> bool {
    wp_config.lines().map(str::trim).filter(|l| !l.starts_with("//") && !l.starts_with('#')).any(|l| {
        let l = l.replace(' ', "").to_ascii_lowercase();
        l.contains("define('multisite',true)") || l.contains("define(\"multisite\",true)")
    })
}

/// The text `rustpress check` prints, and whether everything runs.
pub fn render(items: &[Item]) -> (String, bool) {
    let mut out = String::new();
    for item in items {
        let mark = match item.support {
            Support::Yes => "ok  ",
            Support::Note => "note",
            Support::No => "no  ",
        };
        let _ = writeln!(out, "[{mark}] {}: {}", item.what, item.detail);
    }
    let missing = items.iter().filter(|i| i.support == Support::No).count();
    let runs = missing == 0;
    let _ = writeln!(out);
    if runs {
        let _ = writeln!(out, "Yes: RustPress can serve this site. Compare a few pages with WordPress before you switch over.");
    } else {
        let _ = writeln!(out, "Not yet: {missing} part(s) of this site do not run in RustPress. Tell us which ones you need:");
        let _ = writeln!(out, "https://github.com/rustpress-cms/rustpress/issues/new?template=port-request.yml");
    }
    (out, runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multisite_is_found_in_wp_config() {
        assert!(is_multisite("<?php\ndefine( 'MULTISITE', true );\n"));
        assert!(is_multisite("define(\"MULTISITE\",true);"));
        assert!(!is_multisite("// define( 'MULTISITE', true );\ndefine( 'WP_DEBUG', true );"));
        assert!(!is_multisite("define( 'MULTISITE', false );"));
    }

    #[test]
    fn themes_are_block_ported_or_unsupported() {
        let root = std::env::temp_dir().join(format!("rustpress-check-{}", std::process::id()));
        std::fs::create_dir_all(root.join("block/templates")).unwrap();
        std::fs::write(root.join("block/templates/index.html"), "").unwrap();
        std::fs::create_dir_all(root.join("twentyseventeen")).unwrap();
        std::fs::create_dir_all(root.join("astra")).unwrap();
        assert_eq!(theme_item(&root, "block").support, Support::Yes);
        assert_eq!(theme_item(&root, "twentyseventeen").support, Support::Yes);
        assert_eq!(theme_item(&root, "astra").support, Support::No);
        assert_eq!(theme_item(&root, "missing").support, Support::No);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn plugin_versions_come_from_the_header() {
        let root = std::env::temp_dir().join(format!("rustpress-check-plugin-{}", std::process::id()));
        std::fs::create_dir_all(root.join("x")).unwrap();
        std::fs::write(root.join("x/x.php"), "<?php\n/**\n * Plugin Name: X\n * Version: 1.2.3\n */\n").unwrap();
        assert_eq!(plugin_version(&root.join("x/x.php")).as_deref(), Some("1.2.3"));
        let item = plugin_item(&root, "x/x.php");
        assert_eq!((item.support, item.what.as_str()), (Support::No, "Plugin x/x.php 1.2.3"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_verdict_counts_what_does_not_run() {
        let ok = Item { what: "Theme a".into(), support: Support::Yes, detail: "block theme".into() };
        let no = Item { what: "Plugin b".into(), support: Support::No, detail: "no module".into() };
        assert!(render(std::slice::from_ref(&ok)).1);
        let (text, runs) = render(&[ok, no]);
        assert!(!runs && text.contains("[no  ] Plugin b: no module") && text.contains("1 part(s)"), "{text}");
    }
}
