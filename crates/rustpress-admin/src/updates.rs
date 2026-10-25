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
    let raw = site.options.fetch(&site.db, &format!("_site_transient_{name}")).await.ok().flatten()?;
    rustpress_phpser::unserialize(raw.as_bytes()).ok()
}

/// Updates of core (`response => upgrade`), plugins, themes and translations.
pub async fn counts(site: &Site) -> UpdateCounts {
    let mut counts = UpdateCounts::default();
    let mut translations = 0;
    if let Some(core) = transient(site, "update_core").await {
        if let Some(Value::Array(updates)) = field(&core, "updates")
            && updates.iter().any(|(_, u)| field(u, "response").and_then(Value::as_str) == Some("upgrade"))
        {
            counts.wordpress = 1;
        }
        translations += entries(field(&core, "translations"));
    }
    if let Some(plugins) = transient(site, "update_plugins").await {
        counts.plugins = entries(field(&plugins, "response")) as u64;
        translations += entries(field(&plugins, "translations"));
    }
    if let Some(themes) = transient(site, "update_themes").await {
        counts.themes = entries(field(&themes, "response")) as u64;
        translations += entries(field(&themes, "translations"));
    }
    counts.translations = u64::from(translations > 0);
    counts
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
    let Some(themes) = transient(site, "update_themes").await else { return out };
    let Some(Value::Array(items)) = field(&themes, "response") else { return out };
    for (key, item) in items {
        let Key::Str(name) = key else { continue };
        let text = |n: &str| field(item, n).and_then(Value::as_str).unwrap_or_default().to_owned();
        out.insert(String::from_utf8_lossy(name).into_owned(), ThemeUpdate { new_version: text("new_version"), url: text("url"), package: text("package") });
    }
    out
}
