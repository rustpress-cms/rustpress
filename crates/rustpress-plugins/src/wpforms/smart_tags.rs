//! The smart tags of a submission (`WPForms\SmartTags\SmartTags::process()` on `wpforms_process_smart_tags`) and the
//! filters of the field values they print (`wpforms_html_field_value`, `wpforms_smart_tags_formatted_field_value`).

use rustpress_core::formatting::{esc_html, esc_url, esc_url_raw, wp_strip_all_tags};
use rustpress_core::kses::kses_post;
use serde_json::Value;

use super::php::{self, empty, set, str_of, string};
use super::submit::Request;

/// `SmartTags::MARKUP_CONTEXTS`.
const MARKUP_CONTEXTS: &[&str] = &["field-properties", "form-description", "confirmation", "user-registration-frontend-message"];

/// The tags of `SmartTags::smart_tags_list()`; any other name is a `Generic` tag that stays as written.
const KNOWN: &[&str] = &[
    "admin_email", "field_id", "field_html_id", "field_value_id", "form_id", "form_name", "page_title", "page_url", "page_id", "date", "query_var",
    "user_ip", "user_id", "user_display", "user_full_name", "user_first_name", "user_last_name", "user_email", "user_meta", "author_id",
    "author_display", "author_email", "url_referer", "url_login", "url_logout", "url_register", "url_lost_password", "unique_value", "site_name",
    "order_summary", "trial_end_date", "trial_first_payment",
];

/// `wpforms_get_multi_fields()`.
pub fn is_multi_field(ty: &str) -> bool {
    matches!(ty, "checkbox" | "select" | "payment-checkbox" | "ranking")
}

/// What the tags of one call see.
pub struct Tags<'a> {
    pub request: &'a Request<'a>,
    pub form_data: &'a Value,
    /// `wpforms()->obj( 'process' )->fields` as the caller passes them.
    pub fields: &'a Value,
    /// `$process->form_data['entry_meta']` once the entry is saved.
    pub entry_meta: Option<&'a Value>,
    /// `Notifications::get_multi_field_formatted_value()` stays on `wpforms_smart_tags_formatted_field_value` once a
    /// notification was made: `Some( plain text template )`.
    pub notification: Option<bool>,
    /// `recipient_has_cap()`: the recipients of the notification are users who can `manage_options`.
    pub recipients_can: bool,
}

/// `wpforms_get_all_smart_tags()`: the tags in order of their first appearance, with their names.
fn all_tags(content: &str) -> Vec<(String, String)> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"\{([a-z0-9_]+)(|[ =][^\n}]*)\}").expect("valid regex"));
    let mut out: Vec<(String, String)> = Vec::new();
    for c in re.captures_iter(content) {
        let tag = c[0].to_owned();
        if !out.iter().any(|(t, _)| *t == tag) {
            out.push((tag, c[1].to_owned()));
        }
    }
    out
}

/// `SmartTag::get_attributes()`.
fn attributes(tag: &str) -> Vec<(String, String)> {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| fancy_regex::Regex::new(r#"(\w+)=(["'])(.+?)\2"#).expect("valid regex"));
    let decoded = decode_quotes(&rustpress_core::formatting::html_entity_decode(tag));
    let mut out: Vec<(String, String)> = Vec::new();
    for c in re.captures_iter(&decoded).flatten() {
        let (k, v) = (c.get(1).map_or("", |m| m.as_str()).to_owned(), c.get(3).map_or("", |m| m.as_str()).to_owned());
        match out.iter_mut().find(|(n, _)| *n == k) {
            Some(slot) => slot.1 = v,
            None => out.push((k, v)),
        }
    }
    out
}

/// The single quotes `html_entity_decode( ENT_QUOTES )` decodes as well.
pub fn decode_quotes(text: &str) -> String {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"&#0*39;|&#[xX]0*27;").expect("valid regex"));
    re.replace_all(text, "'").into_owned()
}

fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// `wpforms_neutralize_html_tags()`.
pub fn neutralize(value: &str) -> String {
    esc_html(&wp_strip_all_tags(value))
}

/// `$fields[ $field_id ]` by the ID a tag names.
fn field<'a>(fields: &'a Value, id: &str) -> Option<&'a Value> {
    fields.get(id)
}

impl Tags<'_> {
    /// `wpforms_process_smart_tags( $content, $form_data, $fields, $entry_id, $context )`. `rawurlencode_field_id`
    /// is the filter `entry_confirmation_redirect()` puts on `wpforms_smarttags_process_field_id_value`.
    pub fn process(&self, content: &str, context: &str, rawurlencode_field_id: bool) -> String {
        let mut content = content.to_owned();
        for (tag, name) in all_tags(&content.clone()) {
            let mut value = self.value(&tag, &name, context);
            let field_id = attr(&attributes(&tag), "field_id").map(|f| php::leading_int(f.split('|').next().unwrap_or_default())).unwrap_or(0);
            if let Some(v) = &value
                && context == "confirmation_redirect"
                && field_id > 0
                && is_multi_field(&str_of(self.fields.get(field_id.to_string()).unwrap_or(&Value::Null), "type"))
            {
                value = Some(v.replace("\r\n", "%7C").replace(['\r', '\n'], "|"));
            }
            if name == "field_id" && rawurlencode_field_id {
                value = value.map(|v| rustpress_core::canonical::rawurlencode(&v));
            }
            // `Notifications::filter_smarttags_process_value()`.
            if let Some(v) = &value
                && self.notification.is_some()
                && name != "admin_email"
                && name != "user_email"
                && (context == "notification-from" || context == "notification-reply-to")
            {
                value = Some(self.validate_address_tag(v, &name, &tag, context));
            }
            if let Some(v) = value {
                content = self.replace(&tag, &v, &content, context);
            }
        }
        content
    }

    /// `Notifications::validate_notification_email_smart_tags()`.
    fn validate_address_tag(&self, value: &str, name: &str, tag: &str, context: &str) -> String {
        let attrs = attributes(tag);
        let id = match name {
            "field_value_id" => attr(&attrs, "field_value_id").map(str::to_owned),
            "field_id" => Some(attr(&attrs, "field_id").unwrap_or_default().split('|').next().unwrap_or_default().to_owned()),
            _ => None,
        };
        let Some(id) = id.filter(|i| !i.is_empty()) else { return String::new() };
        let Some(ty) = self.fields.get(&id).and_then(|f| f.get("type")) else { return String::new() };
        let ty = string(ty);
        if ty == "email" || (ty == "name" && context == "notification-reply-to") {
            return value.to_owned();
        }
        if super::submit::is_email(value) { value.to_owned() } else { String::new() }
    }

    /// `SmartTags::replace()`.
    fn replace(&self, tag: &str, value: &str, content: &str, context: &str) -> String {
        let value = rustpress_core::shortcodes::strip_shortcodes_with(value, self.request.shortcodes);
        if !MARKUP_CONTEXTS.contains(&context) {
            return content.replace(tag, &value);
        }
        // `replace_in_markup()`: a tag inside an HTML tag or a script is escaped for its place; RustPress replaces it
        // as text (see PLAN.md).
        content.replace(tag, &value.replace('[', "&#091;").replace(']', "&#093;"))
    }

    fn is_markup_context(context: &str) -> bool {
        MARKUP_CONTEXTS.contains(&context)
    }

    /// `SmartTag::get_formatted_field_value()`.
    fn formatted_field_value(&self, field_id: &str, key: &str) -> String {
        let field = self.fields.get(field_id).unwrap_or(&Value::Null);
        let mut value = field.get(key).map(string).unwrap_or_default();
        let ty = str_of(field, "type");
        // `WPForms_Field_Radio::smart_tags_formatted_field_value()`.
        if ty == "radio" && key == "value" && set(field, "is_other") {
            value = radio_other_value(field).unwrap_or(value);
        }
        if let Some(plain) = self.notification
            && is_multi_field(&ty)
        {
            value = if plain { value.replace("<br/>", "\r\n") } else { value.replace("\r\n", "<br/>").replace(['\r', '\n'], "<br/>") };
        }
        value
    }

    /// `SmartTag::sanitize_field_value()`.
    fn sanitize_field_value(value: &str, field: &Value, context: &str) -> String {
        let value = super::fields::stripslashes(value);
        if !Self::is_markup_context(context) {
            return kses_post(&value);
        }
        if !matches!(str_of(field, "type").as_str(), "richtext" | "gdpr-checkbox" | "map") {
            static BR: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
            let br = BR.get_or_init(|| regex::Regex::new(r"(?i)<br\s*/?>").expect("valid regex"));
            return neutralize(&br.replace_all(&value, "\n"));
        }
        kses_post(&value)
    }

    /// `SmartTag::get_user()`: the signed-in user (Lite has no entries to take it from).
    fn user(&self) -> Option<&super::submit::User> {
        self.request.user.as_ref()
    }

    /// `SmartTag::has_cap()`.
    fn has_cap(&self, context: &str) -> bool {
        match context {
            "notification" | "notification-carboncopy" | "notification-from" | "notification-reply-to" | "email" => self.recipients_can,
            "confirmation" => self.request.can_manage,
            _ => true,
        }
    }

    /// `$smart_tag_object->get_value()` of the tag's class.
    fn value(&self, tag: &str, name: &str, context: &str) -> Option<String> {
        if !KNOWN.contains(&name) {
            return None;
        }
        let r = self.request;
        let post = |key: &str| r.post.get(key).filter(|v| !v.is_object() && !v.is_array()).map(string);
        let attrs = attributes(tag);
        let value = match name {
            "admin_email" => rustpress_content::sanitize::sanitize_email(&r.admin_email),
            "field_id" => {
                let Some(id) = attr(&attrs, "field_id").filter(|f| !f.is_empty()) else { return Some(String::new()) };
                let parts: Vec<&str> = id.split('|').collect();
                let field_id = parts[0];
                let Some(f) = field(self.fields, field_id).filter(|f| f.as_str() != Some("")) else { return Some(String::new()) };
                let key = parts.get(1).filter(|k| !k.is_empty() && **k != "0").map_or_else(|| "value".to_owned(), |k| super::fields::sanitize_key(k));
                let value = self.formatted_field_value(&php::leading_int(field_id).to_string(), &key);
                Self::sanitize_field_value(&value, f, context)
            }
            "field_html_id" => {
                let Some(id) = attr(&attrs, "field_html_id").filter(|i| php::numeric_str(i).is_some_and(|n| n >= 0.0)) else { return Some(String::new()) };
                let field_id = php::leading_int(id).abs().to_string();
                let Some(f) = field(self.fields, &field_id).filter(|f| !empty(Some(f))) else { return Some(String::new()) };
                if f.get("value").is_none_or(|v| string(v).is_empty()) {
                    return Some(format!("<em>{}</em>", esc_html(&(r.tr)("(empty)"))));
                }
                let value = self.formatted_field_value(&field_id, "value");
                let value = Self::sanitize_field_value(&value, f, context);
                let value = html_field_value(&value, f, self.form_data, "smart-tag", r);
                kses_post(&value)
            }
            "field_value_id" => {
                let Some(id) = attr(&attrs, "field_value_id").filter(|f| !f.is_empty()) else { return Some(String::new()) };
                let Some(f) = field(self.fields, id).filter(|f| f.as_str() != Some("")) else { return Some(String::new()) };
                let key = if f.get("value_raw").is_some_and(|v| !v.is_null() && !v.is_array() && !v.is_object() && !string(v).is_empty()) { "value_raw" } else { "value" };
                let value = self.formatted_field_value(&php::leading_int(id).to_string(), key);
                Self::sanitize_field_value(&value, f, context)
            }
            "form_id" => php::int(self.form_data.get("id")).abs().to_string(),
            "form_name" => {
                let title = self.form_data.get("settings").and_then(|s| s.get("form_title"));
                if title.is_none_or(|t| t.is_null() || string(t).is_empty()) {
                    String::new()
                } else {
                    neutralize(&string(title.unwrap_or(&Value::Null)))
                }
            }
            // Lite keeps no entry meta (`get_meta()` is empty): the tags read the request.
            "page_id" => match r.post.get("page_id") {
                Some(v) if !empty(Some(v)) => php::int(Some(v)).abs().to_string(),
                _ => String::new(),
            },
            "page_title" => match post("page_title").filter(|t| !t.is_empty() && t != "0") {
                Some(title) => neutralize(&title),
                None => neutralize(""),
            },
            "page_url" => {
                let url = match post("page_url").filter(|u| !u.is_empty() && u != "0") {
                    Some(u) => esc_url_raw(&u),
                    None => esc_url_raw(&format!("{}{}", r.origin, r.request_uri)),
                };
                esc_url(&super::redirection_php::urldecode(&url))
            }
            "url_referer" => {
                let referer = self.entry_meta.and_then(|m| m.get("url_referer")).map(string).unwrap_or_default();
                if !referer.is_empty() && referer != "0" {
                    let decoded = super::redirection_php::urldecode(&referer);
                    if context == "confirmation_redirect" { decoded } else { esc_url(&decoded) }
                } else {
                    String::new()
                }
            }
            "user_ip" => {
                if !collecting_ip_allowed(r, None) {
                    String::new()
                } else {
                    esc_html(r.remote_addr)
                }
            }
            "date" => {
                let at = r.now + (r.gmt_offset * 3600.0) as i64;
                match attr(&attrs, "format").filter(|f| !f.is_empty() && *f != "0") {
                    None => (r.date)(&r.date_format, at),
                    Some(f) => {
                        let format = if f.to_lowercase() == "timestamp" { "U" } else { f };
                        (r.date)(format, at)
                    }
                }
            }
            "query_var" => {
                let Some(key) = attr(&attrs, "key").filter(|k| !k.is_empty() && *k != "0") else { return Some(String::new()) };
                if let Some(v) = r.get.get(key).filter(|v| !empty(Some(v))) {
                    return Some(esc_html(&rustpress_content::sanitize::text_field(&super::fields::stripslashes(&string(v)))));
                }
                let page_url = post("page_url").filter(|u| !u.is_empty() && u != "0").unwrap_or_default();
                if page_url.is_empty() {
                    return Some(String::new());
                }
                let url = esc_url_raw(&super::fields::stripslashes(&page_url));
                let query = url.split_once('?').map(|(_, q)| q.split('#').next().unwrap_or_default().to_owned()).unwrap_or_default();
                if query.is_empty() {
                    return Some(String::new());
                }
                let params = super::redirection_php::parse_str(&query);
                match super::redirection_php::get(&params, key) {
                    Some(super::redirection_php::Param::Str(v)) if !v.is_empty() && v != "0" => esc_html(&rustpress_content::sanitize::text_field(&super::fields::stripslashes(v))),
                    _ => String::new(),
                }
            }
            "user_id" => self.user().map(|u| u.id.to_string()).unwrap_or_default(),
            "user_display" => self.user().map(|u| neutralize(&u.display_name)).unwrap_or_default(),
            "user_full_name" => self.user().map(|u| neutralize(&format!("{} {}", u.first_name, u.last_name))).unwrap_or_default(),
            "user_first_name" => self.user().map(|u| neutralize(&u.first_name)).unwrap_or_default(),
            "user_last_name" => self.user().map(|u| neutralize(&u.last_name)).unwrap_or_default(),
            "user_email" => self.user().map(|u| rustpress_content::sanitize::sanitize_email(&u.email)).unwrap_or_default(),
            "author_id" => {
                // The author of `$_POST['page_id']`, else the current user (see the request's `page_author`).
                let id = r.page_author.as_ref().map_or(r.user.as_ref().map_or(0, |u| u.id), |(id, _)| *id);
                if self.has_cap(context) && id != 0 { id.to_string() } else { String::new() }
            }
            "author_display" => {
                let name = r.page_author.as_ref().map(|(_, n)| n.clone()).or_else(|| r.user.as_ref().map(|u| u.display_name.clone())).unwrap_or_default();
                if self.has_cap(context) { neutralize(&name) } else { String::new() }
            }
            "url_login" => esc_url(&r.login_url),
            "url_register" => esc_url(&format!("{}?action=register", r.login_url)),
            "url_lost_password" => esc_url(&format!("{}?action=lostpassword", r.login_url)),
            "unique_value" => {
                let mut length = 16usize;
                if let Some(l) = attr(&attrs, "length") {
                    length = length.max(php::leading_int(l).unsigned_abs() as usize);
                }
                let format = attr(&attrs, "format").filter(|f| !f.is_empty() && *f != "0").unwrap_or("alphanumeric").to_lowercase();
                let chars: &[u8] = match format.as_str() {
                    "numeric" => b"0123456789",
                    "alpha" => b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
                    _ => b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
                };
                // `str_pad()` to the length, `str_shuffle()` and the first characters.
                let mut pool: Vec<u8> = chars.iter().copied().cycle().take(length.max(chars.len())).collect();
                use rand::seq::SliceRandom;
                pool.shuffle(&mut rand::rng());
                String::from_utf8_lossy(&pool[..length]).into_owned()
            }
            "site_name" => rustpress_admin::mail::specialchars_decode(&r.blogname),
            // `url_logout` needs a nonce of the user's session, `user_meta` and the payment tags data RustPress does
            // not read for a submission (see PLAN.md): they stay as written.
            _ => return None,
        };
        Some(value)
    }
}

/// `wpforms_is_collecting_ip_allowed( $form_data )`.
pub fn collecting_ip_allowed(r: &Request<'_>, form_data: Option<&Value>) -> bool {
    !(r.settings.set("gdpr") && (r.settings.set("gdpr-disable-details") || form_data.is_some_and(|f| set(&f["settings"], "disable_ip"))))
}

/// `WPForms_Field_Radio::get_other_choice_value()`.
fn radio_other_value(field: &Value) -> Option<String> {
    if !set(field, "is_other") {
        return None;
    }
    let value = str_of(field, "value");
    let raw = str_of(field, "value_raw");
    if raw.is_empty() {
        return Some(value);
    }
    if !value.is_empty() {
        return Some(format!("{raw}: {value}"));
    }
    Some(raw)
}

/// `apply_filters( 'wpforms_html_field_value', $value, $field, $form_data, $context )` of the Lite fields: the number
/// slider (`html_email_value()`), the "Other" choice of a radio field and the choices with images
/// (`WPForms_Field::field_html_value()`).
pub fn html_field_value(value: &str, field: &Value, form_data: &Value, context: &str, r: &Request<'_>) -> String {
    let ty = str_of(field, "type");
    match ty.as_str() {
        "number-slider" => {
            let raw = field.get("value_raw").filter(|v| !empty(Some(v)));
            let Some(raw) = raw else { return value.to_owned() };
            let v = raw.get("value").map_or(0.0, |v| php::to_float(Some(v)));
            let min = raw.get("min").map_or(0.0, |v| php::to_float(Some(v)));
            let max = raw.get("max").map_or(10.0, |v| php::to_float(Some(v)));
            let display = str_of(raw, "value_display");
            if display.contains("{value}") {
                let text = (r.tr)("%1$s (%2$s min / %3$s max)").replace("%1$s", &php::float(v)).replace("%2$s", &php::float(min)).replace("%3$s", &php::float(max));
                return display.replace("{value}", &esc_html_keep(&text));
            }
            php::float(v)
        }
        "radio" | "checkbox" => {
            let value = if ty == "radio" { radio_other_value(field).unwrap_or_else(|| value.to_owned()) } else { value.to_owned() };
            let form_field = form_data.get("fields").and_then(|f| f.get(string(field.get("id").unwrap_or(&Value::Null)))).cloned().unwrap_or(Value::Null);
            // `field_html_value_images()`.
            let images = !set(&form_field, "choices_images_hide") || context == "entry-preview";
            if empty(field.get("value")) || context == "entry-table" || !images {
                return value;
            }
            // `get_field_html()` with `wpforms_get_choices_value()`.
            let choices_value = || {
                let raw = field.get("value_raw").map(string).unwrap_or_default();
                if set(&form_field, "show_values") && !set(field, "dynamic") && !raw.is_empty() { raw } else { field.get("value").map(string).unwrap_or_default() }
            };
            let image = |url: &str, label: &str| format!("<span style=\"max-width:200px;display:block;margin:0 0 5px 0;\"><img src=\"{}\" style=\"max-width:100%;display:block;margin:0;\" alt=\"\"></span>{label}", esc_url(url));
            if set(field, "image") {
                return image(&str_of(field, "image"), &choices_value());
            }
            if set(field, "images") {
                let list = field["images"].clone();
                return choices_value()
                    .split('\n')
                    .enumerate()
                    .map(|(i, label)| match list.get(i).or_else(|| list.get(i.to_string())) {
                        Some(url) if !empty(Some(url)) => image(&string(url), label),
                        _ => label.to_owned(),
                    })
                    .collect();
            }
            value
        }
        _ => value.to_owned(),
    }
}

/// `esc_html__()` of a translation that has no markup.
fn esc_html_keep(text: &str) -> String {
    esc_html(text)
}
