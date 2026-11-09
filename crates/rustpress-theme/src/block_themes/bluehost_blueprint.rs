//! The theme classes of Bluehost Blueprint 1.0.2 (`inc/classes/`, loaded by `functions.php`).
//!
//! `Version::check_version()` only stores the theme version in the option `bluehost_blueprint_theme_version` (it has
//! no migrations), and `enqueue_editor_style()` concerns the editor; neither changes the website.

use std::path::Path;

use rustpress_core::native::Asset;

use super::{BlockThemePort, ScriptContext};
use crate::functions_php::BlockStylesheet;

pub struct BluehostBlueprint;

impl BlockThemePort for BluehostBlueprint {
    fn version(&self) -> &'static str {
        "1.0.2"
    }

    /// `Assets::enqueue_frontend_assets()`: `style.css` with the theme version and no `path`.
    fn style_handle(&self) -> Option<String> {
        Some("bluehost-blueprint-style".to_owned())
    }

    /// `Assets::enqueue_custom_block_styles()` on `init`: every `assets/block-styles/*.css` in `glob()` order (sorted)
    /// for the block named by the file (`core-`/`woocommerce-` at the start become `core/`/`woocommerce/`), as handle
    /// `nfd-block-<file>`, the `.min.css` file when it exists (`wp_get_environment_type()` is `production`). The
    /// `.min.css` files match the glob as well and register styles for blocks such as `core/archives.min`, which never
    /// render; they are left out.
    fn block_stylesheets(&self, template_dir: &Path) -> Vec<BlockStylesheet> {
        let dir = template_dir.join("assets").join("block-styles");
        let Ok(entries) = std::fs::read_dir(&dir) else { return Vec::new() };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".css") && !n.starts_with('.'))
            .collect();
        names.sort();
        names
            .into_iter()
            .filter_map(|file| {
                let filename = file.strip_suffix(".css")?.to_owned();
                if filename.ends_with(".min") {
                    return None;
                }
                let block = if filename.starts_with("core-") {
                    filename.replace("core-", "core/")
                } else if filename.starts_with("woocommerce-") {
                    filename.replace("woocommerce-", "woocommerce/")
                } else {
                    filename.clone()
                };
                let min = format!("assets/block-styles/{filename}.min.css");
                let path = if rustpress_core::files::is_file(&template_dir.join(&min)) { min } else { format!("assets/block-styles/{filename}.css") };
                Some(BlockStylesheet { block, handle: format!("nfd-block-{filename}"), path, theme_version: false })
            })
            .collect()
    }

    /// `Back_Compat::enqueue_frontend_assets()`: the styles for sites that used YITH Wonder before, while its theme
    /// mods exist (`get_option( 'theme_mods_yith-wonder' )` is truthy).
    fn styles(&self, ctx: &ScriptContext<'_>) -> Vec<Asset> {
        if !ctx.site.options.value("theme_mods_yith-wonder").is_some_and(|v| php_truthy(&v)) {
            return Vec::new();
        }
        let file = ["assets/back-compat/yith-wonder.min.css", "assets/back-compat/yith-wonder.css"]
            .into_iter()
            .find(|f| rustpress_core::files::is_file(&ctx.template_dir.join(f)))
            .unwrap_or("assets/back-compat/yith-wonder.css");
        vec![Asset {
            handle: "bluehost-blueprint-legacy-yith-wonder".to_owned(),
            src: format!("{}/{file}", ctx.template_uri),
            ver: Some(ctx.version.clone()),
            media: "all".to_owned(),
            priority: 10,
            ..Asset::default()
        }]
    }
}

/// PHP's boolean conversion of an option value.
fn php_truthy(value: &rustpress_phpser::Value) -> bool {
    use rustpress_phpser::Value;
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !(s.is_empty() || s.as_slice() == b"0"),
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}
