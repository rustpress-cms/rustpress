//! `core/button`: empty buttons vanish and the `style.dimensions.width` attribute becomes a style.

use serde_json::Value;

use crate::Result;
use crate::html::{TagEditor, tags};
use crate::render::BlockCtx;
use crate::style_engine::to_kebab_case;

const PRESET: &str = "var:preset|dimension|";

/// The size of the dimension preset `slug`, looked up in custom, theme and default order.
fn preset_size(bc: &BlockCtx<'_>, slug: &str) -> Option<String> {
    let settings = bc.env.theme.global_settings();
    let sizes = settings
        .get("blocks")
        .and_then(|b| b.get("core/button"))
        .and_then(|b| b.get("dimensions"))
        .and_then(|d| d.get("dimensionSizes"))
        .or_else(|| settings.get("dimensions").and_then(|d| d.get("dimensionSizes")))?;
    for origin in ["custom", "theme", "default"] {
        let Some(list) = sizes.get(origin).and_then(Value::as_array) else { continue };
        for preset in list {
            if preset.get("slug").and_then(Value::as_str) == Some(slug) {
                return preset.get("size").and_then(Value::as_str).map(str::to_owned);
            }
        }
    }
    None
}

/// PHP's `(float) "40%"`, printed the way string interpolation does.
fn php_float(text: &str) -> String {
    let text = text.trim_start();
    let mut end = 0;
    let mut seen_dot = false;
    for (i, c) in text.char_indices() {
        match c {
            '0'..='9' => end = i + 1,
            '.' if !seen_dot => seen_dot = true,
            '-' | '+' if i == 0 => {}
            _ => break,
        }
    }
    let value: f64 = text[..end].parse().unwrap_or(0.0);
    if value == value.trunc() && value.abs() < 1e15 { format!("{}", value as i64) } else { format!("{value}") }
}

/// `render_block_core_button()`.
pub(super) fn button(bc: &BlockCtx<'_>) -> Result<String> {
    let content = &bc.content;
    let all = tags(content, 0);
    let Some(link) = all.iter().find(|t| !t.is_closer && matches!(t.name.as_str(), "a" | "button")) else { return Ok(content.clone()) };

    // The button is empty when only comments sit between its opener and closer.
    let mut at = link.end;
    let empty = loop {
        let rest = &content[at..];
        if let Some(after) = rest.strip_prefix("<!--") {
            match after.find("-->") {
                Some(n) => at += 4 + n + 3,
                None => break false,
            }
            continue;
        }
        break match rest.strip_prefix("</") {
            Some(closer) => closer[..closer.find('>').unwrap_or(closer.len())].trim().eq_ignore_ascii_case(&link.name),
            None => false,
        };
    };
    if empty {
        return Ok(String::new());
    }

    let Some(width) = bc.attrs.get("style").and_then(|s| s.get("dimensions")).and_then(|d| d.get("width")).and_then(Value::as_str).filter(|w| !w.is_empty() && *w != "0") else {
        return Ok(content.clone());
    };
    let slug = width.strip_prefix(PRESET);
    let resolved = match slug {
        Some(slug) => preset_size(bc, slug).unwrap_or_else(|| width.to_owned()),
        None => width.to_owned(),
    };
    let Some(wrapper) = all.iter().find(|t| !t.is_closer && t.attr("class").is_some_and(|c| c.split_ascii_whitespace().any(|x| x == "wp-block-button"))) else {
        return Ok(content.clone());
    };
    let mut editor = TagEditor::from_tag(content, wrapper.clone());
    editor.add_class("has-custom-width");
    let existing = editor.get_attribute("style").map(str::to_owned).unwrap_or_default();
    let suffix = if existing.is_empty() { String::new() } else { format!(" {existing}") };
    let style = if resolved.ends_with('%') {
        editor.add_class("wp-block-button__width");
        let legacy = match resolved.as_str() {
            "25%" => Some("wp-block-button__width-25"),
            "50%" => Some("wp-block-button__width-50"),
            "75%" => Some("wp-block-button__width-75"),
            "100%" => Some("wp-block-button__width-100"),
            _ => None,
        };
        if let Some(legacy) = legacy {
            editor.add_class(legacy);
        }
        format!("--wp--block-button--width: {};", php_float(&resolved))
    } else {
        let css = match slug {
            Some(slug) => format!("var(--wp--preset--dimension--{})", to_kebab_case(slug)),
            None => width.to_owned(),
        };
        format!("width: {css};")
    };
    editor.set_attribute("style", &format!("{style}{suffix}"));
    Ok(editor.finish())
}

#[cfg(test)]
mod tests {
    use super::php_float;

    #[test]
    fn percentages_print_like_php_floats() {
        assert_eq!(php_float("40%"), "40");
        assert_eq!(php_float("37.5%"), "37.5");
        assert_eq!(php_float("abc"), "0");
    }
}
