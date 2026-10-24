//! Block style variations (`wp-includes/block-supports/block-style-variations.php`):
//! a block with a class like `is-style-pill` gets a per-instance class
//! (`is-style-pill--2`) and the variation's CSS is generated for that class.

use serde_json::{Map, Value};

use crate::env::Env;
use crate::html::TagEditor;
use crate::registry::BlockType;

/// Names after `is-style-` (excluding `default`).
fn variation_names(class_string: &str) -> Vec<String> {
    class_string
        .split_whitespace()
        .filter_map(|c| c.strip_prefix("is-style-"))
        .filter(|n| !n.is_empty() && *n != "default" && !n.starts_with("default"))
        .map(str::to_owned)
        .collect()
}

/// `wp_render_block_style_variation_support_styles` (a `render_block_data` filter).
pub fn prepare_block_data(env: &Env<'_>, bt: &BlockType, attrs: &mut Map<String, Value>) {
    let Some(class_string) = attrs.get("className").and_then(Value::as_str) else { return };
    let names = variation_names(class_string);
    if names.is_empty() {
        return;
    }
    let Some((variation, data)) = names.iter().find_map(|n| env.theme.style_variation(&bt.name, n).map(|d| (n.clone(), d))) else { return };

    let instance = env.with_state(|s| s.unique_id(&format!("{variation}--")));
    let class = format!("is-style-{instance}");
    let css = env.theme.variation_css(&bt.name, &data, &format!(".{class}"));
    if css.is_empty() {
        return;
    }
    env.with_state(|s| s.inline_styles.push(("block-style-variation-styles".to_owned(), css)));
    let updated = format!("{} {class}", attrs.get("className").and_then(Value::as_str).unwrap_or(""));
    attrs.insert("className".into(), Value::from(updated));
}

/// `wp_render_block_style_variation_class_name` (a `render_block` filter): the
/// instance class goes onto the first tag of the output.
pub fn render_class_name(attrs: &Map<String, Value>, content: String) -> String {
    if content.is_empty() {
        return content;
    }
    let Some(class_string) = attrs.get("className").and_then(Value::as_str) else { return content };
    // `is-style-<name>--<number>`
    let Some(instance) = class_string.split_whitespace().find(|c| c.strip_prefix("is-style-").is_some_and(|rest| rest.rsplit_once("--").is_some_and(|(_, n)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())))) else {
        return content;
    };
    match TagEditor::first(&content) {
        Some(mut editor) => {
            editor.add_class(instance);
            editor.finish()
        }
        None => content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variation_names_skip_default() {
        assert_eq!(variation_names("a is-style-pill is-style-default b"), ["pill"]);
        assert!(variation_names("plain").is_empty());
    }

    #[test]
    fn instance_class_is_added_to_the_first_tag() {
        let attrs = serde_json::json!({"className": "is-style-pill is-style-pill--2"});
        let out = render_class_name(attrs.as_object().unwrap(), "<div class=\"x\">y</div>".into());
        assert_eq!(out, "<div class=\"x is-style-pill--2\">y</div>");
        let attrs = serde_json::json!({"className": "is-style-pill"});
        assert_eq!(render_class_name(attrs.as_object().unwrap(), "<div>y</div>".into()), "<div>y</div>");
    }
}
