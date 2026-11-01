//! `WPCF7_ContactForm` (`includes/contact-form.php`): a contact form as the post type `wpcf7_contact_form` stores it,
//! and `form_html()`, the HTML the shortcode prints.

use std::sync::LazyLock;

use regex::Regex;
use rustpress_core::formatting::{esc_html, esc_url};
use rustpress_phpser::Value;

use super::controls::{self, Render};
use super::form_tag;
use super::formatter::{Att, Atts, HtmlFormatter, autop, format_atts};

pub const POST_TYPE: &str = "wpcf7_contact_form";

/// A contact form: the post and the properties in its meta (`_form`, `_mail`, ...).
#[derive(Debug, Clone, Default)]
pub struct ContactForm {
    pub id: u64,
    pub name: String,
    pub title: String,
    pub status: String,
    pub locale: String,
    pub hash: String,
    pub form: String,
    pub mail: Option<Value>,
    pub mail_2: Option<Value>,
    pub messages: Option<Value>,
    pub additional_settings: String,
    /// `_old_cf7_unit_id`, for the old shortcode `[contact-form 1 "title"]`.
    pub old_unit_id: Option<String>,
}

impl ContactForm {
    /// The form from its post and meta rows (`(key, value)`), like the constructor reads them: `_name` first, else
    /// `name`.
    pub fn from_post(post: &rustpress_db::Post, meta: &[(String, String)]) -> Self {
        let get = |key: &str| meta.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
        let prop = |name: &str| get(&format!("_{name}")).or_else(|| get(name));
        let parsed = |name: &str| prop(name).map(|raw| rustpress_phpser::unserialize(raw.as_bytes()).unwrap_or(Value::Str(raw.into_bytes())));
        Self {
            id: post.id,
            name: post.post_name.clone(),
            title: post.post_title.clone(),
            status: post.post_status.clone(),
            locale: get("_locale").unwrap_or_default(),
            hash: get("_hash").unwrap_or_default(),
            form: prop("form").unwrap_or_default(),
            mail: parsed("mail"),
            mail_2: parsed("mail_2"),
            messages: parsed("messages"),
            additional_settings: prop("additional_settings").unwrap_or_default(),
            old_unit_id: get("_old_cf7_unit_id"),
        }
    }

    /// `hash( $length )`.
    pub fn short_hash(&self, length: usize) -> &str {
        &self.hash[..self.hash.len().min(length)]
    }

    /// `additional_setting( $name, $max )`: the values of `name: value` lines of the additional settings.
    pub fn additional_setting(&self, name: &str, max: usize) -> Vec<String> {
        static LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([a-zA-Z0-9_]+)[\t ]*:(.*)$").unwrap());
        let mut values = Vec::new();
        for setting in self.additional_settings.split('\n') {
            if let Some(c) = LINE.captures(setting) {
                if &c[1] != name {
                    continue;
                }
                if max == 0 || values.len() < max {
                    values.push(super::formatter::php_trim(&c[2]).to_owned());
                }
            }
        }
        values
    }

    /// `is_true( $name )`.
    pub fn is_true(&self, name: &str) -> bool {
        self.additional_setting(name, 1).first().is_some_and(|v| ["on", "true", "1"].contains(&v.as_str()))
    }
}

/// `wpcf7_is_rtl( $locale )`.
pub fn is_rtl_locale(locale: &str) -> bool {
    ["ar", "ary", "azb", "fa_IR", "haz", "he_IL", "ps", "ug_CN"].contains(&locale)
}

/// The options of `form_html()` (the attributes of the shortcode).
#[derive(Debug, Clone, Default)]
pub struct HtmlOptions {
    pub html_id: String,
    pub html_name: String,
    pub html_title: String,
    pub html_class: String,
    pub raw_form: bool,
}

/// Where the form is printed.
pub struct Place<'a> {
    /// `wpcf7_get_request_uri()`.
    pub request_uri: &'a str,
    /// `home_url()`.
    pub home: &'a str,
    /// `get_the_ID()` while `in_the_loop()`.
    pub loop_post: Option<u64>,
    /// `get_option( 'html_type' )`.
    pub html_type: &'a str,
    /// The number of the form among the forms of the request (`static $global_count`).
    pub count: u64,
    /// `is_user_logged_in()` and `wpcf7_create_nonce()` for the user.
    pub nonce: Option<String>,
    pub logged_in: bool,
    /// `WPCF7_VERSION`.
    pub version: &'a str,
}

/// `sanitize_html_class()` and the classes of the form element.
fn form_class(options: &HtmlOptions, demo: bool) -> String {
    let mut class = "wpcf7-form init".to_owned();
    if !options.html_class.is_empty() && options.html_class != "0" {
        class.push(' ');
        class.push_str(&options.html_class);
    }
    if demo {
        class.push_str(" demo");
    }
    let mut unique: Vec<String> = Vec::new();
    for c in class.split(' ').map(rustpress_core::formatting::sanitize_html_class) {
        if !c.is_empty() && c != "0" && !unique.contains(&c) {
            unique.push(c);
        }
    }
    unique.join(" ")
}

/// `WPCF7_ContactForm::form_html()`; `r.tr` translates in the language of the form (`wpcf7_switch_locale()`).
pub fn form_html(form: &ContactForm, options: &HtmlOptions, place: &Place<'_>, r: &mut Render<'_>) -> String {
    static ID_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9:._-]").unwrap());
    static LANG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^([a-z]+-[a-z]+)-").unwrap());
    if options.raw_form {
        return format!("<pre class=\"wpcf7-raw-form\"><code>{}</code></pre>", esc_html(&form.form));
    }
    if form.is_true("subscribers_only") && !place.logged_in {
        let text = (r.tr)("This contact form is available only for logged in users.");
        return format!("<p class=\"wpcf7-subscribers-only\">{}</p>", rustpress_core::kses::kses_data(&text));
    }
    let unit_tag = match place.loop_post {
        Some(post) => format!("wpcf7-f{}-p{post}-o{}", form.id, place.count),
        None => format!("wpcf7-f{}-o{}", form.id, place.count),
    };
    let mut action_url = place.request_uri.to_owned();
    if let Some(at) = action_url.find('#') {
        action_url.truncate(at);
    }
    action_url.push('#');
    action_url.push_str(&unit_tag);
    if action_url.starts_with("//") || (!action_url.starts_with('/') && !action_url.starts_with(place.home)) {
        return format!("<p class=\"wpcf7-invalid-action-url\"><strong>{}</strong> {}</p>", esc_html(&(r.tr)("Error:")), esc_html(&(r.tr)("Invalid action URL is detected.")));
    }
    let mut lang = form.locale.replace('_', "-");
    if let Some(c) = LANG.captures(&lang) {
        lang = c[1].to_owned();
    }
    let mut wrapper = Atts::default();
    wrapper.set("class", Att::str("wpcf7 no-js"));
    wrapper.set("id", Att::Str(unit_tag.clone()));
    wrapper.set(if place.html_type == "text/html" { "lang" } else { "xml:lang" }, Att::Str(lang));
    wrapper.set("dir", Att::str(if is_rtl_locale(&form.locale) { "rtl" } else { "ltr" }));
    // `id()` of a form that was never saved is null.
    let id = if form.id == 0 { Att::Null } else { Att::Int(form.id as i64) };
    wrapper.set("data-wpcf7-id", id);
    let mut html = format!("\n<div {}>", format_atts(&wrapper));
    html.push_str("\n<div class=\"screen-reader-response\"><p role=\"status\" aria-live=\"polite\" aria-atomic=\"true\"></p> <ul></ul></div>\n");
    let id_attr = ID_CHARS.replace_all(&options.html_id, "").into_owned();
    let name_attr = ID_CHARS.replace_all(&options.html_name, "").into_owned();
    let class = form_class(options, form.is_true("demo_mode"));
    let tags = form_tag::scan(&form.form);
    let enctype = if tags.iter().any(|t| form_tag::features(&t.tag_type).file_uploading) { "multipart/form-data" } else { "" };
    let mut atts = Atts::default();
    atts.set("action", Att::Str(esc_url(&action_url)));
    atts.set("method", Att::str("post"));
    atts.set("class", if class.is_empty() { Att::Null } else { Att::Str(class) });
    atts.set("id", if id_attr.is_empty() { Att::Null } else { Att::Str(id_attr) });
    atts.set("name", if name_attr.is_empty() { Att::Null } else { Att::Str(name_attr) });
    atts.set("aria-label", Att::Str(if options.html_title.is_empty() { (r.tr)("Contact form") } else { options.html_title.clone() }));
    atts.set("enctype", if enctype.is_empty() { Att::Null } else { Att::str(enctype) });
    atts.set("autocomplete", Att::Null);
    atts.set("novalidate", Att::Bool(true));
    atts.set("data-status", Att::str("init"));
    html.push_str(&format!("<form {}>\n", format_atts(&atts)));
    html.push_str(&hidden_fields(form, &unit_tag, place));
    let response = response_output();
    html.push_str(&replace_all_form_tags(form, r, &response));
    if r.responses == 0 {
        html.push_str(&response);
    }
    html.push_str("\n</form>\n</div>\n");
    html
}

/// `form_hidden_fields()`.
fn hidden_fields(form: &ContactForm, unit_tag: &str, place: &Place<'_>) -> String {
    let mut fields: Vec<(&str, Att)> = vec![
        ("_wpcf7", if form.id == 0 { Att::Null } else { Att::Int(form.id as i64) }),
        ("_wpcf7_version", Att::str(place.version)),
        ("_wpcf7_locale", Att::str(&form.locale)),
        ("_wpcf7_unit_tag", Att::str(unit_tag)),
        ("_wpcf7_container_post", Att::Int(place.loop_post.unwrap_or(0) as i64)),
        ("_wpcf7_posted_data_hash", Att::str("")),
    ];
    if form.is_true("subscribers_only") && place.logged_in
        && let Some(nonce) = &place.nonce
    {
        fields.push(("_wpnonce", Att::str(nonce)));
    }
    let mut formatter = HtmlFormatter::new(true);
    let mut container = Atts::default();
    container.set("class", Att::str("hidden-fields-container"));
    formatter.append_start_tag("fieldset", Some(&container), "");
    for (name, value) in fields {
        let mut input = Atts::default();
        input.set("type", Att::str("hidden"));
        input.set("name", Att::str(name));
        input.set("value", value);
        formatter.append_start_tag("input", Some(&input), "");
    }
    formatter.output() + "\n"
}

/// `form_response_output()` of a form that was not posted.
fn response_output() -> String {
    "<div class=\"wpcf7-response-output\" aria-hidden=\"true\"></div>".to_owned()
}

/// `replace_all_form_tags()`: `wpcf7_autop()` around placeholders, then every form-tag replaced by its control.
fn replace_all_form_tags(form: &ContactForm, r: &mut Render<'_>, response: &str) -> String {
    let (with_placeholders, placeholders) = form_tag::replace_with_placeholders(&form.form);
    let formatted = form_tag::restore_from_placeholders(&autop(&with_placeholders), &placeholders);
    let mut scanned: Vec<form_tag::FormTag> = Vec::new();
    let mut out = String::with_capacity(formatted.len() * 2);
    let mut at = 0;
    for m in form_tag::matches(&formatted) {
        out.push_str(&formatted[at..m.start]);
        at = m.end;
        if m.open == "[" && m.close == "]" {
            out.push_str(&m.whole[1..m.whole.len() - 1]);
            continue;
        }
        let Some(tag) = form_tag::scan_one(&m, &scanned) else {
            out.push_str(m.whole);
            continue;
        };
        let html = match controls::render(r, &tag) {
            Some(html) => html,
            None => {
                r.responses += 1;
                response.to_owned()
            }
        };
        scanned.push(tag);
        out.push_str(m.open);
        out.push_str(&html);
        out.push_str(m.close);
    }
    out.push_str(&formatted[at..]);
    out
}
