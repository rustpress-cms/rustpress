//! `WPForms\Frontend\CSSVars`: the CSS variables of the Modern render engine.

use serde_json::Value;

use super::php::{self, str_of};
use super::render::{disable_css, modern};
use super::settings::Settings;

/// `CSSVars::ROOT_VARS` and the medium sizes: `:root` of `init_vars()`.
pub const ROOT_VARS: &[(&str, &str)] = &[
    ("field-border-radius", "3px"),
    ("field-border-style", "solid"),
    ("field-border-size", "1px"),
    ("field-background-color", "#ffffff"),
    ("field-border-color", "rgba( 0, 0, 0, 0.25 )"),
    ("field-text-color", "rgba( 0, 0, 0, 0.7 )"),
    ("field-menu-color", "#ffffff"),
    ("label-color", "rgba( 0, 0, 0, 0.85 )"),
    ("label-sublabel-color", "rgba( 0, 0, 0, 0.55 )"),
    ("label-error-color", "#d63637"),
    ("button-border-radius", "3px"),
    ("button-border-style", "none"),
    ("button-border-size", "1px"),
    ("button-background-color", "#066aab"),
    ("button-border-color", "#066aab"),
    ("button-text-color", "#ffffff"),
    ("page-break-color", "#066aab"),
    ("background-image", "none"),
    ("background-position", "center center"),
    ("background-repeat", "no-repeat"),
    ("background-size", "cover"),
    ("background-width", "100px"),
    ("background-height", "100px"),
    ("background-color", "rgba( 0, 0, 0, 0 )"),
    ("background-url", "url()"),
    ("container-padding", "0px"),
    ("container-border-style", "none"),
    ("container-border-width", "1px"),
    ("container-border-color", "#000000"),
    ("container-border-radius", "3px"),
    ("field-size-input-height", "43px"),
    ("field-size-input-spacing", "15px"),
    ("field-size-font-size", "16px"),
    ("field-size-line-height", "19px"),
    ("field-size-padding-h", "14px"),
    ("field-size-checkbox-size", "16px"),
    ("field-size-sublabel-spacing", "5px"),
    ("field-size-icon-size", "1"),
    ("label-size-font-size", "16px"),
    ("label-size-line-height", "19px"),
    ("label-size-sublabel-font-size", "14px"),
    ("label-size-sublabel-line-height", "17px"),
    ("button-size-font-size", "17px"),
    ("button-size-height", "41px"),
    ("button-size-padding-h", "15px"),
    ("button-size-margin-top", "10px"),
    ("container-shadow-size-box-shadow", "none"),
];

/// `CSSVars::get_vars_css()`.
fn vars_css(vars: &[(String, String)]) -> String {
    let mut out = String::new();
    for (name, value) in vars {
        let value = if name == "background-url" && value == "url()" { "none" } else { value.as_str() };
        let value = if value == "0" { "0px" } else { value };
        out.push_str(&format!("--wpforms-{name}: {value};\n"));
        if name == "field-border-color" {
            out.push_str(&format!("--wpforms-{name}-spare: {value};\n"));
        }
    }
    out
}

/// `CSSVars::get_selector_vars_css()`.
fn selector_css(selector: &str, vars: &[(String, String)]) -> String {
    format!("{selector} {{\n\t\t\t\t{}\n\t\t\t}}", rustpress_core::formatting::wp_strip_all_tags(&vars_css(vars)))
}

/// `CSSVars::get_root_vars_css()`: the inline style of the full stylesheet.
pub fn root_css() -> String {
    let vars: Vec<(String, String)> = ROOT_VARS.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    selector_css(":root", &vars)
}

/// `CSSVars::FIELD_SIZE`, `LABEL_SIZE`, `BUTTON_SIZE` and `CONTAINER_SHADOW_SIZE`.
const SIZES: &[(&str, &str, &[(&str, &[(&str, &str)])])] = &[
    ("field", "fieldSize", &[
        ("small", &[("input-height", "31px"), ("input-spacing", "10px"), ("font-size", "14px"), ("line-height", "17px"), ("padding-h", "9px"), ("checkbox-size", "14px"), ("sublabel-spacing", "5px"), ("icon-size", "0.75")]),
        ("medium", &[("input-height", "43px"), ("input-spacing", "15px"), ("font-size", "16px"), ("line-height", "19px"), ("padding-h", "14px"), ("checkbox-size", "16px"), ("sublabel-spacing", "5px"), ("icon-size", "1")]),
        ("large", &[("input-height", "50px"), ("input-spacing", "20px"), ("font-size", "18px"), ("line-height", "21px"), ("padding-h", "14px"), ("checkbox-size", "18px"), ("sublabel-spacing", "10px"), ("icon-size", "1.25")]),
    ]),
    ("label", "labelSize", &[
        ("small", &[("font-size", "14px"), ("line-height", "17px"), ("sublabel-font-size", "13px"), ("sublabel-line-height", "16px")]),
        ("medium", &[("font-size", "16px"), ("line-height", "19px"), ("sublabel-font-size", "14px"), ("sublabel-line-height", "17px")]),
        ("large", &[("font-size", "18px"), ("line-height", "21px"), ("sublabel-font-size", "16px"), ("sublabel-line-height", "19px")]),
    ]),
    ("button", "buttonSize", &[
        ("small", &[("font-size", "14px"), ("height", "37px"), ("padding-h", "15px"), ("margin-top", "5px")]),
        ("medium", &[("font-size", "17px"), ("height", "41px"), ("padding-h", "15px"), ("margin-top", "10px")]),
        ("large", &[("font-size", "20px"), ("height", "48px"), ("padding-h", "20px"), ("margin-top", "15px")]),
    ]),
    ("container-shadow", "containerShadowSize", &[
        ("none", &[("box-shadow", "none")]),
        ("small", &[("box-shadow", "0px 3px 5px 0px rgba(0, 0, 0, 0.1)")]),
        ("medium", &[("box-shadow", "0px 10px 20px 0px rgba(0, 0, 0, 0.1)")]),
        ("large", &[("box-shadow", "0px 30px 50px -10px rgba(0, 0, 0, 0.15)")]),
    ]),
];

/// `CSSVars::get_color_as_rgb_array()`: the parts of a color as numbers (the alpha of a HEX color from 0 to 1).
fn rgb_array(color: &str) -> Vec<f64> {
    let color = color.trim().to_lowercase();
    let color = color.strip_prefix('#').unwrap_or(&color).replace(' ', "");
    let color = if color == "transparent" { "rgba(0,0,0,0)".to_owned() } else { color };
    static HEX: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"[0-9a-f]{6,8}$").expect("pattern"));
    static PAIR: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"[0-9A-Za-z_]{2}").expect("pattern"));
    if HEX.is_match(&color) {
        let mut parts: Vec<f64> = PAIR.find_iter(&color).map(|m| i64::from_str_radix(m.as_str(), 16).unwrap_or(0) as f64).collect();
        let alpha = parts.get(3).copied().unwrap_or(255.0) / 255.0;
        if parts.len() > 3 {
            parts[3] = alpha;
        } else {
            while parts.len() < 3 {
                parts.push(0.0);
            }
            parts.push(alpha);
        }
        return parts;
    }
    let kept: String = color.chars().filter(|c| c.is_ascii_digit() || matches!(c, ',' | '.')).collect();
    kept.split(',').map(php::leading_float).collect()
}

/// `CSSVars::is_transparent_color()`.
fn is_transparent(color: &str) -> bool {
    rgb_array(color).get(3).copied().unwrap_or(1.0) < 0.33
}

/// `CSSVars::get_contrast_color()`.
fn contrast(color: &str) -> &'static str {
    let rgba = rgb_array(color);
    let avg = ((rgba.iter().sum::<f64>() / 3.0) * rgba.get(3).copied().unwrap_or(1.0)) as i64;
    if avg < 128 { "#ffffff" } else { "#000000" }
}

/// `CSSVars::get_customized_css_vars()`: the variables of the attributes that differ from the defaults, the button
/// colors that keep the text readable and the variables of the size presets.
pub fn customized(attr: &serde_json::Map<String, Value>) -> Vec<(String, Value)> {
    let root = |name: &str| ROOT_VARS.iter().find(|(k, _)| *k == name).map(|(_, v)| *v);
    let mut vars: Vec<(String, Value)> = Vec::new();
    let put = |vars: &mut Vec<(String, Value)>, name: &str, value: Value| match vars.iter_mut().find(|(k, _)| k == name) {
        Some(slot) => slot.1 = value,
        None => vars.push((name.to_owned(), value)),
    };
    for (key, value) in attr {
        let mut name = String::new();
        for c in key.chars() {
            if c.is_ascii_uppercase() {
                name.push('-');
            }
            name.push(c);
        }
        let name = name.to_lowercase();
        let Some(default) = root(&name) else { continue };
        if default.is_empty() || value.as_str() == Some(default) {
            continue;
        }
        put(&mut vars, &name, value.clone());
    }
    let get = |vars: &Vec<(String, Value)>, name: &str| vars.iter().find(|(k, _)| k == name).map(|(_, v)| php::string(v)).unwrap_or_else(|| root(name).unwrap_or_default().to_owned());
    for key in ["field-border", "button-border"] {
        if vars.iter().any(|(k, v)| *k == format!("{key}-style") && v.as_str() == Some("none")) {
            put(&mut vars, &format!("{key}-size"), Value::String("0px".into()));
        }
    }
    let mut background = get(&vars, "button-background-color");
    if is_transparent(&background) {
        put(&mut vars, "button-background-color-alt", Value::String(background.clone()));
        let border = get(&vars, "button-border-color");
        let color = if is_transparent(&border) { root("button-background-color").unwrap_or_default().to_owned() } else { border };
        put(&mut vars, "button-background-color", Value::String(color.clone()));
        background = color;
    }
    let background = background.to_lowercase();
    let text = get(&vars, "button-text-color").to_lowercase();
    if background == text || is_transparent(&text) {
        put(&mut vars, "button-text-color-alt", Value::String(contrast(&background).to_owned()));
    }
    for (item, attr_name, presets) in SIZES {
        let Some(size) = attr.get(*attr_name).filter(|v| !php::empty(Some(v))) else { continue };
        let Some((_, values)) = presets.iter().find(|(name, _)| *name == php::string(size)) else { continue };
        for (key, value) in values.iter() {
            put(&mut vars, &format!("{item}-size-{key}"), Value::String((*value).to_owned()));
        }
    }
    vars
}

/// `CSSVars::add_css_vars_units()`.
fn add_units(vars: &mut [(String, Value)]) {
    for key in ["field-border-size", "field-border-radius", "button-border-size", "button-border-radius", "container-padding", "container-border-width", "container-border-radius"] {
        if let Some(slot) = vars.iter_mut().find(|(k, _)| k == key)
            && php::is_numeric(Some(&slot.1))
            && php::to_float(Some(&slot.1)) > 0.0
        {
            slot.1 = Value::String(format!("{}px", php::string(&slot.1)));
        }
    }
}

/// `ThemesData::get_theme()` of the themes of the Lite release and the site's own themes.
pub struct Themes {
    pub wpforms: serde_json::Map<String, Value>,
    pub custom: serde_json::Map<String, Value>,
}

impl Themes {
    /// `assets/lite/js/integrations/gutenberg/themes.json` of the plugin and `wpforms/themes/themes-custom.json` of
    /// the uploads.
    pub fn load(wp_root: &std::path::Path) -> Self {
        let read = |path: std::path::PathBuf| rustpress_core::files::read(&path).and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default();
        let content = wp_root.join("wp-content");
        Self { wpforms: read(content.join("plugins/wpforms-lite/assets/lite/js/integrations/gutenberg/themes.json")), custom: read(content.join("uploads/wpforms/themes/themes-custom.json")) }
    }

    pub fn get(&self, slug: &str) -> Option<&Value> {
        self.wpforms.get(slug).filter(|t| !php::empty(Some(t))).or_else(|| self.custom.get(slug).filter(|t| !php::empty(Some(t))))
    }
}

/// `<style id="...">` of `CSSVars::output_selector_vars()`.
fn style(id: &str, selector: &str, vars: &[(String, Value)]) -> String {
    let vars: Vec<(String, String)> = vars.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned()))).collect();
    format!("<style id=\"{}\">\n\t\t\t\t{}\n\t\t\t</style>", super::fields::sanitize_key(id), selector_css(selector, &vars))
}

/// `CSSVars::output_css_vars_for_shortcode()`.
pub fn for_shortcode(form_data: &Value, form_id: u64, settings: &Settings, themes: &Themes) -> String {
    if !modern(settings) || disable_css(settings) != 1 {
        return String::new();
    }
    let mut attr = form_data["settings"].get("themes").and_then(Value::as_object).cloned().unwrap_or_default();
    // `maybe_override_attributes()`: the settings of a named theme win.
    let slug = attr.get("wpformsTheme").map(php::string).unwrap_or_default();
    if !slug.is_empty() && slug != "0" {
        if let Some(url) = attr.get("backgroundUrl").map(php::string)
            && !url.starts_with("url(")
        {
            attr.insert("backgroundUrl".into(), Value::String(format!("url({url})")));
        }
        if let Some(Value::Object(theme)) = themes.get(&slug).and_then(|t| t.get("settings")) {
            attr.extend(theme.clone());
        }
    }
    let mut vars = customized(&attr);
    add_units(&mut vars);
    let mut out = style(&format!("wpforms-css-vars-{form_id}"), &format!("#wpforms-{form_id}"), &vars);
    let custom = str_of(&Value::Object(attr), "customCss");
    let custom = custom.trim();
    if !custom.is_empty() {
        out.push_str(&format!(
            "<style id=\"wpforms-css-vars-{form_id}-custom-css\">\n\t\t\t\t#wpforms-{form_id} {{\n\t\t\t\t\t{}\n\t\t\t\t}}\n\t\t\t</style>",
            rustpress_core::formatting::wp_strip_all_tags(custom)
        ));
    }
    out
}

/// What `FormSelector::get_content()` prints before the form of the block: `output_css_vars()`,
/// `output_form_custom_css()` (once per form and request: `first` says whether this is the first time) and
/// `output_custom_css()`. `attr` are the block's attributes after `maybe_override_block_attributes()`.
pub fn for_block(attr: &serde_json::Map<String, Value>, form_data: &Value, form_id: u64, settings: &Settings, first: bool) -> String {
    let mut out = String::new();
    if !modern(settings) {
        return out;
    }
    let client = attr.get("clientId").map(php::string).unwrap_or_default();
    let form = attr.get("formId").map(php::string).unwrap_or_default();
    if disable_css(settings) == 1 {
        let vars = customized(attr);
        if !vars.is_empty() {
            let id = format!("#wpforms-css-vars-{form}-block-{client}");
            let selector = format!("#wpforms-{form}.wpforms-block-{client}");
            out.push_str(&style(id.trim_end_matches('-'), selector.trim_end_matches('-'), &vars));
        }
    }
    let form_css = str_of(&form_data["settings"]["themes"], "customCss");
    if first && !form_css.trim().is_empty() {
        out.push_str(&format!(
            "<style id=\"{}\">\n\t\t\t\t#wpforms-{form_id} {{\n\t\t\t\t\t{}\n\t\t\t\t}}\n\t\t\t</style>",
            super::fields::sanitize_key(&format!("wpforms-custom-css-{form_id}-form")),
            rustpress_core::formatting::wp_strip_all_tags(form_css.trim())
        ));
    }
    let custom = attr.get("customCss").map(php::string).unwrap_or_default();
    if !custom.trim().is_empty() {
        out.push_str(&format!(
            "<style id=\"{}\">\n\t\t\t\t{}\n\t\t\t</style>",
            super::fields::sanitize_key(&format!("#wpforms-custom-css-{form}-block-{client}")),
            rustpress_core::formatting::wp_strip_all_tags(custom.trim())
        ));
    }
    out
}

/// `FormSelector::maybe_override_block_attributes()`.
pub fn override_block_attributes(mut attr: serde_json::Map<String, Value>, form_data: &Value, themes: &Themes) -> serde_json::Map<String, Value> {
    let mut slug = attr.get("theme").map(php::string).unwrap_or_default();
    let name = attr.get("themeName");
    if name.is_none() || (php::empty(name) && slug == "default") {
        return attr;
    }
    if slug.is_empty() {
        slug = str_of(&form_data["settings"]["themes"], "wpformsTheme");
        if php::empty(form_data["settings"]["themes"].get("wpformsTheme")) {
            slug = "default".into();
        }
    }
    if let Some(Value::Object(theme)) = themes.get(&slug).and_then(|t| t.get("settings")) {
        attr.extend(theme.clone());
    }
    attr
}
