//! Port of `WP_Style_Engine`: turns the `style` attribute of a block (and the
//! preset attributes like `textColor`) into inline CSS declarations and class
//! names.

use serde_json::Value;

/// How a style value is turned into classes. `Fixed` always adds the class,
/// `Preset` adds a class with `$slug` replaced if the value is a preset reference.
#[derive(Clone, Copy)]
enum ClassSource {
    Fixed,
    Preset(&'static str),
}

#[derive(Clone, Copy, PartialEq)]
enum Func {
    None,
    UrlOrValue,
    IndividualProperty,
}

struct Def {
    group: &'static str,
    path: &'static [&'static str],
    property: &'static str,
    individual: Option<&'static str>,
    /// `(preset kind, css variable pattern)`.
    css_vars: &'static [(&'static str, &'static str)],
    classnames: &'static [(&'static str, ClassSource)],
    func: Func,
}

const fn def(
    group: &'static str,
    path: &'static [&'static str],
    property: &'static str,
    individual: Option<&'static str>,
    css_vars: &'static [(&'static str, &'static str)],
    classnames: &'static [(&'static str, ClassSource)],
) -> Def {
    Def { group, path, property, individual, css_vars, classnames, func: Func::None }
}

const fn side(group: &'static str, path: &'static [&'static str]) -> Def {
    Def {
        group,
        path,
        property: "",
        individual: None,
        css_vars: &[("color", "--wp--preset--color--$slug")],
        classnames: &[],
        func: Func::IndividualProperty,
    }
}

use ClassSource::{Fixed, Preset};

/// `WP_Style_Engine::BLOCK_STYLE_DEFINITIONS_METADATA`, in the same order.
static DEFINITIONS: &[Def] = &[
    Def {
        group: "background",
        path: &["background", "backgroundImage"],
        property: "background-image",
        individual: None,
        css_vars: &[],
        classnames: &[],
        func: Func::UrlOrValue,
    },
    def("background", &["background", "backgroundPosition"], "background-position", None, &[], &[]),
    def("background", &["background", "backgroundRepeat"], "background-repeat", None, &[], &[]),
    def("background", &["background", "backgroundSize"], "background-size", None, &[], &[]),
    def("background", &["background", "backgroundAttachment"], "background-attachment", None, &[], &[]),
    def(
        "background",
        &["background", "gradient"],
        "background-image",
        None,
        &[("gradient", "--wp--preset--gradient--$slug")],
        &[("has-background", Fixed)],
    ),
    def(
        "color",
        &["color", "text"],
        "color",
        None,
        &[("color", "--wp--preset--color--$slug")],
        &[("has-text-color", Fixed), ("has-$slug-color", Preset("color"))],
    ),
    def(
        "color",
        &["color", "background"],
        "background-color",
        None,
        &[("color", "--wp--preset--color--$slug")],
        &[("has-background", Fixed), ("has-$slug-background-color", Preset("color"))],
    ),
    def(
        "color",
        &["color", "gradient"],
        "background",
        None,
        &[("gradient", "--wp--preset--gradient--$slug")],
        &[("has-background", Fixed), ("has-$slug-gradient-background", Preset("gradient"))],
    ),
    def(
        "border",
        &["border", "color"],
        "border-color",
        Some("border-%s-color"),
        &[],
        &[("has-border-color", Fixed), ("has-$slug-border-color", Preset("color"))],
    ),
    def(
        "border",
        &["border", "radius"],
        "border-radius",
        Some("border-%s-radius"),
        &[("border-radius", "--wp--preset--border-radius--$slug")],
        &[],
    ),
    def("border", &["border", "style"], "border-style", Some("border-%s-style"), &[], &[]),
    def("border", &["border", "width"], "border-width", Some("border-%s-width"), &[], &[]),
    side("border", &["border", "top"]),
    side("border", &["border", "right"]),
    side("border", &["border", "bottom"]),
    side("border", &["border", "left"]),
    def("shadow", &["shadow"], "box-shadow", None, &[("shadow", "--wp--preset--shadow--$slug")], &[]),
    def("dimensions", &["dimensions", "aspectRatio"], "aspect-ratio", None, &[], &[("has-aspect-ratio", Fixed)]),
    def("dimensions", &["dimensions", "height"], "height", None, &[("dimension", "--wp--preset--dimension--$slug")], &[]),
    def("dimensions", &["dimensions", "minHeight"], "min-height", None, &[("dimension", "--wp--preset--dimension--$slug")], &[]),
    def("dimensions", &["dimensions", "minWidth"], "min-width", None, &[("dimension", "--wp--preset--dimension--$slug")], &[]),
    def("dimensions", &["dimensions", "objectFit"], "object-fit", None, &[], &[]),
    def("dimensions", &["dimensions", "width"], "width", None, &[("dimension", "--wp--preset--dimension--$slug")], &[]),
    def("spacing", &["spacing", "padding"], "padding", Some("padding-%s"), &[("spacing", "--wp--preset--spacing--$slug")], &[]),
    def("spacing", &["spacing", "margin"], "margin", Some("margin-%s"), &[("spacing", "--wp--preset--spacing--$slug")], &[]),
    def(
        "typography",
        &["typography", "fontSize"],
        "font-size",
        None,
        &[("font-size", "--wp--preset--font-size--$slug")],
        &[("has-$slug-font-size", Preset("font-size"))],
    ),
    def(
        "typography",
        &["typography", "fontFamily"],
        "font-family",
        None,
        &[("font-family", "--wp--preset--font-family--$slug")],
        &[("has-$slug-font-family", Preset("font-family"))],
    ),
    def("typography", &["typography", "fontStyle"], "font-style", None, &[], &[]),
    def("typography", &["typography", "fontWeight"], "font-weight", None, &[], &[]),
    def("typography", &["typography", "lineHeight"], "line-height", None, &[], &[]),
    def("typography", &["typography", "textColumns"], "column-count", None, &[], &[]),
    def("typography", &["typography", "textDecoration"], "text-decoration", None, &[], &[]),
    def("typography", &["typography", "textIndent"], "text-indent", None, &[], &[]),
    def("typography", &["typography", "textTransform"], "text-transform", None, &[], &[]),
    def("typography", &["typography", "letterSpacing"], "letter-spacing", None, &[], &[]),
    def("typography", &["typography", "writingMode"], "writing-mode", None, &[], &[]),
];

/// Result of [`get_styles`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Styles {
    /// `color:red;padding:1px;`
    pub css: String,
    /// Space-separated class names.
    pub classnames: String,
    pub declarations: Vec<(String, String)>,
}

impl Styles {
    pub fn declaration(&self, property: &str) -> Option<&str> {
        self.declarations.iter().find(|(p, _)| p == property).map(|(_, v)| v.as_str())
    }
}

/// `wp_style_engine_get_styles()`. `convert_vars_to_classnames` leaves preset
/// references out of the inline CSS (they become classes instead).
pub fn get_styles(block_styles: &Value, convert_vars_to_classnames: bool) -> Styles {
    let mut classnames: Vec<String> = Vec::new();
    let mut declarations: Vec<(String, String)> = Vec::new();
    let Some(styles) = block_styles.as_object() else { return Styles::default() };

    for def in DEFINITIONS {
        // `empty( $block_styles[ $group ] )` for the group key.
        let group_key = def.path[0];
        if styles.get(group_key).is_none_or(is_empty) {
            continue;
        }
        let Some(value) = path_get(block_styles, def.path) else { continue };
        if !is_valid_style_value(value) {
            continue;
        }
        classnames.extend(get_classnames(value, def));
        let new = get_css_declarations(value, def, convert_vars_to_classnames);
        for (property, mut v) in new {
            if property == "background-image"
                && let Some((_, existing)) = declarations.iter().find(|(p, _)| p == "background-image") {
                    v = format!("{v}, {existing}");
                }
            match declarations.iter_mut().find(|(p, _)| *p == property) {
                Some(slot) => slot.1 = v,
                None => declarations.push((property, v)),
            }
        }
    }

    let mut seen = Vec::new();
    classnames.retain(|c| {
        if seen.contains(c) {
            false
        } else {
            seen.push(c.clone());
            true
        }
    });
    let css = compile_declarations(&declarations);
    Styles { css, classnames: classnames.join(" "), declarations }
}

/// `prop:value;prop:value;` with the declaration filter applied.
pub fn compile_declarations(declarations: &[(String, String)]) -> String {
    let mut out = String::new();
    for (property, value) in declarations {
        let property = sanitize_key(property);
        let value = value.trim();
        if property.is_empty() || value.is_empty() {
            continue;
        }
        if let Some(clean) = safecss_filter_declaration(&property, &strip_tags(value)) {
            out.push_str(&clean);
            out.push(';');
        }
    }
    out
}

/// Minimal `safecss_filter_attr` for a single declaration: rejects values that
/// could break out of the attribute or execute script.
pub fn safecss_filter_declaration(property: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let lower = value.to_ascii_lowercase();
    let unsafe_value = value.contains('\\')
        || value.contains("/*")
        || value.contains("*/")
        || value.contains("&#")
        || value.contains('<')
        || value.contains('>')
        || has_top_level_semicolon(value)
        || lower.contains("expression(")
        || lower.contains("javascript:")
        || lower.contains("behavior:")
        || lower.contains("-moz-binding");
    if unsafe_value {
        return None;
    }
    if lower.contains("url(") {
        // Only plain http(s)/relative URLs.
        let inner = lower.split("url(").nth(1).unwrap_or("").trim_start_matches(['"', '\'', ' ']);
        if inner.contains(':') && !(inner.starts_with("http:") || inner.starts_with("https:") || inner.starts_with("data:image/")) {
            return None;
        }
    }
    Some(format!("{property}:{value}"))
}

/// A `;` outside of parentheses would start a second declaration.
fn has_top_level_semicolon(value: &str) -> bool {
    let mut depth = 0i32;
    for c in value.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ';' if depth <= 0 => return true,
            _ => {}
        }
    }
    false
}

fn strip_tags(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_tag = false;
    for c in value.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// `sanitize_key`: lowercase `[a-z0-9_-]`.
pub fn sanitize_key(key: &str) -> String {
    key.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect::<String>().to_ascii_lowercase()
}

/// `PHP empty()`.
pub fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty() || s == "0",
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
    }
}

fn is_valid_style_value(value: &Value) -> bool {
    matches!(value, Value::String(s) if s == "0") || !is_empty(value)
}

pub fn path_get<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

/// `_wp_to_kebab_case`.
pub fn to_kebab_case(input: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    for segment in input.split(|c: char| !c.is_ascii_alphanumeric()) {
        if segment.is_empty() {
            continue;
        }
        let chars: Vec<char> = segment.chars().collect();
        let mut current = String::new();
        for (i, &c) in chars.iter().enumerate() {
            let boundary = i > 0
                && ((c.is_ascii_uppercase() && chars[i - 1].is_ascii_lowercase())
                    || (c.is_ascii_uppercase()
                        && chars[i - 1].is_ascii_uppercase()
                        && chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase()))
                    || (c.is_ascii_digit() && !chars[i - 1].is_ascii_digit() && current.chars().all(|x| x.is_ascii_digit()) && false));
            if boundary && !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            current.push(c);
        }
        // A leading number followed by letters is two words (`2xl` -> `2`, `xl`).
        if !current.is_empty() {
            let digits = current.chars().take_while(char::is_ascii_digit).count();
            if digits > 0 && digits < current.len() {
                words.push(current[..digits].to_owned());
                words.push(current[digits..].to_owned());
            } else {
                words.push(current);
            }
        }
    }
    words.iter().map(|w| w.to_ascii_lowercase()).collect::<Vec<_>>().join("-")
}

/// `get_slug_from_preset_value`.
fn slug_from_preset(value: &str, kind: &str) -> String {
    let marker = format!("var:preset|{kind}|");
    if value.contains(&marker) {
        let slug = value.rsplit('|').next().unwrap_or("");
        return to_kebab_case(slug);
    }
    String::new()
}

fn css_var_value(value: &str, css_vars: &[(&str, &str)]) -> String {
    for (kind, pattern) in css_vars {
        let slug = slug_from_preset(value, kind);
        if !slug.is_empty() {
            return format!("var({})", pattern.replace("$slug", &slug));
        }
    }
    String::new()
}

fn get_classnames(value: &Value, def: &Def) -> Vec<String> {
    let mut out = Vec::new();
    for (class, source) in def.classnames {
        match source {
            Fixed => out.push((*class).to_owned()),
            Preset(kind) => {
                if let Some(s) = value.as_str() {
                    let slug = slug_from_preset(s, kind);
                    if !slug.is_empty() {
                        out.push(class.replace("$slug", &slug));
                    }
                }
            }
        }
    }
    out
}

/// A JSON scalar as PHP would output it in a declaration. Only strings are
/// accepted by `WP_Style_Engine_CSS_Declarations`, numbers are dropped.
fn as_declaration_string(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn get_css_declarations(value: &Value, def: &Def, skip_css_vars: bool) -> Vec<(String, String)> {
    match def.func {
        Func::UrlOrValue => {
            let mut out = Vec::new();
            let v = match value {
                Value::Object(o) if o.get("url").is_some_and(|u| !is_empty(u)) => {
                    Some(format!("url('{}')", o["url"].as_str().unwrap_or("")))
                }
                Value::String(s) => Some(s.clone()),
                _ => None,
            };
            if let Some(v) = v {
                out.push((def.property.to_owned(), v));
            }
            return out;
        }
        Func::IndividualProperty => return individual_property_declarations(value, def, skip_css_vars),
        Func::None => {}
    }

    let mut out = Vec::new();
    match value {
        Value::String(s) if s.contains("var:") => {
            if !skip_css_vars && !def.css_vars.is_empty() {
                let css_var = css_var_value(s, def.css_vars);
                if !css_var.is_empty() {
                    out.push((def.property.to_owned(), css_var));
                }
            }
        }
        Value::Object(map) => {
            let Some(individual) = def.individual else { return out };
            for (key, v) in map {
                let mut v = v.clone();
                if let Value::String(s) = &v
                    && s.contains("var:") && !skip_css_vars && !def.css_vars.is_empty() {
                        v = Value::String(css_var_value(s, def.css_vars));
                    }
                if is_valid_style_value(&v)
                    && let Some(s) = as_declaration_string(&v) {
                        out.push((individual.replace("%s", &to_kebab_case(key)), s));
                    }
            }
        }
        other => {
            if let Some(s) = as_declaration_string(other) {
                out.push((def.property.to_owned(), s));
            }
        }
    }
    out
}

/// `border.top` etc.: `{width, color, style}` becomes `border-top-width` ...
fn individual_property_declarations(value: &Value, def: &Def, skip_css_vars: bool) -> Vec<(String, String)> {
    let Some(map) = value.as_object().filter(|m| !m.is_empty()) else { return Vec::new() };
    let side = def.path[1];
    let mut out = Vec::new();
    for (css_property, v) in map {
        if is_empty(v) {
            continue;
        }
        let individual = DEFINITIONS
            .iter()
            .find(|d| d.group == "border" && d.path == ["border", css_property.as_str()])
            .and_then(|d| d.individual);
        let Some(individual) = individual else { continue };
        let mut v = v.clone();
        if let Value::String(s) = &v
            && s.contains("var:") && !skip_css_vars && !def.css_vars.is_empty() {
                v = Value::String(css_var_value(s, def.css_vars));
            }
        if let Some(s) = as_declaration_string(&v) {
            out.push((individual.replace("%s", side), s));
        }
    }
    out
}

/// `var:preset|spacing|60` -> `var(--wp--preset--spacing--60)`, other values unchanged.
pub fn spacing_preset_to_var(value: &str) -> String {
    if value.contains("var:preset|spacing|") {
        let slug = to_kebab_case(value.rsplit('|').next().unwrap_or(""));
        format!("var(--wp--preset--spacing--{slug})")
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn spacing_presets_become_css_variables() {
        let s = get_styles(&json!({"spacing": {"padding": {"top": "var:preset|spacing|60", "bottom": "1rem"}, "margin": {"top": "0"}}}), false);
        assert_eq!(s.css, "padding-top:var(--wp--preset--spacing--60);padding-bottom:1rem;margin-top:0;");
        assert_eq!(s.classnames, "");
    }

    #[test]
    fn preset_colors_become_classes_when_converting() {
        let s = get_styles(&json!({"color": {"text": "var:preset|color|contrast", "background": "#fff"}}), true);
        assert_eq!(s.classnames, "has-text-color has-contrast-color has-background");
        assert_eq!(s.css, "background-color:#fff;");
    }

    #[test]
    fn preset_colors_become_variables_otherwise() {
        let s = get_styles(&json!({"color": {"text": "var:preset|color|accent-1"}}), false);
        assert_eq!(s.css, "color:var(--wp--preset--color--accent-1);");
        assert_eq!(s.classnames, "has-text-color has-accent-1-color");
    }

    #[test]
    fn typography_and_borders() {
        let s = get_styles(
            &json!({"typography": {"fontSize": "var:preset|font-size|large", "fontWeight": "300", "lineHeight": 1.5},
                    "border": {"radius": "4px", "top": {"width": "1px", "color": "var:preset|color|accent-6"}}}),
            false,
        );
        assert_eq!(
            s.css,
            "border-radius:4px;border-top-width:1px;border-top-color:var(--wp--preset--color--accent-6);font-size:var(--wp--preset--font-size--large);font-weight:300;"
        );
        // Numbers are ignored, as in WordPress.
        assert!(s.declaration("line-height").is_none());
        assert_eq!(s.classnames, "has-large-font-size");
    }

    #[test]
    fn declaration_order_follows_the_definition_table() {
        let s = get_styles(&json!({"typography": {"fontSize": "1rem"}, "color": {"text": "red"}}), false);
        assert_eq!(s.css, "color:red;font-size:1rem;");
    }

    #[test]
    fn empty_groups_and_values_are_skipped() {
        assert_eq!(get_styles(&json!({"spacing": {}, "color": {"text": ""}}), false), Styles::default());
        assert_eq!(get_styles(&json!(null), false), Styles::default());
    }

    #[test]
    fn dangerous_values_are_dropped() {
        let s = get_styles(&json!({"color": {"text": "red;background:url(javascript:alert(1))"}}), false);
        assert_eq!(s.css, "");
    }

    #[test]
    fn kebab_case_matches_wordpress() {
        assert_eq!(to_kebab_case("60"), "60");
        assert_eq!(to_kebab_case("accent-1"), "accent-1");
        assert_eq!(to_kebab_case("x-large"), "x-large");
        assert_eq!(to_kebab_case("contentSize"), "content-size");
        assert_eq!(to_kebab_case("2xl"), "2-xl");
        assert_eq!(to_kebab_case("Accent_2"), "accent-2");
    }
}
