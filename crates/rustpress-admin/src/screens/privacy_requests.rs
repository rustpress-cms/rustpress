//! `export-personal-data.php` and `erase-personal-data.php`: the requests for the personal data of a person
//! (the `user_request` posts), and what is done with them. RustPress cannot send e-mail, so a request is added
//! as confirmed, and the steps that need the person's confirmation are not offered.

use rustpress_core::OrLog;
use rustpress_core::datetime::SiteTimezone;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use crate::sanitize::{is_email, sanitize_email};
use rustpress_core::i18n::sprintf;
use rustpress_db::{Order, OrderBy, Post, PostQuery};

use super::notices::{SettingsError, render_settings_errors};
use super::{AdminResponse, Ctx, die, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::{Content, PostData};
use crate::list_table::{Column, Table, view_link, views};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Export,
    Erase,
}

impl Kind {
    /// The request type: the `post_name` of the request.
    fn request_type(self) -> &'static str {
        match self {
            Kind::Export => "export_personal_data",
            Kind::Erase => "remove_personal_data",
        }
    }

    fn file(self) -> &'static str {
        match self {
            Kind::Export => "export-personal-data.php",
            Kind::Erase => "erase-personal-data.php",
        }
    }
}

const STATUSES: [&str; 4] = ["request-pending", "request-confirmed", "request-failed", "request-completed"];

fn status_label(ctx: &Ctx<'_>, status: &str) -> String {
    let site = ctx.site();
    match status {
        "request-pending" => site.trx("Pending", "request status"),
        "request-confirmed" => site.trx("Confirmed", "request status"),
        "request-failed" => site.trx("Failed", "request status"),
        _ => site.trx("Completed", "request status"),
    }
}

fn status_count_label(ctx: &Ctx<'_>, status: &str, count: u64) -> String {
    let label = match status {
        "request-pending" => "Pending <span class=\"count\">(%s)</span>",
        "request-confirmed" => "Confirmed <span class=\"count\">(%s)</span>",
        "request-failed" => "Failed <span class=\"count\">(%s)</span>",
        _ => "Completed <span class=\"count\">(%s)</span>",
    };
    sprintf(&ctx.tr(label), &[&ctx.site().number_i18n(count)])
}

/// `get_timestamp_as_date()`.
fn timestamp_as_date(ctx: &Ctx<'_>, timestamp: i64) -> String {
    if timestamp == 0 {
        return String::new();
    }
    let site = ctx.site();
    let diff = ctx.env.now - timestamp;
    if (0..86_400).contains(&diff) {
        return sprintf(&ctx.tr("%s ago"), &[&super::media_lib::human_time_diff(ctx, timestamp, ctx.env.now)]);
    }
    sprintf(&ctx.tr("%1$s at %2$s"), &[&site.date(&ctx.tr("Y/m/d"), timestamp), &site.date(&ctx.tr("g:i a"), timestamp)])
}

fn help(ctx: &Ctx<'_>, kind: Kind) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let guide = sprintf(&tr("If you are not sure, check the plugin documentation or contact the plugin author to see if the plugin collects data and if it supports the Data Exporter tool. This information may be available in the <a href=\"%s\">Privacy Policy Guide</a>."), &[&ctx.admin_url("options-privacy.php?tab=policyguide")]);
    match kind {
        Kind::Export => Help {
            tabs: vec![
                HelpTab {
                    id: "overview".into(),
                    title: tr("Overview"),
                    content: format!(
                        "<p>{}</p><p>{}</p><p>{}</p><p><strong>{}</strong></p>",
                        tr("This screen is where you manage requests for an export of personal data."),
                        tr("Privacy Laws around the world require businesses and online services to provide an export of some of the data they collect about an individual, and to deliver that export on request. The rights those laws enshrine are sometimes called the \"Right of Data Portability\". It allows individuals to obtain and reuse their personal data for their own purposes across different services. It allows them to move, copy or transfer personal data easily from one IT environment to another."),
                        tr("The tool associates data stored in WordPress with a supplied email address, including profile data and comments."),
                        tr("Note: Since this tool only gathers data from WordPress and participating plugins, you may need to do more to comply with export requests. For example, you should also send the requester some of the data collected from or stored with the 3rd party services your organization uses.")
                    ),
                },
                HelpTab {
                    id: "default-data".into(),
                    title: tr("Default Data"),
                    content: format!(
                        "<p>{}</p><p>{}</p><p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
                        tr("WordPress collects (but <em>never</em> publishes) a limited amount of data from registered users who have logged in to the site. Generally, these users are people who contribute to the site in some way -- content, store management, and so on. With rare exceptions, these users do not include occasional visitors who might have registered to comment on articles or buy products. The data WordPress retains can include:"),
                        tr("<strong>Profile Information</strong> &mdash; user email address, username, display name, nickname, first name, last name, description/bio, and registration date."),
                        tr("<strong>Community Events Location</strong> &mdash; The IP Address of the user, which populates the Upcoming Community Events dashboard widget with relevant information."),
                        tr("<strong>Session Tokens</strong> &mdash; User login information, IP Addresses, Expiration Date, User Agent (Browser/OS), and Last Login."),
                        tr("<strong>Comments</strong> &mdash; For user comments, Email Address, IP Address, User Agent (Browser/OS), Date/Time, Comment Content, and Content URL."),
                        tr("<strong>Media</strong> &mdash; A list of URLs for media files the user uploads.")
                    ),
                },
                HelpTab {
                    id: "plugin-data".into(),
                    title: tr("Plugin Data"),
                    content: format!(
                        "<p>{}</p><p>{guide}</p><p>{}</p>",
                        tr("Many plugins may collect or store personal data either in the WordPress database or remotely. Any Export Personal Data request should include data from plugins as well."),
                        tr("If you are a plugin author, you can learn more about <a href=\"https://developer.wordpress.org/plugins/privacy/adding-the-personal-data-exporter-to-your-plugin/\">how to add the Personal Data Exporter to a plugin</a>.")
                    ),
                },
            ],
            sidebar: format!(
                "<p><strong>{}</strong></p><p>{}</p><p>{}</p>",
                tr("For more information:"),
                tr("<a href=\"https://wordpress.org/documentation/article/tools-export-personal-data-screen/\">Documentation on Export Personal Data</a>"),
                tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
            ),
        },
        Kind::Erase => Help {
            tabs: vec![
                HelpTab {
                    id: "overview".into(),
                    title: tr("Overview"),
                    content: format!(
                        "<p>{}</p><p>{}</p><p>{}</p><p><strong>{}</strong></p>",
                        tr("This screen is where you manage requests to erase personal data."),
                        tr("Privacy Laws around the world require businesses and online services to delete, anonymize, or forget the data they collect about an individual. The rights those laws enshrine are sometimes called the \"Right to be Forgotten\"."),
                        tr("The tool associates data stored in WordPress with a supplied email address, including profile data and comments."),
                        tr("Note: As this tool only gathers data from WordPress and participating plugins, you may need to do more to comply with erasure requests. For example, you are also responsible for ensuring that data collected by or stored with the 3rd party services your organization uses gets deleted.")
                    ),
                },
                HelpTab {
                    id: "default-data".into(),
                    title: tr("Default Data"),
                    content: format!(
                        "<p>{}</p><p>{}</p><p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
                        tr("WordPress collects (but <em>never</em> publishes) a limited amount of data from logged-in users but then deletes it or anonymizes it. That data can include:"),
                        tr("<strong>Profile Information</strong> &mdash; user email address, username, display name, nickname, first name, last name, description/bio, and registration date."),
                        tr("<strong>Community Events Location</strong> &mdash; The IP Address of the user which is used for the Upcoming Community Events shown in the dashboard widget."),
                        tr("<strong>Session Tokens</strong> &mdash; User login information, IP Addresses, Expiration Date, User Agent (Browser/OS), and Last Login."),
                        tr("<strong>Comments</strong> &mdash; WordPress does not delete comments. The software does anonymize (but, again, <em>never</em> publishes) the associated Email Address, IP Address, and User Agent (Browser/OS)."),
                        tr("<strong>Media</strong> &mdash; A list of URLs for all media file uploads made by the user.")
                    ),
                },
                HelpTab {
                    id: "plugin-data".into(),
                    title: tr("Plugin Data"),
                    content: format!(
                        "<p>{}</p><p>{}</p><p>{}</p>",
                        tr("Many plugins may collect or store personal data either in the WordPress database or remotely. Any Erase Personal Data request should delete data from plugins as well."),
                        sprintf(&tr("If you are not sure, check the plugin documentation or contact the plugin author to see if the plugin collects data and if it supports the Data Eraser tool. This information may be available in the <a href=\"%s\">Privacy Policy Guide</a>."), &[&ctx.admin_url("options-privacy.php?tab=policyguide")]),
                        tr("If you are a plugin author, you can learn more about <a href=\"https://developer.wordpress.org/plugins/privacy/adding-the-personal-data-eraser-to-your-plugin/\">how to add the Personal Data Eraser to a plugin</a>.")
                    ),
                },
            ],
            sidebar: format!(
                "<p><strong>{}</strong></p><p>{}</p><p>{}</p>",
                tr("For more information:"),
                tr("<a href=\"https://wordpress.org/documentation/article/tools-erase-personal-data-screen/\">Documentation on Erase Personal Data</a>"),
                tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
            ),
        },
    }
}

/// The `request_id[]` values of a bulk action or a row link (whose `request_id[0]` PHP reads into the same array).
fn request_ids(ctx: &Ctx<'_>) -> Vec<u64> {
    let request = ctx.request;
    let is_id = |k: &str| k == "request_id" || k.strip_prefix("request_id[").is_some_and(|rest| rest.ends_with(']'));
    request.form.iter().chain(request.query.iter()).filter(|(k, _)| is_id(k)).flat_map(|(_, v)| v.split(',').filter_map(|p| p.trim().parse::<u64>().ok()).collect::<Vec<_>>()).collect()
}

fn content<'a>(ctx: &'a Ctx<'_>) -> Option<Content<'a>> {
    Some(Content { site: ctx.site(), db: ctx.env.db, writer: ctx.env.writer?, user: &ctx.user, now: ctx.env.now })
}

/// `_wp_personal_data_cleanup_requests()`: requests nobody confirmed within a day have failed.
async fn cleanup(ctx: &Ctx<'_>, kind: Kind) {
    let Some(content) = content(ctx) else { return };
    let query = PostQuery { post_types: vec!["user_request".into()], names: vec![kind.request_type().into()], statuses: vec!["request-pending".into()], ..Default::default() };
    for post in ctx.env.db.posts(&query).await.unwrap_or_default() {
        let modified = SiteTimezone::gmt_timestamp(&post.post_modified_gmt).unwrap_or(0);
        if modified < ctx.env.now - 86_400 {
            content.insert_post(&PostData { id: Some(post.id), post_status: Some("request-failed".into()), post_password: Some(String::new()), ..Default::default() }).await.or_log();
        }
    }
}

/// `_wp_privacy_resend_request()`: the confirmation e-mail again, with a new key.
async fn resend(ctx: &Ctx<'_>, request_id: u64) -> Result<(), String> {
    if ctx.env.db.post(request_id).await.ok().flatten().filter(|p| p.post_type == "user_request").is_none() {
        return Err(ctx.tr("Invalid personal data request."));
    }
    ctx.env.mail_ctx().send_user_request(request_id).await
}

/// `_wp_personal_data_handle_actions()` and `process_bulk_action()`: the errors and messages they add.
async fn handle_actions(ctx: &Ctx<'_>, kind: Kind) -> Option<Result<Vec<SettingsError>, AdminResponse>> {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let mut errors: Vec<SettingsError> = Vec::new();
    let add = |errors: &mut Vec<SettingsError>, setting: &str, message: String, kind: &str| errors.push(SettingsError { setting: setting.to_owned(), code: setting.to_owned(), message, kind: kind.to_owned() });
    let ays = || die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired."));

    if request.post && request.form.iter().any(|(k, _)| k.starts_with("privacy_action_email_retry[")) {
        if !ctx.verify_nonce("bulk-privacy_requests") {
            return Some(Err(ays()));
        }
        let request_id = request.form.iter().find_map(|(k, _)| k.strip_prefix("privacy_action_email_retry[").and_then(|k| k.strip_suffix(']')).and_then(|k| k.parse::<u64>().ok())).unwrap_or(0);
        match resend(ctx, request_id).await {
            Ok(()) => add(&mut errors, "privacy_action_email_retry", tr("Confirmation request sent again successfully."), "success"),
            Err(message) => add(&mut errors, "privacy_action_email_retry", message, "error"),
        }
    } else if request.post && request.post_field("action").is_some_and(|a| !a.is_empty() && a != "-1") && request.post_field("action") != Some("-1") && matches!(request.post_field("action"), Some("add_export_personal_data_request" | "add_remove_personal_data_request")) {
        if !ctx.verify_nonce("personal-data-request") {
            return Some(Err(ays()));
        }
        let action_type = request.post_field("type_of_action").unwrap_or_default().trim().to_owned();
        let username_or_email = request.post_field("username_or_email_for_privacy_request").unwrap_or_default().trim().to_owned();
        let send_email = request.post_field("send_confirmation_email").is_some();
        if !["export_personal_data", "remove_personal_data"].contains(&action_type.as_str()) {
            add(&mut errors, "action_type", tr("Invalid personal data action."), "error");
        }
        let email = if is_email(&username_or_email) {
            sanitize_email(&username_or_email)
        } else {
            match ctx.env.db.user_by_login(&username_or_email).await.ok().flatten() {
                Some(user) => user.user_email,
                None => {
                    add(&mut errors, "username_or_email_for_privacy_request", tr("Unable to add this request. A valid email address or username must be supplied."), "error");
                    String::new()
                }
            }
        };
        if !email.is_empty() {
            {
                // Duplicates: an incomplete request of the same kind for the same address.
                let duplicate_query = PostQuery { post_types: vec!["user_request".into()], names: vec![action_type.clone()], statuses: vec!["request-pending".into(), "request-confirmed".into()], ..Default::default() };
                let duplicate = ctx.env.db.posts(&duplicate_query).await.unwrap_or_default().iter().any(|p| p.post_title == email);
                if duplicate {
                    add(&mut errors, "username_or_email_for_privacy_request", tr("An incomplete personal data request for this email address already exists."), "error");
                } else if let Some(content) = content(ctx) {
                    let user_id = ctx.env.db.user_by_email(&email).await.ok().flatten().map_or(0, |u| u.id);
                    let now = ctx.site().date("Y-m-d H:i:s", ctx.env.now);
                    let now_gmt = ctx.site().date_utc("Y-m-d H:i:s", ctx.env.now);
                    let data = PostData {
                        post_author: Some(user_id),
                        post_name: Some(action_type.clone()),
                        post_title: Some(email.clone()),
                        post_content: Some("[]".into()),
                        post_status: Some(if send_email { "request-pending" } else { "request-confirmed" }.into()),
                        post_type: Some("user_request".into()),
                        post_date: Some(now),
                        post_date_gmt: Some(now_gmt),
                        ..Default::default()
                    };
                    match content.insert_post(&data).await {
                        Ok(id) if id != 0 => {
                            if send_email {
                                // The result of the mail does not change the message, as in WordPress.
                                ctx.env.mail_ctx().send_user_request(id).await.or_log();
                                add(&mut errors, "username_or_email_for_privacy_request", tr("Confirmation request initiated successfully."), "success");
                            } else {
                                if let Some(writer) = ctx.env.writer {
                                    writer.add_post_meta(id, "_wp_user_request_confirmed_timestamp", &ctx.env.now.to_string()).await.or_log();
                                }
                                add(&mut errors, "username_or_email_for_privacy_request", tr("Request added successfully."), "success");
                            }
                        }
                        Ok(_) => add(&mut errors, "username_or_email_for_privacy_request", tr("Unable to initiate confirmation request."), "error"),
                        Err(e) => add(&mut errors, "username_or_email_for_privacy_request", e.to_string(), "error"),
                    }
                }
            }
        }
    }

    // `process_bulk_action()`: the action of the form or of the link.
    let ids = request_ids(ctx);
    if !ids.is_empty() {
        let action = request.post_field("action").filter(|a| !a.is_empty() && *a != "-1").or(request.get("action")).or_else(|| request.post_field("action2").filter(|a| !a.is_empty() && *a != "-1")).unwrap_or_default().to_owned();
        if !ctx.verify_nonce("bulk-privacy_requests") {
            return Some(Err(ays()));
        }
        let site = ctx.site();
        match action.as_str() {
            "resend" => {
                let (mut failures, mut count) = (0u64, 0u64);
                for id in &ids {
                    match resend(ctx, *id).await {
                        Ok(()) => count += 1,
                        Err(_) => failures += 1,
                    }
                }
                if failures > 0 {
                    add(&mut errors, "bulk_action", sprintf(&site.trn("%d confirmation request failed to resend.", "%d confirmation requests failed to resend.", failures as i64), &[&failures.to_string()]), "error");
                }
                if count > 0 {
                    add(&mut errors, "bulk_action", sprintf(&site.trn("%d confirmation request re-sent successfully.", "%d confirmation requests re-sent successfully.", count as i64), &[&count.to_string()]), "success");
                }
            }
            "complete" => {
                let mut count = 0u64;
                if let (Some(content), Some(writer)) = (content(ctx), ctx.env.writer) {
                    for id in &ids {
                        let Some(post) = ctx.env.db.post(*id).await.ok().flatten().filter(|p| p.post_type == "user_request") else { continue };
                        writer.update_post_meta(post.id, "_wp_user_request_completed_timestamp", &ctx.env.now.to_string()).await.or_log();
                        if content.insert_post(&PostData { id: Some(post.id), post_status: Some("request-completed".into()), ..Default::default() }).await.is_ok() {
                            count += 1;
                        }
                    }
                }
                add(&mut errors, "bulk_action", sprintf(&site.trn("%d request marked as complete.", "%d requests marked as complete.", count as i64), &[&count.to_string()]), "success");
            }
            "delete" => {
                let (mut count, mut failures) = (0u64, 0u64);
                if let Some(content) = content(ctx) {
                    for id in &ids {
                        match ctx.env.db.post(*id).await.ok().flatten().filter(|p| p.post_type == "user_request") {
                            Some(_) if content.delete_post(*id, true).await.unwrap_or(false) => count += 1,
                            _ => failures += 1,
                        }
                    }
                }
                if failures > 0 {
                    add(&mut errors, "bulk_action", sprintf(&site.trn("%d request failed to delete.", "%d requests failed to delete.", failures as i64), &[&failures.to_string()]), "error");
                }
                if count > 0 {
                    add(&mut errors, "bulk_action", sprintf(&site.trn("%d request deleted successfully.", "%d requests deleted successfully.", count as i64), &[&count.to_string()]), "success");
                }
            }
            _ => {}
        }
    }
    let _ = kind;
    Some(Ok(errors))
}

/// One row of the list.
async fn row(ctx: &Ctx<'_>, kind: Kind, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let id = post.id;
    let status = post.post_status.as_str();
    let email = &post.post_title;
    let file = kind.file();
    let meta = |key: &'static str| {
        let db = ctx.env.db;
        async move { db.post_meta_value(id, key).await.ok().flatten().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0) }
    };
    let created = SiteTimezone::gmt_timestamp(&post.post_date_gmt).unwrap_or(0);
    let nonce_link = |action: &str| {
        let url = format!("{}?action={action}&request_id%5B0%5D={id}&_wpnonce={}", ctx.admin_url(file), ctx.create_nonce("bulk-privacy_requests"));
        esc_url(&url)
    };

    // The requester column with the actions.
    let mut actions: Vec<(String, String)> = Vec::new();
    match kind {
        Kind::Export => {
            let nonce = ctx.create_nonce(&format!("wp-privacy-export-personal-data-{id}"));
            actions.push((
                "download-data".into(),
                format!(
                    "<span class=\"export-personal-data\" data-exporters-count=\"3\" data-request-id=\"{id}\" data-nonce=\"{nonce}\"><span class=\"export-personal-data-idle\"><button type=\"button\" class=\"button-link export-personal-data-handle\">{}</button></span><span class=\"export-personal-data-processing hidden\">{} <span class=\"export-progress\"></span></span><span class=\"export-personal-data-success hidden\"><button type=\"button\" class=\"button-link export-personal-data-handle\">{}</button></span><span class=\"export-personal-data-failed hidden\">{} <button type=\"button\" class=\"button-link export-personal-data-handle\">{}</button></span></span>",
                    tr("Download personal data"),
                    tr("Downloading data..."),
                    tr("Download personal data again"),
                    tr("Download failed."),
                    tr("Retry")
                ),
            ));
        }
        Kind::Erase => {
            if status != "request-confirmed" {
                let nonce = ctx.create_nonce(&format!("wp-privacy-erase-personal-data-{id}"));
                actions.push((
                    "remove-data".into(),
                    format!(
                        "<span class=\"remove-personal-data force-remove-personal-data\" data-erasers-count=\"1\" data-request-id=\"{id}\" data-nonce=\"{nonce}\"><span class=\"remove-personal-data-idle\"><button type=\"button\" class=\"button-link remove-personal-data-handle\">{}</button></span><span class=\"remove-personal-data-processing hidden\">{} <span class=\"erasure-progress\"></span></span><span class=\"remove-personal-data-success hidden\">{}</span><span class=\"remove-personal-data-failed hidden\">{} <button type=\"button\" class=\"button-link remove-personal-data-handle\">{}</button></span></span>",
                        tr("Force erase personal data"),
                        tr("Erasing data..."),
                        tr("Erasure completed."),
                        tr("Force erasure has failed."),
                        tr("Retry")
                    ),
                ));
            }
        }
    }
    if status != "request-completed" {
        actions.push((
            "complete-request".into(),
            format!("<span><a href=\"{}\" class=\"complete-request\" aria-label=\"{}\">{}</a></span>", nonce_link("complete"), esc_attr(&sprintf(&tr("Mark export request for &#8220;%s&#8221; as completed."), &[email])), tr("Complete request")),
        ));
    }
    let row_actions = {
        let mut out = String::from("<div class=\"row-actions\">");
        for (index, (class, html)) in actions.iter().enumerate() {
            out.push_str(&format!("<span class='{class}'>{html}{}</span>", if index + 1 < actions.len() { " | " } else { "" }));
        }
        out.push_str("</div>");
        out
    };
    let toggle = format!("<button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button>", tr("Show more details"));
    let email_html = format!("<a href=\"{}\">{}</a> {row_actions}", esc_url(&format!("mailto:{email}")), esc_html(email));

    // The status.
    let timestamp = match status {
        "request-confirmed" => meta("_wp_user_request_confirmed_timestamp").await,
        "request-completed" => meta("_wp_user_request_completed_timestamp").await,
        _ => 0,
    };
    let mut status_html = format!("<span class=\"status-label status-{}\">{}", esc_attr(status), esc_html(&status_label(ctx, status)));
    if timestamp != 0 {
        status_html.push_str(&format!("<span class=\"status-date\">{}</span>", timestamp_as_date(ctx, timestamp)));
    }
    status_html.push_str("</span>");

    // The next steps.
    let next = match status {
        "request-pending" => esc_html(&tr("Waiting for confirmation")),
        "request-confirmed" => match kind {
            Kind::Export => {
                let nonce = ctx.create_nonce(&format!("wp-privacy-export-personal-data-{id}"));
                format!(
                    "<div class=\"export-personal-data\" data-send-as-email=\"1\" data-exporters-count=\"3\" data-request-id=\"{id}\" data-nonce=\"{nonce}\">\t\t\t\t<span class=\"export-personal-data-idle\"><button type=\"button\" class=\"button-link export-personal-data-handle\">{}</button></span>\n\t\t\t\t<span class=\"export-personal-data-processing hidden\">{} <span class=\"export-progress\"></span></span>\n\t\t\t\t<span class=\"export-personal-data-success success-message hidden\">{}</span>\n\t\t\t\t<span class=\"export-personal-data-failed hidden\">{} <button type=\"button\" class=\"button-link export-personal-data-handle\">{}</button></span>\n\t\t\t\t</div>",
                    tr("Send export link"),
                    tr("Sending email..."),
                    tr("Email sent."),
                    tr("Email could not be sent."),
                    tr("Retry")
                )
            }
            Kind::Erase => {
                let nonce = ctx.create_nonce(&format!("wp-privacy-erase-personal-data-{id}"));
                format!(
                    "<div class=\"remove-personal-data\" data-force-erase=\"1\" data-erasers-count=\"1\" data-request-id=\"{id}\" data-nonce=\"{nonce}\">\t\t\t\t<span class=\"remove-personal-data-idle\"><button type=\"button\" class=\"button-link remove-personal-data-handle\">{}</button></span>\n\t\t\t\t<span class=\"remove-personal-data-processing hidden\">{} <span class=\"erasure-progress\"></span></span>\n\t\t\t\t<span class=\"remove-personal-data-success success-message hidden\" >{}</span>\n\t\t\t\t<span class=\"remove-personal-data-failed hidden\">{} <button type=\"button\" class=\"button-link remove-personal-data-handle\">{}</button></span>\n\t\t\t\t</div>",
                    tr("Erase personal data"),
                    tr("Erasing data..."),
                    tr("Erasure completed."),
                    tr("Data erasure has failed."),
                    tr("Retry")
                )
            }
        },
        "request-failed" => format!("<button type=\"submit\" class=\"button-link\" name=\"privacy_action_email_retry[{id}]\" id=\"privacy_action_email_retry[{id}]\">{}</button>", tr("Retry")),
        "request-completed" => format!("<a href=\"{}\">{}</a>", nonce_link("delete"), esc_html(&tr("Remove request"))),
        _ => String::new(),
    };
    format!(
        "<tr id=\"request-{id}\" class=\"status-{}\"><td class=\"check-column\"><input type=\"checkbox\" name=\"request_id[]\" id=\"requester_{id}\" value=\"{id}\" /><label for=\"requester_{id}\"><span class=\"screen-reader-text\">{}</span></label><span class=\"spinner\"></span></td><th class='email column-email has-row-actions column-primary' data-colname=\"{}\" scope=\"row\" aria-label=\"{}\">{email_html}{toggle}{toggle}</th><td class='status column-status' data-colname=\"{}\">{status_html}</td><td class='created_timestamp column-created_timestamp' data-colname=\"{}\">{}</td><td class='next_steps column-next_steps' data-colname=\"{}\">{next}</td></tr>",
        esc_attr(status),
        sprintf(&tr("Select %s"), &[&esc_html(email)]),
        tr("Requester"),
        esc_attr(email),
        tr("Status"),
        tr("Requested"),
        timestamp_as_date(ctx, created),
        tr("Next steps")
    )
}

pub async fn page(ctx: &Ctx<'_>, kind: Kind) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    let (cap, denied) = match kind {
        Kind::Export => ("export_others_personal_data", "Sorry, you are not allowed to export personal data on this site."),
        Kind::Erase => ("erase_others_personal_data", "Sorry, you are not allowed to erase personal data on this site."),
    };
    if !ctx.can(cap) {
        return super::common::die_plain(ctx, denied);
    }
    let errors = match handle_actions(ctx, kind).await {
        Some(Ok(errors)) => errors,
        Some(Err(response)) => return response,
        None => Vec::new(),
    };
    cleanup(ctx, kind).await;

    let db = ctx.env.db;
    let per_page_option = format!("{}_requests_per_page", kind.request_type());
    let per_page = super::common::items_per_page(ctx, &per_page_option, 20).await;
    let paged = request.request("paged").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
    let filter_status = request.request("filter-status").unwrap_or_default().to_owned();
    let orderby_param = request.request("orderby").unwrap_or_default().to_owned();
    let order_param = request.request("order").unwrap_or_default().to_ascii_uppercase();
    let order_by = match orderby_param.as_str() {
        "requester" => OrderBy::Title,
        _ => OrderBy::Date,
    };
    let desc = !(order_param == "ASC");
    let mut query = PostQuery {
        post_types: vec!["user_request".into()],
        names: vec![kind.request_type().into()],
        statuses: if filter_status.is_empty() { Vec::new() } else { vec![filter_status.clone()] },
        order_by,
        order: if desc { Order::Desc } else { Order::Asc },
        ..Default::default()
    };
    let total = db.count_posts(&query).await.unwrap_or(0);
    query.limit = Some(per_page);
    query.offset = (paged - 1).saturating_mul(per_page);
    let posts = db.posts(&query).await.unwrap_or_default();

    // The views with the numbers per status.
    let mut counts: Vec<(&str, u64)> = Vec::new();
    for status in STATUSES {
        let q = PostQuery { post_types: vec!["user_request".into()], names: vec![kind.request_type().into()], statuses: vec![status.into()], ..Default::default() };
        counts.push((status, db.count_posts(&q).await.unwrap_or(0)));
    }
    let all: u64 = counts.iter().map(|(_, n)| *n).sum();
    let admin_url = ctx.admin_url(kind.file());
    let mut links: Vec<(String, String)> = vec![(
        "all".into(),
        view_link(&admin_url, &sprintf(&ctx.site().trnx("All <span class=\"count\">(%s)</span>", "All <span class=\"count\">(%s)</span>", all as i64, "requests"), &[&ctx.site().number_i18n(all)]), filter_status.is_empty()),
    )];
    for (status, count) in &counts {
        if *count == 0 {
            continue;
        }
        links.push(((*status).to_owned(), view_link(&format!("{admin_url}?filter-status={status}"), &status_count_label(ctx, status, *count), filter_status == *status)));
    }

    let (heading, intro, form_heading, email_label, action, type_of_action) = match kind {
        Kind::Export => ("Export Personal Data", "This tool helps site owners comply with local laws and regulations by exporting known data for a given user in a .zip file.", "Add Data Export Request", "Send personal data export confirmation email.", "add_export_personal_data_request", "export_personal_data"),
        Kind::Erase => ("Erase Personal Data", "This tool helps site owners comply with local laws and regulations by deleting or anonymizing known data for a given user.", "Add Data Erasure Request", "Send personal data erasure confirmation email.", "add_remove_personal_data_request", "remove_personal_data"),
    };
    let has_items = !posts.is_empty();
    let mut rows = String::new();
    for post in &posts {
        rows.push_str(&row(ctx, kind, post).await);
    }
    if rows.is_empty() {
        rows = format!("<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"5\">{}</td></tr>", tr("No items found."));
    }
    let mut columns = vec![
        Column::new("cb", ""),
        Column::new("email", &tr("Requester")).sortable("requester", request.request("orderby").is_some(), None, None),
        Column::new("status", &tr("Status")),
        Column::new("created_timestamp", &tr("Requested")).sortable("requested", request.request("orderby").is_some(), None, None),
        Column::new("next_steps", &tr("Next steps")),
    ];
    let screen_options = super::common::list_screen_options(ctx, kind.file().trim_end_matches(".php"), &mut columns, (&per_page_option, per_page), None).await;
    let table = Table {
        site: ctx.site(),
        plural: "privacy_requests".into(),
        singular: "privacy_request".into(),
        columns,
        primary: "email".into(),
        current_url: format!("{}{}", super::request_origin(ctx.env, ctx.request), request.request_uri),
        orderby: orderby_param.clone(),
        order: if order_param.is_empty() { String::new() } else { order_param.to_ascii_lowercase() },
        total_items: total,
        per_page,
        current_page: paged,
        bulk_actions: vec![("resend".into(), tr("Resend confirmation requests")), ("complete".into(), tr("Mark requests as completed")), ("delete".into(), tr("Delete requests"))],
        has_items,
        view_mode: super::common::list_mode(ctx).await,
    };
    let list_heading = match kind {
        Kind::Export => tr("Export personal data list"),
        Kind::Erase => tr("Erase personal data list"),
    };
    let nonce_field = ctx.nonce_field("bulk-privacy_requests");
    let list_html = table.display(&nonce_field, "", "", "", &rows, &list_heading);
    let views_heading = match kind {
        Kind::Export => tr("Filter export personal data list"),
        Kind::Erase => tr("Filter erase personal data list"),
    };
    let search_box = if has_items || !request.request("s").unwrap_or_default().is_empty() {
        let mut hidden = String::new();
        for name in ["orderby", "order"] {
            if let Some(value) = request.request(name).filter(|v| !v.is_empty()) {
                hidden.push_str(&format!("<input type=\"hidden\" name=\"{name}\" value=\"{}\" />\n", esc_attr(value)));
            }
        }
        format!(
            "{hidden}<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"requests-search-input\">{0}:</label>\n\t<input type=\"search\" id=\"requests-search-input\" name=\"s\" value=\"{1}\" />\n\t\t<input type=\"submit\" id=\"search-submit\" class=\"button button-compact\" value=\"{0}\"  /></p>\n\t\t",
            tr("Search Requests"),
            esc_attr(request.request("s").unwrap_or_default())
        )
    } else {
        String::new()
    };
    let h = format!(
        "<div class=\"wrap nosubsub\">\n\t<h1>{}</h1>\n\t<p>{}</p>\n\t<hr class=\"wp-header-end\" />\n\n\t{}\n\t<form action=\"{}\" method=\"post\" class=\"wp-privacy-request-form\">\n\t\t<h2>{}</h2>\n\t\t<div class=\"wp-privacy-request-form-field\">\n{}<table class=\"form-table\">\n\t\t\t\t<tr>\n\t\t\t\t\t<th scope=\"row\">\n\t\t\t\t\t\t<label for=\"username_or_email_for_privacy_request\">{}</label>\n\t\t\t\t\t</th>\n\t\t\t\t\t<td>\n\t\t\t\t\t\t<input type=\"text\" required class=\"regular-text ltr\" id=\"username_or_email_for_privacy_request\" name=\"username_or_email_for_privacy_request\" />\n\t\t\t\t\t</td>\n\t\t\t\t</tr>\n\t\t\t\t<tr>\n\t\t\t\t\t<th scope=\"row\">\n\t\t\t\t\t\t{}\t\t\t\t\t</th>\n\t\t\t\t\t<td>{}\n\t\t\t\t\t\t<input type=\"checkbox\" name=\"send_confirmation_email\" id=\"send_confirmation_email\" value=\"1\" checked=\"checked\" />\n\t\t\t\t\t\t<label for=\"send_confirmation_email\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</label>\n\t\t\t\t\t</td>\n\t\t\t\t</tr>\n\t\t\t</table>\n\t\t\t<p class=\"submit\">\n\t\t\t\t{}\t\t\t</p>\n\t\t</div>\n\t\t{}\t\t<input type=\"hidden\" name=\"action\" value=\"{action}\" />\n\t\t<input type=\"hidden\" name=\"type_of_action\" value=\"{type_of_action}\" />\n\t</form>\n\t<hr />\n\n\t{}\n\t<form class=\"search-form wp-clearfix\">\n\t\t{}\t\t<input type=\"hidden\" name=\"filter-status\" value=\"{}\" />\n\t\t<input type=\"hidden\" name=\"orderby\" value=\"{}\" />\n\t\t<input type=\"hidden\" name=\"order\" value=\"{}\" />\n\t</form>\n\n\t<form method=\"post\">\n\t\t{}\t\t</form>\n</div>\n\n",
        esc_html(&tr(heading)),
        tr(intro),
        render_settings_errors(&errors, false),
        esc_url(&ctx.admin_url(kind.file())),
        esc_html(&tr(form_heading)),
        if kind == Kind::Erase { "\t\t\t" } else { "\t\t" },
        esc_html(&tr("Username or email address")),
        tr("Confirmation email"),
        if kind == Kind::Erase { "\t" } else { "" },
        tr(email_label),
        format!("<input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button\" value=\"{}\"  />", esc_attr(&tr("Send Request"))),
        ctx.nonce_field("personal-data-request"),
        views(&views_heading, &links),
        search_box,
        esc_attr(&filter_status),
        esc_attr(&orderby_param),
        esc_attr(request.request("order").unwrap_or_default()),
        list_html.trim_end_matches(['\n', '\t', ' ']).to_owned() + "\n\t"
    );
    let screen = Screen { file: kind.file().into(), parent_file: "tools.php".into(), submenu_file: Some(kind.file().into()), title: tr(heading), help: Some(help(ctx, kind)), options: Some(screen_options), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["privacy-tools".into()], ..Default::default() }).await
}
