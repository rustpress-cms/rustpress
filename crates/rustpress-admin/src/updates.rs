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

/// `wp_get_update_data()`: the counts and the title that sums them up.
pub fn update_data(site: &Site, counts: UpdateCounts) -> serde_json::Value {
    let mut titles = Vec::new();
    if counts.wordpress > 0 {
        titles.push(rustpress_core::i18n::sprintf(&site.tr("%d WordPress Update"), &[&counts.wordpress.to_string()]));
    }
    if counts.plugins > 0 {
        titles.push(rustpress_core::i18n::sprintf(&site.trn("%d Plugin Update", "%d Plugin Updates", counts.plugins as i64), &[&counts.plugins.to_string()]));
    }
    if counts.themes > 0 {
        titles.push(rustpress_core::i18n::sprintf(&site.trn("%d Theme Update", "%d Theme Updates", counts.themes as i64), &[&counts.themes.to_string()]));
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
    let Value::Array(events) = rustpress_phpser::unserialize(raw.as_bytes()).ok()? else { return None };
    events.iter().find_map(|(key, hooks)| match key {
        Key::Int(timestamp) if field(hooks, hook).and_then(|h| field(h, NO_ARGS)).is_some() => Some(*timestamp),
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
    let message = if now - next > 0 { "Automatic update overdue by %s. There may be a problem with WP-Cron." } else { "Automatic update scheduled in %s." };
    rustpress_core::i18n::sprintf(&ctx.tr(message), &[&diff])
}

/// The WordPress.org slugs the transient `update_plugins` knows (`response` and `no_update`), by plugin file.
pub async fn plugin_slugs(site: &Site) -> Vec<(String, String)> {
    let Some(plugins) = transient(site, "update_plugins").await else { return Vec::new() };
    let mut slugs = Vec::new();
    for list in ["response", "no_update"] {
        if let Some(Value::Array(items) | Value::Object { props: items, .. }) = field(&plugins, list) {
            for (key, entry) in items {
                if let (Key::Str(file), Some(slug)) = (key, field(entry, "slug").and_then(Value::as_str)) {
                    slugs.push((String::from_utf8_lossy(file).into_owned(), slug.to_owned()));
                }
            }
        }
    }
    slugs
}

/// The plugin files the transient `update_plugins` has an update for.
pub async fn plugin_updates(site: &Site) -> Vec<String> {
    let Some(plugins) = transient(site, "update_plugins").await else { return Vec::new() };
    match field(&plugins, "response") {
        Some(Value::Array(items)) | Some(Value::Object { props: items, .. }) => items.iter().filter_map(|(k, _)| if let Key::Str(s) = k { Some(String::from_utf8_lossy(s).into_owned()) } else { None }).collect(),
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
    let Some(themes) = transient(site, "update_themes").await else { return out };
    let Some(Value::Array(items)) = field(&themes, "response") else { return out };
    for (key, item) in items {
        let Key::Str(name) = key else { continue };
        let text = |n: &str| field(item, n).and_then(Value::as_str).unwrap_or_default().to_owned();
        out.insert(String::from_utf8_lossy(name).into_owned(), ThemeUpdate { new_version: text("new_version"), url: text("url"), package: text("package") });
    }
    out
}
