//! The notification e-mails of a submission: `WPForms_Process::entry_email()` with `WPForms\Emails\Notifications`
//! (its `Mailer`), the e-mail templates (`Templates\Classic`, `Templates\Compact`, `Templates\Plain` on
//! `Templates\General`), the `{all_fields}` tag (`AllFieldsTag`) and the inlining of the styles (`Styler`).
//!
//! Not ported: the legacy template `default` (`WPForms_WP_Emails`, the Classic template is used instead), sending in
//! the background (`email-async`, the e-mails are sent right away), and the log of notifications that were not sent.

use std::path::Path;

use rustpress_content::sanitize::sanitize_email;
use rustpress_core::native::Email;
use serde_json::{Value, json};

use super::php::{self, empty, str_of, string};
use super::smart_tags::Tags;
use super::submit::{Request, decode_string};
use super::template::{self, Val};

/// `Notifications::DEFAULT_TEMPLATE`, `PLAIN_TEMPLATE` and `LEGACY_TEMPLATE`.
const DEFAULT_TEMPLATE: &str = "classic";
const PLAIN_TEMPLATE: &str = "none";

/// `trim()` with PHP's characters.
fn trim(s: &str) -> &str {
    s.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}'])
}

/// `WPForms_Process::entry_email()`: the e-mails of the submission, and whether a `Notifications` object was made
/// (its filter of the multiple-choice values stays on for the rest of the request: `Some( plain text )`).
pub fn entry_email(fields: &Value, form_data: &Value, r: &Request<'_>) -> (Vec<Email>, Option<bool>) {
    let settings = &form_data["settings"];
    let mut emails = Vec::new();
    let mut notification_hooks = None;
    if empty(settings.get("notification_enable")) {
        return (emails, None);
    }
    let notifications: Vec<(String, Value)> = if empty(settings.get("notifications")) && !empty(settings.get("notification_email")) {
        let legacy = |key: &str| settings.get(key).cloned().unwrap_or(Value::Null);
        vec![(
            "1".into(),
            json!({
                "email": legacy("notification_email"),
                "subject": legacy("notification_subject"),
                "sender_name": legacy("notification_fromname"),
                "sender_address": legacy("notification_fromaddress"),
                "replyto": legacy("notification_replyto"),
                "message": "{all_fields}",
            }),
        )]
    } else {
        match settings.get("notifications") {
            Some(Value::Object(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            Some(Value::Array(a)) => a.iter().enumerate().map(|(k, v)| (k.to_string(), v.clone())).collect(),
            _ => Vec::new(),
        }
    };
    // Lite sends the first notification only.
    for (_, notification) in notifications.into_iter().take(1) {
        if notification.get("enable").is_some_and(|e| !e.is_null()) && php::int(notification.get("enable")) == 0 {
            continue;
        }
        if empty(notification.get("email")) {
            continue;
        }
        let carbon_copy_enabled = r.settings.set("email-carbon-copy");
        let subject = if !empty(notification.get("subject")) {
            str_of(&notification, "subject")
        } else {
            esc_html_tr(r, "New %s Entry").replace("%s", &str_of(settings, "form_title"))
        };
        // `get_notification_send_to()`.
        let tags = Tags { request: r, form_data, fields, entry_meta: form_data.get("entry_meta"), notification: notification_hooks, recipients_can: false };
        let send_to = tags.process(&str_of(&notification, "email"), "notification-send-to-email", false);
        let addresses: Vec<String> = send_to.split(',').map(sanitize_email).filter(|a| !a.is_empty() && a != "0").collect();
        let pick = |key: &str| (!empty(notification.get(key))).then(|| str_of(&notification, key));
        let sender_address = pick("sender_address").unwrap_or_else(|| r.admin_email.clone());
        let sender_name = pick("sender_name").unwrap_or_else(|| r.blogname.clone());
        let reply_to = pick("replyto");
        let message = pick("message").unwrap_or_else(|| "{all_fields}".into());
        let template = pick("template").unwrap_or_default();
        let carbon_copy = if carbon_copy_enabled && !empty(notification.get("carboncopy")) {
            let tags = Tags { recipients_can: recipients_can(r, &addresses), ..tags };
            let list = tags.process(&str_of(&notification, "carboncopy"), "notification-carboncopy", false);
            list.split(',').map(sanitize_email).filter(|a| !a.is_empty() && a != "0").collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if addresses.is_empty() {
            continue;
        }
        // `( new Notifications() )->init( $email['template'] )`.
        let current_template = current_template_name(r, &template);
        let plain = current_template == PLAIN_TEMPLATE;
        notification_hooks = Some(plain);
        let mut mailer = Notifications {
            r,
            form_data,
            fields,
            current_template,
            plain,
            from_name: sender_name,
            from_address: sender_address,
            reply_to,
            cc: (carbon_copy_enabled && !carbon_copy.is_empty()).then(|| carbon_copy.join(",")),
            to_email: Vec::new(),
        };
        for address in &addresses {
            if let Some(email) = mailer.send(trim(address), &subject, &message) {
                emails.push(email);
            }
        }
    }
    (emails, notification_hooks)
}

/// `esc_html__( $text, 'wpforms-lite' )`.
fn esc_html_tr(r: &Request<'_>, text: &str) -> String {
    rustpress_core::formatting::esc_html(&(r.tr)(text))
}

/// `SmartTag::recipient_has_cap()`: every recipient is a user who can `manage_options`.
fn recipients_can(r: &Request<'_>, to: &[String]) -> bool {
    let mut emails: Vec<&str> = to.iter().map(|e| trim(e)).filter(|e| !e.is_empty() && *e != "0").collect();
    emails.dedup();
    !emails.is_empty() && emails.iter().all(|e| r.manager_emails.iter().any(|m| m.eq_ignore_ascii_case(e)))
}

/// `Helpers::get_current_template_name( $template_name )` of Lite: `classic`, `compact` or `none`.
fn current_template_name(r: &Request<'_>, name: &str) -> String {
    let available = |t: &str| matches!(t, "classic" | "compact" | "none");
    let settings_template = r.settings.string("email-template", DEFAULT_TEMPLATE);
    let mut template = if !name.is_empty() && name != "0" { trim(&rustpress_content::sanitize::text_field(name)).to_owned() } else { settings_template.clone() };
    if template == "default" {
        // The legacy template (`WPForms_WP_Emails`) is not ported: the default template takes its place.
        template = if settings_template == "default" { DEFAULT_TEMPLATE.to_owned() } else { settings_template.clone() };
    }
    if available(&template) {
        return template;
    }
    if available(&settings_template) { settings_template } else { DEFAULT_TEMPLATE.to_owned() }
}

/// A `WPForms\Emails\Notifications` object with what `entry_email()` sets on it.
struct Notifications<'a> {
    r: &'a Request<'a>,
    form_data: &'a Value,
    fields: &'a Value,
    current_template: String,
    plain: bool,
    from_name: String,
    from_address: String,
    reply_to: Option<String>,
    cc: Option<String>,
    to_email: Vec<String>,
}

impl Notifications<'_> {
    fn tags(&self) -> Tags<'_> {
        Tags {
            request: self.r,
            form_data: self.form_data,
            fields: self.fields,
            entry_meta: self.form_data.get("entry_meta"),
            notification: Some(self.plain),
            recipients_can: recipients_can(self.r, &self.to_email),
        }
    }

    /// `Notifications::process_tag()`.
    fn process_tag(&self, input: &str, context: &str) -> String {
        self.tags().process(input, context, false)
    }

    /// `Notifications::sanitize()`.
    fn sanitize(&self, input: &str, context: &str) -> String {
        decode_string(&self.process_tag(input, context))
    }

    /// `Mailer::get_from_name()`, which keeps what it returns.
    fn get_from_name(&mut self) -> String {
        self.from_name = if !self.from_name.is_empty() && self.from_name != "0" { self.sanitize(&self.from_name.clone(), "notification") } else { self.r.blogname.clone() };
        self.from_name.clone()
    }

    /// `Mailer::get_from_address()`.
    fn get_from_address(&self) -> String {
        let address = self.sanitize(&self.from_address, "notification-from");
        if address.is_empty() || address == "0" { self.r.admin_email.clone() } else { address }
    }

    /// `Notifications::get_reply_to_address()`.
    fn get_reply_to_address(&self) -> String {
        let Some(reply_to) = self.reply_to.as_ref().filter(|r| !r.is_empty() && *r != "0") else { return String::new() };
        static H: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        static NAMED: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let h = H.get_or_init(|| regex::Regex::new(r"[\t \x{a0}\x{1680}\x{180e}\x{2000}-\x{200a}\x{202f}\x{205f}\x{3000}]").expect("valid regex"));
        let named = NAMED.get_or_init(|| regex::Regex::new(r"^(.+) (<.+>)\n?$").expect("valid regex"));
        let mut reply_to = h.replace_all(reply_to, " ").into_owned();
        let mut name = None;
        if let Some(c) = named.captures(&reply_to) {
            name = Some(self.sanitize(&c[1], "notification"));
            reply_to = c[2].trim_matches(['<', '>', ' ']).to_owned();
        }
        let reply_to = self.process_tag(&reply_to, "notification-reply-to");
        if !rustpress_content::sanitize::is_email(&reply_to) {
            return String::new();
        }
        match name.filter(|n| !n.is_empty() && n != "0") {
            Some(n) => format!("{} <{reply_to}>", sanitize_email_header_name(&n)),
            None => reply_to,
        }
    }

    /// `Mailer::get_cc_address()`, which keeps what it returns.
    fn get_cc_address(&mut self) -> String {
        let Some(cc) = self.cc.clone().filter(|c| !c.is_empty() && c != "0") else { return String::new() };
        let cc = self.sanitize(&cc, "notification");
        let cc = cc.split(',').map(sanitize_email).filter(|a| !a.is_empty() && a != "0").collect::<Vec<_>>().join(",");
        self.cc = Some(cc.clone());
        cc
    }

    /// `Mailer::get_headers()`.
    fn get_headers(&mut self) -> String {
        let from_name = sanitize_email_header_name(&self.get_from_name());
        let from_address = sanitize_email_header(&self.get_from_address());
        let reply_to = sanitize_email_header(&self.get_reply_to_address());
        let cc = sanitize_email_header(&self.get_cc_address());
        let mut headers = format!("From: {from_name} <{from_address}>\r\n");
        if !reply_to.is_empty() && reply_to != "0" {
            headers.push_str(&format!("Reply-To: {reply_to}\r\n"));
        }
        if !cc.is_empty() && cc != "0" {
            headers.push_str(&format!("Cc: {cc}\r\n"));
        }
        headers.push_str(&format!("Content-Type: {}; charset=utf-8\r\n", if self.plain { "text/plain" } else { "text/html" }));
        headers
    }

    /// `Notifications::send( $to, $subject, $message )` and `Mailer::send()`: the arguments of `wp_mail()`, with
    /// `From` as the filters `wp_mail_from` and `wp_mail_from_name` leave it.
    fn send(&mut self, to: &str, subject: &str, message: &str) -> Option<Email> {
        if !rustpress_content::sanitize::is_email(to) {
            return None;
        }
        self.to_email = to.split(',').map(str::to_owned).collect();
        let headers = self.get_headers();
        // `process_subject()`, then `Mailer::subject()`.
        let subject = self.process_tag(subject, "notification");
        let subject = decode_string(trim(&subject.replace("\r\n", " ").replace(['\r', '\n'], " ")));
        let mut subject = self.sanitize(&subject, "notification");
        let body = self.process_email_template(message);
        if !self.to_email.iter().all(|e| rustpress_content::sanitize::is_email(e)) {
            return None;
        }
        if subject.is_empty() || subject == "0" {
            subject = (self.r.tr)("New Email Submit");
        }
        if body.is_empty() || body == "0" {
            return None;
        }
        // `wp_mail()` asks the filters for the sender while the message is sent.
        let from = format!("From: {} <{}>", self.get_from_name(), self.get_from_address());
        let headers = headers.split("\r\n").filter(|h| !h.is_empty()).map(|h| if h.starts_with("From:") { from.clone() } else { h.to_owned() }).collect();
        Some(Email { to: self.to_email.join(","), subject, body, headers, attachments: Vec::new() })
    }

    fn template_site(&self) -> Option<template::Site<'_>> {
        let dir = self.r.plugin_dir.as_deref()?;
        Some(template::Site {
            plugin_dir: dir,
            tr: self.r.tr,
            blogname: &self.r.blogname,
            charset: &self.r.charset,
            language_attributes: &self.r.language_attributes,
            rtl: self.r.rtl,
            home_url: &self.r.home_url,
        })
    }

    /// `General::get_full_template_name()`.
    fn template_name(&self, site: &template::Site<'_>, part: &str) -> String {
        let slug = match self.current_template.as_str() {
            "compact" => "compact",
            PLAIN_TEMPLATE => "plain",
            _ => "classic",
        };
        let part = if self.plain { format!("{part}-plain") } else { part.to_owned() };
        let name = format!("emails/{slug}-{part}");
        if template::exists(site, &name) { name } else { format!("emails/general-{part}") }
    }

    /// `Notifications::process_email_template()` and `General::get()`.
    fn process_email_template(&self, message: &str) -> String {
        let Some(site) = self.template_site() else { return String::new() };
        let field_template = template::get_html(&site, &self.template_name(&site, "field"), Vec::new());
        let message = self.process_message(message, &field_template);
        let settings = self.r.settings;
        let mut header = vec![("title".to_owned(), Val::str(esc_html_tr(self.r, "WPForms")))];
        if !self.plain || settings.string("email-template", DEFAULT_TEMPLATE) != PLAIN_TEMPLATE {
            let setting = |key: &str, default: &str| Val::str(settings.string(key, default));
            header.push((
                "header_image".into(),
                Val::Arr(vec![
                    ("url_light".into(), setting("email-header-image", "")),
                    ("size_light".into(), setting("email-header-image-size", "medium")),
                    ("url_dark".into(), setting("email-header-image-dark", "")),
                    ("size_dark".into(), setting("email-header-image-size-dark", "medium")),
                ]),
            ));
        }
        let content = [
            template::get_html(&site, &self.template_name(&site, "header"), header),
            template::get_html(&site, &self.template_name(&site, "body"), vec![("message".into(), Val::Str(message))]),
            template::get_html(&site, &self.template_name(&site, "footer"), Vec::new()),
        ]
        .concat();
        if content.is_empty() {
            return String::new();
        }
        if self.plain {
            return rustpress_core::formatting::wp_strip_all_tags(&content);
        }
        let content = rustpress_core::text::make_clickable(&content, &rustpress_core::text::Clickable { home_host: &self.r.home_host, comment: false });
        // `Styler::process_markup()`.
        let overrides = style_overrides(settings);
        let style = template::get_html(&site, &self.template_name(&site, "style"), overrides.clone());
        let queries = template::get_html(&site, &self.template_name(&site, "queries"), overrides);
        let styled = super::inline::convert(&content, &style);
        styled.replace("</head>", &format!("<style type=\"text/css\">{queries}</style>\n</head>"))
    }

    /// `Notifications::process_message()`.
    fn process_message(&self, message: &str, field_template: &str) -> String {
        let message = self.process_tag(message, "notification");
        let message = self.replace_all_fields_tag(&message, field_template);
        let message = fix_table_body_markup(&message);
        if self.plain {
            return message;
        }
        rustpress_core::text::make_clickable(&message.replace("\r\n", "<br/>"), &rustpress_core::text::Clickable { home_host: &self.r.home_host, comment: false })
    }

    /// `Notifications::replace_all_fields_tag()` with `AllFieldsTag::replace()`.
    fn replace_all_fields_tag(&self, message: &str, field_template: &str) -> String {
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| regex::Regex::new(r"\{all_fields([ =][^\n}]*)?\}").expect("valid regex"));
        let mut cache: Vec<(Option<Exclude>, String)> = Vec::new();
        re.replace_all(message, |c: &regex::Captures<'_>| {
            let options = parse_exclude(c.get(1).map_or("", |m| m.as_str()));
            if let Some((_, v)) = cache.iter().find(|(o, _)| *o == options) {
                return v.clone();
            }
            let rendered = self.process_field_values(options.as_ref(), field_template);
            cache.push((options, rendered.clone()));
            rendered
        })
        .into_owned()
    }

    /// `Notifications::process_field_values()`.
    fn process_field_values(&self, exclude: Option<&Exclude>, field_template: &str) -> String {
        if empty(Some(self.fields)) {
            return String::new();
        }
        let message = if self.plain { self.process_plain_message(exclude) } else { self.process_html_message(exclude, field_template) };
        if message.is_empty() { esc_html_tr(self.r, "An empty form was submitted.") } else { message }
    }

    fn form_fields(&self) -> Vec<Value> {
        match self.form_data.get("fields") {
            Some(Value::Object(m)) => m.values().cloned().collect(),
            Some(Value::Array(a)) => a.clone(),
            _ => Vec::new(),
        }
    }

    /// `Notifications::get_default_field_name()`.
    fn default_field_name(&self, id: &Value) -> String {
        esc_html_tr(self.r, "Field ID #%1$s").replace("%1$s", &php::leading_int(&string(id)).abs().to_string())
    }

    /// `Notifications::process_plain_message()` with `get_field_plain()`.
    fn process_plain_message(&self, exclude: Option<&Exclude>) -> String {
        let mut message = String::new();
        for form_field in self.form_fields() {
            if is_excluded(&form_field, exclude) {
                continue;
            }
            let id = form_field.get("id").cloned().unwrap_or(json!(""));
            let field = self.fields.get(string(&id)).cloned().unwrap_or(form_field);
            let Some(value) = field.get("value").filter(|v| !v.is_null() && !string(v).is_empty()) else { continue };
            let name = field.get("name").map(string).unwrap_or_default();
            let value = if empty(Some(value)) && !php::is_numeric(Some(value)) { esc_html_tr(self.r, "(empty)") } else { string(value) };
            let name = if name.is_empty() || name == "0" { self.default_field_name(field.get("id").unwrap_or(&id)) } else { name };
            message.push_str(&format!("--- {name} ---\r\n\r\n{}\r\n\r\n", decode_string(&value)));
        }
        message.trim_end_matches(['\r', '\n']).to_owned()
    }

    /// `Notifications::process_html_message()` with `get_field_html()`.
    fn process_html_message(&self, exclude: Option<&Exclude>, field_template: &str) -> String {
        let mut message = String::new();
        for form_field in self.form_fields() {
            if is_excluded(&form_field, exclude) {
                continue;
            }
            let ty = str_of(&form_field, "type");
            let id = form_field.get("id").cloned().unwrap_or(json!(""));
            let Some(field) = self.fields.get(string(&id)).filter(|f| !empty(Some(f))) else { continue };
            let Some(value) = field.get("value").filter(|v| !v.is_null() && !string(v).is_empty()) else { continue };
            let name = field.get("name").map(string).unwrap_or_default();
            let value = if empty(Some(value)) && !php::is_numeric(Some(value)) {
                format!("<em>{}</em>", esc_html_tr(self.r, "(empty)"))
            } else {
                string(value)
            };
            let name = if name.is_empty() || name == "0" { self.default_field_name(&id) } else { name };
            let value = super::smart_tags::html_field_value(&value, field, self.form_data, "email-html", self.r);
            let value = rustpress_core::kses::kses_post(&value).replace("\r\n", "<br/>").replace(['\r', '\n'], "<br/>");
            let html = field_template.replace("{field_type}", &ty).replace("{field_name}", &name).replace("{field_value}", &value);
            message.push_str(trim(&html));
        }
        message
    }
}

/// The options of `{all_fields exclude="…"}` (`AllFieldsTag::parse()`).
#[derive(Debug, Clone, PartialEq)]
struct Exclude {
    ids: Vec<i64>,
    hidden: bool,
}

fn parse_exclude(params: &str) -> Option<Exclude> {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    static ID: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| fancy_regex::Regex::new(r#"(?:^|\s)exclude=(?:(["'])(.+?)\1|([^\s"'}]+))"#).expect("valid regex"));
    let id_re = ID.get_or_init(|| regex::Regex::new(r"^\d+(?:_\d+)?$").expect("valid regex"));
    let c = re.captures(params).ok().flatten()?;
    let list = c.get(3).or_else(|| c.get(2)).map_or("", |m| m.as_str());
    let mut options = Exclude { ids: Vec::new(), hidden: false };
    for token in list.split(',') {
        let token = trim(token).to_lowercase();
        if token == "hidden" {
            options.hidden = true;
            continue;
        }
        if id_re.is_match(&token) {
            let id = php::leading_int(&token).abs();
            if !options.ids.contains(&id) {
                options.ids.push(id);
            }
        }
    }
    (!options.ids.is_empty() || options.hidden).then_some(options)
}

/// `AllFieldsTag::is_excluded()` (Lite has no layout or repeater fields for `expand()`).
fn is_excluded(field: &Value, options: Option<&Exclude>) -> bool {
    let Some(options) = options else { return false };
    if options.hidden && str_of(field, "type") == "hidden" {
        return true;
    }
    options.ids.contains(&php::leading_int(&field.get("id").map(string).unwrap_or_default()).abs())
}

/// `Notifications::fix_table_body_markup()`: what is not a table row is wrapped in one.
fn fix_table_body_markup(content: &str) -> String {
    let content = trim(content);
    let html = format!("<!DOCTYPE html><html><head><meta charset=\"UTF-8\"></head><body>{}</body></html>", rustpress_core::kses::pre_kses_less_than(content));
    let dom = super::dom::parse(&html, super::dom::Options { no_implied: true });
    let Some(body) = dom.elements().into_iter().find(|&e| dom.name(e) == Some("body")) else { return wrap_with_row(content) };
    let mut modified = String::new();
    let mut to_wrap = String::new();
    for &child in &dom.nodes[body].children {
        let text = match &dom.nodes[child].kind {
            super::dom::Kind::Text(t) => t.clone(),
            _ => dom.save(child, false),
        };
        if dom.name(child) != Some("tr") {
            to_wrap.push_str(&text);
            continue;
        }
        modified.push_str(&wrap_with_row(&to_wrap));
        modified.push_str(&text);
        to_wrap.clear();
    }
    if !to_wrap.is_empty() {
        modified.push_str(&wrap_with_row(&to_wrap));
    }
    modified
}

/// `Notifications::wrap_content_with_row()`.
fn wrap_with_row(content: &str) -> String {
    let content = trim(content);
    if content.is_empty() {
        return String::new();
    }
    format!("<tr class=\"smart-tag\"><td class=\"field-name field-value\" colspan=\"2\">{content}</td></tr>")
}

/// `Mailer::sanitize_email_header()`.
fn sanitize_email_header(value: &str) -> String {
    trim(&value.replace("\r\n", " ").replace(['\r', '\n'], " ")).to_owned()
}

/// `Mailer::sanitize_email_header_name()`.
fn sanitize_email_header_name(name: &str) -> String {
    trim(&sanitize_email_header(name).replace([',', '<', '>'], "")).to_owned()
}

/// `Helpers::get_current_template_style_overrides()` of Lite.
fn style_overrides(settings: &super::settings::Settings) -> Vec<(String, Val)> {
    const SANS_SERIF: &str = "-apple-system, BlinkMacSystemFont, avenir next, avenir, segoe ui, helvetica neue, helvetica, Cantarell, Ubuntu, roboto, noto, arial, sans-serif";
    let size = |key: &str| match settings.string(key, "medium").as_str() {
        "small" => ("240", "120"),
        "large" => ("500", "240"),
        _ => ("350", "180"),
    };
    let (width, height) = size("email-header-image-size");
    let (width_dark, height_dark) = size("email-header-image-size-dark");
    [
        ("email_background_color", settings.string("email-background-color", "#e9eaec")),
        ("email_body_color", "#ffffff".into()),
        ("email_text_color", "#333333".into()),
        ("email_links_color", "#e27730".into()),
        ("email_background_color_dark", settings.string("email-background-color-dark", "#2d2f31")),
        ("email_body_color_dark", "#1f1f1f".into()),
        ("email_text_color_dark", "#dddddd".into()),
        ("email_links_color_dark", "#e27730".into()),
        ("email_typography", SANS_SERIF.into()),
        ("email_typography_dark", SANS_SERIF.into()),
        ("header_image_max_width", width.into()),
        ("header_image_max_height", height.into()),
        ("header_image_max_width_dark", width_dark.into()),
        ("header_image_max_height_dark", height_dark.into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), Val::Str(v)))
    .collect()
}

/// `WPFORMS_PLUGIN_DIR` of a site.
pub fn plugin_dir(wp_content: &Path) -> std::path::PathBuf {
    wp_content.join("plugins").join("wpforms-lite")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_what_is_no_row() {
        assert_eq!(fix_table_body_markup("Hello <b>x</b>\n<tr><td>a</td></tr> tail"), "<tr class=\"smart-tag\"><td class=\"field-name field-value\" colspan=\"2\">Hello <b>x</b></td></tr><tr><td>a</td></tr><tr class=\"smart-tag\"><td class=\"field-name field-value\" colspan=\"2\">tail</td></tr>");
        assert_eq!(parse_exclude(" exclude=\"1, hidden,2_3\""), Some(Exclude { ids: vec![1, 2], hidden: true }));
        assert_eq!(parse_exclude(""), None);
    }
}
