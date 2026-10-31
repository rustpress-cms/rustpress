//! `WPCF7_HTMLFormatter` (`includes/html-formatter.php`) and the helpers of `includes/formatting.php` it relies on:
//! `wpcf7_autop()`, `wpcf7_format_atts()`, `wpcf7_strip_whitespaces()`.

use std::sync::LazyLock;

use regex::Regex;
use rustpress_core::formatting::esc_attr;

/// `\s` of PCRE without the `u` flag.
const WS: &str = r"[\t\n\x0B\x0C\r ]";

pub const PLACEHOLDER_BLOCK: &str = "placeholder:block";
pub const PLACEHOLDER_INLINE: &str = "placeholder:inline";

const VOID_ELEMENTS: &[&str] = &["area", "base", "br", "col", "embed", "hr", "img", "input", "keygen", "link", "meta", "param", "source", "track", "wbr", PLACEHOLDER_BLOCK, PLACEHOLDER_INLINE];

const NESTABLE_ELEMENTS: &[&str] = &["article", "aside", "blockquote", "div", "fieldset", "section", "span"];

const P_PARENT_ELEMENTS: &[&str] = &[
    "address", "article", "aside", "blockquote", "body", "caption", "dd", "details", "dialog", "div", "dt", "fieldset", "figcaption", "figure", "footer", "form", "header", "li", "main", "nav", "section", "td", "th",
];

const P_NONPARENT_ELEMENTS: &[&str] = &[
    "colgroup", "dl", "h1", "h2", "h3", "h4", "h5", "h6", "head", "hgroup", "html", "legend", "menu", "ol", "pre", "style", "summary", "table", "tbody", "template", "tfoot", "thead", "title", "tr", "ul",
];

const P_CHILD_ELEMENTS: &[&str] = &[
    "a", "abbr", "area", "audio", "b", "bdi", "bdo", "br", "button", "canvas", "cite", "code", "data", "datalist", "del", "dfn", "em", "embed", "i", "iframe", "img", "input", "ins", "kbd", "keygen", "label", "link", "map",
    "mark", "meta", "meter", "noscript", "object", "output", "picture", "progress", "q", "ruby", "s", "samp", "script", "select", "slot", "small", "span", "strong", "sub", "sup", "textarea", "time", "u", "var", "video", "wbr",
    "optgroup", "option", "rp", "rt", PLACEHOLDER_INLINE,
];

const BR_PARENT_ELEMENTS: &[&str] = &[
    "a", "abbr", "address", "article", "aside", "audio", "b", "bdi", "bdo", "blockquote", "button", "canvas", "caption", "cite", "code", "data", "datalist", "dd", "del", "details", "dfn", "dialog", "div", "dt", "em",
    "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "i", "ins", "kbd", "label", "legend", "li", "main", "map", "mark", "meter", "nav", "noscript", "object", "output", "p",
    "progress", "q", "rt", "ruby", "s", "samp", "section", "slot", "small", "span", "strong", "sub", "summary", "sup", "td", "th", "time", "u", "var", "video",
];

/// The whitespace of `wpcf7_get_unicode_whitespaces()`.
pub fn is_unicode_whitespace(c: char) -> bool {
    matches!(c, '\u{09}'..='\u{0D}' | ' ' | '\u{85}' | '\u{A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}')
}

/// Which side `wpcf7_strip_whitespaces()` strips.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Start,
    End,
    Both,
}

/// `wpcf7_strip_whitespaces( $input, $side )`: the Unicode whitespace and U+FEFF around the text.
pub fn strip_whitespaces(input: &str, side: Side) -> &str {
    let ws = |c: char| is_unicode_whitespace(c) || c == '\u{FEFF}';
    match side {
        Side::Start => input.trim_start_matches(ws),
        Side::End => input.trim_end_matches(ws),
        Side::Both => input.trim_matches(ws),
    }
}

/// `trim()` of PHP.
pub fn php_trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B'])
}

/// A value of the array `wpcf7_format_atts()` takes.
#[derive(Debug, Clone, PartialEq)]
pub enum Att {
    Str(String),
    Int(i64),
    /// A float from arithmetic (`( $min + $max ) / 2`), printed like PHP casts it.
    Float(f64),
    Bool(bool),
    /// `null`: the attribute is left out.
    Null,
}

impl Att {
    pub fn str(s: impl Into<String>) -> Self {
        Self::Str(s.into())
    }

    /// A value that is `false` when missing (`get_option()` and friends).
    pub fn opt(value: Option<String>) -> Self {
        value.map_or(Self::Bool(false), Self::Str)
    }
}

/// The attributes of a tag in the order PHP's array keeps them: setting a key again keeps its place.
#[derive(Debug, Clone, Default)]
pub struct Atts(pub Vec<(String, Att)>);

impl Atts {
    pub fn set(&mut self, name: &str, value: Att) {
        match self.0.iter_mut().find(|(n, _)| n == name) {
            Some((_, slot)) => *slot = value,
            None => self.0.push((name.to_owned(), value)),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Att> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// `unset( $atts[ $name ] )`.
    pub fn unset(&mut self, name: &str) {
        self.0.retain(|(n, _)| n != name);
    }
}

/// `(string) $float` of PHP (`precision` 14).
pub fn php_float(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 1e15 {
        return format!("{}", f as i64);
    }
    let s = format!("{:.*e}", 13, f);
    let (mantissa, exp) = s.split_once('e').unwrap_or((&s, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if (-5..15).contains(&exp) {
        let digits = (13 - exp).max(0) as usize;
        let fixed = format!("{f:.digits$}");
        if fixed.contains('.') { fixed.trim_end_matches('0').trim_end_matches('.').to_owned() } else { fixed }
    } else {
        let mantissa = if mantissa.contains('.') { mantissa.trim_end_matches('0').trim_end_matches('.') } else { mantissa };
        format!("{mantissa}E{}{}", if exp < 0 { "-" } else { "+" }, exp.abs())
    }
}

/// `wpcf7_format_atts( $atts )`.
pub fn format_atts(atts: &Atts) -> String {
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z_:][a-z_:.0-9-]*$").unwrap());
    const BOOLEAN: &[&str] = &["checked", "disabled", "inert", "multiple", "readonly", "required", "selected"];
    let mut filtered: Vec<(String, String)> = Vec::new();
    for (name, value) in &atts.0 {
        let name = php_trim(name).to_ascii_lowercase();
        if !NAME.is_match(&name) {
            continue;
        }
        let value = match value {
            Att::Str(s) if s.is_empty() && BOOLEAN.contains(&name.as_str()) => Att::Bool(false),
            Att::Int(n) => Att::Str(n.to_string()),
            Att::Float(f) => Att::Str(php_float(*f)),
            other => other.clone(),
        };
        match value {
            Att::Null | Att::Bool(false) => filtered.retain(|(n, _)| *n != name),
            Att::Bool(true) => set(&mut filtered, name.clone(), name),
            Att::Str(s) => set(&mut filtered, name, php_trim(&s).to_owned()),
            Att::Int(_) | Att::Float(_) => {}
        }
    }
    fn set(list: &mut Vec<(String, String)>, name: String, value: String) {
        match list.iter_mut().find(|(n, _)| *n == name) {
            Some((_, slot)) => *slot = value,
            None => list.push((name, value)),
        }
    }
    let mut out = String::new();
    for (name, value) in &filtered {
        out.push_str(&format!(" {name}=\"{}\"", esc_attr(value)));
    }
    php_trim(&out).to_owned()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Text,
    StartTag,
    EndTag,
    Comment,
}

/// The pieces `separate_into_chunks()` cuts the input into.
fn separate_into_chunks(input: &str) -> Vec<(Kind, String)> {
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)(?:<!--.*?-->|</?[a-z].*?>)").unwrap());
    let mut chunks = Vec::new();
    let mut position = 0;
    while position < input.len() {
        let Some(m) = TAG.find_at(input, position) else {
            chunks.push((Kind::Text, input[position..].to_owned()));
            break;
        };
        if position < m.start() {
            chunks.push((Kind::Text, input[position..m.start()].to_owned()));
        }
        let tag = m.as_str();
        let kind = if tag.starts_with("<!") {
            Kind::Comment
        } else if tag.starts_with("</") {
            Kind::EndTag
        } else {
            Kind::StartTag
        };
        chunks.push((kind, tag.to_owned()));
        position = m.end();
    }
    chunks
}

/// `normalize_start_tag()`: `(tag, tag name)`; a void element ends in ` />`.
pub fn normalize_start_tag(tag: &str) -> (String, String) {
    static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<(.+?)[\t\n\x0B\x0C\r /> ]").unwrap());
    static VOID_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"{WS}*/?>")).unwrap());
    let (mut tag, name) = match NAME.captures(tag) {
        Some(c) => (tag.to_owned(), c[1].to_lowercase()),
        None => {
            let name = tag.to_lowercase();
            (format!("<{name}>"), name)
        }
    };
    if VOID_ELEMENTS.contains(&name.as_str()) {
        tag = VOID_END.replace_all(&tag, " />").into_owned();
    }
    (tag, name)
}

/// `normalize_paragraph()`.
fn normalize_paragraph(paragraph: &str, auto_br: bool) -> String {
    static BR: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"{WS}*\n{WS}*")).unwrap());
    static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ ]+").unwrap());
    let paragraph = if auto_br { BR.replace_all(paragraph, "<br />\n").into_owned() } else { paragraph.to_owned() };
    SPACES.replace_all(&paragraph, " ").into_owned()
}

fn indent(level: usize) -> String {
    "\t".repeat(level)
}

/// `WPCF7_HTMLFormatter` with `auto_indent` on.
pub struct HtmlFormatter {
    auto_br: bool,
    /// The open elements, innermost first.
    stacked: Vec<String>,
    output: String,
}

impl HtmlFormatter {
    pub fn new(auto_br: bool) -> Self {
        Self { auto_br, stacked: Vec::new(), output: String::new() }
    }

    /// `format( separate_into_chunks( $input ) )`.
    pub fn format(&mut self, input: &str) -> String {
        static BR_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"(?i)^<br{WS}*/?>$")).unwrap());
        // pre_format() and concatenate_texts().
        let mut chunks: Vec<(Kind, String)> = Vec::new();
        for (mut kind, content) in separate_into_chunks(input) {
            let mut content = content.replace("\r\n", "\n").replace('\r', "\n");
            if kind == Kind::StartTag {
                content = normalize_start_tag(&content).0;
                if self.auto_br && BR_TAG.is_match(&content) {
                    kind = Kind::Text;
                    content = "\n".to_owned();
                }
            }
            match chunks.last_mut() {
                Some((Kind::Text, text)) if kind == Kind::Text => text.push_str(&content),
                _ => chunks.push((kind, content)),
            }
        }
        self.output.clear();
        self.stacked.clear();
        for (kind, content) in chunks {
            match kind {
                Kind::Text => self.append_text(&content),
                Kind::StartTag => self.start_tag(&content),
                Kind::EndTag => self.end_tag(&content),
                Kind::Comment => self.output.push_str(&content),
            }
        }
        self.output()
    }

    fn is_inside(&self, names: &[&str]) -> bool {
        self.stacked.iter().any(|e| names.contains(&e.as_str()))
    }

    fn has_parent(&self, names: &[&str]) -> bool {
        self.stacked.first().is_some_and(|e| names.contains(&e.as_str()))
    }

    /// `append_text()`.
    pub fn append_text(&mut self, content: &str) {
        static STARTS_DOUBLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"^{WS}*\n{WS}*\n{WS}*")).unwrap());
        static ENDS_DOUBLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"{WS}*\n{WS}*\n{WS}*$")).unwrap());
        static SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"{WS}*\n{WS}*\n{WS}*")).unwrap());
        static SINGLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"^{WS}*\n{WS}*$")).unwrap());
        if self.is_inside(&["pre", "template"]) {
            self.output.push_str(content);
            return;
        }
        if self.stacked.is_empty() || self.has_parent(&["p"]) || self.has_parent(P_PARENT_ELEMENTS) {
            if STARTS_DOUBLE.is_match(content) {
                self.end_tag("p");
            }
            let paragraphs: Vec<&str> = SPLIT.split(content).filter(|p| !strip_whitespaces(p, Side::Both).is_empty()).collect();
            let mut paragraphs = paragraphs.into_iter();
            if paragraphs.len() > 0 {
                if self.is_inside(&["p"])
                    && let Some(first) = paragraphs.next()
                {
                    self.output.push_str(&normalize_paragraph(first, self.auto_br));
                }
                for paragraph in paragraphs {
                    self.start_tag("p");
                    let paragraph = strip_whitespaces(paragraph, Side::Start);
                    self.output.push_str(&normalize_paragraph(paragraph, self.auto_br));
                }
            }
            if ENDS_DOUBLE.is_match(content) {
                self.end_tag("p");
            }
            if SINGLE.is_match(content) {
                let auto_br = self.auto_br && self.is_inside(&["p"]);
                self.output.push_str(&normalize_paragraph(content, auto_br));
            }
        } else {
            let auto_br = self.auto_br && self.has_parent(BR_PARENT_ELEMENTS);
            self.output.push_str(&normalize_paragraph(content, auto_br));
        }
    }

    /// `start_tag()`.
    pub fn start_tag(&mut self, tag: &str) {
        let (tag, name) = normalize_start_tag(tag);
        if P_CHILD_ELEMENTS.contains(&name.as_str()) && !self.is_inside(&["p"]) && !self.is_inside(P_CHILD_ELEMENTS) && !self.has_parent(P_NONPARENT_ELEMENTS) {
            self.start_tag("p");
        }
        self.append_start_tag(&name, None, &tag);
    }

    /// `append_start_tag( $tag_name, $atts, $tag )`: `tag` is the whole start tag, else one is made from `atts`.
    pub fn append_start_tag(&mut self, name: &str, atts: Option<&Atts>, tag: &str) {
        static VALID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z][0-9a-z]*(?:[:][a-z][0-9a-z]*)?$").unwrap());
        if !VALID.is_match(name) {
            return;
        }
        if name == "p" || P_PARENT_ELEMENTS.contains(&name) || P_NONPARENT_ELEMENTS.contains(&name) {
            self.end_tag("p");
        }
        if name == "dd" || name == "dt" {
            self.end_tag("dd");
            self.end_tag("dt");
        }
        if name == "li" {
            self.end_tag("li");
        }
        if name == "optgroup" {
            self.end_tag("option");
            self.end_tag("optgroup");
        }
        if name == "option" {
            self.end_tag("option");
        }
        if name == "rp" || name == "rt" {
            self.end_tag("rp");
            self.end_tag("rt");
        }
        if name == "td" || name == "th" {
            self.end_tag("td");
            self.end_tag("th");
        }
        if name == "tr" {
            self.end_tag("tr");
        }
        if name == "tbody" || name == "tfoot" {
            self.end_tag("thead");
        }
        if name == "tfoot" {
            self.end_tag("tbody");
        }
        if self.has_parent(&[name]) && !NESTABLE_ELEMENTS.contains(&name) {
            self.end_tag(name);
        }
        let void = VOID_ELEMENTS.contains(&name);
        if !void {
            self.stacked.insert(0, name.to_owned());
        }
        if !P_CHILD_ELEMENTS.contains(&name) {
            if !self.output.is_empty() {
                let kept = strip_whitespaces(&self.output, Side::End).len();
                self.output.truncate(kept);
                self.output.push('\n');
            }
            self.output.push_str(&indent(self.stacked.len().saturating_sub(1)));
        }
        if !tag.is_empty() {
            self.output.push_str(tag);
        } else {
            let atts = atts.map(format_atts).unwrap_or_default();
            match (atts.is_empty(), void) {
                (false, true) => self.output.push_str(&format!("<{name} {atts} />")),
                (false, false) => self.output.push_str(&format!("<{name} {atts}>")),
                (true, true) => self.output.push_str(&format!("<{name} />")),
                (true, false) => self.output.push_str(&format!("<{name}>")),
            }
        }
    }

    /// `end_tag()`: closes the element and the open elements inside it.
    pub fn end_tag(&mut self, tag: &str) {
        static END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</(.+?)(?:[\t\n\x0B\x0C\r ]|>)").unwrap());
        let name = match END.captures(tag) {
            Some(c) => c[1].to_lowercase(),
            None => tag.to_lowercase(),
        };
        let Some(position) = self.stacked.iter().position(|e| *e == name) else { return };
        const FAMILIES: &[(&[&str], &[&str])] = &[(&["dl"], &["dd", "dt"]), (&["ol", "ul", "menu"], &["li"]), (&["table"], &["td", "th", "tr", "thead", "tbody", "tfoot"])];
        for (ancestors, descendants) in FAMILIES {
            if descendants.contains(&name.as_str()) && self.stacked[..position].iter().any(|e| ancestors.contains(&e.as_str())) {
                return;
            }
        }
        while !self.stacked.is_empty() {
            let element = self.stacked.remove(0);
            self.append_end_tag(&element);
            if element == name {
                break;
            }
        }
    }

    fn append_end_tag(&mut self, name: &str) {
        static TRAILING_BR: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"{WS}*<br />{WS}*$")).unwrap());
        static EMPTY_P: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"<p>{WS}*</p>$")).unwrap());
        if !P_CHILD_ELEMENTS.contains(&name) {
            self.output = TRAILING_BR.replace(&self.output, "").into_owned();
            let kept = strip_whitespaces(&self.output, Side::End).len();
            self.output.truncate(kept);
            self.output.push('\n');
            self.output.push_str(&indent(self.stacked.len()));
        }
        self.output.push_str(&format!("</{name}>"));
        self.output = EMPTY_P.replace(&self.output, "").into_owned();
    }

    /// `output()`: closes what is still open and hands out the result.
    pub fn output(&mut self) -> String {
        while !self.stacked.is_empty() {
            let element = self.stacked.remove(0);
            self.append_end_tag(&element);
        }
        std::mem::take(&mut self.output)
    }
}

/// `hash( 'sha256', $text )`.
pub fn sha256_hex(text: &str) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// `wpcf7_autop( $input )`.
pub fn autop(input: &str) -> String {
    static EMBEDDED: LazyLock<fancy_regex::Regex> = LazyLock::new(|| fancy_regex::Regex::new(r"(?is)<(math|svg).*?</\1>").unwrap());
    let mut placeholders: Vec<(String, String)> = Vec::new();
    let mut replaced = String::with_capacity(input.len());
    let mut at = 0;
    while let Ok(Some(m)) = EMBEDDED.find_from_pos(input, at) {
        replaced.push_str(&input[at..m.start()]);
        let placeholder = normalize_start_tag(&format!("<{PLACEHOLDER_INLINE} id=\"{}\" />", sha256_hex(m.as_str()))).0;
        replaced.push_str(&placeholder);
        placeholders.push((placeholder, m.as_str().to_owned()));
        at = m.end();
        if m.start() == m.end() {
            break;
        }
    }
    replaced.push_str(&input[at..]);
    let mut output = HtmlFormatter::new(true).format(&replaced);
    // str_replace() with arrays replaces one key after the other.
    for (placeholder, original) in &placeholders {
        output = output.replace(placeholder.as_str(), original);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_like_php() {
        assert_eq!(php_float(50.0), "50");
        assert_eq!(php_float(1.5), "1.5");
        assert_eq!(php_float(0.1 + 0.2), "0.3");
        assert_eq!(php_float(-2.25), "-2.25");
    }

    #[test]
    fn atts_keep_their_place_and_drop_false() {
        let mut atts = Atts::default();
        atts.set("size", Att::str("40"));
        atts.set("maxlength", Att::str(" 400 "));
        atts.set("readonly", Att::Bool(false));
        atts.set("multiple", Att::Bool(true));
        atts.set("size", Att::Int(4));
        atts.set("checked", Att::str(""));
        atts.set("Bad Name", Att::str("x"));
        assert_eq!(format_atts(&atts), r#"size="4" maxlength="400" multiple="multiple""#);
    }

    #[test]
    fn hidden_fields_container() {
        let mut f = HtmlFormatter::new(true);
        let mut atts = Atts::default();
        atts.set("class", Att::str("hidden-fields-container"));
        f.append_start_tag("fieldset", Some(&atts), "");
        let mut input = Atts::default();
        input.set("type", Att::str("hidden"));
        input.set("name", Att::str("_wpcf7"));
        input.set("value", Att::Int(5));
        f.append_start_tag("input", Some(&input), "");
        assert_eq!(f.output(), "<fieldset class=\"hidden-fields-container\"><input type=\"hidden\" name=\"_wpcf7\" value=\"5\" />\n</fieldset>");
    }

    #[test]
    fn autop_paragraphs() {
        assert_eq!(autop("<label> Your name\n    [text]</label>\n\n[submit]"), "<p><label> Your name<br />\n[text]</label>\n</p>\n<p>[submit]\n</p>");
    }
}
