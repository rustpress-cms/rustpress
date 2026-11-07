//! Classic (PHP) themes, ported to Rust. A theme here is not interpreted: its templates are
//! written out in Rust, following the theme's PHP files, and compared with the real WordPress
//! (see the "Classic themes" section of PLAN.md). Its stylesheets, scripts and images are
//! served from its directory unchanged.

pub mod comments;
mod media_widgets;
pub mod menu;
pub mod seventeen;
pub mod tags;
pub mod widgets;

use rustpress_blocks::{Env, Registry};
use rustpress_core::formatting::esc_url;
use rustpress_core::{QueryResult, Site};
use rustpress_phpser::Value;

use crate::Result;
use crate::document::Document;
use crate::theme::Theme;

/// Slugs of the classic themes that exist as Rust ports.
const PORTS: &[&str] = &["twentyseventeen"];

/// Whether the active theme is a classic theme with a port (a theme without `theme.json`).
pub fn is_ported(theme: &Theme) -> bool {
    !rustpress_core::files::is_file(&theme.template_dir().join("theme.json")) && PORTS.contains(&theme.template.as_str())
}

/// `wp_site_icon()`.
pub async fn wp_site_icon(site: &Site) -> Result<String> {
    let id = site.options.u64("site_icon", 0);
    if id == 0 {
        return Ok(String::new());
    }
    let Some(post) = site.db.post(id).await? else { return Ok(String::new()) };
    let (meta, _) = rustpress_blocks::images::load_metadata(site, &post).await?;
    let Some(meta) = meta else { return Ok(String::new()) };
    let Some(file) = meta.file() else { return Ok(String::new()) };
    let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
    let base = rustpress_blocks::images::upload_baseurl(site);
    let sized = |size: u64| -> Option<String> {
        let name = meta.sizes()?.values().find_map(|s| (s.get("width").and_then(serde_json::Value::as_u64) == Some(size) && s.get("height").and_then(serde_json::Value::as_u64) == Some(size)).then(|| s.get("file").and_then(serde_json::Value::as_str).map(str::to_owned)).flatten())?;
        Some(if dir.is_empty() { format!("{base}/{name}") } else { format!("{base}/{dir}/{name}") })
    };
    let mut out = String::new();
    for (rel, size) in [("icon", 32u64), ("icon", 192), ("apple-touch-icon", 180)] {
        if let Some(url) = sized(size) {
            if rel == "icon" {
                out.push_str(&format!("<link rel=\"icon\" href=\"{}\" sizes=\"{size}x{size}\" />\n", esc_url(&url)));
            } else {
                out.push_str(&format!("<link rel=\"apple-touch-icon\" href=\"{}\" />\n", esc_url(&url)));
            }
        }
    }
    if let Some(url) = sized(270) {
        out.push_str(&format!("<meta name=\"msapplication-TileImage\" content=\"{}\" />\n", esc_url(&url)));
    }
    Ok(out)
}

/// `wp_get_custom_css()`.
pub async fn wp_get_custom_css(site: &Site, mods: &Value) -> Result<String> {
    let Some(id) = mods.get("custom_css_post_id").and_then(Value::as_i64).filter(|id| *id > 0) else { return Ok(String::new()) };
    let Some(post) = site.db.post(id as u64).await? else { return Ok(String::new()) };
    if post.post_type != "custom_css" || post.post_status != "publish" {
        return Ok(String::new());
    }
    Ok(post.post_content)
}

/// `get_theme_support( 'post-formats' )`, in the order of the theme.
pub fn post_formats(theme: &str) -> &'static [&'static str] {
    match theme {
        "twentyseventeen" => &["aside", "image", "video", "quote", "link", "gallery", "audio"],
        _ => &["aside", "audio", "chat", "gallery", "image", "link", "quote", "status", "video"],
    }
}

/// `$content_width` a ported theme sets on `after_setup_theme`.
pub fn content_width(theme: &str) -> Option<u64> {
    (theme == "twentyseventeen").then_some(525)
}

/// The names of the sizes a ported theme registers with `add_image_size()`, in its order.
pub fn image_size_names(theme: &str) -> Vec<&'static str> {
    rustpress_core::image_sizes::of_theme(theme).iter().map(|(name, ..)| *name).collect()
}

/// The sizes a ported theme registers with `add_image_size()`.
pub fn image_size(theme: &str, name: &str) -> Option<(u64, u64)> {
    rustpress_core::image_sizes::of_theme(theme).iter().find(|(size, ..)| *size == name).map(|(_, width, height, _)| (u64::from(*width), u64::from(*height)))
}

/// Everything a classic template needs.
pub struct Ctx<'a> {
    pub theme: &'a Theme,
    pub site: &'a Site,
    pub q: &'a QueryResult,
    pub env: Env<'a>,
    /// The theme's `theme_mods_<stylesheet>` option.
    pub mods: Value,
}

impl Ctx<'_> {
    /// `get_theme_mod()` as a string.
    pub fn mod_str(&self, name: &str) -> Option<String> {
        match self.mods.get(name)? {
            Value::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
            Value::Int(i) => Some(i.to_string()),
            _ => None,
        }
    }

    /// The path and query of the request, for menu items that link to the current URL.
    pub fn request_uri(&self) -> String {
        format!("{}{}", self.q.url_path, if self.q.raw_query.is_empty() { String::new() } else { format!("?{}", self.q.raw_query) })
    }
}

/// `body_class` of the ported theme for a page it does not render itself.
pub async fn add_body_classes(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry, classes: &mut Vec<String>) -> Result<()> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    seventeen::add_body_classes(&ctx, classes).await
}

/// Renders a request with the ported theme.
/// The preview of a legacy widget in a page of the theme.
pub async fn legacy_widget_preview(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry, id_base: &str, instance: Option<&Value>) -> Result<String> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    seventeen::legacy_widget_preview(&ctx, registry, id_base, instance).await
}

pub async fn render_document(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry, extras: &crate::document::Extras) -> Result<Document> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    seventeen::render(&ctx, registry, extras).await
}
