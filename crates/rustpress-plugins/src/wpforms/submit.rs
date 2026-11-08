//! A submission through `admin-ajax.php` (`wp_ajax_wpforms_submit`): `WPForms_Process::ajax_submit()` with
//! `listen()` and `process()`, the validation and formatting of the Lite fields (`validate()` and `format()` of
//! `includes/fields/class-*.php`), the anti-spam checks (`AntiSpam`, `Honeypot`, `Token`, the time limit), the
//! errors of the answer (`ajax_process_errors()`) and the confirmation (`entry_confirmation_redirect()`,
//! `Frontend::confirmation()`). The notification e-mails are made in [`super::notify`].

use rustpress_content::sanitize::text_field;
use rustpress_core::formatting::{esc_html, esc_url_raw, wpautop};
use serde_json::{Map, Value, json};

use super::php::{self, empty, set, str_of, string};
use super::settings::Settings;
use super::smart_tags::Tags;

/// The signed-in user the smart tags print.
#[derive(Debug, Clone, Default)]
pub struct User {
    pub id: u64,
    pub display_name: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
}

/// What the submission sees of the site and the request.
pub struct Request<'a> {
    pub settings: &'a Settings,
    /// `__( $text, 'wpforms-lite' )` and `_n()`.
    pub tr: &'a (dyn Fn(&str) -> String + Sync),
    pub trn: &'a (dyn Fn(&str, &str, i64) -> String + Sync),
    /// `$_POST` and `$_GET` as PHP builds them.
    pub post: &'a Value,
    pub get: &'a Value,
    /// The signed-in user, if any; `current_user_can( 'manage_options' )`, `is_super_admin()`.
    pub user: Option<User>,
    pub can_manage: bool,
    pub super_admin: bool,
    /// `wp_verify_nonce( $nonce, $action )`.
    pub verify_nonce: &'a (dyn Fn(&str, &str) -> bool + Sync),
    pub now: i64,
    pub remote_addr: &'a str,
    /// The scheme and host of the home URL, and `$_SERVER['REQUEST_URI']` (`wpforms_current_url()`).
    pub origin: String,
    pub request_uri: String,
    pub admin_email: String,
    pub blogname: String,
    pub home_url: String,
    pub home_host: String,
    /// `wp_login_url()`.
    pub login_url: String,
    pub charset: String,
    /// `get_language_attributes()`.
    pub language_attributes: String,
    pub rtl: bool,
    /// `date_i18n( $format, $timestamp_with_offset )`.
    pub date: &'a (dyn Fn(&str, i64) -> String + Sync),
    pub gmt_offset: f64,
    pub date_format: String,
    /// `Crypto::get_secret_key()`.
    pub secret: &'a [u8],
    /// `get_permalink()` of the pages the confirmations name.
    pub permalinks: Vec<(i64, String)>,
    /// The registered shortcodes (`strip_shortcodes()`).
    pub shortcodes: &'a [&'a str],
    /// The author of `$_POST['page_id']` (ID and display name; 0 and `''` when the page has none), `None` without a
    /// page ID.
    pub page_author: Option<(u64, String)>,
    /// The e-mail addresses of the users who can `manage_options` (`SmartTag::recipient_has_cap()`), lowercase.
    pub manager_emails: Vec<String>,
    /// `WPFORMS_PLUGIN_DIR`, for the e-mail templates.
    pub plugin_dir: Option<std::path::PathBuf>,
}

/// What a submission leaves: the answer of `admin-ajax.php`, the e-mails to send and whether the entry was saved
/// (Lite counts it).
#[derive(Debug, Default)]
pub struct Outcome {
    pub body: String,
    pub emails: Vec<rustpress_core::native::Email>,
    /// `update_entry_count()` counts the entry (it was saved and is no spam).
    pub saved: bool,
    /// `wpforms_process_entry_saved` and `wpforms_process_complete` ran.
    pub complete: bool,
}

/// `$_POST` as PHP builds it from the pairs of the request.
pub fn post_tree(pairs: &[(String, String)]) -> Value {
    use crate::redirection::php::{Param, Params};
    fn to_value(params: &Params) -> Value {
        Value::Object(
            params
                .iter()
                .map(|(k, v)| {
                    let v = match v {
                        Param::Null => Value::String(String::new()),
                        Param::Str(s) => Value::String(s.clone()),
                        Param::Arr(a) => to_value(a),
                    };
                    (k.clone(), v)
                })
                .collect(),
        )
    }
    let mut params = Params::new();
    for (name, value) in pairs {
        crate::redirection::php::register(&mut params, name, value.clone());
    }
    to_value(&params)
}

/// `is_array()` of a value of the request or of the form data.
fn is_arr(v: &Value) -> bool {
    v.is_object() || v.is_array()
}

/// The values of a PHP array.
fn values(v: &Value) -> Vec<Value> {
    match v {
        Value::Object(m) => m.values().cloned().collect(),
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    }
}

/// `$array[ $key ]` of an object or a list.
fn at<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => m.get(key),
        Value::Array(a) => key.parse::<usize>().ok().and_then(|i| a.get(i)),
        _ => None,
    }
}

/// The key/value pairs of a PHP array.
fn pairs(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Value::Array(a) => a.iter().enumerate().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        _ => Vec::new(),
    }
}

/// `(string) $value` of a submitted value.
fn s(v: &Value) -> String {
    string(v)
}

/// `$value === ''`.
fn is_empty_string(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::String(s)) if s.is_empty())
}

/// `trim()` with PHP's characters.
fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}'])
}

/// `wpforms_sanitize_textarea_field()`: `_sanitize_text_fields( $string, true )` until nothing changes.
pub fn sanitize_textarea(v: &Value) -> String {
    if is_arr(v) {
        return String::new();
    }
    let mut value = s(v);
    loop {
        let next = rustpress_content::sanitize::textarea_field(&value);
        if next.len() == value.len() {
            return next;
        }
        value = next;
    }
}

/// `sanitize_text_field()` of a value (an array is `''`).
fn stf(v: &Value) -> String {
    if is_arr(v) { String::new() } else { text_field(&s(v)) }
}

/// `wpforms_sanitize_array_combine()` of a list: `None` for an empty array.
fn combine(items: &[Value]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    Some(items.iter().map(stf).collect::<Vec<_>>().join("\n"))
}

/// `wpforms_decode_string()`.
pub fn decode_string(text: &str) -> String {
    let deep = |t: &str| sanitize_textarea(&Value::String(t.to_owned()));
    let text = deep(text);
    let decoded = super::smart_tags::decode_quotes(&rustpress_core::formatting::html_entity_decode(&text));
    deep(&rustpress_core::kses::decode_entities(&decoded))
}

// ---------------------------------------------------------------------------------------------------------------
// E-mail addresses

/// `wpforms_is_email( $email, false )`.
pub fn is_email(email: &str) -> bool {
    if email.len() > 254 {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    let [local, domain] = parts[..] else { return false };
    if local.len() > 63 {
        return false;
    }
    for label in domain.split('.') {
        let label = trim(label);
        if label.is_empty() || label == "0" || label.len() > 63 {
            return false;
        }
    }
    let Some(encoded) = super::punycode::encode(domain) else { return false };
    rustpress_content::sanitize::is_email(&format!("{local}@{encoded}"))
}

/// `wpforms_is_email( $email )`: a smart tag passes.
fn is_email_or_tag(email: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\{.+?\}").expect("valid regex")).is_match(email) || is_email(email)
}

/// `WPForms_Field_Email::transform_punycode()`: every part between `*`, `@` and `.` through the library.
fn transform_punycode(pattern: &str, f: fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut start = 0;
    let bytes = pattern.as_bytes();
    let mut i = 0;
    loop {
        if i == bytes.len() || matches!(bytes[i], b'*' | b'@' | b'.') {
            let part = &pattern[start..i];
            if start > 0 {
                out.push(bytes[start - 1] as char);
            }
            if part.is_empty() || part == "0" {
                out.push_str(part);
            } else {
                match f(part) {
                    Some(p) => out.push_str(&p),
                    None => return String::new(),
                }
            }
            if i == bytes.len() {
                break;
            }
            start = i + 1;
        }
        i += 1;
    }
    out
}

fn encode_punycode(pattern: &str) -> String {
    transform_punycode(pattern, super::punycode::encode)
}

fn decode_punycode(pattern: &str) -> String {
    transform_punycode(pattern, super::punycode::decode)
}

/// `WPForms_Field_Email::email_encode_punycode()`.
fn email_encode_punycode(email: &Value) -> String {
    if is_arr(email) || !is_email(&s(email)) {
        return String::new();
    }
    let encoded = encode_punycode(&s(email));
    if decode_punycode(&encoded).is_empty() { String::new() } else { encoded }
}

/// `sanitize_part_pattern()`: the characters outside the class go, smart tags stay.
fn sanitize_part_pattern(part: &str, class: &regex::Regex) -> String {
    static TAG: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let tag = TAG.get_or_init(|| regex::Regex::new(r"\{.+?\}").expect("valid regex"));
    let tags: Vec<String> = tag.find_iter(part).map(|m| m.as_str().to_owned()).collect();
    let mut part = part.to_owned();
    for t in &tags {
        part = part.replacen(t.as_str(), "-wpforms-smart-tag-", 1);
    }
    let mut part = class.replace_all(&part, "").into_owned();
    for t in &tags {
        part = part.replacen("-wpforms-smart-tag-", t, 1);
    }
    part
}

/// `get_pattern()`: `implode( '@', array_filter( [ $local, $domain ] ) )`.
fn get_pattern(local: &str, domain: &str) -> String {
    [local, domain].into_iter().filter(|p| !p.is_empty() && *p != "0").collect::<Vec<_>>().join("@")
}

/// `WPForms_Field_Email::is_email_pattern()`.
fn is_email_pattern(pattern: &str) -> Option<String> {
    static LOCAL: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    static DOMAIN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let local_re = LOCAL.get_or_init(|| regex::Regex::new(r"[^a-zA-Z0-9\x{0080}-\x{0FFF}!#$%&'*+/=?^_`{|}~.\-]").expect("valid regex"));
    let domain_re = DOMAIN.get_or_init(|| regex::Regex::new(r"[^a-z0-9\x{0080}-\x{FFFF}\-.*]").expect("valid regex"));
    if pattern.is_empty() || pattern == "0" {
        return None;
    }
    let mut split = pattern.split('@');
    let local = sanitize_part_pattern(split.next().unwrap_or_default(), local_re);
    let domain = sanitize_part_pattern(split.next().unwrap_or_default(), domain_re);
    if !pattern.contains('@') {
        if !local.contains('*') {
            return None;
        }
        if is_email_or_tag(&get_pattern(&local, "a.me")) || is_email_or_tag(&get_pattern("a", &local.replace('*', ""))) {
            return Some(get_pattern(&local, ""));
        }
        return None;
    }
    // `maybe_adjust_domain()`.
    let check = domain.replace('*', "");
    let mut subs: Vec<String> = check.split('.').map(str::to_owned).collect();
    while subs.len() < 2 {
        subs.push(String::new());
    }
    for (i, template) in ["a", "me"].iter().enumerate() {
        let t = trim(&subs[i]).to_owned();
        subs[i] = if t.is_empty() || t == "0" { (*template).to_owned() } else { t };
    }
    let check = subs.join(".");
    is_email_or_tag(&get_pattern(&local, &check)).then(|| get_pattern(&local, &domain))
}

/// `WPForms_Field_Email::is_restricted_email()`: whether the address may be used.
fn email_allowed(email: &str, field: &Value) -> bool {
    let filter = str_of(field, "filter_type");
    if !set(field, "filter_type") || !set(field, &filter) {
        return true;
    }
    let email = trim(email).to_lowercase();
    if !is_email_or_tag(&email) {
        return false;
    }
    let domain = email.split('@').nth(1).unwrap_or_default();
    let email = if domain.starts_with("xn--") { decode_punycode(&email) } else { email };
    if email.is_empty() {
        return false;
    }
    static SPLIT: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let split = SPLIT.get_or_init(|| regex::Regex::new(r"\r\n|\r|\n|,").expect("valid regex"));
    let mut patterns: Vec<String> = Vec::new();
    for rule in split.split(&str_of(field, &filter)) {
        if rule.is_empty() || rule == "0" {
            continue;
        }
        let rule = trim(rule).to_lowercase();
        if let Some(p) = is_email_pattern(&rule) {
            let p = encode_punycode(&p);
            if !patterns.contains(&p) {
                patterns.push(p);
            }
        }
    }
    let allowlist = filter == "allowlist";
    for p in patterns {
        let p = decode_punycode(&p);
        let regex = format!("^{}$", p.replace('.', "\\.").replace('*', ".*").replace('/', "\\/"));
        if regex::Regex::new(&regex).is_ok_and(|re| re.is_match(&email)) {
            return allowlist;
        }
    }
    !allowlist
}

// ---------------------------------------------------------------------------------------------------------------
// The processing

/// `wpforms()->obj( 'process' )`.
struct Process<'a> {
    request: &'a Request<'a>,
    form_data: Value,
    /// `$this->errors[ $form_id ]` (only the submitted form has any).
    errors: Map<String, Value>,
    spam_errors: Map<String, Value>,
    spam_reason: Option<String>,
    fields: Map<String, Value>,
    /// The confirmation of `entry_confirmation_redirect()` and its message.
    confirmation: Option<Value>,
    confirmation_message: Option<String>,
    scroll: bool,
    redirect: Option<String>,
    emails: Vec<rustpress_core::native::Email>,
    saved: bool,
    complete: bool,
    /// A notification was made (`Notifications::hooks()` stays on): `Some( plain text )`.
    notification: Option<bool>,
}

/// `wpforms_get_required_label()`.
fn required(r: &Request<'_>) -> String {
    esc_html(&(r.tr)("This field is required."))
}

impl Process<'_> {
    fn tr(&self, text: &str) -> String {
        (self.request.tr)(text)
    }

    fn error(&mut self, field_id: &str, message: impl Into<Value>) {
        self.errors.insert(field_id.to_owned(), message.into());
    }

    fn field(&self, field_id: &str) -> Value {
        self.form_data["fields"].get(field_id).cloned().unwrap_or(Value::Null)
    }

    /// `WPForms_Field::validate()`.
    fn validate_base(&mut self, field_id: &str, submit: &Value) {
        if set(&self.field(field_id), "required") && empty(Some(submit)) && s(submit) != "0" {
            let label = required(self.request);
            self.error(field_id, label);
        }
    }

    /// `WPForms_Field::build_choices_allowlist()`.
    fn allowlist(&self, field: &Value) -> (Vec<String>, bool) {
        let show_values = set(field, "show_values");
        let mut list = Vec::new();
        let mut has_other = false;
        let choices = if set(field, "choices") && is_arr(&field["choices"]) { pairs(&field["choices"]) } else { Vec::new() };
        for (key, choice) in choices {
            if set(&choice, "other") {
                has_other = true;
            }
            let value = match (choice.get("value"), choice.get("label")) {
                (Some(v), _) if show_values && !v.is_null() && !is_empty_string(Some(v)) => s(v),
                (_, Some(l)) if !show_values && !l.is_null() && !is_empty_string(Some(l)) => s(l),
                _ => self.tr("Choice %s").replace("%s", &key),
            };
            list.push(trim(&value).to_owned());
        }
        (list, has_other)
    }

    /// `WPForms_Field::validate_choices_allowlist()`.
    fn validate_choices_allowlist(&mut self, field_id: &str, submit: &Value) {
        let field = self.field(field_id);
        if empty(Some(&field)) || is_empty_string(Some(submit)) || submit.is_null() || (is_arr(submit) && values(submit).is_empty()) || set(&field, "dynamic_choices") {
            return;
        }
        let (list, has_other) = self.allowlist(&field);
        let valueless = |v: &Value| is_empty_string(Some(v)) || v.is_null();
        let known = |v: &Value| list.iter().any(|a| *a == trim(&s(v)));
        let reject = if is_arr(submit) && set(submit, "other") {
            !has_other || pairs(submit).iter().any(|(k, item)| k != "other" && !valueless(item) && !known(item))
        } else {
            let items = if is_arr(submit) { values(submit) } else { vec![submit.clone()] };
            items.iter().any(|item| !valueless(item) && !known(item))
        };
        if reject {
            let message = esc_html(&self.tr("The selected option is invalid."));
            self.error(&php::leading_int(field_id).to_string(), message);
        }
    }

    /// `WPForms_Field::validate_other_choice_value()`.
    fn validate_other_choice(&mut self, field_id: &str, submit: &Value) {
        let field = self.field(field_id);
        if set(&field, "read_only") || !is_arr(submit) {
            return;
        }
        let Some(other) = at(submit, "other").filter(|o| !o.is_null() && !is_arr(o)) else { return };
        if !self.allowlist(&field).1 || !trim(&s(other)).is_empty() {
            return;
        }
        let label = required(self.request);
        self.error(field_id, label);
    }

    /// `WPForms_Field::sanitize_choices_submission()`.
    fn sanitize_choices(&self, submit: &Value, field: &Value) -> Value {
        if set(field, "dynamic_choices") || is_empty_string(Some(submit)) || (is_arr(submit) && values(submit).is_empty()) {
            return submit.clone();
        }
        let (list, has_other) = self.allowlist(field);
        let known = |v: &Value| list.iter().any(|a| *a == trim(&s(v)));
        if !is_arr(submit) {
            return if known(submit) { submit.clone() } else { json!("") };
        }
        let mut out = Map::new();
        for (key, item) in pairs(submit) {
            if key != "other" && known(&item) {
                out.insert(out.len().to_string(), item);
            }
        }
        if let Some(other) = at(submit, "other").filter(|o| !o.is_null())
            && has_other
        {
            out.insert("other".into(), other.clone());
        }
        Value::Object(out)
    }

    /// `WPForms_Field::get_choices_label()`.
    fn choices_label(&self, label: &Value, key: i64, field: &Value) -> String {
        let label = trim(&s(label)).to_owned();
        if (set(field, "choices_icons") || set(field, "choices_images")) && label.is_empty() {
            return String::new();
        }
        if label.is_empty() { self.tr("Choice %d").replace("%d", &key.to_string()) } else { label }
    }

    /// `wpforms_process_validate_{$type}`.
    fn validate(&mut self, ty: &str, field_id: &str, submit: &Value) {
        let field = self.field(field_id);
        match ty {
            "text" | "textarea" => {
                self.validate_base(field_id, submit);
                if empty(Some(&field)) || !set(&field, "limit_enabled") {
                    return;
                }
                let value = if ty == "text" {
                    stf(submit)
                } else if is_arr(submit) {
                    sanitize_textarea(&json!(values(submit).iter().filter(|v| !empty(Some(v))).map(s).collect::<Vec<_>>().join("\r\n")))
                } else {
                    sanitize_textarea(submit)
                };
                self.validate_length_limit(field_id, &value, &field);
            }
            "email" => self.validate_email(field_id, submit, &field),
            "name" => {
                if !set(&field, "required") {
                    return;
                }
                let format = str_of(&field, "format");
                let label = required(self.request);
                if format == "simple" && is_empty_string(Some(submit)) {
                    self.error(field_id, label);
                    return;
                }
                if format != "first-last" && format != "first-middle-last" {
                    return;
                }
                let first = is_arr(submit) && is_empty_string(at(submit, "first"));
                let last = is_arr(submit) && is_empty_string(at(submit, "last"));
                if (first || last) && !self.errors.get(field_id).is_some_and(Value::is_object) {
                    // `(array) $errors[ $form_id ][ $field_id ]`.
                    let previous = self.errors.get(field_id).cloned();
                    let mut map = Map::new();
                    if let Some(p) = previous {
                        map.insert("0".into(), p);
                    }
                    self.error(field_id, Value::Object(map));
                }
                if first {
                    self.errors.get_mut(field_id).and_then(Value::as_object_mut).map(|m| m.insert("first".into(), json!(label.clone())));
                }
                if last {
                    self.errors.get_mut(field_id).and_then(Value::as_object_mut).map(|m| m.insert("last".into(), json!(label)));
                }
            }
            "number" => {
                let value = number_value(submit);
                let v = Value::String(value.clone());
                if set(&field, "required") && empty(Some(&v)) && php::numeric_str(&value).is_none() {
                    let label = required(self.request);
                    self.error(field_id, label);
                }
                if !empty(Some(&v)) && php::numeric_str(&value).is_none() {
                    let message = esc_html(&self.tr("Please enter a valid number."));
                    self.error(field_id, message);
                }
            }
            "number-slider" => {
                let value = slider_value(submit);
                if set(&field, "required") && value == 0.0 && php::float(value) != "0" {
                    let label = required(self.request);
                    self.error(field_id, label);
                }
            }
            "select" | "radio" | "checkbox" if set(&field, "dynamic_choices") => {
                // `is_dynamic_choices_empty()` needs the posts or terms of the choices (not ported, see PLAN.md).
            }
            "select" => {
                self.validate_base(field_id, submit);
                self.validate_choices_allowlist(field_id, submit);
            }
            "radio" => {
                self.validate_base(field_id, submit);
                self.validate_other_choice(field_id, submit);
                self.validate_choices_allowlist(field_id, submit);
            }
            "checkbox" => {
                let submit = if is_arr(submit) {
                    submit.clone()
                } else if is_empty_string(Some(submit)) {
                    json!({})
                } else {
                    json!({"0": submit})
                };
                let choices: Vec<(String, Value)> = pairs(&submit).into_iter().filter(|(k, _)| k != "other").collect();
                let limit = if field.get("choice_limit").is_some_and(|v| !v.is_null()) { php::int(field.get("choice_limit")) } else { 0 };
                if limit != 0 && choices.len() as i64 > limit {
                    let message = self.request.settings.string("validation-check-limit", &esc_html(&self.tr("You have exceeded the number of allowed selections: {#}."))).replace("{#}", &limit.to_string());
                    self.error(field_id, message);
                }
                let all = pairs(&submit);
                if set(&field, "required") && (all.is_empty() || (all.len() == 1 && at(&submit, "0").is_none_or(|v| v.is_null() || s(v).is_empty()))) {
                    let label = required(self.request);
                    self.error(field_id, label);
                }
                self.validate_other_choice(field_id, &submit);
                self.validate_choices_allowlist(field_id, &submit);
            }
            "gdpr-checkbox" => {
                self.validate_base(field_id, submit);
                self.validate_choices_allowlist(field_id, submit);
            }
            _ => {}
        }
    }

    /// `TextLengthLimit::validate_length_limit()`.
    fn validate_length_limit(&mut self, field_id: &str, value: &str, field: &Value) {
        let (min, max) = super::fields::length_limits(field);
        if value.is_empty() {
            if min > 0 && set(field, "required") {
                let label = required(self.request);
                self.error(field_id, label);
            }
            return;
        }
        let words = set(field, "limit_mode") && super::fields::sanitize_key(&str_of(field, "limit_mode")) != "characters";
        let length = if words { count_words(value) } else { value.replace("\r\n", "\n").chars().count() as i64 };
        let r = self.request;
        let message = if max > 0 && length > max {
            if words {
                (r.trn)("Text can't exceed %d word.", "Text can't exceed %d words.", max).replace("%d", &max.to_string())
            } else {
                (r.trn)("Text can't exceed %d character.", "Text can't exceed %d characters.", max).replace("%d", &max.to_string())
            }
        } else if length >= min {
            return;
        } else {
            let template = if words {
                r.settings.string("validation-word-minimum", &esc_html(&(r.tr)("Minimum is %1$s words. Words needed: %2$s.")).replace("%1$s", "{min}").replace("%2$s", "{needed}"))
            } else {
                r.settings.string("validation-character-minimum", &esc_html(&(r.tr)("Minimum is %1$s characters. Characters needed: %2$s.")).replace("%1$s", "{min}").replace("%2$s", "{needed}"))
            };
            template.replace("{min}", &min.to_string()).replace("{needed}", &(min - length).to_string())
        };
        self.error(field_id, message);
    }

    /// `WPForms_Field_Email::validate()`.
    fn validate_email(&mut self, field_id: &str, submit: &Value, field: &Value) {
        self.validate_base(field_id, submit);
        let submit = if !is_arr(submit) && !empty(Some(submit)) { json!({"primary": submit}) } else { submit.clone() };
        if empty(at(&submit, "primary")) {
            return;
        }
        let invalid = esc_html(&self.tr("The provided email is not valid."));
        let primary = email_encode_punycode(at(&submit, "primary").unwrap_or(&Value::Null));
        if primary.is_empty() {
            self.error(field_id, invalid);
            return;
        }
        let restricted = || self.request.settings.string("validation-email-restricted", &esc_html(&self.tr("This email address is not allowed.")));
        let confirmation = field.get("confirmation").is_some_and(|v| !v.is_null());
        if confirmation && !empty(at(&submit, "secondary")) {
            let secondary = email_encode_punycode(at(&submit, "secondary").unwrap_or(&Value::Null));
            if secondary.is_empty() {
                self.error(field_id, invalid);
                return;
            }
            if primary != secondary {
                let message = esc_html(&self.tr("The provided emails do not match."));
                self.error(field_id, message);
                return;
            }
            if !email_allowed(&primary, field) {
                let message = restricted();
                self.error(field_id, message);
                return;
            }
        }
        if !confirmation && !email_allowed(&primary, field) {
            let message = restricted();
            self.error(field_id, message);
        }
    }

    /// `wpforms_process_format_{$type}`.
    fn format(&mut self, ty: &str, field_id: &str, submit: &Value) {
        let field = self.field(field_id);
        let label = || if set(&field, "label") { text_field(&str_of(&field, "label")) } else { String::new() };
        let id = json!(php::leading_int(field_id).abs());
        let data = match ty {
            "text" => json!({"name": label(), "value": stf(submit), "id": id, "type": ty}),
            "textarea" => {
                let submit = if is_arr(submit) { json!(values(submit).iter().filter(|v| !empty(Some(v))).map(s).collect::<Vec<_>>().join("\r\n")) } else { submit.clone() };
                json!({"name": label(), "value": sanitize_textarea(&submit), "id": id, "type": ty})
            }
            "email" => {
                let value = if is_arr(submit) { at(submit, "primary").filter(|v| !empty(Some(v))).cloned().unwrap_or(json!("")) } else if empty(Some(submit)) { json!("") } else { submit.clone() };
                let truthy = !empty(Some(&value));
                let decoded = if truthy && !is_arr(&value) && is_email(&s(&value)) { decode_punycode(&s(&value)) } else { String::new() };
                if truthy && decoded.is_empty() {
                    let message = esc_html(&self.tr("The provided email is not valid."));
                    self.error(field_id, message);
                    return;
                }
                let name = if set(&field, "label") { str_of(&field, "label") } else { String::new() };
                json!({"name": text_field(&name), "value": text_field(&decoded), "id": id, "type": ty})
            }
            "name" => {
                let name = field.get("label").filter(|l| !l.is_null() && !is_empty_string(Some(l))).map(s).unwrap_or_default();
                let part = |key: &str| if is_arr(submit) { at(submit, key).filter(|v| !is_empty_string(Some(v))).cloned().unwrap_or(json!("")) } else { json!("") };
                let (first, middle, last) = (part("first"), part("middle"), part("last"));
                let value = if is_arr(submit) {
                    [&first, &middle, &last].into_iter().filter(|v| !empty(Some(v))).map(s).collect::<Vec<_>>().join(" ")
                } else {
                    s(submit)
                };
                json!({"name": text_field(&name), "value": text_field(&value), "id": id, "type": ty, "first": stf(&first), "middle": stf(&middle), "last": stf(&last)})
            }
            "number" => {
                let name = if set(&field, "label") { str_of(&field, "label") } else { String::new() };
                json!({"name": text_field(&name), "value": number_value(submit), "id": id, "type": ty})
            }
            "number-slider" => {
                let name = if set(&field, "label") { str_of(&field, "label") } else { String::new() };
                let value = slider_value(submit);
                let raw = json!({
                    "value": value,
                    "min": php::to_float(field.get("min")),
                    "max": php::to_float(field.get("max")),
                    "value_display": rustpress_core::kses::kses_post(&str_of(&field, "value_display")),
                });
                json!({"name": text_field(&name), "value": value, "value_raw": raw, "id": id, "type": ty})
            }
            "select" => self.format_select(&field, submit, id),
            "radio" => self.format_radio(&field, submit, id),
            "checkbox" => self.format_checkbox(&field, submit, id),
            "gdpr-checkbox" => json!({
                "name": label(),
                "value": field.get("choices").and_then(|c| at(c, "1")).and_then(|c| c.get("label")).cloned().unwrap_or(Value::Null),
                "id": id,
                "type": ty,
            }),
            _ => {
                // `WPForms_Field::format()`.
                let submit = if is_arr(submit) { json!(values(submit).iter().filter(|v| !empty(Some(v))).map(s).collect::<Vec<_>>().join("\r\n")) } else { submit.clone() };
                json!({"name": label(), "value": sanitize_textarea(&submit), "id": id, "type": ty})
            }
        };
        self.fields.insert(field_id.to_owned(), data);
    }

    /// `WPForms_Field_Select::format()` (static choices).
    fn format_select(&self, field: &Value, submit: &Value, id: Value) -> Value {
        let submit = if is_arr(submit) { submit.clone() } else { json!({"0": submit}) };
        let submit = self.sanitize_choices(&submit, field);
        let items = values(&submit);
        let raw = combine(&items).unwrap_or_default();
        let mut data = json!({"name": text_field(&str_of(field, "label")), "value": "", "value_raw": raw, "id": id, "type": "select"});
        if set(field, "show_values") && php::int(field.get("show_values")) == 1 {
            let mut labels = Vec::new();
            for item in &items {
                for (key, choice) in pairs(&field["choices"]) {
                    if choice.get("value").is_some_and(|v| v == item) {
                        let label = self.choices_label(choice.get("label").unwrap_or(&json!("")), php::leading_int(&key), field);
                        if !label.is_empty() {
                            labels.push(json!(label));
                        }
                        break;
                    }
                }
            }
            data["value"] = json!(combine(&labels).unwrap_or_default());
        } else {
            data["value"] = json!(raw);
        }
        data
    }

    /// `WPForms_Field_Radio::format()` (static choices).
    fn format_radio(&self, field: &Value, submit: &Value, id: Value) -> Value {
        let submit = self.sanitize_choices(submit, field);
        let raw = if is_arr(&submit) { String::new() } else { stf(&submit) };
        let mut data = json!({"name": text_field(&str_of(field, "label")), "value": "", "value_raw": raw, "id": id, "type": "radio"});
        let mut choice_key = String::new();
        if set(field, "show_values") {
            for (key, choice) in pairs(&field["choices"]) {
                if !empty(Some(&submit)) && choice.get("value").is_some_and(|v| *v == submit) {
                    data["value"] = json!(choice.get("label").map(stf).unwrap_or_default());
                    choice_key = key;
                    break;
                }
            }
        } else {
            data["value"] = json!(raw);
            for (key, choice) in pairs(&field["choices"]) {
                let label = choice.get("label").filter(|l| !l.is_null()).cloned().unwrap_or(json!(""));
                if submit == label || raw == self.tr("Choice %s").replace("%s", &key) {
                    choice_key = key;
                    break;
                }
            }
        }
        if !choice_key.is_empty() && choice_key != "0" && set(field, "choices_images") {
            let image = field["choices"].get(&choice_key).filter(|c| set(c, "image")).map(|c| esc_url_raw(&str_of(c, "image"))).unwrap_or_default();
            data["image"] = json!(image);
        }
        if is_arr(&submit) && set(&submit, "other") {
            data["value"] = json!(stf(&submit["other"]));
            data["is_other"] = json!(true);
            for (_, choice) in pairs(&field["choices"]) {
                if choice.get("other").is_some_and(|o| !o.is_null()) {
                    data["value_raw"] = choice.get("label").cloned().unwrap_or(json!(""));
                    data["image"] = json!(if set(field, "choices_images") && set(&choice, "image") { esc_url_raw(&str_of(&choice, "image")) } else { String::new() });
                    break;
                }
            }
        }
        data
    }

    /// `WPForms_Field_Checkbox::format()` (static choices) with `format_other_option()`.
    fn format_checkbox(&self, field: &Value, submit: &Value, id: Value) -> Value {
        let submit = if is_arr(submit) { submit.clone() } else { json!({"0": submit}) };
        let submit = self.sanitize_choices(&submit, field);
        let submit = if is_arr(&submit) { submit } else { json!({"0": submit}) };
        let other_raw = at(&submit, "other").filter(|o| !o.is_null() && !is_arr(o)).map(|o| super::fields::stripslashes(&s(o))).unwrap_or_default();
        let items: Vec<Value> = pairs(&submit).into_iter().filter(|(k, _)| k != "other").map(|(_, v)| v).collect();
        let raw = combine(&items).unwrap_or_default();
        let mut data = json!({"name": text_field(&str_of(field, "label")), "value": "", "value_raw": raw, "id": id, "type": "checkbox"});
        let mut keys: Vec<String> = Vec::new();
        if set(field, "show_values") && php::int(field.get("show_values")) == 1 {
            let mut labels = Vec::new();
            for item in &items {
                for (key, choice) in pairs(&field["choices"]) {
                    let value = choice.get("value").cloned().unwrap_or(Value::Null);
                    let by_number = empty(Some(&value)) && php::leading_int(&s(item).replace("Choice ", "")) == php::leading_int(&key) && key.parse::<i64>().is_ok();
                    if (!empty(Some(item)) && *item == value) || by_number {
                        let label = self.choices_label(choice.get("label").unwrap_or(&json!("")), php::leading_int(&key), field);
                        if !label.is_empty() {
                            labels.push(json!(label));
                        }
                        keys.push(key);
                        break;
                    }
                }
            }
            data["value"] = json!(combine(&labels).unwrap_or_default());
        } else {
            data["value"] = json!(raw);
            for item in &items {
                for (key, choice) in pairs(&field["choices"]) {
                    let label = choice.get("label").filter(|l| !l.is_null()).cloned().unwrap_or(json!(""));
                    if *item == label || *item == json!(self.tr("Choice %s").replace("%s", &key)) {
                        keys.push(key);
                        break;
                    }
                }
            }
        }
        if !keys.is_empty() && set(field, "choices_images") {
            data["images"] = Value::Array(keys.iter().map(|k| json!(field["choices"].get(k).filter(|c| set(c, "image")).map(|c| esc_url_raw(&str_of(c, "image"))).unwrap_or_default())).collect());
        }
        // `format_other_option()`.
        let other = pairs(&field["choices"]).into_iter().find(|(_, c)| set(c, "other"));
        if let Some((_, choice)) = other
            && !other_raw.is_empty()
        {
            let other_value = text_field(&other_raw);
            let other_label = choice.get("label").map(s).unwrap_or_default();
            data["is_other"] = json!(true);
            data["other_label"] = json!(other_label);
            data["other_value"] = json!(other_value);
            let formatted = if other_label.is_empty() {
                other_value.clone()
            } else if other_value.is_empty() {
                String::new()
            } else {
                format!("{other_label}: {other_value}")
            };
            let current = str_of(&data, "value");
            data["value"] = json!(if current.is_empty() {
                formatted
            } else {
                let mut parts: Vec<&str> = current.split('\n').filter(|p| trim(p) != other_label).collect();
                parts.push(&formatted);
                parts.join("\n")
            });
        }
        data
    }
}

/// `WPForms_Field_Number::sanitize_value()`.
fn number_value(value: &Value) -> String {
    if is_arr(value) {
        return if values(value).is_empty() { String::new() } else { "Array".into() };
    }
    let text = s(value);
    if empty(Some(value)) && php::numeric_str(&text).is_none() {
        return String::new();
    }
    let signed: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '-' || *c == '.').collect();
    let abs = signed.replace('-', "");
    // `$signed_value < 0`: numeric strings compare as numbers, others as strings with "0".
    let negative = match php::numeric_str(&signed) {
        Some(n) => n < 0.0,
        None => signed.as_str() < "0",
    };
    if negative { format!("-{abs}") } else { abs }
}

/// `(float) WPForms_Field_Number_Slider::sanitize_value()`.
fn slider_value(value: &Value) -> f64 {
    let signed: String = s(value).chars().filter(|c| c.is_ascii_digit() || *c == '-' || *c == '.').collect();
    let Some(n) = php::numeric_str(&signed) else { return 0.0 };
    if signed.starts_with('-') { -n.abs() } else { n.abs() }
}

/// `wpforms_count_words()`.
fn count_words(text: &str) -> i64 {
    let text = trim(text);
    if text.is_empty() {
        return 0;
    }
    static RES: std::sync::OnceLock<[regex::Regex; 3]> = std::sync::OnceLock::new();
    let res = RES.get_or_init(|| {
        [
            regex::Regex::new(r"(?i-u)([A-Z]+),([A-Z]+)").expect("valid regex"),
            regex::Regex::new(r"(?i-u)([0-9]+),([A-Z]+)").expect("valid regex"),
            regex::Regex::new(r"(?i-u)([A-Z]+),([0-9]+)").expect("valid regex"),
        ]
    });
    let mut text = text.to_owned();
    for re in res {
        text = re.replace_all(&text, "$1, $2").into_owned();
    }
    text.split([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']).collect::<Vec<_>>().len() as i64 - count_merged(&text)
}

/// `preg_split( '/[\s]+/' )` treats a run of whitespace as one separator.
fn count_merged(text: &str) -> i64 {
    let ws = |c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}');
    let mut merged = 0;
    let mut previous = false;
    for c in text.chars() {
        let is = ws(c);
        if is && previous {
            merged += 1;
        }
        previous = is;
    }
    merged
}

/// `Token::get_error_message()`.
fn token_error(r: &Request<'_>, text: &str) -> String {
    let text = format!("{} {}", esc_html(&(r.tr)(text)), esc_html(&(r.tr)("Please reload the page and try submitting the form again.")));
    if !r.can_manage {
        return text;
    }
    if !r.super_admin {
        return text;
    }
    format!(
        "{text} {}",
        esc_html(&(r.tr)("Please check out our %1$stroubleshooting guide%2$s for details on resolving this issue.")).replace("%1$s", "<a href=\"https://wpforms.com/docs/getting-support-wpforms/\">").replace("%2$s", "</a>")
    )
}

/// The form `$_POST['wpforms']['id']` names, as `wpforms()->obj( 'form' )->get()` finds it.
pub struct Form<'a> {
    pub status: &'a str,
    pub data: &'a Value,
    pub post_date: &'a str,
}

/// `WPForms_Process::ajax_submit()`: the JSON `admin-ajax.php` answers with.
pub fn ajax_submit<'f>(forms: &dyn Fn(u64) -> Option<Form<'f>>, r: &Request<'_>) -> Outcome {
    let wpforms = r.post.get("wpforms").cloned().unwrap_or(Value::Null);
    let form_id = php::int(at(&wpforms, "id")).max(0) as u64;
    let fail = || Outcome { body: "{\"success\":false}".into(), ..Outcome::default() };
    if form_id == 0 {
        return fail();
    }
    let mut p = Process {
        request: r,
        form_data: Value::Null,
        errors: Map::new(),
        spam_errors: Map::new(),
        spam_reason: None,
        fields: Map::new(),
        confirmation: None,
        confirmation_message: None,
        scroll: false,
        redirect: None,
        emails: Vec::new(),
        saved: false,
        complete: false,
        notification: None,
    };
    let form = forms(form_id);
    process(&mut p, form.as_ref(), wpforms);
    if let Some(url) = p.redirect.take() {
        // `ajax_process_redirect()` on `wp_redirect` with `maybe_open_in_new_tab()`.
        let mut data = json!({"form_id": form_id, "redirect_url": url});
        if p.confirmation.as_ref().is_some_and(|c| set(c, "redirect_new_tab")) {
            data["new_tab"] = json!(true);
        }
        return Outcome { body: super::encode(&json!({"success": true, "data": data})), emails: p.emails, saved: p.saved, complete: p.complete };
    }
    let form_data = if empty(Some(&p.form_data)) {
        form.as_ref().map(|f| f.data.clone()).unwrap_or(Value::Bool(false))
    } else {
        p.form_data.clone()
    };
    if !p.errors.is_empty() {
        let body = errors_response(&p, &form_data);
        return Outcome { body, emails: p.emails, saved: p.saved, complete: p.complete };
    }
    // `Frontend::confirmation()`.
    let mut confirmation = String::new();
    if let (Some(_), Some(message)) = (&p.confirmation, p.confirmation_message.as_ref().filter(|m| !m.is_empty() && *m != "0")) {
        let fields = Value::Object(p.fields.clone());
        let tags = Tags { request: r, form_data: &form_data, fields: &fields, entry_meta: form_data.get("entry_meta"), notification: p.notification, recipients_can: false };
        let message = wpautop(&tags.process(message, "confirmation", false));
        if !message.is_empty() {
            let mut class = if super::render::disable_css(r.settings) == 1 { "wpforms-confirmation-container-full".to_owned() } else { "wpforms-confirmation-container".to_owned() };
            if p.scroll {
                class.push_str(" wpforms-confirmation-scroll");
            }
            confirmation = format!("<div class=\"{}\" id=\"wpforms-confirmation-{}\" tabindex=\"-1\">{}</div>", php::sanitize_class_string(&class), php::int(form_data.get("id")).abs(), message);
        }
    }
    Outcome { body: super::encode(&json!({"success": true, "data": {"confirmation": confirmation}})), emails: p.emails, saved: p.saved, complete: p.complete }
}

/// `WPForms_Process::ajax_process_errors()`.
fn errors_response(p: &Process<'_>, form_data: &Value) -> String {
    let r = p.request;
    let modern = super::render::modern(r.settings);
    let id = form_data.get("id").map(string).unwrap_or_else(|| "0".into());
    let mut general = Map::new();
    for (key, error) in &p.errors {
        if matches!(key.as_str(), "header" | "footer" | "header_styled" | "footer_styled" | "recaptcha") {
            general.insert(key.clone(), json!(super::render::form_error_html(key, &s(error), &id, r.tr, modern)));
        }
    }
    let fields = form_data.get("fields").cloned().unwrap_or(Value::Null);
    let mut field_errors = Map::new();
    for (key, error) in &p.errors {
        let Some(field) = at(&fields, key) else { continue };
        if let Some(name) = error_field_name(field, form_data, error, r).filter(|n| !n.is_empty() && n != "0") {
            field_errors.insert(name, error.clone());
        }
    }
    let mut errors = Map::new();
    if !general.is_empty() {
        errors.insert("general".into(), Value::Object(general));
    }
    if !field_errors.is_empty() {
        errors.insert("field".into(), Value::Object(field_errors));
    }
    if errors.is_empty() {
        return "{\"success\":false,\"data\":[]}".into();
    }
    super::encode(&json!({"success": false, "data": {"errors": errors}}))
}

/// `WPForms_Field::ajax_error_field_name()`: the name of the input the error belongs to.
fn error_field_name(field: &Value, form_data: &Value, error: &Value, r: &Request<'_>) -> Option<String> {
    let props = super::render::field_props(field, form_data, r.settings, r.tr);
    let inputs = props.get("inputs").cloned().unwrap_or(Value::Null);
    let input = match error {
        Value::Object(m) if m.keys().next().is_some_and(|k| inputs.get(k).is_some()) => inputs.get(m.keys().next()?.as_str()).cloned(),
        _ => inputs.get("primary").cloned().or_else(|| inputs.as_object().and_then(|m| m.values().next_back().cloned())),
    };
    input.and_then(|i| i.get("attr").and_then(|a| a.get("name")).map(string))
}

/// `WPForms_Process::process()` and what `listen()` does after it.
fn process(p: &mut Process<'_>, form: Option<&Form<'_>>, mut entry: Value) {
    let r = p.request;
    let form_id = php::int(at(&entry, "id")).max(0).to_string();
    let Some(form) = form.filter(|f| f.status == "publish") else {
        let message = esc_html(&p.tr("Invalid form."));
        p.error("header", message);
        return;
    };
    p.form_data = before_form_data(form.data.clone(), r);
    // `is_valid_ajax_submit_action()`: the form is sent to `admin-ajax.php` with `$_POST['action']`.
    if !empty(p.form_data["settings"].get("ajax_submit")) && r.post.get("action").and_then(Value::as_str) != Some("wpforms_submit") {
        let message = esc_html(&p.tr("Attempt to submit corrupted post data."));
        p.error("header", message);
        return;
    }
    let store_spam = set(&p.form_data["settings"], "store_spam_entries");
    // `modern_anti_spam_check()`.
    if set(&p.form_data["settings"], "antispam_v3") {
        let known = p.form_data.get("fields").cloned().unwrap_or(Value::Null);
        let submitted = entry.get("fields").cloned().unwrap_or(Value::Null);
        let mut valid = true;
        if let Value::Object(map) = &submitted {
            let mut kept = Map::new();
            for (key, value) in map {
                if at(&known, key).is_some() {
                    kept.insert(key.clone(), value.clone());
                } else if !empty(Some(value)) {
                    valid = false;
                }
            }
            entry["fields"] = Value::Object(kept);
        }
        if !valid {
            let message = esc_html(&p.tr("Anti-spam Honeypot V2 verification was failed, please try again later."));
            p.spam_errors.insert("header".into(), json!(message));
            p.spam_reason = Some("Honeypot V2".into());
            if !store_spam {
                return;
            }
        }
    }
    if p.form_data.get("fields").is_none_or(Value::is_null) || p.form_data.get("id").is_none_or(Value::is_null) {
        let message = esc_html(&p.tr("Your form has not been submitted because data is missing from the entry."));
        p.errors.clear();
        p.error("header", message);
        return;
    }
    let field_list: Vec<Value> = values(&p.form_data["fields"]);
    let submitted = entry.get("fields").cloned().unwrap_or(Value::Null);
    let submit_of = |field: &Value| at(&submitted, &s(field.get("id").unwrap_or(&Value::Null))).cloned().unwrap_or(json!(""));
    for field in &field_list {
        let ty = str_of(field, "type");
        if super::fields::is_known(&ty) && (ty != "gdpr-checkbox" || r.settings.set("gdpr")) {
            p.validate(&ty, &s(field.get("id").unwrap_or(&Value::Null)), &submit_of(field));
        }
    }
    if !p.errors.is_empty() {
        if !p.errors.contains_key("header") && !p.errors.contains_key("footer") || (empty(p.errors.get("header")) && empty(p.errors.get("footer"))) {
            let message = esc_html(&p.tr("Form has not been submitted, please see the errors below."));
            p.error("header", message);
        }
        return;
    }
    if r.user.is_some() {
        let nonce = at(&entry, "nonce").map(s).unwrap_or_default();
        if nonce.is_empty() || nonce == "0" || !(r.verify_nonce)(&nonce, &format!("wpforms::form_{form_id}")) {
            let message = esc_html(&p.tr("The form could not be submitted due to a security issue."));
            p.error("footer_styled", message);
            return;
        }
    }
    for field in &field_list {
        let ty = str_of(field, "type");
        if super::fields::is_known(&ty) && (ty != "gdpr-checkbox" || r.settings.set("gdpr")) {
            p.format(&ty, &s(field.get("id").unwrap_or(&Value::Null)), &submit_of(field));
        }
    }
    // `Honeypot::validate()`: the old honeypot fails silently.
    if str_of(&p.form_data["settings"], "honeypot") == "1" && !empty(at(&entry, "hp")) {
        return;
    }
    // `Token::validate()`.
    let settings = p.form_data["settings"].clone();
    if settings.get("antispam").and_then(Value::as_str) == Some("1") {
        let token = at(&entry, "token").map(s);
        let valid = token.is_some_and(|t| super::token::verify(&t, &str_of(&p.form_data, "id"), r.secret, r.now));
        if !valid {
            p.error("header", token_error(r, "Antispam token is invalid."));
            return;
        }
    }
    // `time_limit_check()`.
    let limit = settings.get("anti_spam").and_then(|a| a.get("time_limit")).cloned().unwrap_or(Value::Null);
    let duration = if set(&limit, "duration") { php::int(limit.get("duration")).abs() } else { 0 };
    if set(&limit, "enable") && duration > 0 {
        let token = at(&entry, "time_token").map(stf).unwrap_or_default();
        let start = super::token::verify_time_token(&token, php::int(p.form_data.get("id")).abs(), r.secret, r.now);
        if start == 0 || r.now - start < duration {
            let message = esc_html(&p.tr("Please wait a little longer before submitting. We’re running a quick security check."));
            p.error("header", message);
        }
    }
    if !store_spam && !p.spam_errors.is_empty() {
        p.errors = p.spam_errors.clone();
    }
    if let Some(reason) = &p.spam_reason {
        p.form_data["spam_reason"] = json!(reason);
    }
    p.form_data["created"] = json!(form.post_date);
    if !p.errors.is_empty() {
        if empty(p.errors.get("header")) && empty(p.errors.get("footer")) {
            let message = esc_html(&p.tr("Form has not been submitted, please see the errors below."));
            p.error("header", message);
        }
        return;
    }
    let page_url = r.post.get("page_url").map(|u| esc_url_raw(&s(u))).unwrap_or_default();
    p.form_data["post_data_raw"] = json!({"page_url": page_url});
    // `entry_save()` (Lite keeps no entries; `update_entry_count()` counts it).
    p.saved = p.spam_reason.is_none();
    p.complete = true;
    p.form_data["entry_meta"] = json!({
        "page_url": page_url,
        "page_title": r.post.get("page_title").map(|t| text_field(&s(t))).unwrap_or_default(),
        "page_id": r.post.get("page_id").map(|i| json!(php::int(Some(i)).abs())).unwrap_or(json!("")),
        "url_referer": r.post.get("url_referer").map(|u| esc_url_raw(&s(u))).unwrap_or_default(),
        "user_id": r.user.as_ref().map_or(0, |u| u.id),
    });
    let fields = Value::Object(p.fields.clone());
    let form_data = p.form_data.clone();
    let (emails, notification) = super::notify::entry_email(&fields, &form_data, r);
    p.emails = emails;
    p.notification = notification;
    confirmation_redirect(p);
}

/// `wpforms_process_before_form_data`: `InternalInformation::process_before_form_data()` and
/// `WPForms_Field::field_fill_empty_choices()`.
fn before_form_data(mut form_data: Value, r: &Request<'_>) -> Value {
    if let Some(Value::Object(fields)) = form_data.get_mut("fields") {
        fields.retain(|_, f| str_of(f, "type") != "internal-information");
        for field in fields.values_mut() {
            if !set(field, "choices") || !matches!(str_of(field, "type").as_str(), "radio" | "checkbox") {
                continue;
            }
            if let Some(Value::Object(choices)) = field.get_mut("choices") {
                for (key, choice) in choices.iter_mut() {
                    let has_value = choice.get("value").is_some_and(|v| !v.is_null() && !trim(&s(v)).is_empty());
                    if has_value || !set(choice, "image") {
                        continue;
                    }
                    choice["value"] = json!(esc_html(&(r.tr)("Choice %d")).replace("%d", &php::leading_int(key).to_string()));
                }
            }
        }
    }
    form_data
}

/// `WPForms_Process::entry_confirmation_redirect()`.
fn confirmation_redirect(p: &mut Process<'_>) {
    let r = p.request;
    let settings = p.form_data["settings"].clone();
    if empty(settings.get("confirmations")) {
        let pick = |key: &str, default: Value| if set(&settings, key) { settings[key].clone() } else { default };
        let confirmation = json!({
            "type": pick("confirmation_type", json!("message")),
            "message": pick("confirmation_message", json!(esc_html(&p.tr("Thanks for contacting us! We will be in touch with you shortly.")))),
            "message_scroll": pick("confirmation_message_scroll", json!(1)),
            "page": pick("confirmation_page", json!("")),
            "redirect": pick("confirmation_redirect", json!("")),
        });
        p.form_data["settings"]["confirmations"] = json!({"1": confirmation});
    }
    let confirmations = p.form_data["settings"]["confirmations"].clone();
    if !is_arr(&confirmations) || empty(Some(&confirmations)) {
        return;
    }
    // `krsort()` and `get_confirmation_id()`: the highest valid confirmation, else the lowest key.
    let mut list = pairs(&confirmations);
    list.sort_by(|a, b| php_key_cmp(&b.0, &a.0));
    let default = list.iter().map(|(k, _)| k.clone()).min_by(|a, b| php_key_cmp(a, b)).unwrap_or_default();
    let mut chosen = String::new();
    for (key, confirmation) in &list {
        chosen = key.clone();
        if *key == default {
            break;
        }
        let ty = str_of(confirmation, "type");
        if set(confirmation, "type") && confirmation.get(&ty).is_some_and(|v| !v.is_null() && !is_empty_string(Some(v))) {
            break;
        }
    }
    let confirmation = list.iter().find(|(k, _)| *k == chosen).map(|(_, c)| c.clone()).unwrap_or(json!({}));
    p.confirmation = Some(confirmation.clone());
    let ty = str_of(&confirmation, "type");
    let fields = Value::Object(p.fields.clone());
    let mut url = String::new();
    if set(&confirmation, "type") && ty != "message" {
        let tags = Tags { request: r, form_data: &p.form_data, fields: &fields, entry_meta: p.form_data.get("entry_meta"), notification: p.notification, recipients_can: false };
        if ty == "redirect" {
            url = tags.process(&str_of(&confirmation, "redirect"), "confirmation_redirect", true);
        }
        if ty == "page" && set(&confirmation, "page") {
            let page = str_of(&confirmation, "page");
            let base = if page != "previous_page" {
                let id = php::leading_int(&page);
                r.permalinks.iter().find(|(i, _)| *i == id).map(|(_, u)| u.clone()).unwrap_or_default()
            } else {
                let referer = tags.process("{url_referer}", "confirmation_redirect", false);
                if !referer.is_empty() && validate_redirect(&referer, r) { referer } else { r.home_url.clone() }
            };
            url = add_url_params(&confirmation, &base);
        }
    }
    if !url.is_empty() && url != "0" {
        p.redirect = Some(esc_url_raw(&url));
        return;
    }
    if set(&confirmation, "type") && ty == "message" {
        p.confirmation_message = Some(str_of(&confirmation, "message"));
        if set(&confirmation, "message_scroll") {
            p.scroll = true;
        }
    }
}

/// The order of PHP array keys: integers by value before strings.
fn php_key_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<i64>(), b.parse::<i64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        (Ok(_), Err(_)) => std::cmp::Ordering::Less,
        (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
        _ => a.cmp(b),
    }
}

/// `wp_validate_redirect()`: a URL of the site's host.
fn validate_redirect(url: &str, r: &Request<'_>) -> bool {
    let url = url.trim();
    let rest = match url.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") => rest,
        Some(_) => return false,
        None => return !url.starts_with("//"),
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h).split(':').next().unwrap_or_default();
    host.eq_ignore_ascii_case(&r.home_host)
}

/// `add_url_params_page_confirmation()`: `add_query_arg( parse_str( $page_url_parameters ), $url )`.
fn add_url_params(confirmation: &Value, url: &str) -> String {
    use crate::redirection::php::{Param, parse_str};
    if empty(confirmation.get("page_url_parameters")) {
        return url.to_owned();
    }
    let params = parse_str(&str_of(confirmation, "page_url_parameters"));
    // `add_query_arg()`: the existing query encoded again, the new arguments as they are (`build_query()`).
    let (before, fragment) = match url.split_once('#') {
        Some((a, f)) => (a, Some(f)),
        None => (url, None),
    };
    let (path, query) = before.split_once('?').unwrap_or((before, ""));
    let mut args: Vec<(String, Vec<String>)> = rustpress_core::canonical::parse_qs(query)
        .into_iter()
        .map(|(k, v)| (k.clone(), vec![format!("{}={}", k.replace('[', "%5B").replace(']', "%5D"), rustpress_core::canonical::php_urlencode(&v))]))
        .collect();
    fn flatten(prefix: &str, param: &Param, out: &mut Vec<String>) {
        match param {
            Param::Null => {}
            Param::Str(v) => out.push(format!("{prefix}={v}")),
            Param::Arr(items) => items.iter().for_each(|(k, v)| flatten(&format!("{prefix}%5B{k}%5D"), v, out)),
        }
    }
    for (key, param) in &params {
        let mut parts = Vec::new();
        flatten(key, param, &mut parts);
        match args.iter_mut().find(|(k, _)| k == key) {
            Some(slot) => slot.1 = parts,
            None => args.push((key.clone(), parts)),
        }
    }
    let query = args.into_iter().flat_map(|(_, p)| p).collect::<Vec<_>>().join("&");
    static EMPTY: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let query = EMPTY.get_or_init(|| regex::Regex::new("=(&|$)").expect("valid regex")).replace_all(&query, "$1").into_owned();
    let mut out = if query.is_empty() { path.to_owned() } else { format!("{path}?{query}") };
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// `Token::ajax_get_token()`.
pub fn ajax_get_token(r: &Request<'_>) -> String {
    let id = r.post.get("formId").map(s).unwrap_or_default();
    let id = trim(&id);
    let valid = !id.is_empty() && id.trim_start_matches(['+', '-']).bytes().all(|b| b.is_ascii_digit()) && id.parse::<i64>().is_ok_and(|n| n >= 1);
    if !valid {
        return "{\"success\":false}".into();
    }
    let n: i64 = id.parse().unwrap_or(0);
    super::encode(&json!({"success": true, "data": {"token": super::token::get(r.now, &n.to_string(), r.secret), "time_token": super::token::time_token(r.now, n, r.secret)}}))
}

/// `WPForms_Field_Email::ajax_check_restricted_email()`.
pub fn ajax_restricted_email<'f>(r: &Request<'_>, forms: &dyn Fn(u64) -> Option<Form<'f>>) -> String {
    let fail = "{\"success\":false}".to_owned();
    let form_id: String = r.post.get("form_id").map(s).unwrap_or_default().chars().filter(|c| c.is_ascii_digit() || *c == '+' || *c == '-').collect();
    let field_id = php::leading_int(&full_special_chars(&r.post.get("field_id").map(s).unwrap_or_default())).abs();
    let email = full_special_chars(&r.post.get("email").map(s).unwrap_or_default()).replace("&amp;", "&");
    if form_id.is_empty() || form_id == "0" || field_id == 0 || email.is_empty() || email == "0" {
        return fail;
    }
    let Some(form) = forms(php::leading_int(&form_id).max(0) as u64) else { return fail };
    let Some(field) = form.data.get("fields").and_then(|f| at(f, &field_id.to_string())).filter(|f| !empty(Some(f))) else { return fail };
    super::encode(&json!({"success": true, "data": email_allowed(&email, field)}))
}

/// `FILTER_SANITIZE_FULL_SPECIAL_CHARS` with `FILTER_FLAG_NO_ENCODE_QUOTES`.
fn full_special_chars(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_like_the_field() {
        assert!(is_email("ada@example.com"));
        assert!(is_email("ada@bücher.de"));
        assert!(!is_email("ada@example..com"));
        assert_eq!(decode_punycode("Ada@Example.COM"), "ada@example.com");
        assert_eq!(encode_punycode("ada@bücher.de"), "ada@xn--bcher-kva.de");
        let field = json!({"filter_type": "denylist", "denylist": "*@spam.com\nbad@example.com"});
        assert!(!email_allowed("x@spam.com", &field));
        assert!(!email_allowed("bad@example.com", &field));
        assert!(email_allowed("good@example.com", &field));
        let field = json!({"filter_type": "allowlist", "allowlist": "*@example.com"});
        assert!(email_allowed("good@example.com", &field));
        assert!(!email_allowed("x@other.com", &field));
    }

    #[test]
    fn numbers_and_words() {
        assert_eq!(number_value(&json!("12a")), "12");
        assert_eq!(number_value(&json!("-3.5")), "-3.5");
        assert_eq!(number_value(&json!("1-2")), "12");
        assert_eq!(count_words("one, two  three"), 3);
        assert_eq!(count_words("a,b"), 2);
    }
}
