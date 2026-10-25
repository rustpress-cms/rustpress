//! `wp-admin/profile.php`, `user-edit.php` and `user-new.php`.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;
use rustpress_db::User;

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

fn color_picker(ctx: &Ctx<'_>, current: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let mut out = format!("\t<fieldset id=\"color-picker\" class=\"scheme-list\">\n\t\t<legend class=\"screen-reader-text\"><span>{}</span></legend>\n\t\t<input type=\"hidden\" id=\"color-nonce\" name=\"color-nonce\" value=\"{}\" />", tr("Administration Color Scheme"), ctx.create_nonce("save-color-scheme"));
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
            out.push_str(&format!("\t\t\t\t\t<div class=\"color-palette-shade\" style=\"background-color: {shade}\">&nbsp;</div>\n\t\t\t\t\t"));
        }
        out.push_str("\t\t\t\t</div>\n\t\t\t</div>\n\t\t");
    }
    out.push_str("\t\t</fieldset>\n");
    out
}

fn avatar(ctx: &Ctx<'_>, email: &str, size: u64) -> String {
    let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&ctx.site().options, email, s));
    format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-{size} photo' height='{size}' width='{size}' loading='lazy' decoding='async'/>", url(size), url(size * 2))
}

fn language_select(ctx: &Ctx<'_>, current: &str, with_site_default: bool) -> String {
    let mut options = String::new();
    if with_site_default {
        options.push_str(&format!("<option value=\"site-default\" data-installed=\"1\"{}>{}</option>\n", selected(current == "site-default"), ctx.tr("Site Default")));
    }
    options.push_str(&format!("<option value=\"\" lang=\"en\" data-installed=\"1\"{}>English (United States)</option>\n", selected(current.is_empty())));
    for language in &ctx.env.languages {
        options.push_str(&format!("<option value=\"{}\" lang=\"{}\" data-installed=\"1\"{}>{}</option>\n", esc_attr(&language.locale), esc_attr(&language.lang), selected(language.locale == current), esc_html(&language.native_name)));
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
    let heading = if own { tr("Profile") } else { sprintf(&tr("Edit User %s"), &[&esc_html(&user.user_login)]) };
    let action_url = ctx.admin_url(if own { "profile.php" } else { "user-edit.php" });
    let mut h = String::from("\t\t<div class=\"wrap\" id=\"profile-page\">\n");
    h.push_str(&format!("\t\t\t<h1 class=\"wp-heading-inline\">\n\t\t\t\t\t{heading}\t\t\t</h1>\n\n\t\t\t"));
    if !own && ctx.can("create_users") {
        h.push_str(&format!("\t\t\t\t\t\t\t\t<a href=\"user-new.php\" class=\"page-title-action\">{}</a>\n\t\t\t\t\t\t\t\n", tr("Add User")));
    }
    h.push_str("\t\t\t<hr class=\"wp-header-end\">\n\n");
    if request.get("updated").is_some() {
        let message = if own { tr("<strong>Profile updated.</strong>") } else { tr("<strong>User updated.</strong>") };
        h.push_str(&format!("<div id=\"message\" class=\"updated notice is-dismissible\"><p>{message}</p>\n"));
        if let Some(referer) = request.get("wp_http_referer").filter(|r| !r.is_empty()) {
            h.push_str(&format!("<p><a href=\"{}\">{}</a></p>\n", esc_url(referer), tr("&larr; Go to Users")));
        }
        h.push_str("</div>\n");
    }
    if !errors.is_empty() {
        h.push_str(&format!("<div class=\"error\"><ul>\n{}</ul></div>\n", errors.iter().map(|e| format!("\t<li>{e}</li>\n")).collect::<String>()));
    }
    h.push_str(&format!(
        "\t\t\t<form id=\"your-profile\" action=\"{}\" method=\"post\" novalidate=\"novalidate\"\n\t\t\t\t\t\t\t\t>\n\t\t\t\t{}\t\t\t\t<p>\n\t\t\t\t\t<input type=\"hidden\" name=\"from\" value=\"profile\" />\n\t\t\t\t\t<input type=\"hidden\" name=\"checkuser_id\" value=\"{}\" />\n\t\t\t\t</p>\n\n",
        esc_url(&action_url),
        ctx.nonce_field(&format!("update-user_{id}")),
        ctx.user.id
    ));
    h.push_str(&format!("\t\t\t\t<h2>{}</h2>\n\n\t\t\t\t<table class=\"form-table\" role=\"presentation\">\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t\t\n", tr("Personal Options")));
    if own {
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
        "\t\t\t\t\t\t\t\t\t\t<tr class=\"user-language-wrap\">\n\t\t\t\t\t\t<th scope=\"row\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<label for=\"locale\">{}<span class=\"dashicons dashicons-translation\" aria-hidden=\"true\"></span></label>\n\t\t\t\t\t\t</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t\t\n\t\t\t\t\t\n\t\t\t\t</table>\n\t\t\t\t\n",
        tr("Language"),
        language_select(ctx, &locale, true)
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
            "\t\t\t\t\t\t\t<tr class=\"user-role-wrap\">\n\t\t\t\t\t\t\t<th><label for=\"role\">{}</label></th>\n\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t<select name=\"role\" id=\"role\">\n\t\t\t\t\t\t\t\t\t{}<option value=\"\">&mdash; {} &mdash;</option>\t\t\t\t\t\t\t</select>\n\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t</tr>\n\t\t\t\t\t\n",
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
        "\t\t\t\t\t<tr class=\"user-display-name-wrap\">\n\t\t\t\t\t\t<th>\n\t\t\t\t\t\t\t<label for=\"display_name\">{}</label>\n\t\t\t\t\t\t</th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t<select name=\"display_name\" id=\"display_name\">\n{choice_html}\t\t\t\t\t\t\t</select>\n\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\t\t\t\t</table>\n\n",
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
    h.push_str(&format!(
        "\t\t\t\t\t<tr class=\"user-email-wrap\">\n\t\t\t\t\t\t<th><label for=\"email\">{} <span class=\"description\">{}</span></label></th>\n\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<input type=\"email\" name=\"email\" id=\"email\"{} value=\"{}\"{} class=\"regular-text ltr\" />\n{email_note}\t\t\t\t\t\t\t\n\t\t\t\t\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t</tr>\n\n",
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
        "\t\t\t\t\t\t<tr class=\"user-profile-picture\">\n\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t<td>\n\t\t\t\t\t\t\t\t{}\n\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t</tr>\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t</table>\n\n",
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
    if own {
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<tr class=\"user-sessions-wrap hide-if-no-js\">\n\t\t\t\t\t\t\t\t\t<th>{}</th>\n\t\t\t\t\t\t\t\t\t<td aria-live=\"assertive\">\n\t\t\t\t\t\t\t\t\t\t<div class=\"destroy-sessions\"><button type=\"button\" class=\"button\" id=\"destroy-sessions\">{}</button></div>\n\t\t\t\t\t\t\t\t\t\t<p class=\"description\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</p>\n\t\t\t\t\t\t\t\t\t</td>\n\t\t\t\t\t\t\t\t</tr>\n",
            tr("Sessions"),
            tr("Log Out Everywhere Else"),
            tr("Did you lose your phone or leave your account logged in at a public computer? You can log out everywhere else, and stay logged in here.")
        ));
    }
    h.push_str("\t\t\t\t\t\t</table>\n\n");
    h.push_str(&format!("\t\t\t\t<input type=\"hidden\" name=\"action\" value=\"update\" />\n\t\t\t\t<input type=\"hidden\" name=\"user_id\" id=\"user_id\" value=\"{id}\" />\n\n\t\t\t\t<p class=\"submit\"><input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button button-primary\" value=\"{}\"  /></p>\n\t\t\t</form>\n\t\t</div>\n\t\t<script>\n\tif (window.location.hash == '#password') {{\n\t\tdocument.getElementById('pass1').focus();\n\t}}\n</script>\n\n", if own { tr("Update Profile") } else { tr("Update User") }));
    let (parent_file, submenu_file) = if ctx.can("list_users") { ("users.php", if own { "profile.php" } else { "users.php" }) } else { ("profile.php", "profile.php") };
    let view_item = (!own).then(|| {
        let url = if ctx.site().settings.permalink_structure.is_empty() { ctx.site().home_url(&format!("?author={id}")) } else { ctx.site().home_url(&format!("author/{}/", user.user_nicename)) };
        (tr("View User"), url)
    });
    let screen = Screen {
        file: if own { "profile.php".into() } else { "user-edit.php".into() },
        parent_file: parent_file.into(),
        submenu_file: Some(submenu_file.into()),
        title: if own { tr("Profile") } else { tr("Edit User") },
        help: Some(help(ctx)),
        view_item,
        ..Default::default()
    };
    render(ctx, &screen, &h, &Assets { scripts: vec!["user-profile".into()], ..Default::default() }).await
}

fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    Help {
        tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content: format!("<p>{}</p>", tr("Your profile contains information about you (your &#8220;account&#8221;) as well as some personal options related to using WordPress.")) }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

/// `profile.php`.
pub async fn profile(ctx: &Ctx<'_>) -> AdminResponse {
    let user = ctx.env.db.user(ctx.user.id).await.ok().flatten();
    let Some(user) = user else { return forbidden(ctx, "Invalid user ID.") };
    if ctx.request.post && ctx.request.post_field("action") == Some("update") {
        return save(ctx, ctx.user.id, true).await;
    }
    form(ctx, &user, true, &[]).await
}

/// `user-edit.php`.
pub async fn user_edit(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let id: u64 = ctx.request.request("user_id").and_then(|v| v.parse().ok()).unwrap_or(0);
    if id == 0 {
        return super::die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Invalid user ID.")));
    }
    if id == ctx.user.id {
        return redirect(&ctx.admin_url("profile.php"));
    }
    if !ctx.can("edit_users") {
        return super::die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit this user.")));
    }
    let Some(user) = ctx.env.db.user(id).await.ok().flatten() else {
        return super::die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Invalid user ID.")));
    };
    if ctx.request.post && ctx.request.post_field("action") == Some("update") {
        return save(ctx, id, false).await;
    }
    form(ctx, &user, false, &[]).await
}

/// `edit_user()` for the profile forms.
async fn save(ctx: &Ctx<'_>, id: u64, own: bool) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    if !ctx.verify_nonce(&format!("update-user_{id}")) {
        return super::die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
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
    let mut data = UserData {
        id: Some(id),
        user_email: Some(email),
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
        data.role = Some(role);
    }
    // `locale` "site-default" is stored as an empty string like WordPress.
    match content.insert_user(&data).await {
        Ok(_) => {}
        Err(e) => return form(ctx, &user, own, &[format!("<strong>Error:</strong> {e}")]).await,
    }
    if data.user_pass.is_some() && own {
        // The session stays; the other ones end like in `wp_update_user()`.
        let _ = rustpress_auth::session::destroy_other_sessions(ctx.env.db, writer, id, &ctx.token, ctx.env.now).await;
    }
    let mut url = if own { ctx.admin_url("profile.php") } else { ctx.admin_url(&format!("user-edit.php?user_id={id}")) };
    url = add_query_arg(&url, "updated", "1");
    if let Some(referer) = request.post_field("_wp_http_referer").filter(|r| r.contains("wp_http_referer")) {
        let _ = referer;
    }
    redirect(&url)
}

/// `user-new.php`.
pub async fn user_new(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("create_users") {
        return super::die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to add users to this network.")));
    }
    let request = ctx.request;
    let mut errors: Vec<String> = Vec::new();
    let submitted = |name: &str| request.post_field(name).unwrap_or_default().to_owned();
    if request.post && request.post_field("action") == Some("createuser") {
        if request.post_field("_wpnonce_create-user").is_none_or(|n| ctx.env.auth.verify_nonce(n, "create-user", ctx.user.id, &ctx.token, ctx.env.now).is_none()) {
            return super::die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
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
    h.push_str(&format!("\t\t\t<tr class=\"form-field user-language-wrap\">\n\t\t\t<th scope=\"row\">\n\t\t\t\t<label for=\"locale\">\n\t\t\t\t\t\t\t\t\t\t{}<span class=\"dashicons dashicons-translation\" aria-hidden=\"true\"></span>\n\t\t\t\t</label>\n\t\t\t</th>\n\t\t\t<td>\n\t\t\t\t{}\t\t\t</td>\n\t\t</tr>\n", tr("Language"), language_select(ctx, "site-default", true)));
    h.push_str(&format!(
        "\t\t<tr class=\"form-field form-required user-pass1-wrap\">\n\t\t<th scope=\"row\">\n\t\t\t<label for=\"pass1\">\n\t\t\t\t{}\t\t\t\t<span class=\"description hide-if-js\">{}</span>\n\t\t\t</label>\n\t\t</th>\n\t\t<td>\n\t\t\t<input type=\"hidden\" value=\" \" /><!-- #24364 workaround -->\n\t\t\t<button type=\"button\" class=\"button wp-generate-pw hide-if-no-js\">{}</button>\n\t\t\t<div class=\"wp-pwd\">\n\t\t\t\t\t\t\t\t<div class=\"password-input-wrapper\">\n\t\t\t\t\t<input type=\"password\" name=\"pass1\" id=\"pass1\" class=\"regular-text ltr\" autocomplete=\"new-password\" spellcheck=\"false\" data-reveal=\"1\" data-pw=\"{}\" aria-describedby=\"pass-strength-result\" />\n\t\t\t\t\t<div style=\"display:none\" id=\"pass-strength-result\" aria-live=\"polite\"></div>\n\t\t\t\t</div>\n\t\t\t\t<button type=\"button\" class=\"button wp-hide-pw user-new-password-toggle hide-if-no-js\" data-toggle=\"0\" aria-label=\"{}\">\n\t\t\t\t\t<span class=\"dashicons dashicons-hidden\" aria-hidden=\"true\"></span>\n\t\t\t\t\t<span class=\"text\">{}</span>\n\t\t\t\t</button>\n\t\t\t</div>\n\t\t</td>\n\t</tr>\n",
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
        checked(true).trim_start(),
        tr("Send the new user an email about their account")
    ));
    h.push_str(&format!("\t\t\t<tr class=\"form-field\">\n\t\t<th scope=\"row\"><label for=\"role\">{}</label></th>\n\t\t<td><select name=\"role\" id=\"role\">\n\t\t\t{}\t\t\t</select>\n\t\t</td>\n\t</tr>\n", tr("Role"), role_options(&names, ctx, &default_role)));
    h.push_str(&format!("\t\t</table>\n\n\t\n\t<p class=\"submit\"><input type=\"submit\" name=\"createuser\" id=\"createusersub\" class=\"button button-primary\" value=\"{}\"  /></p>\n</form>\n</div>\n\n", esc_attr(&tr("Add User"))));
    let screen = Screen { file: "user-new.php".into(), parent_file: "users.php".into(), submenu_file: Some("user-new.php".into()), title: tr("Add User"), help: Some(help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["user-profile".into()], ..Default::default() }).await
}
