//! Block themes whose `functions.php` does more than the literal calls `functions_php.rs` reads. Their templates,
//! parts, patterns and `theme.json` are used as they are; what their PHP adds is written out here, following one
//! release of the theme each (`version`), and compared with the real WordPress.

pub mod bluehost_blueprint;
pub mod extendable;
pub mod ollie;

use rustpress_core::native::Asset;

use crate::functions_php::BlockStylesheet;

/// What a theme's `wp_enqueue_scripts` callbacks read of the site.
pub struct ScriptContext<'a> {
    pub site: &'a rustpress_core::Site,
    /// `get_template_directory()`.
    pub template_dir: std::path::PathBuf,
    /// `get_template_directory_uri()`.
    pub template_uri: String,
    /// `wp_get_theme()->get( 'Version' )` (the parent theme's).
    pub version: String,
    /// `wp_get_attachment_image_url( get_theme_mod( 'custom_logo' ), 'full' )`, empty without a logo.
    pub custom_logo_url: String,
}

/// What the `functions.php` of a ported block theme contributes.
pub trait BlockThemePort: Send + Sync {
    /// The release this port follows.
    fn version(&self) -> &'static str;
    /// The handle of the theme's `style.css` (`wp_enqueue_style()` on `wp_enqueue_scripts`).
    fn style_handle(&self) -> Option<String> {
        None
    }
    /// `wp_enqueue_block_style()` calls, in the order they are made.
    fn block_stylesheets(&self, _template_dir: &std::path::Path) -> Vec<BlockStylesheet> {
        Vec::new()
    }
    /// What the theme prints on `wp_head` at priority 10, after core's actions.
    fn wp_head(&self, _query: &rustpress_core::QueryResult) -> String {
        String::new()
    }
    /// The `render_block_{$name}` filters of the theme.
    fn render_block(&self, _name: &str, content: String) -> String {
        content
    }
    /// The scripts the theme enqueues on `wp_enqueue_scripts` (after the plugins' of the same priority).
    fn scripts(&self, _ctx: &ScriptContext<'_>) -> Vec<Asset> {
        Vec::new()
    }
    /// Stylesheets the theme enqueues on `wp_enqueue_scripts` after its `style.css`.
    fn styles(&self, _ctx: &ScriptContext<'_>) -> Vec<Asset> {
        Vec::new()
    }
    /// Areas the theme adds with `default_wp_template_part_areas`: the area and its `area_tag`.
    fn template_part_areas(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
}

/// The port for the theme in `template` (the parent theme of a child theme), if there is one.
pub fn port(template: &str) -> Option<&'static dyn BlockThemePort> {
    match template {
        "bluehost-blueprint" => Some(&bluehost_blueprint::BluehostBlueprint),
        "extendable" => Some(&extendable::Extendable),
        "ollie" => Some(&ollie::Ollie),
        _ => None,
    }
}
