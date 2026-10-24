//! Sidebars and widgets (`wp_get_sidebars_widgets()`, `dynamic_sidebar()` and the core widgets).

use rustpress_core::formatting::{esc_attr, esc_url, strip_tags};
use rustpress_core::{QueryResult, Site};
use rustpress_db::{Order, OrderBy, PostQuery};
use rustpress_phpser::Value;

use crate::Result;

/// How a theme registered a sidebar (`register_sidebar()`).
#[derive(Debug, Clone)]
pub struct Sidebar {
    pub id: &'static str,
    pub before_widget: &'static str,
    pub after_widget: &'static str,
    pub before_title: &'static str,
    pub after_title: &'static str,
}

/// `wp_get_sidebars_widgets()`: sidebar id -> widget ids.
pub fn sidebars_widgets(site: &Site) -> Vec<(String, Vec<String>)> {
    let Some(value) = site.options.value("sidebars_widgets") else { return Vec::new() };
    let Some(entries) = value.as_array() else { return Vec::new() };
    entries
        .iter()
        .filter_map(|(key, list)| {
            let rustpress_phpser::Key::Str(name) = key else { return None };
            let ids = list.as_array()?.iter().filter_map(|(_, id)| id.as_str().map(str::to_owned)).collect();
            Some((String::from_utf8_lossy(name).into_owned(), ids))
        })
        .collect()
}

/// `is_active_sidebar()`.
pub fn is_active_sidebar(site: &Site, id: &str) -> bool {
    sidebars_widgets(site).iter().any(|(name, ids)| name == id && !ids.is_empty())
}

/// The widget class name WordPress puts on the wrapper (`widget_ops['classname']`).
fn classname(base: &str) -> String {
    match base {
        "recent-posts" => "widget_recent_entries".into(),
        "recent-comments" => "widget_recent_comments".into(),
        "archives" => "widget_archive".into(),
        "nav_menu" => "widget_nav_menu".into(),
        "tag_cloud" => "widget_tag_cloud".into(),
        other => format!("widget_{}", other.replace('-', "_")),
    }
}

/// Splits `recent-posts-6` into `recent-posts` and `6`.
fn split_id(id: &str) -> Option<(&str, u64)> {
    let (base, number) = id.rsplit_once('-')?;
    Some((base, number.parse().ok()?))
}

/// The settings of one widget instance from the `widget_<base>` option.
fn instance(site: &Site, base: &str, number: u64) -> Option<Value> {
    let option = site.options.value(&format!("widget_{base}"))?;
    let entries = option.as_array()?;
    entries.iter().find_map(|(key, value)| match key {
        rustpress_phpser::Key::Int(n) if *n as u64 == number => Some(value.clone()),
        _ => None,
    })
}

fn setting_str(instance: &Value, key: &str) -> String {
    match instance.get(key) {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => String::new(),
    }
}

fn setting_int(instance: &Value, key: &str) -> i64 {
    instance.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// `dynamic_sidebar()`: the widgets of a sidebar, wrapped the way the theme registered it.
pub async fn dynamic_sidebar(site: &Site, q: &QueryResult, sidebar: &Sidebar) -> Result<String> {
    let mut out = String::new();
    let Some((_, ids)) = sidebars_widgets(site).into_iter().find(|(name, _)| name == sidebar.id) else { return Ok(out) };
    for id in ids {
        let Some((base, number)) = split_id(&id) else { continue };
        let Some(settings) = instance(site, base, number) else { continue };
        let before = sidebar.before_widget.replacen("%1$s", &id, 1).replacen("%2$s", &classname(base), 1);
        let body = match base {
            "recent-posts" => recent_posts(site, q, sidebar, &before, &settings).await?,
            _ => None,
        };
        if let Some(body) = body {
            out.push_str(&body);
        }
    }
    Ok(out)
}

/// `WP_Widget_Recent_Posts::widget()`.
async fn recent_posts(site: &Site, q: &QueryResult, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<Option<String>> {
    let default_title = site.tr("Recent Posts");
    let title = setting_str(settings, "title");
    let title = if title.is_empty() { default_title.clone() } else { title };
    let number = match setting_int(settings, "number") {
        n if n > 0 => n as u64,
        _ => 5,
    };
    let show_date = settings.get("show_date").is_some_and(|v| matches!(v, Value::Bool(true)) || v.as_i64() == Some(1));
    let query = PostQuery { post_types: vec!["post".into()], limit: Some(number), order_by: OrderBy::Date, order: Order::Desc, ..PostQuery::default() };
    let posts = site.db.posts(&query).await?;
    if posts.is_empty() {
        return Ok(None);
    }
    let queried_id = match &q.queried {
        rustpress_core::Queried::Post(p) => p.id,
        _ => 0,
    };
    let mut out = format!("\n\t\t{before}\n\t\t");
    if !title.is_empty() {
        out.push_str(&format!("{}{title}{}", sidebar.before_title, sidebar.after_title));
    }
    let plain = strip_tags(&title).trim().to_owned();
    let aria = if plain.is_empty() { default_title } else { plain };
    out.push_str(&format!("<nav aria-label=\"{}\">\n\t\t<ul>\n\t\t\t", esc_attr(&aria)));
    for post in &posts {
        let post_title = rustpress_blocks::content::the_title_for(site, post);
        let text = if post_title.is_empty() { site.tr("(no title)") } else { post_title };
        let current = if queried_id == post.id { " aria-current=\"page\"" } else { "" };
        let date = if show_date {
            let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
            format!("\t<span class=\"post-date\">{}</span>\n\t\t\t\t\t", site.date(&site.options.string("date_format", "F j, Y"), ts))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "\t\t\t\t\t\t\t\t<li>\n\t\t\t\t\t<a href=\"{}\"{current}>{text}</a>\n\t\t\t\t\t{date}\t\t\t\t</li>\n\t\t\t",
            esc_url(&site.permalink(post).await?)
        ));
    }
    out.push_str(&format!("\t\t</ul>\n\n\t\t</nav>{}", sidebar.after_widget));
    Ok(Some(out))
}
