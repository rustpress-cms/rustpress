//! `CMPLZ_COOKIEBANNER`: a row of `{prefix}cmplz_cookiebanners` read with the defaults of
//! `cmplz_add_cookiebanner_settings()` (`parse_value()`), `get_html_settings()` and the banner HTML of
//! `cmplz_banner_loader::cookiebanner_html()`.
//!
//! The translations of WPML and Polylang (`translate()`, the `_x` copies of the texts) are not ported: the copies are
//! the texts themselves.

use rustpress_phpser::{Key, Value};

use super::options::{php_string, s, truthy, Options};

/// `cookiebanner/templates/cookiebanner.php` as `cmplz_get_template()` renders it in English, with `{consent_type}` left in.
const BANNER_TEMPLATE: &str = include_str!("cookiebanner.html");
/// `cookiebanner/templates/manage-consent.php`.
const MANAGE_CONSENT_TEMPLATE: &str = include_str!("manage-consent.html");

/// The type of a field of the banner (`get_field_type()`), and its default (`T` texts go through `__()`).
#[derive(Clone, Copy)]
enum Field {
    Text(&'static str),
    TextT(&'static str),
    Select(&'static str),
    Editor(&'static str),
    Checkbox(bool),
    Number(i64),
    NumberStr(&'static str),
    /// `text_checkbox`: `{text, show}`.
    TextCheckbox(&'static str),
    /// `colorpicker`, `borderradius`, `borderwidth`.
    Array(&'static [(&'static str, &'static str)]),
    Css,
    Other,
}

const MESSAGE: &str = "To provide the best experiences, we use technologies like cookies to store and/or access device information. Consenting to these technologies will allow us to process data such as browsing behavior or unique IDs on this site. Not consenting or withdrawing consent, may adversely affect certain features and functions.";

/// The fields of `cmplz_add_cookiebanner_settings()` the website reads.
const FIELDS: &[(&str, Field)] = &[
    ("title", Field::Text("")),
    ("use_categories", Field::Select("view-preferences")),
    ("manage_consent_options", Field::Select("hover-hide-mobile")),
    ("disable_cookiebanner", Field::Checkbox(false)),
    ("dismiss_on_scroll", Field::Checkbox(false)),
    ("dismiss_on_timeout", Field::Checkbox(false)),
    ("dismiss_timeout", Field::Number(10)),
    ("position", Field::Select("bottom-right")),
    ("animation", Field::Select("none")),
    ("banner_width", Field::NumberStr("526")),
    ("checkbox_style", Field::Select("slider")),
    ("legal_documents", Field::Checkbox(true)),
    ("use_logo", Field::Select("hide")),
    ("logo_attachment_id", Field::Other),
    ("close_button", Field::Checkbox(true)),
    ("use_box_shadow", Field::Checkbox(true)),
    ("header_footer_shadow", Field::Checkbox(false)),
    ("soft_cookiewall", Field::Checkbox(false)),
    ("colorpalette_border_radius", Field::Array(&[("top", "12"), ("right", "12"), ("bottom", "12"), ("left", "12"), ("type", "px")])),
    ("border_width", Field::Array(&[("top", "0"), ("right", "0"), ("bottom", "0"), ("left", "0")])),
    ("buttons_border_radius", Field::Array(&[("top", "6"), ("right", "6"), ("bottom", "6"), ("left", "6"), ("type", "px")])),
    ("font_size", Field::Number(12)),
    ("custom_css", Field::Css),
    ("revoke", Field::TextT("Manage consent")),
    ("header", Field::TextCheckbox("Manage Consent")),
    ("accept", Field::TextT("Accept")),
    ("accept_informational", Field::TextCheckbox("Accept")),
    ("dismiss", Field::TextCheckbox("Deny")),
    ("view_preferences", Field::TextT("View preferences")),
    ("save_preferences", Field::TextT("Save preferences")),
    ("message_optin", Field::Editor(MESSAGE)),
    ("category_functional", Field::TextT("Functional")),
    ("functional_text", Field::TextCheckbox("The technical storage or access is strictly necessary for the legitimate purpose of enabling the use of a specific service explicitly requested by the subscriber or user, or for the sole purpose of carrying out the transmission of a communication over an electronic communications network.")),
    ("category_prefs", Field::TextCheckbox("Preferences")),
    ("preferences_text", Field::TextCheckbox("The technical storage or access is necessary for the legitimate purpose of storing preferences that are not requested by the subscriber or user.")),
    ("category_stats", Field::TextCheckbox("Statistics")),
    ("statistics_text", Field::TextCheckbox("The technical storage or access that is used exclusively for statistical purposes.")),
    ("statistics_text_anonymous", Field::TextCheckbox("The technical storage or access that is used exclusively for anonymous statistical purposes. Without a subpoena, voluntary compliance on the part of your Internet Service Provider, or additional records from a third party, information stored or retrieved for this purpose alone cannot usually be used to identify you.")),
    ("category_all", Field::TextCheckbox("Marketing")),
    ("marketing_text", Field::TextCheckbox("The technical storage or access is required to create user profiles to send advertising, or to track the user on a website or across several websites for similar marketing purposes.")),
    ("message_optout", Field::Editor(MESSAGE)),
];

fn field(name: &str) -> Field {
    FIELDS.iter().find(|(n, _)| *n == name).map_or(Field::Other, |(_, f)| *f)
}

/// PHP's `empty()`.
fn empty(value: &Value) -> bool {
    !truthy(value)
}

fn array(items: Vec<(&str, Value)>) -> Value {
    Value::Array(items.into_iter().map(|(k, v)| (Key::Str(k.as_bytes().to_vec()), v)).collect())
}

/// `is_serialized()` + `unserialize()` of a column.
fn unserialized(value: &Value) -> Option<Value> {
    let text = value.as_str()?.trim();
    if text.len() < 4 || !matches!(text.as_bytes()[0], b'a' | b's' | b'i' | b'b' | b'd' | b'O' | b'N') {
        return None;
    }
    rustpress_phpser::unserialize(text.as_bytes()).ok()
}

/// One cookie banner as the website reads it.
pub struct Banner {
    pub id: String,
    pub banner_version: String,
    values: Vec<(String, Value)>,
}

impl Banner {
    /// `new CMPLZ_COOKIEBANNER( $id )` from the row of the table.
    pub fn from_row(row: &[(String, Option<String>)], options: &Options) -> Self {
        let mut banner = Banner { id: String::new(), banner_version: "0".into(), values: Vec::new() };
        for (column, value) in row {
            let value = value.as_ref().map_or(Value::Null, |v| s(v));
            match column.as_str() {
                "ID" => banner.id = php_string(&value),
                "banner_version" => banner.banner_version = php_string(&value),
                _ => {}
            }
            if FIELDS.iter().any(|(n, _)| n == column) {
                let parsed = parse_value(column, value, options);
                banner.values.push((column.clone(), parsed));
            }
        }
        if banner.string("use_categories") == "hidden" {
            banner.set("use_categories", s("view-preferences"));
        }
        banner
    }

    fn set(&mut self, name: &str, value: Value) {
        match self.values.iter_mut().find(|(n, _)| n == name) {
            Some((_, slot)) => *slot = value,
            None => self.values.push((name.to_owned(), value)),
        }
    }

    pub fn value(&self, name: &str) -> Value {
        self.values.iter().find(|(n, _)| n == name).map_or(Value::Null, |(_, v)| v.clone())
    }

    pub fn string(&self, name: &str) -> String {
        php_string(&self.value(name))
    }

    /// `get_banner_logo()`: the logo of the site and an image of the media library are not ported (empty).
    fn logo(&self, plugin_dir: &std::path::Path) -> String {
        match self.string("use_logo").as_str() {
            "complianz" => std::fs::read_to_string(plugin_dir.join("assets/images/poweredbycomplianz.svg")).unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// `get_html_settings()`: the texts put into the templates (`{field}`), each with its `['text']` if it has one.
    fn html_settings(&self, plugin_dir: &std::path::Path) -> Vec<(&'static str, String)> {
        let text = |name: &str| {
            let value = self.value(name);
            match &value {
                Value::Array(_) => super::options::item(&value, "text").map(php_string),
                // An array without a text is skipped; anything else is printed as a string.
                _ => Some(php_string(&value)),
            }
        };
        let fields: [(&'static str, &str); 22] = [
            ("header", "header"),
            ("accept_optin", "accept"),
            ("accept_optout", "accept_informational"),
            ("manage_consent", "revoke"),
            ("manage_options", "view_preferences"),
            ("save_settings", "save_preferences"),
            ("dismiss", "dismiss"),
            ("message_optout", "message_optout"),
            ("message_optin", "message_optin"),
            ("category_functional", "category_functional"),
            ("category_preferences", "category_prefs"),
            ("category_statistics", "category_stats"),
            ("functional_text", "functional_text"),
            ("statistics_text", "statistics_text"),
            ("statistics_text_anonymous", "statistics_text_anonymous"),
            ("preferences_text", "preferences_text"),
            ("marketing_text", "marketing_text"),
            ("category_marketing", "category_all"),
            ("position", "position"),
            ("manage_consent_options", "manage_consent_options"),
            ("use_categories", "use_categories"),
            ("", ""),
        ];
        let mut out = vec![("id", self.id.clone()), ("title_class", rustpress_core::slug::sanitize_title(&self.string("title"))), ("logo", self.logo(plugin_dir))];
        for (key, name) in fields.into_iter().filter(|(k, _)| !k.is_empty()) {
            if let Some(value) = text(name) {
                out.push((key, value));
            }
        }
        out
    }

    /// The banner and the manage consent button of one consent type (the loop of `cookiebanner_html()`).
    pub fn html(&self, consent_type: &str, options: &Options, plugin_dir: &std::path::Path) -> (String, String) {
        let mut banner = translated_template(options).replace("{consent_type}", consent_type);
        let mut manage = MANAGE_CONSENT_TEMPLATE.to_owned();
        for (field, value) in self.html_settings(plugin_dir) {
            let key = format!("{{{field}}}");
            let value = if field == "logo" { value } else { nl2br(&value) };
            banner = banner.replace(&key, &value);
            manage = manage.replace(&key, &value);
        }
        (banner, manage.replace("{consent_type}", consent_type))
    }
}

impl Banner {
    /// The categories of the banner for `cmplz_rest_api_manage_consent_html()`: the part between
    /// `<!-- categories start -->` and `<!-- categories end -->`, with the settings put in (without `nl2br()`).
    pub fn categories_html(&self, consent_type: &str, options: &Options, plugin_dir: &std::path::Path) -> String {
        let template = translated_template(options).replace("{consent_type}", consent_type);
        let (start, end) = ("<!-- categories start -->", "<!-- categories end -->");
        let Some(from) = template.find(start) else { return String::new() };
        let Some(to) = template[from..].find(end) else { return String::new() };
        let mut html = template[from..from + to + end.len()].to_owned();
        for (field, value) in self.html_settings(plugin_dir) {
            html = html.replace(&format!("{{{field}}}"), &value);
        }
        html
    }
}

/// The banner template with the texts it translates itself (`esc_html_e()`, `esc_attr_e()`).
fn translated_template(options: &Options) -> String {
    let esc_attr = |t: &str| rustpress_core::formatting::esc_attr(&options.tr(t));
    let esc_html = |t: &str| rustpress_core::formatting::esc_html(&options.tr(t));
    let vendors = rustpress_core::formatting::esc_html(&options.tr("Manage %s vendors").replacen("%s", "{vendor_count}", 1).replace("%1$s", "{vendor_count}"));
    BANNER_TEMPLATE
        .replace("aria-label=\"Close dialog\"", &format!("aria-label=\"{}\"", esc_attr("Close dialog")))
        .replace("\t\t\t\t\t\tAlways active\t", &format!("\t\t\t\t\t\t{}\t", esc_html("Always active")))
        .replace(">Manage options</a>", &format!(">{}</a>", esc_html("Manage options")))
        .replace(">Manage services</a>", &format!(">{}</a>", esc_html("Manage services")))
        .replace(">Manage {vendor_count} vendors</a>", &format!(">{vendors}</a>"))
        .replace("aria-label=\"Read more about TCF purposes on Cookie Database\"", &format!("aria-label=\"{}\"", esc_attr("Read more about TCF purposes on Cookie Database")))
        .replace(">Read more about these purposes</a>", &format!(">{}</a>", esc_html("Read more about these purposes")))
}

/// `CMPLZ_COOKIEBANNER::parse_value()` with `set_defaults` on.
fn parse_value(name: &str, value: Value, options: &Options) -> Value {
    match field(name) {
        Field::Text(default) | Field::Select(default) | Field::Editor(default) => {
            if empty(&value) {
                s(default)
            } else {
                value
            }
        }
        Field::TextT(default) => {
            let mut value = value;
            if name == "revoke"
                && let Some(old) = unserialized(&value) {
                    value = super::options::item(&old, "text").cloned().unwrap_or_else(|| s(&options.tr(default)));
                }
            if empty(&value) {
                s(&options.tr(default))
            } else {
                value
            }
        }
        Field::Checkbox(default) => match value {
            Value::Bool(false) => Value::Bool(default),
            v => v,
        },
        Field::Number(default) => {
            if empty(&value) {
                Value::Int(default)
            } else {
                Value::Int(php_string(&value).trim().parse::<f64>().map_or(0, |f| f as i64))
            }
        }
        Field::NumberStr(default) => {
            if empty(&value) {
                s(default)
            } else {
                Value::Int(php_string(&value).trim().parse::<f64>().map_or(0, |f| f as i64))
            }
        }
        Field::TextCheckbox(default) => {
            let defaults = vec![("text", s(&options.tr(default))), ("show", Value::Bool(true))];
            merge(name, value, defaults, false)
        }
        Field::Array(defaults) => merge(name, value, defaults.iter().map(|(k, v)| (*k, s(v))).collect(), true),
        Field::Css => {
            let text = if empty(&value) { String::new() } else { rustpress_core::formatting::html_entity_decode(&php_string(&value)) };
            if text.is_empty() || text == "0" {
                s("")
            } else {
                s(&text)
            }
        }
        Field::Other => value,
    }
}

/// The array types of `parse_value()`: unserialized, empty values dropped (not for `text_checkbox`), then the
/// defaults of missing keys and of empty ones.
fn merge(_name: &str, value: Value, defaults: Vec<(&str, Value)>, filter: bool) -> Value {
    let mut value = unserialized(&value).unwrap_or(value);
    if let Value::Array(items) = &value
        && items.iter().any(|(_, v)| unserialized(v).is_some()) {
            value = array(defaults.clone());
        }
    let mut items: Vec<(Key, Value)> = match value {
        Value::Array(items) => items,
        _ => Vec::new(),
    };
    if filter {
        items.retain(|(_, v)| !matches!(v, Value::Null | Value::Bool(false)) && !matches!(v, Value::Str(b) if b.is_empty()));
    }
    for (key, default) in defaults {
        let k = Key::Str(key.as_bytes().to_vec());
        match items.iter_mut().find(|(existing, _)| *existing == k) {
            None => items.push((k, default)),
            Some((_, slot)) => {
                if key != "show" && matches!(slot, Value::Str(b) if b.is_empty()) {
                    *slot = default;
                }
            }
        }
    }
    Value::Array(items)
}

/// `nl2br()`: `<br />` before every line break (`\r\n`, `\n\r`, `\n`, `\r`).
fn nl2br(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 16);
    let (mut i, mut start) = (0, 0);
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\n' || b == b'\r' {
            out.push_str(&text[start..i]);
            out.push_str("<br />");
            let pair = bytes.get(i + 1).is_some_and(|&n| (n == b'\n' || n == b'\r') && n != b);
            let end = if pair { i + 2 } else { i + 1 };
            out.push_str(&text[i..end]);
            i = end;
            start = end;
        } else {
            i += 1;
        }
    }
    out.push_str(&text[start..]);
    out
}
