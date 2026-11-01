//! Form-tags (`includes/form-tag.php`, `includes/form-tags-manager.php`, `includes/pipe.php`): the scanner that finds
//! `[text* your-name ...]` in a form template, the parsed tag with its options, and the pipes of its values.

use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

use super::formatter::{PLACEHOLDER_BLOCK, PLACEHOLDER_INLINE, Side, is_unicode_whitespace, normalize_start_tag, php_trim, sha256_hex, strip_whitespaces};

/// `wpcf7_get_unicode_whitespaces()` as a character class body.
const UNICODE_WS: &str = r"\x09-\x0D\x20\x{85}\x{A0}\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}";

/// What a form-tag type supports (`$features` of `wpcf7_add_form_tag()`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Features {
    pub name_attr: bool,
    pub selectable_values: bool,
    pub multiple_controls_container: bool,
    pub zero_controls_container: bool,
    pub display_block: bool,
    pub display_hidden: bool,
    pub singular: bool,
    pub not_for_mail: bool,
    pub do_not_store: bool,
    pub file_uploading: bool,
}

/// The form-tag types the modules of Contact Form 7 register, in the order `WPCF7::load_modules()` adds them, with
/// what each supports (`stripe` and `turnstile` exist even while their services are not set up).
pub const TAG_TYPES: &[(&str, Features)] = {
    const NAME: Features = Features { name_attr: true, ..EMPTY };
    const EMPTY: Features = Features {
        name_attr: false,
        selectable_values: false,
        multiple_controls_container: false,
        zero_controls_container: false,
        display_block: false,
        display_hidden: false,
        singular: false,
        not_for_mail: false,
        do_not_store: false,
        file_uploading: false,
    };
    const SELECTABLE: Features = Features { name_attr: true, selectable_values: true, ..EMPTY };
    const CHECKBOX: Features = Features { name_attr: true, selectable_values: true, multiple_controls_container: true, ..EMPTY };
    const COUNT: Features = Features { name_attr: true, zero_controls_container: true, not_for_mail: true, ..EMPTY };
    const DATE: Features = NAME;
    const FILE: Features = Features { name_attr: true, file_uploading: true, ..EMPTY };
    const HIDDEN: Features = Features { name_attr: true, display_hidden: true, ..EMPTY };
    const QUIZ: Features = Features { name_attr: true, do_not_store: true, not_for_mail: true, ..EMPTY };
    const REFLECTION: Features = Features { name_attr: true, display_block: true, not_for_mail: true, ..EMPTY };
    const OUTPUT: Features = Features { name_attr: true, not_for_mail: true, ..EMPTY };
    const RESPONSE: Features = Features { display_block: true, ..EMPTY };
    &[
        ("acceptance", SELECTABLE),
        ("checkbox", CHECKBOX),
        ("checkbox*", CHECKBOX),
        ("radio", CHECKBOX),
        ("count", COUNT),
        ("date", DATE),
        ("date*", DATE),
        ("file", FILE),
        ("file*", FILE),
        ("hidden", HIDDEN),
        ("number", NAME),
        ("number*", NAME),
        ("range", NAME),
        ("range*", NAME),
        ("quiz", QUIZ),
        ("captchac", COUNT),
        ("captchar", QUIZ),
        ("reflection", REFLECTION),
        ("output", OUTPUT),
        ("response", RESPONSE),
        ("select", SELECTABLE),
        ("select*", SELECTABLE),
        ("stripe", Features { display_block: true, singular: true, ..EMPTY }),
        ("submit", EMPTY),
        ("text", NAME),
        ("text*", NAME),
        ("email", NAME),
        ("email*", NAME),
        ("url", NAME),
        ("url*", NAME),
        ("tel", NAME),
        ("tel*", NAME),
        ("textarea", NAME),
        ("textarea*", NAME),
        ("turnstile", RESPONSE),
    ]
};

/// `tag_type_supports()`'s answer for a type: its features.
pub fn features(tag_type: &str) -> Features {
    TAG_TYPES.iter().find(|(t, _)| *t == tag_type).map(|(_, f)| *f).unwrap_or_default()
}

/// `WPCF7_Pipe`: `before|after`.
#[derive(Debug, Clone, PartialEq)]
pub struct Pipe {
    pub before: String,
    pub after: String,
}

impl Pipe {
    pub fn new(text: &str) -> Self {
        match text.find('|') {
            None => {
                let s = strip_whitespaces(text, Side::Both).to_owned();
                Self { before: s.clone(), after: s }
            }
            Some(at) => Self { before: strip_whitespaces(&text[..at], Side::Both).to_owned(), after: strip_whitespaces(&text[at + 1..], Side::Both).to_owned() },
        }
    }
}

/// `WPCF7_FormTag`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormTag {
    pub tag_type: String,
    pub basetype: String,
    pub raw_name: String,
    pub name: String,
    pub options: Vec<String>,
    pub raw_values: Vec<String>,
    pub values: Vec<String>,
    pub pipes: Vec<Pipe>,
    pub labels: Vec<String>,
    /// The attribute text of a tag that is not in the options-and-values syntax.
    pub attr: String,
    pub content: String,
}

/// `preg_quote()` for building the patterns of the option getters.
fn quote(s: &str) -> String {
    regex::escape(s)
}

impl FormTag {
    pub fn is_required(&self) -> bool {
        self.tag_type.ends_with('*')
    }

    /// `has_option( $name )`: `name` or `name:...`.
    pub fn has_option(&self, name: &str) -> bool {
        let pattern = Regex::new(&format!("(?i)^{}(:.+)?$", quote(name))).expect("valid option pattern");
        self.options.iter().any(|o| pattern.is_match(o))
    }

    fn preset(pattern: &str) -> &str {
        match pattern {
            "date" => "[0-9]{4}-[0-9]{2}-[0-9]{2}",
            "int" => "[0-9]+",
            "signed_int" => "[-]?[0-9]+",
            "num" => "(?:[0-9]+|(?:[0-9]+)?[.][0-9]+)",
            "signed_num" => "[-]?(?:[0-9]+|(?:[0-9]+)?[.][0-9]+)",
            "class" | "id" => "[-0-9a-zA-Z_]+",
            "" => ".+",
            other => other,
        }
    }

    fn option_regex(name: &str, pattern: &str) -> Regex {
        Regex::new(&format!("(?i)^{}:{}$", quote(name), Self::preset(pattern))).expect("valid option pattern")
    }

    /// `get_option( $name, $pattern, true )`: the value after `name:` of the first option that matches.
    pub fn get_option(&self, name: &str, pattern: &str) -> Option<String> {
        let regex = Self::option_regex(name, pattern);
        self.options.iter().find(|o| regex.is_match(o)).map(|o| o[name.len() + 1..].to_owned())
    }

    /// `get_option( $name, $pattern )`: the values of every option that matches.
    pub fn get_options(&self, name: &str, pattern: &str) -> Vec<String> {
        let regex = Self::option_regex(name, pattern);
        self.options.iter().filter(|o| regex.is_match(o)).map(|o| o[name.len() + 1..].to_owned()).collect()
    }

    /// `get_class_option( $default_classes )`.
    pub fn get_class_option(&self, default_classes: &str) -> Option<String> {
        let mut classes: Vec<String> = Vec::new();
        for class in default_classes.split(' ').map(str::to_owned).chain(self.get_options("class", "")) {
            let class = rustpress_core::formatting::sanitize_html_class(&class);
            if !class.is_empty() && !classes.contains(&class) {
                classes.push(class);
            }
        }
        (!classes.is_empty()).then(|| classes.join(" "))
    }

    /// `get_autocomplete_option()`.
    pub fn get_autocomplete_option(&self) -> Option<String> {
        static TOKEN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z]+(?:-[0-9a-z]+)*$").unwrap());
        let mut tokens: Vec<String> = Vec::new();
        for option in self.get_options("autocomplete", "[-0-9a-zA-Z|]+") {
            for token in option.split('|').map(str::to_lowercase) {
                if TOKEN.is_match(&token) && !tokens.contains(&token) {
                    tokens.push(token);
                }
            }
        }
        if tokens.is_empty() {
            None
        } else if tokens.iter().any(|t| t == "off") {
            Some("off".to_owned())
        } else if tokens.iter().any(|t| t == "on") {
            Some("on".to_owned())
        } else {
            Some(tokens.join(" "))
        }
    }

    /// The first non-empty capture `group` of `pattern` over the options.
    fn first_capture(&self, pattern: &Regex, group: usize) -> Option<String> {
        self.options.iter().filter_map(|o| pattern.captures(o)).find_map(|c| c.get(group).map(|m| m.as_str().to_owned()).filter(|s| !s.is_empty()))
    }

    /// `get_size_option()`: `size:N` or `N/M`.
    pub fn get_size_option(&self) -> Option<String> {
        static SIZE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]*)/[0-9]*$").unwrap());
        self.get_option("size", "int").filter(|s| truthy(s)).or_else(|| self.first_capture(&SIZE, 1))
    }

    /// `get_maxlength_option()`: `maxlength:N`, `N/M` or `NxM/L`.
    pub fn get_maxlength_option(&self) -> Option<String> {
        static MAX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[0-9]*x?[0-9]*)?/([0-9]+)$").unwrap());
        self.get_option("maxlength", "int").filter(|s| truthy(s)).or_else(|| self.first_capture(&MAX, 1))
    }

    pub fn get_minlength_option(&self) -> Option<String> {
        self.get_option("minlength", "int").filter(|s| truthy(s))
    }

    /// `get_cols_option()`: `cols:N` or `NxM`.
    pub fn get_cols_option(&self) -> Option<String> {
        static COLS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]*)x([0-9]*)(?:/[0-9]+)?$").unwrap());
        self.get_option("cols", "int").filter(|s| truthy(s)).or_else(|| self.first_capture(&COLS, 1))
    }

    /// `get_rows_option()`: `rows:N` or `NxM`.
    pub fn get_rows_option(&self) -> Option<String> {
        static ROWS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]*)x([0-9]*)(?:/[0-9]+)?$").unwrap());
        self.get_option("rows", "int").filter(|s| truthy(s)).or_else(|| self.first_capture(&ROWS, 2))
    }

    /// `get_limit_option()`: the size limit of a file field in bytes (1 MB by default).
    pub fn get_limit_option(&self) -> u64 {
        static LIMIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^limit:([1-9][0-9]*)([kKmM]?[bB])?$").unwrap());
        for option in &self.options {
            if let Some(c) = LIMIT.captures(option) {
                let size: u64 = c[1].parse().unwrap_or(0);
                return match c.get(2).map(|m| m.as_str().to_lowercase()).as_deref() {
                    Some("kb") => size * 1024,
                    Some("mb") => size * 1024 * 1024,
                    _ => size,
                };
            }
        }
        1024 * 1024
    }
}

/// `if ( $option )` for a string.
pub fn truthy(s: &str) -> bool {
    !s.is_empty() && s != "0"
}

/// `wpcf7_is_name()`.
pub fn is_name(text: &str) -> bool {
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z][-A-Za-z0-9_:.]*$").unwrap());
    NAME.is_match(text)
}

/// `wpcf7_strip_quote()`.
fn strip_quote(text: &str) -> String {
    let text = strip_whitespaces(text, Side::Both);
    for q in ['"', '\''] {
        if text.len() >= 2 && text.starts_with(q) && text.ends_with(q) {
            return text[1..text.len() - 1].to_owned();
        }
    }
    text.to_owned()
}

/// `parse_atts()`: `(options, values)`, or `None` when the text is not in that syntax.
fn parse_atts(text: &str) -> Option<(Vec<String>, Vec<String>)> {
    static PATTERN: LazyLock<FancyRegex> = LazyLock::new(|| {
        FancyRegex::new(&format!(
            r#"^([-+*=0-9a-zA-Z:.!?#$&@_/|%{UNICODE_WS}]*?)((?:[{UNICODE_WS}]*"[^"]*"|[{UNICODE_WS}]*'[^']*')*)$"#
        ))
        .unwrap()
    });
    static VALUE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]*"|'[^']*'"#).unwrap());
    static NBSP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\x{a0}\x{200b}]+").unwrap());
    let text = NBSP.replace_all(text, " ");
    let text = strip_whitespaces(&text, Side::Both);
    let c = PATTERN.captures(text).ok().flatten()?;
    let options_text = c.get(1).map_or("", |m| m.as_str());
    let values_text = c.get(2).map_or("", |m| m.as_str());
    let options = if options_text.is_empty() || options_text == "0" {
        Vec::new()
    } else {
        strip_whitespaces(options_text, Side::Both).split(is_unicode_whitespace).filter(|s| !s.is_empty()).map(str::to_owned).collect()
    };
    let values = if values_text.is_empty() || values_text == "0" { Vec::new() } else { VALUE.find_iter(values_text).map(|m| strip_quote(m.as_str())).collect() };
    Some((options, values))
}

/// A match of the form-tag regex: the six groups of `tag_regex()`.
pub struct TagMatch<'a> {
    pub whole: &'a str,
    pub start: usize,
    pub end: usize,
    pub open: &'a str,
    pub tag_type: &'a str,
    pub attr: &'a str,
    pub slash: &'a str,
    pub content: &'a str,
    pub close: &'a str,
}

/// `tag_regex()` for the registered types.
fn tag_regex() -> &'static FancyRegex {
    static REGEX: LazyLock<FancyRegex> = LazyLock::new(|| {
        let types = TAG_TYPES.iter().map(|(t, _)| fancy_regex::escape(t).into_owned()).collect::<Vec<_>>().join("|");
        FancyRegex::new(&format!(r"(?s)(\[?)\[({types})(?:[{UNICODE_WS}]+(.*?))?(?:[{UNICODE_WS}]+(/))?\](?:([^\[]*?)\[/\2\])?(\]?)")).unwrap()
    });
    &REGEX
}

/// Every match of the form-tag regex in `content`, like `preg_replace_callback()` walks them.
pub fn matches(content: &str) -> Vec<TagMatch<'_>> {
    let mut out = Vec::new();
    for c in tag_regex().captures_iter(content).flatten() {
        let whole = c.get(0).expect("group 0");
        let g = |i: usize| c.get(i).map_or("", |m| m.as_str());
        out.push(TagMatch { whole: whole.as_str(), start: whole.start(), end: whole.end(), open: g(1), tag_type: g(2), attr: g(3), slash: g(4), content: g(5), close: g(6) });
    }
    out
}

/// `replace_with_placeholders()`: the form-tags become placeholder elements that `wpcf7_autop()` leaves alone.
pub fn replace_with_placeholders(content: &str) -> (String, Vec<(String, String)>) {
    let mut placeholders = Vec::new();
    let mut out = String::with_capacity(content.len());
    let mut at = 0;
    for m in matches(content) {
        out.push_str(&content[at..m.start]);
        at = m.end;
        if m.open == "[" && m.close == "]" {
            out.push_str(m.whole);
            continue;
        }
        let f = features(m.tag_type);
        let name = if f.display_block || f.display_hidden { PLACEHOLDER_BLOCK } else { PLACEHOLDER_INLINE };
        let placeholder = normalize_start_tag(&format!("<{name} id=\"{}\" />", sha256_hex(m.whole))).0;
        placeholders.push((placeholder.clone(), m.whole.to_owned()));
        out.push_str(&placeholder);
    }
    out.push_str(&content[at..]);
    (out, placeholders)
}

/// `restore_from_placeholders()`.
pub fn restore_from_placeholders(content: &str, placeholders: &[(String, String)]) -> String {
    let mut out = content.to_owned();
    for (placeholder, tag) in placeholders {
        out = out.replace(placeholder.as_str(), tag);
    }
    out
}

/// What `scan_callback()` makes of a match: the tag, or `None` when the match stays as it is.
pub fn scan_one(m: &TagMatch<'_>, scanned: &[FormTag]) -> Option<FormTag> {
    static BR_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)<br[\r\n\t ]*/?>$").unwrap());
    let f = features(m.tag_type);
    let basetype = m.tag_type.trim_matches('*').to_owned();
    if f.singular && scanned.iter().any(|t| t.basetype == basetype) {
        return None;
    }
    let attr = parse_atts(m.attr);
    let mut tag = FormTag { tag_type: m.tag_type.to_owned(), basetype, ..FormTag::default() };
    let mut attr = attr;
    if f.name_attr {
        let (options, _) = attr.as_mut()?;
        tag.raw_name = if options.is_empty() { String::new() } else { options.remove(0) };
        if !is_name(&tag.raw_name) {
            return None;
        }
        tag.name = tag.raw_name.replace('.', "_");
    }
    match attr {
        Some((options, values)) => {
            tag.options = options;
            tag.raw_values = values;
            tag.pipes = tag.raw_values.iter().map(|v| Pipe::new(v)).collect();
            tag.values = tag.pipes.iter().map(|p| p.before.clone()).collect();
            tag.labels = tag.values.clone();
        }
        None => tag.attr = strip_whitespaces(m.attr, Side::Both).to_owned(),
    }
    tag.values = tag.values.iter().map(|v| php_trim(v).to_owned()).collect();
    tag.labels = tag.labels.iter().map(|v| php_trim(v).to_owned()).collect();
    let content = php_trim(m.content);
    tag.content = BR_END.replace_all(content, "").into_owned();
    Some(tag)
}

/// `scan()`: the tags of a form template.
pub fn scan(content: &str) -> Vec<FormTag> {
    let mut scanned = Vec::new();
    for m in matches(content) {
        if m.open == "[" && m.close == "]" {
            continue;
        }
        if let Some(tag) = scan_one(&m, &scanned) {
            scanned.push(tag);
        }
    }
    scanned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_options_and_values() {
        let tags = scan(r#"[text* your-name class:a 30/80 "Jane" 'x|y'] [[text escaped]] [submit "Send"]"#);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "your-name");
        assert_eq!(tags[0].options, ["class:a", "30/80"]);
        assert_eq!(tags[0].values, ["Jane", "x"]);
        assert_eq!(tags[0].pipes[1].after, "y");
        assert_eq!(tags[0].get_size_option().as_deref(), Some("30"));
        assert_eq!(tags[0].get_maxlength_option().as_deref(), Some("80"));
        assert_eq!(tags[1].tag_type, "submit");
        assert_eq!(tags[1].values, ["Send"]);
    }

    #[test]
    fn options_after_values_break_the_tag() {
        assert!(scan(r#"[text* your-name placeholder "Jane Doe" class:x]"#).is_empty());
    }

    #[test]
    fn tags_with_content() {
        let tags = scan("[acceptance terms] I accept <a href=\"/t\">terms</a>. [/acceptance]");
        assert_eq!(tags[0].content, "I accept <a href=\"/t\">terms</a>.");
    }
}
