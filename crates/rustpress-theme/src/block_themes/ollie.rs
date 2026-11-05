//! The `functions.php` of Ollie 1.6.6.

use std::path::Path;

use rustpress_core::QueryResult;

use super::BlockThemePort;
use crate::functions_php::BlockStylesheet;

pub struct Ollie;

impl BlockThemePort for Ollie {
    fn version(&self) -> &'static str {
        "1.6.6"
    }

    /// `enqueue_style_sheet()`: `sanitize_title( __NAMESPACE__ )` of `namespace Ollie`.
    fn style_handle(&self) -> Option<String> {
        Some("ollie".to_owned())
    }

    /// `enqueue_custom_block_styles()`: every `assets/styles/*.css` (in `glob()` order, which sorts) for the block
    /// named by the file, `core-` replaced by `core/` (`str_replace()`, so every occurrence).
    fn block_stylesheets(&self, template_dir: &Path) -> Vec<BlockStylesheet> {
        let Ok(entries) = std::fs::read_dir(template_dir.join("assets").join("styles")) else { return Vec::new() };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.ends_with(".css") && !n.starts_with('.'))
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|file| {
                let filename = file.trim_end_matches(".css").to_owned();
                BlockStylesheet { block: filename.replace("core-", "core/"), handle: format!("ollie-block-{filename}"), path: format!("assets/styles/{filename}.css"), theme_version: false }
            })
            .collect()
    }

    /// `is_paginated()`: hides the last separator of a post list when there is no pagination.
    fn wp_head(&self, query: &QueryResult) -> String {
        if query.max_pages < 2 {
            "<style>.blog .wp-block-post-template .wp-block-post:last-child .entry-content + .wp-block-separator, .archive .wp-block-post-template .wp-block-post:last-child .entry-content + .wp-block-separator, .blog .wp-block-post-template .wp-block-post:last-child .entry-content + .wp-block-separator, .search .wp-block-post-template .wp-block-post:last-child .wp-block-post-excerpt + .wp-block-separator { display: none; }</style>".to_owned()
        } else {
            String::new()
        }
    }

    /// `fix_submenu_toggle_binding()` on `render_block_core/navigation`: submenu toggles bound to
    /// `state.isSubmenuOpen` are bound to `state.isMenuOpen`.
    fn render_block(&self, name: &str, content: String) -> String {
        if name != "core/navigation" {
            return content;
        }
        rebind_submenu_toggles(&content)
    }

    /// `template_part_areas()`.
    fn template_part_areas(&self) -> &'static [(&'static str, &'static str)] {
        &[("sidebar", "section")]
    }
}

/// Every `<button class="… wp-block-navigation-submenu__toggle …">` whose `data-wp-bind--aria-expanded` is
/// `state.isSubmenuOpen` gets `state.isMenuOpen` (`WP_HTML_Tag_Processor::set_attribute()` replaces the value in
/// place, in double quotes).
fn rebind_submenu_toggles(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = find_ci(rest, "<button") {
        let Some(len) = rest[start..].find('>') else { break };
        let tag = &rest[start..start + len + 1];
        out.push_str(&rest[..start]);
        if has_class(tag, "wp-block-navigation-submenu__toggle") {
            out.push_str(&tag.replacen("data-wp-bind--aria-expanded=\"state.isSubmenuOpen\"", "data-wp-bind--aria-expanded=\"state.isMenuOpen\"", 1));
        } else {
            out.push_str(tag);
        }
        rest = &rest[start + len + 1..];
    }
    out.push_str(rest);
    out
}

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack.as_bytes().windows(needle.len()).position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

fn has_class(tag: &str, class: &str) -> bool {
    let Some(at) = tag.find("class=\"") else { return false };
    let value = &tag[at + 7..];
    let value = &value[..value.find('"').unwrap_or(value.len())];
    value.split_ascii_whitespace().any(|c| c == class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_submenu_toggles_are_rebound() {
        let html = "<li><button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" class=\" wp-block-navigation-item__content wp-block-navigation-submenu__toggle\" >A</button><button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" class=\"other\">B</button></li>";
        assert_eq!(
            rebind_submenu_toggles(html),
            "<li><button data-wp-bind--aria-expanded=\"state.isMenuOpen\" class=\" wp-block-navigation-item__content wp-block-navigation-submenu__toggle\" >A</button><button data-wp-bind--aria-expanded=\"state.isSubmenuOpen\" class=\"other\">B</button></li>"
        );
    }
}
