//! `wp-admin/options.php`: saves the settings of the settings screens.

use rustpress_core::formatting::{esc_html, esc_url_raw, strip_tags};
use rustpress_phpser::{Key, Value};

use super::notices::SettingsError;
use super::{AdminResponse, Ctx, die, redirect, timezones};

/// The options each settings page may save (`$allowed_options`).
fn allowed_options(page: &str) -> Option<Vec<&'static str>> {
    let list: Vec<&'static str> = match page {
        "general" => vec!["blogname", "blogdescription", "site_icon", "gmt_offset", "date_format", "time_format", "start_of_week", "timezone_string", "WPLANG", "new_admin_email", "siteurl", "home", "users_can_register", "default_role"],
        "discussion" => vec![
            "default_pingback_flag",
            "default_ping_status",
            "default_comment_status",
            "comments_notify",
            "moderation_notify",
            "comment_moderation",
            "require_name_email",
            "comment_previously_approved",
            "comment_max_links",
            "moderation_keys",
            "disallowed_keys",
            "show_avatars",
            "avatar_rating",
            "avatar_default",
            "close_comments_for_old_posts",
            "close_comments_days_old",
            "thread_comments",
            "thread_comments_depth",
            "page_comments",
            "comments_per_page",
            "default_comments_page",
            "comment_order",
            "comment_registration",
            "show_comments_cookies_opt_in",
            "wp_notes_notify",
        ],
        "media" => vec!["thumbnail_size_w", "thumbnail_size_h", "thumbnail_crop", "medium_size_w", "medium_size_h", "large_size_w", "large_size_h", "image_default_size", "image_default_align", "image_default_link_type", "uploads_use_yearmonth_folders"],
        "reading" => vec!["posts_per_page", "posts_per_rss", "rss_use_excerpt", "show_on_front", "page_on_front", "page_for_posts", "blog_public"],
        "writing" => vec!["default_category", "default_email_category", "default_link_category", "default_post_format", "mailserver_url", "mailserver_port", "mailserver_login", "mailserver_pass", "ping_sites"],
        "misc" | "privacy" => vec![],
        _ => return None,
    };
    Some(list)
}

/// `absint()`.
fn absint(value: &str) -> String {
    let number = value.trim().trim_start_matches('+');
    let digits: String = number.trim_start_matches('-').chars().take_while(char::is_ascii_digit).collect();
    let parsed: i64 = digits.parse().unwrap_or(0);
    parsed.to_string()
}

/// `is_email()`: the shape WordPress accepts.
pub fn is_email(email: &str) -> bool {
    if email.len() < 6 || email.len() > 254 {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else { return false };
    if local.is_empty() || domain.contains('@') {
        return false;
    }
    if !local.chars().all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c)) {
        return false;
    }
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|p| !p.is_empty() && !p.starts_with('-') && !p.ends_with('-') && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
}

/// `sanitize_email()`: the cleaned address, or an empty string when nothing valid is left.
pub fn sanitize_email(email: &str) -> String {
    let email = email.trim();
    if email.len() < 6 {
        return String::new();
    }
    let Some((local, domain)) = email.split_once('@') else { return String::new() };
    let local: String = local.chars().filter(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(*c)).collect();
    if local.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = domain.split('.').map(|p| p.trim_matches('-').chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect::<String>()).filter(|p: &String| !p.is_empty()).collect();
    if parts.len() < 2 {
        return String::new();
    }
    format!("{local}@{}", parts.join("."))
}

/// What `sanitize_option()` made of a value: the value to store, or the error message.
fn sanitize(ctx: &Ctx<'_>, option: &str, value: &str) -> Result<String, String> {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    Ok(match option {
        "admin_email" | "new_admin_email" => {
            let cleaned = sanitize_email(value);
            if !is_email(&cleaned) {
                return Err(tr("The email address entered did not appear to be a valid email address. Please enter a valid email address."));
            }
            cleaned
        }
        "thumbnail_size_w" | "thumbnail_size_h" | "medium_size_w" | "medium_size_h" | "medium_large_size_w" | "medium_large_size_h" | "large_size_w" | "large_size_h" | "mailserver_port" | "comment_max_links" | "page_on_front" | "page_for_posts"
        | "rss_excerpt_length" | "default_category" | "default_email_category" | "default_link_category" | "close_comments_days_old" | "comments_per_page" | "thread_comments_depth" | "users_can_register" | "start_of_week" | "site_icon"
        | "fileupload_maxk" => absint(value),
        "posts_per_page" | "posts_per_rss" => {
            let mut number: i64 = value.trim().parse().unwrap_or(0);
            if number == 0 {
                number = 1;
            }
            if number < -1 {
                number = number.abs();
            }
            number.to_string()
        }
        "default_ping_status" | "default_comment_status" => {
            if value == "0" || value.is_empty() { "closed".to_owned() } else { value.to_owned() }
        }
        "blogname" | "blogdescription" => esc_html(value),
        "blog_public" => value.trim().parse::<i64>().unwrap_or(0).to_string(),
        "date_format" | "time_format" | "mailserver_url" | "mailserver_login" | "mailserver_pass" | "upload_path" => strip_tags(value),
        "gmt_offset" => {
            if value.trim().parse::<f64>().is_ok() { value.chars().filter(|c| c.is_ascii_digit() || matches!(c, ':' | '.' | '-')).collect() } else { String::new() }
        }
        "siteurl" | "home" => {
            let lower = value.to_lowercase();
            if lower.starts_with("http://") && value.len() > 7 || lower.starts_with("https://") && value.len() > 8 {
                esc_url_raw(value)
            } else if option == "siteurl" {
                return Err(tr("The WordPress address you entered did not appear to be a valid URL. Please enter a valid URL."));
            } else {
                return Err(tr("The Site address you entered did not appear to be a valid URL. Please enter a valid URL."));
            }
        }
        "WPLANG" => {
            let allowed = value.is_empty() || ctx.env.languages.iter().any(|l| l.locale == value);
            if allowed { value.to_owned() } else { site.options.string("WPLANG", "") }
        }
        "timezone_string" => {
            if !value.is_empty() && !timezones::is_valid(value) {
                return Err(tr("The timezone you have entered is not valid. Please select a valid timezone."));
            }
            value.to_owned()
        }
        "moderation_keys" | "disallowed_keys" => {
            let mut seen: Vec<String> = Vec::new();
            for line in value.split('\n') {
                let line = line.trim().to_owned();
                if !line.is_empty() && !seen.contains(&line) {
                    seen.push(line);
                }
            }
            seen.join("\n")
        }
        _ => value.to_owned(),
    })
}

/// `serialize()` of the settings errors, for the `settings_errors` transient.
fn serialize_errors(errors: &[SettingsError]) -> String {
    let entry = |e: &SettingsError| {
        let field = |k: &str, v: &str| (Key::Str(k.as_bytes().to_vec()), Value::Str(v.as_bytes().to_vec()));
        Value::Array(vec![field("setting", &e.setting), field("code", &e.code), field("message", &e.message), field("type", &e.kind)])
    };
    let list = Value::Array(errors.iter().enumerate().map(|(i, e)| (Key::Int(i as i64), entry(e))).collect());
    String::from_utf8_lossy(&rustpress_phpser::serialize(&list)).into_owned()
}

/// `wp_nonce_ays()`: the page for a missing or stale nonce.
fn nonce_ays(ctx: &Ctx<'_>) -> AdminResponse {
    let mut html = ctx.tr("The link you followed has expired.");
    if let Some(referer) = referer(ctx) {
        html.push_str(&format!("</p><p><a href=\"{}\">{}</a>", rustpress_core::formatting::esc_url(&referer), ctx.tr("Please try again.")));
    }
    die(ctx.env, 403, &ctx.tr("WordPress Failure Notice"), &html)
}

/// `wp_get_referer()`: the page the form came from.
fn referer(ctx: &Ctx<'_>) -> Option<String> {
    let value = ctx.request.request("_wp_http_referer").filter(|r| !r.is_empty())?;
    // A path on this site, or an address on it.
    if value.starts_with('/') && !value.starts_with("//") {
        return Some(value.to_owned());
    }
    let site = &ctx.site().settings.siteurl;
    let host = |url: &str| url.split_once("://").map(|(_, rest)| rest.split('/').next().unwrap_or_default().to_owned());
    (host(value) == host(site)).then(|| value.to_owned())
}

/// `add_query_arg( 'settings-updated', 'true', $url )`.
fn with_settings_updated(url: &str) -> String {
    let (base, fragment) = match url.split_once('#') {
        Some((base, fragment)) => (base, Some(fragment)),
        None => (url, None),
    };
    let (path, query) = match base.split_once('?') {
        Some((path, query)) => (path, query),
        None => (base, ""),
    };
    let mut pairs: Vec<String> = query.split('&').filter(|p| !p.is_empty() && !p.starts_with("settings-updated=")).map(str::to_owned).collect();
    pairs.push("settings-updated=true".to_owned());
    let mut out = format!("{path}?{}", pairs.join("&"));
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

pub async fn handle(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let action = request.request("action").unwrap_or_default().to_owned();
    let option_page = request.request("option_page").filter(|p| !p.is_empty()).unwrap_or("options").to_owned();
    if !ctx.can("manage_options") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", ctx.tr("You need a higher level of permission."), ctx.tr("Sorry, you are not allowed to manage options for this site.")));
    }
    let Some(writer) = ctx.env.writer else {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>");
    };
    if let Some(hash) = request.get("adminhash").filter(|h| !h.is_empty()) {
        let stored = ctx.site().options.fetch(ctx.env.db, "adminhash").await.ok().flatten().and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok());
        let mut target = "options-general.php?updated=false";
        if let Some(stored) = stored
            && stored.get("hash").and_then(Value::as_str) == Some(hash)
            && let Some(email) = stored.get("newemail").and_then(Value::as_str).filter(|e| !e.is_empty())
        {
            let _ = writer.update_option("admin_email", email).await;
            let _ = writer.delete_option("adminhash").await;
            let _ = writer.delete_option("new_admin_email").await;
            target = "options-general.php?updated=true";
        }
        return redirect(&ctx.admin_url(target));
    }
    if request.get("dismiss") == Some("new_admin_email") {
        if !ctx.verify_nonce("dismiss-1-new_admin_email") {
            return nonce_ays(ctx);
        }
        let _ = writer.delete_option("adminhash").await;
        let _ = writer.delete_option("new_admin_email").await;
        return redirect(&ctx.admin_url("options-general.php?updated=true"));
    }
    if action != "update" || !request.post {
        return die(ctx.env, 404, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr("The requested page does not exist.")));
    }
    if !ctx.verify_nonce(&format!("{option_page}-options")) {
        return nonce_ays(ctx);
    }
    let Some(options) = allowed_options(&option_page) else {
        return die(ctx.env, 200, "WordPress &rsaquo; Error", &rustpress_core::i18n::sprintf(&ctx.tr("<strong>Error:</strong> The %s options page is not in the allowed options list."), &[&format!("<code>{}</code>", esc_html(&option_page))]));
    };

    // The form's own fixes before the options are read.
    let mut form: Vec<(String, String)> = request.form.clone();
    let set = |form: &mut Vec<(String, String)>, name: &str, value: String| {
        form.retain(|(k, _)| k != name);
        form.push((name.to_owned(), value));
    };
    let field = |form: &[(String, String)], name: &str| form.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    let mut errors: Vec<SettingsError> = Vec::new();
    let site = ctx.site();
    if option_page == "general" {
        for (name, custom) in [("date_format", "date_format_custom"), ("time_format", "time_format_custom")] {
            if field(&form, name).is_some_and(|v| v == "\\c\\u\\s\\t\\o\\m")
                && let Some(value) = field(&form, custom)
            {
                set(&mut form, name, value);
            }
        }
        if let Some(zone) = field(&form, "timezone_string").filter(|z| !z.is_empty() && (z.starts_with("UTC+") || z.starts_with("UTC-"))) {
            let offset = zone.replacen("UTC+", "", 1).replacen("UTC", "", 1);
            set(&mut form, "gmt_offset", offset);
            set(&mut form, "timezone_string", String::new());
        } else if let Some(zone) = field(&form, "timezone_string")
            && !timezones::is_valid(&zone)
        {
            let current = site.options.string("timezone_string", "");
            if current.is_empty() {
                set(&mut form, "gmt_offset", site.options.string("gmt_offset", ""));
                set(&mut form, "timezone_string", String::new());
            } else {
                set(&mut form, "timezone_string", current);
            }
            errors.push(SettingsError { setting: "general".into(), code: "settings_updated".into(), message: ctx.tr("The timezone you have entered is not valid. Please select a valid timezone."), kind: "error".into() });
        }
    }

    let mut admin_email_changed: Option<String> = None;
    for option in options {
        let supplied = field(&form, option);
        let value = supplied.clone().unwrap_or_default().trim().to_owned();
        // A checkbox that is not sent stores its empty value.
        let new = match sanitize(ctx, option, &value) {
            Ok(new) => new,
            Err(message) => {
                errors.push(SettingsError { setting: option.to_owned(), code: format!("invalid_{option}"), message, kind: "error".into() });
                continue;
            }
        };
        if option == "new_admin_email" {
            let current = site.options.string("admin_email", "");
            if new == current || new.is_empty() {
                // Nothing to confirm.
                if new.is_empty() && supplied.is_none() {
                    continue;
                }
            } else if site.options.string("new_admin_email", "") != new {
                admin_email_changed = Some(new.clone());
            }
        }
        if let Err(e) = writer.update_option(option, &new).await {
            eprintln!("options: {option}: {e}");
            errors.push(SettingsError { setting: option.to_owned(), code: "settings_failed".into(), message: ctx.tr("Settings save failed."), kind: "error".into() });
        }
    }

    // `update_option_new_admin_email()`: the new address becomes active once confirmed.
    if let Some(email) = admin_email_changed {
        let hash = {
            use sha2::{Digest, Sha256};
            let digest = Sha256::digest(format!("{email}{}{}", ctx.env.now, ctx.token).as_bytes());
            digest.iter().take(16).map(|b| format!("{b:02x}")).collect::<String>()
        };
        let stored = Value::Array(vec![(Key::Str(b"hash".to_vec()), Value::Str(hash.as_bytes().to_vec())), (Key::Str(b"newemail".to_vec()), Value::Str(email.as_bytes().to_vec()))]);
        let _ = writer.update_option("adminhash", &String::from_utf8_lossy(&rustpress_phpser::serialize(&stored))).await;
        let link = ctx.admin_url(&format!("options.php?adminhash={hash}"));
        errors.push(SettingsError {
            setting: "general".into(),
            code: "new_admin_email".into(),
            message: format!("{} <a href=\"{}\">{}</a>", esc_html("RustPress cannot send e-mail. Open this link to confirm the new administration email address:"), rustpress_core::formatting::esc_url(&link), esc_html(&link)),
            kind: "info".into(),
        });
    }

    if errors.is_empty() {
        errors.push(SettingsError { setting: "general".into(), code: "settings_updated".into(), message: ctx.tr("Settings saved."), kind: "success".into() });
    }
    let _ = writer.set_transient("settings_errors", &serialize_errors(&errors), 30, ctx.env.now).await;
    let goback = with_settings_updated(&referer(ctx).unwrap_or_else(|| ctx.request.request_uri.clone()));
    redirect(&goback)
}
