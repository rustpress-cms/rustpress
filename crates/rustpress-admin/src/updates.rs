//! `wp_get_update_data()`: how many updates the stored update transients announce.

use rustpress_core::Site;
use rustpress_phpser::{Key, Value};

/// The counts of `wp_get_update_data()['counts']`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UpdateCounts {
    pub plugins: u64,
    pub themes: u64,
    pub wordpress: u64,
    pub translations: u64,
}

impl UpdateCounts {
    pub fn total(&self) -> u64 {
        self.plugins + self.themes + self.wordpress + self.translations
    }
}

/// A property of an object or an entry of an array.
fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    let props = match value {
        Value::Object { props, .. } => props,
        Value::Array(items) => items,
        _ => return None,
    };
    props.iter().find_map(|(k, v)| match k {
        Key::Str(s) if s == name.as_bytes() => Some(v),
        _ => None,
    })
}

fn entries(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        Some(Value::Object { props, .. }) => props.len(),
        _ => 0,
    }
}

async fn transient(site: &Site, name: &str) -> Option<Value> {
    let raw = site
        .options
        .fetch(&site.db, &format!("_site_transient_{name}"))
        .await
        .ok()
        .flatten()?;
    rustpress_phpser::unserialize(raw.as_bytes()).ok()
}

/// Updates of core (`response => upgrade`), plugins, themes and translations. With `wp_root`, the plugins count those
/// of [plugin_update_list].
pub async fn counts(site: &Site, wp_root: Option<&std::path::Path>) -> UpdateCounts {
    let mut counts = UpdateCounts::default();
    let mut translations = 0;
    if let Some(core) = transient(site, "update_core").await {
        if let Some(Value::Array(updates)) = field(&core, "updates")
            && updates
                .iter()
                .any(|(_, u)| field(u, "response").and_then(Value::as_str) == Some("upgrade"))
        {
            counts.wordpress = 1;
        }
        translations += entries(field(&core, "translations"));
    }
    if let Some(plugins) = transient(site, "update_plugins").await {
        counts.plugins = entries(field(&plugins, "response")) as u64;
        translations += entries(field(&plugins, "translations"));
    }
    if let Some(root) = wp_root
        && !rustpress_core::native::catalog().is_empty()
    {
        counts.plugins = plugin_update_list(site, Some(root)).await.len() as u64;
    }
    if let Some(themes) = transient(site, "update_themes").await {
        counts.themes = entries(field(&themes, "response")) as u64;
        translations += entries(field(&themes, "translations"));
    }
    counts.translations = u64::from(translations > 0);
    counts
}

/// `wp_get_update_data()`: the counts and the title that sums them up.
pub fn update_data(site: &Site, counts: UpdateCounts) -> serde_json::Value {
    let mut titles = Vec::new();
    if counts.wordpress > 0 {
        titles.push(rustpress_core::i18n::sprintf(
            &site.tr("%d WordPress Update"),
            &[&counts.wordpress.to_string()],
        ));
    }
    if counts.plugins > 0 {
        titles.push(rustpress_core::i18n::sprintf(
            &site.trn(
                "%d Plugin Update",
                "%d Plugin Updates",
                counts.plugins as i64,
            ),
            &[&counts.plugins.to_string()],
        ));
    }
    if counts.themes > 0 {
        titles.push(rustpress_core::i18n::sprintf(
            &site.trn("%d Theme Update", "%d Theme Updates", counts.themes as i64),
            &[&counts.themes.to_string()],
        ));
    }
    if counts.translations > 0 {
        titles.push(site.tr("Translation Updates"));
    }
    serde_json::json!({
        "counts": {"plugins": counts.plugins, "themes": counts.themes, "wordpress": counts.wordpress, "translations": counts.translations, "total": counts.total()},
        "title": rustpress_core::formatting::esc_attr(&titles.join(", ")),
    })
}

/// `wp_next_scheduled( $hook )` for an event without arguments: the first timestamp of the `cron` option that has it.
pub async fn next_scheduled(site: &Site, hook: &str) -> Option<i64> {
    // `md5( serialize( array() ) )`, the key of the arguments.
    const NO_ARGS: &str = "40cd750bba9870f18aada2478b24840a";
    let raw = site.options.fetch(&site.db, "cron").await.ok().flatten()?;
    let Value::Array(events) = rustpress_phpser::unserialize(raw.as_bytes()).ok()? else {
        return None;
    };
    events.iter().find_map(|(key, hooks)| match key {
        Key::Int(timestamp) if field(hooks, hook).and_then(|h| field(h, NO_ARGS)).is_some() => {
            Some(*timestamp)
        }
        _ => None,
    })
}

/// `wp_get_auto_update_message()`: when WP-Cron runs the next update check.
pub async fn auto_update_message(ctx: &crate::screens::Ctx<'_>) -> String {
    let Some(next) = next_scheduled(ctx.site(), "wp_version_check").await else {
        return ctx.tr("Automatic update not scheduled. There may be a problem with WP-Cron.");
    };
    let now = ctx.env.now;
    let diff = crate::screens::media_lib::human_time_diff(ctx, next, now);
    let message = if now - next > 0 {
        "Automatic update overdue by %s. There may be a problem with WP-Cron."
    } else {
        "Automatic update scheduled in %s."
    };
    rustpress_core::i18n::sprintf(&ctx.tr(message), &[&diff])
}

/// The WordPress.org slugs the transient `update_plugins` knows (`response` and `no_update`), by plugin file.
pub async fn plugin_slugs(site: &Site) -> Vec<(String, String)> {
    let Some(plugins) = transient(site, "update_plugins").await else {
        return Vec::new();
    };
    let mut slugs = Vec::new();
    for list in ["response", "no_update"] {
        if let Some(Value::Array(items) | Value::Object { props: items, .. }) =
            field(&plugins, list)
        {
            for (key, entry) in items {
                if let (Key::Str(file), Some(slug)) =
                    (key, field(entry, "slug").and_then(Value::as_str))
                {
                    slugs.push((String::from_utf8_lossy(file).into_owned(), slug.to_owned()));
                }
            }
        }
    }
    slugs
}

/// A plugin update the transient `update_plugins` announces.
#[derive(Debug, Clone, Default)]
pub struct PluginUpdate {
    pub slug: String,
    pub new_version: String,
    pub package: String,
}

/// The updates of the transient `update_plugins`, by plugin file. With `wp_root`, the plugins of the plugin directory of
/// RustPress get the update [directory_updates] offers instead of the one of WordPress.org (whose release no module
/// follows), or none: a deliberate difference from WordPress.
pub async fn plugin_update_list(
    site: &Site,
    wp_root: Option<&std::path::Path>,
) -> Vec<(String, PluginUpdate)> {
    let mut list = transient_plugin_updates(site).await;
    if let Some(root) = wp_root {
        let catalog = rustpress_core::native::catalog();
        list.retain(|(file, _)| !catalog.iter().any(|l| l.file == file));
        list.extend(directory_updates(root));
    }
    list
}

async fn transient_plugin_updates(site: &Site) -> Vec<(String, PluginUpdate)> {
    let Some(plugins) = transient(site, "update_plugins").await else {
        return Vec::new();
    };
    let Some(Value::Array(items) | Value::Object { props: items, .. }) =
        field(&plugins, "response")
    else {
        return Vec::new();
    };
    let text = |entry: &Value, name: &str| {
        field(entry, name)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    items
        .iter()
        .filter_map(|(key, entry)| match key {
            Key::Str(file) => Some((
                String::from_utf8_lossy(file).into_owned(),
                PluginUpdate {
                    slug: text(entry, "slug"),
                    new_version: text(entry, "new_version"),
                    package: text(entry, "package"),
                },
            )),
            _ => None,
        })
        .collect()
}

/// The plugin files the transient `update_plugins` has an update for.
pub async fn plugin_updates(site: &Site) -> Vec<String> {
    let Some(plugins) = transient(site, "update_plugins").await else {
        return Vec::new();
    };
    match field(&plugins, "response") {
        Some(Value::Array(items)) | Some(Value::Object { props: items, .. }) => items
            .iter()
            .filter_map(|(k, _)| {
                if let Key::Str(s) = k {
                    Some(String::from_utf8_lossy(s).into_owned())
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A theme update the transient `update_themes` announces.
#[derive(Debug, Clone, Default)]
pub struct ThemeUpdate {
    pub new_version: String,
    pub url: String,
    pub package: String,
}

/// The pending theme updates by stylesheet.
pub async fn theme_updates(site: &Site) -> std::collections::HashMap<String, ThemeUpdate> {
    let mut out = std::collections::HashMap::new();
    let Some(themes) = transient(site, "update_themes").await else {
        return out;
    };
    let Some(Value::Array(items)) = field(&themes, "response") else {
        return out;
    };
    for (key, item) in items {
        let Key::Str(name) = key else { continue };
        let text = |n: &str| {
            field(item, n)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        out.insert(
            String::from_utf8_lossy(name).into_owned(),
            ThemeUpdate {
                new_version: text("new_version"),
                url: text("url"),
                package: text("package"),
            },
        );
    }
    out
}

/// PHP's `version_compare()` of two versions (without an operator).
pub fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    /// `php_canonicalize_version()`.
    fn canonical(version: &str) -> String {
        let bytes = version.as_bytes();
        let Some(&first) = bytes.first() else {
            return String::new();
        };
        let not_digit = |c: u8| !c.is_ascii_digit() && c != b'.';
        let mut out = vec![first];
        let mut last = first;
        for &c in &bytes[1..] {
            if matches!(c, b'-' | b'_' | b'+') {
                if out.last() != Some(&b'.') {
                    out.push(b'.');
                }
            } else if (not_digit(last) && c.is_ascii_digit())
                || (last.is_ascii_digit() && not_digit(c))
            {
                if out.last() != Some(&b'.') {
                    out.push(b'.');
                }
                out.push(c);
            } else if !c.is_ascii_alphanumeric() {
                if out.last() != Some(&b'.') {
                    out.push(b'.');
                }
            } else {
                out.push(c);
            }
            last = c;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
    /// `compare_special_version_forms()`.
    fn special(a: &str, b: &str) -> Ordering {
        const FORMS: &[(&str, i32)] = &[
            ("dev", 0),
            ("alpha", 1),
            ("a", 1),
            ("beta", 2),
            ("b", 2),
            ("RC", 3),
            ("rc", 3),
            ("#", 4),
            ("pl", 5),
            ("p", 5),
        ];
        let order = |s: &str| {
            FORMS
                .iter()
                .find(|(name, _)| s.starts_with(name))
                .map_or(-6, |(_, n)| *n)
        };
        order(a).cmp(&order(b))
    }
    fn number(s: &str) -> i64 {
        s.bytes().take_while(u8::is_ascii_digit).fold(0i64, |n, d| {
            n.saturating_mul(10).saturating_add(i64::from(d - b'0'))
        })
    }
    let digit = |s: &str| s.as_bytes().first().is_some_and(u8::is_ascii_digit);
    let (a, b) = (canonical(a), canonical(b));
    if a.is_empty() || b.is_empty() {
        return match (a.is_empty(), b.is_empty()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Less,
            _ => Ordering::Greater,
        };
    }
    let (pa, pb): (Vec<&str>, Vec<&str>) = (a.split('.').collect(), b.split('.').collect());
    for (x, y) in pa.iter().zip(&pb) {
        let order = match (digit(x), digit(y)) {
            (true, true) => number(x).cmp(&number(y)),
            (false, false) => special(x, y),
            (true, false) => special("#N#", y),
            (false, true) => special(x, "#N#"),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    match pa.len().cmp(&pb.len()) {
        Ordering::Greater => {
            let rest = pa[pb.len()..].join(".");
            if digit(&rest) {
                Ordering::Greater
            } else {
                version_compare(&rest, "#N#")
            }
        }
        Ordering::Less => {
            let rest = pb[pa.len()..].join(".");
            if digit(&rest) {
                Ordering::Less
            } else {
                version_compare("#N#", &rest)
            }
        }
        Ordering::Equal => Ordering::Equal,
    }
}

/// The updates the plugin directory of RustPress offers (`rustpress_core::native::catalog()`): an installed plugin with
/// a native module that is older than the release its module follows, `version_compare( $installed, $module, '<' )`.
/// Only the main files of the listed plugins are read.
pub fn directory_updates(wp_root: &std::path::Path) -> Vec<(String, PluginUpdate)> {
    let plugins = wp_root.join("wp-content").join("plugins");
    rustpress_core::native::catalog()
        .iter()
        .filter_map(|listing| {
            let text = std::fs::read(plugins.join(listing.file)).ok()?;
            let head = &text[..text.len().min(8192)];
            let version =
                rustpress_core::plugins::file_header(&String::from_utf8_lossy(head), "Version");
            (version_compare(&version, listing.version) == std::cmp::Ordering::Less).then(|| {
                (
                    listing.file.to_owned(),
                    PluginUpdate {
                        slug: listing.slug.to_owned(),
                        new_version: listing.version.to_owned(),
                        package: listing.package.to_owned(),
                    },
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod version_tests {
    use super::version_compare;
    use std::cmp::Ordering::*;

    /// The answers of PHP 8's `version_compare()`.
    #[test]
    fn versions_compare_like_php() {
        for (a, b, order) in [
            ("2.6.1", "2.6.4", Less),
            ("6.2.4.1", "6.2.4", Greater),
            ("1.0", "1.0.0", Less),
            ("1.0rc1", "1.0", Less),
            ("1.0", "1.0pl1", Less),
            ("1.0-beta", "1.0-alpha", Greater),
            ("28.8", "28.10", Less),
            ("1.191.0", "1.191.0", Equal),
            ("", "1.0", Less),
            ("1.0.0", "1.0", Greater),
            ("1.0a", "1.0", Less),
        ] {
            assert_eq!(version_compare(a, b), order, "{a} {b}");
        }
    }
}
