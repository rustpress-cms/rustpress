//! `wp-admin/authorize-application.php`: an application asks for an application password.

use rustpress_auth::app_passwords::{self, AppPassword};
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;

use super::notices::notice;
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};

/// `add_query_arg( array( ... ), $url )`: what the URL had is encoded again, the new values are taken as given.
fn add_args(url: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = rustpress_core::urls::remove_query_arg(url, "no-such-argument");
    for (key, value) in pairs {
        out.push(if out.contains('?') { '&' } else { '?' });
        out.push_str(key);
        out.push('=');
        out.push_str(value);
    }
    out
}

/// `wp_is_uuid()`.
fn is_uuid(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 5 && [8, 4, 4, 4, 12].iter().zip(&parts).all(|(n, p)| p.len() == *n && p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// `wp_is_authorize_application_redirect_url_valid()`: https, or a custom scheme; plain http only in a local environment.
fn redirect_valid(ctx: &Ctx<'_>, url: &str) -> Result<(), String> {
    let Some((scheme, _)) = url.split_once(':') else { return Err(ctx.tr("The URL provided is not valid.")) };
    let scheme = scheme.to_lowercase();
    if matches!(scheme.as_str(), "javascript" | "data" | "vbscript" | "file") {
        return Err(ctx.tr("The URL scheme is not allowed."));
    }
    let local = matches!(std::env::var("RUSTPRESS_ENVIRONMENT_TYPE").as_deref(), Ok("local" | "development"));
    if scheme == "http" && !local {
        return Err(ctx.tr("The URL must be served over a secure connection."));
    }
    Ok(())
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    let site = ctx.site();
    let mut error: Option<String> = None;
    let mut new_password = String::new();

    if request.post && request.post_field("action") == Some("authorize_application_password") {
        if !ctx.verify_nonce("authorize_application_password") {
            return die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
        }
        let field = |name: &str| request.post_field(name).unwrap_or_default().to_owned();
        let (success_url, reject_url, app_name, app_id) = (field("success_url"), field("reject_url"), field("app_name"), field("app_id"));
        let mut target = String::new();
        if request.post_field("reject").is_some() {
            target = if reject_url.is_empty() { ctx.admin_url("") } else { reject_url };
        } else if request.post_field("approve").is_some() {
            match ctx.env.writer {
                None => error = Some(tr("Application passwords could not be saved.")),
                Some(writer) => {
                    if app_name.trim().is_empty() {
                        error = Some(tr("An application name is required to create an application password."));
                    } else {
                        let plain = app_passwords::new_password();
                        let mut list = app_passwords::list(ctx.env.db, ctx.user.id).await;
                        list.push(AppPassword { uuid: app_passwords::new_uuid(), app_id, name: rustpress_core::formatting::strip_tags(&app_name).split_whitespace().collect::<Vec<_>>().join(" "), password: app_passwords::hash(&plain), created: ctx.env.now, last_used: None, last_ip: None });
                        match app_passwords::save(writer, ctx.user.id, &list).await {
                            Ok(()) => {
                                new_password = plain;
                                if !success_url.is_empty() {
                                    use rustpress_core::canonical::php_urlencode as enc;
                                    target = add_args(&success_url, &[("site_url", &enc(site.settings.siteurl.trim_end_matches('/'))), ("user_login", &enc(&ctx.user.login)), ("password", &enc(&new_password))]);
                                }
                            }
                            Err(e) => error = Some(e.to_string()),
                        }
                    }
                }
            }
        }
        if !target.is_empty() {
            return redirect(&target);
        }
    }

    let param = |name: &str| request.request(name).filter(|v| !v.is_empty()).map(str::to_owned);
    let app_name = param("app_name").unwrap_or_default();
    let app_id = param("app_id").unwrap_or_default();
    let success_url = param("success_url");
    let reject_url = match (param("reject_url"), &success_url) {
        (Some(url), _) => Some(url),
        (None, Some(success)) => Some(add_query_arg(success, "success", "false")),
        _ => None,
    };

    // `wp_is_authorize_application_password_request_valid()`
    let mut problems = Vec::new();
    for url in [&success_url, &reject_url].into_iter().flatten() {
        if let Err(message) = redirect_valid(ctx, url) {
            problems.push(message);
        }
    }
    if !app_id.is_empty() && !is_uuid(&app_id) {
        problems.push(tr("The provided Application ID must be a UUID."));
    }
    if !problems.is_empty() {
        return die(ctx.env, 400, &tr("Cannot Authorize Application"), &format!("{} {}", tr("The Authorize Application request is not allowed."), problems.join(" ")));
    }
    if !site.app_passwords_available() {
        return die(ctx.env, 501, &tr("Cannot Authorize Application"), &tr("Application passwords are not available."));
    }

    let mut h = String::from("<div class=\"wrap\">\n\t<h1>");
    h.push_str(&esc_html(&tr("Authorize Application")));
    h.push_str("</h1>\n\n\t");
    if let Some(message) = &error {
        h.push_str(&notice(message, "error", false, "", &[], true));
    }
    h.push_str("\n\n\t<div class=\"card auth-app-card\">\n\t\t<h2 class=\"title\">");
    h.push_str(&tr("An application would like to connect to your account."));
    h.push_str("</h2>\n\t\t");
    if app_name.is_empty() {
        h.push_str(&format!("			<p>{}</p>", tr("Would you like to give this application access to your account? You should only do this if you trust the application in question.")));
    } else {
        h.push_str(&format!("\t\t\t<p>\n\t\t\t\t{}\t\t\t</p>\n\t\t", sprintf(&tr("Would you like to give the application identifying itself as %s access to your account? You should only do this if you trust the application in question."), &[&format!("<strong>{}</strong>", esc_html(&app_name))])));
    }
    h.push_str("\n\n\t\t");
    if !new_password.is_empty() {
        let message = format!(
            "<p class=\"application-password-display\">\n\t\t\t\t<label for=\"new-application-password-value\">{}\n\t\t\t\t</label>\n\t\t\t\t<input id=\"new-application-password-value\" type=\"text\" class=\"code\" readonly=\"readonly\" value=\"{}\" />\n\t\t\t</p>\n\t\t\t<p>{}</p>",
            sprintf(&esc_html(&tr("Your new password for %s is:")), &[&format!("<strong>{}</strong>", esc_html(&app_name))]),
            esc_attr(&app_passwords::display(&new_password)),
            tr("Be sure to save this in a safe location. You will not be able to retrieve it.")
        );
        h.push_str(&notice(&message, "success", false, "", &["notice-alt", "below-h2"], false));
    } else {
        let success_text = match &success_url {
            Some(url) => {
                let shown = add_args(url, &[("site_url", site.settings.siteurl.trim_end_matches('/')), ("user_login", &ctx.user.login), ("password", "[------]")]);
                sprintf(&tr("You will be sent to %s"), &[&format!("<strong><code>{}</code></strong>", esc_html(&shown))])
            }
            None => tr("You will be given a password to manually enter into the application in question."),
        };
        let reject_text = match &reject_url {
            Some(url) => sprintf(&tr("You will be sent to %s"), &[&format!("<strong><code>{}</code></strong>", esc_html(url))]),
            None => tr("You will be returned to the WordPress Dashboard, and no changes will be made."),
        };
        h.push_str(&format!(
            "\t\t\t<form action=\"{}\" method=\"post\" class=\"form-wrap\">\n\t\t\t\t{}\t\t\t\t<input type=\"hidden\" name=\"action\" value=\"authorize_application_password\" />\n\t\t\t\t<input type=\"hidden\" name=\"app_id\" value=\"{}\" />\n\t\t\t\t<input type=\"hidden\" name=\"success_url\" value=\"{}\" />\n\t\t\t\t<input type=\"hidden\" name=\"reject_url\" value=\"{}\" />\n\n\t\t\t\t<div class=\"form-field\">\n\t\t\t\t\t<label for=\"app_name\">{}</label>\n\t\t\t\t\t<input type=\"text\" id=\"app_name\" name=\"app_name\" value=\"{}\" required />\n\t\t\t\t</div>\n\n\t\t\t\t<input type=\"submit\" name=\"approve\" id=\"approve\" class=\"button button-primary\" value=\"{}\" aria-describedby=\"description-approve\"  />\t\t\t\t<p class=\"description\" id=\"description-approve\">\n\t\t\t\t\t{success_text}\t\t\t\t</p>\n\n\t\t\t\t<input type=\"submit\" name=\"reject\" id=\"reject\" class=\"button\" value=\"{}\" aria-describedby=\"description-reject\"  />\t\t\t\t<p class=\"description\" id=\"description-reject\">\n\t\t\t\t\t{reject_text}\t\t\t\t</p>\n\t\t\t</form>\n\t\t",
            esc_url(&ctx.admin_url("authorize-application.php")),
            ctx.nonce_field("authorize_application_password"),
            esc_attr(&app_id),
            esc_url(success_url.as_deref().unwrap_or_default()),
            esc_url(reject_url.as_deref().unwrap_or_default()),
            tr("New Application Password Name"),
            esc_attr(&app_name),
            esc_attr(&tr("Yes, I approve of this connection")),
            esc_attr(&tr("No, I do not approve of this connection")),
        ));
    }
    h.push_str("\t</div>\n</div>\n");
    let settings = serde_json::json!({
        "site_url": site.settings.siteurl.trim_end_matches('/'),
        "user_login": ctx.user.login,
        "success": success_url,
        "reject": reject_url.clone().unwrap_or_else(|| ctx.admin_url("")),
    });
    let screen = Screen { file: "authorize-application.php".into(), parent_file: "authorize-application.php".into(), submenu_file: Some("authorize-application.php".into()), title: tr("Authorize Application"), help: Some(crate::chrome::Help::default()), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["auth-app".into()], localize: vec![("auth-app".into(), "authApp".into(), settings)], ..Default::default() }).await
}
