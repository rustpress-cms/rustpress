//! Block themes whose `functions.php` does more than the literal calls `functions_php.rs` reads. Their templates,
//! parts, patterns and `theme.json` are used as they are; what their PHP adds is written out here, following one
//! release of the theme each (`version`), and compared with the real WordPress.

pub mod ollie;

use crate::functions_php::BlockStylesheet;

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
    /// Areas the theme adds with `default_wp_template_part_areas`: the area and its `area_tag`.
    fn template_part_areas(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
}

/// The port for the theme in `template` (the parent theme of a child theme), if there is one.
pub fn port(template: &str) -> Option<&'static dyn BlockThemePort> {
    match template {
        "ollie" => Some(&ollie::Ollie),
        _ => None,
    }
}
