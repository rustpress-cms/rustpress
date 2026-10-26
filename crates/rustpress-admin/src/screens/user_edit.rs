//! `wp-admin/profile.php`, `user-edit.php` and `user-new.php`.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;
use rustpress_db::User;
use rustpress_phpser::{Key, Value};

use super::common::{checked, forbidden, read_only, selected};
use super::{AdminResponse, Ctx, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::{Content, UserData};
use crate::sanitize::{self, esc_textarea};

/// The admin color schemes: `(slug, name, icon colors, shades)`.
const SCHEMES: [(&str, &str, [&str; 3], &[&str]); 9] = [
    ("modern", "Default", ["#f3f1f1", "#fff", "#fff"], &["#1e1e1e", "#3858e9", "#7b90ff"]),
    ("fresh", "Fresh", ["#a7aaad", "#72aee6", "#fff"], &["#1d2327", "#2c3338", "#2271b1", "#72aee6"]),
    ("light", "Light", ["#999", "#ccc", "#ccc"], &["#e5e5e5", "#6a6a6a", "#c64606", "#007cba"]),
    ("blue", "Blue", ["#e5f8ff", "#fff", "#fff"], &["#183751", "#245278", "#437aa8", "#e1a948"]),
    ("coffee", "Coffee", ["#f3f2f1", "#fff", "#fff"], &["#382e27", "#5c4c40", "#916745", "#9ea476"]),
    ("ectoplasm", "Ectoplasm", ["#ece6f6", "#fff", "#fff"], &["#392751", "#4a3369", "#646c3e", "#d46f15"]),
    ("midnight", "Midnight", ["#f1f2f3", "#fff", "#fff"], &["#232a2e", "#333c42", "#69a8bb", "#cf4339"]),
    ("ocean", "Ocean", ["#f2fcff", "#fff", "#fff"], &["#2b3f44", "#39535a", "#567958", "#aa9d88"]),
    ("sunrise", "Sunrise", ["#f3f1f1", "#fff", "#fff"], &["#6f2724", "#8a312d", "#ad631e", "#ccaf0b"]),
];

const APP_PASSWORDS_SECTION: &str = include_str!("../data/app_passwords_section.tpl");
const APP_PASSWORDS_TEMPLATES: &str = include_str!("../data/app_passwords_templates.tpl");

/// The rows of `WP_Application_Passwords_List_Table`.
async fn app_password_rows(ctx: &Ctx<'_>, user_id: u64) -> String {
    let tr = |s: &str| ctx.tr(s);
    let list = rustpress_auth::app_passwords::list(ctx.env.db, user_id).await;
    if list.is_empty() {
        return format!("<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"5\">{}</td></tr>", tr("No items found."));
    }
    let site = ctx.site();
    let dash = "&mdash;";
    list.iter()
        .map(|item| {
            let name = esc_html(&item.name);
            let last_used = item.last_used.map_or_else(|| dash.to_owned(), |t| site.date(&tr("F j, Y"), t));
            let last_ip = item.last_ip.as_deref().map_or_else(|| dash.to_owned(), esc_html);
            format!(
                "<tr data-uuid=\"{uuid}\"><th class='name column-name has-row-actions column-primary' data-colname=\"{}\" scope=\"row\">{name}<button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button></th><td class='created column-created' data-colname=\"{}\">{}</td><td class='last_used column-last_used' data-colname=\"{}\">{last_used}</td><td class='last_ip column-last_ip' data-colname=\"{}\">{last_ip}</td><td class='revoke column-revoke' data-colname=\"{}\"><button type=\"button\" name=\"revoke-application-password-{uuid}\" id=\"revoke-application-password-{uuid}\" class=\"button delete\" aria-label=\"{}\">{}</button></td></tr>",
                tr("Name"),
                tr("Show more details"),
                tr("Created"),
                site.date(&tr("F j, Y"), item.created),
                tr("Last Used"),
                tr("Last IP"),
                tr("Revoke"),
                esc_attr(&rustpress_core::i18n::sprintf(&tr("Revoke \"%s\""), &[&item.name])),
                tr("Revoke"),
                uuid = item.uuid,
            )
        })
        .collect()
}

fn color_picker(ctx: &Ctx<'_>, current: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let mut out = format!("\t\t\t\t\t\t\t\t<fieldset id=\"color-picker\" class=\"scheme-list\">\n\t\t<legend class=\"screen-reader-text\"><span>{}</span></legend>\n\t\t<input type=\"hidden\" id=\"color-nonce\" name=\"color-nonce\" value=\"{}\" />", tr("Administration Color Scheme"), ctx.create_nonce("save-color-scheme"));
    for (slug, name, icons, shades) in SCHEMES {
        let css = if slug == "fresh" { String::new() } else { format!("{}/wp-admin/css/colors/{slug}/colors.min.css", ctx.site().settings.siteurl.trim_end_matches('/')) };
        let icon_json = format!("{{\"icons\":{{\"base\":\"{}\",\"focus\":\"{}\",\"current\":\"{}\"}}}}", icons[0], icons[1], icons[2]);
        let is_current = slug == current;
        out.push_str(&format!(
            "\t\t\t<div class=\"color-option{}\">\n\t\t\t\t<input name=\"admin_color\" id=\"admin_color_{slug}\" type=\"radio\" value=\"{slug}\" {} />\n\t\t\t\t<input type=\"hidden\" class=\"css_url\" value=\"{css}\" />\n\t\t\t\t<input type=\"hidden\" class=\"icon_colors\" value=\"{}\" />\n\t\t\t\t<label for=\"admin_color_{slug}\">{}</label>\n\t\t\t\t<div class=\"color-palette\">\n",
            if is_current { " selected" } else { " " },
            checked(is_current),
            esc_attr(&icon_json),
            tr(name)
        ));
        for shade in shades {
            out.push_str(&format!("\t\t\t\t\t\t\t\t\t<div class=\"color-palette-shade\" style=\"background-color: {shade}\">&nbsp;</div>\n\t"));
        }
        out.push_str("\t\t\t\t\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t");
    }
    out.push_str("\t</fieldset>\n");
    out
}

fn avatar(ctx: &Ctx<'_>, email: &str, size: u64) -> String {
    let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&ctx.site().options, email, s));
    format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-{size} photo' height='{size}' width='{size}' loading='lazy' decoding='async'/>", url(size), url(size * 2))
}

fn language_select(ctx: &Ctx<'_>, current: &str, with_site_default: bool, show_available: bool) -> String {
    let installed: Vec<&str> = ctx.env.languages.iter().map(|l| l.locale.as_str()).collect();
    let available: Vec<&(String, String, String)> = if show_available && (ctx.can("update_core") || ctx.can("install_plugins") || ctx.can("install_themes")) { ctx.env.available_translations.iter().filter(|(locale, _, _)| !installed.contains(&locale.as_str())).collect() } else { Vec::new() };
    let groups = !available.is_empty();
    let mut options = String::new();
    if groups {
        options.push_str(&format!("<optgroup label=\"{}\">
", esc_attr(&ctx.site().trx("Installed", "translations"))));
    }
    if with_site_default {
        options.push_str(&format!("<option value=\"site-default\" data-installed=\"1\"{}>{}</option>
", selected(current == "site-default"), ctx.tr("Site Default")));
    }
    options.push_str(&format!("<option value=\"\" lang=\"en\" data-installed=\"1\"{}>English (United States)</option>
", selected(current.is_empty())));
    for language in &ctx.env.languages {
        options.push_str(&format!("<option value=\"{}\" lang=\"{}\" data-installed=\"1\"{}>{}</option>
", esc_attr(&language.locale), esc_attr(&language.lang), selected(language.locale == current), esc_html(&language.native_name)));
    }
    if groups {
        options.push_str(&format!("</optgroup>
<optgroup label=\"{}\">
", esc_attr(&ctx.site().trx("Available", "translations"))));
        for (locale, native_name, lang) in available {
            options.push_str(&format!("<option value=\"{}\" lang=\"{}\"{}>{}</option>
", esc_attr(locale), esc_attr(lang), selected(locale == current), esc_html(native_name)));
        }
        options.push_str("</optgroup>
");
    }
    format!("<select name=\"locale\" id=\"locale\">{}</select>", options.trim_end_matches('\n'))
}

fn role_options(names: &[(String, String)], ctx: &Ctx<'_>, current: &str) -> String {
    let mut out = String::new();
    for (slug, name) in names.iter().rev() {
        let label = ctx.site().trx(name, "User role");
        if slug == current {
            out.push_str(&format!("\n\t<option selected='selected' value='{}'>{label}</option>", esc_attr(slug)));
        } else {
            out.push_str(&format!("\n\t<option value='{}'>{label}</option>", esc_attr(slug)));
        }
    }
    out
}

async fn meta(ctx: &Ctx<'_>, user_id: u64, key: &str) -> String {
    ctx.env.db.user_meta(user_id, key).await.ok().flatten().unwrap_or_default()
}

/// The profile form for `user`; `own` is true for `profile.php`.
async fn form(ctx: &Ctx<'_>, user: &User, own: bool, errors: &[String]) -> AdminResponse {
    form_on(ctx, user, own, errors, "profile.php").await
}

/// The form of a profile; `file` is the page it is shown on (`user-edit.php` shows the own profile, too).
async fn form_on(ctx: &Ctx<'_>, user: &User, own: bool, errors: &[String], file: &str) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    let id = user.id;
    let nickname = meta(ctx, id, "nickname").await;
    let first = meta(ctx, id, "first_name").await;
    let last = meta(ctx, id, "last_name").await;
    let description = meta(ctx, id, "description").await;
    let admin_color = {
        let c = meta(ctx, id, "admin_color").await;
        if c.is_empty() { "modern".to_owned() } else { c }
    };
    let locale = {
        let l = ctx.env.db.user_meta(id, "locale").await.ok().flatten().unwrap_or_default();
        if l.is_empty() { "site-default".to_owned() } else if l == "en_US" { String::new() } else { l }
    };
    let bar_front = meta(ctx, id, "show_admin_bar_front").await;
    let comment_shortcuts = meta(ctx, id, "comment_shortcuts").await;
    let syntax = meta(ctx, id, "syntax_highlighting").await;
    let infinite = meta(ctx, id, "media_infinite_scrolling").await;
    let heading = if own { tr("Profile") } else { sprintf(&tr("Edit User %s"), &[&esc_html(&user.display_name)]) };
    let action_url = ctx.admin_url(if own { "profile.php" } else { "user-edit.php" });
    // `$wp_http_referer`: where the list of users was, kept while the user is edited.
    let wp_http_referer = {
        let raw = request.request("wp_http_referer").filter(|r| !r.is_empty()).map(rustpress_core::formatting::esc_url_raw).unwrap_or_default();
        ["update", "delete_count", "user_id"].iter().fold(raw, |url, arg| rustpress_core::urls::remove_query_arg(&url, arg))
    };
    // The notices come before the wrap: the one of `new_user_email_admin_notice()`, then those of the page.
    let notice = |message: &str, kind: &str, dismissible: bool, id: &str, extra: &[&str], paragraph: bool| format!("\t\t{}\n", super::notices::notice(message, kind, dismissible, id, extra, paragraph).trim_end());
    let mut h = String::new();
    let updated = request.get("updated").is_some();
    if updated && own && let Some((_, new_email)) = pending_email(ctx, ctx.user.id).await {
        let message = sprintf(&tr("Your email address has not been updated yet. Please check your inbox at %s for a confirmation email."), &[&format!("<code>{}</code>", esc_html(&new_email))]);
        h.push_str(&notice(&message, "info", false, "", &[], true));
    }
    if updated {
        let mut message = format!("<p><strong>{}</strong></p>", if own { tr("Profile updated.") } else { tr("User updated.") });
        if !own && let Some(referer) = request.get("wp_http_referer").filter(|r| !r.is_empty() && !r.contains("user-new.php")) {
            message.push_str(&format!("<p><a href=\"{}\">{}</a></p>", esc_url(referer), tr("&larr; Go to Users")));
        }
        h.push_str(&notice(&message, "", true, "message", &["updated"], false));
    }
    if request.get("error") == Some("new-email") {
        h.push_str(&notice(&tr("Error while saving the new email address. Please try again."), "error", false, "", &[], true));
    } else if request.get("error").is_some() {
        h.push_str(&notice("", "error", false, "", &[], true));
    }
    if !errors.is_empty() {
        h.push_str(&notice(&errors.join("</p>\n<p>"), "", false, "", &["error"], true));
    }
    h.push_str("\t\t<div class=\"wrap\" id=\"profile-page\">\n");
    h.push_str(&format!("\t\t\t<h1 class=\"wp-heading-inline\">\n\t\t\t\t\t{heading}\t\t\t</h1>\n\n\t\t\t"));
    if !own && ctx.can("create_users") {
        h.push_str(&format!("\t\t\t\t\t\t\t\t\t<a href=\"user-new.php\" class=\"page-title-action\">{}</a>\n\t\t\t\t\t\t\t\n", tr("Add User")));
    }
    h.push_str(if !own && ctx.can("create_users") { "\t\t\t<hr class=\"wp-header-end\">\n\n" } else { "<hr class=\"wp-header-end\">\n\n" });
    h.push_str(&format!(
        "\t\t\t<form id=\"your-profile\" action=\"{}\" method=\"post\" novalidate=\"novalidate\"\n\t\t\t\t\t\t\t\t>\n\t\t\t\t{}\t\t\t\t{}\t\t\t\t<p>\n\t\t\t\t\t<input type=\"hidden\" name=\"from\" value=\"profile\" />\n\t\t\t\t\t<input type=\"hidden\" name=\"checkuser_id\" value=\"{}\" />\n\t\t\t\t</p>\n\n",
        esc_url(&action_url),
        ctx.nonce_field(&format!("update-user_{id}")),
        if wp_http_referer.is_empty() { String::new() } else { format!("\t\t\t\t\t<input type=\"hidden\" name=\"wp_http_referer\" value=\"{}\" />\n\t\t\t\t", esc_url(&wp_http_referer)) },
        ctx.user.id
    ));
    h.push_str(&format!("\t\t\t\t<h2>{}</h2>\n\n\t\t\t\t<table class=\"form-table\" role=\"presentation\">\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t\n", tr("Personal Options")));
    // `$show_syntax_highlighting_preference`: the edited user may edit the theme options, plugins or themes.
    let edited_can = {
        let loaded = match ctx.env.db.auth_user_by_id(id).await.ok().flatten() {
            Some(auth_user) => rustpress_auth::CurrentUser::load(ctx.env.db, ctx.env.roles, &auth_user).await.ok(),
            None => None,
        };
        loaded.is_some_and(|u| u.can("edit_theme_options") || u.can("edit_plugins") || u.can("edit_themes"))
    };
    if edited_can {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t<tr class=\"user-syntax-highlighting-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">{}</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t<label for=\"syntax_highlighting\"><input name=\"syntax_highlighting\" type=\"checkbox\" id=\"syntax_highlighting\" value=\"false\" {} />\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n",
            tr("Syntax Highlighting"),
            checked(syntax == "false").trim_start(),
            tr("Disable syntax highlighting when editing code")
        ));
    }
    h.push_str(&format!("\t\t\t\t\t\t\t\t\t\t<tr class=\"user-admin-color-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">{}</th>\n\t\t\t\t\t\t<td>\n{}\t\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t\t\n", tr("Administration Color Scheme"), color_picker(ctx, &admin_color)));
    h.push_str(&format!(
        "\t\t\t\t\t\t\t\t\t\t<tr class=\"user-comment-shortcuts-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">{}</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t<label for=\"comment_shortcuts\">\n\t\t\t\t\t\t\t\t<input type=\"checkbox\" name=\"comment_shortcuts\" id=\"comment_shortcuts\" value=\"true\" {} />\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t\t<a href=\"https://wordpress.org/documentation/article/keyboard-shortcuts-classic-editor/#keyboard-shortcuts-for-comments\">{}</a>\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t\t\n",
        tr("Keyboard Shortcuts"),
        checked(comment_shortcuts == "true").trim_start(),
        tr("Enable keyboard shortcuts for comment moderation."),
        tr("Documentation on Keyboard Shortcuts")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"show-admin-bar user-admin-bar-front-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">{}</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t<label for=\"admin_bar_front\">\n\t\t\t\t\t\t\t\t<input name=\"admin_bar_front\" type=\"checkbox\" id=\"admin_bar_front\" value=\"1\"{} />\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</label><br />\n\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\n",
        tr("Toolbar"),
        checked(bar_front != "false"),
        tr("Show Toolbar when viewing site")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t\t\t\t\t\t<tr class=\"user-infinite-scrolling-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">{}</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t<label for=\"infinite_scrolling\"><input name=\"infinite_scrolling\" type=\"checkbox\" id=\"infinite_scrolling\" value=\"false\" {} />\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t\t\n",
        tr("Infinite Scrolling"),
        checked(infinite == "false").trim_start(),
        tr("Disable infinite scrolling in the Media Library grid view")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-language-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<label for=\"locale\">{}<span class=\"dashicons dashicons-translation\" aria-hidden=\"true\"></span></label>\n\t\t\t\t\t\t</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t</table>\n\t\t\t\t\n",
        tr("Language"),
        language_select(ctx, &locale, true, true)
    ));

    // Name
    h.push_str(&format!("\t\t\t\t<h2>{}</h2>\n\n\t\t\t\t<table class=\"form-table\" role=\"presentation\">\n", tr("Name")));
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-user-login-wrap\">\n\t\t\t\t\t\t<th><label for=\"user_login\">{}</label></th>\n\t\t\t\t\t\t<td><input type=\"text\" name=\"user_login\" id=\"user_login\" value=\"{}\" readonly=\"readonly\" class=\"regular-text ltr\" /> <span class=\"description\">{}</span></td>\n\t\t\t\t\t</tr>\n\n",
        tr("Username"),
        esc_attr(&user.user_login),
        tr("Usernames cannot be changed.")
    ));
    if !own && ctx.can("promote_users") {
        let names = super::general::role_names(ctx).await;
        let roles = super::users::roles_of(ctx, id).await;
        let current = roles.first().map(|(s, _)| s.clone()).unwrap_or_default();
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-role-wrap\">\n\t\t\t\t\t\t\t<th><label for=\"role\">{}</label></th>\n\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t<select name=\"role\" id=\"role\">\n\t\t\t\t\t\t\t\t\t{}<option value=\"\">&mdash; {} &mdash;</option>\t\t\t\t\t\t\t</select>\n\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t</tr>\n\t\t\t\t\t\n",
            tr("Role"),
            role_options(&names, ctx, &current),
            tr("No role for this site")
        ));
    }
    h.push_str("\t\t\t\t\t\n");
    let ac = |name: &str| if own { format!(" autocomplete=\"{name}\"") } else { String::new() };
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-first-name-wrap\">\n\t\t\t\t\t\t<th><label for=\"first_name\">{}</label></th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<input type=\"text\" name=\"first_name\" id=\"first_name\" value=\"{}\"{} class=\"regular-text\" />\n\t\t\t\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\n",
        tr("First Name"),
        esc_attr(&first),
        ac("given-name")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-last-name-wrap\">\n\t\t\t\t\t\t<th><label for=\"last_name\">{}</label></th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<input type=\"text\" name=\"last_name\" id=\"last_name\" value=\"{}\"{} class=\"regular-text\" />\n\t\t\t\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\n",
        tr("Last Name"),
        esc_attr(&last),
        ac("family-name")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-nickname-wrap\">\n\t\t\t\t\t\t<th><label for=\"nickname\">{} <span class=\"description\">{}</span></label></th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t\t\t\t<input type=\"text\" name=\"nickname\" id=\"nickname\" value=\"{}\"{} class=\"regular-text\" />\n\t\t\t\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\n",
        tr("Nickname"),
        tr("(required)"),
        esc_attr(&nickname),
        ac("nickname")
    ));
    // The display name choices.
    let mut choices: Vec<String> = vec![user.user_login.clone()];
    for candidate in [nickname.clone(), first.clone(), last.clone(), format!("{first} {last}").trim().to_owned(), format!("{last} {first}").trim().to_owned()] {
        if !candidate.is_empty() && !choices.contains(&candidate) {
            choices.push(candidate);
        }
    }
    if !choices.contains(&user.display_name) && !user.display_name.is_empty() {
        choices.insert(0, user.display_name.clone());
    }
    let mut choice_html = String::new();
    for choice in &choices {
        choice_html.push_str(&format!("\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<option {}>{}</option>\n", if *choice == user.display_name { " selected='selected'" } else { "" }, esc_html(choice)));
    }
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-display-name-wrap\">\n\t\t\t\t\t\t<th>\n\t\t\t\t\t\t\t<label for=\"display_name\">{}</label>\n\t\t\t\t\t\t</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t<select name=\"display_name\" id=\"display_name\">\n\t\t\t\t\t\t\t\t{choice_html}\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t</select>\n\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t</table>\n\n",
        tr("Display name publicly as")
    ));

    // Contact info
    h.push_str(&format!("\t\t\t\t<h2>{}</h2>\n\n\t\t\t\t<table class=\"form-table\" role=\"presentation\">\n", tr("Contact Info")));
    let email_note = if own {
        format!(
            "\n\t\t\t\t\t\t\t\t<p class=\"description\" id=\"email-description\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</p>\n",
            tr("If you change this, an email will be sent at your new address to confirm it. <strong>The new address will not become active until confirmed.</strong>")
        )
    } else {
        String::new()
    };
    // `_new_email`: a change of the own address that waits for its link.
    let pending = match pending_email(ctx, ctx.user.id).await {
        Some((_, new_email)) if own && new_email != ctx.user.email => {
            let cancel = ctx.admin_url(&format!("profile.php?dismiss={}_new_email&amp;_wpnonce={}", ctx.user.id, ctx.create_nonce(&format!("dismiss-{}_new_email", ctx.user.id))));
            format!(
                "<div class=\"notice updated inline\"><p>{} <a href=\"{}\">{}</a></p></div>",
                sprintf(&tr("There is a pending change of your email to %s."), &[&format!("<code>{}</code>", esc_html(&new_email))]),
                cancel,
                tr("Cancel")
            )
        }
        _ => String::new(),
    };
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-email-wrap\">\n\t\t\t\t\t\t<th><label for=\"email\">{} <span class=\"description\">{}</span></label></th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<input type=\"email\" name=\"email\" id=\"email\"{} value=\"{}\"{} class=\"regular-text ltr\" />\n{email_note}\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t{pending}\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\n",
        tr("Email"),
        tr("(required)"),
        if own { " aria-describedby=\"email-description\"" } else { "" },
        esc_attr(&user.user_email),
        ac("email")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-url-wrap\">\n\t\t\t\t\t\t<th><label for=\"url\">{}</label></th>\n\t\t\t\t\t\t<td><input type=\"url\" name=\"url\" id=\"url\" value=\"{}\" class=\"regular-text code\" /></td>\n\t\t\t\t\t</tr>\n\n\t\t\t\t\t\t\t\t\t</table>\n\n",
        tr("Website"),
        esc_attr(&user.user_url)
    ));

    // About
    h.push_str(&format!("\t\t\t\t<h2>{}</h2>\n\n\t\t\t\t<table class=\"form-table\" role=\"presentation\">\n", if own { tr("About Yourself") } else { tr("About the user") }));
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-description-wrap\">\n\t\t\t\t\t\t<th><label for=\"description\">{}</label></th>\n\t\t\t\t\t\t<td><textarea name=\"description\" id=\"description\" rows=\"5\" cols=\"30\">{}</textarea>\n\t\t\t\t\t\t<p class=\"description\">{}</p></td>\n\t\t\t\t\t</tr>\n\n",
        tr("Biographical Info"),
        esc_textarea(&description),
        tr("Share a little biographical information to fill out your profile. This may be shown publicly.")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-profile-picture\">\n\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t</tr>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t</table>\n\n",
        tr("Profile Picture"),
        avatar(ctx, &user.user_email, 96),
        if own { tr("<a href=\"https://gravatar.com/\">You can change your profile picture on Gravatar</a>.") } else { String::new() }
    ));

    // Account management
    let password_hint = sanitize::random_password(24, true);
    h.push_str(&format!("\t\t\t\t\t\t<h2>{}</h2>\n\n\t\t\t\t\t\t<table class=\"form-table\" role=\"presentation\">\n", tr("Account Management")));
    h.push_str(&format!(
        "\t\t\t\t\t\t\t<tr id=\"password\" class=\"user-pass1-wrap\">\n\t\t\t\t\t\t\t\t<th><label for=\"pass1\">{}</label></th>\n\t\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t<input type=\"hidden\" value=\" \" /><!-- #24364 workaround -->\n\t\t\t\t\t\t\t\t\t<button type=\"button\" class=\"button wp-generate-pw hide-if-no-js\" aria-expanded=\"false\">{}</button>\n\t\t\t\t\t\t\t\t\t<div class=\"wp-pwd hide-if-js\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"password-input-wrapper\">\n\t\t\t\t\t\t\t\t\t\t\t<input type=\"password\" name=\"pass1\" id=\"pass1\" class=\"regular-text ltr\" value=\"\" autocomplete=\"new-password\" spellcheck=\"false\" data-pw=\"{}\" aria-describedby=\"pass-strength-result\" />\n\t\t\t\t\t\t\t\t\t\t\t<div style=\"display:none\" id=\"pass-strength-result\" aria-live=\"polite\"></div>\n\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t\t<button type=\"button\" class=\"button wp-hide-pw user-new-password-toggle hide-if-no-js\" data-toggle=\"0\" aria-label=\"{}\">\n\t\t\t\t\t\t\t\t\t\t\t<span class=\"dashicons dashicons-hidden\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t\t\t\t\t\t<span class=\"text\">{}</span>\n\t\t\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t\t\t\t<button type=\"button\" class=\"button wp-cancel-pw hide-if-no-js\" data-toggle=\"0\" aria-label=\"{}\">\n\t\t\t\t\t\t\t\t\t\t\t<span class=\"dashicons dashicons-no\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t\t\t\t\t\t<span class=\"text\">{}</span>\n\t\t\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t</tr>\n",
        tr("New Password"),
        tr("Set New Password"),
        esc_attr(&password_hint),
        tr("Hide password"),
        tr("Hide"),
        tr("Cancel password change"),
        tr("Cancel")
    ));
    h.push_str(&format!(
        "\t\t\t\t\t\t\t<tr class=\"user-pass2-wrap hide-if-js\">\n\t\t\t\t\t\t\t\t<th scope=\"row\"><label for=\"pass2\">{}</label></th>\n\t\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t<input type=\"password\" name=\"pass2\" id=\"pass2\" class=\"regular-text\" value=\"\" autocomplete=\"new-password\" spellcheck=\"false\" aria-describedby=\"pass2-desc\" />\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<p class=\"description\" id=\"pass2-desc\">{}</p>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t</tr>\n\t\t\t\t\t\t\t<tr class=\"pw-weak\">\n\t\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t<label>\n\t\t\t\t\t\t\t\t\t\t<input type=\"checkbox\" name=\"pw_weak\" class=\"pw-checkbox\" />\n\t\t\t\t\t\t\t\t\t\t<span id=\"pw-weak-text-label\">{}</span>\n\t\t\t\t\t\t\t\t\t</label>\n\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t</tr>\n\t\t\t\t\t\t\t\n",
        tr("Repeat New Password"),
        if own { tr("Type your new password again.") } else { tr("Type the new password again.") },
        tr("Confirm Password"),
        tr("Confirm use of weak password")
    ));
    // The link to reset the password, for somebody else (`wp_is_password_reset_allowed_for_user()`).
    if !own && ctx.can("edit_users") {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-generate-reset-link-wrap hide-if-no-js\">\n\t\t\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t<div class=\"generate-reset-link\">\n\t\t\t\t\t\t\t\t\t\t\t<button type=\"button\" class=\"button button-secondary\" id=\"generate-reset-link\">\n\t\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t\t</button>\n\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t\t</tr>\n\t\t\t\t\t\t\t\n",
            tr("Password Reset"),
            tr("Send Reset Link"),
            sprintf(&tr("Send %s a link to reset their password. This will not change their password, nor will it force a change."), &[&user.display_name])
        ));
    }
    // The sessions: where the person is signed in (`WP_Session_Tokens::get_all()`).
    let sessions = session_count(ctx, id).await;
    if own && sessions == 1 {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-sessions-wrap hide-if-no-js\">\n\t\t\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t\t\t<td aria-live=\"assertive\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"destroy-sessions\"><button type=\"button\" disabled class=\"button\">{}</button></div>\n\t\t\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t\t</tr>\n",
            tr("Sessions"),
            tr("Log Out Everywhere Else"),
            tr("You are only logged in at this location.")
        ));
    } else if own && sessions > 1 {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-sessions-wrap hide-if-no-js\">\n\t\t\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t\t\t<td aria-live=\"assertive\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"destroy-sessions\"><button type=\"button\" class=\"button\" id=\"destroy-sessions\">{}</button></div>\n\t\t\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t\t</tr>\n",
            tr("Sessions"),
            tr("Log Out Everywhere Else"),
            tr("Did you lose your phone or leave your account logged in at a public computer? You can log out everywhere else, and stay logged in here.")
        ));
    } else if !own && sessions > 0 {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-sessions-wrap hide-if-no-js\">\n\t\t\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t<p><button type=\"button\" class=\"button\" id=\"destroy-sessions\">{}</button></p>\n\t\t\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t\t</tr>\n",
            tr("Sessions"),
            tr("Log Out Everywhere"),
            sprintf(&tr("Log %s out of all locations."), &[&user.display_name])
        ));
    }
    h.push_str("\t\t\t\t\t\t\t\t\t\t\t\t\t</table>\n\n");
    // `wp_is_application_passwords_available_for_user()`: needs HTTPS (or a local environment), which RustPress cannot assume.
    let app_passwords = own || ctx.can("edit_users");
    let app_available = app_passwords && ctx.site().app_passwords_available();
    if app_available {
        let rows = app_password_rows(ctx, id).await;
        h.push_str(&super::pages::translate_html(ctx, APP_PASSWORDS_SECTION).replace("{{ROWS}}", &rows));
    } else if app_passwords {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t<div class=\"application-passwords hide-if-no-js\" id=\"application-passwords-section\">\n\t\t\t\t\t\t\t<h2>{}</h2>\n\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t\t\t<p>\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\n",
            tr("Application Passwords"),
            tr("Application passwords allow authentication via non-interactive systems, such as XML-RPC or the REST API, without providing your actual password. Application passwords can be easily revoked. They cannot be used for traditional logins to your website."),
            tr("The application password feature requires HTTPS, which is not enabled on this site."),
            rustpress_core::i18n::sprintf(&tr("If this is a development website, you can <a href=\"%s\">set the environment type accordingly</a> to enable application passwords."), &["https://developer.wordpress.org/apis/wp-config-php/#wp-environment-type"])
        ));
    }
    h.push_str(&format!("<input type=\"hidden\" name=\"action\" value=\"update\" />\n\t\t\t\t<input type=\"hidden\" name=\"user_id\" id=\"user_id\" value=\"{id}\" />\n\n\t\t\t\t<p class=\"submit\"><input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button button-primary\" value=\"{}\"  /></p>\n\t\t\t</form>\n\t\t</div>\n\t\t<script>\n\tif (window.location.hash == '#password') {{\n\t\tdocument.getElementById('pass1').focus();\n\t}}\n</script>\n\n<script>\n\tjQuery( function( $ ) {{\n\t\tvar languageSelect = $( '#locale' );\n\t\t$( 'form' ).on( 'submit', function() {{\n\t\t\t/*\n\t\t\t * Don't show a spinner for English and installed languages,\n\t\t\t * as there is nothing to download.\n\t\t\t */\n\t\t\tif ( ! languageSelect.find( 'option:selected' ).data( 'installed' ) ) {{\n\t\t\t\t$( '#submit', this ).after( '<span class=\"spinner language-install-spinner is-active\" />' );\n\t\t\t}}\n\t\t}});\n\t}} );\n</script>\n\n", if own { tr("Update Profile") } else { tr("Update User") }));
    if app_available {
        h.push_str(&super::pages::translate_html(ctx, APP_PASSWORDS_TEMPLATES).replace("{{SITE}}", ctx.site().settings.siteurl.trim_end_matches('/')));
    }
    let (parent_file, submenu_file) = if ctx.can("list_users") { ("users.php", if own { "profile.php" } else { "users.php" }) } else { ("profile.php", "profile.php") };
    let view_item = (!own || file == "user-edit.php").then(|| {
        let url = if ctx.site().settings.permalink_structure.is_empty() { ctx.site().home_url(&format!("?author={id}")) } else { ctx.site().home_url(&format!("author/{}/", user.user_nicename)) };
        (tr("View User"), url)
    });
    let screen = Screen {
        file: file.to_owned(),
        parent_file: parent_file.into(),
        submenu_file: Some(submenu_file.into()),
        title: if own { tr("Profile") } else { tr("Edit User") },
        help: Some(help(ctx)),
        view_item,
        ..Default::default()
    };
    let mut scripts = vec!["user-profile".to_owned()];
    if app_available {
        scripts.push("application-passwords".into());
    }
    render(ctx, &screen, &h, &Assets { scripts, ..Default::default() }).await
}

/// The help of `profile.php` and `user-edit.php`.
fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let overview: String = [
        "Your profile contains information about you (your &#8220;account&#8221;) as well as some personal options related to using WordPress.",
        "You can change your password, turn on keyboard shortcuts, change the color scheme of your WordPress administration screens, and turn off the WYSIWYG (Visual) editor, among other things. You can hide the Toolbar (formerly called the Admin Bar) from the front end of your site, however it cannot be disabled on the admin screens.",
        "You can select the language you wish to use while using the WordPress administration screen without affecting the language site visitors see.",
        "Your username cannot be changed, but you can use other fields to enter your real name or a nickname, and change which name to display on your posts.",
        "You can log out of other devices, such as your phone or a public computer, by clicking the Log Out Everywhere Else button.",
        "Required fields are indicated; the rest are optional. Profile information will only be displayed if your theme is set up to do so.",
        "Remember to click the Update Profile button when you are finished.",
    ]
    .iter()
    .map(|p| format!("<p>{}</p>", tr(p)))
    .collect();
    Help {
        tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content: overview }],
        sidebar: format!(
            "<p><strong>{}</strong></p><p>{}</p><p>{}</p>",
            tr("For more information:"),
            tr("<a href=\"https://wordpress.org/documentation/article/users-your-profile-screen/\">Documentation on User Profiles</a>"),
            tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
        ),
    }
}

/// The help of `user-new.php`.
fn new_help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let p = |s: &str| format!("<p>{}</p>", tr(s));
    Help {
        tabs: vec![
            HelpTab {
                id: "overview".into(),
                title: tr("Overview"),
                content: [
                    "To add a new user to your site, fill in the form on this screen and click the Add User button at the bottom.",
                    "New users are automatically assigned a password, which they can change after logging in. You can view or edit the assigned password by clicking the Show Password button. The username cannot be changed once the user has been added.",
                    "By default, new users will receive an email letting them know they&#8217;ve been added as a user for your site. This email will also contain a password reset link. Uncheck the box if you do not want to send the new user a welcome email.",
                    "Remember to click the Add User button at the bottom of this screen when you are finished.",
                ]
                .iter()
                .map(|s| p(s))
                .collect(),
            },
            HelpTab {
                id: "user-roles".into(),
                title: tr("User Roles"),
                content: format!(
                    "{}<ul>{}</ul>",
                    p("Here is a basic overview of the different user roles and the permissions associated with each one:"),
                    [
                        "Subscribers can read comments/comment/receive newsletters, etc. but cannot create regular site content.",
                        "Contributors can write and manage their posts but not publish posts or upload media files.",
                        "Authors can publish and manage their own posts, and are able to upload files.",
                        "Editors can publish posts, manage posts as well as manage other people&#8217;s posts, etc.",
                        "Administrators have access to all the administration features.",
                    ]
                    .iter()
                    .map(|s| format!("<li>{}</li>", tr(s)))
                    .collect::<String>()
                ),
            },
        ],
        sidebar: format!(
            "<p><strong>{}</strong></p><p>{}</p><p>{}</p>",
            tr("For more information:"),
            tr("<a href=\"https://wordpress.org/documentation/article/users-add-new-screen/\">Documentation on Adding New Users</a>"),
            tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
        ),
    }
}

/// `profile.php`.
pub async fn profile(ctx: &Ctx<'_>) -> AdminResponse {
    let user = ctx.env.db.user(ctx.user.id).await.ok().flatten();
    let Some(user) = user else { return forbidden(ctx, "Invalid user ID.") };
    if let Some(response) = new_email_links(ctx, &user).await {
        return response;
    }
    if ctx.request.post && ctx.request.post_field("action") == Some("update") {
        return save(ctx, ctx.user.id, true).await;
    }
    form(ctx, &user, true, &[]).await
}

/// `user-edit.php`.
pub async fn user_edit(ctx: &Ctx<'_>) -> AdminResponse {
    let id: u64 = ctx.request.request("user_id").and_then(|v| v.parse().ok()).unwrap_or(0);
    if id == 0 {
        return super::common::die_plain(ctx, "Invalid user ID.");
    }
    if id != ctx.user.id && !ctx.can("edit_users") {
        return super::common::die_plain(ctx, "Sorry, you are not allowed to edit this user.");
    }
    let Some(user) = ctx.env.db.user(id).await.ok().flatten() else {
        return super::common::die_plain(ctx, "Invalid user ID.");
    };
    let own = id == ctx.user.id;
    if ctx.request.post && ctx.request.post_field("action") == Some("update") {
        return save(ctx, id, own).await;
    }
    // The own profile on `user-edit.php` is the profile page with the toolbar entry "View User".
    form_on(ctx, &user, own, &[], "user-edit.php").await
}

/// The number of sessions of a user that have not ended (`WP_Session_Tokens::get_all()`).
async fn session_count(ctx: &Ctx<'_>, user_id: u64) -> usize {
    let Some(raw) = ctx.env.db.user_meta(user_id, "session_tokens").await.ok().flatten() else { return 0 };
    let Ok(rustpress_phpser::Value::Array(tokens)) = rustpress_phpser::unserialize(raw.as_bytes()) else { return 0 };
    tokens.iter().filter(|(_, session)| session.get("expiration").and_then(Value::as_i64).is_some_and(|expiration| expiration >= ctx.env.now)).count()
}

/// The pending change of the e-mail address: `_new_email` holds the hash and the address.
async fn pending_email(ctx: &Ctx<'_>, user_id: u64) -> Option<(String, String)> {
    let raw = ctx.env.db.user_meta(user_id, "_new_email").await.ok().flatten()?;
    let value = rustpress_phpser::unserialize(raw.as_bytes()).ok()?;
    let text = |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    Some((text("hash")?, text("newemail")?))
}

/// `profile.php?newuseremail=<hash>` confirms the change of the address, `?dismiss=<id>_new_email` cancels it.
async fn new_email_links(ctx: &Ctx<'_>, user: &User) -> Option<AdminResponse> {
    let request = ctx.request;
    if let Some(hash) = request.get("newuseremail") {
        let writer = ctx.env.writer?;
        return Some(match pending_email(ctx, user.id).await.filter(|(h, _)| h == hash) {
            Some((_, email)) => {
                let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
                content.insert_user(&UserData { id: Some(user.id), user_email: Some(rustpress_core::formatting::esc_html(email.trim())), ..Default::default() }).await.or_log();
                writer.delete_user_meta(user.id, "_new_email").await.or_log();
                redirect(&add_query_arg(&ctx.admin_url("profile.php"), "updated", "true"))
            }
            None => redirect(&add_query_arg(&ctx.admin_url("profile.php"), "error", "new-email")),
        });
    }
    if request.get("dismiss") == Some(&format!("{}_new_email", user.id)) {
        if !ctx.verify_nonce(&format!("dismiss-{}_new_email", user.id)) {
            return Some(super::die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired.")));
        }
        let writer = ctx.env.writer?;
        writer.delete_user_meta(user.id, "_new_email").await.or_log();
        return Some(redirect(&add_query_arg(&ctx.admin_url("profile.php"), "updated", "true")));
    }
    None
}

/// `send_confirmation_on_profile_email()`: the new address gets a link; the address stays as it is until it is used.
async fn send_confirmation_on_profile_email(ctx: &Ctx<'_>, user: &User, email: &str) -> bool {
    let Some(writer) = ctx.env.writer else { return false };
    if ctx.env.mailer.is_none() {
        return false;
    }
    use rand::Rng;
    let hash = format!("{:x}", md5::compute(format!("{email}{}{}", ctx.env.now, rand::rng().random::<u32>())));
    let stored = Value::Array(vec![(Key::Str(b"hash".to_vec()), Value::Str(hash.as_bytes().to_vec())), (Key::Str(b"newemail".to_vec()), Value::Str(email.as_bytes().to_vec()))]);
    writer.update_user_meta(user.id, "_new_email", &String::from_utf8_lossy(&rustpress_phpser::serialize(&stored))).await.or_log();
    let sitename = ctx.env.mail_ctx().sitename();
    let content = ctx
        .tr(crate::mail_texts::NEW_USER_EMAIL)
        .replace("###USERNAME###", &user.user_login)
        .replace("###ADMIN_URL###", &rustpress_core::formatting::esc_url(&ctx.admin_url(&format!("profile.php?newuseremail={hash}"))))
        .replace("###EMAIL###", email)
        .replace("###SITENAME###", &sitename)
        .replace("###SITEURL###", &ctx.site().settings.home);
    let subject = sprintf(&ctx.tr("[%s] Email Change Request"), &[&sitename]);
    // A mail that cannot be sent is no error of the request (as `wp_mail()` in WordPress).
    let _ = ctx.env.send_mail(email, &subject, &content).await;
    true
}

/// `edit_user()` for the profile forms.
async fn save(ctx: &Ctx<'_>, id: u64, own: bool) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    if !ctx.verify_nonce(&format!("update-user_{id}")) {
        return super::die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
    }
    let Some(writer) = ctx.env.writer else { return read_only(ctx) };
    let Some(user) = ctx.env.db.user(id).await.ok().flatten() else { return forbidden(ctx, "Invalid user ID.") };
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let field = |name: &str| request.post_field(name).map(str::to_owned);
    let mut errors: Vec<String> = Vec::new();
    let nickname = field("nickname").unwrap_or_default();
    if nickname.trim().is_empty() {
        errors.push(tr("<strong>Error:</strong> Please enter a nickname."));
    }
    let email = field("email").unwrap_or_default();
    let email = email.trim().to_owned();
    if email.is_empty() {
        errors.push(tr("<strong>Error:</strong> Please enter an email address."));
    } else if !super::options::is_email(&super::options::sanitize_email(&email)) {
        errors.push(tr("<strong>Error:</strong> The email address isn&#8217;t correct."));
    } else if let Some(other) = ctx.env.db.user_by_email(&super::options::sanitize_email(&email)).await.ok().flatten()
        && other.id != id
    {
        errors.push(tr("<strong>Error:</strong> This email is already registered. Please choose another one."));
    }
    let pass1 = field("pass1").unwrap_or_default();
    let pass2 = field("pass2").unwrap_or_default();
    if pass1.contains('\\') {
        errors.push(tr("<strong>Error:</strong> Passwords may not contain the character &#8220;\\&#8221;."));
    }
    if !pass1.is_empty() && pass1 != pass2 {
        errors.push(tr("<strong>Error:</strong> Passwords do not match. Please enter the same password in both password fields."));
    }
    if !errors.is_empty() {
        return form(ctx, &user, own, &errors).await;
    }
    // Your own address changes only when the link sent to the new one is used.
    let old_email = user.user_email.clone();
    let old_login = user.user_login.clone();
    let email_changed = email != old_email;
    let email = if own && email_changed && send_confirmation_on_profile_email(ctx, &user, &email).await { old_email.clone() } else { email };
    let mut data = UserData {
        id: Some(id),
        user_email: Some(email.clone()),
        user_url: field("url"),
        display_name: field("display_name"),
        nickname: Some(nickname),
        first_name: field("first_name"),
        last_name: field("last_name"),
        description: field("description"),
        rich_editing: Some(if field("rich_editing").as_deref() == Some("false") { "false".into() } else { "true".into() }),
        comment_shortcuts: Some(if field("comment_shortcuts").as_deref() == Some("true") { "true".into() } else { "false".into() }),
        show_admin_bar_front: Some(if field("admin_bar_front").is_some() { "true".into() } else { "false".into() }),
        ..Default::default()
    };
    if own {
        data.syntax_highlighting = Some(if field("syntax_highlighting").as_deref() == Some("false") { "false".into() } else { "true".into() });
    }
    if !pass1.is_empty() {
        data.user_pass = Some(pass1);
    }
    if let Some(scheme) = field("admin_color").filter(|c| SCHEMES.iter().any(|(s, ..)| s == c)) {
        data.admin_color = Some(scheme);
    }
    if let Some(locale) = field("locale") {
        data.locale = Some(if locale == "site-default" { String::new() } else { locale });
    }
    if !own && ctx.can("promote_users") && let Some(role) = field("role") {
        // `get_editable_roles()`: a role that does not exist is refused.
        if !role.is_empty() && !ctx.env.roles.contains(&role) {
            return super::die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr("Sorry, you are not allowed to give users that role.")));
        }
        data.role = Some(role);
    }
    // `locale` "site-default" is stored as an empty string like WordPress.
    match content.insert_user(&data).await {
        Ok(_) => {}
        Err(e) => return form(ctx, &user, own, &[format!("<strong>Error:</strong> {e}")]).await,
    }
    if data.user_pass.is_some() && own {
        // The session stays; the other ones end like in `wp_update_user()`.
        rustpress_auth::session::destroy_other_sessions(ctx.env.db, writer, id, &ctx.token, ctx.env.now).await.or_log();
    }
    // `wp_update_user()` tells the person about a new password and a new address.
    if ctx.env.mailer.is_some() {
        let mail = ctx.env.mail_ctx();
        let sitename = mail.sitename();
        let admin_email = ctx.site().options.string("admin_email", "");
        let home = ctx.site().settings.home.clone();
        if data.user_pass.is_some() {
            let message = ctx.tr(crate::mail_texts::PASSWORD_CHANGED).replace("###USERNAME###", &old_login).replace("###ADMIN_EMAIL###", &admin_email).replace("###EMAIL###", &old_email).replace("###SITENAME###", &sitename).replace("###SITEURL###", &home);
            // A mail that cannot be sent is no error of the request (as `wp_mail()` in WordPress).
            let _ = mail.send(&old_email, &sprintf(&ctx.tr("[%s] Password Changed"), &[&sitename]), &message).await;
        }
        if email != old_email {
            let message = ctx.tr(crate::mail_texts::EMAIL_CHANGED).replace("###USERNAME###", &old_login).replace("###ADMIN_EMAIL###", &admin_email).replace("###EMAIL###", &old_email).replace("###NEW_EMAIL###", &email).replace("###SITENAME###", &sitename).replace("###SITEURL###", &home);
            // A mail that cannot be sent is no error of the request (as `wp_mail()` in WordPress).
            let _ = mail.send(&old_email, &sprintf(&ctx.tr("[%s] Email Changed"), &[&sitename]), &message).await;
        }
    }
    let mut url = if own { ctx.admin_url("profile.php") } else { ctx.admin_url(&format!("user-edit.php?user_id={id}")) };
    url = add_query_arg(&url, "updated", "1");
    // The list of users the editing started from is not forgotten.
    if let Some(referer) = request.post_field("wp_http_referer").filter(|r| !r.is_empty()) {
        let referer = ["update", "delete_count", "user_id"].iter().fold(rustpress_core::formatting::esc_url_raw(referer), |url, arg| rustpress_core::urls::remove_query_arg(&url, arg));
        if !referer.is_empty() {
            url = add_query_arg(&url, "wp_http_referer", &rustpress_core::canonical::php_urlencode(&referer));
        }
    }
    redirect(&url)
}

/// `user-new.php`.
pub async fn user_new(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("create_users") {
        return super::common::forbidden_heading(ctx, "Sorry, you are not allowed to create users.");
    }
    let request = ctx.request;
    let mut errors: Vec<String> = Vec::new();
    let submitted = |name: &str| request.post_field(name).unwrap_or_default().to_owned();
    if request.post && request.post_field("action") == Some("createuser") {
        if request.post_field("_wpnonce_create-user").is_none_or(|n| ctx.env.auth.verify_nonce(n, "create-user", ctx.user.id, &ctx.token, ctx.env.now).is_none()) {
            return super::die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
        }
        let Some(writer) = ctx.env.writer else { return read_only(ctx) };
        let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
        let login = submitted("user_login");
        let email = submitted("email");
        let pass1 = submitted("pass1");
        let pass2 = submitted("pass2");
        if sanitize::user_login(&login, true).is_empty() {
            errors.push(tr("<strong>Error:</strong> Please enter a username."));
        }
        if email.trim().is_empty() {
            errors.push(tr("<strong>Error:</strong> Please enter an email address."));
        } else if !super::options::is_email(&super::options::sanitize_email(&email)) {
            errors.push(tr("<strong>Error:</strong> The email address isn&#8217;t correct."));
        }
        if pass1.is_empty() {
            errors.push(tr("<strong>Error:</strong> Please enter a password."));
        } else if pass1 != pass2 {
            errors.push(tr("<strong>Error:</strong> Passwords do not match. Please enter the same password in both password fields."));
        }
        if ctx.can("promote_users") && !submitted("role").is_empty() && !ctx.env.roles.contains(&submitted("role")) {
            return super::die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr("Sorry, you are not allowed to give users that role.")));
        }
        if errors.is_empty() {
            let role = submitted("role");
            let data = UserData {
                user_login: Some(login),
                user_email: Some(email),
                user_pass: Some(pass1),
                first_name: Some(submitted("first_name")),
                last_name: Some(submitted("last_name")),
                user_url: Some(submitted("url")),
                role: Some(role),
                locale: Some(match submitted("locale").as_str() {
                    "site-default" => String::new(),
                    other => other.to_owned(),
                }),
                ..Default::default()
            };
            match content.insert_user(&data).await {
                Ok(new_id) => {
                    return redirect(&ctx.admin_url(&format!("users.php?id={new_id}&update=add")));
                }
                Err(e) => errors.push(format!("<strong>Error:</strong> {e}")),
            }
        }
    }
    let names = super::general::role_names(ctx).await;
    let default_role = {
        let r = submitted("role");
        if r.is_empty() { ctx.site().options.string("default_role", "subscriber") } else { r }
    };
    let mut h = format!("\t\t<div class=\"wrap\">\n<h1 id=\"add-new-user\">\n{}</h1>\n\n", tr("Add User"));
    if !errors.is_empty() {
        h.push_str(&format!("<div class=\"error\"><ul>\n{}</ul></div>\n", errors.iter().map(|e| format!("\t<li>{e}</li>\n")).collect::<String>()));
    }
    h.push_str("\n<div id=\"ajax-response\"></div>\n\n");
    h.push_str(&format!("<p>{}</p>\n", tr("Create a brand new user and add them to this site.")));
    h.push_str("<form method=\"post\" name=\"createuser\" id=\"createuser\" class=\"validate\" novalidate=\"novalidate\"\n\t>\n<input name=\"action\" type=\"hidden\" value=\"createuser\" />\n");
    h.push_str(&format!("\t<input type=\"hidden\" id=\"_wpnonce_create-user\" name=\"_wpnonce_create-user\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />\t<table class=\"form-table\" role=\"presentation\">\n", ctx.create_nonce("create-user"), esc_url(&request.request_uri)));
    h.push_str(&format!("\t<tr class=\"form-field form-required\">\n\t\t<th scope=\"row\"><label for=\"user_login\">{} <span class=\"description\">{}</span></label></th>\n\t\t<td><input name=\"user_login\" type=\"text\" id=\"user_login\" class=\"ltr\" value=\"{}\" aria-required=\"true\" autocapitalize=\"none\" autocorrect=\"off\" autocomplete=\"off\" maxlength=\"60\" /></td>\n\t</tr>\n", tr("Username"), tr("(required)"), esc_attr(&submitted("user_login"))));
    h.push_str(&format!("\t<tr class=\"form-field form-required\">\n\t\t<th scope=\"row\"><label for=\"email\">{} <span class=\"description\">{}</span></label></th>\n\t\t<td><input name=\"email\" type=\"email\" id=\"email\" value=\"{}\" /></td>\n\t</tr>\n", tr("Email"), tr("(required)"), esc_attr(&submitted("email"))));
    h.push_str(&format!("\t\t<tr class=\"form-field\">\n\t\t<th scope=\"row\"><label for=\"first_name\">{} </label></th>\n\t\t<td><input name=\"first_name\" type=\"text\" id=\"first_name\" value=\"{}\" /></td>\n\t</tr>\n", tr("First Name"), esc_attr(&submitted("first_name"))));
    h.push_str(&format!("\t<tr class=\"form-field\">\n\t\t<th scope=\"row\"><label for=\"last_name\">{} </label></th>\n\t\t<td><input name=\"last_name\" type=\"text\" id=\"last_name\" value=\"{}\" /></td>\n\t</tr>\n", tr("Last Name"), esc_attr(&submitted("last_name"))));
    h.push_str(&format!("\t<tr class=\"form-field\">\n\t\t<th scope=\"row\"><label for=\"url\">{}</label></th>\n\t\t<td><input name=\"url\" type=\"url\" id=\"url\" class=\"code\" value=\"{}\" /></td>\n\t</tr>\n", tr("Website"), esc_attr(&submitted("url"))));
    h.push_str(&format!("\t\t\t\t<tr class=\"form-field user-language-wrap\">\n\t\t\t<th scope=\"row\">\n\t\t\t\t<label for=\"locale\">\n\t\t\t\t\t\t\t\t\t\t{}<span class=\"dashicons dashicons-translation\" aria-hidden=\"true\"></span>\n\t\t\t\t</label>\n\t\t\t</th>\n\t\t\t<td>\n\t\t\t\t{}\t\t\t</td>\n\t\t</tr>\n", tr("Language"), language_select(ctx, "site-default", true, false)));
    h.push_str(&format!(
        "\t\t\t<tr class=\"form-field form-required user-pass1-wrap\">\n\t\t<th scope=\"row\">\n\t\t\t<label for=\"pass1\">\n\t\t\t\t{}\t\t\t\t<span class=\"description hide-if-js\">{}</span>\n\t\t\t</label>\n\t\t</th>\n\t\t<td>\n\t\t\t<input type=\"hidden\" value=\" \" /><!-- #24364 workaround -->\n\t\t\t<button type=\"button\" class=\"button wp-generate-pw hide-if-no-js\">{}</button>\n\t\t\t<div class=\"wp-pwd\">\n\t\t\t\t\t\t\t\t<div class=\"password-input-wrapper\">\n\t\t\t\t\t<input type=\"password\" name=\"pass1\" id=\"pass1\" class=\"regular-text ltr\" autocomplete=\"new-password\" spellcheck=\"false\" data-reveal=\"1\" data-pw=\"{}\" aria-describedby=\"pass-strength-result\" />\n\t\t\t\t\t<div style=\"display:none\" id=\"pass-strength-result\" aria-live=\"polite\"></div>\n\t\t\t\t</div>\n\t\t\t\t<button type=\"button\" class=\"button wp-hide-pw user-new-password-toggle hide-if-no-js\" data-toggle=\"0\" aria-label=\"{}\">\n\t\t\t\t\t<span class=\"dashicons dashicons-hidden\" aria-hidden=\"true\"></span>\n\t\t\t\t\t<span class=\"text\">{}</span>\n\t\t\t\t</button>\n\t\t\t</div>\n\t\t</td>\n\t</tr>\n",
        tr("Password"),
        tr("(required)"),
        tr("Generate password"),
        esc_attr(&sanitize::random_password(24, true)),
        tr("Hide password"),
        tr("Hide")
    ));
    h.push_str(&format!(
        "\t<tr class=\"form-field form-required user-pass2-wrap hide-if-js\">\n\t\t<th scope=\"row\"><label for=\"pass2\">{} <span class=\"description\">{}</span></label></th>\n\t\t<td>\n\t\t<input type=\"password\" name=\"pass2\" id=\"pass2\" autocomplete=\"new-password\" spellcheck=\"false\" aria-describedby=\"pass2-desc\" />\n\t\t<p class=\"description\" id=\"pass2-desc\">{}</p>\n\t\t</td>\n\t</tr>\n\t<tr class=\"pw-weak\">\n\t\t<th>{}</th>\n\t\t<td>\n\t\t\t<label>\n\t\t\t\t<input type=\"checkbox\" name=\"pw_weak\" class=\"pw-checkbox\" />\n\t\t\t\t{}\t\t\t</label>\n\t\t</td>\n\t</tr>\n",
        tr("Repeat Password"),
        tr("(required)"),
        tr("Type the password again."),
        tr("Confirm Password"),
        tr("Confirm use of weak password")
    ));
    h.push_str(&format!(
        "\t<tr>\n\t\t<th scope=\"row\">{}</th>\n\t\t<td>\n\t\t\t<input type=\"checkbox\" name=\"send_user_notification\" id=\"send_user_notification\" value=\"1\" {} />\n\t\t\t<label for=\"send_user_notification\">{}</label>\n\t\t</td>\n\t</tr>\n",
        tr("Send User Notification"),
        checked(true),
        tr("Send the new user an email about their account")
    ));
    h.push_str(&format!("\t\t\t<tr class=\"form-field\">\n\t\t<th scope=\"row\"><label for=\"role\">{}</label></th>\n\t\t<td><select name=\"role\" id=\"role\">\n\t\t\t{}\t\t\t</select>\n\t\t</td>\n\t</tr>\n", tr("Role"), role_options(&names, ctx, &default_role)));
    h.push_str(&format!("\t\t</table>\n\n\t\n\t<p class=\"submit\"><input type=\"submit\" name=\"createuser\" id=\"createusersub\" class=\"button button-primary\" value=\"{}\"  /></p>\n</form>\n</div>\n\n", esc_attr(&tr("Add User"))));
    let screen = Screen { file: "user-new.php".into(), parent_file: "users.php".into(), submenu_file: Some("user-new.php".into()), title: tr("Add User"), help: Some(new_help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["wp-ajax-response".into(), "user-profile".into()], ..Default::default() }).await
}
