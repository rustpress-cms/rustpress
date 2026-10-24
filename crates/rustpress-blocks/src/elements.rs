//! The "elements" block support: link, heading and button colors set on a
//! block get a generated `wp-elements-N` class and matching CSS rules.

use serde_json::{Map, Value};

use crate::env::Env;
use crate::html::TagEditor;
use crate::registry::BlockType;
use crate::style_engine::{get_styles, path_get};

const COLOR_PATHS: &[(&str, &[&[&str]])] = &[
    ("button", &[&["button", "color", "text"], &["button", "color", "background"], &["button", "color", "gradient"]]),
    ("link", &[&["link", "color", "text"], &["link", ":hover", "color", "text"]]),
    (
        "heading",
        &[
            &["heading", "color", "text"],
            &["heading", "color", "background"],
            &["heading", "color", "gradient"],
            &["h1", "color", "text"],
            &["h1", "color", "background"],
            &["h1", "color", "gradient"],
            &["h2", "color", "text"],
            &["h2", "color", "background"],
            &["h2", "color", "gradient"],
            &["h3", "color", "text"],
            &["h3", "color", "background"],
            &["h3", "color", "gradient"],
            &["h4", "color", "text"],
            &["h4", "color", "background"],
            &["h4", "color", "gradient"],
            &["h5", "color", "text"],
            &["h5", "color", "background"],
            &["h5", "color", "gradient"],
            &["h6", "color", "text"],
            &["h6", "color", "background"],
            &["h6", "color", "gradient"],
        ],
    ),
];

/// `wp_render_elements_support_styles` (a `render_block_data` filter):
/// adds the generated class to `className` and stores the CSS rules.
pub fn prepare_block_data(env: &Env<'_>, bt: &BlockType, attrs: &mut Map<String, Value>) {
    let Some(elements) = attrs.get("style").and_then(|s| s.get("elements")).filter(|e| !crate::style_engine::is_empty(e)).cloned() else {
        return;
    };
    let skip = |element: &str| bt.skips_serialization("color", Some(element));
    let (skip_link, skip_heading, skip_button) = (skip("link"), skip("heading"), skip("button"));
    if skip_link && skip_heading && skip_button {
        return;
    }
    let skipped = |name: &str| match name {
        "button" => skip_button,
        "link" => skip_link,
        _ => skip_heading,
    };
    let should_add = COLOR_PATHS
        .iter()
        .filter(|(name, _)| !skipped(name))
        .any(|(_, paths)| paths.iter().any(|path| path_get(&elements, path).is_some_and(|v| !v.is_null())));
    if !should_add {
        return;
    }

    let class = env.with_state(|s| s.unique_prefixed_id("wp-elements-"));
    let updated = match attrs.get("className").and_then(Value::as_str) {
        Some(existing) => format!("{existing} {class}"),
        None => class.clone(),
    };
    attrs.insert("className".into(), Value::from(updated));

    let store = |selector: String, style: &Value| {
        let styles = get_styles(style, false);
        if !styles.declarations.is_empty() {
            env.with_state(|s| s.block_supports_css.add(&selector, &styles.declarations, None));
        }
    };
    if !skip_button
        && let Some(style) = elements.get("button").filter(|v| !crate::style_engine::is_empty(v)) {
            store(format!(".{class} .wp-element-button, .{class} .wp-block-button__link"), style);
        }
    if !skip_link
        && let Some(style) = elements.get("link").filter(|v| !crate::style_engine::is_empty(v)) {
            store(format!(".{class} a:where(:not(.wp-element-button))"), style);
            if let Some(hover) = style.get(":hover") {
                store(format!(".{class} a:where(:not(.wp-element-button)):hover"), hover);
            }
        }
    if !skip_heading {
        if let Some(style) = elements.get("heading").filter(|v| !crate::style_engine::is_empty(v)) {
            let selector = (1..=6).map(|n| format!(".{class} h{n}")).collect::<Vec<_>>().join(", ");
            store(selector, style);
        }
        for n in 1..=6 {
            let element = format!("h{n}");
            if let Some(style) = elements.get(&element).filter(|v| !crate::style_engine::is_empty(v)) {
                store(format!(".{class} {element}"), style);
            }
        }
    }
}

/// `wp_render_elements_class_name` (a `render_block` filter): puts the
/// generated class on the first tag of the output.
pub fn render_class_name(attrs: &Map<String, Value>, content: String) -> String {
    let Some(class_attr) = attrs.get("className").and_then(Value::as_str) else { return content };
    let Some(generated) = class_attr.split_whitespace().find(|c| c.starts_with("wp-elements-")) else { return content };
    match TagEditor::first(&content) {
        Some(mut editor) => {
            editor.add_class(generated);
            editor.finish()
        }
        None => content,
    }
}

