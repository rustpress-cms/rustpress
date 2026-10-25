//! Block supports as applied to dynamic blocks through
//! `get_block_wrapper_attributes()`: alignment, custom and generated class
//! names, colors, typography, borders, spacing, dimensions, shadow, anchor and
//! aria-label. The order matches the registration order in `wp-settings.php`.

use serde_json::{Map, Value, json};

use crate::html::esc_attr;
use crate::registry::BlockType;
use crate::style_engine::{get_styles, safecss_filter_declaration, to_kebab_case};

/// Attribute name -> value, in insertion order.
type Attrs = Vec<(String, String)>;

fn push(out: &mut Attrs, name: &str, value: String) {
    if value.is_empty() {
        return;
    }
    match out.iter_mut().find(|(n, _)| n == name) {
        Some((_, existing)) if !existing.is_empty() => {
            existing.push(' ');
            existing.push_str(&value);
        }
        Some((_, existing)) => *existing = value,
        None => out.push((name.to_owned(), value)),
    }
}

fn attr_str<'a>(attrs: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    attrs.get(key).and_then(Value::as_str)
}

/// `wp_get_block_default_classname`.
pub fn default_classname(block_name: &str) -> String {
    let flattened = block_name.replace('/', "-");
    format!("wp-block-{}", flattened.strip_prefix("core-").unwrap_or(&flattened))
}

/// All block supports of one block, merged by attribute name.
pub fn apply_block_supports(bt: &BlockType, attrs: &Map<String, Value>, settings: Option<&Value>) -> Attrs {
    let mut out: Attrs = Vec::new();

    // align
    if bt.has_support(&["align"], false)
        && let Some(align) = attrs.get("align")
            && let Some(align) = align.as_str() {
                push(&mut out, "class", format!("align{align}"));
            }
    // custom class name
    if bt.has_support(&["customClassName"], true)
        && let Some(name) = attr_str(attrs, "className") {
            push(&mut out, "class", name.to_owned());
        }
    // generated class name
    if bt.has_support(&["className"], true) {
        push(&mut out, "class", default_classname(&bt.name));
    }

    for (name, value) in colors(bt, attrs).into_iter().chain(typography(bt, attrs, settings)).chain(border(bt, attrs)) {
        push(&mut out, &name, value);
    }
    for (name, value) in spacing(bt, attrs).into_iter().chain(dimensions(bt, attrs)).chain(shadow(bt, attrs)) {
        push(&mut out, &name, value);
    }

    // aria-label
    if bt.has_support(&["ariaLabel"], false) && !bt.skips_serialization("ariaLabel", None)
        && let Some(label) = attr_str(attrs, "ariaLabel") {
            push(&mut out, "aria-label", label.to_owned());
        }
    // anchor
    if bt.has_support(&["anchor"], false)
        && let Some(anchor) = attr_str(attrs, "anchor").filter(|a| !a.is_empty()) {
            push(&mut out, "id", anchor.to_owned());
        }
    out
}

fn style_and_classes(styles: crate::style_engine::Styles) -> Attrs {
    let mut out = Vec::new();
    if !styles.classnames.is_empty() {
        out.push(("class".to_owned(), styles.classnames));
    }
    if !styles.css.is_empty() {
        out.push(("style".to_owned(), styles.css));
    }
    out
}

fn style_path<'a>(attrs: &'a Map<String, Value>, path: &[&str]) -> Option<&'a Value> {
    let mut current = attrs.get("style")?;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current).filter(|v| !v.is_null())
}

fn colors(bt: &BlockType, attrs: &Map<String, Value>) -> Attrs {
    let support = bt.support(&["color"]).cloned().unwrap_or(Value::Bool(false));
    let is_array = support.is_object();
    if is_array && bt.skips_serialization("color", None) {
        return Vec::new();
    }
    let flag = |key: &str| support.get(key).is_some_and(|v| !crate::style_engine::is_empty(v));
    let has_key = |key: &str| support.get(key).is_some();
    let all = support == Value::Bool(true);
    let has_text = all || flag("text") || (is_array && !has_key("text"));
    let has_background = all || flag("background") || (is_array && !has_key("background"));
    let has_gradients = flag("gradients");

    let mut color = Map::new();
    if has_text && !bt.skips_serialization("color", Some("text")) {
        let value = attr_str(attrs, "textColor")
            .map(|c| Value::from(format!("var:preset|color|{c}")))
            .or_else(|| style_path(attrs, &["color", "text"]).cloned());
        color.insert("text".into(), value.unwrap_or(Value::Null));
    }
    if has_background && !bt.skips_serialization("color", Some("background")) {
        let value = attr_str(attrs, "backgroundColor")
            .map(|c| Value::from(format!("var:preset|color|{c}")))
            .or_else(|| style_path(attrs, &["color", "background"]).cloned());
        color.insert("background".into(), value.unwrap_or(Value::Null));
    }
    let has_background_gradient = bt.has_support(&["background", "gradient"], false)
        && style_path(attrs, &["background", "gradient"]).is_some_and(|v| !crate::style_engine::is_empty(v));
    if has_gradients && !bt.skips_serialization("color", Some("gradients")) && !has_background_gradient {
        let value = attr_str(attrs, "gradient")
            .map(|c| Value::from(format!("var:preset|gradient|{c}")))
            .or_else(|| style_path(attrs, &["color", "gradient"]).cloned());
        color.insert("gradient".into(), value.unwrap_or(Value::Null));
    }
    style_and_classes(get_styles(&json!({ "color": color }), true))
}

/// `wp_typography_get_preset_inline_style_value`.
fn typography_preset_value(value: &Value, property: &str) -> Value {
    match value.as_str() {
        Some(s) if s.contains(&format!("var:preset|{property}|")) => {
            let slug = to_kebab_case(s.rsplit('|').next().unwrap_or(""));
            Value::from(format!("var(--wp--preset--{property}--{slug})"))
        }
        _ => value.clone(),
    }
}

/// `wp_render_typography_support()`: a custom font size in the saved markup becomes fluid.
pub fn render_typography_support(env: &crate::env::Env<'_>, attrs: &Map<String, Value>, content: String) -> String {
    let Some(size) = attrs.get("style").and_then(|s| s.get("typography")).and_then(|t| t.get("fontSize")).and_then(Value::as_str) else { return content };
    let fluid = crate::fluid::fluid_font_size(size, None, env.theme.global_settings());
    if fluid.is_empty() || fluid == size {
        return content;
    }
    let Ok(pattern) = regex::Regex::new(&format!(r"font-size\s*:\s*{}\s*;?", regex::escape(size))) else { return content };
    let replacement = format!("font-size:{};", rustpress_core::formatting::esc_attr(&fluid));
    pattern.replace(&content, regex::NoExpand(&replacement)).into_owned()
}

fn typography(bt: &BlockType, attrs: &Map<String, Value>, settings: Option<&Value>) -> Attrs {
    let Some(support) = bt.support(&["typography"]).filter(|v| !crate::style_engine::is_empty(v)) else { return Vec::new() };
    if bt.skips_serialization("typography", None) {
        return Vec::new();
    }
    let has = |key: &str| support.get(key).is_some_and(|v| !crate::style_engine::is_empty(v));
    let skip = |feature: &str| bt.skips_serialization("typography", Some(feature));
    let typo = |key: &str| style_path(attrs, &["typography", key]).cloned();

    let mut t = Map::new();
    if has("fontSize") && !skip("fontSize") {
        let value = attr_str(attrs, "fontSize").map(|s| Value::from(format!("var:preset|font-size|{s}"))).or_else(|| {
            // A custom size is made fluid like a preset one (`wp_get_typography_font_size_value()`).
            typo("fontSize").map(|v| match (v.as_str(), settings) {
                (Some(size), Some(settings)) => Value::from(crate::fluid::fluid_font_size(size, None, settings)),
                _ => v,
            })
        });
        t.insert("fontSize".into(), value.unwrap_or(Value::Null));
    }
    if has("__experimentalFontFamily") && !skip("fontFamily") {
        let value = attr_str(attrs, "fontFamily")
            .map(|s| Value::from(format!("var:preset|font-family|{s}")))
            .or_else(|| typo("fontFamily").map(|v| typography_preset_value(&v, "font-family")));
        t.insert("fontFamily".into(), value.unwrap_or(Value::Null));
    }
    for (support_key, key, property) in [
        ("__experimentalFontStyle", "fontStyle", "font-style"),
        ("__experimentalFontWeight", "fontWeight", "font-weight"),
        ("__experimentalTextDecoration", "textDecoration", "text-decoration"),
        ("__experimentalTextTransform", "textTransform", "text-transform"),
        ("__experimentalLetterSpacing", "letterSpacing", "letter-spacing"),
    ] {
        if has(support_key) && !skip(key)
            && let Some(v) = typo(key) {
                t.insert(key.into(), typography_preset_value(&v, property));
            }
    }
    for (support_key, key) in [("lineHeight", "lineHeight"), ("textColumns", "textColumns"), ("__experimentalWritingMode", "writingMode"), ("textIndent", "textIndent")] {
        if has(support_key) && !skip(key)
            && let Some(v) = typo(key) {
                t.insert(key.into(), v);
            }
    }
    let mut out = style_and_classes(get_styles(&json!({ "typography": t }), true));
    if has("textAlign") && !skip("textAlign")
        && let Some(Value::String(align)) = typo("textAlign") {
            let class = format!("has-text-align-{align}");
            match out.iter_mut().find(|(n, _)| n == "class") {
                Some((_, existing)) => existing.push_str(&format!(" {class}")),
                None => out.insert(0, ("class".into(), class)),
            }
        }
    out
}

fn has_border_feature(bt: &BlockType, feature: &str) -> bool {
    if bt.support(&["__experimentalBorder"]) == Some(&Value::Bool(true)) {
        return true;
    }
    bt.has_support(&["__experimentalBorder", feature], false)
}

fn border(bt: &BlockType, attrs: &Map<String, Value>) -> Attrs {
    if bt.skips_serialization("border", None) {
        return Vec::new();
    }
    let skip = |feature: &str| bt.skips_serialization("__experimentalBorder", Some(feature));
    let mut b = Map::new();
    let has_color = has_border_feature(bt, "color");
    let has_width = has_border_feature(bt, "width");

    let numeric_px = |v: &Value| match v {
        Value::Number(n) => Value::from(format!("{n}px")),
        other => other.clone(),
    };
    if has_border_feature(bt, "radius") && !skip("radius")
        && let Some(radius) = style_path(attrs, &["border", "radius"]) {
            b.insert("radius".into(), numeric_px(radius));
        }
    if has_border_feature(bt, "style") && !skip("style")
        && let Some(style) = style_path(attrs, &["border", "style"]) {
            b.insert("style".into(), style.clone());
        }
    if has_width && !skip("width")
        && let Some(width) = style_path(attrs, &["border", "width"]) {
            b.insert("width".into(), numeric_px(width));
        }
    if has_color && !skip("color") {
        let value = attr_str(attrs, "borderColor")
            .map(|c| Value::from(format!("var:preset|color|{c}")))
            .or_else(|| style_path(attrs, &["border", "color"]).cloned());
        b.insert("color".into(), value.unwrap_or(Value::Null));
    }
    if has_color || has_width {
        for side in ["top", "right", "bottom", "left"] {
            let border = style_path(attrs, &["border", side]);
            let pick = |key: &str, feature: &str| {
                border.and_then(|b| b.get(key)).filter(|v| !v.is_null() && !skip(feature)).cloned().unwrap_or(Value::Null)
            };
            b.insert(side.into(), json!({ "width": pick("width", "width"), "color": pick("color", "color"), "style": pick("style", "style") }));
        }
    }
    style_and_classes(get_styles(&json!({ "border": b }), false))
}

fn spacing(bt: &BlockType, attrs: &Map<String, Value>) -> Attrs {
    if bt.skips_serialization("spacing", None) || attrs.get("style").is_none_or(crate::style_engine::is_empty) {
        return Vec::new();
    }
    let mut s = Map::new();
    if bt.has_support(&["spacing", "padding"], false) && !bt.skips_serialization("spacing", Some("padding")) {
        s.insert("padding".into(), style_path(attrs, &["spacing", "padding"]).cloned().unwrap_or(Value::Null));
    }
    if bt.has_support(&["spacing", "margin"], false) && !bt.skips_serialization("spacing", Some("margin")) {
        s.insert("margin".into(), style_path(attrs, &["spacing", "margin"]).cloned().unwrap_or(Value::Null));
    }
    let styles = get_styles(&json!({ "spacing": s }), false);
    if styles.css.is_empty() { Vec::new() } else { vec![("style".into(), styles.css)] }
}

fn dimensions(bt: &BlockType, attrs: &Map<String, Value>) -> Attrs {
    if bt.skips_serialization("dimensions", None) || attrs.get("style").is_none_or(crate::style_engine::is_empty) {
        return Vec::new();
    }
    let mut d = Map::new();
    for feature in ["minHeight", "minWidth", "height", "width"] {
        let supported = bt.has_support(&["dimensions", feature], false) && !bt.skips_serialization("dimensions", Some(feature));
        d.insert(feature.into(), if supported { style_path(attrs, &["dimensions", feature]).cloned().unwrap_or(Value::Null) } else { Value::Null });
    }
    let styles = get_styles(&json!({ "dimensions": d }), false);
    if styles.css.is_empty() { Vec::new() } else { vec![("style".into(), styles.css)] }
}

fn shadow(bt: &BlockType, attrs: &Map<String, Value>) -> Attrs {
    if !bt.has_support(&["shadow"], false) || bt.skips_serialization("shadow", None) {
        return Vec::new();
    }
    let value = style_path(attrs, &["shadow"]).cloned().unwrap_or(Value::Null);
    let styles = get_styles(&json!({ "shadow": value }), false);
    if styles.css.is_empty() { Vec::new() } else { vec![("style".into(), styles.css)] }
}

/// `safecss_filter_attr`: keeps each declaration as written, drops unsafe
/// ones, joins with `;` and no trailing semicolon.
pub fn safecss_filter_attr(css: &str) -> String {
    let mut items = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut current = String::new();
    for c in css.chars() {
        match c {
            '"' | '\'' if quote.is_none() => quote = Some(c),
            c if Some(c) == quote => quote = None,
            '(' if quote.is_none() => depth += 1,
            ')' if quote.is_none() => depth -= 1,
            ';' if quote.is_none() && depth <= 0 => {
                items.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    items.push(current);
    let mut out = Vec::new();
    for item in items {
        let item = item.trim();
        let Some((property, value)) = item.split_once(':') else { continue };
        if safecss_filter_declaration(property.trim(), value).is_some() {
            out.push(item.to_owned());
        }
    }
    out.join(";")
}

/// `get_block_wrapper_attributes( $extra )`: `class="..." style="..."` ready
/// to put into a start tag. `extra` takes precedence for `id`/`aria-label`,
/// and its classes come first.
pub fn wrapper_attributes(bt: Option<&BlockType>, attrs: &Map<String, Value>, extra: &[(&str, String)]) -> String {
    wrapper_attributes_with(bt, attrs, extra, None)
}

/// [`wrapper_attributes`] with the global settings, which fluid typography needs.
pub fn wrapper_attributes_with(bt: Option<&BlockType>, attrs: &Map<String, Value>, extra: &[(&str, String)], settings: Option<&Value>) -> String {
    let new = bt.map(|bt| apply_block_supports(bt, attrs, settings)).unwrap_or_default();
    if new.is_empty() && extra.iter().all(|(_, v)| v.is_empty()) {
        return String::new();
    }
    let lookup = |list: &[(String, String)], name: &str| list.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or_default();
    let extra_owned: Vec<(String, String)> = extra.iter().map(|(n, v)| ((*n).to_owned(), v.clone())).collect();

    let mut result: Vec<(String, String)> = Vec::new();
    for name in ["style", "class", "id", "aria-label"] {
        let n = lookup(&new, name);
        let e = lookup(&extra_owned, name);
        if n.is_empty() && e.is_empty() {
            continue;
        }
        let merged = match name {
            "style" => {
                let parts: Vec<String> =
                    [n.trim().trim_end_matches(';').trim().to_owned(), e.trim().trim_end_matches(';').trim().to_owned()].into_iter().filter(|p| !p.is_empty()).collect();
                safecss_filter_attr(&parts.join(";"))
            }
            "class" => {
                let mut classes: Vec<&str> = Vec::new();
                for class in e.split_whitespace().chain(n.split_whitespace()) {
                    if !classes.contains(&class) {
                        classes.push(class);
                    }
                }
                classes.join(" ")
            }
            _ => {
                if e.is_empty() { n } else { e }
            }
        };
        result.push((name.to_owned(), merged));
    }
    for (name, value) in &extra_owned {
        if !["style", "class", "id", "aria-label"].contains(&name.as_str()) {
            result.push((name.clone(), value.clone()));
        }
    }
    result.iter().map(|(k, v)| format!("{k}=\"{}\"", esc_attr(v))).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::registry::BlockType;

    fn bt(name: &str, supports: Value) -> BlockType {
        BlockType {
            name: name.into(),
            attributes: Map::new(),
            supports,
            uses_context: Vec::new(),
            provides_context: Map::new(),
            selectors: Value::Null,
            dir: Default::default(),
            has_style: false,
            styles: Vec::new(),
        }
    }

    fn attrs(v: Value) -> Map<String, Value> {
        v.as_object().cloned().unwrap_or_default()
    }

    #[test]
    fn generated_class_strips_the_core_prefix() {
        assert_eq!(default_classname("core/post-title"), "wp-block-post-title");
        assert_eq!(default_classname("my-plugin/thing"), "wp-block-my-plugin-thing");
    }

    #[test]
    fn wrapper_has_style_before_class_and_extra_classes_first() {
        let b = bt("core/post-terms", json!({"typography": {"fontSize": true, "__experimentalFontWeight": true}}));
        let a = attrs(json!({"style": {"typography": {"fontWeight": "300"}}}));
        let out = wrapper_attributes(Some(&b), &a, &[("class", "taxonomy-category".into())]);
        assert_eq!(out, "style=\"font-weight:300\" class=\"taxonomy-category wp-block-post-terms\"");
    }

    #[test]
    fn preset_colors_become_classes_and_custom_values_inline() {
        let b = bt("core/site-title", json!({"color": {"text": true, "background": true}}));
        let a = attrs(json!({"textColor": "contrast", "style": {"color": {"background": "#fff"}}, "className": "x"}));
        assert_eq!(
            wrapper_attributes(Some(&b), &a, &[]),
            "style=\"background-color:#fff\" class=\"x wp-block-site-title has-text-color has-contrast-color has-background\""
        );
    }

    #[test]
    fn align_anchor_and_aria_label() {
        let b = bt("core/group", json!({"align": ["wide", "full"], "anchor": true, "ariaLabel": true}));
        let a = attrs(json!({"align": "full", "anchor": "top", "ariaLabel": "Hi \"there\""}));
        assert_eq!(wrapper_attributes(Some(&b), &a, &[]), "class=\"alignfull wp-block-group\" id=\"top\" aria-label=\"Hi &quot;there&quot;\"");
    }

    #[test]
    fn spacing_and_font_size_presets() {
        let b = bt("core/x", json!({"spacing": {"padding": true, "margin": true}, "typography": {"fontSize": true}}));
        let a = attrs(json!({"fontSize": "small", "style": {"spacing": {"margin": {"top": "var:preset|spacing|60"}}}}));
        assert_eq!(wrapper_attributes(Some(&b), &a, &[]), "style=\"margin-top:var(--wp--preset--spacing--60)\" class=\"wp-block-x has-small-font-size\"");
    }

    #[test]
    fn no_attributes_yields_an_empty_string() {
        assert_eq!(wrapper_attributes(None, &Map::new(), &[]), "");
        assert_eq!(wrapper_attributes(None, &Map::new(), &[("class", String::new())]), "");
    }

    #[test]
    fn extra_non_standard_attributes_are_appended() {
        assert_eq!(wrapper_attributes(None, &Map::new(), &[("data-x", "1".into()), ("class", "a".into())]), "class=\"a\" data-x=\"1\"");
    }

    #[test]
    fn safecss_keeps_declarations_and_respects_parentheses() {
        assert_eq!(safecss_filter_attr("a:b; c: d ;;e"), "a:b;c: d");
        assert_eq!(safecss_filter_attr("background:url(data:image/png;base64,AAA)"), "background:url(data:image/png;base64,AAA)");
        assert_eq!(safecss_filter_attr("color:red;background:url(javascript:x)"), "color:red");
    }
}
