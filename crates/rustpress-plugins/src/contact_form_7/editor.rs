//! The editor of a contact form (`admin/edit-contact-form.php`, `admin/includes/editor.php`,
//! `admin/includes/tag-generator.php` and the tag generators of the modules): the page of `admin.php?page=wpcf7&post=ID`
//! and `admin.php?page=wpcf7-new`.

use std::sync::OnceLock;

use regex::Regex;
use rustpress_core::formatting::{esc_html, esc_url};
use rustpress_core::kses::Allowed;
use rustpress_core::native::AdminContext;
use rustpress_core::Site;
use rustpress_phpser::Value;

use super::DOMAIN;
use super::admin::{READ_WRITE, form_context};
use super::form::ContactForm;
use super::form_tag;
use super::formatter::{Att, HtmlFormatter};
use super::mail::{php_string, php_truthy};

fn tr(site: &Site, text: &str) -> String {
    site.trd(DOMAIN, text)
}

fn s(text: impl Into<String>) -> Att {
    Att::Str(text.into())
}

/// `esc_textarea()`.
fn esc_textarea(text: &str) -> String {
    rustpress_core::formatting::esc_html(text)
}

/// `wpcf7_messages()` with the descriptions, as the editor lists them (`captcha_not_match` only with Really Simple
/// CAPTCHA, which RustPress does not have).
const MESSAGES: &[(&str, &str)] = &[
    ("mail_sent_ok", "Sender&#8217;s message was sent successfully"),
    ("mail_sent_ng", "Sender&#8217;s message failed to send"),
    ("validation_error", "Validation errors occurred"),
    ("spam", "Submission was referred to as spam"),
    ("accept_terms", "There are terms that the sender must accept"),
    ("invalid_required", "There is a field that the sender must fill in"),
    ("invalid_too_long", "There is a field with input that is longer than the maximum allowed length"),
    ("invalid_too_short", "There is a field with input that is shorter than the minimum allowed length"),
    ("upload_failed", "Uploading a file fails for any reason"),
    ("upload_file_type_invalid", "Uploaded file is not allowed for file type"),
    ("upload_file_too_large", "Uploaded file is too large"),
    ("upload_failed_php_error", "Uploading a file fails for PHP error"),
    ("invalid_date", "Date format that the sender entered is invalid"),
    ("date_too_early", "Date is earlier than minimum limit"),
    ("date_too_late", "Date is later than maximum limit"),
    ("invalid_number", "Number format that the sender entered is invalid"),
    ("number_too_small", "Number is smaller than minimum limit"),
    ("number_too_large", "Number is larger than maximum limit"),
    ("quiz_answer_not_correct", "Sender does not enter the correct answer to the quiz"),
    ("invalid_email", "Email address that the sender entered is invalid"),
    ("invalid_url", "URL that the sender entered is invalid"),
    ("invalid_tel", "Telephone number that the sender entered is invalid"),
];

/// What the editor shows of a form: a stored one, or the template of a new one (`initial()`).
pub struct Edited<'a> {
    pub form: &'a ContactForm,
    pub initial: bool,
}

impl Edited<'_> {
    fn post_id(&self) -> i64 {
        if self.initial { -1 } else { self.form.id as i64 }
    }

    /// `message( $status, false )` of the upgraded properties.
    fn message(&self, site: &Site, key: &str) -> String {
        super::submission::stored_message(self.form, key, &|text| tr(site, text))
    }

    /// `prop( $name )` of a mail template after `upgrade()`, with the defaults of `wp_parse_args()`.
    fn mail(&self, site: &Site, name: &str) -> Vec<(&'static str, Value)> {
        let stored = if name == "mail" { self.form.mail.as_ref() } else { self.form.mail_2.as_ref() };
        let empty = Value::Str(Vec::new());
        let mut out: Vec<(&'static str, Value)> = ["active", "recipient", "sender", "subject", "body", "additional_headers", "attachments", "use_html", "exclude_blank"]
            .iter()
            .map(|key| (*key, stored.and_then(|m| m.get(key)).cloned().unwrap_or_else(|| if matches!(*key, "active" | "use_html" | "exclude_blank") { Value::Bool(false) } else { empty.clone() })))
            .collect();
        // `upgrade()`: a mail without a recipient goes to the administrator.
        if name == "mail" && stored.is_none_or(|m| m.as_array().is_some() && m.get("recipient").is_none()) {
            out[1].1 = Value::Str(site.options.get("admin_email").unwrap_or_default().as_bytes().to_vec());
        }
        out
    }
}

fn mail_value<'a>(mail: &'a [(&'static str, Value)], key: &str) -> &'a Value {
    mail.iter().find(|(k, _)| *k == key).map(|(_, v)| v).expect("every key of the mail has a default")
}

/// `collect_mail_tags()`: the names of the form-tags whose type is for mail.
fn collect_mail_tags(form: &ContactForm) -> Vec<String> {
    let not_for_mail: Vec<&str> = form_tag::TAG_TYPES.iter().filter(|(_, f)| f.not_for_mail).map(|(t, _)| *t).collect();
    let mut out: Vec<String> = Vec::new();
    for tag in form_tag::scan(&form.form) {
        if tag.basetype.is_empty() || not_for_mail.contains(&tag.basetype.as_str()) || tag.name.is_empty() || out.contains(&tag.name) {
            continue;
        }
        out.push(tag.name);
    }
    out
}

/// `suggest_mail_tags( $template_name )`.
fn suggest_mail_tags(form: &ContactForm, mail: &[(&'static str, Value)]) -> String {
    let texts: Vec<String> = mail.iter().filter(|(_, v)| php_truthy(v)).map(|(_, v)| php_string(v)).collect();
    let mut out = String::new();
    for tag in collect_mail_tags(form) {
        let pattern = Regex::new(&format!(r"\[(_[a-z]+_)?{}([ \t]+[^\]]+)?\]", regex::escape(&tag))).expect("valid mail-tag pattern");
        let used = texts.iter().any(|t| pattern.is_match(t));
        out.push_str(&format!("<span class=\"mailtag code {}\">[{}]</span>", if used { "used" } else { "unused" }, esc_html(&tag)));
    }
    out
}

/// `wpcf7_editor_box_mail()`.
fn mail_box(site: &Site, edited: &Edited<'_>, id: &str, name: &str, title: &str, use_label: Option<&str>) -> String {
    let t = |text: &str| tr(site, text);
    let mail = edited.mail(site, name);
    let value = |key: &str| s(php_string(mail_value(&mail, key)));
    // `wpcf7_format_atts()` takes the stored value as it is: `false` leaves `checked` out, `0` prints it.
    let flag = |key: &str| match mail_value(&mail, key) {
        Value::Bool(b) => Att::Bool(*b),
        Value::Int(n) => Att::Int(*n),
        Value::Null => Att::Null,
        other => Att::Str(php_string(other)),
    };
    let mut f = HtmlFormatter::new(false);
    f.open("div", &[("class", s("contact-form-editor-box-mail")), ("id", s(id))]);
    f.open("h2", &[]);
    f.append_preformatted(&esc_html(title));
    f.end_tag("h2");
    if let Some(use_label) = use_label {
        f.open("label", &[("for", s(format!("{id}-active")))]);
        f.open("input", &[("type", s("checkbox")), ("id", s(format!("{id}-active"))), ("name", s(format!("{id}[active]"))), ("data-config-field", s("")), ("data-toggle", s(format!("{id}-fieldset"))), ("value", s("1")), ("checked", flag("active"))]);
        f.append_whitespace();
        f.append_preformatted(&esc_html(use_label));
        f.end_tag("label");
        f.open("p", &[("class", s("description"))]);
        f.append_preformatted(&esc_html(&t("Mail (2) is an additional mail template often used as an autoresponder.")));
        f.end_tag("p");
    }
    f.open("fieldset", &[("id", s(format!("{id}-fieldset")))]);
    f.open("legend", &[]);
    f.append_preformatted(&t("You can edit the email template here. For details, see <a href=\"%s\">Setting up mail</a>.").replacen("%s", "https://contactform7.com/setting-up-mail/", 1));
    f.open("br", &[]);
    f.append_preformatted(&esc_html(&t("In the following fields, you can use these mail-tags:")));
    f.open("br", &[]);
    f.append_printed(&suggest_mail_tags(edited.form, &mail));
    f.end_tag("legend");
    f.open("table", &[("class", s("form-table"))]);
    f.open("tbody", &[]);
    let row = |f: &mut HtmlFormatter, key: &str, field: &str, label: &str| {
        f.open("tr", &[]);
        f.open("th", &[("scope", s("row"))]);
        f.open("label", &[("for", s(format!("{id}-{field}")))]);
        f.append_preformatted(&esc_html(&t(label)));
        f.open("td", &[]);
        f.open("input", &[("type", s("text")), ("id", s(format!("{id}-{field}"))), ("name", s(format!("{id}[{key}]"))), ("class", s("large-text code")), ("size", Att::Int(70)), ("value", value(key)), ("data-config-field", s(format!("{name}.{key}")))]);
        f.end_tag("tr");
    };
    row(&mut f, "recipient", "recipient", "To");
    row(&mut f, "sender", "sender", "From");
    row(&mut f, "subject", "subject", "Subject");
    let textarea_row = |f: &mut HtmlFormatter, key: &str, field: &str, label: &str, rows: i64| {
        f.open("tr", &[]);
        f.open("th", &[("scope", s("row"))]);
        f.open("label", &[("for", s(format!("{id}-{field}")))]);
        f.append_preformatted(&esc_html(&t(label)));
        f.open("td", &[]);
        f.open("textarea", &[("id", s(format!("{id}-{field}"))), ("name", s(format!("{id}[{key}]"))), ("cols", Att::Int(100)), ("rows", Att::Int(rows)), ("class", s("large-text code")), ("data-config-field", s(format!("{name}.{key}")))]);
        f.append_preformatted(&esc_textarea(&php_string(mail_value(&mail, key))));
    };
    textarea_row(&mut f, "additional_headers", "additional-headers", "Additional headers", 4);
    f.end_tag("tr");
    textarea_row(&mut f, "body", "body", "Message body", 18);
    f.end_tag("textarea");
    for (key, field, label) in [("exclude_blank", "exclude-blank", "Exclude a line from output if all of its mail-tags are blank"), ("use_html", "use-html", "Use HTML content type")] {
        f.open("p", &[]);
        f.open("label", &[("for", s(format!("{id}-{field}")))]);
        f.open("input", &[("type", s("checkbox")), ("id", s(format!("{id}-{field}"))), ("name", s(format!("{id}[{key}]"))), ("value", s("1")), ("checked", flag(key))]);
        f.append_whitespace();
        f.append_preformatted(&esc_html(&t(label)));
    }
    f.end_tag("tr");
    textarea_row(&mut f, "attachments", "attachments", "File attachments", 4);
    f.end_tag("textarea");
    f.end_tag("tr");
    kses(&f.output())
}

/// `wpcf7_editor_panel_form()`.
fn panel_form(site: &Site, edited: &Edited<'_>) -> String {
    let t = |text: &str| tr(site, text);
    let mut f = HtmlFormatter::new(false);
    f.open("h2", &[]);
    f.append_preformatted(&esc_html(&t("Form")));
    f.end_tag("h2");
    f.open("fieldset", &[]);
    f.open("legend", &[]);
    f.append_preformatted(&t("You can edit the form template here. For details, see <a href=\"%s\">Editing form template</a>.").replacen("%s", "https://contactform7.com/editing-form-template/", 1));
    f.end_tag("legend");
    f.append_printed(&tag_generator_buttons(site));
    f.open("textarea", &[("id", s("wpcf7-form")), ("name", s("wpcf7-form")), ("cols", Att::Int(100)), ("rows", Att::Int(24)), ("class", s("large-text code")), ("data-config-field", s("form.body"))]);
    f.append_preformatted(&esc_textarea(&edited.form.form));
    f.end_tag("textarea");
    kses(&f.output())
}

/// `wpcf7_editor_panel_mail()`.
fn panel_mail(site: &Site, edited: &Edited<'_>) -> String {
    format!(
        "{}<br class=\"clear\" />{}",
        mail_box(site, edited, "wpcf7-mail", "mail", &tr(site, "Mail"), None),
        mail_box(site, edited, "wpcf7-mail-2", "mail_2", &tr(site, "Mail (2)"), Some(&tr(site, "Use Mail (2)")))
    )
}

/// `wpcf7_editor_panel_messages()`.
fn panel_messages(site: &Site, edited: &Edited<'_>) -> String {
    let t = |text: &str| tr(site, text);
    let mut f = HtmlFormatter::new(false);
    f.open("h2", &[]);
    f.append_preformatted(&esc_html(&t("Messages")));
    f.end_tag("h2");
    f.open("fieldset", &[]);
    f.open("legend", &[]);
    f.append_preformatted(&t("You can edit messages used in various situations here. For details, see <a href=\"%s\">Editing messages</a>.").replacen("%s", "https://contactform7.com/editing-messages/", 1));
    f.end_tag("legend");
    for (key, description) in MESSAGES {
        let field_id = format!("wpcf7-message-{}", key.replace('_', "-"));
        f.open("p", &[("class", s("description"))]);
        f.open("label", &[("for", s(&field_id))]);
        f.append_preformatted(&esc_html(&t(description)));
        f.open("br", &[]);
        f.open("input", &[("type", s("text")), ("id", s(&field_id)), ("name", s(format!("wpcf7-messages[{key}]"))), ("class", s("large-text")), ("size", Att::Int(70)), ("value", s(edited.message(site, key))), ("data-config-field", s(format!("messages.{key}")))]);
    }
    kses(&f.output())
}

/// `wpcf7_editor_panel_additional_settings()`.
fn panel_additional_settings(site: &Site, edited: &Edited<'_>) -> String {
    let t = |text: &str| tr(site, text);
    let mut f = HtmlFormatter::new(false);
    f.open("h2", &[]);
    f.append_preformatted(&esc_html(&t("Additional Settings")));
    f.end_tag("h2");
    f.open("fieldset", &[]);
    f.open("legend", &[]);
    f.append_preformatted(&t("You can add customization code snippets here. For details, see <a href=\"%s\">Additional settings</a>.").replacen("%s", "https://contactform7.com/additional-settings/", 1));
    f.end_tag("legend");
    f.open("textarea", &[("id", s("wpcf7-additional-settings")), ("name", s("wpcf7-additional-settings")), ("cols", Att::Int(100)), ("rows", Att::Int(24)), ("class", s("large-text")), ("data-config-field", s("additional_settings.body"))]);
    f.append_preformatted(&esc_textarea(&edited.form.additional_settings));
    f.end_tag("textarea");
    kses(&f.output())
}

/// `WPCF7_Editor::display()` with the four panels.
fn editor_panels(ctx: &AdminContext<'_>, edited: &Edited<'_>) -> String {
    let site = ctx.site;
    let t = |text: &str| tr(site, text);
    if !(ctx.can)(READ_WRITE) {
        return String::new();
    }
    let settings = edited.form.additional_settings.trim().split('\n').filter(|l| !l.is_empty() && *l != "0").count();
    let additional_title = if settings > 0 { t("Additional Settings (%d)").replacen("%d", &settings.to_string(), 1) } else { t("Additional Settings") };
    let panels: Vec<(&str, String)> = vec![("form-panel", t("Form")), ("mail-panel", t("Mail")), ("messages-panel", t("Messages")), ("additional-settings-panel", additional_title)];
    let requested = ctx.get("active-tab").unwrap_or_default();
    let active = panels.iter().position(|(id, _)| *id == requested).unwrap_or(0);
    let mut f = HtmlFormatter::new(false);
    f.open("nav", &[("id", s("contact-form-editor-tabs")), ("role", s("tablist")), ("aria-label", s(t("Contact form editor tabs"))), ("data-active-tab", Att::Int(active as i64))]);
    for (index, (id, title)) in panels.iter().enumerate() {
        let current = index == active;
        f.open("button", &[("type", s("button")), ("role", s("tab")), ("aria-selected", s(if current { "true" } else { "false" })), ("aria-controls", s(*id)), ("id", s(format!("{id}-tab"))), ("tabindex", s(if current { "0" } else { "-1" })), ("data-panel", s(*id))]);
        f.append_preformatted(&esc_html(title));
    }
    f.end_tag("nav");
    for (index, (id, _)) in panels.iter().enumerate() {
        f.open("section", &[("role", s("tabpanel")), ("aria-labelledby", s(format!("{id}-tab"))), ("id", s(*id)), ("class", s("contact-form-editor-panel")), ("tabindex", s("0")), ("hidden", Att::Bool(index != active))]);
        let content = match *id {
            "form-panel" => panel_form(site, edited),
            "mail-panel" => panel_mail(site, edited),
            "messages-panel" => panel_messages(site, edited),
            _ => panel_additional_settings(site, edited),
        };
        f.append_printed(&content);
        f.end_tag("section");
    }
    kses(&f.output())
}

/// `wpcf7_kses_allowed_html()` with the `form` attributes of `edit-contact-form.php`.
fn page_allowed() -> &'static Allowed {
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    ALLOWED.get_or_init(|| form_context().merged(&[("form", &["method", "action", "id", "class", "disabled"])]))
}

/// The same with the `dialog` and the `form` of the tag generator panels.
fn panels_allowed() -> &'static Allowed {
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    ALLOWED.get_or_init(|| form_context().merged(&[("dialog", &["id", "class"]), ("form", &["method", "class", "data-*"])]))
}

/// `print()` of a formatter made with the default options.
fn kses(html: &str) -> String {
    static ALLOWED: OnceLock<Allowed> = OnceLock::new();
    rustpress_core::kses::kses(html, ALLOWED.get_or_init(form_context))
}

/// `edit-contact-form.php`: the editor of `edited`, followed by the dialogs of the tag generators.
pub fn page(ctx: &AdminContext<'_>, edited: &Edited<'_>, notices: &str) -> String {
    let site = ctx.site;
    let t = |text: &str| tr(site, text);
    let can_edit = (ctx.can)(READ_WRITE);
    let post_id = edited.post_id();
    let form = edited.form;
    let mut f = HtmlFormatter::new(false);
    f.open("div", &[("class", s("wrap")), ("id", s("wpcf7-contact-form-editor"))]);
    f.open("h1", &[("class", s("wp-heading-inline"))]);
    f.append_preformatted(&esc_html(&if edited.initial { t("Add Contact Form") } else { t("Edit Contact Form") }));
    f.end_tag("h1");
    if !edited.initial && can_edit {
        f.append_whitespace();
        f.append_preformatted(&format!("<a class=\"page-title-action\" href=\"{}\">{}</a>", esc_url(&site.site_url("wp-admin/admin.php?page=wpcf7-new")), esc_html(&t("Add Contact Form"))));
    }
    f.open("hr", &[("class", s("wp-header-end"))]);
    f.append_printed(notices);
    let action = format!("{}&post={post_id}", site.site_url("wp-admin/admin.php?page=wpcf7"));
    f.open("form", &[("method", s("post")), ("action", s(esc_url(&action))), ("id", s("wpcf7-admin-form-element")), ("disabled", Att::Bool(!can_edit))]);
    if can_edit {
        f.append_printed(&format!(
            "<input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />",
            (ctx.nonce)(&format!("wpcf7-save-contact-form_{post_id}")),
            rustpress_core::formatting::esc_attr(ctx.request_uri)
        ));
    }
    f.open("input", &[("type", s("hidden")), ("id", s("post_ID")), ("name", s("post_ID")), ("value", Att::Int(post_id))]);
    f.open("input", &[("type", s("hidden")), ("id", s("wpcf7-locale")), ("name", s("wpcf7-locale")), ("value", s(&form.locale))]);
    f.open("input", &[("type", s("hidden")), ("id", s("hiddenaction")), ("name", s("action")), ("value", s("save"))]);
    f.open("input", &[("type", s("hidden")), ("id", s("active-tab")), ("name", s("active-tab")), ("value", s(ctx.get("active-tab").unwrap_or_default()))]);
    f.open("div", &[("id", s("poststuff"))]);
    f.open("div", &[("id", s("post-body")), ("class", s("metabox-holder columns-2 wp-clearfix"))]);
    f.open("div", &[("id", s("post-body-content"))]);
    f.open("div", &[("id", s("titlediv"))]);
    f.open("div", &[("id", s("titlewrap"))]);
    f.open(
        "input",
        &[
            ("type", s("text")),
            ("name", s("post_title")),
            ("value", s(if edited.initial { String::new() } else { form.title.clone() })),
            ("id", s("title")),
            ("spellcheck", s("true")),
            ("autocomplete", s("off")),
            ("disabled", Att::Bool(!can_edit)),
            ("placeholder", s(t("Enter title here"))),
            ("aria-label", s(t("Enter title here"))),
        ],
    );
    f.end_tag("div");
    f.open("div", &[("class", s("inside"))]);
    if !edited.initial {
        f.open("div", &[("id", s("wpcf7-shortcode-wrap")), ("class", s("hide-shortcode"))]);
        f.open("button", &[("class", s("toggle-shortcode alignright button-link")), ("type", s("button"))]);
        f.append_preformatted(&esc_html(&t("Toggle shortcode")));
        f.end_tag("button");
        f.open("div", &[("id", s("wpcf7-shortcode-inside"))]);
        let title: String = form.title.chars().filter(|c| !matches!(c, '"' | '[' | ']')).collect();
        let shortcode = format!("[contact-form-7 id=\"{}\" title=\"{title}\"]", form.short_hash(7));
        f.open("p", &[("class", s("description"))]);
        f.open("label", &[("for", s("wpcf7-shortcode"))]);
        f.append_preformatted(&esc_html(&t("Shortcodes are only needed in the Classic Editor environment; otherwise, use the Contact Form 7 block in the Block Editor.")));
        f.end_tag("label");
        f.append_whitespace();
        f.open("span", &[("class", s("shortcode wp-ui-highlight"))]);
        f.open("input", &[("type", s("text")), ("id", s("wpcf7-shortcode")), ("readonly", Att::Bool(true)), ("class", s("large-text code selectable")), ("value", s(shortcode))]);
        f.end_tag("p");
        f.end_tag("div");
        f.end_tag("div");
    }
    f.end_tag("div");
    f.end_tag("div");
    f.end_tag("div");
    f.open("div", &[("id", s("postbox-container-1")), ("class", s("postbox-container"))]);
    if can_edit {
        f.open("section", &[("id", s("submitdiv")), ("class", s("postbox"))]);
        f.open("h2", &[]);
        f.append_preformatted(&esc_html(&t("Status")));
        f.end_tag("h2");
        f.open("div", &[("class", s("inside"))]);
        f.open("div", &[("class", s("submitbox")), ("id", s("submitpost"))]);
        f.open("div", &[("id", s("minor-publishing-actions"))]);
        f.open("div", &[("class", s("hidden"))]);
        f.open("button", &[("type", s("submit")), ("class", s("button-primary")), ("name", s("wpcf7-save")), ("value", s(t("Save")))]);
        f.append_preformatted(&esc_html(&t("Save")));
        f.end_tag("div");
        if !edited.initial {
            f.open("button", &[("type", s("submit")), ("name", s("wpcf7-copy")), ("class", s("copy button-link")), ("value", s(t("Duplicate")))]);
            f.append_preformatted(&esc_html(&t("Duplicate")));
            f.end_tag("button");
        }
        f.end_tag("div");
        f.open("div", &[("id", s("misc-publishing-actions"))]);
        f.append_printed("");
        f.end_tag("div");
        f.open("div", &[("id", s("major-publishing-actions"))]);
        if !edited.initial {
            f.open("div", &[("id", s("delete-action"))]);
            f.open("button", &[("type", s("submit")), ("name", s("wpcf7-delete")), ("class", s("delete submitdelete")), ("value", s(t("Delete")))]);
            f.append_preformatted(&esc_html(&t("Delete")));
            f.end_tag("div");
        }
        f.open("div", &[("id", s("publishing-action"))]);
        f.append_preformatted("<span class=\"spinner\"></span>");
        f.open("button", &[("type", s("submit")), ("class", s("button-primary")), ("name", s("wpcf7-save")), ("value", s(t("Save")))]);
        f.append_preformatted(&esc_html(&t("Save")));
        f.end_tag("div");
        f.append_preformatted("<div class=\"clear\"></div>");
        f.end_tag("div");
        f.end_tag("div");
        f.end_tag("div");
        f.end_tag("section");
    }
    f.end_tag("div");
    f.open("div", &[("id", s("postbox-container-2")), ("class", s("postbox-container"))]);
    f.open("div", &[("id", s("contact-form-editor"))]);
    f.append_printed(&editor_panels(ctx, edited));
    f.end_tag("div");
    f.end_tag("div");
    f.end_tag("div");
    f.append_preformatted("<br class=\"clear\" />");
    f.end_tag("div");
    f.end_tag("form");
    f.end_tag("div");
    let page = rustpress_core::kses::kses(&f.output(), page_allowed());
    format!("{page}{}", tag_generator_panels(site))
}

/// A tag generator (`WPCF7_TagGenerator::add()`), in the order the modules register them.
struct Generator {
    id: &'static str,
    title: &'static str,
}

const GENERATORS: &[Generator] = &[
    Generator { id: "text", title: "text" },
    Generator { id: "email", title: "email" },
    Generator { id: "url", title: "URL" },
    Generator { id: "tel", title: "tel" },
    Generator { id: "number", title: "number" },
    Generator { id: "date", title: "date" },
    Generator { id: "textarea", title: "text area" },
    Generator { id: "menu", title: "drop-down menu" },
    Generator { id: "checkbox", title: "checkboxes" },
    Generator { id: "radio", title: "radio buttons" },
    Generator { id: "acceptance", title: "acceptance" },
    Generator { id: "quiz", title: "quiz" },
    Generator { id: "file", title: "file" },
    Generator { id: "submit", title: "submit" },
];

/// `WPCF7_TagGenerator::print_buttons()`.
fn tag_generator_buttons(site: &Site) -> String {
    let mut f = HtmlFormatter::new(false);
    f.open("span", &[("id", s("tag-generator-list")), ("class", s("hide-if-no-js"))]);
    for generator in GENERATORS {
        let title = tr(site, generator.title);
        f.open("button", &[("type", s("button")), ("data-taggen", s("open-dialog")), ("data-target", s(format!("tag-generator-panel-{}", generator.id))), ("title", s(tr(site, "Form-tag Generator: %s").replacen("%s", &title, 1)))]);
        f.append_preformatted(&esc_html(&title));
    }
    kses(&f.output())
}

/// `WPCF7_TagGeneratorGenerator`: the parts the panels are made of.
struct Parts<'a> {
    site: &'a Site,
    key: String,
}

impl Parts<'_> {
    fn t(&self, text: &str) -> String {
        tr(self.site, text)
    }

    /// `ref( $suffix )`.
    fn r(&self, suffix: &str) -> String {
        let reference = format!("{}-{suffix}", self.key).to_lowercase();
        let reference: String = reference.chars().filter(|c| c.is_ascii_digit() || c.is_ascii_lowercase() || *c == '-').collect();
        let mut out = String::new();
        for c in reference.chars() {
            if c == '-' && out.ends_with('-') {
                continue;
            }
            out.push(c);
        }
        out.trim_matches('-').to_owned()
    }

    fn field_type(&self, with_required: bool, with_optional: bool, select_options: &[(&str, String)]) -> String {
        let mut f = HtmlFormatter::new(false);
        f.open("fieldset", &[]);
        f.open("legend", &[("id", s(self.r("type-legend")))]);
        f.append_preformatted(&esc_html(&self.t("Field type")));
        f.end_tag("legend");
        f.open("select", &[("data-tag-part", s("basetype")), ("aria-labelledby", s(self.r("type-legend")))]);
        for (basetype, title) in select_options {
            f.open("option", &[("value", s(*basetype))]);
            f.append_preformatted(&esc_html(title));
        }
        f.end_tag("select");
        if with_required {
            f.open("br", &[]);
            f.open("label", &[]);
            f.open("input", &[("type", s("checkbox")), ("data-tag-part", s("type-suffix")), ("value", s("*"))]);
            f.append_whitespace();
            f.append_preformatted(&esc_html(&self.t("This is a required field.")));
        }
        if with_optional {
            f.open("br", &[]);
            f.open("label", &[]);
            f.open("input", &[("type", s("checkbox")), ("data-tag-part", s("option")), ("data-tag-option", s("optional")), ("checked", Att::Bool(true))]);
            f.append_whitespace();
            f.append_preformatted(&esc_html(&self.t("This checkbox is optional.")));
        }
        kses(&f.output())
    }

    fn field_name(&self, ask_if: &str) -> String {
        let legend = self.r("name-legend");
        let mut out = format!(
            "<fieldset>\n\t<legend id=\"{legend}\">{}</legend>\n\t<input type=\"text\" data-tag-part=\"name\" pattern=\"[A-Za-z][A-Za-z0-9_\\-]*\" aria-labelledby=\"{legend}\" />\n\n",
            esc_html(&self.t("Field name"))
        );
        let (option, label) = match ask_if {
            "author_name" => ("autocomplete:name", "This field expects the submitter name."),
            "author_email" => ("autocomplete:email", "This field expects the submitter email."),
            "author_url" => ("autocomplete:url", "This field expects the submitter URL."),
            "author_tel" => ("autocomplete:tel", "This field expects the submitter telephone number."),
            _ => ("", ""),
        };
        if !option.is_empty() {
            out.push_str(&format!("\t<br />\n\t<label>\n\t\t<input type=\"checkbox\" data-tag-part=\"option\" data-tag-option=\"{option}\" />\n\t\t{}\t</label>\n", esc_html(&self.t(label))));
        }
        out.push_str("</fieldset>\n");
        out
    }

    fn class_attr(&self) -> String {
        let legend = self.r("class-legend");
        format!(
            "<fieldset>\n\t<legend id=\"{legend}\">{}</legend>\n\t<input type=\"text\" data-tag-part=\"option\" data-tag-option=\"class:\" pattern=\"[A-Za-z0-9_\\-\\s]*\" aria-labelledby=\"{legend}\" />\n</fieldset>\n",
            esc_html(&self.t("Class attribute"))
        )
    }

    fn min_max(&self, kind: &str, title: &str, min_option: &str, max_option: &str) -> String {
        let mut f = HtmlFormatter::new(false);
        f.open("fieldset", &[]);
        f.open("legend", &[]);
        f.append_preformatted(&esc_html(title));
        f.end_tag("legend");
        for (index, (label, option)) in [("Min", min_option), ("Max", max_option)].iter().enumerate() {
            if index == 1 {
                f.append_preformatted(" &#8660; ");
            }
            f.open("label", &[]);
            f.append_preformatted(&esc_html(&self.t(label)));
            f.append_whitespace();
            f.open("input", &[("type", s(kind)), ("data-tag-part", s("option")), ("data-tag-option", s(*option)), ("min", Att::Int(0))]);
            f.end_tag("label");
        }
        kses(&f.output())
    }

    fn default_value(&self, kind: &str, title: &str, with_placeholder: bool, use_content: bool) -> String {
        let mut f = HtmlFormatter::new(false);
        f.open("fieldset", &[]);
        f.open("legend", &[("id", s(self.r("value-legend")))]);
        f.append_preformatted(&esc_html(title));
        f.end_tag("legend");
        f.open("input", &[("type", s(kind)), ("data-tag-part", s(if use_content { "content" } else { "value" })), ("aria-labelledby", s(self.r("value-legend")))]);
        if with_placeholder {
            f.open("br", &[]);
            f.open("label", &[]);
            f.open("input", &[("type", s("checkbox")), ("data-tag-part", s("option")), ("data-tag-option", s("placeholder"))]);
            f.append_whitespace();
            f.append_preformatted(&esc_html(&self.t("Use this text as the placeholder.")));
        }
        kses(&f.output())
    }

    /// `selectable_values()`: `first_as_label` and `use_label_element` are `None` (no option), `Some(false)` (shown) or
    /// `Some(true)` (shown and checked).
    fn selectable_values(&self, first_as_label: Option<bool>, use_label_element: Option<bool>) -> String {
        let mut f = HtmlFormatter::new(false);
        f.open("fieldset", &[]);
        f.open("legend", &[("id", s(self.r("selectable-values-legend")))]);
        f.append_preformatted(&esc_html(&self.t("Selectable values")));
        f.end_tag("legend");
        f.open("span", &[("id", s(self.r("selectable-values-description")))]);
        f.append_preformatted(&esc_html(&self.t("One item per line.")));
        f.end_tag("span");
        f.open("br", &[]);
        f.open("textarea", &[("required", Att::Bool(true)), ("data-tag-part", s("value")), ("aria-labelledby", s(self.r("selectable-values-legend"))), ("aria-describedby", s(self.r("selectable-values-description")))]);
        f.append_preformatted(&esc_html(&self.t("Option 1\nOption 2\nOption 3")));
        f.end_tag("textarea");
        for (setting, option, label) in [(first_as_label, "first_as_label", "Use the first item as a label."), (use_label_element, "use_label_element", "Wrap each item with a label element.")] {
            if let Some(checked) = setting {
                f.open("br", &[]);
                f.open("label", &[]);
                f.open("input", &[("type", s("checkbox")), ("checked", Att::Bool(checked)), ("data-tag-part", s("option")), ("data-tag-option", s(option))]);
                f.append_whitespace();
                f.append_preformatted(&esc_html(&self.t(label)));
                f.end_tag("label");
            }
        }
        kses(&f.output())
    }

    fn insert_box_content(&self) -> String {
        let mut f = HtmlFormatter::new(false);
        f.open("div", &[("class", s("flex-container"))]);
        f.open("input", &[("type", s("text")), ("class", s("code selectable")), ("readonly", Att::Bool(true)), ("data-tag-part", s("tag")), ("aria-label", s(self.t("The form-tag to be inserted into the form template")))]);
        f.append_whitespace();
        f.open("button", &[("type", s("button")), ("class", s("button button-primary")), ("data-taggen", s("insert-tag"))]);
        f.append_preformatted(&esc_html(&self.t("Insert Tag")));
        f.end_tag("button");
        kses(&f.output())
    }

    fn mail_tag_tip(&self) -> String {
        let mut f = HtmlFormatter::new(false);
        f.open("p", &[("class", s("mail-tag-tip"))]);
        f.append_preformatted(&esc_html(&self.t("To use the user input in the email, insert the corresponding mail-tag %s into the email template.")).replacen("%s", "<strong data-tag-part=\"mail-tag\"></strong>", 1));
        kses(&f.output())
    }
}

/// The heading, the description and the documentation of a field type.
struct FieldType {
    display_name: &'static str,
    heading: &'static str,
    description: &'static str,
    url: &'static str,
}

fn field_type_of(id: &str) -> FieldType {
    let ft = |display_name, heading, description, url| FieldType { display_name, heading, description, url };
    match id {
        "email" => ft("Email address field", "Email address field form-tag generator", "Generates a form-tag for an <a href=\"%s\">email address input field</a>.", "https://contactform7.com/text-fields/"),
        "url" => ft("URL field", "URL field form-tag generator", "Generates a form-tag for a <a href=\"%s\">URL input field</a>.", "https://contactform7.com/text-fields/"),
        "tel" => ft("Telephone number field", "Telephone number field form-tag generator", "Generates a form-tag for a <a href=\"%s\">telephone number input field</a>.", "https://contactform7.com/text-fields/"),
        "number" => ft("Number field", "Number field form-tag generator", "Generates a form-tag for a <a href=\"%s\">number input field</a>.", "https://contactform7.com/number-fields/"),
        "date" => ft("Date field", "Date field form-tag generator", "Generates a form-tag for a <a href=\"%s\">date input field</a>.", "https://contactform7.com/date-field/"),
        "textarea" => ft("Text area", "Text area form-tag generator", "Generates a form-tag for a <a href=\"%s\">multi-line plain text input area</a>.", "https://contactform7.com/text-fields/"),
        "menu" => ft("Drop-down menu", "Drop-down menu form-tag generator", "Generates a form-tag for a <a href=\"%s\">drop-down menu</a>.", "https://contactform7.com/checkboxes-radio-buttons-and-menus/"),
        "checkbox" => ft("Checkboxes", "Checkboxes form-tag generator", "Generates a form-tag for a group of <a href=\"%s\">checkboxes</a>.", "https://contactform7.com/checkboxes-radio-buttons-and-menus/"),
        "radio" => ft("Radio buttons", "Radio buttons form-tag generator", "Generates a form-tag for a group of <a href=\"%s\">radio buttons</a>.", "https://contactform7.com/checkboxes-radio-buttons-and-menus/"),
        "acceptance" => ft("Acceptance checkbox", "Acceptance checkbox form-tag generator", "Generates a form-tag for an <a href=\"%s\">acceptance checkbox</a>.", "https://contactform7.com/acceptance-checkbox/"),
        "quiz" => ft("Quiz", "Quiz form-tag generator", "Generates a form-tag for a <a href=\"%s\">quiz</a>.", "https://contactform7.com/quiz/"),
        "file" => ft("File uploading field", "File uploading field form-tag generator", "Generates a form-tag for a <a href=\"%s\">file uploading field</a>.", "https://contactform7.com/file-uploading-and-attachment/"),
        "submit" => ft("Submit button", "Submit button form-tag generator", "Generates a form-tag for a <a href=\"%s\">submit button</a>.", "https://contactform7.com/submit-button/"),
        _ => ft("Text field", "Text field form-tag generator", "Generates a form-tag for a <a href=\"%s\">single-line plain text input field</a>.", "https://contactform7.com/text-fields/"),
    }
}

/// The callback of a tag generator (`wpcf7_tag_generator_text()` and the others).
fn generator_panel(site: &Site, id: &str) -> String {
    let p = Parts { site, key: format!("tag-generator-panel-{id}") };
    let t = |text: &str| tr(site, text);
    let ft = field_type_of(id);
    let mut f = HtmlFormatter::new(false);
    f.open("header", &[("class", s("description-box"))]);
    f.open("h3", &[]);
    f.append_preformatted(&esc_html(&t(ft.heading)));
    f.end_tag("h3");
    f.open("p", &[]);
    f.append_preformatted(&rustpress_core::kses::kses_data(&t(ft.description).replacen("%s", ft.url, 1)));
    f.end_tag("header");
    f.open("div", &[("class", s("control-box"))]);
    let display = t(ft.display_name);
    let basetype = match id {
        "menu" => "select",
        other => other,
    };
    let controls = match id {
        "text" | "email" | "url" | "tel" => {
            let purpose = match id {
                "email" => "author_email",
                "url" => "author_url",
                "tel" => "author_tel",
                _ => "author_name",
            };
            [p.field_type(true, false, &[(basetype, display)]), p.field_name(purpose), p.class_attr(), p.min_max("number", &t("Length"), "minlength:", "maxlength:"), p.default_value("text", &t("Default value"), true, false)].concat()
        }
        "number" => [p.field_type(true, false, &[("number", t("Spinbox")), ("range", t("Slider"))]), p.field_name(""), p.class_attr(), p.min_max("number", &t("Range"), "min:", "max:"), p.default_value("number", &t("Default value"), false, false)].concat(),
        "date" => [p.field_type(true, false, &[("date", display)]), p.field_name(""), p.class_attr(), p.min_max("date", &t("Range"), "min:", "max:"), p.default_value("date", &t("Default value"), false, false)].concat(),
        "textarea" => [p.field_type(true, false, &[("textarea", display)]), p.field_name(""), p.class_attr(), p.min_max("number", &t("Length"), "minlength:", "maxlength:"), p.default_value("text", &t("Default value"), true, true)].concat(),
        "menu" => [p.field_type(true, false, &[("select", display)]), p.field_name(""), p.class_attr(), p.selectable_values(Some(false), None)].concat(),
        "checkbox" | "radio" => [p.field_type(id == "checkbox", false, &[(basetype, display)]), p.field_name(""), p.class_attr(), p.selectable_values(None, Some(true))].concat(),
        "acceptance" => [p.field_type(false, true, &[("acceptance", display)]), p.field_name(""), p.class_attr()].concat(),
        "quiz" | "file" => [p.field_type(id == "file", false, &[(basetype, display)]), p.field_name(""), p.class_attr()].concat(),
        _ => [p.field_type(false, false, &[("submit", display)]), p.class_attr(), p.default_value("text", &t("Label"), false, false)].concat(),
    };
    f.append_printed(&controls);
    match id {
        "acceptance" => {
            f.open("fieldset", &[]);
            f.open("legend", &[("id", s(p.r("value-legend")))]);
            f.append_preformatted(&esc_html(&t("Condition")));
            f.end_tag("legend");
            f.open("input", &[("type", s("text")), ("required", Att::Bool(true)), ("value", s(t("Put the condition for consent here."))), ("data-tag-part", s("content")), ("aria-labelledby", s(p.r("value-legend")))]);
        }
        "quiz" => {
            f.open("fieldset", &[]);
            f.open("legend", &[("id", s(p.r("selectable-values-legend")))]);
            f.append_preformatted(&esc_html(&t("Questions and answers")));
            f.end_tag("legend");
            f.open("span", &[("id", s(p.r("selectable-values-description")))]);
            f.append_preformatted(&esc_html(&t("One pipe-separated question-answer pair (question|answer) per line.")));
            f.end_tag("span");
            f.open("br", &[]);
            f.open("textarea", &[("required", Att::Bool(true)), ("data-tag-part", s("value")), ("aria-labelledby", s(p.r("selectable-values-legend"))), ("aria-describedby", s(p.r("selectable-values-description")))]);
            f.append_preformatted(&esc_html(&t("The capital of Brazil? | Rio")));
            f.end_tag("textarea");
        }
        "file" => {
            for (part, legend, description, pattern, value, option) in [
                ("filetypes-option", "Acceptable file types", "Pipe-separated file types list. You can use file extensions and MIME types.", "[0-9a-z*\\/\\|]*", "audio/*|video/*|image/*", "filetypes:"),
                ("limit-option", "File size limit", "In bytes. You can use kb and mb suffixes.", "[1-9][0-9]*([kKmM]?[bB])?", "1mb", "limit:"),
            ] {
                f.open("fieldset", &[]);
                f.open("legend", &[("id", s(p.r(&format!("{part}-legend"))))]);
                f.append_preformatted(&esc_html(&t(legend)));
                f.end_tag("legend");
                f.open("label", &[]);
                f.open("span", &[("id", s(p.r(&format!("{part}-description"))))]);
                f.append_preformatted(&esc_html(&t(description)));
                f.end_tag("span");
                f.open("br", &[]);
                f.open(
                    "input",
                    &[
                        ("type", s("text")),
                        ("pattern", s(pattern)),
                        ("value", s(value)),
                        ("aria-labelledby", s(p.r(&format!("{part}-legend")))),
                        ("aria-describedby", s(p.r(&format!("{part}-description")))),
                        ("data-tag-part", s("option")),
                        ("data-tag-option", s(option)),
                    ],
                );
                f.end_tag("fieldset");
            }
        }
        _ => {}
    }
    f.end_tag("div");
    f.open("footer", &[("class", s("insert-box"))]);
    let footer = if matches!(id, "quiz" | "submit") { p.insert_box_content() } else { p.insert_box_content() + &p.mail_tag_tip() };
    f.append_printed(&footer);
    kses(&f.output())
}

/// `WPCF7_TagGenerator::print_panels()`.
fn tag_generator_panels(site: &Site) -> String {
    let mut f = HtmlFormatter::new(false);
    for generator in GENERATORS {
        let content = format!("tag-generator-panel-{}", generator.id);
        f.open("dialog", &[("id", s(&content)), ("class", s("tag-generator-dialog"))]);
        f.open("button", &[("class", s("close-button")), ("title", s(tr(site, "Close this dialog box"))), ("data-taggen", s("close-dialog"))]);
        f.append_preformatted(&esc_html(&tr(site, "Close")));
        f.end_tag("button");
        f.open("form", &[("method", s("dialog")), ("class", s("tag-generator-panel")), ("data-id", s(generator.id)), ("data-version", s("2"))]);
        f.append_printed(&generator_panel(site, generator.id));
        f.close_all_tags();
    }
    rustpress_core::kses::kses(&f.output(), panels_allowed())
}
