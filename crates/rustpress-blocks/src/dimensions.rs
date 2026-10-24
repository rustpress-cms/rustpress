//! The aspect-ratio part of the dimensions block support
//! (`wp_render_dimensions_support`), applied to the rendered block.

use serde_json::{Map, Value, json};

use crate::html::TagEditor;
use crate::registry::BlockType;
use crate::style_engine::get_styles;

fn is_explicit_aspect_ratio(value: Option<&Value>) -> bool {
    let text = match value {
        Some(Value::String(s)) => s.trim().to_ascii_lowercase(),
        Some(Value::Number(n)) => n.to_string(),
        _ => return false,
    };
    !text.is_empty() && text != "auto"
}

pub fn render_dimensions_support(bt: &BlockType, attrs: &Map<String, Value>, content: String) -> String {
    if !bt.has_support(&["dimensions", "aspectRatio"], false) || bt.skips_serialization("dimensions", Some("aspectRatio")) {
        return content;
    }
    let dimensions = attrs.get("style").and_then(|s| s.get("dimensions"));
    let aspect_ratio = dimensions.and_then(|d| d.get("aspectRatio"));
    let mut styles = Map::new();
    styles.insert("aspectRatio".into(), aspect_ratio.cloned().unwrap_or(Value::Null));
    let explicit = is_explicit_aspect_ratio(aspect_ratio);
    if explicit {
        styles.insert("minHeight".into(), json!("unset"));
        styles.insert("height".into(), json!("unset"));
    } else if dimensions.and_then(|d| d.get("minHeight")).is_some_and(|v| !v.is_null()) || attrs.get("minHeight").is_some_and(|v| !v.is_null()) {
        styles.insert("aspectRatio".into(), json!("unset"));
    }
    let result = get_styles(&json!({ "dimensions": styles }), false);
    if result.css.is_empty() {
        return content;
    }
    let Some(mut editor) = TagEditor::first(&content) else { return content };
    let existing = editor.get_attribute("style").unwrap_or("").to_owned();
    let mut updated = existing.clone();
    if !existing.is_empty() && !existing.ends_with(';') {
        updated.push(';');
    }
    updated.push_str(&result.css);
    editor.set_attribute("style", &updated);
    for class in result.classnames.split(' ').filter(|c| !c.is_empty()) {
        if class.contains("aspect-ratio") && !explicit {
            continue;
        }
        editor.add_class(class);
    }
    editor.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::BlockType;

    fn bt() -> BlockType {
        BlockType {
            name: "core/post-featured-image".into(),
            attributes: Map::new(),
            supports: json!({"dimensions": {"aspectRatio": true}}),
            uses_context: Vec::new(),
            provides_context: Map::new(),
            selectors: Value::Null,
            dir: Default::default(),
            has_style: false,
            styles: Vec::new(),
        }
    }

    #[test]
    fn explicit_ratio_adds_style_and_class() {
        let attrs = json!({"style": {"dimensions": {"aspectRatio": "3/2"}}});
        let out = render_dimensions_support(&bt(), attrs.as_object().unwrap(), "<figure class=\"x\">i</figure>".into());
        assert_eq!(out, "<figure style=\"aspect-ratio:3/2;height:unset;min-height:unset;\" class=\"x has-aspect-ratio\">i</figure>");
    }

    #[test]
    fn no_aspect_ratio_means_no_change() {
        let attrs = json!({});
        assert_eq!(render_dimensions_support(&bt(), attrs.as_object().unwrap(), "<p>x</p>".into()), "<p>x</p>");
    }
}
