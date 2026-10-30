//! `wp_kses()` for the tags WordPress lets the comments of visitors and the descriptions of users have
//! (`$allowedtags`): every tag and attribute that is not listed goes, entities are normalized, the addresses of links
//! keep only allowed protocols. The parts of WordPress this follows: `wp_kses_split2()`, `wp_kses_attr()`,
//! `wp_kses_hair()` (attributes are read as an HTML tokenizer reads them), `wp_kses_bad_protocol()` and
//! `wp_kses_normalize_entities()`.

use std::collections::HashMap;
use std::sync::OnceLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

use crate::html_entities::{ALLOWED, NAMED};

/// `wp_allowed_protocols()`.
pub const PROTOCOLS: [&str; 22] = [
    "http", "https", "ftp", "ftps", "mailto", "news", "irc", "irc6", "ircs", "gopher", "nntp", "feed", "telnet", "mms", "rtsp", "sms", "svn", "tel", "fax", "xmpp", "webcal", "urn",
];

/// `wp_kses_uri_attributes()`.
const URI_ATTRIBUTES: [&str; 17] = ["action", "archive", "background", "cite", "classid", "codebase", "data", "formaction", "href", "icon", "longdesc", "manifest", "poster", "profile", "src", "usemap", "xmlns"];

/// The elements that are allowed and the attributes each one keeps.
pub struct Allowed {
    tags: HashMap<&'static str, Vec<&'static str>>,
    /// The rules of `wp_kses_allowed_html( 'post' )`: `data-*`, `style` through `safecss_filter_attr()`, a valueless
    /// `download`, `object` only for PDF files.
    post: bool,
}

impl Allowed {
    /// `array()`: no tag at all.
    pub fn none() -> Self {
        Self { tags: HashMap::new(), post: false }
    }

    /// `wp_kses_allowed_html( 'post' )`: what post content may contain (`wp_kses_post()`).
    pub fn post() -> Self {
        Self { tags: crate::kses_post_table::ALLOWED_POST.iter().map(|(name, attrs)| (*name, attrs.to_vec())).collect(), post: true }
    }

    /// Whether `name` (lower case) is an attribute the element keeps.
    fn permits(&self, element: &str, name: &str) -> bool {
        let Some(permitted) = self.attributes(element) else { return false };
        if permitted.contains(&name) {
            return true;
        }
        static DATA: OnceLock<Regex> = OnceLock::new();
        permitted.contains(&"data-*") && re(&DATA, r"^data-[a-z0-9_-]+$").is_match(name)
    }

    /// `$allowedtags`: what a comment of a visitor may contain.
    pub fn data() -> Self {
        let tags: [(&str, &[&str]); 15] = [
            ("a", &["href", "title"]),
            ("abbr", &["title"]),
            ("acronym", &["title"]),
            ("b", &[]),
            ("blockquote", &["cite"]),
            ("cite", &[]),
            ("code", &[]),
            ("del", &["datetime"]),
            ("em", &[]),
            ("i", &[]),
            ("q", &["cite"]),
            ("s", &[]),
            ("strike", &[]),
            ("strong", &[]),
            ("span", &["class"]),
        ];
        Self { tags: tags.into_iter().map(|(name, attrs)| (name, attrs.to_vec())).collect(), post: false }
    }

    /// The comments of `pre_comment_content` also allow `span` with a `class` (note mentions); other users of
    /// `$allowedtags` do not.
    pub fn data_without_span() -> Self {
        let mut allowed = Self::data();
        allowed.tags.remove("span");
        allowed
    }

    fn attributes(&self, element: &str) -> Option<&Vec<&'static str>> {
        self.tags.get(element.to_lowercase().as_str())
    }
}

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("a valid pattern"))
}

/// `wp_kses_no_null()`: control characters go; with `remove_slash_zero` also `\0` written out.
pub fn no_null(content: &str, remove_slash_zero: bool) -> String {
    let mut out: String = content.chars().filter(|c| !matches!(*c as u32, 0..=8 | 0x0b | 0x0c | 0x0e..=0x1f)).collect();
    if remove_slash_zero {
        static CELL: OnceLock<Regex> = OnceLock::new();
        out = re(&CELL, r"\\+0+").replace_all(&out, "").into_owned();
    }
    out
}

/// `valid_unicode()`.
fn valid_unicode(i: u32) -> bool {
    i == 0x9 || i == 0xa || i == 0xd || (0x20..=0xd7ff).contains(&i) || (0xe000..=0xfffd).contains(&i) || (0x10000..=0x10ffff).contains(&i)
}

/// `wp_kses_normalize_entities()`: every `&` is disarmed, then the references that are good come back.
pub fn normalize_entities(content: &str) -> String {
    static DECIMAL: OnceLock<Regex> = OnceLock::new();
    static HEX: OnceLock<Regex> = OnceLock::new();
    static NAMED_RE: OnceLock<Regex> = OnceLock::new();
    let content = content.replace('&', "&amp;");
    let content = re(&DECIMAL, r"&amp;#(0*[1-9][0-9]{0,6});").replace_all(&content, |caps: &regex::Captures| {
        let digits = &caps[1];
        if valid_unicode(digits.parse::<u32>().unwrap_or(0)) {
            format!("&#{:0>3};", digits.trim_start_matches('0'))
        } else {
            format!("&amp;#{digits};")
        }
    });
    let content = re(&HEX, r"&amp;#[Xx](0*[1-9A-Fa-f][0-9A-Fa-f]{0,5});").replace_all(&content, |caps: &regex::Captures| {
        let digits = &caps[1];
        if valid_unicode(u32::from_str_radix(digits, 16).unwrap_or(0)) { format!("&#x{};", digits.trim_start_matches('0')) } else { format!("&amp;#x{digits};") }
    });
    re(&NAMED_RE, r"&amp;([A-Za-z]{2,8}[0-9]{0,2});")
        .replace_all(&content, |caps: &regex::Captures| if ALLOWED.contains(&&caps[1]) { format!("&{};", &caps[1]) } else { format!("&amp;{};", &caps[1]) })
        .into_owned()
}

/// `wp_kses_decode_entities()`: only the numeric references, each as the one byte `chr()` makes of it.
fn decode_entities(content: &str) -> String {
    static DECIMAL: OnceLock<Regex> = OnceLock::new();
    static HEX: OnceLock<Regex> = OnceLock::new();
    let content = re(&DECIMAL, r"&#([0-9]+);").replace_all(content, |caps: &regex::Captures| char::from((caps[1].parse::<u64>().unwrap_or(0) % 256) as u8).to_string());
    re(&HEX, r"&#[Xx]([0-9A-Fa-f]+);").replace_all(&content, |caps: &regex::Captures| char::from((u64::from_str_radix(&caps[1], 16).unwrap_or(0) % 256) as u8).to_string()).into_owned()
}

/// `wp_kses_bad_protocol_once2()`: `scheme:` when the scheme is allowed, nothing otherwise.
fn bad_protocol_once2(scheme: &str, allowed: &[&str]) -> String {
    let scheme = decode_entities(scheme);
    let scheme: String = scheme.chars().filter(|c| !c.is_whitespace()).collect();
    let scheme = no_null(&scheme, true).to_lowercase();
    if allowed.iter().any(|p| p.to_lowercase() == scheme) { format!("{scheme}:") } else { String::new() }
}

/// `wp_kses_bad_protocol_once()`.
fn bad_protocol_once(content: &str, allowed: &[&str], count: u32) -> String {
    static COLON_ENTITY: OnceLock<FancyRegex> = OnceLock::new();
    static SPLIT: OnceLock<Regex> = OnceLock::new();
    static QUERY: OnceLock<Regex> = OnceLock::new();
    let colon = COLON_ENTITY.get_or_init(|| FancyRegex::new(r"(?i)(&#0*58(?![;0-9])|&#x0*3a(?![;a-f0-9]))").expect("pattern"));
    let content = colon.replace_all(content, "${1};").into_owned();
    let split = re(&SPLIT, r"(?i):|&#0*58;|&#x0*3a;|&colon;");
    let Some(found) = split.find(&content) else { return content };
    let (head, rest) = (&content[..found.start()], &content[found.end()..]);
    if re(&QUERY, r"/\?").is_match(head) {
        return content;
    }
    let mut remainder = rest.trim().to_owned();
    let protocol = bad_protocol_once2(head, allowed);
    if protocol == "feed:" {
        if count > 2 {
            return String::new();
        }
        remainder = bad_protocol_once(&remainder, allowed, count + 1);
        if remainder.is_empty() {
            return remainder;
        }
    }
    format!("{protocol}{remainder}")
}

/// `wp_kses_bad_protocol()`: a link keeps only a protocol that is allowed.
pub fn bad_protocol(content: &str, allowed: &[&str]) -> String {
    let mut content = no_null(content, true);
    if (content.starts_with("https://") && allowed.contains(&"https")) || (content.starts_with("http://") && allowed.contains(&"http")) {
        return content;
    }
    let mut iterations = 0;
    loop {
        let original = content.clone();
        content = bad_protocol_once(&content, allowed, 1);
        iterations += 1;
        if original == content {
            return content;
        }
        if iterations >= 6 {
            return String::new();
        }
    }
}

// ---------------------------------------------------------------------------------------------- attributes

/// The text of a numeric character reference of an attribute value (HTML 5: the replacement table of the C1 controls).
fn numeric_reference(code: u32) -> char {
    const C1: [u32; 32] = [
        0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039, 0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
    ];
    let code = match code {
        0 => 0xfffd,
        0x80..=0x9f => C1[(code - 0x80) as usize],
        c if c > 0x10ffff || (0xd800..=0xdfff).contains(&c) => 0xfffd,
        c => c,
    };
    char::from_u32(code).unwrap_or('\u{fffd}')
}

fn named_references() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| NAMED.iter().copied().collect())
}

/// `WP_HTML_Decoder::decode_attribute()`: the character references of an attribute value.
pub fn decode_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < value.len() {
        let rest = &value[i..];
        if !rest.starts_with('&') {
            let c = rest.chars().next().unwrap_or(' ');
            out.push(c);
            i += c.len_utf8();
            continue;
        }
        // Numeric.
        if let Some(number) = rest.strip_prefix("&#") {
            let hex = number.starts_with(['x', 'X']);
            let digits_at = if hex { 3 } else { 2 };
            let digits: String = rest[digits_at..].chars().take_while(|c| if hex { c.is_ascii_hexdigit() } else { c.is_ascii_digit() }).collect();
            if !digits.is_empty() {
                let code = u32::from_str_radix(&digits, if hex { 16 } else { 10 }).unwrap_or(0x110000);
                out.push(numeric_reference(code));
                i += digits_at + digits.len();
                if value[i..].starts_with(';') {
                    i += 1;
                }
                continue;
            }
            out.push('&');
            i += 1;
            continue;
        }
        // Named: the longest name that is in the table, with its semicolon or, for the old names, without.
        let names = named_references();
        let mut matched: Option<(usize, &str)> = None;
        let max = rest.len().min(33);
        for len in (2..=max).rev() {
            if !rest.is_char_boundary(len) {
                continue;
            }
            if let Some(text) = names.get(&rest[1..len]) {
                matched = Some((len, text));
                break;
            }
        }
        match matched {
            Some((len, text)) => {
                let ends_with_semicolon = rest[..len].ends_with(';');
                let next = bytes.get(i + len).copied();
                if !ends_with_semicolon && next.is_some_and(|b| b == b'=' || b.is_ascii_alphanumeric()) {
                    out.push('&');
                    i += 1;
                } else {
                    out.push_str(text);
                    i += len;
                }
            }
            None => {
                out.push('&');
                i += 1;
            }
        }
    }
    out
}

/// One attribute as `wp_kses_hair()` returns it: the lower-case name, the value (already encoded for output) and whether it has none.
struct Attribute {
    name: String,
    value: String,
    whole: String,
    valueless: bool,
    /// The value as written, before the syntax characters were encoded.
    decoded: String,
}

/// The attributes of the tag `<wp {attr}>` as a tokenizer reads them; `None` when the tag does not end.
fn read_attributes(attr: &str) -> Option<Vec<(String, Option<String>)>> {
    let chars: Vec<char> = attr.chars().collect();
    let is_space = |c: char| matches!(c, ' ' | '\t' | '\n' | '\x0c' | '\r');
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    let mut i = 0;
    let n = chars.len();
    loop {
        // Before the name of an attribute.
        while i < n && is_space(chars[i]) {
            i += 1;
        }
        if i >= n {
            // The tag is not closed by `>` in the text we were given: it is, because we add it.
            return Some(out);
        }
        if chars[i] == '/' {
            i += 1;
            continue;
        }
        // The name.
        let mut name = String::new();
        if chars[i] == '=' {
            name.push('=');
            i += 1;
        }
        while i < n && !is_space(chars[i]) && chars[i] != '/' && chars[i] != '=' {
            name.push(chars[i]);
            i += 1;
        }
        let name = name.to_lowercase();
        // After the name: spaces, then `=` or the next attribute.
        let mut j = i;
        while j < n && is_space(chars[j]) {
            j += 1;
        }
        let mut value: Option<String> = None;
        if j < n && chars[j] == '=' {
            j += 1;
            while j < n && is_space(chars[j]) {
                j += 1;
            }
            if j >= n {
                // `name=` at the end: the value is empty.
                value = Some(String::new());
            } else if chars[j] == '"' || chars[j] == '\'' {
                let quote = chars[j];
                let start = j + 1;
                let mut end = start;
                while end < n && chars[end] != quote {
                    end += 1;
                }
                if end >= n {
                    // The quote is never closed: the tag is incomplete.
                    return None;
                }
                value = Some(decode_attribute(&chars[start..end].iter().collect::<String>()));
                j = end + 1;
            } else {
                let start = j;
                while j < n && !is_space(chars[j]) {
                    j += 1;
                }
                value = Some(decode_attribute(&chars[start..j].iter().collect::<String>()));
            }
            i = j;
        } else {
            i = j;
        }
        if !name.is_empty() && !out.iter().any(|(existing, _)| *existing == name) {
            out.push((name, value));
        }
    }
}

/// `wp_kses_hair()`.
fn hair(attr: &str, allowed_protocols: &[&str]) -> Vec<Attribute> {
    let Some(read) = read_attributes(attr) else { return Vec::new() };
    read.into_iter()
        .map(|(name, value)| match value {
            None => Attribute { whole: name.clone(), name, value: String::new(), valueless: true, decoded: String::new() },
            Some(mut value) => {
                if URI_ATTRIBUTES.contains(&name.as_str()) {
                    value = bad_protocol(&value, allowed_protocols);
                }
                let recoded = value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "&apos;").replace('"', "&quot;");
                Attribute { whole: format!("{name}=\"{recoded}\""), name, value: recoded, valueless: false, decoded: value }
            }
        })
        .collect()
}

/// `_wp_kses_directive_binds_value_restricted_attribute()` (7.1.4) for `object`, the one element of post content whose
/// attribute values KSES restricts (`data` and `type`): an Interactivity API binding stays only when it binds an
/// attribute allowed there with any value. Both the whole name and the part before a unique ID (`---`) are checked.
fn binds_restricted_object_attribute(allowed: &Allowed, name_low: &str) -> bool {
    let Some(bound) = name_low.strip_prefix("data-wp-bind--") else { return false };
    let mut names = vec![bound];
    if let Some(at) = bound.find("---") {
        names.push(&bound[..at]);
    }
    names.iter().any(|name| matches!(*name, "data" | "type") || !allowed.permits("object", name))
}

/// `wp_kses_attr()`: the start tag with the attributes that are allowed.
fn attr_tag(element: &str, attr: &str, allowed: &Allowed) -> String {
    static SLASH: OnceLock<Regex> = OnceLock::new();
    let xhtml_slash = if re(&SLASH, r"\s*/\s*$").is_match(attr) { " /" } else { "" };
    if allowed.attributes(element).is_none_or(|a| a.is_empty()) {
        return format!("<{element}{xhtml_slash}>");
    }
    let element_low = element.to_lowercase();
    // `wp_kses_attr()`: an `object` of post content needs both of its attributes, each with an allowed value.
    let mut required: Vec<&str> = if allowed.post && element_low == "object" { vec!["data", "type"] } else { Vec::new() };
    let mut kept = String::new();
    for mut attribute in hair(attr, &PROTOCOLS) {
        // Attribute names are kept as read; the rules apply to the lower case.
        let name_low = attribute.name.to_lowercase();
        if !allowed.permits(element, &name_low) {
            continue;
        }
        if allowed.post {
            if element_low == "object" && binds_restricted_object_attribute(allowed, &name_low) {
                continue;
            }
            if name_low == "style" {
                let cleaned = crate::safe_css::safecss_filter_attr(&attribute.decoded);
                if cleaned.is_empty() {
                    continue;
                }
                if cleaned != attribute.decoded {
                    let encoded = crate::formatting::esc_attr(&cleaned);
                    attribute.whole = attribute.whole.replace(&attribute.value, &encoded);
                    attribute.value = encoded;
                }
            }
            // `wp_kses_check_attr_val()`: `download` is valueless; `object` takes PDF files of the site only, and
            // without the site in reach none is allowed (`_wp_kses_allow_pdf_objects`).
            if element_low == "a" && name_low == "download" && !attribute.valueless {
                continue;
            }
            if element_low == "object" {
                let ok = match name_low.as_str() {
                    "type" => attribute.decoded == "application/pdf",
                    "data" => false,
                    _ => true,
                };
                if !ok {
                    // A failed required attribute: nothing for a self-closing tag, the bare tag otherwise (its closing tag stays).
                    return if xhtml_slash.is_empty() { format!("<{element}>") } else { String::new() };
                }
                required.retain(|r| *r != name_low);
            }
        }
        kept.push(' ');
        kept.push_str(&attribute.whole);
    }
    if !required.is_empty() {
        return if xhtml_slash.is_empty() { format!("<{element}>") } else { String::new() };
    }
    let kept: String = kept.chars().filter(|c| *c != '<' && *c != '>').collect();
    format!("<{element}{kept}{xhtml_slash}>")
}

/// `wp_kses_split2()`: one token of the content.
fn split2(token: &str, allowed: &Allowed) -> String {
    static BOGUS: OnceLock<Regex> = OnceLock::new();
    static TAG: OnceLock<Regex> = OnceLock::new();
    // `wp_kses_stripslashes()`: the markup was slashed, `\"` is `"`.
    let token = token.replace("\\\"", "\"");
    if !token.starts_with('<') {
        return "&gt;".to_owned();
    }
    if re(&BOGUS, r"^(?:</[^a-zA-Z][^>]*>|<![a-z][^>]*>)$").is_match(&token) {
        let opener = token.chars().nth(1).unwrap_or('!');
        let mut content = token[2..token.len() - 1].to_owned();
        loop {
            let previous = content.clone();
            content = kses_inner(&content, allowed);
            if previous == content {
                break;
            }
        }
        return format!("<{opener}{content}>");
    }
    if token.starts_with("<!--") {
        let mut content = token.replace("<!--", "").replace("-->", "");
        loop {
            let next = kses_inner(&content, allowed);
            if next == content {
                break;
            }
            content = next;
        }
        if content.is_empty() {
            return String::new();
        }
        static DASHES: OnceLock<Regex> = OnceLock::new();
        let content = re(&DASHES, r"--+").replace_all(&content, "-").into_owned();
        let content = content.strip_suffix('-').unwrap_or(&content).to_owned();
        return format!("<!--{content}-->");
    }
    let Some(caps) = re(&TAG, r"^<\s*(/\s*)?([a-zA-Z0-9-]+)([^>]*)>?$").captures(&token) else { return String::new() };
    let slash = caps.get(1).map_or("", |m| m.as_str()).trim();
    let element = &caps[2];
    if allowed.attributes(element).is_none() {
        return String::new();
    }
    if !slash.is_empty() {
        return format!("</{element}>");
    }
    attr_tag(element, &caps[3], allowed)
}

/// The tokens `wp_kses_split()` cuts the content into: comments, tags, and a lone `>`.
fn split(content: &str, allowed: &Allowed) -> String {
    static TOKENS: OnceLock<Regex> = OnceLock::new();
    re(&TOKENS, r"(?s)(<!--.*?(-->|$))|(<[^>]*(>|$)|>)").replace_all(content, |caps: &regex::Captures| split2(&caps[0], allowed)).into_owned()
}

/// `wp_pre_kses_less_than()`: a `<` that no `>` closes is text.
pub fn pre_kses_less_than(content: &str) -> String {
    static CELL: OnceLock<FancyRegex> = OnceLock::new();
    let pattern = CELL.get_or_init(|| FancyRegex::new(r"<[^>]*?((?=<)|>|$)").expect("pattern"));
    pattern
        .replace_all(content, |caps: &fancy_regex::Captures<str>| {
            let found = &caps[0];
            if found.contains('>') { found.to_owned() } else { crate::formatting::esc_html(found) }
        })
        .into_owned()
}

/// `wp_kses()` of the content after the entities were normalized (the recursion of comments).
fn kses_inner(content: &str, allowed: &Allowed) -> String {
    let content = no_null(content, false);
    let content = normalize_entities(&content);
    let content = pre_kses_less_than(&content);
    split(&content, allowed)
}

/// `wp_kses( $content, $allowed )`.
pub fn kses(content: &str, allowed: &Allowed) -> String {
    kses_inner(content, allowed)
}

/// `wp_kses_post()`: what post content may contain.
pub fn kses_post(content: &str) -> String {
    static POST: OnceLock<Allowed> = OnceLock::new();
    kses(content, POST.get_or_init(Allowed::post))
}

/// `wp_filter_kses()` / `wp_kses_data()`: what a visitor may write.
pub fn kses_data(content: &str) -> String {
    kses(content, &Allowed::data_without_span())
}

/// The `pre_comment_content` filters of a visitor: `convert_invalid_entities`, `wp_filter_kses`, the classes of
/// note mentions, `wp_rel_ugc`. `home` is the address of the site (a link to it is not "nofollow").
pub fn comment_content_filter(content: &str, home: &str, strict: bool) -> String {
    let content = convert_invalid_entities(content);
    // Somebody who may write any HTML (`unfiltered_html`) has no kses filter.
    if !strict {
        return rel_ugc(&content, home);
    }
    let content = kses(&content, &Allowed::data());
    let content = sanitize_note_mention_classes(&content);
    rel_ugc(&content, home)
}

/// `convert_invalid_entities()`: the numeric references of the Windows-1252 range name the character they were meant to.
fn convert_invalid_entities(content: &str) -> String {
    static CELL: OnceLock<Regex> = OnceLock::new();
    const TABLE: [(u32, &str); 27] = [
        (128, "&#8364;"), (130, "&#8218;"), (131, "&#402;"), (132, "&#8222;"), (133, "&#8230;"), (134, "&#8224;"), (135, "&#8225;"), (136, "&#710;"), (137, "&#8240;"), (138, "&#352;"), (139, "&#8249;"), (140, "&#338;"),
        (142, "&#381;"), (145, "&#8216;"), (146, "&#8217;"), (147, "&#8220;"), (148, "&#8221;"), (149, "&#8226;"), (150, "&#8211;"), (151, "&#8212;"), (152, "&#732;"), (153, "&#8482;"), (154, "&#353;"), (155, "&#8250;"),
        (156, "&#339;"), (158, "&#382;"), (159, "&#376;"),
    ];
    re(&CELL, r"&#(1[2-5][0-9]);")
        .replace_all(content, |caps: &regex::Captures| {
            let code: u32 = caps[1].parse().unwrap_or(0);
            TABLE.iter().find(|(c, _)| *c == code).map_or_else(|| caps[0].to_owned(), |(_, text)| (*text).to_owned())
        })
        .into_owned()
}

/// `_wp_kses_sanitize_note_mention_classes()`: a `span` keeps only the classes of a mention.
fn sanitize_note_mention_classes(content: &str) -> String {
    static SPAN: OnceLock<Regex> = OnceLock::new();
    static USER: OnceLock<Regex> = OnceLock::new();
    re(&SPAN, r#"(?i)<span\s+class="([^"]*)"([^>]*)>"#)
        .replace_all(content, |caps: &regex::Captures| {
            let kept: Vec<&str> = caps[1].split_whitespace().filter(|token| *token == "wp-note-mention" || re(&USER, r"^user-[1-9][0-9]*$").is_match(token)).collect();
            if kept.is_empty() { format!("<span{}>", &caps[2]) } else { format!("<span class=\"{}\"{}>", kept.join(" "), &caps[2]) }
        })
        .into_owned()
}

/// `wp_is_internal_link()`.
fn is_internal_link(link: &str, home: &str) -> bool {
    let lower = link.to_lowercase();
    let scheme = lower.split_once(':').map(|(s, _)| s);
    scheme.is_some_and(|s| PROTOCOLS.contains(&s)) && lower.starts_with(&home.to_lowercase())
}

/// `wp_rel_ugc()`: links get `rel="nofollow ugc"`, links to the site `rel="ugc"`.
pub fn rel_ugc(content: &str, home: &str) -> String {
    static LINK: OnceLock<Regex> = OnceLock::new();
    re(&LINK, r"(?i)<a (.+?)>")
        .replace_all(content, |caps: &regex::Captures| {
            let text = caps[1].to_owned();
            let attrs = hair(&text, &PROTOCOLS);
            let mut rel = "nofollow ugc".to_owned();
            if attrs.iter().find(|a| a.name == "href").is_some_and(|href| !href.value.is_empty() && is_internal_link(&href.value, home)) {
                rel = rel.replace("nofollow", "").trim().to_owned();
            }
            let mut text = text;
            if let Some(existing) = attrs.iter().find(|a| a.name == "rel").filter(|a| !a.value.is_empty()) {
                let mut parts: Vec<String> = existing.value.split(' ').map(|p| p.trim().to_owned()).collect();
                for part in rel.split(' ').map(|p| p.trim().to_owned()) {
                    if !parts.contains(&part) {
                        parts.push(part);
                    }
                }
                rel = parts.join(" ");
                let mut html = String::new();
                for attribute in attrs.iter().filter(|a| a.name != "rel") {
                    if attribute.valueless {
                        html.push_str(&format!("{} ", attribute.name));
                    } else {
                        html.push_str(&format!("{}=\"{}\" ", attribute.name, crate::formatting::esc_attr(&attribute.value)));
                    }
                }
                text = html.trim().to_owned();
            }
            let rel_attr = if rel.is_empty() { String::new() } else { format!(" rel=\"{}\"", crate::formatting::esc_attr(&rel)) };
            format!("<a {text}{rel_attr}>")
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_and_attributes_are_filtered() {
        assert_eq!(kses_data("<b onclick=x>t</b> <script>x</script>"), "<b>t</b> x");
        assert_eq!(kses_data("<a href=\"javascript:alert(1)\" title=x>hi</a>"), "<a href=\"alert(1)\" title=\"x\">hi</a>");
        assert_eq!(kses_data("Tom & Jerry <3 &copy; &bogus;"), "Tom &amp; Jerry &lt;3 &copy; &amp;bogus;");
        assert_eq!(kses_data("<a href=\"http://example.com/?a=1&b=2\">x</a>"), "<a href=\"http://example.com/?a=1&amp;b=2\">x</a>");
    }

    #[test]
    fn object_bindings_of_restricted_attributes_are_dropped() {
        let post = Allowed::post();
        for kept in ["data-wp-bind--class", "data-wp-bind--aria-label", "data-wp-bind--data-x", "data-wp-context"] {
            assert!(!binds_restricted_object_attribute(&post, kept), "{kept}");
        }
        // `data` and `type` have restricted values, `width` and `onload` are not allowed on `object` at all, and a
        // unique ID makes the whole name one that is not allowed.
        for dropped in ["data-wp-bind--data", "data-wp-bind--type---x", "data-wp-bind--title---id", "data-wp-bind--width", "data-wp-bind--onload"] {
            assert!(binds_restricted_object_attribute(&post, dropped), "{dropped}");
        }
    }

    #[test]
    fn comments_get_rel_ugc() {
        assert_eq!(comment_content_filter("<a href=\"http://example.org/\">x</a>", "http://localhost", true), "<a href=\"http://example.org/\" rel=\"nofollow ugc\">x</a>");
        assert_eq!(comment_content_filter("<a href=\"http://localhost/page/\">x</a>", "http://localhost", true), "<a href=\"http://localhost/page/\" rel=\"ugc\">x</a>");
    }
}
