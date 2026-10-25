//! Duotone filters (`class-wp-duotone.php`): a block with a duotone preset or colors gets a
//! `wp-duotone-<slug>` class, a CSS rule using the filter, and an SVG filter in the footer.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

use crate::env::Env;
use crate::general::php_float;
use crate::html::TagEditor;
use crate::registry::BlockType;
use crate::style_engine::{is_empty, sanitize_key};

/// What the page needs from the duotone filters it uses.
#[derive(Debug, Default, Clone)]
pub struct DuotoneState {
    /// `(selector, filter value)` in the order the blocks used them.
    pub css: Vec<(String, String)>,
    /// Presets used on the page: `(filter id, slug, colors or CSS value)`.
    pub presets: Vec<(String, String, Colors)>,
    /// SVG filters to print in the footer: `(filter id, colors)`.
    pub svgs: Vec<(String, Vec<String>)>,
}

/// The colors of a preset: a list, or a plain CSS filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Colors {
    List(Vec<String>),
    Css(String),
}

fn kebab(s: &str) -> String {
    crate::style_engine::to_kebab_case(s)
}

fn filter_id(slug: &str) -> String {
    format!("wp-duotone-{slug}")
}

/// `get_slug_from_attribute()`.
fn slug_from_attribute(attr: &str) -> String {
    static PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"var:preset\|duotone\|(\S+)|var\(--wp--preset--duotone--(\S+)\)").unwrap());
    PATTERN.captures(attr).and_then(|c| c.get(1).or_else(|| c.get(2))).map(|m| m.as_str().to_owned()).unwrap_or_default()
}

/// `get_all_global_styles_presets()`: filter id to `(slug, colors)`.
fn global_presets(env: &Env<'_>) -> Vec<(String, String, Colors)> {
    let mut out: Vec<(String, String, Colors)> = Vec::new();
    let Some(origins) = env.theme.global_settings().get("color").and_then(|c| c.get("duotone")) else { return out };
    let lists: Vec<&Value> = match origins {
        Value::Object(map) => map.values().collect(),
        Value::Array(_) => vec![origins],
        _ => Vec::new(),
    };
    for list in lists {
        for preset in list.as_array().into_iter().flatten() {
            let Some(slug) = preset.get("slug").and_then(Value::as_str) else { continue };
            let colors = match preset.get("colors") {
                Some(Value::Array(a)) => Colors::List(a.iter().filter_map(Value::as_str).map(str::to_owned).collect()),
                Some(Value::String(s)) => Colors::Css(s.clone()),
                _ => continue,
            };
            let id = filter_id(&kebab(slug));
            match out.iter_mut().find(|(i, _, _)| *i == id) {
                Some(slot) => *slot = (id, slug.to_owned(), colors),
                None => out.push((id, slug.to_owned(), colors)),
            }
        }
    }
    out
}

/// `get_all_global_style_block_names()`: blocks whose theme.json styles set a duotone preset.
fn global_style_blocks(env: &Env<'_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(blocks) = env.theme.global_styles().get("blocks").and_then(Value::as_object) else { return out };
    for (name, node) in blocks {
        let Some(attr) = node.get("filter").and_then(|f| f.get("duotone")).and_then(Value::as_str) else { continue };
        let slug = slug_from_attribute(attr);
        if !slug.is_empty() && slug != attr {
            out.push((name.clone(), slug));
        }
    }
    out
}

/// `WP_Theme_JSON::scope_selector( $scope, $selector )`.
fn scope_selector(scope: &str, selector: &str) -> String {
    selector.split(',').map(|part| format!("{} {}", scope.trim(), part.trim())).collect::<Vec<_>>().join(", ")
}

/// `get_selector()`: the CSS selector the filter applies to.
fn duotone_selector(bt: &BlockType) -> Option<String> {
    if !bt.has_support(&["filter", "duotone"], false) {
        return None;
    }
    let root = bt.selectors.get("root").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| format!(".wp-block-{}", bt.name.trim_start_matches("core/").replace('/', "-")));
    if let Some(experimental) = bt.support(&["color", "__experimentalDuotone"]).filter(|v| !is_empty(v)) {
        return Some(match experimental.as_str() {
            Some(selector) => scope_selector(&root, selector),
            None => root,
        });
    }
    match bt.selectors.get("filter").and_then(|f| f.get("duotone")) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Object(o)) => o.get("root").and_then(Value::as_str).map(str::to_owned),
        _ => Some(root),
    }
}

/// `colord_parse()` for hex and `rgb()` strings: `(r, g, b, a)`.
fn parse_color(input: &str) -> Option<(f64, f64, f64, f64)> {
    let input = input.trim();
    if let Some(hex) = input.strip_prefix('#') {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let expand = |s: &str| u8::from_str_radix(&s.repeat(2), 16).ok().map(f64::from);
        let pair = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok().map(f64::from);
        return match hex.len() {
            3 => Some((expand(&hex[0..1])?, expand(&hex[1..2])?, expand(&hex[2..3])?, 1.0)),
            4 => Some((expand(&hex[0..1])?, expand(&hex[1..2])?, expand(&hex[2..3])?, (expand(&hex[3..4])? / 255.0 * 100.0).round() / 100.0)),
            6 => Some((pair(0)?, pair(2)?, pair(4)?, 1.0)),
            8 => Some((pair(0)?, pair(2)?, pair(4)?, (pair(6)? / 255.0 * 100.0).round() / 100.0)),
            _ => None,
        };
    }
    static RGB: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^rgba?\(\s*([\d.]+)\s*[, ]\s*([\d.]+)\s*[, ]\s*([\d.]+)\s*(?:[,/]\s*([\d.]+)\s*)?\)$").unwrap());
    let c = RGB.captures(input)?;
    let n = |i: usize| c.get(i).and_then(|m| m.as_str().parse::<f64>().ok());
    Some((n(1)?.clamp(0.0, 255.0), n(2)?.clamp(0.0, 255.0), n(3)?.clamp(0.0, 255.0), n(4).unwrap_or(1.0).clamp(0.0, 1.0)))
}

/// `get_filter_svg()`.
pub fn filter_svg(id: &str, colors: &[String]) -> String {
    let (mut r, mut g, mut b, mut a) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for color in colors {
        if let Some((cr, cg, cb, ca)) = parse_color(color) {
            r.push(php_float(cr / 255.0));
            g.push(php_float(cg / 255.0));
            b.push(php_float(cb / 255.0));
            a.push(php_float(ca));
        }
    }
    let attr = |list: &[String]| rustpress_core::formatting::esc_attr(&list.join(" "));
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 0 0\" width=\"0\" height=\"0\" focusable=\"false\" role=\"none\" style=\"visibility: hidden; position: absolute; left: -9999px; overflow: hidden;\" ><defs><filter id=\"{}\"><feColorMatrix color-interpolation-filters=\"sRGB\" type=\"matrix\" values=\" .299 .587 .114 0 0 .299 .587 .114 0 0 .299 .587 .114 0 0 .299 .587 .114 0 0 \" /><feComponentTransfer color-interpolation-filters=\"sRGB\" ><feFuncR type=\"table\" tableValues=\"{}\" /><feFuncG type=\"table\" tableValues=\"{}\" /><feFuncB type=\"table\" tableValues=\"{}\" /><feFuncA type=\"table\" tableValues=\"{}\" /></feComponentTransfer><feComposite in2=\"SourceGraphic\" operator=\"in\" /></filter></defs></svg>",
        rustpress_core::formatting::esc_attr(id),
        attr(&r),
        attr(&g),
        attr(&b),
        attr(&a)
    )
}

/// `:root{--wp--preset--duotone--<slug>:url(#id);}` for the presets used on the page.
pub fn global_styles_presets(state: &DuotoneState) -> String {
    if state.presets.is_empty() {
        return String::new();
    }
    let mut css = String::from(":root{");
    for (id, slug, colors) in &state.presets {
        let value = match colors {
            Colors::Css(s) => s.clone(),
            Colors::List(_) => format!("url(#{id})"),
        };
        css.push_str(&format!("--wp--preset--duotone--{slug}:{value};"));
    }
    css.push('}');
    css
}

/// `WP_Duotone::render_duotone_support()`.
pub fn render_duotone_support(env: &Env<'_>, bt: &BlockType, name: &str, attrs: &Map<String, Value>, content: String) -> String {
    let Some(selector) = duotone_selector(bt) else { return content };
    let global_blocks = global_style_blocks(env);
    let attribute = attrs.get("style").and_then(|s| s.get("color")).and_then(|c| c.get("duotone")).filter(|v| !v.is_null());
    let global = global_blocks.iter().find(|(n, _)| n == name).map(|(_, slug)| slug.clone());
    if attribute.is_none() && global.is_none() {
        return content;
    }
    let presets = global_presets(env);
    let use_preset = |env: &Env<'_>, slug: &str| -> Option<String> {
        let id = filter_id(slug);
        let (_, preset_slug, colors) = presets.iter().find(|(i, _, _)| *i == id)?.clone();
        env.with_state(|s| {
            if !s.duotone.presets.iter().any(|(i, _, _)| *i == id) {
                s.duotone.presets.push((id.clone(), preset_slug, colors.clone()));
            }
            if let Colors::List(list) = &colors
                && !s.duotone.svgs.iter().any(|(i, _)| *i == id)
            {
                s.duotone.svgs.push((id.clone(), list.clone()));
            }
            s.duotone.css.push((scoped_duotone_selector(&id, &selector), format!("var(--wp--preset--duotone--{slug})")));
        });
        Some(id)
    };
    let id = if let Some(attr) = attribute {
        match attr {
            Value::String(s) => {
                let slug = slug_from_attribute(s);
                let is_preset = presets.iter().any(|(i, _, _)| *i == filter_id(&slug));
                if is_preset {
                    use_preset(env, &slug)
                } else {
                    let slug = env.with_state(|st| st.unique_id(&sanitize_key(&format!("{s}-"))));
                    let id = filter_id(&slug);
                    env.with_state(|st| st.duotone.css.push((scoped_duotone_selector(&id, &selector), s.clone())));
                    Some(id)
                }
            }
            Value::Array(colors) => {
                let list: Vec<String> = colors.iter().filter_map(Value::as_str).map(str::to_owned).collect();
                let slug = env.with_state(|st| st.unique_id(&sanitize_key(&format!("{}-", list.join("-")))));
                let id = filter_id(&slug);
                env.with_state(|st| {
                    st.duotone.svgs.push((id.clone(), list));
                    st.duotone.css.push((scoped_duotone_selector(&id, &selector), format!("url(#{id})")));
                });
                Some(id)
            }
            _ => None,
        }
    } else {
        global.and_then(|slug| use_preset(env, &slug))
    };
    let Some(id) = id else { return content };
    match TagEditor::first(&content) {
        Some(mut editor) => {
            editor.add_class(&id);
            editor.finish()
        }
        None => content,
    }
}

/// `enqueue_block_css()`: the filter id class in front of every part of the selector.
fn scoped_duotone_selector(id: &str, selector: &str) -> String {
    selector.split(',').map(|part| format!(".{id}{}", part.trim())).collect::<Vec<_>>().join(", ")
}

/// The footer markup: one SVG per filter used.
pub fn footer_svgs(state: &DuotoneState) -> String {
    state.svgs.iter().map(|(id, colors)| filter_svg(id, colors)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_become_table_values() {
        let svg = filter_svg("wp-duotone-x", &["#000000".into(), "#ff8800".into()]);
        assert!(svg.contains("tableValues=\"0 1\""));
        assert!(svg.contains("tableValues=\"0 0.53333333333333\""));
        assert!(svg.contains("tableValues=\"0 0\""));
    }

    #[test]
    fn selectors_are_scoped_per_part() {
        assert_eq!(scoped_duotone_selector("f", ".a img, .a .b"), ".f.a img, .f.a .b");
    }
}
