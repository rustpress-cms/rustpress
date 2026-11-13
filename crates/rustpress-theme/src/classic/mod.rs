//! Classic (PHP) themes, ported to Rust. A theme here is not interpreted: its templates are
//! written out in Rust, following the theme's PHP files, and compared with the real WordPress
//! (see the "Classic themes" section of PLAN.md). Its stylesheets, scripts and images are
//! served from its directory unchanged.

pub mod comments;
pub mod custom_header;
mod media_widgets;
pub mod menu;
pub mod seventeen;
pub mod tags;
pub mod twenty;
pub mod twentyone;
mod twentyone_icons;
mod twentytwenty_icons;
pub mod widgets;

use rustpress_blocks::{Env, Registry};
use rustpress_core::formatting::esc_url;
use rustpress_core::{QueryResult, Site};
use rustpress_phpser::Value;

use crate::Result;
use crate::document::Document;
use crate::theme::Theme;

/// Slugs of the classic themes that exist as Rust ports.
pub const PORTS: &[&str] = &["twentyseventeen", "twentytwenty", "twentytwentyone"];

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
        "twentytwentyone" => &["link", "aside", "gallery", "image", "quote", "status", "video", "audio", "chat"],
        // Twenty Twenty does not support post formats.
        "twentytwenty" => &[],
        _ => &["aside", "audio", "chat", "gallery", "image", "link", "quote", "status", "video"],
    }
}

/// `$content_width` a ported theme sets on `after_setup_theme`.
pub fn content_width(theme: &str) -> Option<u64> {
    match theme {
        "twentyseventeen" => Some(525),
        // `twenty_twenty_one_content_width()`.
        "twentytwentyone" => Some(750),
        // `twentytwenty_theme_support()`.
        "twentytwenty" => Some(580),
        _ => None,
    }
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
    if is_twentyone(theme) {
        return twentyone::add_body_classes(&ctx, classes).await;
    }
    if is_twentytwenty(theme) {
        return twenty::add_body_classes(&ctx, classes).await;
    }
    seventeen::add_body_classes(&ctx, classes).await
}

/// Whether the ported theme is Twenty Twenty-One.
pub fn is_twentyone(theme: &Theme) -> bool {
    theme.template == "twentytwentyone"
}

/// Whether the ported theme is Twenty Twenty.
pub fn is_twentytwenty(theme: &Theme) -> bool {
    theme.template == "twentytwenty"
}

/// Renders a request with the ported theme.
/// The preview of a legacy widget in a page of the theme.
pub async fn legacy_widget_preview(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry, id_base: &str, instance: Option<&Value>) -> Result<String> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    if is_twentyone(theme) {
        return twentyone::legacy_widget_preview(&ctx, registry, id_base, instance).await;
    }
    if is_twentytwenty(theme) {
        return twenty::legacy_widget_preview(&ctx, registry, id_base, instance).await;
    }
    seventeen::legacy_widget_preview(&ctx, registry, id_base, instance).await
}

/// The partials of a request of selective refresh (`handle_render_partials_request()`), each for its placements, and
/// the settings of the header video when the header is one of them (`export_header_video_settings()`).
pub async fn render_partials(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry, partials: &[(String, serde_json::Value)]) -> Result<(serde_json::Map<String, serde_json::Value>, Option<serde_json::Value>)> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    let mut contents = serde_json::Map::new();
    for (id, contexts) in partials {
        let placements: Vec<Option<&serde_json::Value>> = match contexts.as_array() {
            Some(list) if !list.is_empty() => list.iter().map(Some).collect(),
            // Without placements the partial is rendered once.
            _ => vec![None],
        };
        let mut rendered = Vec::new();
        let mut missing = false;
        for placement in placements {
            let partial = if is_twentyone(theme) {
                twentyone::render_partial(&ctx, id, placement).await?
            } else if is_twentytwenty(theme) {
                twenty::render_partial(&ctx, id, placement).await?
            } else {
                seventeen::render_partial(&ctx, id, placement).await?
            };
            match partial {
                Some(value) => rendered.push(value),
                None => missing = true,
            }
        }
        contents.insert(id.clone(), if missing { serde_json::Value::Null } else { serde_json::Value::Array(rendered) });
    }
    let header = if theme.template == "twentyseventeen" && partials.iter().any(|(id, _)| id == "custom_header") { Some(seventeen::header_video_export(&ctx).await?) } else { None };
    Ok((contents, header))
}

pub async fn render_document(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry, extras: &crate::document::Extras) -> Result<Document> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    if is_twentyone(theme) {
        return twentyone::render(&ctx, registry, extras).await;
    }
    if is_twentytwenty(theme) {
        return twenty::render(&ctx, registry, extras).await;
    }
    seventeen::render(&ctx, registry, extras).await
}

/// What `WP_Theme_JSON_Resolver::get_theme_data()` makes of the theme supports of a ported classic theme without a
/// `theme.json`: `WP_Theme_JSON::get_from_editor_settings( get_classic_theme_supports_block_editor_settings() )` and the
/// defaults it adds for classic themes.
pub fn theme_supports_json(template: &str, i18n: &rustpress_core::i18n::I18n, mods: &Value) -> Option<serde_json::Value> {
    use serde_json::json;
    // Twenty Seventeen was compared without this layer; it adds no presets.
    if template != "twentytwentyone" && template != "twentytwenty" {
        return None;
    }
    // `get_theme_support( 'disable-custom-colors' )` and the others are `false` when not supported, which `isset()`
    // accepts: custom colors, gradients and font sizes stay on.
    let mut settings = json!({
        "color": { "custom": true, "customGradient": true },
        "typography": { "customFontSize": true },
    });
    if template == "twentytwenty" {
        let (palette, sizes) = twenty::editor_presets(i18n, mods);
        settings["color"]["palette"] = palette;
        settings["typography"]["fontSizes"] = sizes;
    }
    if template == "twentytwentyone" {
        let t = |text: &str| rustpress_core::formatting::esc_html(&i18n.gettext(template, text));
        // `twenty_twenty_one_setup()`.
        let palette = [
            ("Black", "black", "#000000"),
            ("Dark gray", "dark-gray", "#28303D"),
            ("Gray", "gray", "#39414D"),
            ("Green", "green", "#D1E4DD"),
            ("Blue", "blue", "#D1DFE4"),
            ("Purple", "purple", "#D1D1E4"),
            ("Red", "red", "#E4D1D1"),
            ("Orange", "orange", "#E4DAD1"),
            ("Yellow", "yellow", "#EEEADD"),
            ("White", "white", "#FFFFFF"),
        ];
        let color = |slug: &str| palette.iter().find(|(_, s, _)| *s == slug).map_or("", |(_, _, c)| *c);
        let gradients = [
            ("Purple to yellow", "purple-to-yellow", "purple", "yellow"),
            ("Yellow to purple", "yellow-to-purple", "yellow", "purple"),
            ("Green to yellow", "green-to-yellow", "green", "yellow"),
            ("Yellow to green", "yellow-to-green", "yellow", "green"),
            ("Red to yellow", "red-to-yellow", "red", "yellow"),
            ("Yellow to red", "yellow-to-red", "yellow", "red"),
            ("Purple to red", "purple-to-red", "purple", "red"),
            ("Red to purple", "red-to-purple", "red", "purple"),
        ];
        let sizes = [
            ("Extra small", "XS", 16, "extra-small"),
            ("Small", "S", 18, "small"),
            ("Normal", "M", 20, "normal"),
            ("Large", "L", 24, "large"),
            ("Extra large", "XL", 40, "extra-large"),
            ("Huge", "XXL", 96, "huge"),
            ("Gigantic", "XXXL", 144, "gigantic"),
        ];
        settings["typography"]["lineHeight"] = json!(true);
        settings["spacing"] = json!({ "units": ["px", "em", "rem", "vh", "vw", "%"], "padding": true });
        settings["color"]["palette"] = palette.iter().map(|(name, slug, value)| json!({ "name": t(name), "slug": slug, "color": value })).collect();
        settings["color"]["gradients"] = gradients
            .iter()
            .map(|(name, slug, from, to)| json!({ "name": t(name), "gradient": format!("linear-gradient(160deg, {} 0%, {} 100%)", color(from), color(to)), "slug": slug }))
            .collect();
        settings["typography"]["fontSizes"] = sizes
            .iter()
            .map(|(name, short, size, slug)| {
                json!({ "name": t(name), "shortName": rustpress_core::formatting::esc_html(&i18n.pgettext(template, "Font size", short)), "size": format!("{size}px"), "slug": slug })
            })
            .collect();
    }
    let has = |settings: &serde_json::Value, section: &str, key: &str| settings[section].get(key).is_some();
    let (palette, gradients, sizes) = (has(&settings, "color", "palette"), has(&settings, "color", "gradients"), has(&settings, "typography", "fontSizes"));
    settings["color"]["defaultPalette"] = json!(!palette);
    settings["color"]["defaultGradients"] = json!(!gradients);
    settings["typography"]["defaultFontSizes"] = json!(!sizes);
    if settings.get("spacing").is_none() {
        settings["spacing"] = json!({});
    }
    settings["spacing"]["defaultSpacingSizes"] = json!(true);
    settings["shadow"] = json!({ "defaultPresets": false });
    // `add_theme_support( 'link-color' )`.
    if template == "twentytwentyone" {
        settings["color"]["link"] = json!(true);
    }
    Some(json!({ "version": 3, "settings": settings }))
}
