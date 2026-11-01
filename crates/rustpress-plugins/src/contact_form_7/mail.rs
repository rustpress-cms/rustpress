//! `WPCF7_Mail` (`includes/mail.php`, `includes/mail-tag.php`, `includes/special-mail-tags.php`): the mail tags of a
//! template replaced by what was submitted, and the arguments of `wp_mail()` for a mail template.

use std::sync::LazyLock;

use regex::{Captures, Regex};
use rustpress_core::formatting::{esc_html, wptexturize};
use rustpress_core::native::Email;

use super::form_tag::FormTag;
use super::formatter::{Att, Atts, autop, format_atts, php_trim};
use super::posted::{PostData, Posted, flat_join};

/// The special mail tags (`[_site_title]`, `[_remote_ip]`, ...) of a submission, read before the mail is composed.
#[derive(Debug, Clone, Default)]
pub struct Special {
    /// There is a submission (the tags of the submission are empty without one).
    pub submission: bool,
    pub remote_ip: String,
    pub user_agent: String,
    pub url: String,
    /// `wp_date()` of the time of the submission in `date_format` and `time_format`.
    pub date: String,
    pub time: String,
    pub invalid_fields: usize,
    pub form_title: String,
    /// The post the form was on (`_wpcf7_container_post`).
    pub post: Option<PostInfo>,
    /// The options `blogname` and `blogdescription` as stored.
    pub site_title: String,
    pub site_description: String,
    pub site_url: String,
    pub admin_email: String,
    /// The signed-in user who submitted.
    pub user: Option<UserInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct PostInfo {
    pub id: u64,
    pub name: String,
    pub title: String,
    pub url: String,
    pub author: String,
    pub author_email: String,
}

#[derive(Debug, Clone, Default)]
pub struct UserInfo {
    pub login: String,
    pub email: String,
    pub url: String,
    pub first_name: String,
    pub last_name: String,
    pub nickname: String,
    pub display_name: String,
}

/// `wp_specialchars_decode( $text, ENT_QUOTES )`.
fn specialchars_decode(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#039;", "'").replace("&#x27;", "'").replace("&amp;", "&")
}

impl Special {
    /// The filters of `wpcf7_special_mail_tags`: the value of `name`, `None` when it is no special tag.
    pub fn value(&self, name: &str, html: bool) -> Option<String> {
        let name = name.strip_prefix("wpcf7.").map_or_else(|| name.to_owned(), |rest| format!("_{rest}"));
        let escaped = |text: &str| if html { esc_html(text) } else { text.to_owned() };
        // `wpcf7_special_mail_tag()`.
        if self.submission {
            match name.as_str() {
                "_remote_ip" => return Some(self.remote_ip.clone()),
                "_user_agent" => return Some(escaped(&self.user_agent)),
                "_url" => return Some(self.url.clone()),
                "_date" => return Some(self.date.clone()),
                "_time" => return Some(self.time.clone()),
                "_invalid_fields" => return Some(self.invalid_fields.to_string()),
                "_contact_form_title" => return Some(escaped(&self.form_title)),
                _ => {}
            }
        }
        // `wpcf7_post_related_smt()`.
        if name.starts_with("_post_") && self.submission {
            let Some(post) = &self.post else { return Some(String::new()) };
            return match name.as_str() {
                "_post_id" => Some(post.id.to_string()),
                "_post_name" => Some(post.name.clone()),
                "_post_title" => Some(escaped(&post.title)),
                "_post_url" => Some(post.url.clone()),
                "_post_author" => Some(post.author.clone()),
                "_post_author_email" => Some(post.author_email.clone()),
                _ => None,
            };
        }
        // `wpcf7_site_related_smt()`: `get_bloginfo( $show, 'display' )` for HTML.
        let bloginfo = |text: &str| if html { esc_html(&wptexturize(text)) } else { specialchars_decode(text) };
        match name.as_str() {
            "_site_title" => return Some(bloginfo(&self.site_title)),
            "_site_description" => return Some(bloginfo(&self.site_description)),
            "_site_url" => return Some(self.site_url.clone()),
            "_site_domain" => {
                let host = self.site_url.split_once("://").map(|(_, rest)| rest.split(['/', ':', '?', '#']).next().unwrap_or_default().to_owned()).unwrap_or_default();
                return Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host));
            }
            "_site_admin_email" => return Some(self.admin_email.clone()),
            _ => {}
        }
        // `wpcf7_user_related_smt()`.
        let user_tag = ["_user_login", "_user_email", "_user_url", "_user_first_name", "_user_last_name", "_user_nickname", "_user_display_name"].contains(&name.as_str());
        if user_tag && self.submission {
            let Some(user) = &self.user else { return Some(String::new()) };
            return Some(
                match name.as_str() {
                    "_user_login" => &user.login,
                    "_user_email" => &user.email,
                    "_user_url" => &user.url,
                    "_user_first_name" => &user.first_name,
                    "_user_last_name" => &user.last_name,
                    "_user_nickname" => &user.nickname,
                    _ => &user.display_name,
                }
                .clone(),
            );
        }
        None
    }
}

/// The texts of the mail, in the language of the form.
pub struct Lang<'a> {
    /// `__( $text, 'contact-form-7' )`.
    pub tr: &'a (dyn Fn(&str) -> String + Sync),
    /// `_x( $text, $context, 'contact-form-7' )`.
    pub trx: &'a (dyn Fn(&str, &str) -> String + Sync),
    /// `wp_get_list_item_separator()`.
    pub list_separator: String,
}

/// What the mail tags are replaced with.
pub struct Tags<'a> {
    pub tags: &'a [FormTag],
    /// `$submission->get_posted_data()`; `None` without a submission.
    pub posted: Option<&'a PostData>,
    /// `$_POST`, for `[_raw_name]`.
    pub post: &'a PostData,
    /// The names of the files uploaded through each field.
    pub uploaded: &'a [(String, Vec<String>)],
    pub special: &'a Special,
    pub lang: &'a Lang<'a>,
    /// `wp_date( $format, date_create( $value, wp_timezone() )->getTimestamp() )` for `[_format_name "..."]`.
    pub format_date: &'a (dyn Fn(&str, &str) -> Option<String> + Sync),
}

/// The component of the template being composed (`WPCF7_Mail::get_current_component_name()`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Component {
    None,
    Subject,
    Sender,
    Body,
    Recipient,
    AdditionalHeaders,
}

/// `wpcf7_mail_replace_tags( $content, $options )`.
pub fn replace_tags(content: &str, html: bool, exclude_blank: bool, component: Component, t: &Tags<'_>) -> String {
    let mut lines = Vec::new();
    for line in content.split('\n') {
        let mut replaced_tags: Vec<String> = Vec::new();
        let replaced = replace_line(line, html, component, t, &mut replaced_tags);
        if exclude_blank && !replaced_tags.is_empty() && replaced_tags.iter().all(String::is_empty) {
            continue;
        }
        lines.push(replaced);
    }
    lines.join("\n")
}

/// `WPCF7_MailTaggedText::replace_tags()` of one line; the values it put in go to `replaced`.
fn replace_line(line: &str, html: bool, component: Component, t: &Tags<'_>, replaced: &mut Vec<String>) -> String {
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(\[?)\[[\t ]*([a-zA-Z_][0-9a-zA-Z:._-]*)((?:[\t ]+"[^"]*"|[\t ]+'[^']*')*)[\t ]*\](\]?)"#).unwrap());
    static VALUE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]*"|'[^']*'"#).unwrap());
    TAG.replace_all(line, |c: &Captures<'_>| {
        if &c[1] == "[" && &c[4] == "]" {
            let whole = &c[0];
            return whole[1..whole.len() - 1].to_owned();
        }
        let tag_name = &c[2];
        let values: Vec<String> = VALUE.find_iter(&c[3]).map(|m| m.as_str()[1..m.as_str().len() - 1].to_owned()).collect();
        // `WPCF7_MailTag`: `_raw_name` and `_format_name` stand for the field `name`.
        let (name, raw, format) = if let Some(rest) = tag_name.strip_prefix("_raw_").filter(|r| !r.is_empty()) {
            (php_trim(rest).to_owned(), true, None)
        } else if let Some(rest) = tag_name.strip_prefix("_format_").filter(|r| !r.is_empty()) {
            (php_trim(rest).to_owned(), false, Some(values.first().cloned().unwrap_or_default()))
        } else {
            (tag_name.to_owned(), false, None)
        };
        let field = name.replace('.', "_");
        let submitted: Option<Posted> = if raw { Some(super::posted::clean(&t.post.get(&field).cloned().unwrap_or(Posted::Str(String::new())))) } else { t.posted.and_then(|p| p.get(&field)).cloned() };
        let mut value: Option<String> = submitted.as_ref().map(|submitted| {
            let mut items = submitted.flatten();
            if let Some(format) = format.as_deref().filter(|f| !f.is_empty()) {
                static DATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$").unwrap());
                for item in items.iter_mut() {
                    if DATE.is_match(item)
                        && let Some(formatted) = (t.format_date)(item, format)
                    {
                        *item = formatted;
                    }
                }
            }
            let separator = if component == Component::Body { t.lang.list_separator.as_str() } else { ", " };
            let joined = flat_join(&items, separator);
            if html { wptexturize(&esc_html(&joined)) } else { joined }
        });
        // `wpcf7_mail_tag_replaced_{$type}` of the form-tag with that name.
        if let Some(form_tag) = t.tags.iter().find(|tag| tag.name == field && !super::form_tag::features(&tag.tag_type).zero_controls_container) {
            match form_tag.tag_type.as_str() {
                "acceptance" => {
                    let consented = submitted.as_ref().is_some_and(Posted::is_truthy);
                    let mut text = (t.lang.tr)(if consented { "Consented" } else { "Not consented" });
                    let content = if form_tag.content.is_empty() { form_tag.values.first().cloned().unwrap_or_default() } else { form_tag.content.clone() };
                    let content = if html { content } else { rustpress_core::formatting::wp_strip_all_tags(&content) };
                    let content = php_trim(&content);
                    if !content.is_empty() && content != "0" {
                        text = (t.lang.trx)("%1$s: %2$s", "mail output for acceptance checkboxes").replacen("%1$s", &text, 1).replacen("%2$s", content, 1);
                    }
                    value = Some(text);
                }
                "file" | "file*" => {
                    if let Some((_, files)) = t.uploaded.iter().find(|(n, _)| *n == field).filter(|(_, files)| !files.is_empty()) {
                        value = Some(flat_join(files, &t.lang.list_separator));
                    }
                }
                _ => {}
            }
        }
        if let Some(value) = value {
            let value = php_trim(&value).to_owned();
            replaced.push(value.clone());
            return value;
        }
        if let Some(special) = t.special.value(tag_name, html) {
            replaced.push(special.clone());
            return special;
        }
        c[0].to_owned()
    })
    .into_owned()
}

/// A mail template (`mail`, `mail_2`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Template {
    pub subject: String,
    pub sender: String,
    pub body: String,
    pub recipient: String,
    pub additional_headers: String,
    pub attachments: String,
    pub use_html: bool,
    pub exclude_blank: bool,
    pub active: bool,
}

impl Template {
    /// The template as the property stores it (a serialized array).
    pub fn from_php(value: &rustpress_phpser::Value) -> Self {
        let get = |key: &str| value.get(key).map(php_string).unwrap_or_default();
        let flag = |key: &str| value.get(key).is_some_and(php_truthy);
        Self {
            subject: get("subject"),
            sender: get("sender"),
            body: get("body"),
            recipient: get("recipient"),
            additional_headers: get("additional_headers"),
            attachments: get("attachments"),
            use_html: flag("use_html"),
            exclude_blank: flag("exclude_blank"),
            active: flag("active"),
        }
    }
}

/// `(string) $value` of a stored property.
pub fn php_string(value: &rustpress_phpser::Value) -> String {
    use rustpress_phpser::Value;
    match value {
        Value::Str(bytes) => String::from_utf8_lossy(bytes).into_owned(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => super::formatter::php_float(*f),
        Value::Bool(true) => "1".to_owned(),
        _ => String::new(),
    }
}

/// `! empty( $value )`.
pub fn php_truthy(value: &rustpress_phpser::Value) -> bool {
    use rustpress_phpser::Value;
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty() && s.as_slice() != b"0",
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

/// `wpcf7_strip_newline()`.
fn strip_newline(text: &str) -> String {
    super::formatter::strip_whitespaces(&text.replace(['\r', '\n'], ""), super::formatter::Side::Both).to_owned()
}

/// The parts of a mail besides the tags: the language of the form and the files it may attach.
pub struct Compose<'a> {
    pub tags: &'a Tags<'a>,
    /// The locale of the form, for the `<html>` of an HTML mail.
    pub locale: &'a str,
    /// The uploaded files by field: name, type and content.
    pub files: &'a [(String, Vec<rustpress_core::native::Attachment>)],
    /// A file named by a line of the attachments (relative to `wp-content`), when it is readable there.
    pub content_file: &'a (dyn Fn(&str) -> Option<rustpress_core::native::Attachment> + Sync),
}

/// `WPCF7_Mail::compose()`: the arguments of `wp_mail()` for a template.
pub fn compose(template: &Template, c: &Compose<'_>) -> Email {
    let t = c.tags;
    let plain = |text: &str, component: Component| replace_tags(text, false, false, component, t);
    let subject = plain(&template.subject, Component::Subject);
    let sender = plain(&template.sender, Component::Sender);
    let mut body = replace_tags(&template.body, template.use_html, template.exclude_blank, Component::Body, t);
    if template.use_html {
        body = html_body(&body, &subject, c.locale);
    }
    let recipient = plain(&template.recipient, Component::Recipient);
    // `additional_headers()`: the first line of each header line that has a colon.
    let mut additional: Vec<String> = Vec::new();
    for line in template.additional_headers.split('\n').filter(|l| l.contains(':')) {
        let line = plain(line, Component::AdditionalHeaders).replace("\r\n", "\n");
        additional.push(line.split('\n').next().unwrap_or_default().to_owned());
    }
    let mut headers = vec![format!("From: {}", strip_newline(&sender))];
    if template.use_html {
        headers.push("Content-Type: text/html".to_owned());
        headers.push("X-WPCF7-Content-Type: text/html".to_owned());
    } else {
        headers.push("X-WPCF7-Content-Type: text/plain".to_owned());
    }
    for header in additional.join("\n").replace("\r\n", "\n").split('\n') {
        headers.push(php_trim(header).to_owned());
    }
    // `attachments()`: the files of the fields the template names, then the files it names itself (at most 25 MB).
    let mut attachments: Vec<rustpress_core::native::Attachment> = Vec::new();
    for (name, files) in c.files {
        if template.attachments.contains(&format!("[{name}]")) {
            attachments.extend(files.iter().cloned());
        }
    }
    for line in template.attachments.split('\n') {
        let line = php_trim(line);
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        if let Some(file) = (c.content_file)(line) {
            attachments.push(file);
        }
    }
    let mut total = 0;
    attachments.retain(|file| {
        total += file.bytes.len();
        total <= 25 * 1024 * 1024
    });
    Email { to: strip_newline(&recipient), subject: strip_newline(&subject), body, headers, attachments }
}

/// The body of an HTML mail: `<addr@example.org>` escaped, wrapped in a document by `htmlize()`.
fn html_body(body: &str, subject: &str, locale: &str) -> String {
    static ANGLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<(.*?)>").unwrap());
    static DOCUMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<html[>\s].*</html>").unwrap());
    let body = ANGLE.replace_all(body, |c: &Captures<'_>| if rustpress_content::sanitize::is_email(&c[1]) { format!("&lt;{}&gt;", &c[1]) } else { c[0].to_owned() }).into_owned();
    if DOCUMENT.is_match(&body) {
        return body;
    }
    let lang = if locale.is_empty() {
        String::new()
    } else {
        let mut atts = Atts::default();
        atts.set("dir", Att::str(if super::form::is_rtl_locale(locale) { "rtl" } else { "ltr" }));
        atts.set("lang", Att::str(locale.replace('_', "-")));
        format!(" {}", format_atts(&atts))
    };
    format!(
        "<!doctype html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\"{lang}>\n<head>\n<title>{}</title>\n</head>\n<body>\n{}</body>\n</html>",
        esc_html(subject),
        autop(&body)
    )
}
