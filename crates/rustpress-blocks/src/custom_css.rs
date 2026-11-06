//! `block-supports/custom-css.php`: the CSS of a block instance (`style.css`).

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value, json};

use crate::env::Env;
use crate::html::TagEditor;
use crate::parser::{Block, Content};
use crate::registry::BlockType;

const CLASS_PREFIX: &str = "wp-custom-css-";

/// `WP_Theme_JSON::process_blocks_custom_css()`: nested rules with `&` become scoped rules.
pub fn process_custom_css(css: &str, selector: &str) -> String {
    let mut out = String::new();
    for part in css.split('&') {
        if part.is_empty() || part == "0" {
            continue;
        }
        if !part.contains('{') {
            // Plain declarations apply to the block itself.
            out.push_str(&format!(":root :where({}){{{}}}", selector.trim(), part.trim()));
            continue;
        }
        let cleaned = part.replace('}', "");
        let pieces: Vec<&str> = cleaned.split('{').collect();
        let [nested, value] = pieces[..] else { continue };
        // A pseudo-element (`::before`, with the combinators in front of it) goes behind the `:where()`.
        let (nested, pseudo) = match nested.find("::") {
            Some(at) => {
                let start = nested[..at].trim_end_matches(|c: char| matches!(c, '>' | '+' | '~') || c.is_whitespace()).len();
                let tail = &nested[at + 2..];
                let end = at + 2 + tail.find(|c: char| !(c.is_ascii_alphabetic() || c == '-')).unwrap_or(tail.len());
                (format!("{}{}", &nested[..start], &nested[end..]), nested[start..end].to_owned())
            }
            None => (nested.to_owned(), String::new()),
        };
        // A nested selector that starts with a space is a descendant of the block (each comma part);
        // otherwise it is appended to the block selector as a whole.
        let scoped = if nested.starts_with(' ') {
            nested.split(',').map(|s| format!("{selector} {}", s.trim()).trim_end().to_owned()).collect::<Vec<_>>().join(", ")
        } else {
            format!("{selector}{nested}")
        };
        out.push_str(&format!(":root :where({scoped}){pseudo}{{{}}}", value.trim()));
    }
    out
}

/// A block as `parse_blocks()` returns it, with `attrs` of the block being rendered.
fn parsed_block(block: &Block, attrs: &Map<String, Value>) -> Value {
    let inner_content: Vec<Value> = block
        .inner_content
        .iter()
        .map(|chunk| match chunk {
            Content::Html(html) => Value::String(html.clone()),
            Content::InnerBlock => Value::Null,
        })
        .collect();
    let inner_blocks: Vec<Value> = block.inner_blocks.iter().map(|inner| parsed_block(inner, &inner.attrs)).collect();
    json!({ "blockName": block.name, "attrs": attrs, "innerBlocks": inner_blocks, "innerHTML": block.inner_html, "innerContent": inner_content })
}

/// `wp_render_custom_css_support_styles()`, a `render_block_data` filter.
pub fn prepare_block_data(env: &Env<'_>, bt: &BlockType, block: &Block, parent_layout: Option<&Value>, attrs: &mut Map<String, Value>) {
    static MARKUP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</?(?-u:\w)+").expect("the pattern is valid"));
    let Some(css) = attrs.get("style").and_then(|s| s.get("css")).and_then(Value::as_str).filter(|css| !css.trim().is_empty()).map(str::to_owned) else { return };
    if !bt.has_support(&["customCSS"], true) || MARKUP.is_match(&css) {
        return;
    }
    let mut parsed = parsed_block(block, attrs);
    // `wp_add_parent_layout_to_parsed_block()` came first.
    if let (Some(layout), Some(map)) = (parent_layout, parsed.as_object_mut()) {
        map.insert("parentLayout".to_owned(), layout.clone());
    }
    let class = crate::php_json::unique_id_from_values(&parsed, CLASS_PREFIX);
    let updated = match attrs.get("className").and_then(Value::as_str) {
        Some(existing) => format!("{existing} {class}"),
        None => class.clone(),
    };
    attrs.insert("className".to_owned(), Value::String(updated));
    let processed = process_custom_css(&css, &format!(".{class}"));
    if !processed.is_empty() {
        env.with_state(|s| {
            let entry = ("wp-block-custom-css".to_owned(), processed);
            if !s.inline_styles.contains(&entry) {
                s.inline_styles.push(entry);
            }
        });
    }
}

/// `wp_render_custom_css_class_name()`.
pub fn render_class_name(attrs: &Map<String, Value>, content: String) -> String {
    let Some(class) = attrs.get("className").and_then(Value::as_str).and_then(|c| c.split([' ', '\t', '\u{c}', '\r', '\n']).find(|token| token.starts_with(CLASS_PREFIX))) else { return content };
    let Some(mut editor) = TagEditor::first(&content) else { return content };
    editor.add_class("has-custom-css");
    editor.add_class(class);
    editor.finish()
}

/// `wp_strip_custom_css_from_blocks()`: what a user without `edit_css` saves keeps no `style.css`. The content is not
/// slashed here.
pub fn strip_custom_css(content: &str) -> String {
    let mut result = String::new();
    let mut was_at = 0;
    for delimiter in crate::parser::block_delimiters(content) {
        let mut attrs = delimiter.attrs;
        let Some(style) = attrs.get_mut("style").and_then(Value::as_object_mut).filter(|s| s.get("css").is_some_and(|css| !css.is_null())) else { continue };
        style.shift_remove("css");
        if style.is_empty() {
            attrs.shift_remove("style");
        }
        let token = &content[delimiter.start..delimiter.start + delimiter.len];
        let (Some(open), Some(close)) = (token.find('{'), token.rfind('}')) else { continue };
        let json_start = delimiter.start + open;
        result.push_str(&content[was_at..json_start]);
        if attrs.is_empty() {
            // The space after the attributes goes with them.
            was_at = delimiter.start + close + 2;
        } else {
            result.push_str(&crate::serializer::serialize_block_attributes(&attrs));
            was_at = delimiter.start + close + 1;
        }
    }
    result.push_str(&content[was_at..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_declarations_and_nested_rules_are_scoped() {
        assert_eq!(process_custom_css("color:red;", ".c"), ":root :where(.c){color:red;}");
        assert_eq!(process_custom_css("color:red;& a{color:blue}", ".c"), ":root :where(.c){color:red;}:root :where(.c a){color:blue}");
        assert_eq!(process_custom_css("&::before{content:'x'}", ".c"), ":root :where(.c)::before{content:'x'}");
    }

    #[test]
    fn the_css_of_a_block_is_stripped_and_the_rest_stays() {
        let saved = "<!-- wp:paragraph {\"style\":{\"css\":\"color:red\"}} --><p>a</p><!-- /wp:paragraph -->";
        assert_eq!(strip_custom_css(saved), "<!-- wp:paragraph --><p>a</p><!-- /wp:paragraph -->");
        let saved = "<!-- wp:paragraph {\"style\":{\"css\":\"x\",\"color\":{\"text\":\"red\"}},\"a\":1} /-->";
        assert_eq!(strip_custom_css(saved), "<!-- wp:paragraph {\"style\":{\"color\":{\"text\":\"red\"}},\"a\":1} /-->");
        assert_eq!(strip_custom_css("<p>plain</p>"), "<p>plain</p>");
    }
}
