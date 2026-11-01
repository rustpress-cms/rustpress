//! Schema-Woven Validation (`includes/swv/`, `vendor/rocklobsterinc/swv`): the rules the modules derive from the
//! form-tags (`wpcf7_swv_create_schema`), the schema `feedback/schema` sends to the browser, and the check of a
//! submission against it.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value, json};

use super::form_tag::FormTag;
use super::posted::{PostData, Posted};

/// `WPCF7_SWV_Schema::version`.
pub const VERSION: &str = "SWV Core 2026-09-20";

/// One rule of the schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub rule: &'static str,
    pub field: String,
    pub error: String,
    pub threshold: Option<String>,
    pub accept: Option<Vec<String>>,
}

impl Rule {
    fn new(rule: &'static str, field: &str, error: String) -> Self {
        Self { rule, field: field.to_owned(), error, threshold: None, accept: None }
    }

    fn threshold(mut self, threshold: impl Into<String>) -> Self {
        self.threshold = Some(threshold.into());
        self
    }

    fn accept(mut self, accept: Vec<String>) -> Self {
        self.accept = Some(accept);
        self
    }

    /// `toArray()`.
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("rule".into(), Value::from(self.rule));
        map.insert("field".into(), Value::from(self.field.clone()));
        map.insert("error".into(), Value::from(self.error.clone()));
        if let Some(threshold) = &self.threshold {
            map.insert("threshold".into(), Value::from(threshold.clone()));
        }
        if let Some(accept) = &self.accept {
            map.insert("accept".into(), Value::from(accept.clone()));
        }
        Value::Object(map)
    }
}

/// `$schema->to_array()`.
pub fn schema_json(locale: &str, rules: &[Rule]) -> Value {
    json!({"version": VERSION, "locale": locale, "rules": rules.iter().map(Rule::to_json).collect::<Vec<_>>()})
}

/// The texts the rules need: `wpcf7_get_message( $status )` of the form, and the two errors the modules write
/// themselves (`filter_message( __( ... ) )`).
pub struct Messages<'a> {
    pub message: &'a (dyn Fn(&str) -> String + Sync),
    pub undefined_value: String,
    pub too_many_items: String,
}

/// `wpcf7_is_number()`.
pub fn is_number(text: &str) -> bool {
    static INT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-]?[0-9]+(?:[eE][+-]?[0-9]+)?$").unwrap());
    static FRACTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[-]?(?:[0-9]+)?[.][0-9]+(?:[eE][+-]?[0-9]+)?$").unwrap());
    INT.is_match(text) || FRACTION.is_match(text)
}

/// `absint()` of an option.
fn absint(text: &str) -> String {
    let digits: String = text.trim_start().trim_start_matches(['-', '+']).chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u64>().unwrap_or(0).to_string()
}

/// The rules of the modules, in the order their callbacks run: priority 10 in the order the modules are loaded
/// (checkbox, date, file, number, select, text, textarea), then the enumerations at priority 20.
pub fn schema(tags: &[FormTag], m: &Messages<'_>, today: chrono::NaiveDate) -> Vec<Rule> {
    let msg = |status: &str| (m.message)(status);
    let of = |basetypes: &[&str]| tags.iter().filter(move |t| basetypes.contains(&t.basetype.as_str())).collect::<Vec<_>>();
    let mut rules = Vec::new();
    // checkbox.php
    for tag in of(&["checkbox", "radio"]) {
        if tag.is_required() || tag.tag_type == "radio" {
            rules.push(Rule::new("required", &tag.name, msg("invalid_required")));
        }
        if tag.tag_type == "radio" || tag.has_option("exclusive") {
            rules.push(Rule::new("maxitems", &tag.name, m.too_many_items.clone()).threshold("1"));
        }
    }
    // date.php
    for tag in of(&["date"]) {
        if tag.is_required() {
            rules.push(Rule::new("required", &tag.name, msg("invalid_required")));
        }
        rules.push(Rule::new("date", &tag.name, msg("invalid_date")));
        let date_option = |name: &str| tag.get_option(name, "").filter(|v| !v.is_empty() && v != "0").and_then(|v| super::controls::parse_date(&v.replace('_', " "), today));
        if let Some(min) = date_option("min") {
            rules.push(Rule::new("mindate", &tag.name, msg("date_too_early")).threshold(min));
        }
        if let Some(max) = date_option("max") {
            rules.push(Rule::new("maxdate", &tag.name, msg("date_too_late")).threshold(max));
        }
    }
    // file.php
    for tag in of(&["file"]) {
        if tag.is_required() {
            rules.push(Rule::new("requiredfile", &tag.name, msg("invalid_required")));
        }
        let accept = acceptable_filetypes_attr(&tag.get_options("filetypes", "")).split(',').map(str::to_owned).collect();
        rules.push(Rule::new("file", &tag.name, msg("upload_file_type_invalid")).accept(accept));
        rules.push(Rule::new("maxfilesize", &tag.name, msg("upload_file_too_large")).threshold(tag.get_limit_option().to_string()));
    }
    // number.php
    for tag in of(&["number", "range"]) {
        if tag.is_required() {
            rules.push(Rule::new("required", &tag.name, msg("invalid_required")));
        }
        rules.push(Rule::new("number", &tag.name, msg("invalid_number")));
        let mut min = tag.get_option("min", "signed_num");
        let mut max = tag.get_option("max", "signed_num");
        if tag.basetype == "range" {
            if !min.as_deref().is_some_and(is_number) {
                min = Some("0".to_owned());
            }
            if !max.as_deref().is_some_and(is_number) {
                max = Some("100".to_owned());
            }
        }
        if let Some(min) = min.filter(|v| is_number(v)) {
            rules.push(Rule::new("minnumber", &tag.name, msg("number_too_small")).threshold(min));
        }
        if let Some(max) = max.filter(|v| is_number(v)) {
            rules.push(Rule::new("maxnumber", &tag.name, msg("number_too_large")).threshold(max));
        }
    }
    // select.php
    for tag in tags.iter().filter(|t| t.tag_type == "select*") {
        rules.push(Rule::new("required", &tag.name, msg("invalid_required")));
    }
    // text.php and textarea.php
    for (basetypes, default_max) in [(&["text", "email", "url", "tel"][..], "400"), (&["textarea"][..], "2000")] {
        for tag in of(basetypes) {
            if tag.is_required() {
                rules.push(Rule::new("required", &tag.name, msg("invalid_required")));
            }
            match tag.basetype.as_str() {
                "email" => rules.push(Rule::new("email", &tag.name, msg("invalid_email"))),
                "url" => rules.push(Rule::new("url", &tag.name, msg("invalid_url"))),
                "tel" => rules.push(Rule::new("tel", &tag.name, msg("invalid_tel"))),
                _ => {}
            }
            if let Some(min) = tag.get_minlength_option() {
                rules.push(Rule::new("minlength", &tag.name, msg("invalid_too_short")).threshold(absint(&min)));
            }
            let max = tag.get_maxlength_option().unwrap_or_else(|| default_max.to_owned());
            if super::form_tag::truthy(&max) {
                rules.push(Rule::new("maxlength", &tag.name, msg("invalid_too_long")).threshold(absint(&max)));
            }
        }
    }
    // The enumerations of checkbox.php (a field with `free_text` takes anything) and select.php.
    let mut checkbox: Vec<(String, Option<Vec<String>>)> = Vec::new();
    for tag in of(&["checkbox", "radio"]) {
        let index = match checkbox.iter().position(|(name, _)| *name == tag.name) {
            Some(index) => index,
            None => {
                checkbox.push((tag.name.clone(), Some(Vec::new())));
                checkbox.len() - 1
            }
        };
        if tag.has_option("free_text") {
            checkbox[index].1 = None;
        }
        if let Some(values) = checkbox[index].1.as_mut() {
            values.extend(tag.values.iter().cloned());
        }
    }
    for (field, values) in checkbox {
        if let Some(values) = values {
            rules.push(Rule::new("enum", &field, m.undefined_value.clone()).accept(enum_values(values)));
        }
    }
    let mut select: Vec<(String, Vec<String>)> = Vec::new();
    for tag in of(&["select"]) {
        let mut values = tag.values.clone();
        if tag.has_option("first_as_label") && !values.is_empty() {
            values.remove(0);
        }
        match select.iter_mut().find(|(name, _)| *name == tag.name) {
            Some((_, existing)) => existing.extend(values),
            None => select.push((tag.name.clone(), values)),
        }
    }
    for (field, values) in select {
        rules.push(Rule::new("enum", &field, m.undefined_value.clone()).accept(enum_values(values)));
    }
    rules
}

/// The values of an enumeration: decoded, unique, without the empty one.
fn enum_values(values: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for value in values {
        let value = rustpress_core::formatting::html_entity_decode(&value);
        if !value.is_empty() && !out.contains(&value) {
            out.push(value);
        }
    }
    out
}

/// `wpcf7_acceptable_filetypes( $types, 'attr' )`.
pub fn acceptable_filetypes_attr(options: &[String]) -> String {
    static SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s|,]+").unwrap());
    static WILDCARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^([a-z]+)/[*]$").unwrap());
    let mut types: Vec<String> = Vec::new();
    for option in options {
        for t in SPLIT.split(&option.to_lowercase()) {
            if !t.is_empty() && !types.iter().any(|seen| seen == t) {
                types.push(t.to_owned());
            }
        }
    }
    if types.is_empty() {
        types = ["audio/*", "video/*", "image/*"].map(String::from).to_vec();
    }
    let out: Vec<String> = types
        .iter()
        .filter_map(|t| {
            if !t.contains('/') {
                Some(format!(".{}", t.trim_matches('.')))
            } else if let Some(c) = WILDCARD.captures(t) {
                ["audio", "video", "image"].contains(&&c[1]).then(|| t.clone())
            } else {
                (!mime_extensions(t).is_empty()).then(|| t.clone())
            }
        })
        .filter(|t| !t.is_empty())
        .collect();
    out.join(",")
}

/// `FileRule::MIME_TYPES`.
const MIME_TYPES: &[(&str, &[&str])] = &[
    ("application/java", &["class"]),
    ("application/javascript", &["js"]),
    ("application/msword", &["doc"]),
    ("application/octet-stream", &["psd", "xcf"]),
    ("application/onenote", &["onetoc", "onetoc2", "onetmp", "onepkg"]),
    ("application/oxps", &["oxps"]),
    ("application/pdf", &["pdf"]),
    ("application/rar", &["rar"]),
    ("application/rtf", &["rtf"]),
    ("application/ttaf+xml", &["dfxp"]),
    ("application/vnd.apple.keynote", &["key"]),
    ("application/vnd.apple.numbers", &["numbers"]),
    ("application/vnd.apple.pages", &["pages"]),
    ("application/vnd.ms-access", &["mdb"]),
    ("application/vnd.ms-excel", &["xla", "xls", "xlt", "xlw"]),
    ("application/vnd.ms-excel.addin.macroEnabled.12", &["xlam"]),
    ("application/vnd.ms-excel.sheet.binary.macroEnabled.12", &["xlsb"]),
    ("application/vnd.ms-excel.sheet.macroEnabled.12", &["xlsm"]),
    ("application/vnd.ms-excel.template.macroEnabled.12", &["xltm"]),
    ("application/vnd.ms-powerpoint", &["pot", "pps", "ppt"]),
    ("application/vnd.ms-powerpoint.addin.macroEnabled.12", &["ppam"]),
    ("application/vnd.ms-powerpoint.presentation.macroEnabled.12", &["pptm"]),
    ("application/vnd.ms-powerpoint.slide.macroEnabled.12", &["sldm"]),
    ("application/vnd.ms-powerpoint.slideshow.macroEnabled.12", &["ppsm"]),
    ("application/vnd.ms-powerpoint.template.macroEnabled.12", &["potm"]),
    ("application/vnd.ms-project", &["mpp"]),
    ("application/vnd.ms-word.document.macroEnabled.12", &["docm"]),
    ("application/vnd.ms-word.template.macroEnabled.12", &["dotm"]),
    ("application/vnd.ms-write", &["wri"]),
    ("application/vnd.ms-xpsdocument", &["xps"]),
    ("application/vnd.oasis.opendocument.chart", &["odc"]),
    ("application/vnd.oasis.opendocument.database", &["odb"]),
    ("application/vnd.oasis.opendocument.formula", &["odf"]),
    ("application/vnd.oasis.opendocument.graphics", &["odg"]),
    ("application/vnd.oasis.opendocument.presentation", &["odp"]),
    ("application/vnd.oasis.opendocument.spreadsheet", &["ods"]),
    ("application/vnd.oasis.opendocument.text", &["odt"]),
    ("application/vnd.openxmlformats-officedocument.presentationml.presentation", &["pptx"]),
    ("application/vnd.openxmlformats-officedocument.presentationml.slide", &["sldx"]),
    ("application/vnd.openxmlformats-officedocument.presentationml.slideshow", &["ppsx"]),
    ("application/vnd.openxmlformats-officedocument.presentationml.template", &["potx"]),
    ("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", &["xlsx"]),
    ("application/vnd.openxmlformats-officedocument.spreadsheetml.template", &["xltx"]),
    ("application/vnd.openxmlformats-officedocument.wordprocessingml.document", &["docx"]),
    ("application/vnd.openxmlformats-officedocument.wordprocessingml.template", &["dotx"]),
    ("application/wordperfect", &["wp", "wpd"]),
    ("application/x-7z-compressed", &["7z"]),
    ("application/x-gzip", &["gz", "gzip"]),
    ("application/x-msdownload", &["exe"]),
    ("application/x-shockwave-flash", &["swf"]),
    ("application/x-tar", &["tar"]),
    ("application/zip", &["zip"]),
    ("audio/3gpp", &["3gp", "3gpp"]),
    ("audio/3gpp2", &["3g2", "3gp2"]),
    ("audio/aac", &["aac"]),
    ("audio/flac", &["flac"]),
    ("audio/midi", &["mid", "midi"]),
    ("audio/mpeg", &["mp3", "m4a", "m4b"]),
    ("audio/ogg", &["ogg", "oga"]),
    ("audio/wav", &["wav", "x-wav"]),
    ("audio/x-matroska", &["mka"]),
    ("audio/x-ms-wax", &["wax"]),
    ("audio/x-ms-wma", &["wma"]),
    ("audio/x-realaudio", &["ra", "ram"]),
    ("image/avif", &["avif"]),
    ("image/bmp", &["bmp"]),
    ("image/gif", &["gif"]),
    ("image/heic", &["heic"]),
    ("image/heic-sequence", &["heic", "heics"]),
    ("image/heif", &["heic", "heif"]),
    ("image/heif-sequence", &["heic", "heifs"]),
    ("image/jpeg", &["jpg", "jpeg", "jpe"]),
    ("image/png", &["png"]),
    ("image/tiff", &["tiff", "tif"]),
    ("image/webp", &["webp"]),
    ("image/x-icon", &["ico"]),
    ("text/calendar", &["ics"]),
    ("text/css", &["css"]),
    ("text/csv", &["csv"]),
    ("text/html", &["htm", "html"]),
    ("text/plain", &["txt", "asc", "c", "cc", "h", "srt"]),
    ("text/richtext", &["rtx"]),
    ("text/tab-separated-values", &["tsv"]),
    ("text/vtt", &["vtt"]),
    ("video/3gpp", &["3gp", "3gpp"]),
    ("video/3gpp2", &["3g2", "3gp2"]),
    ("video/avi", &["avi"]),
    ("video/divx", &["divx"]),
    ("video/mp4", &["mp4", "m4v"]),
    ("video/mpeg", &["mpeg", "mpg", "mpe"]),
    ("video/ogg", &["ogv"]),
    ("video/quicktime", &["mov", "qt"]),
    ("video/webm", &["webm"]),
    ("video/x-flv", &["flv"]),
    ("video/x-matroska", &["mkv"]),
    ("video/x-ms-asf", &["asf", "asx"]),
    ("video/x-ms-wm", &["wm"]),
    ("video/x-ms-wmv", &["wmv"]),
    ("video/x-ms-wmx", &["wmx"]),
];

/// `FileRule::convertMimeToExt()`.
pub fn mime_extensions(mime: &str) -> Vec<&'static str> {
    static MIME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^([a-z]+)/([*]|[a-z0-9.+-]+)$").unwrap());
    let Some(c) = MIME.captures(mime) else { return Vec::new() };
    let mut out: Vec<&'static str> = Vec::new();
    for (key, extensions) in MIME_TYPES {
        let hit = if &c[2] == "*" { key.starts_with(&format!("{}/", &c[1])) } else { *key == mime };
        if hit {
            for ext in *extensions {
                if !out.contains(ext) {
                    out.push(ext);
                }
            }
        }
    }
    out
}

/// A file of the submission as `FormDataTree` sees it (an empty one is no file).
#[derive(Debug, Clone)]
pub struct SubmittedFile {
    pub field: String,
    pub name: String,
    pub size: u64,
}

/// `RockLobsterInc\Functions\strip_whitespaces()`.
fn strip(text: &str) -> &str {
    let ws = |c: char| matches!(c, '\u{09}'..='\u{0D}' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}');
    text.trim_matches(ws)
}

/// `dissolve_name()`: the name and its layers (`a[b][c]`).
fn dissolve_name(name: &str) -> Vec<String> {
    static NAME: LazyLock<Regex> = LazyLock::new(|| {
        let s = r"[\x09-\x0D\x20\xA0\x{1680}\x{2000}-\x{200A}\x{2028}\x{2029}\x{202F}\x{205F}\x{3000}\x{FEFF}]*";
        Regex::new(&format!(r"(?i)^([a-z][0-9a-z:_-]*)((?:\[{s}(?:[a-z][0-9a-z:_-]*|[0-9]*){s}\])*)$")).unwrap()
    });
    static LAYER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(.*?)\]").unwrap());
    let name = strip(name);
    let Some(c) = NAME.captures(name) else { return Vec::new() };
    let mut parts = vec![c[1].to_owned()];
    parts.extend(LAYER.captures_iter(&c[2]).map(|l| strip(&l[1]).to_owned()));
    parts
}

/// What the submission sent: `$_POST` and the files.
pub struct FormData<'a> {
    pub post: &'a PostData,
    pub files: &'a [SubmittedFile],
}

impl FormData<'_> {
    /// `getAll( $name )`, flattened: the values without blanks, trimmed.
    pub fn values(&self, name: &str) -> Vec<String> {
        let parts = dissolve_name(name);
        let Some((first, layers)) = parts.split_first() else { return Vec::new() };
        let Some(mut value) = self.post.get(first) else { return Vec::new() };
        // The walk ends at the first empty or "0" layer; a numeric layer finds nothing.
        for layer in layers {
            if layer.is_empty() || layer == "0" {
                break;
            }
            if layer.bytes().all(|b| b.is_ascii_digit()) {
                return Vec::new();
            }
            match value {
                Posted::Map(items) => match items.iter().find(|(k, _)| k == layer) {
                    Some((_, v)) => value = v,
                    None => return Vec::new(),
                },
                Posted::Str(_) => return Vec::new(),
            }
        }
        let mut out = Vec::new();
        collect(value, &mut out);
        out
    }

    /// `getAllFiles( $name )`, flattened.
    pub fn files(&self, name: &str) -> Vec<&SubmittedFile> {
        let parts = dissolve_name(name);
        let Some(first) = parts.first() else { return Vec::new() };
        self.files.iter().filter(|f| f.field == *first && f.size != 0).collect()
    }
}

fn collect(value: &Posted, out: &mut Vec<String>) {
    match value {
        Posted::Str(s) => {
            let s = strip(s);
            if !s.is_empty() {
                out.push(s.to_owned());
            }
        }
        Posted::Map(items) => items.iter().for_each(|(_, v)| collect(v, out)),
    }
}

/// `count_code_units()`: the length in UTF-16 code units, with the line breaks as one character.
fn code_units(text: &str) -> usize {
    text.replace("\r\n", "\n").replace('\r', "\n").encode_utf16().count()
}

/// `is_numeric()` for the thresholds the modules write.
fn numeric(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok()
}

/// `(float) $value` of a number `is_number()` accepted.
fn float(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(0.0)
}

/// `EmailRule::isEmail()`.
pub fn is_email(value: &str) -> bool {
    static LOCAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9!#$%&'*+/=?^_`{|}~.-]+$").unwrap());
    static SUB: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z0-9-]+$").unwrap());
    if value.len() < 6 || !value[1..].contains('@') {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else { return false };
    if !LOCAL.is_match(local) || domain.contains("..") {
        return false;
    }
    if domain.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B', '.']) != domain {
        return false;
    }
    let subs: Vec<&str> = domain.split('.').collect();
    subs.len() >= 2 && subs.iter().all(|sub| sub.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0B', '-']) == *sub && SUB.is_match(sub))
}

/// `UrlRule::isUrl()`: `parse_url()` finds the scheme `http` or `https`.
fn is_url(value: &str) -> bool {
    static SCHEME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([a-zA-Z][a-zA-Z0-9+.-]*):").unwrap());
    let Some(c) = SCHEME.captures(value) else { return false };
    let rest = &value[c[0].len()..];
    // `host:port` has no scheme for parse_url().
    let port = rest.split('/').next().unwrap_or_default();
    if !rest.starts_with("//") && !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    matches!(&c[1], "http" | "https")
}

/// `TelRule::isTel()`.
fn is_tel(value: &str) -> bool {
    static EXTENSION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[#*].*$").unwrap());
    static SEPARATORS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[()/.*#\s-]+").unwrap());
    static LEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[+0]+").unwrap());
    static DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[+]?[0-9]+$").unwrap());
    let value = EXTENSION.replace(value, "");
    let mut value = SEPARATORS.replace_all(&value, "").into_owned();
    if value.starts_with('+') || value.starts_with("00") {
        value = format!("+{}", LEADING.replace(&value, ""));
    }
    DIGITS.is_match(&value) && value.len() > 5 && value.len() < 16
}

/// `DateRule::isDate()`.
fn is_date(value: &str) -> bool {
    static DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]{4})-([0-9]{2})-([0-9]{2})$").unwrap());
    DATE.captures(value).is_some_and(|c| chrono::NaiveDate::from_ymd_opt(c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0), c[3].parse().unwrap_or(0)).is_some())
}

/// The extensions a `file` rule accepts.
fn acceptable_suffixes(accept: &[String]) -> Vec<String> {
    static EXT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\.[a-z0-9]+$").unwrap());
    let mut out: Vec<String> = Vec::new();
    for item in accept {
        let found: Vec<String> = if EXT.is_match(item) { vec![item.to_lowercase()] } else { mime_extensions(item).iter().map(|e| format!(".{}", e.trim_matches([' ', '.'])).to_lowercase()).collect() };
        for suffix in found {
            if !out.contains(&suffix) {
                out.push(suffix);
            }
        }
    }
    out
}

impl Rule {
    /// `validate()`: whether the submission passes.
    fn passes(&self, data: &FormData<'_>) -> bool {
        let threshold = self.threshold.as_deref().unwrap_or_default();
        match self.rule {
            "requiredfile" => !data.files(&self.field).is_empty(),
            "file" => {
                let files = data.files(&self.field);
                let suffixes = acceptable_suffixes(self.accept.as_deref().unwrap_or_default());
                files.iter().all(|f| f.name.rfind('.').is_some_and(|at| suffixes.contains(&f.name[at..].to_lowercase())))
            }
            "maxfilesize" | "minfilesize" => {
                let files = data.files(&self.field);
                if files.is_empty() {
                    return true;
                }
                let size: u64 = files.iter().map(|f| f.size).sum();
                let limit = threshold.trim().parse::<i64>().unwrap_or(0);
                if self.rule == "maxfilesize" { size as i64 <= limit } else { size as i64 >= limit }
            }
            _ => {
                let values = data.values(&self.field);
                if self.rule == "required" {
                    return !values.is_empty();
                }
                if values.is_empty() {
                    return true;
                }
                match self.rule {
                    "email" => values.iter().all(|v| is_email(v)),
                    "url" => values.iter().all(|v| is_url(v)),
                    "tel" => values.iter().all(|v| is_tel(v)),
                    "number" => values.iter().all(|v| is_number(v)),
                    "date" => values.iter().all(|v| is_date(v)),
                    "minlength" | "maxlength" => match numeric(threshold) {
                        Some(limit) => {
                            let total = values.iter().map(|v| code_units(v)).sum::<usize>() as f64;
                            let limit = limit.trunc();
                            if self.rule == "maxlength" { total <= limit } else { total >= limit }
                        }
                        None => true,
                    },
                    "minnumber" | "maxnumber" => match numeric(threshold) {
                        Some(limit) => values.iter().filter(|v| is_number(v)).all(|v| if self.rule == "maxnumber" { float(v) <= limit } else { float(v) >= limit }),
                        None => true,
                    },
                    "mindate" | "maxdate" => {
                        if !is_date(threshold) {
                            return true;
                        }
                        values.iter().filter(|v| is_date(v)).all(|v| if self.rule == "maxdate" { v.as_str() <= threshold } else { v.as_str() >= threshold })
                    }
                    "minitems" | "maxitems" => match numeric(threshold) {
                        Some(limit) => {
                            let count = values.len() as f64;
                            if self.rule == "maxitems" { count <= limit.trunc() } else { count >= limit.trunc() }
                        }
                        None => true,
                    },
                    "enum" => {
                        let accept: Vec<String> = self.accept.as_deref().unwrap_or_default().iter().map(|a| a.replace("\r\n", "\n").replace('\r', "\n")).collect();
                        values.iter().all(|v| accept.contains(&v.replace("\r\n", "\n").replace('\r', "\n")))
                    }
                    _ => true,
                }
            }
        }
    }
}

/// `WPCF7_SWV_Schema::validate()`: the field and error of every rule the submission breaks, in the order of the
/// schema.
pub fn validate(rules: &[Rule], data: &FormData<'_>) -> Vec<(String, String)> {
    rules.iter().filter(|rule| !rule.passes(data)).map(|rule| (rule.field.clone(), rule.error.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checks_of_the_rules() {
        assert!(is_email("jane@example.org"));
        assert!(!is_email("bad"));
        assert!(!is_email("jane@example..org"));
        assert!(!is_email("jane@-example.org"));
        assert!(is_url("https://example.org"));
        assert!(!is_url("example.org"));
        assert!(!is_url("ftp://example.org"));
        assert!(!is_url("http:8080"));
        assert!(is_tel("+49 (0) 30 1234-567"));
        assert!(!is_tel("12345"));
        assert!(is_number("-1.5e3") && is_number(".5") && !is_number("1."));
        assert!(is_date("2026-02-28") && !is_date("2026-02-30"));
        assert_eq!(code_units("a\r\nb😀"), 5);
    }

    #[test]
    fn filetypes() {
        assert_eq!(acceptable_filetypes_attr(&["pdf|txt".to_owned()]), ".pdf,.txt");
        assert_eq!(acceptable_filetypes_attr(&[]), "audio/*,video/*,image/*");
        assert_eq!(acceptable_filetypes_attr(&["application/pdf".to_owned(), "text/*".to_owned()]), "application/pdf");
        assert!(acceptable_suffixes(&["image/*".to_owned()]).contains(&".jpeg".to_owned()));
    }
}
