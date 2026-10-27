//! `core/icon`: an SVG from the core icon library.

use rustpress_core::formatting::esc_attr;
use serde_json::{Map, Value, json};

use crate::Result;
use crate::html::{TagEditor, tags};
use crate::render::BlockCtx;

/// The file of a core icon (`core/<name>`) in the icon library, if the library has it.
fn icon_file(includes: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let key = name.strip_prefix("core/")?;
    let manifest = rustpress_core::files::read(&includes.join("assets").join("icon-library-manifest.php"))?;
    let anchor = rustpress_core::formatting::cached_plain_regex(&format!(r"(?m)^\s*'{}'\s*=>", regex::escape(key)));
    let start = anchor.find(&manifest)?.start();
    let rest = &manifest[start..];
    let end = rest.find("
	),").unwrap_or(rest.len());
    let entry = &rest[..end];
    let file = entry.split("'filePath'").nth(1)?.split('\'').nth(1)?;
    Some(includes.join("images").join("icon-library").join(file))
}

/// `WP_Icons_Registry::sanitize_icon_content()`: `wp_kses()` with the SVG tags of the registry.
/// Only `svg`, `path` and `polygon` survive, with their allowed attributes, names in lower case.
fn sanitize(svg: &str) -> String {
    const ALLOWED: [(&str, &[&str]); 3] = [
        ("svg", &["class", "xmlns", "width", "height", "viewbox", "aria-hidden", "role", "focusable"]),
        ("path", &["fill", "fill-rule", "d", "transform"]),
        ("polygon", &["fill", "fill-rule", "points", "transform", "focusable"]),
    ];
    let mut out = String::new();
    let mut at = 0;
    for tag in tags(svg, 0) {
        out.push_str(&svg[at..tag.start]);
        at = tag.end;
        let Some((_, attrs)) = ALLOWED.iter().find(|(name, _)| *name == tag.name) else { continue };
        if tag.is_closer {
            out.push_str(&format!("</{}>", tag.name));
            continue;
        }
        out.push('<');
        out.push_str(&tag.name);
        for attr in &tag.attrs {
            let name = attr.name.to_ascii_lowercase();
            if attrs.contains(&name.as_str()) {
                match &attr.value {
                    Some(value) => out.push_str(&format!(" {name}=\"{}\"", esc_attr(value))),
                    None => out.push_str(&format!(" {name}")),
                }
            }
        }
        if svg[tag.start..tag.end].trim_end_matches('>').trim_end().ends_with('/') {
            out.push_str(" /");
        }
        out.push('>');
    }
    out.push_str(&svg[at..]);
    out
}

/// `wp_get_icon()` without a size.
fn get_icon(svg: &str, class: &str, label: &str) -> Option<String> {
    let tag = tags(svg, 0).into_iter().find(|t| !t.is_closer && t.name == "svg")?;
    let mut editor = TagEditor::from_tag(svg, tag);
    for name in class.split_ascii_whitespace() {
        editor.add_class(name);
    }
    if label.is_empty() {
        editor.set_attribute("aria-hidden", "true");
        editor.set_attribute("focusable", "false");
        editor.remove_attribute("role");
        editor.remove_attribute("aria-label");
    } else {
        editor.set_attribute("role", "img");
        editor.set_attribute("aria-label", label);
        editor.remove_attribute("aria-hidden");
        editor.remove_attribute("focusable");
    }
    Some(editor.finish())
}

fn string_attr<'a>(bc: &'a BlockCtx<'_>, key: &str) -> Option<&'a str> {
    bc.raw_attrs.get(key).and_then(Value::as_str)
}

/// `render_block_core_icon()`.
pub(super) fn icon(bc: &BlockCtx<'_>) -> Result<String> {
    let Some(name) = string_attr(bc, "icon").filter(|n| !n.is_empty()) else { return Ok(String::new()) };
    let style = bc.raw_attrs.get("style");
    let nested = |path: &[&str]| -> Value {
        let mut current = style;
        for key in path {
            current = current.and_then(|v| v.get(*key));
        }
        current.cloned().unwrap_or(Value::Null)
    };
    let preset = |key: &str| bc.raw_attrs.contains_key(key).then(|| format!("var:preset|color|{}", string_attr(bc, key).unwrap_or_default()));

    let text = preset("textColor").map(Value::from).unwrap_or_else(|| nested(&["color", "text"]));
    let background = preset("backgroundColor").map(Value::from).unwrap_or_else(|| nested(&["color", "background"]));
    let mut border = Map::new();
    for key in ["radius", "style", "width"] {
        let value = nested(&["border", key]);
        if !value.is_null() {
            border.insert(key.into(), value);
        }
    }
    border.insert("color".into(), preset("borderColor").map(Value::from).unwrap_or_else(|| nested(&["border", "color"])));
    for side in ["top", "right", "bottom", "left"] {
        border.insert(side.into(), json!({ "color": nested(&["border", side, "color"]), "style": nested(&["border", side, "style"]), "width": nested(&["border", side, "width"]) }));
    }
    let mut spacing = Map::new();
    if !nested(&["spacing", "padding"]).is_null() {
        spacing.insert("padding".into(), nested(&["spacing", "padding"]));
    }
    let mut dimensions = Map::new();
    if !nested(&["dimensions", "width"]).is_null() {
        dimensions.insert("width".into(), nested(&["dimensions", "width"]));
    }
    let styles = crate::style_engine::get_styles(
        &json!({ "color": { "text": text, "background": background }, "border": Value::Object(border), "spacing": Value::Object(spacing), "dimensions": Value::Object(dimensions) }),
        false,
    );

    let Some(block_type) = bc.block_type.as_deref() else { return Ok(String::new()) };
    let Some(includes) = block_type.dir.parent().and_then(std::path::Path::parent) else { return Ok(String::new()) };
    let Some(file) = icon_file(includes, name) else { return Ok(String::new()) };
    let Some(raw) = rustpress_core::files::read(&file) else { return Ok(String::new()) };
    let label = string_attr(bc, "ariaLabel").unwrap_or_default();
    let Some(mut svg) = get_icon(&sanitize(&raw), &styles.classnames, label) else { return Ok(String::new()) };

    if let Some(tag) = tags(&svg, 0).into_iter().find(|t| !t.is_closer && t.name == "svg") {
        let mut editor = TagEditor::from_tag(&svg, tag);
        if !styles.css.is_empty() {
            editor.set_attribute("style", &styles.css);
        }
        let flag = |key: &str| bc.raw_attrs.get(key).is_some_and(|v| v.as_bool().unwrap_or(false));
        if flag("flipHorizontal") {
            editor.add_class("is-flip-horizontal");
        }
        if flag("flipVertical") {
            editor.add_class("is-flip-vertical");
        }
        let rotation = bc.raw_attrs.get("rotation").and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(0);
        if rotation != 0 {
            let rotate = format!("rotate: {rotation}deg;");
            let current = editor.get_attribute("style").unwrap_or("").to_owned();
            editor.set_attribute("style", &if current.is_empty() { rotate } else { format!("{current} {rotate}") });
        }
        svg = editor.finish();
    }
    Ok(format!("<div {}>{svg}</div>", bc.wrapper_attributes(&[])))
}
