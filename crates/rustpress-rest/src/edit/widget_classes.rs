//! The classes of the core widgets: `form()` and `update()` of each `WP_Widget_*`, and the PHP arrays they work on.

use rustpress_content::sanitize::{esc_textarea, text_field};
use rustpress_core::formatting::{esc_attr, esc_html, esc_url, esc_url_raw, sanitize_html_class, strip_tags};
use rustpress_core::kses::kses_post;
use rustpress_phpser::{Key, Value as Php};
use serde_json::Value;

use crate::Rest;

/// `(string) $value`.
pub(super) fn string(value: Option<&Php>) -> String {
    match value {
        Some(Php::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Php::Int(n)) => n.to_string(),
        Some(Php::Bool(true)) => "1".to_owned(),
        Some(Php::Float(f)) => f.to_string(),
        _ => String::new(),
    }
}

/// `(bool) $value`, and `! empty()` of what may be missing.
pub(super) fn truthy(value: Option<&Php>) -> bool {
    match value {
        None | Some(Php::Null | Php::Bool(false) | Php::Int(0)) => false,
        Some(Php::Float(f)) => *f != 0.0,
        Some(Php::Str(s)) => !s.is_empty() && s != b"0",
        Some(Php::Array(items)) => !items.is_empty(),
        Some(_) => true,
    }
}

/// `(int) $value`: the number a string starts with.
fn int(value: Option<&Php>) -> i64 {
    match value {
        Some(Php::Int(n)) => *n,
        Some(Php::Bool(true)) => 1,
        Some(Php::Float(f)) => *f as i64,
        Some(Php::Str(s)) => {
            let text = String::from_utf8_lossy(s);
            let text = text.trim_start_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']);
            let end = text.char_indices().find(|(i, c)| !(c.is_ascii_digit() || (*i == 0 && matches!(c, '+' | '-')))).map_or(text.len(), |(i, _)| i);
            text[..end].parse().unwrap_or(0)
        }
        _ => 0,
    }
}

fn isset(instance: &Php, key: &str) -> bool {
    !matches!(instance.get(key), None | Some(Php::Null))
}

fn str_value(text: &str) -> Php {
    Php::Str(text.as_bytes().to_vec())
}

/// `$array[ $key ] = $value`: replaces in place or appends.
fn put(array: &mut Php, key: &str, value: Php) {
    if !matches!(array, Php::Array(_)) {
        *array = Php::Array(Vec::new());
    }
    let Php::Array(items) = array else { return };
    let wanted = array_key(key);
    match items.iter_mut().find(|(k, _)| *k == wanted) {
        Some(slot) => slot.1 = value,
        None => items.push((wanted, value)),
    }
}

/// A key of a PHP array: a string that is a canonical integer becomes one.
pub(super) fn array_key(text: &str) -> Key {
    let canonical = text.parse::<i64>().ok().filter(|n| n.to_string() == text);
    canonical.map_or_else(|| Key::Str(text.as_bytes().to_vec()), Key::Int)
}

/// The value of `$_POST` for a value of a JSON body.
pub(super) fn json_to_php(value: &Value) -> Php {
    match value {
        Value::Null => Php::Null,
        Value::Bool(b) => Php::Bool(*b),
        Value::Number(n) => n.as_i64().map_or_else(|| Php::Float(n.as_f64().unwrap_or(0.0)), Php::Int),
        Value::String(s) => str_value(s),
        Value::Array(items) => Php::Array(items.iter().enumerate().map(|(i, item)| (Key::Int(i as i64), json_to_php(item))).collect()),
        Value::Object(map) => Php::Array(map.iter().map(|(key, item)| (array_key(key), json_to_php(item))).collect()),
    }
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() && bytes[i + 1].is_ascii_hexdigit() && bytes[i + 2].is_ascii_hexdigit() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("0");
                out.push(u8::from_str_radix(hex, 16).unwrap_or(0));
                i += 2;
            }
            other => out.push(other),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `wp_parse_str()`: `a[b][]=1` becomes nested arrays. A top-level name keeps what PHP keeps: dots and spaces become `_`.
pub(super) fn parse_str(query: &str) -> Php {
    let mut root = Php::Array(Vec::new());
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let name = percent_decode(name);
        let value = percent_decode(value);
        let (first, rest) = name.split_once('[').map_or((name.as_str(), ""), |(a, b)| (a, b));
        if first.is_empty() {
            continue;
        }
        let mut path = vec![first.replace(['.', ' '], "_")];
        let mut rest = rest;
        // The part after the first `[`, up to its `]`; what does not close is part of the last name.
        if !name.contains('[') {
            rest = "";
        } else if let Some((inner, after)) = rest.split_once(']') {
            path.push(inner.to_owned());
            rest = after;
            while let Some(after_open) = rest.strip_prefix('[') {
                let Some((inner, after)) = after_open.split_once(']') else { break };
                path.push(inner.to_owned());
                rest = after;
            }
        }
        let _ = rest;
        let mut node = &mut root;
        for (depth, part) in path.iter().enumerate() {
            if !matches!(node, Php::Array(_)) {
                *node = Php::Array(Vec::new());
            }
            let Php::Array(items) = node else { break };
            let key = if part.is_empty() {
                Key::Int(items.iter().filter_map(|(k, _)| if let Key::Int(n) = k { Some(*n + 1) } else { None }).max().unwrap_or(0).max(0))
            } else {
                array_key(part)
            };
            let at = match items.iter().position(|(k, _)| *k == key) {
                Some(at) => at,
                None => {
                    items.push((key, Php::Null));
                    items.len() - 1
                }
            };
            if depth + 1 == path.len() {
                items[at].1 = str_value(&value);
                break;
            }
            node = &mut items[at].1;
        }
    }
    root
}

/// `stripslashes()`.
fn stripslashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('0') => out.push('\0'),
                Some(next) => out.push(next),
                None => {}
            },
            other => out.push(other),
        }
    }
    out
}

/// `checked( $value )`: `(string) $value === '1'`.
fn checked(value: Option<&Php>) -> &'static str {
    if string(value) == "1" { " checked='checked'" } else { "" }
}

/// `selected( $a, $b )`.
fn selected(a: &str, b: &str) -> &'static str {
    if a == b { " selected='selected'" } else { "" }
}

/// `get_field_id()` and `get_field_name()` of a widget.
struct Fields<'a> {
    base: &'a str,
    number: &'a str,
}

impl Fields<'_> {
    fn id(&self, field: &str) -> String {
        format!("widget-{}-{}-{field}", self.base, self.number)
    }

    fn name(&self, field: &str) -> String {
        format!("widget-{}[{}][{field}]", self.base, self.number)
    }
}

/// The kinds of `get_instance_schema()` of the media widgets.
#[derive(Clone, Copy)]
enum Kind {
    Text(Clean),
    Enum(&'static [&'static str], &'static str),
    Sizes(&'static str),
    Url,
    Int { default: i64, minimum: i64, maximum: Option<i64> },
    Bool,
    Ids,
}

/// The `sanitize_callback` of a text field.
#[derive(Clone, Copy)]
enum Clean {
    Line,
    Post,
    Tokens,
}

const MEDIA_COMMON: [(&str, Kind); 3] = [("attachment_id", Kind::Int { default: 0, minimum: 0, maximum: None }), ("url", Kind::Url), ("title", Kind::Text(Clean::Line))];

const IMAGE: [(&str, Kind); 12] = [
    ("size", Kind::Sizes("medium")),
    ("width", Kind::Int { default: 0, minimum: 0, maximum: None }),
    ("height", Kind::Int { default: 0, minimum: 0, maximum: None }),
    ("caption", Kind::Text(Clean::Post)),
    ("alt", Kind::Text(Clean::Line)),
    ("link_type", Kind::Enum(&["none", "file", "post", "custom"], "custom")),
    ("link_url", Kind::Url),
    ("image_classes", Kind::Text(Clean::Tokens)),
    ("link_classes", Kind::Text(Clean::Tokens)),
    ("link_rel", Kind::Text(Clean::Tokens)),
    ("link_target_blank", Kind::Bool),
    ("image_title", Kind::Text(Clean::Line)),
];

const AUDIO: [(&str, Kind); 7] = [("preload", Kind::Enum(&["none", "auto", "metadata"], "none")), ("loop", Kind::Bool), ("mp3", Kind::Url), ("ogg", Kind::Url), ("flac", Kind::Url), ("m4a", Kind::Url), ("wav", Kind::Url)];

const VIDEO: [(&str, Kind); 8] = [
    ("preload", Kind::Enum(&["none", "auto", "metadata"], "metadata")),
    ("loop", Kind::Bool),
    ("content", Kind::Text(Clean::Post)),
    ("mp4", Kind::Url),
    ("m4v", Kind::Url),
    ("webm", Kind::Url),
    ("ogv", Kind::Url),
    ("flv", Kind::Url),
];

const GALLERY: [(&str, Kind); 6] = [
    ("title", Kind::Text(Clean::Line)),
    ("ids", Kind::Ids),
    ("columns", Kind::Int { default: 3, minimum: 1, maximum: Some(9) }),
    ("size", Kind::Sizes("thumbnail")),
    ("link_type", Kind::Enum(&["post", "file", "none"], "post")),
    ("orderby_random", Kind::Bool),
];

/// `get_instance_schema()`.
fn media_schema(base: &str) -> Vec<(&'static str, Kind)> {
    match base {
        "media_image" => IMAGE.iter().chain(&MEDIA_COMMON).copied().collect(),
        "media_audio" => AUDIO.iter().chain(&MEDIA_COMMON).copied().collect(),
        "media_video" => VIDEO.iter().chain(&MEDIA_COMMON).copied().collect(),
        _ => GALLERY.to_vec(),
    }
}

fn media_default(kind: Kind) -> Php {
    match kind {
        Kind::Text(_) | Kind::Url => str_value(""),
        Kind::Enum(_, default) | Kind::Sizes(default) => str_value(default),
        Kind::Int { default, .. } => Php::Int(default),
        Kind::Bool => Php::Bool(false),
        Kind::Ids => Php::Array(Vec::new()),
    }
}

/// `sanitize_token_list()`.
fn token_list(text: &str) -> String {
    text.split_whitespace().map(sanitize_html_class).filter(|t| !t.is_empty()).collect::<Vec<_>>().join(" ")
}

/// `rest_validate_value_from_schema()`, `rest_sanitize_value_from_schema()` and the `sanitize_callback` of one field.
fn media_value(kind: Kind, value: &Php, sizes: &[String]) -> Option<Php> {
    let boolean = |value: &Php| match value {
        Php::Bool(b) => Some(*b),
        Php::Int(n @ (0 | 1)) => Some(*n == 1),
        Php::Str(s) => match String::from_utf8_lossy(s).to_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    };
    match kind {
        Kind::Bool => {
            // An empty string stands for false in a form.
            if matches!(value, Php::Str(s) if s.is_empty()) {
                return Some(Php::Bool(false));
            }
            boolean(value).map(Php::Bool)
        }
        Kind::Text(clean) => {
            let Php::Str(_) = value else { return None };
            let text = string(Some(value));
            Some(str_value(&match clean {
                Clean::Line => text_field(&text),
                Clean::Post => kses_post(&text),
                Clean::Tokens => token_list(&text),
            }))
        }
        Kind::Url => {
            let Php::Str(_) = value else { return None };
            Some(str_value(&esc_url_raw(&string(Some(value)))))
        }
        Kind::Enum(values, _) => {
            let Php::Str(_) = value else { return None };
            let text = string(Some(value));
            values.contains(&text.as_str()).then(|| str_value(&text))
        }
        Kind::Sizes(_) => {
            let Php::Str(_) = value else { return None };
            let text = string(Some(value));
            (sizes.contains(&text) || text == "full" || text == "custom").then(|| str_value(&text))
        }
        Kind::Int { minimum, maximum, .. } => {
            let number = match value {
                Php::Int(n) => *n,
                Php::Str(s) => String::from_utf8_lossy(s).trim().parse::<i64>().ok()?,
                _ => return None,
            };
            (number >= minimum && maximum.is_none_or(|m| number <= m)).then_some(Php::Int(number))
        }
        Kind::Ids => {
            let list: Vec<String> = match value {
                Php::Array(items) => items.iter().map(|(_, v)| string(Some(v))).collect(),
                Php::Str(_) => string(Some(value)).split(|c: char| c == ',' || c.is_whitespace()).filter(|p| !p.is_empty()).map(str::to_owned).collect(),
                _ => return None,
            };
            let mut ids: Vec<i64> = Vec::new();
            for item in list {
                let id = item.trim().parse::<i64>().ok()?.abs();
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
            Some(Php::Array(ids.into_iter().enumerate().map(|(i, id)| (Key::Int(i as i64), Php::Int(id))).collect()))
        }
    }
}

/// The rule of `$safe_elements_attributes` in `WP_Widget_Text::is_legacy_instance()`; `None` for an unsafe element.
fn safe_attributes(tag: &str) -> Option<&'static [&'static str]> {
    match tag {
        "strong" | "em" | "b" | "i" | "u" | "s" | "ul" | "ol" | "li" | "hr" | "abbr" | "acronym" | "code" | "dfn" => Some(&[]),
        "a" => Some(&["href"]),
        "img" => Some(&["src", "alt"]),
        _ => None,
    }
}

/// The parsing part of `is_legacy_instance()`: whether the text has an element or attribute TinyMCE could drop, or an
/// element without text. `DOMDocument::loadHTML()` is approximated by walking the tags.
fn has_unsafe_markup(text: &str) -> bool {
    let tag = regex::Regex::new(r#"(?is)<(/?)([a-z][a-z0-9:-]*)((?:[^>"']|"[^"]*"|'[^']*')*)>"#).expect("valid pattern");
    let attribute = regex::Regex::new(r#"(?is)([a-z_:][a-z0-9_:.-]*)(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+))?"#).expect("valid pattern");
    // Open elements with the offset behind their start tag.
    let mut open: Vec<(String, usize)> = Vec::new();
    for found in tag.captures_iter(text) {
        let whole = found.get(0).expect("whole match");
        let name = found[2].to_ascii_lowercase();
        if &found[1] == "/" {
            if let Some(at) = open.iter().rposition(|(n, _)| *n == name) {
                let (_, start) = open[at].clone();
                open.truncate(at);
                let inner = &text[start..whole.start()];
                let plain = strip_tags(inner);
                if plain.trim().is_empty() {
                    return true;
                }
            }
            continue;
        }
        let Some(allowed) = safe_attributes(&name) else { return true };
        for attr in attribute.captures_iter(found.get(3).map_or("", |m| m.as_str())) {
            if !allowed.contains(&attr[1].to_ascii_lowercase().as_str()) {
                return true;
            }
        }
        if !matches!(name.as_str(), "img" | "hr") {
            open.push((name, whole.end()));
        }
    }
    false
}

/// `WP_Widget_Text::is_legacy_instance()`.
fn is_legacy_instance(instance: &Php) -> bool {
    if isset(instance, "visual") {
        return !truthy(instance.get("visual"));
    }
    if string(instance.get("filter")) == "content" && matches!(instance.get("filter"), Some(Php::Str(_))) {
        return false;
    }
    if !truthy(instance.get("text")) {
        return false;
    }
    let text = string(instance.get("text"));
    let wpautop = truthy(instance.get("filter"));
    let has_line_breaks = text.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}']).contains('\n');
    if !wpautop && has_line_breaks {
        return true;
    }
    if text.contains("<!--") {
        return true;
    }
    has_unsafe_markup(&text)
}

/// The `title` field most widgets start their form with.
fn title_field(fields: &Fields<'_>, title_label: &str, value: &str) -> String {
    format!(
        "\t\t<p>\n\t\t\t<label for=\"{id}\">{title_label}</label>\n\t\t\t<input class=\"widefat\" id=\"{id}\" name=\"{name}\" type=\"text\" value=\"{value}\" />\n\t\t</p>\n",
        id = esc_attr(&fields.id("title")),
        name = esc_attr(&fields.name("title")),
        value = esc_attr(value)
    )
}

impl Rest<'_> {
    /// `current_user_can( 'unfiltered_html' )`, or `wp_kses_post()`.
    fn filtered(&self, value: Option<&Php>) -> Php {
        let text = string(value);
        str_value(&if self.can("unfiltered_html") { text } else { kses_post(&text) })
    }

    /// `WP_Widget::update()` of a core widget.
    pub(super) fn widget_update(&self, base: &str, new: &Php, old: &Php) -> Php {
        let title = |new: &Php| str_value(&text_field(&string(new.get("title"))));
        let mut instance = if matches!(old, Php::Array(_)) { old.clone() } else { Php::Array(Vec::new()) };
        match base {
            "pages" => {
                put(&mut instance, "title", title(new));
                let sortby = string(new.get("sortby"));
                let sortby = if matches!(new.get("sortby"), Some(Php::Str(_))) && matches!(sortby.as_str(), "post_title" | "menu_order" | "ID") { sortby } else { "menu_order".to_owned() };
                put(&mut instance, "sortby", str_value(&sortby));
                put(&mut instance, "exclude", str_value(&text_field(&string(new.get("exclude")))));
            }
            "calendar" | "meta" | "search" => put(&mut instance, "title", title(new)),
            "archives" => {
                put(&mut instance, "title", title(new));
                put(&mut instance, "count", Php::Int(i64::from(truthy(new.get("count")))));
                put(&mut instance, "dropdown", Php::Int(i64::from(truthy(new.get("dropdown")))));
            }
            "recent-posts" => {
                put(&mut instance, "title", title(new));
                put(&mut instance, "number", Php::Int(int(new.get("number"))));
                put(&mut instance, "show_date", Php::Bool(isset(new, "show_date") && truthy(new.get("show_date"))));
            }
            "recent-comments" => {
                put(&mut instance, "title", title(new));
                put(&mut instance, "number", Php::Int(int(new.get("number")).abs()));
            }
            "categories" => {
                put(&mut instance, "title", title(new));
                for key in ["count", "hierarchical", "dropdown"] {
                    put(&mut instance, key, Php::Int(i64::from(truthy(new.get(key)))));
                }
            }
            "tag_cloud" => {
                instance = Php::Array(Vec::new());
                put(&mut instance, "title", title(new));
                put(&mut instance, "count", Php::Int(i64::from(truthy(new.get("count")))));
                put(&mut instance, "taxonomy", str_value(&stripslashes(&string(new.get("taxonomy")))));
            }
            "nav_menu" => {
                instance = Php::Array(Vec::new());
                if truthy(new.get("title")) {
                    put(&mut instance, "title", title(new));
                }
                if truthy(new.get("nav_menu")) {
                    put(&mut instance, "nav_menu", Php::Int(int(new.get("nav_menu"))));
                }
            }
            "block" => {
                let mut merged = Php::Array(vec![(Key::Str(b"content".to_vec()), str_value(""))]);
                if let Php::Array(items) = &instance {
                    for (key, value) in items {
                        if let Key::Str(name) = key {
                            put(&mut merged, &String::from_utf8_lossy(name), value.clone());
                        }
                    }
                }
                put(&mut merged, "content", self.filtered(new.get("content")));
                instance = merged;
            }
            "custom_html" => {
                let mut merged = Php::Array(vec![(Key::Str(b"title".to_vec()), str_value("")), (Key::Str(b"content".to_vec()), str_value(""))]);
                if let Php::Array(items) = &instance {
                    for (key, value) in items {
                        if let Key::Str(name) = key {
                            put(&mut merged, &String::from_utf8_lossy(name), value.clone());
                        }
                    }
                }
                put(&mut merged, "title", title(new));
                put(&mut merged, "content", self.filtered(new.get("content")));
                instance = merged;
            }
            "text" => {
                put(&mut instance, "title", title(new));
                put(&mut instance, "text", self.filtered(new.get("text")));
                put(&mut instance, "filter", Php::Bool(truthy(new.get("filter"))));
                let is_content = |value: Option<&Php>| matches!(value, Some(Php::Str(s)) if s == b"content");
                if is_content(old.get("filter")) || is_content(new.get("filter")) {
                    put(&mut instance, "visual", Php::Bool(true));
                }
                if isset(new, "visual") {
                    put(&mut instance, "visual", Php::Bool(truthy(new.get("visual"))));
                }
                if truthy(instance.get("visual")) {
                    put(&mut instance, "filter", Php::Bool(true));
                }
            }
            "rss" => return self.rss_process(new),
            _ => {
                let mut sizes: Vec<String> = rustpress_content::media::registered_sizes(&self.site.options).into_iter().map(|(name, ..)| name).collect();
                sizes.extend(rustpress_theme::classic::image_size_names(&self.theme.stylesheet).iter().map(|name| (*name).to_owned()));
                for (field, kind) in media_schema(base) {
                    let Some(value) = new.get(field) else { continue };
                    if let Some(value) = media_value(kind, value, &sizes) {
                        put(&mut instance, field, value);
                    }
                }
            }
        }
        instance
    }

    /// `wp_widget_rss_process()`. The feed is not fetched (`PLAN.md`): `link` stays empty and `error` false.
    fn rss_process(&self, new: &Php) -> Php {
        let items = match int(new.get("items")) {
            n @ 1..=20 => n,
            _ => 10,
        };
        let mut out = Php::Array(Vec::new());
        put(&mut out, "title", str_value(strip_tags(&string(new.get("title"))).trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}'])));
        put(&mut out, "url", str_value(&esc_url_raw(&strip_tags(&string(new.get("url"))))));
        put(&mut out, "link", str_value(""));
        put(&mut out, "items", Php::Int(items));
        put(&mut out, "error", Php::Bool(false));
        for key in ["show_summary", "show_author", "show_date"] {
            put(&mut out, key, Php::Int(int(new.get(key))));
        }
        out
    }

    /// `WP_Widget::form()` of a core widget, with the whitespace PHP leaves around its tags.
    pub(super) async fn widget_form(&self, base: &str, number: &str, instance: &Php, rich_text: bool) -> String {
        let tr = |text: &str| self.site.tr(text);
        let fields = Fields { base, number };
        let id = |field: &str| fields.id(field);
        let name = |field: &str| fields.name(field);
        let title = string(instance.get("title"));
        match base {
            "pages" => {
                let sortby = if isset(instance, "sortby") { string(instance.get("sortby")) } else { "post_title".to_owned() };
                format!(
                    "{}\n\t\t<p>\n\t\t\t<label for=\"{sid}\">{}</label>\n\t\t\t<select name=\"{sname}\" id=\"{sid}\" class=\"widefat\">\n\t\t\t\t<option value=\"post_title\"{}>{}</option>\n\t\t\t\t<option value=\"menu_order\"{}>{}</option>\n\t\t\t\t<option value=\"ID\"{}>{}</option>\n\t\t\t</select>\n\t\t</p>\n\n\t\t<p>\n\t\t\t<label for=\"{eid}\">{}</label>\n\t\t\t<input type=\"text\" value=\"{}\" name=\"{ename}\" id=\"{eid}\" class=\"widefat\" />\n\t\t\t<br />\n\t\t\t<small>{}</small>\n\t\t</p>\n",
                    title_field(&fields, &tr("Title:"), &title),
                    tr("Sort by:"),
                    selected(&sortby, "post_title"),
                    tr("Page title"),
                    selected(&sortby, "menu_order"),
                    tr("Page order"),
                    selected(&sortby, "ID"),
                    tr("Page ID"),
                    tr("Exclude:"),
                    esc_attr(&string(instance.get("exclude"))),
                    tr("Page IDs, separated by commas."),
                    sid = esc_attr(&id("sortby")),
                    sname = esc_attr(&name("sortby")),
                    eid = esc_attr(&id("exclude")),
                    ename = esc_attr(&name("exclude")),
                )
            }
            "calendar" | "meta" | "search" => title_field(&fields, &tr("Title:"), &title),
            "archives" => format!(
                "{}\t\t<p>\n\t\t\t<input class=\"checkbox\" type=\"checkbox\"{} id=\"{did}\" name=\"{dname}\" />\n\t\t\t<label for=\"{did}\">{}</label>\n\t\t\t<br />\n\t\t\t<input class=\"checkbox\" type=\"checkbox\"{} id=\"{cid}\" name=\"{cname}\" />\n\t\t\t<label for=\"{cid}\">{}</label>\n\t\t</p>\n",
                title_field(&fields, &tr("Title:"), &title),
                checked(instance.get("dropdown")),
                tr("Display as dropdown"),
                checked(instance.get("count")),
                tr("Show post counts"),
                did = id("dropdown"),
                dname = name("dropdown"),
                cid = id("count"),
                cname = name("count"),
            ),
            "recent-posts" => {
                let number_value = if isset(instance, "number") { int(instance.get("number")).abs() } else { 5 };
                let show_date = Php::Bool(isset(instance, "show_date") && truthy(instance.get("show_date")));
                format!(
                    "{}\n\t\t<p>\n\t\t\t<label for=\"{nid}\">{}</label>\n\t\t\t<input class=\"tiny-text\" id=\"{nid}\" name=\"{nname}\" type=\"number\" step=\"1\" min=\"1\" value=\"{number_value}\" size=\"3\" />\n\t\t</p>\n\n\t\t<p>\n\t\t\t<input class=\"checkbox\" type=\"checkbox\"{} id=\"{did}\" name=\"{dname}\" />\n\t\t\t<label for=\"{did}\">{}</label>\n\t\t</p>\n",
                    title_field(&fields, &tr("Title:"), &title),
                    tr("Number of posts to show:"),
                    checked(Some(&show_date)),
                    tr("Display post date?"),
                    nid = id("number"),
                    nname = name("number"),
                    did = id("show_date"),
                    dname = name("show_date"),
                )
            }
            "recent-comments" => {
                let number_value = if isset(instance, "number") { int(instance.get("number")).abs() } else { 5 };
                format!(
                    "{}\n\t\t<p>\n\t\t\t<label for=\"{nid}\">{}</label>\n\t\t\t<input class=\"tiny-text\" id=\"{nid}\" name=\"{nname}\" type=\"number\" step=\"1\" min=\"1\" value=\"{number_value}\" size=\"3\" />\n\t\t</p>\n",
                    title_field(&fields, &tr("Title:"), &title),
                    tr("Number of comments to show:"),
                    nid = id("number"),
                    nname = name("number"),
                )
            }
            "categories" => {
                let flag = |key: &str| Php::Bool(isset(instance, key) && truthy(instance.get(key)));
                format!(
                    "{}\n\t\t<p>\n\t\t\t<input type=\"checkbox\" class=\"checkbox\" id=\"{did}\" name=\"{dname}\"{} />\n\t\t\t<label for=\"{did}\">{}</label>\n\t\t\t<br />\n\n\t\t\t<input type=\"checkbox\" class=\"checkbox\" id=\"{cid}\" name=\"{cname}\"{} />\n\t\t\t<label for=\"{cid}\">{}</label>\n\t\t\t<br />\n\n\t\t\t<input type=\"checkbox\" class=\"checkbox\" id=\"{hid}\" name=\"{hname}\"{} />\n\t\t\t<label for=\"{hid}\">{}</label>\n\t\t</p>\n",
                    title_field(&fields, &tr("Title:"), &title),
                    checked(Some(&flag("dropdown"))),
                    tr("Display as dropdown"),
                    checked(Some(&flag("count"))),
                    tr("Show post counts"),
                    checked(Some(&flag("hierarchical"))),
                    tr("Show hierarchy"),
                    did = id("dropdown"),
                    dname = name("dropdown"),
                    cid = id("count"),
                    cname = name("count"),
                    hid = id("hierarchical"),
                    hname = name("hierarchical"),
                )
            }
            "tag_cloud" => self.tag_cloud_form(&fields, instance),
            "nav_menu" => self.nav_menu_form(&fields, instance).await,
            "block" => format!(
                "\t\t<p>\n\t\t\t<label for=\"{cid}\">\n\t\t\t\t{}\t\t\t</label>\n\t\t\t<textarea id=\"{cid}\" name=\"{cname}\" rows=\"6\" cols=\"50\" class=\"widefat code\">{}</textarea>\n\t\t</p>\n",
                tr("Block HTML:"),
                esc_textarea(&string(instance.get("content"))),
                cid = id("content"),
                cname = name("content"),
            ),
            "custom_html" => format!(
                "\t\t<input id=\"{tid}\" name=\"{tname}\" class=\"title sync-input\" type=\"hidden\" value=\"{}\" />\n\t\t<textarea id=\"{cid}\" name=\"{cname}\" class=\"content sync-input\" hidden>{}</textarea>\n",
                esc_attr(&title),
                esc_textarea(&string(instance.get("content"))),
                tid = id("title"),
                tname = name("title"),
                cid = id("content"),
                cname = name("content"),
            ),
            "text" => self.text_form(&fields, instance, rich_text),
            "rss" => self.rss_form(number, instance),
            _ => {
                let schema = media_schema(base);
                let mut out = String::new();
                for (field, kind) in schema {
                    let value = instance.get(field).cloned().unwrap_or_else(|| media_default(kind));
                    let value = match &value {
                        Php::Array(items) => items.iter().map(|(_, v)| string(Some(v))).collect::<Vec<_>>().join(","),
                        other => string(Some(other)),
                    };
                    out.push_str(&format!(
                        "\t\t\t<input\n\t\t\t\ttype=\"hidden\"\n\t\t\t\tdata-property=\"{}\"\n\t\t\t\tclass=\"media-widget-instance-property\"\n\t\t\t\tname=\"{}\"\n\t\t\t\tid=\"{}\"\n\t\t\t\tvalue=\"{}\"\n\t\t\t/>\n\t\t\t",
                        esc_attr(field),
                        esc_attr(&name(field)),
                        esc_attr(&id(field)),
                        esc_attr(&value)
                    ));
                }
                out
            }
        }
    }

    /// `WP_Widget_Text::form()`.
    fn text_form(&self, fields: &Fields<'_>, instance: &Php, rich_text: bool) -> String {
        let tr = |text: &str| self.site.tr(text);
        let title = string(instance.get("title"));
        let text = string(instance.get("text"));
        let (tid, tname) = (fields.id("title"), fields.name("title"));
        let (cid, cname) = (fields.id("text"), fields.name("text"));
        let (fid, fname) = (fields.id("filter"), fields.name("filter"));
        let (vid, vname) = (fields.id("visual"), fields.name("visual"));
        if !is_legacy_instance(instance) {
            // `format_for_editor()` runs where TinyMCE can edit the text.
            let shown = if rich_text && !text.is_empty() { text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;") } else { text };
            let closing = regex::Regex::new("(?i)</textarea").expect("valid pattern");
            let escaped = closing.replace_all(&shown, "&lt;/textarea");
            return format!(
                "\t\t\t\t\t\t<input id=\"{tid}\" name=\"{tname}\" class=\"title sync-input\" type=\"hidden\" value=\"{}\">\n\t\t\t<textarea id=\"{cid}\" name=\"{cname}\" class=\"text sync-input\" hidden>{escaped}</textarea>\n\t\t\t<input id=\"{fid}\" name=\"{fname}\" class=\"filter sync-input\" type=\"hidden\" value=\"on\">\n\t\t\t<input id=\"{vid}\" name=\"{vname}\" class=\"visual sync-input\" type=\"hidden\" value=\"on\">\n\t\t",
                esc_attr(&title)
            );
        }
        let info = if isset(instance, "visual") {
            tr("This widget may have contained code that may work better in the &#8220;Custom HTML&#8221; widget. If you have not yet, how about trying that widget instead?")
        } else {
            tr("This widget may contain code that may work better in the &#8220;Custom HTML&#8221; widget. How about trying that widget instead?")
        };
        format!(
            "\t\t\t\t\t\t<input id=\"{vid}\" name=\"{vname}\" class=\"visual\" type=\"hidden\" value=\"\">\n\t\t\t<p>\n\t\t\t\t<label for=\"{tid}\">{}</label>\n\t\t\t\t<input class=\"widefat\" id=\"{tid}\" name=\"{tname}\" type=\"text\" value=\"{}\" />\n\t\t\t</p>\n\t\t\t<div class=\"notice notice-info notice-alt inline\"><p>{info}</p></div>\t\t\t<p>\n\t\t\t\t<label for=\"{cid}\">{}</label>\n\t\t\t\t<textarea class=\"widefat\" rows=\"16\" cols=\"20\" id=\"{cid}\" name=\"{cname}\">{}</textarea>\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<input id=\"{fid}\" name=\"{fname}\" type=\"checkbox\"{} />&nbsp;<label for=\"{fid}\">{}</label>\n\t\t\t</p>\n\t\t\t",
            tr("Title:"),
            esc_attr(&title),
            tr("Content:"),
            esc_textarea(&text),
            checked(Some(&Php::Bool(truthy(instance.get("filter"))))),
            tr("Automatically add paragraphs"),
        )
    }

    /// `WP_Widget_Tag_Cloud::form()`.
    fn tag_cloud_form(&self, fields: &Fields<'_>, instance: &Php) -> String {
        let tr = |text: &str| self.site.tr(text);
        let (id, name) = (fields.id("taxonomy"), fields.name("taxonomy"));
        let (title_id, title_name) = (fields.id("title"), fields.name("title"));
        let (count_id, count_name) = (fields.id("count"), fields.name("count"));
        // `get_taxonomies( array( 'show_tagcloud' => true ), 'object' )`.
        let taxonomies: Vec<(String, String)> = self.tagcloud_taxonomies();
        let wanted = string(instance.get("taxonomy"));
        let current = if taxonomies.iter().any(|(name, _)| *name == wanted) || self.site.types.taxonomy(&wanted).is_some() { wanted } else { "post_tag".to_owned() };
        let count = Php::Bool(isset(instance, "count") && truthy(instance.get("count")));
        let mut out = format!(
            "\t\t<p>\n\t\t\t<label for=\"{title_id}\">{}</label>\n\t\t\t<input type=\"text\" class=\"widefat\" id=\"{title_id}\" name=\"{title_name}\" value=\"{}\" />\n\t\t</p>\n\t\t",
            tr("Title:"),
            esc_attr(&if truthy(instance.get("title")) { string(instance.get("title")) } else { String::new() })
        );
        match taxonomies.len() {
            0 => out.push_str(&format!("\t\t\t\t<input type=\"hidden\" id=\"{id}\" name=\"{name}\" value=\"\" />\n\t\t\t\t<p>\n\t\t\t\t\t{}\n\t\t\t\t</p>\n\t\t\t\t", tr("The tag cloud will not be displayed since there are no taxonomies that support the tag cloud widget."))),
            1 => out.push_str(&format!("\t\t\t\t<input type=\"hidden\" id=\"{id}\" name=\"{name}\" value=\"{}\" />\n\t\t\t\t", esc_attr(&taxonomies[0].0))),
            _ => {
                out.push_str(&format!("\t\t\t\t<p>\n\t\t\t\t\t<label for=\"{id}\">{}</label>\n\t\t\t\t\t<select class=\"widefat\" id=\"{id}\" name=\"{name}\">\n\t\t\t\t\t", tr("Taxonomy:")));
                for (taxonomy, label) in &taxonomies {
                    out.push_str(&format!("\t\t\t\t\t\t<option value=\"{}\" {}>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</option>\n\t\t\t\t\t", esc_attr(taxonomy), selected(taxonomy, &current), esc_html(label)));
                }
                out.push_str("\t\t\t\t\t</select>\n\t\t\t\t</p>\n\t\t\t\t");
            }
        }
        if !taxonomies.is_empty() {
            out.push_str(&format!(
                "\t\t\t<p>\n\t\t\t\t<input type=\"checkbox\" class=\"checkbox\" id=\"{count_id}\" name=\"{count_name}\" {} />\n\t\t\t\t<label for=\"{count_id}\">{}</label>\n\t\t\t</p>\n\t\t\t",
                checked(Some(&count)),
                tr("Show tag counts")
            ));
        }
        out
    }

    /// `WP_Nav_Menu_Widget::form()`.
    async fn nav_menu_form(&self, fields: &Fields<'_>, instance: &Php) -> String {
        let tr = |text: &str| self.site.tr(text);
        let (title_id, title_name) = (fields.id("title"), fields.name("title"));
        let (menu_id, menu_name) = (fields.id("nav_menu"), fields.name("nav_menu"));
        // `wp_get_nav_menus()`.
        let menus = self.site.db.terms("nav_menu").await.unwrap_or_default();
        let (no_menus_style, form_style) = if menus.is_empty() { ("", " style=\"display:none\" ") } else { (" style=\"display:none\" ", "") };
        let nav_menu = string(instance.get("nav_menu"));
        let url = self.site.site_url("wp-admin/nav-menus.php");
        let mut out = format!(
            "\t\t<p class=\"nav-menu-widget-no-menus-message\" {no_menus_style}>\n\t\t\t{}\t\t</p>\n\t\t<div class=\"nav-menu-widget-form-controls\" {form_style}>\n\t\t\t<p>\n\t\t\t\t<label for=\"{title_id}\">{}</label>\n\t\t\t\t<input type=\"text\" class=\"widefat\" id=\"{title_id}\" name=\"{title_name}\" value=\"{}\" />\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<label for=\"{menu_id}\">{}</label>\n\t\t\t\t<select id=\"{menu_id}\" name=\"{menu_name}\">\n\t\t\t\t\t<option value=\"0\">{}</option>\n\t\t\t\t\t",
            rustpress_core::i18n::sprintf(&tr("No menus have been created yet. <a href=\"%s\">Create some</a>."), &[&esc_attr(&url)]),
            tr("Title:"),
            esc_attr(&string(instance.get("title"))),
            tr("Select Menu:"),
            tr("&mdash; Select &mdash;"),
        );
        for menu in &menus {
            let term = menu.term_id.to_string();
            out.push_str(&format!("\t\t\t\t\t\t<option value=\"{term}\" {}>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</option>\n\t\t\t\t\t", selected(&nav_menu, &term), esc_html(&menu.name)));
        }
        out.push_str("\t\t\t\t</select>\n\t\t\t</p>\n\t\t\t\t\t</div>\n\t\t");
        out
    }

    /// `WP_Widget_RSS::form()` with `wp_widget_rss_form()`.
    fn rss_form(&self, number: &str, instance: &Php) -> String {
        let tr = |text: &str| self.site.tr(text);
        let items = match int(instance.get("items")) {
            n @ 1..=20 => n,
            _ => 10,
        };
        // Without an instance all three are off; with one, what is missing counts as on.
        let flag = |key: &str| if truthy(Some(instance)) { if isset(instance, key) { int(instance.get(key)) } else { 1 } } else { 0 };
        let mut out = String::new();
        if truthy(instance.get("error")) {
            out.push_str(&format!("<p class=\"widget-error\"><strong>{}</strong> {}</p>", tr("RSS Error:"), esc_html(&string(instance.get("error")))));
        }
        let number = esc_attr(number);
        out.push_str(&format!(
            "\t<p><label for=\"rss-url-{number}\">{}</label>\n\t<input class=\"widefat\" id=\"rss-url-{number}\" name=\"widget-rss[{number}][url]\" type=\"text\" value=\"{}\" /></p>\n\t<p><label for=\"rss-title-{number}\">{}</label>\n\t<input class=\"widefat\" id=\"rss-title-{number}\" name=\"widget-rss[{number}][title]\" type=\"text\" value=\"{}\" /></p>\n\t<p><label for=\"rss-items-{number}\">{}</label>\n\t<select id=\"rss-items-{number}\" name=\"widget-rss[{number}][items]\">\n\t",
            tr("Enter the RSS feed URL here:"),
            esc_url(&string(instance.get("url"))),
            tr("Give the feed a title (optional):"),
            esc_attr(&string(instance.get("title"))),
            tr("How many items would you like to display?"),
        ));
        for i in 1..=20 {
            out.push_str(&format!("<option value='{i}' {}>{i}</option>", selected(&items.to_string(), &i.to_string())));
        }
        out.push_str("\t</select></p>\n\t<p>\n\t\t");
        let check = |value: i64| checked(Some(&Php::Int(value)));
        out.push_str(&format!(
            "\t<input id=\"rss-show-summary-{number}\" name=\"widget-rss[{number}][show_summary]\" type=\"checkbox\" value=\"1\" {} />\n\t\t<label for=\"rss-show-summary-{number}\">{}</label><br />\n\t\t\t<input id=\"rss-show-author-{number}\" name=\"widget-rss[{number}][show_author]\" type=\"checkbox\" value=\"1\" {} />\n\t\t<label for=\"rss-show-author-{number}\">{}</label><br />\n\t\t\t<input id=\"rss-show-date-{number}\" name=\"widget-rss[{number}][show_date]\" type=\"checkbox\" value=\"1\" {} />\n\t\t<label for=\"rss-show-date-{number}\">{}</label><br />\n\t\t</p>\n\t",
            check(flag("show_summary")),
            tr("Display item content?"),
            check(flag("show_author")),
            tr("Display item author if available?"),
            check(flag("show_date")),
            tr("Display item date?"),
        ));
        out
    }

    /// `get_taxonomies( array( 'show_tagcloud' => true ), 'object' )`: the name and `labels->name` of each.
    fn tagcloud_taxonomies(&self) -> Vec<(String, String)> {
        let mut out = vec![("category".to_owned(), self.site.taxonomy_label("category")), ("post_tag".to_owned(), self.site.taxonomy_label("post_tag")), ("link_category".to_owned(), self.site.tr("Link Categories"))];
        // `show_tagcloud` follows `show_ui`, which follows `public`.
        for registered in self.site.types.registered_types(true).into_iter().filter(|t| t.public) {
            if !out.iter().any(|(name, _)| *name == registered.name) {
                let label = registered.label.clone().unwrap_or_else(|| registered.name.clone());
                out.push((registered.name, label));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_data_makes_nested_arrays() {
        let parsed = parse_str("widget-text%5B3%5D%5Btitle%5D=A+b%26c&widget-text[3][visual]=on&plain=1&list[]=a&list[]=b");
        let widget = parsed.get("widget-text").expect("widget");
        let instance = widget.as_array().expect("array")[0].clone();
        assert_eq!(instance.0, Key::Int(3));
        assert_eq!(instance.1.get("title"), Some(&str_value("A b&c")));
        assert_eq!(instance.1.get("visual"), Some(&str_value("on")));
        assert_eq!(parsed.get("plain"), Some(&str_value("1")));
        let list = parsed.get("list").and_then(Php::as_array).expect("list");
        assert_eq!(list[1], (Key::Int(1), str_value("b")));
    }

    #[test]
    fn plain_text_is_no_legacy_instance() {
        let text = |t: &str| Php::Array(vec![(Key::Str(b"text".to_vec()), str_value(t))]);
        assert!(!is_legacy_instance(&text("<strong>Hallo</strong> <a href=\"x\">Welt</a>")));
        assert!(is_legacy_instance(&text("<div>Hallo</div>")));
        assert!(is_legacy_instance(&text("<a href=\"x\" class=\"y\">Welt</a>")));
        assert!(is_legacy_instance(&text("<b></b>")));
        assert!(is_legacy_instance(&text("a\nb")));
        assert!(is_legacy_instance(&text("a <!-- b -->")));
    }
}
