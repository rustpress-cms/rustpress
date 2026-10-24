//! Classic (PHP) themes, ported to Rust. A theme here is not interpreted: its templates are
//! written out in Rust, following the theme's PHP files, and compared with the real WordPress
//! (see the "Klassische Themes" section of PLAN.md). Its stylesheets, scripts and images are
//! served from its directory unchanged.

pub mod menu;
pub mod seventeen;
pub mod tags;
pub mod widgets;

use rustpress_blocks::{Env, Registry};
use rustpress_core::{QueryResult, Site};
use rustpress_phpser::Value;

use crate::Result;
use crate::document::Document;
use crate::theme::Theme;

/// Slugs of the classic themes that exist as Rust ports.
const PORTS: &[&str] = &["twentyseventeen"];

/// Whether the active theme is a classic theme with a port (a theme without `theme.json`).
pub fn is_ported(theme: &Theme) -> bool {
    !theme.template_dir().join("theme.json").is_file() && PORTS.contains(&theme.template.as_str())
}

/// The sizes a ported theme registers with `add_image_size()`.
pub fn image_size(theme: &str, name: &str) -> Option<(u64, u64)> {
    match (theme, name) {
        ("twentyseventeen", "twentyseventeen-featured-image") => Some((2000, 1200)),
        ("twentyseventeen", "twentyseventeen-thumbnail-avatar") => Some((100, 100)),
        _ => None,
    }
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

/// Renders a request with the ported theme.
pub async fn render_document(theme: &Theme, site: &Site, q: &QueryResult, registry: &Registry) -> Result<Document> {
    let env = Env::new(site, q, theme, registry);
    let mods = menu::theme_mod(site, &theme.stylesheet);
    let ctx = Ctx { theme, site, q, env, mods };
    seventeen::render(&ctx, registry).await
}
