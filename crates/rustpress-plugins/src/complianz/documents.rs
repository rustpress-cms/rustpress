//! The legal documents of `cmplz_document`: `get_document_html()` with the cookie policies of the free plugin
//! (`config/documents/cookie-policy-{region}.php`) and `cmplz_used_cookies()`, the list of the cookies of the
//! plugin's tables.
//!
//! `documents.json` (made by `dev/complianz-documents.sh`) holds the document elements with their translatable texts
//! kept as calls, and the fields of `cmplz_config::$fields`. The texts are translated here with the plugin's language
//! pack, as the plugin does when it loads the configuration. The premium documents (privacy statements, imprint,
//! disclaimer) have no elements in the free plugin: their pages hold the wrapper alone, as in WordPress. Annexes,
//! dropdowns and loop elements of `get_document_html()` only occur in premium documents and are left out; the
//! translations of WPML and Polylang (`cmplz_translate()`) are not ported.

use std::sync::OnceLock;

use rustpress_phpser::Value;
use serde_json::Value as Json;

use super::options::{Options, item, php_string, truthy};

const CONFIG: &str = include_str!("documents.json");

/// A piece of a text of the configuration: literal text or a recorded call.
enum Part {
    Text(String),
    /// `__( $text, 'complianz-gdpr' )`.
    Gettext(Vec<Part>),
    /// `_x( $text, $context, 'complianz-gdpr' )`.
    GettextX(Vec<Part>, Vec<Part>),
    /// `_n( $single, $plural, cmplz_count_socialmedia(), 'complianz-gdpr' )`.
    Ngettext(Vec<Part>, Vec<Part>),
    /// `cmplz_sprintf( $format, ...$args )`.
    Sprintf(Vec<Part>, Vec<Vec<Part>>),
}

#[derive(Default)]
struct Element {
    title: Option<Vec<Part>>,
    subtitle: Option<Vec<Part>>,
    content: Option<Vec<Part>>,
    condition: Vec<(String, String)>,
    callback_condition: Vec<String>,
    callback: Option<String>,
    /// `'p' => false`: the content is not wrapped in a paragraph.
    p: Option<bool>,
}

struct Document {
    region: String,
    kind: String,
    elements: Vec<(String, Element)>,
}

/// A field of `cmplz_config::$fields`.
struct Field {
    id: String,
    kind: String,
    options: Vec<(String, String)>,
    has_options: bool,
    document_label: Option<String>,
}

struct Config {
    documents: Vec<Document>,
    fields: Vec<Field>,
}

fn parts(json: &Json) -> Vec<Part> {
    let Some(list) = json.as_array() else {
        return json.as_str().map(|s| vec![Part::Text(s.to_owned())]).unwrap_or_default();
    };
    list.iter()
        .filter_map(|part| match part {
            Json::String(s) => Some(Part::Text(s.clone())),
            Json::Object(call) => {
                let (name, args) = call.iter().next()?;
                let args: Vec<Vec<Part>> = args.as_array()?.iter().map(parts).collect();
                let mut args = args.into_iter();
                let mut next = || args.next().unwrap_or_default();
                Some(match name.as_str() {
                    "__" => Part::Gettext(next()),
                    "_x" => Part::GettextX(next(), next()),
                    "_n" => Part::Ngettext(next(), next()),
                    _ => {
                        let format = next();
                        Part::Sprintf(format, args.collect())
                    }
                })
            }
            _ => None,
        })
        .collect()
}

fn config() -> &'static Config {
    static CONFIG_DATA: OnceLock<Config> = OnceLock::new();
    CONFIG_DATA.get_or_init(|| {
        let json: Json = serde_json::from_str(CONFIG).expect("valid documents.json");
        let documents = json["documents"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|doc| Document {
                region: doc["region"].as_str().unwrap_or_default().to_owned(),
                kind: doc["type"].as_str().unwrap_or_default().to_owned(),
                elements: doc["elements"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|pair| {
                        let key = pair[0].as_str().unwrap_or_default().to_owned();
                        let e = &pair[1];
                        let element = Element {
                            title: e.get("title").map(parts),
                            subtitle: e.get("subtitle").map(parts),
                            content: e.get("content").map(parts),
                            condition: e
                                .get("condition")
                                .and_then(Json::as_object)
                                .map(|c| c.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned())).collect())
                                .unwrap_or_default(),
                            callback_condition: match e.get("callback_condition") {
                                Some(Json::String(s)) => vec![s.clone()],
                                Some(Json::Array(list)) => list.iter().filter_map(Json::as_str).map(str::to_owned).collect(),
                                _ => Vec::new(),
                            },
                            callback: e.get("callback").and_then(Json::as_str).map(str::to_owned),
                            p: e.get("p").map(|p| p.as_bool().unwrap_or(true)),
                        };
                        (key, element)
                    })
                    .collect(),
            })
            .collect();
        let fields = json["fields"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| Field {
                id: f["id"].as_str().unwrap_or_default().to_owned(),
                kind: f["type"].as_str().unwrap_or_default().to_owned(),
                options: f.get("options").and_then(Json::as_object).map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned())).collect()).unwrap_or_default(),
                has_options: f.get("options").is_some(),
                document_label: f.get("document_label").and_then(Json::as_str).map(str::to_owned),
            })
            .collect();
        Config { documents, fields }
    })
}

/// `cmplz_count_socialmedia()`.
fn count_socialmedia(options: &Options) -> i64 {
    match options.get("socialmedia_on_site") {
        Value::Array(items) => items.iter().filter(|(_, v)| truthy(v)).count() as i64,
        v if truthy(&v) => 1,
        _ => 0,
    }
}

/// `cmplz_sprintf()`: PHP's `sprintf()` when the number of `%s` placeholders matches the arguments, else the format
/// as it is (the link to the "translation error" page, which only administrators see, is left out).
pub(super) fn cmplz_sprintf(format: &str, args: &[&str]) -> String {
    static PLACEHOLDER: OnceLock<regex::Regex> = OnceLock::new();
    let count = PLACEHOLDER.get_or_init(|| regex::Regex::new(r"%(?:\d+\$)?s").expect("valid regex")).find_iter(format).count();
    if count == args.len() { rustpress_core::i18n::sprintf(format, args) } else { format.to_owned() }
}

fn eval(parts: &[Part], options: &Options) -> String {
    let mut out = String::new();
    for part in parts {
        match part {
            Part::Text(text) => out.push_str(text),
            Part::Gettext(text) => out.push_str(&options.tr(&eval(text, options))),
            Part::GettextX(text, context) => out.push_str(&options.tr_x(&eval(text, options), &eval(context, options))),
            Part::Ngettext(single, plural) => out.push_str(&options.tr_n(&eval(single, options), &eval(plural, options), count_socialmedia(options))),
            Part::Sprintf(format, args) => {
                let args: Vec<String> = args.iter().map(|a| eval(a, options)).collect();
                out.push_str(&cmplz_sprintf(&eval(format, options), &args.iter().map(String::as_str).collect::<Vec<_>>()));
            }
        }
    }
    out
}

/// What a document needs from the site besides the options.
pub(super) struct Inputs<'a> {
    pub options: &'a Options,
    /// The functions of the `callback_condition`s; `None` for a function that does not exist.
    pub callback: &'a dyn Fn(&str) -> Option<bool>,
    /// `get_page_url( $type, $region )` and `get_document_title( $type, $region )`.
    pub page_url: &'a dyn Fn(&str, &str) -> String,
    pub document_title: &'a dyn Fn(&str, &str) -> String,
    /// `cmplz_used_cookies()`.
    pub used_cookies: &'a dyn Fn() -> String,
    /// `antispambot()`.
    pub antispambot: &'a dyn Fn(&str) -> String,
    /// `accept_x` and `save_preferences_x` of the default banner.
    pub accept_text: String,
    pub save_preferences_text: String,
    pub home_url: String,
    pub site_url: String,
    /// `cmplz_localize_date()` of the options `cmplz_publish_date` and `cmplz_documents_update_date`, and
    /// `get_last_cookie_sync_date()`.
    pub publish_date: String,
    pub checked_date: String,
    pub sync_date: String,
}

/// The document types and regions of `cmplz_config::$pages` (the free plugin has elements for the cookie policies
/// only).
const PAGES: &[(&str, &[&str])] = super::PAGES;

/// `get_document_html( $type, $region )` up to `wp_kses()` with `cmplz_allowed_html()`; the caller runs
/// `do_shortcode()` on it.
pub(super) fn document_html(kind: &str, region: &str, inputs: &Inputs<'_>) -> String {
    let options = inputs.options;
    let (kind, region) = if region.is_empty() {
        let region = region_from_legacy_type(kind);
        (kind.replace(&format!("-{region}"), ""), region)
    } else {
        (kind.to_owned(), region.to_owned())
    };
    let known = PAGES.iter().any(|(r, kinds)| *r == region && kinds.contains(&kind.as_str()));
    if !options.has_region(&region) || !known {
        return cmplz_sprintf(&options.tr("Region %1$s not activated for %2$s."), &[&region.to_uppercase(), &kind]);
    }
    let elements: &[(String, Element)] = config().documents.iter().find(|d| d.region == region && d.kind == kind).map_or(&[], |d| &d.elements);
    let mut html = String::new();
    let mut paragraph = 0;
    let mut sub_paragraph = 0;
    let mut paragraph_ids: Vec<(&str, i64)> = Vec::new();
    for (id, element) in elements {
        let insert = callback_condition_applies(element, inputs) && condition_applies(element, options);
        if insert {
            if element.title.is_some() {
                sub_paragraph = 0;
                paragraph += 1;
                paragraph_ids.push((id, paragraph));
            }
            if element.subtitle.is_some() && paragraph > 0 {
                sub_paragraph += 1;
                paragraph_ids.retain(|(i, _)| i != id);
                paragraph_ids.push((id, paragraph));
            }
            html.push_str(&wrap_header(element, paragraph, sub_paragraph, options));
            if let Some(content) = &element.content {
                html.push_str(&wrap_content(&eval(content, options), element));
            }
        }
        if element.callback.as_deref() == Some("cmplz_used_cookies") {
            html.push_str(&(inputs.used_cookies)());
        }
    }
    let html = replace_fields(html, &paragraph_ids, &region, inputs);
    let html = format!(
        "\n<!-- Legal document generated by Complianz | GDPR/CCPA Cookie Consent https://wordpress.org/plugins/complianz-gdpr -->\n<div id=\"cmplz-document\" class=\"cmplz-document {kind} cmplz-document-{region}\">{html}</div>"
    );
    kses(&html)
}

/// `wp_kses( $html, cmplz_allowed_html() )`.
pub(super) fn kses(html: &str) -> String {
    use rustpress_core::kses::{Allowed, kses};
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    let allowed = ALLOWED.get_or_init(|| {
        Allowed::tags(&[
            ("a", &["class", "href", "rel", "title", "target", "id"]),
            ("button", &["id", "class", "href", "rel", "title", "target", "aria-expanded", "aria-controls"]),
            ("b", &[]),
            ("br", &[]),
            ("blockquote", &["cite"]),
            ("div", &["class", "id"]),
            ("h1", &[]),
            ("h2", &[]),
            ("h3", &[]),
            ("h4", &[]),
            ("h5", &[]),
            ("h6", &[]),
            ("i", &[]),
            ("input", &["type", "class", "name", "id", "required", "value", "placeholder", "data-category", "data-service", "style"]),
            ("img", &["alt", "class", "height", "src", "width"]),
            ("label", &["for", "class", "style"]),
            ("li", &["class", "id"]),
            ("ol", &["class", "id"]),
            ("p", &["class", "id"]),
            ("span", &["class", "title", "style", "id"]),
            ("strong", &[]),
            ("table", &["class", "id"]),
            ("tr", &[]),
            ("details", &["class", "id"]),
            ("summary", &["class", "id"]),
            ("svg", &["width", "height", "viewBox"]),
            ("polyline", &["points"]),
            ("path", &["d"]),
            ("style", &[]),
            ("ul", &["class", "id"]),
            ("form", &["id"]),
        ])
    });
    kses(html, allowed)
}

/// `cmplz_get_region_from_legacy_type( $type )`.
pub(super) fn region_from_legacy_type(kind: &str) -> String {
    let mut region = if kind.contains("disclaimer") || kind.contains("all") { "all".to_owned() } else { String::new() };
    let bytes = kind.as_bytes();
    if bytes.len() >= 3 && bytes[bytes.len() - 3] == b'-' {
        region = String::from_utf8_lossy(&bytes[bytes.len() - 2..]).into_owned();
    }
    if region.is_empty() {
        region = "eu".into();
    }
    region
}

/// `callback_condition_applies()`: a function that does not exist ends the check (the element is inserted).
fn callback_condition_applies(element: &Element, inputs: &Inputs<'_>) -> bool {
    for func in &element.callback_condition {
        let invert = func.contains("NOT ");
        let name = func.replace("NOT ", "");
        let Some(show) = (inputs.callback)(&name) else {
            break;
        };
        if show == invert {
            return false;
        }
    }
    true
}

/// `condition_applies()` for the options (no post), which reads them without their defaults.
fn condition_applies(element: &Element, options: &Options) -> bool {
    let mut met = true;
    for (question, answer) in &element.condition {
        if answer == "loop" {
            continue;
        }
        let Some(field) = config().fields.iter().find(|f| f.id == *question) else {
            continue;
        };
        // `cmplz_get_option( $question, $post_id )` with `$post_id` false: no default.
        let value = options.stored(question);
        let invert = answer.contains("NOT ");
        let answer = answer.replace("NOT ", "");
        let current = if answer == "EMPTY" {
            match &value {
                Value::Array(items) if !items.is_empty() => false,
                v => !truthy(v),
            }
        } else if field.kind == "multicheckbox" {
            matches!(&value, Value::Array(items) if items.iter().any(|(_, v)| loose_eq(v, &answer)))
        } else {
            loose_eq(&value, &answer)
        };
        met = met && (current != invert);
    }
    met
}

/// PHP 8's `$value == $string`.
fn loose_eq(value: &Value, text: &str) -> bool {
    let numeric = |s: &str| s.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']).trim_end_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']).parse::<f64>().ok();
    match value {
        Value::Null => text.is_empty(),
        Value::Bool(b) => *b == !(text.is_empty() || text == "0"),
        Value::Int(n) => numeric(text).map_or_else(|| n.to_string() == text, |t| t == *n as f64),
        Value::Float(f) => numeric(text).map_or_else(|| php_string(value) == text, |t| t == *f),
        Value::Str(bytes) => {
            let s = String::from_utf8_lossy(bytes);
            match (numeric(&s), numeric(text)) {
                (Some(a), Some(b)) => a == b,
                _ => s == text,
            }
        }
        _ => false,
    }
}

fn esc_html(text: &str) -> String {
    rustpress_core::formatting::esc_html(text)
}

/// `wrap_header()` of a title or subtitle.
fn wrap_header(element: &Element, paragraph: i64, sub_paragraph: i64, options: &Options) -> String {
    if let Some(title) = &element.title {
        let title = eval(title, options);
        if title.is_empty() || title == "0" {
            return String::new();
        }
        let nr = if paragraph > 0 { format!("{paragraph}. ") } else { String::new() };
        return format!("<h2>{}{}</h2>", esc_html(&nr), esc_html(&title));
    }
    if let Some(subtitle) = &element.subtitle {
        let nr = if paragraph > 0 && sub_paragraph > 0 { format!("{paragraph}.{sub_paragraph} ") } else { String::new() };
        return format!("<h3 class=\"cmplz-subtitle\">{}{}</h3>", esc_html(&nr), esc_html(&eval(subtitle, options)));
    }
    String::new()
}

/// `wrap_content()`.
fn wrap_content(content: &str, element: &Element) -> String {
    if content.is_empty() || content == "0" {
        return String::new();
    }
    if element.p == Some(false) {
        return content.to_owned();
    }
    format!("<p >{content}</p>")
}

/// `cmplz_localize_date( $unix_time )` with the site's date format: the month and weekday names translated with the
/// default text domain. `date()` runs in UTC, the time zone WordPress sets for PHP.
pub(super) fn localize_date(unix_time: i64, date_format: &str, translate: &dyn Fn(&str) -> String) -> String {
    use chrono::{FixedOffset, TimeZone};
    let utc = FixedOffset::east_opt(0).expect("valid offset");
    let Some(moment) = utc.timestamp_opt(unix_time, 0).single() else {
        return String::new();
    };
    let date = |format: &str| rustpress_core::datetime::php_date(format, &moment, "UTC", "UTC", false);
    let formatted = date(date_format);
    let month = date("F");
    let formatted = formatted.replace(&month, &translate(&month));
    let weekday = date("l");
    formatted.replace(&weekday, &translate(&weekday))
}

/// `replace_fields()` (the annexes are left out: no free document has one).
fn replace_fields(mut html: String, paragraph_ids: &[(&str, i64)], region: &str, inputs: &Inputs<'_>) -> String {
    let options = inputs.options;
    for (id, main) in paragraph_ids {
        let key = format!("[article-{id}]");
        if html.contains(&key) {
            html = html.replace(&key, &cmplz_sprintf(&options.tr("(See paragraph %s)"), &[&main.to_string()]));
        }
    }
    html = html.replace("[cookie_accept_text]", &inputs.accept_text);
    html = html.replace("[cookie_save_preferences_text]", &inputs.save_preferences_text);
    let home = rustpress_core::formatting::esc_url_raw(&inputs.home_url);
    html = html.replace("[domain]", &format!("<a href=\"{home}\">{home}</a>"));
    html = html.replace("[cookie-statement-url]", &(inputs.page_url)("cookie-statement", region));
    html = html.replace("[privacy-statement-url]", &(inputs.page_url)("privacy-statement", region));
    html = html.replace("[privacy-statement-children-url]", &(inputs.page_url)("privacy-statement-children", region));
    html = html.replace("[site_url]", &inputs.site_url);
    html = html.replace("[cookie-statement-title]", &esc_html(&(inputs.document_title)("cookie-statement", region)));
    html = html.replace("[publish_date]", &esc_html(&inputs.publish_date));
    html = html.replace("[sync_date]", &esc_html(&inputs.sync_date));
    html = html.replace("[checked_date]", &esc_html(&inputs.checked_date));
    let by_phone = |phone: &str| {
        cmplz_sprintf(&options.tr_x("or by telephone on %s", "if phonenumber is entered, this string is part of the sentence \"you may contact %s, via %s or by telephone via %s\""), &[phone])
    };
    let phone_dpo = options.get("phone_dpo");
    let mut contact = options.string("email_dpo");
    if truthy(&phone_dpo) {
        contact = format!("{contact} {}", by_phone(&php_string(&phone_dpo)));
    }
    html = html.replace("[email_dpo]", &contact);
    // The plugin looks at `phone_dpo` for the UK contact as well.
    let mut contact_uk = options.string("email_dpo_uk");
    if truthy(&phone_dpo) {
        contact_uk = format!("{contact_uk} {}", by_phone(&options.string("phone_dpo_uk")));
    }
    html = html.replace("[email_dpo_uk]", &contact_uk);
    for field in &config().fields {
        let key = format!("[{}]", field.id);
        if html.contains(&key) {
            html = html.replace(&key, &plain_text_value(field, true, inputs));
            html = html.replace(&format!("[/{}]", field.id), "</a>");
        }
        let key = format!("[comma_{}]", field.id);
        if html.contains(&key) {
            html = html.replace(&key, &plain_text_value(field, false, inputs));
        }
    }
    html
}

/// `get_plain_text_value( $field, $list_style )`.
fn plain_text_value(field: &Field, list_style: bool, inputs: &Inputs<'_>) -> String {
    let options = inputs.options;
    let raw = options.get(&field.id);
    let option = |key: &str| field.options.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    let value = match field.kind.as_str() {
        "url" => format!("<a href=\"{}\" target=\"_blank\">", php_string(&raw)),
        "email" => obfuscate_email(&php_string(&raw), inputs.antispambot),
        "radio" => option(&php_string(&raw)).unwrap_or_default(),
        "textarea" => nl2br(&php_string(&raw)),
        _ => match &raw {
            Value::Array(items) => {
                let labels: Vec<String> = items.iter().filter_map(|(_, v)| option(&php_string(v))).collect();
                if list_style {
                    format!("<ul>{}</ul>", labels.iter().map(|l| format!("<li>{}</li>", esc_html(l))).collect::<String>())
                } else {
                    let joined: String = labels.iter().map(|l| format!("{l}, ")).collect();
                    let joined = esc_html(joined.trim_end_matches([',', ' ']));
                    match joined.rfind(',') {
                        Some(at) => format!("{} {}{}", &joined[..at], options.tr("and"), &joined[at + 1..]),
                        None => joined,
                    }
                }
            }
            v if field.has_options => {
                let v = php_string(v);
                option(&v).unwrap_or(v)
            }
            v => php_string(v),
        },
    };
    match &field.document_label {
        Some(label) if !value.is_empty() && value != "0" => {
            format!("{}{value}", options.tr(label.trim_end()) + " ")
        }
        _ => value,
    }
}

/// `nl2br()`.
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

/// `obfuscate_email()`: the address in parts through `antispambot()`, with a fake domain the stylesheet hides.
fn obfuscate_email(email: &str, antispambot: &dyn Fn(&str) -> String) -> String {
    if email.is_empty() || email == "0" {
        return String::new();
    }
    let (user, domain) = email.split_once('@').unwrap_or((email, ""));
    let domain = domain.split('@').next().unwrap_or_default();
    format!("<span class=\"cmplz-obfuscate\">{}@<span class=\"cmplz-fmail-domain\">{}</span>{}</span>", antispambot(user), antispambot("ex.com"), antispambot(domain))
}

/// `antispambot( $email )`: each character at random as a decimal entity or as itself, then `@` as `&#64;`.
pub(super) fn antispambot(text: &str) -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut out = String::new();
    for ch in text.chars() {
        if rng.random_range(0..2) == 0 {
            out.push_str(&format!("&#{};", ch as u32));
        } else {
            out.push(ch);
        }
    }
    out.replace('@', "&#64;")
}

/// A row of `{prefix}cmplz_cookies` or `{prefix}cmplz_services`: its columns as `$wpdb` returns them.
pub(super) type Row = Vec<(String, Option<String>)>;

fn col<'r>(row: &'r Row, name: &str) -> Option<&'r str> {
    row.iter().find(|(c, _)| c == name).and_then(|(_, v)| v.as_deref())
}

/// `(bool) $row->column`.
fn flag(row: &Row, name: &str) -> bool {
    col(row, name).is_some_and(|v| !v.is_empty() && v != "0")
}

/// `empty( $value )` of a string column.
fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.is_empty() || v == "0")
}

/// `html_entity_decode( $text, ENT_QUOTES )`.
fn decode(text: &str) -> String {
    rustpress_core::formatting::html_entity_decode(text).replace("&#039;", "'").replace("&#39;", "'").replace("&#x27;", "'")
}

/// `CMPLZ_SERVICE` loaded by its ID.
#[derive(Default)]
struct Service {
    found: bool,
    name: Option<String>,
    service_type: String,
    shares_data: Option<String>,
    privacy_statement_url: Option<String>,
    slug: Option<String>,
}

fn service(services: &[Row], id: &str) -> Service {
    let Some(row) = services.iter().find(|r| col(r, "ID") == Some(id)) else {
        return Service::default();
    };
    let mut service = Service {
        found: true,
        name: col(row, "name").map(str::to_owned),
        service_type: decode(col(row, "serviceType").unwrap_or_default()),
        shares_data: col(row, "thirdParty").map(str::to_owned),
        privacy_statement_url: col(row, "privacyStatementURL").map(str::to_owned),
        slug: col(row, "slug").map(str::to_owned),
    };
    let parent_id = col(row, "isTranslationFrom");
    if !blank(parent_id)
        && (blank(Some(&service.service_type)) || blank(service.privacy_statement_url.as_deref()))
        && let Some(parent) = services.iter().find(|r| col(r, "ID") == parent_id)
    {
        if blank(Some(&service.service_type)) {
            service.service_type = decode(col(parent, "serviceType").unwrap_or_default());
        }
        if blank(service.privacy_statement_url.as_deref()) {
            service.privacy_statement_url = col(parent, "privacyStatementURL").map(str::to_owned);
        }
        if blank(service.shares_data.as_deref()) {
            service.shares_data = col(parent, "thirdParty").map(str::to_owned);
        }
    }
    service
}

/// A cookie as `new CMPLZ_COOKIE( $row )` reads it.
struct Cookie<'r> {
    name: String,
    service_id: Option<&'r str>,
    /// The name of its service (`$cookie->service`).
    service: Option<String>,
    retention: String,
    cookie_function: String,
    purpose: String,
    slug: String,
}

fn cookie<'r>(row: &'r Row, rows: &[Row], services: &[Row], options: &Options) -> Cookie<'r> {
    let name: String = col(row, "name").unwrap_or_default().chars().take(200).collect();
    let mut retention = col(row, "retention").unwrap_or_default().to_owned();
    let mut cookie_function = col(row, "cookieFunction").unwrap_or_default().to_owned();
    let mut purpose = decode(col(row, "purpose").unwrap_or_default());
    let parent_id = col(row, "isTranslationFrom");
    if !blank(parent_id)
        && (blank(Some(&retention)) || blank(Some(&cookie_function)) || blank(Some(&purpose)))
        && let Some(parent) = rows.iter().find(|r| col(r, "ID") == parent_id)
    {
        if blank(Some(&retention)) {
            retention = col(parent, "retention").unwrap_or_default().to_owned();
        }
        if blank(Some(&cookie_function)) {
            cookie_function = col(parent, "cookieFunction").unwrap_or_default().to_owned();
        }
        if blank(Some(&purpose)) {
            purpose = decode(col(parent, "purpose").unwrap_or_default());
        }
    }
    if name.contains("cmplz") || name.contains("complianz") {
        retention = cmplz_sprintf(&options.tr("%s days"), &[&options.string("cookie_expiry")]);
    }
    let service_id = col(row, "serviceID").filter(|v| !v.is_empty() && *v != "0");
    let service = service_id.and_then(|id| service(services, id).name);
    Cookie { name, service_id, service, retention, cookie_function, purpose, slug: col(row, "slug").unwrap_or_default().to_owned() }
}

/// The cookie rows `get_cookies( $settings )` keeps: `language` is `None` for the originals only.
fn filter_cookies<'r>(rows: &'r [Row], language: Option<&str>, members_only_all: bool, show_on_policy: bool, hide_empty: bool) -> Vec<&'r Row> {
    rows.iter()
        .filter(|r| members_only_all || !flag(r, "isMembersOnly"))
        .filter(|r| !show_on_policy || flag(r, "showOnPolicy"))
        .filter(|r| !flag(r, "ignored"))
        .filter(|r| match language {
            None => !flag(r, "isTranslationFrom"),
            Some(language) => col(r, "language") == Some(language),
        })
        .filter(|r| !hide_empty || col(r, "name") != Some(""))
        .filter(|r| !flag(r, "deleted"))
        .collect()
}

/// The data of `cmplz_used_cookies()`: the plugin's tables and the cookie shredder list.
pub(super) struct CookieData<'a> {
    pub options: &'a Options,
    pub cookies: &'a [Row],
    pub services: &'a [Row],
    /// `get_locale()`.
    pub locale: &'a str,
    /// The transient `cmplz_cookie_shredder_list`, if set.
    pub shredder_list: Option<&'a Value>,
    pub plugin_url: &'a str,
}

/// `uses_non_functional_cookies()` without Google Tag Manager (checked by the caller); `optinstats` is not a
/// consent type of the free plugin's regions.
pub(super) fn uses_non_functional_cookies(data: &CookieData<'_>, statistics_privacy_friendly: bool) -> bool {
    for row in filter_cookies(data.cookies, None, true, false, false) {
        let cookie = cookie(row, data.cookies, data.services, data.options);
        let service = cookie.service.as_deref().filter(|s| !s.is_empty() && *s != "0").map(super::blocker::sanitize_service_name).unwrap_or_default();
        if (service == "google-analytics" || service == "matomo") && !statistics_privacy_friendly {
            return true;
        }
        if !cookie.purpose.to_lowercase().contains("functional") {
            return true;
        }
    }
    false
}

/// `cookie_blocker->cookie_list`: the categories and services of the cookie shredder (`load_cookie_data()`), set only
/// with consent per service.
fn in_cookie_list(data: &CookieData<'_>, category: &str, service: &str) -> bool {
    let options = data.options;
    if super::options::loose_eq_int(&options.get("safe_mode"), 1) || !options.is("consent_per_service", "yes") {
        return false;
    }
    if let Some(list) = data.shredder_list.filter(|v| truthy(v)) {
        return item(list, category).and_then(|services| item(services, service)).is_some();
    }
    let members_all = options.is("wp_admin_access_users", "yes");
    filter_cookies(data.cookies, Some("en"), members_all, true, false).into_iter().any(|row| {
        let cookie = cookie(row, data.cookies, data.services, options);
        cookie.purpose.to_lowercase().contains(category) && super::blocker::sanitize_service_name(cookie.service.as_deref().unwrap_or_default()) == service
    })
}

const SERVICES_TEMPLATE: &str = "<details class=\"cmplz-dropdown cmplz-service-desc cmplz-dropdown-cookiepolicy \">\n\t<summary class=\"cmplz-service-header\"><div>\n\t\t<h3>{service}</h3>\n\t\t<p>{allPurposes}</p>\n\t\t<label for=\"cmplz_service_{service_slug}\" class=\"cmplz_consent_per_service_label\"><span class=\"screen-reader-text\">Consent to service {service_slug}</span></label>\n\t\t<input type=\"checkbox\" id=\"cmplz_service_{service_slug}\" class=\"cmplz-accept-service {serviceCheckboxClass}\" data-service=\"{service_slug}\" data-category=\"{topCategory}\"></div>\n\t</summary>\n\t<div class=\"cmplz-service-description\">\n\t\t<h4>{usage}</h4>\n\t\t<p>{purposeDescription}</p>\n\t</div>\n\t<div class=\"cmplz-sharing-data\">\n\t\t<h4>{sharing_data}</h4>\n\t\t<p>{sharing}</p>\n\t</div>\n\t{cookies}\n</details>\n";
const COOKIES_ROW_TEMPLATE: &str = "<div class=\"name-header\">\n\t<h5>{name_header}</h5>\n</div>\n<div class=\"name\">{link_open}{name}{link_close}</div>\n<div class=\"retention-header\">\n\t<h5>{expiration_header}</h5>\n</div>\n<div class=\"retention\">{retention}</div>\n<div class=\"function-header\">\n\t<h5>{function_header}</h5>\n</div>\n<div class=\"function\">{cookieFunction}</div>\n";
const PURPOSE_ROW_TEMPLATE: &str = "<div class=\"cookies-per-purpose\">\n    <div class=\"purpose\"><h4>{purpose}</h4></div>\n    {cookies_per_purpose}\n</div>";

/// PHP's `str_replace()` with arrays: each search in turn on the result of the one before.
fn replace_all(mut text: String, pairs: &[(&str, &str)]) -> String {
    for (search, replace) in pairs {
        text = text.replace(search, replace);
    }
    text
}

/// PHP's `ucfirst()` (ASCII).
fn ucfirst(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        }
        _ => text.to_owned(),
    }
}

/// `cmplz_used_cookies()`: the services with their cookies by purpose (`templates/cookiepolicy/*.php`).
pub(super) fn used_cookies(data: &CookieData<'_>) -> String {
    let options = data.options;
    let language = data.locale.get(..2).unwrap_or(data.locale);
    let members_all = options.is("wp_admin_access_users", "yes");
    let rows = filter_cookies(data.cookies, Some(language), members_all, true, true);
    // get_cookies_by_service(): by service (cookies without one last), then by purpose.
    let purpose_map: Vec<(String, String)> = if language.is_empty() || language == "en" {
        Vec::new()
    } else {
        let mut map: Vec<(String, String)> = Vec::new();
        for t in data.cookies.iter().filter(|r| col(r, "language") == Some(language)) {
            let Some(en) = data.cookies.iter().find(|r| col(r, "ID") == col(t, "isTranslationFrom") && col(r, "language") == Some("en")) else {
                continue;
            };
            let (en_purpose, t_purpose) = (col(en, "purpose").unwrap_or_default(), col(t, "purpose").unwrap_or_default());
            if en_purpose.is_empty() || t_purpose.is_empty() || en_purpose == t_purpose {
                continue;
            }
            let (from, to) = (decode(en_purpose), decode(t_purpose));
            if !from.is_empty() && !to.is_empty() {
                map.retain(|(k, _)| *k != from);
                map.push((from, to));
            }
        }
        map
    };
    let pending = options.tr("Purpose pending investigation");
    let mut grouped: Vec<(String, Vec<(String, Vec<Cookie<'_>>)>)> = Vec::new();
    let mut top: i64 = 0;
    for row in rows {
        let cookie = cookie(row, data.cookies, data.services, options);
        let service_id = cookie.service_id.map_or("999999999".to_owned(), str::to_owned);
        top = top.max(service_id.parse().unwrap_or(0));
        let purpose = if cookie.purpose.is_empty() {
            pending.clone()
        } else {
            let decoded = decode(&cookie.purpose);
            purpose_map.iter().find(|(k, _)| *k == decoded).map_or(decoded, |(_, v)| v.clone())
        };
        let index = match grouped.iter().position(|(id, _)| *id == service_id) {
            Some(i) => i,
            None => {
                grouped.push((service_id, Vec::new()));
                grouped.len() - 1
            }
        };
        let purposes = &mut grouped[index].1;
        match purposes.iter_mut().find(|(p, _)| *p == purpose) {
            Some((_, list)) => list.push(cookie),
            None => purposes.push((purpose, vec![cookie])),
        }
    }
    if let Some(i) = grouped.iter().position(|(id, _)| id == "999999999") {
        let (_, misc) = grouped.remove(i);
        grouped.push(((top + 1).to_string(), misc));
    }

    let use_cdb_links = options.is("use_cdb_links", "yes");
    let consent_per_service = options.is("consent_per_service", "yes");
    let mut services_html = String::new();
    for (service_id, purposes) in &grouped {
        let service = service(data.services, service_id);
        let service_key = super::blocker::sanitize_service_name(service.name.as_deref().unwrap_or_default());
        let top_category = ["marketing", "statistics", "preferences"].into_iter().find(|c| in_cookie_list(data, c, &service_key)).unwrap_or("functional");
        let mut checkbox_class = if consent_per_service { "" } else { "cmplz-hidden" };
        let mut all_purposes: Vec<&str> = Vec::new();
        let mut cookie_html = String::new();
        for (purpose, cookies) in purposes {
            let mut rows_html = String::new();
            for cookie in cookies {
                let (mut link_open, mut link_close) = (String::new(), "");
                if use_cdb_links && !cookie.slug.is_empty() {
                    let service_slug = service.slug.as_deref().filter(|s| !s.is_empty() && *s != "0").unwrap_or("unknown-service");
                    link_open = format!(
                        "<a target=\"_blank\" rel=\"noopener noreferrer nofollow\" href=\"https://cookiedatabase.org/cookie/{service_slug}/{}\">",
                        rustpress_core::urls::trailingslashit(&cookie.slug)
                    );
                    link_close = "</a>";
                }
                let function = ucfirst(&cookie.cookie_function);
                let row = COOKIES_ROW_TEMPLATE
                    .replace("{name_header}", &esc_html(&options.tr("Name")))
                    .replace("{expiration_header}", &esc_html(&options.tr("Expiration")))
                    .replace("{function_header}", &esc_html(&options.tr("Function")));
                rows_html.push_str(&replace_all(
                    row,
                    &[("{name}", &cookie.name), ("{retention}", &cookie.retention), ("{cookieFunction}", &function), ("{link_open}", &link_open), ("{link_close}", link_close)],
                ));
            }
            cookie_html.push_str(&replace_all(PURPOSE_ROW_TEMPLATE.to_owned(), &[("{purpose}", purpose), ("{cookies_per_purpose}", &rows_html)]));
            all_purposes.push(purpose);
        }
        let mut service_name = service.name.clone().unwrap_or_default();
        if service_id.parse::<i64>().unwrap_or(0) == 0 || service_name.is_empty() || service_name == "0" {
            service_name = options.tr("Miscellaneous");
            checkbox_class = "cmplz-hidden";
        }
        let has_name = service.name.as_deref().is_some_and(|n| !n.is_empty() && n != "0");
        let read_policy = |link: &str| cmplz_sprintf(&options.tr("For more information, please read the %s%s Privacy Statement%s."), &[link, &service_name, "</a>"]);
        let sharing = if service_name == "Complianz" {
            format!(
                "{}&nbsp;{}",
                options.tr("This data is not shared with third parties."),
                read_policy("<a target=\"_blank\" rel=\"noopener noreferrer\" href=\"https://complianz.io/legal/privacy-statement/\">")
            )
        } else if service.shares_data.as_deref().is_some_and(|v| !v.is_empty() && v != "0") {
            match service.privacy_statement_url.as_deref() {
                Some("") => String::new(),
                url => read_policy(&format!("<a target=\"_blank\" rel=\"noopener noreferrer nofollow\" href=\"{}\">", url.unwrap_or_default())),
            }
        } else if has_name {
            options.tr("This data is not shared with third parties.")
        } else {
            options.tr("Sharing of data is pending investigation")
        };
        let mut purpose_description = if !service_name.is_empty() && !blank(Some(&service.service_type)) {
            cmplz_sprintf(&options.tr_x("We use %s for %s.", "Legal document cookie policy"), &[&service_name, &service.service_type])
        } else {
            String::new()
        };
        if use_cdb_links && let Some(slug) = service.slug.as_deref().filter(|s| !s.is_empty() && *s != "0" && *s != "unknown-service") {
            let link_text = cmplz_sprintf(&options.tr("Read more about %s"), &[&service_name]);
            let aria = cmplz_sprintf(&options.tr("Read more about %s service on Cookie Database"), &[&service_name]);
            purpose_description.push_str(&format!(
                " <a target=\"_blank\" rel=\"noopener noreferrer nofollow\" href=\"https://cookiedatabase.org/service/{slug}/\" aria-label=\"{}\">{link_text}</a>",
                rustpress_core::formatting::esc_attr(&aria)
            ));
        }
        if all_purposes.len() > 1
            && let Some(i) = all_purposes.iter().position(|p| *p == pending)
        {
            all_purposes.remove(i);
        }
        let all_purposes = all_purposes.join(", ");
        let service_slug = service_name.to_ascii_lowercase().replace(' ', "-");
        let template = SERVICES_TEMPLATE.replace("{usage}", &options.tr_x("Usage", "cookie policy")).replace("{sharing_data}", &options.tr_x("Sharing data", "Legal document cookie policy"));
        services_html.push_str(&replace_all(
            template,
            &[
                ("{service}", &service_name),
                ("{service_slug}", &service_slug),
                ("{sharing}", &sharing),
                ("{purposeDescription}", &purpose_description),
                ("{cookies}", &cookie_html),
                ("{allPurposes}", &all_purposes),
                ("{serviceCheckboxClass}", checkbox_class),
                ("{topCategory}", top_category),
            ],
        ));
        let _ = service.found;
    }
    format!("<div id=\"cmplz-cookies-overview\">{services_html}</div>").replace("{plugin_url}", data.plugin_url)
}
