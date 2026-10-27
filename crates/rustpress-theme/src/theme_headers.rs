//! A few headers of a theme's `style.css`.

use std::path::Path;

/// `Text Domain:` of the theme in `dir` (its directory name without one).
pub fn text_domain(dir: &Path) -> String {
    let path = dir.join("style.css");
    let head = match rustpress_core::files::read(&path) {
        Some(text) => String::from_utf8_lossy(&text.as_bytes()[..text.len().min(8192)]).into_owned(),
        // Not UTF-8 (or missing): the bytes as they are.
        None => {
            let bytes = std::fs::read(&path).unwrap_or_default();
            String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]).into_owned()
        }
    };
    head.lines()
        .find_map(|line| {
            let line = line.trim().trim_start_matches(['*', '/', '#', '@']).trim();
            let (key, value) = line.split_once(':')?;
            key.trim().eq_ignore_ascii_case("Text Domain").then(|| value.trim().to_owned())
        })
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}

/// The labels `get_theme_feature_list( false )` has for the tags of `Tags:`.
pub fn feature_label(tag: &str) -> Option<&'static str> {
    const FEATURES: &[(&str, &str)] = &[
        ("blog", "Blog"),
        ("e-commerce", "E-Commerce"),
        ("education", "Education"),
        ("entertainment", "Entertainment"),
        ("food-and-drink", "Food & Drink"),
        ("holiday", "Holiday"),
        ("news", "News"),
        ("photography", "Photography"),
        ("portfolio", "Portfolio"),
        ("accessibility-ready", "Accessibility Ready"),
        ("block-patterns", "Block Editor Patterns"),
        ("block-styles", "Block Editor Styles"),
        ("custom-background", "Custom Background"),
        ("custom-colors", "Custom Colors"),
        ("custom-header", "Custom Header"),
        ("custom-logo", "Custom Logo"),
        ("editor-style", "Editor Style"),
        ("featured-image-header", "Featured Image Header"),
        ("featured-images", "Featured Images"),
        ("footer-widgets", "Footer Widgets"),
        ("full-site-editing", "Site Editor"),
        ("full-width-template", "Full Width Template"),
        ("post-formats", "Post Formats"),
        ("sticky-post", "Sticky Post"),
        ("style-variations", "Style Variations"),
        ("template-editing", "Template Editing"),
        ("theme-options", "Theme Options"),
        ("grid-layout", "Grid Layout"),
        ("one-column", "One Column"),
        ("two-columns", "Two Columns"),
        ("three-columns", "Three Columns"),
        ("four-columns", "Four Columns"),
        ("left-sidebar", "Left Sidebar"),
        ("right-sidebar", "Right Sidebar"),
        ("wide-blocks", "Wide Blocks"),
    ];
    FEATURES.iter().find(|(slug, _)| *slug == tag || *slug == tag.to_lowercase()).map(|(_, label)| *label)
}
