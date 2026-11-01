//! `wpcf7_install()` (`load.php`) and `WPCF7_ContactFormTemplate` (`includes/contact-form-template.php`): on its
//! first activation the plugin stores its version and creates "Contact form 1" from the default template.

use rustpress_core::{Result, Site};
use rustpress_db::{Val, Writer};
use rustpress_phpser::{Key, Value};

use super::{DOMAIN, VERSION};

fn s(text: &str) -> Value {
    Value::Str(text.as_bytes().to_vec())
}

fn array(items: Vec<(&str, Value)>) -> Value {
    Value::Array(items.into_iter().map(|(k, v)| (Key::Str(k.as_bytes().to_vec()), v)).collect())
}

/// The translations of the template, in the language of the site.
struct Tr<'a>(&'a Site);

impl Tr<'_> {
    fn t(&self, text: &str) -> String {
        self.0.trd(DOMAIN, text)
    }

    fn x(&self, text: &str, context: &str) -> String {
        self.0.i18n().pgettext(DOMAIN, context, text)
    }
}

/// `WPCF7_ContactFormTemplate::from_email()`.
fn from_email(site: &Site) -> String {
    let admin_email = site.options.get("admin_email").unwrap_or_default().to_owned();
    let home = site.home_url("");
    let host = home.split_once("://").map(|(_, rest)| rest.split(['/', ':', '?', '#']).next().unwrap_or_default()).unwrap_or_default().to_lowercase();
    if host == "localhost" || host == "127.0.0.1" {
        return admin_email;
    }
    let sitename = host.strip_prefix("www.").unwrap_or(&host).to_owned();
    if admin_email.find('@').is_some_and(|at| admin_email[at..] == format!("@{sitename}")) {
        return admin_email;
    }
    format!("wordpress@{sitename}")
}

/// `WPCF7_ContactFormTemplate::form()`.
fn form(tr: &Tr<'_>) -> String {
    format!(
        "<label> {}\n    [text* your-name autocomplete:name] </label>\n\n<label> {}\n    [email* your-email autocomplete:email] </label>\n\n<label> {}\n    [text* your-subject] </label>\n\n<label> {} {}\n    [textarea your-message] </label>\n\n[submit \"{}\"]",
        tr.t("Your name"),
        tr.t("Your email"),
        tr.t("Subject"),
        tr.t("Your message"),
        tr.t("(optional)"),
        tr.t("Submit")
    )
}

fn subject(tr: &Tr<'_>) -> String {
    tr.x("%1$s \"%2$s\"", "mail subject").replacen("%1$s", "[_site_title]", 1).replacen("%2$s", "[your-subject]", 1)
}

/// `WPCF7_ContactFormTemplate::mail()`.
fn mail(tr: &Tr<'_>, sender: &str) -> Value {
    let body = format!(
        "{}\n{}\n\n{}\n[your-message]\n\n-- \n{}",
        tr.t("From: %s").replacen("%s", "[your-name] [your-email]", 1),
        tr.t("Subject: %s").replacen("%s", "[your-subject]", 1),
        tr.t("Message Body:"),
        tr.t("This is a notification that a contact form was submitted on your website (%1$s %2$s).").replacen("%1$s", "[_site_title]", 1).replacen("%2$s", "[_site_url]", 1)
    );
    array(vec![
        ("subject", s(&subject(tr))),
        ("sender", s(sender)),
        ("body", s(&body)),
        ("recipient", s("[_site_admin_email]")),
        ("additional_headers", s("Reply-To: [your-email]")),
        ("attachments", s("")),
        ("use_html", Value::Int(0)),
        ("exclude_blank", Value::Int(0)),
    ])
}

/// `WPCF7_ContactFormTemplate::mail_2()`.
fn mail_2(tr: &Tr<'_>, sender: &str) -> Value {
    let body = format!(
        "{}\n[your-message]\n\n-- \n{}",
        tr.t("Message Body:"),
        tr.t("This email is a receipt for your contact form submission on our website (%1$s %2$s) in which your email address was used. If that was not you, please ignore this message.")
            .replacen("%1$s", "[_site_title]", 1)
            .replacen("%2$s", "[_site_url]", 1)
    );
    array(vec![
        ("active", Value::Bool(false)),
        ("subject", s(&subject(tr))),
        ("sender", s(sender)),
        ("body", s(&body)),
        ("recipient", s("[your-email]")),
        ("additional_headers", s("Reply-To: [_site_admin_email]")),
        ("attachments", s("")),
        ("use_html", Value::Int(0)),
        ("exclude_blank", Value::Int(0)),
    ])
}

/// `WPCF7_ContactFormTemplate::messages()` as an activation sees them: the modules are not loaded yet, so only the
/// messages of the plugin and of `includes/file.php` are there.
fn messages(tr: &Tr<'_>) -> Value {
    let keys = [
        ("mail_sent_ok", "Thank you for your message. It has been sent."),
        ("mail_sent_ng", "There was an error trying to send your message. Please try again later."),
        ("validation_error", "One or more fields have an error. Please check and try again."),
        ("spam", "There was an error trying to send your message. Please try again later."),
        ("accept_terms", "You must accept the terms and conditions before sending your message."),
        ("invalid_required", "Please fill out this field."),
        ("invalid_too_long", "This field has a too long input."),
        ("invalid_too_short", "This field has a too short input."),
        ("upload_failed", "There was an unknown error uploading the file."),
        ("upload_file_type_invalid", "You are not allowed to upload files of this type."),
        ("upload_file_too_large", "The uploaded file is too large."),
        ("upload_failed_php_error", "There was an error uploading the file."),
    ];
    array(keys.iter().map(|(k, text)| (*k, s(&tr.t(text)))).collect())
}

/// `WPCF7_ContactForm::get_template()` in a request where every module is loaded (the editor of a new form): the
/// messages are all of `wpcf7_messages()`.
pub fn template(site: &Site) -> super::form::ContactForm {
    let tr = Tr(site);
    let sender = format!("[_site_title] <{}>", from_email(site));
    let messages = array(super::submission::DEFAULT_MESSAGES.iter().map(|(k, text)| (*k, s(&tr.t(text)))).collect());
    super::form::ContactForm {
        title: tr.t("Untitled"),
        locale: site.locale(),
        form: form(&tr),
        mail: Some(mail(&tr, &sender)),
        mail_2: Some(mail_2(&tr, &sender)),
        messages: Some(messages),
        ..super::form::ContactForm::default()
    }
}

/// `wpcf7_array_flatten()` of the properties, as `implode( "\n", ... )` writes them into `post_content`.
fn flatten(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Array(items) => items.iter().for_each(|(_, v)| flatten(v, out)),
        Value::Str(bytes) => out.push(String::from_utf8_lossy(bytes).into_owned()),
        Value::Int(n) => out.push(n.to_string()),
        Value::Bool(true) => out.push("1".to_owned()),
        _ => out.push(String::new()),
    }
}

/// `wp_unique_post_slug()` among the contact forms.
async fn unique_slug(site: &Site, writer: &Writer, slug: &str) -> Result<String> {
    let table = format!("{}posts", site.db.table_prefix());
    let mut candidate = slug.to_owned();
    let mut suffix = 2;
    loop {
        let taken = writer.column_u64(&format!("SELECT ID FROM {table} WHERE post_name = '{}' AND post_type = 'wpcf7_contact_form' LIMIT 1", candidate.replace('\'', "''"))).await?;
        if taken.is_empty() {
            return Ok(candidate);
        }
        candidate = format!("{slug}-{suffix}");
        suffix += 1;
    }
}

/// `WPCF7_ContactForm::save()` of a new form: the post, a meta row for each property, the locale and the hash.
/// Returns the ID of the post.
pub async fn store_new(site: &Site, writer: &Writer, user: u64, now: i64, title: &str, locale: &str, properties: &[(&str, Value)]) -> Result<u64> {
    let mut flat = Vec::new();
    for (_, value) in properties {
        flatten(value, &mut flat);
    }
    let content = flat.join("\n").trim().to_owned();
    let slug = unique_slug(site, writer, &rustpress_core::slug::sanitize_title_save(title, &site.locale())).await?;
    let local = site.timezone.format("Y-m-d H:i:s", now);
    let gmt = rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", now);
    let str_ = |v: &str| Val::Str(v.to_owned());
    let id = writer
        .insert_row(
            "posts",
            &[
                ("post_author", Val::Int(user as i64)),
                ("post_date", str_(&local)),
                ("post_date_gmt", str_(&gmt)),
                ("post_content", str_(&content)),
                ("post_title", str_(title)),
                ("post_excerpt", str_("")),
                ("post_status", str_("publish")),
                ("comment_status", str_("closed")),
                ("ping_status", str_("closed")),
                ("post_password", str_("")),
                ("post_name", str_(&slug)),
                ("to_ping", str_("")),
                ("pinged", str_("")),
                ("post_modified", str_(&local)),
                ("post_modified_gmt", str_(&gmt)),
                ("post_content_filtered", str_("")),
                ("post_parent", Val::Int(0)),
                ("guid", str_("")),
                ("menu_order", Val::Int(0)),
                ("post_type", str_(super::form::POST_TYPE)),
                ("post_mime_type", str_("")),
                ("comment_count", Val::Int(0)),
            ],
        )
        .await?;
    // `wp_insert_post()` names a post without a guid by its (unpretty) address.
    let guid = site.home_url(&format!("?post_type={}&p={id}", super::form::POST_TYPE));
    writer.update_rows("posts", "ID", id, &[("guid", Val::Str(guid))]).await?;
    for (name, value) in properties {
        let stored = match value {
            Value::Str(bytes) => normalize_newline(&String::from_utf8_lossy(bytes)),
            other => String::from_utf8_lossy(&rustpress_phpser::serialize(&normalize_newlines(other))).into_owned(),
        };
        writer.add_post_meta(id, &format!("_{name}"), &stored).await?;
    }
    if !locale.is_empty() {
        writer.add_post_meta(id, "_locale", locale).await?;
    }
    let hash = super::formatter::sha256_hex(&format!("{user}|{id}|{now}|{}", site.home_url("").trim_end_matches('/')));
    writer.add_post_meta(id, "_hash", &hash).await?;
    Ok(id)
}

/// `WPCF7_ContactForm::save()` of a stored form: `wp_update_post()` of the title and the content, the meta rows of the
/// properties and the locale; a hash only when the form has none yet.
#[allow(clippy::too_many_arguments)]
pub async fn update(site: &Site, writer: &Writer, user: u64, now: i64, id: u64, title: &str, locale: &str, properties: &[(&str, Value)]) -> Result<()> {
    let mut flat = Vec::new();
    for (_, value) in properties {
        flatten(value, &mut flat);
    }
    let content = flat.join("\n").trim().to_owned();
    let local = site.timezone.format("Y-m-d H:i:s", now);
    let gmt = rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", now);
    writer
        .update_rows(
            "posts",
            "ID",
            id,
            &[
                ("post_title", Val::Str(title.to_owned())),
                ("post_content", Val::Str(content)),
                ("post_status", Val::Str("publish".to_owned())),
                ("post_modified", Val::Str(local)),
                ("post_modified_gmt", Val::Str(gmt)),
            ],
        )
        .await?;
    for (name, value) in properties {
        let stored = match value {
            Value::Str(bytes) => normalize_newline(&String::from_utf8_lossy(bytes)),
            other => String::from_utf8_lossy(&rustpress_phpser::serialize(&normalize_newlines(other))).into_owned(),
        };
        writer.update_post_meta(id, &format!("_{name}"), &stored).await?;
    }
    if !locale.is_empty() {
        writer.update_post_meta(id, "_locale", locale).await?;
    }
    if site.db.post_meta(id).await?.iter().all(|m| m.meta_key.as_deref() != Some("_hash")) {
        let hash = super::formatter::sha256_hex(&format!("{user}|{id}|{now}|{}", site.home_url("").trim_end_matches('/')));
        writer.add_post_meta(id, "_hash", &hash).await?;
    }
    Ok(())
}

/// `wpcf7_normalize_newline()`.
fn normalize_newline(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `wpcf7_normalize_newline_deep()`.
fn normalize_newlines(value: &Value) -> Value {
    match value {
        Value::Str(bytes) => Value::Str(normalize_newline(&String::from_utf8_lossy(bytes)).into_bytes()),
        Value::Array(items) => Value::Array(items.iter().map(|(k, v)| (k.clone(), normalize_newlines(v))).collect()),
        other => other.clone(),
    }
}

/// `wpcf7_install()`.
pub async fn install(site: &Site, writer: &Writer, user: u64, now: i64) -> Result<()> {
    if site.options.fetch(&site.db, "wpcf7").await?.is_some_and(|v| !v.is_empty() && v != "0") {
        return Ok(());
    }
    // `wpcf7_upgrade()`: there is nothing to upgrade from, the version is stored.
    let mut option = vec![("version", s(VERSION))];
    writer.update_option("wpcf7", &String::from_utf8_lossy(&rustpress_phpser::serialize(&array(option.clone())))).await?;
    let table = format!("{}posts", site.db.table_prefix());
    if !writer.column_u64(&format!("SELECT ID FROM {table} WHERE post_type = 'wpcf7_contact_form' AND post_status = 'publish' LIMIT 1")).await?.is_empty() {
        return Ok(());
    }
    let tr = Tr(site);
    let title = tr.t("Contact form %d").replacen("%d", "1", 1);
    let sender = format!("[_site_title] <{}>", from_email(site));
    let properties = vec![
        ("form", s(&form(&tr))),
        ("mail", mail(&tr, &sender)),
        ("mail_2", mail_2(&tr, &sender)),
        ("messages", messages(&tr)),
        ("additional_settings", s("")),
    ];
    store_new(site, writer, user, now, &title, &site.locale(), &properties).await?;
    option.push(("bulk_validate", array(vec![("timestamp", Value::Int(now)), ("version", s(VERSION)), ("count_valid", Value::Int(1)), ("count_invalid", Value::Int(0))])));
    writer.update_option("wpcf7", &String::from_utf8_lossy(&rustpress_phpser::serialize(&array(option)))).await?;
    Ok(())
}
