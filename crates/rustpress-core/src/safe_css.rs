//! `safecss_filter_attr()`: the inline styles a `style` attribute may keep, and the CSS properties the style
//! engine lets through (the default of the `safe_style_css` filter). From wp-includes/kses.php.

use std::sync::OnceLock;

use regex::Regex;

/// Allowed property names, in WordPress' order (`--*` stands for custom properties).
pub const SAFE_STYLE_PROPERTIES: &[&str] = &[
    "background", "background-color", "background-image", "background-position", "background-repeat", "background-size", "background-attachment", "background-blend-mode",
    "border", "border-radius", "border-width", "border-color", "border-style", "border-right", "border-right-color", "border-right-style", "border-right-width",
    "border-bottom", "border-bottom-color", "border-bottom-left-radius", "border-bottom-right-radius", "border-bottom-style", "border-bottom-width",
    "border-left", "border-left-color", "border-left-style", "border-left-width", "border-top", "border-top-color", "border-top-left-radius", "border-top-right-radius",
    "border-top-style", "border-top-width", "border-spacing", "border-collapse", "caption-side",
    "columns", "column-count", "column-fill", "column-gap", "column-rule", "column-span", "column-width",
    "display", "color", "filter", "font", "font-family", "font-size", "font-style", "font-variant", "font-weight", "letter-spacing", "line-height",
    "text-align", "text-decoration", "text-indent", "text-transform", "white-space",
    "height", "min-height", "max-height", "width", "min-width", "max-width",
    "margin", "margin-right", "margin-bottom", "margin-left", "margin-top", "margin-block-start", "margin-block-end", "margin-inline-start", "margin-inline-end",
    "padding", "padding-right", "padding-bottom", "padding-left", "padding-top", "padding-block-start", "padding-block-end", "padding-inline-start", "padding-inline-end",
    "flex", "flex-basis", "flex-direction", "flex-flow", "flex-grow", "flex-shrink", "flex-wrap", "gap", "row-gap",
    "grid-template-columns", "grid-auto-columns", "grid-column-start", "grid-column-end", "grid-column", "grid-column-gap", "grid-template-rows", "grid-auto-rows",
    "grid-row-start", "grid-row-end", "grid-row", "grid-row-gap", "grid-gap",
    "justify-content", "justify-items", "justify-self", "align-content", "align-items", "align-self",
    "clear", "cursor", "direction", "float", "list-style-type", "object-fit", "object-position", "opacity", "overflow", "vertical-align", "writing-mode",
    "position", "top", "right", "bottom", "left", "z-index", "box-shadow", "aspect-ratio", "container-type",
    "fill", "fill-opacity", "fill-rule", "stroke", "stroke-dasharray", "stroke-dashoffset", "stroke-linecap", "stroke-linejoin", "stroke-miterlimit", "stroke-opacity", "stroke-width",
    "color-interpolation", "color-interpolation-filters", "paint-order", "stop-color", "stop-opacity", "flood-color", "flood-opacity", "lighting-color",
    "marker", "marker-end", "marker-mid", "marker-start", "clip-path", "clip-rule", "mask", "mask-type", "cx", "cy", "r", "rx", "ry", "x", "y", "d",
    "alignment-baseline", "baseline-shift", "dominant-baseline", "glyph-orientation-horizontal", "glyph-orientation-vertical", "text-anchor", "unicode-bidi", "word-spacing",
    "font-size-adjust", "font-stretch", "color-rendering", "image-rendering", "shape-rendering", "text-rendering", "vector-effect", "transform", "transform-origin",
    "pointer-events", "visibility",
];

/// Properties whose value may contain `url()`.
const URL_PROPERTIES: &[&str] = &["background", "background-image", "cursor", "filter", "list-style", "list-style-image", "clip-path", "fill", "marker", "marker-end", "marker-mid", "marker-start", "mask", "stroke"];

/// Properties whose value may contain a gradient.
const GRADIENT_PROPERTIES: &[&str] = &["background", "background-image"];

/// Whether a declaration with this property survives the style engine (custom properties always do).
pub fn is_safe_property(property: &str) -> bool {
    property.starts_with("--") || SAFE_STYLE_PROPERTIES.contains(&property)
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("a valid pattern"))
}

/// `safecss_filter_attr( $css )`: the declarations that are allowed, joined with `;` again.
pub fn safecss_filter_attr(css: &str) -> String {
    static CUSTOM: OnceLock<Regex> = OnceLock::new();
    static URL: OnceLock<Regex> = OnceLock::new();
    static URL_PIECES: OnceLock<Regex> = OnceLock::new();
    static GRADIENT: OnceLock<Regex> = OnceLock::new();
    static FORBIDDEN: OnceLock<Regex> = OnceLock::new();
    let css = crate::kses::no_null(css, true).replace(['\n', '\r', '\t'], "");
    let mut out = String::new();
    for item in css.trim().split(';') {
        if item.is_empty() {
            continue;
        }
        let item = item.trim();
        let mut test_string = item.to_owned();
        let mut found = false;
        let mut url_attr = false;
        let mut gradient_attr = false;
        let mut value = "";
        if !item.contains(':') {
            found = true;
        } else {
            let (selector, rest) = item.split_once(':').unwrap_or((item, ""));
            let selector = selector.trim();
            value = rest;
            let is_custom = re(&CUSTOM, r"^--[a-zA-Z0-9-_]+$").is_match(selector);
            if is_custom || SAFE_STYLE_PROPERTIES.contains(&selector) {
                found = true;
                url_attr = URL_PROPERTIES.contains(&selector);
                gradient_attr = GRADIENT_PROPERTIES.contains(&selector);
            }
            if is_custom {
                let trimmed = rest.trim();
                url_attr = trimmed.starts_with("url(");
                gradient_attr = trimmed.contains("-gradient(");
            }
        }
        if found && url_attr {
            for url_match in re(&URL, r"url\([^)]+\)").find_iter(value) {
                let whole = url_match.as_str();
                let pieces = re(&URL_PIECES, r#"^url\(\s*(['"]?)(.*)(['"]?)\s*\)$"#).captures(whole);
                let inner = pieces.as_ref().and_then(|c| c.get(2)).map(|m| m.as_str()).unwrap_or_default();
                // The quotes must match (`\g1` in WordPress).
                let (open, close) = pieces.as_ref().map(|c| (c.get(1).map_or("", |m| m.as_str()), c.get(3).map_or("", |m| m.as_str()))).unwrap_or_default();
                let inner = if open == close { inner.to_owned() } else { format!("{inner}{close}") };
                let url = inner.trim();
                if url.is_empty() || crate::kses::bad_protocol(url, &crate::kses::PROTOCOLS) != url {
                    found = false;
                    break;
                }
                test_string = test_string.replace(whole, "");
            }
        }
        if found && gradient_attr {
            for gradient in re(&GRADIENT, r"(?:repeating-)?(?:linear|radial|conic)-gradient\((?:[^()]|\([^()]*\))*\)").find_iter(&test_string.clone()) {
                test_string = test_string.replace(gradient.as_str(), "");
            }
        }
        if found {
            test_string = strip_value_functions(&test_string);
            // Nothing that could run or escape may be left.
            let allow = !re(&FORBIDDEN, r"[\\(&=}]|/\*").is_match(&test_string);
            if allow {
                if !out.is_empty() {
                    out.push(';');
                }
                out.push_str(item);
            }
        }
    }
    out
}

/// The value functions WordPress strips (with their nested parentheses) before the final check.
fn strip_value_functions(text: &str) -> String {
    const NAMES: &[&str] = &[
        "var", "calc", "min", "max", "minmax", "clamp", "repeat", "matrix", "matrix3d", "perspective", "rotate", "rotate3d", "rotateX", "rotateY", "rotateZ", "scale", "scale3d", "scaleX", "scaleY", "scaleZ",
        "skew", "skewX", "skewY", "translate", "translate3d", "translateX", "translateY", "translateZ", "circle", "ellipse", "inset", "path", "polygon", "rect", "shape", "xywh",
    ];
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let boundary = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let name = NAMES.iter().find(|n| boundary && rest.starts_with(*n) && rest[n.len()..].starts_with('('));
        if let Some(name) = name {
            // Balanced parentheses from the one after the name.
            let mut depth = 0usize;
            let mut end = None;
            for (offset, c) in rest[name.len()..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(name.len() + offset + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(end) = end {
                i += end;
                continue;
            }
        }
        let c = rest.chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::safecss_filter_attr;

    #[test]
    fn keeps_safe_declarations_and_drops_the_rest() {
        assert_eq!(safecss_filter_attr("color: red; behavior: url(x.htc)"), "color: red");
        assert_eq!(safecss_filter_attr("background-image: url(javascript:alert(1))"), "");
        assert_eq!(safecss_filter_attr("background-image: url('https://example.org/a.png')"), "background-image: url('https://example.org/a.png')");
        assert_eq!(safecss_filter_attr("width: calc(100% - 10px); color: expression(alert(1))"), "width: calc(100% - 10px)");
        assert_eq!(safecss_filter_attr("--my-var: var(--other); margin: 0"), "--my-var: var(--other);margin: 0");
        assert_eq!(safecss_filter_attr("background: linear-gradient(red, blue)"), "background: linear-gradient(red, blue)");
        assert_eq!(safecss_filter_attr("color: red /* x */"), "");
    }
}
