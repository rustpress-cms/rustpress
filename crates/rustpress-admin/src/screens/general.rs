//! `wp-admin/options-general.php`: the General Settings screen.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use super::notices::{self, SettingsError};
use super::{AdminResponse, Ctx, render, timezones};
use crate::chrome::{Assets, Screen};

/// `form_option( $name )`: the option escaped for an attribute.
fn form_option(ctx: &Ctx<'_>, name: &str) -> String {
    esc_attr(&ctx.site().options.string(name, ""))
}

/// The roles of `wp_user_roles` in stored order, with their display names.
pub async fn role_names(ctx: &Ctx<'_>) -> Vec<(String, String)> {
    let key = format!("{}user_roles", ctx.env.db.table_prefix());
    let raw = ctx.site().options.fetch(ctx.env.db, &key).await.ok().flatten().unwrap_or_default();
    let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Vec::new() };
    let Some(items) = value.as_array() else { return Vec::new() };
    items
        .iter()
        .map(|(slug, role)| {
            let slug = match slug {
                rustpress_phpser::Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                rustpress_phpser::Key::Int(i) => i.to_string(),
            };
            let name = role.get("name").and_then(|n| n.as_str()).unwrap_or(&slug).to_owned();
            (slug, name)
        })
        .collect()
}

/// `wp_dropdown_roles()` for a list given in WordPress's (reversed) order.
fn dropdown_roles(ctx: &Ctx<'_>, roles: &[(String, String)], selected: &str) -> String {
    let mut out = String::new();
    for (slug, name) in roles {
        let name = ctx.site().trx(name, "User role");
        if slug == selected {
            out.push_str(&format!("\n\t<option selected='selected' value='{}'>{name}</option>", esc_attr(slug)));
        } else {
            out.push_str(&format!("\n\t<option value='{}'>{name}</option>", esc_attr(slug)));
        }
    }
    out
}

/// `wp_dropdown_languages()`: the installed languages and, when WordPress knows them, the available translations.
fn dropdown_languages(ctx: &Ctx<'_>, selected: &str) -> String {
    let selected = if selected == "en_US" { "" } else { selected };
    let installed: Vec<&str> = ctx.env.languages.iter().map(|l| l.locale.as_str()).collect();
    let available: Vec<&(String, String, String)> = ctx.env.available_translations.iter().filter(|(locale, _, _)| !installed.contains(&locale.as_str())).collect();
    let show_available = !available.is_empty();
    let mut structure: Vec<String> = Vec::new();
    if show_available {
        structure.push(format!("<optgroup label=\"{}\">", esc_attr(&ctx.site().trx("Installed", "translations"))));
    }
    structure.push(format!("<option value=\"\" lang=\"en\" data-installed=\"1\"{}>English (United States)</option>", if selected.is_empty() { " selected='selected'" } else { "" }));
    for language in &ctx.env.languages {
        structure.push(format!(
            "<option value=\"{}\" lang=\"{}\"{} data-installed=\"1\">{}</option>",
            esc_attr(&language.locale),
            esc_attr(&language.lang),
            if language.locale == selected { " selected='selected'" } else { "" },
            esc_html(&language.native_name)
        ));
    }
    if show_available {
        structure.push("</optgroup>".to_owned());
        structure.push(format!("<optgroup label=\"{}\">", esc_attr(&ctx.site().trx("Available", "translations"))));
        for (locale, native_name, lang) in available {
            structure.push(format!("<option value=\"{}\" lang=\"{}\"{}>{}</option>", esc_attr(locale), esc_attr(lang), if locale == selected { " selected='selected'" } else { "" }, esc_html(native_name)));
        }
        structure.push("</optgroup>".to_owned());
    }
    format!("<select name=\"WPLANG\" id=\"WPLANG\">{}</select>", structure.join("\n"))
}

/// When the offset of `zone` next changes after `now`: `(timestamp, daylight saving begins)`.
fn next_transition(zone: chrono_tz::Tz, now: i64) -> Option<(i64, bool)> {
    use chrono::{Offset, TimeZone};
    let offsets = |ts: i64| -> Option<(i32, i32)> {
        let local = zone.timestamp_opt(ts, 0).single()?;
        let offset = local.offset();
        use chrono_tz::OffsetComponents;
        Some((offset.fix().local_minus_utc(), offset.dst_offset().num_seconds() as i32))
    };
    let start = offsets(now)?;
    let mut low = now;
    let mut step = 86_400;
    let mut high = now;
    while high < now + 800 * 86_400 {
        high += step;
        if offsets(high)? != start {
            // Bisect (low, high] to the second.
            let (mut lo, mut hi) = (high - step, high);
            while hi - lo > 1 {
                let mid = lo + (hi - lo) / 2;
                if offsets(mid)? == start { lo = mid } else { hi = mid }
            }
            let after = offsets(hi)?;
            return Some((hi, after.1 != 0));
        }
        low = high;
    }
    let _ = (low, &mut step);
    None
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    if !ctx.can("manage_options") {
        return super::die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr("Sorry, you are not allowed to manage options for this site.")));
    }
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let title = tr("General Settings");
    let mut h = String::new();

    // Settings errors of the last save.
    let mut notices_html = String::new();
    if ctx.request.get("settings-updated").is_some() {
        let key = format!("{}settings_errors", "_transient_");
        if let Ok(Some(raw)) = ctx.env.db.option(&key).await
            && let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes())
            && let Some(items) = value.as_array()
        {
            let errors: Vec<SettingsError> = items
                .iter()
                .map(|(_, e)| {
                    let field = |n: &str| e.get(n).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
                    SettingsError { setting: field("setting"), code: field("code"), message: field("message"), kind: field("type") }
                })
                .collect();
            notices_html = notices::render_settings_errors(&errors, false);
        }
    }

    h.push_str("<div class=\"wrap\">\n");
    h.push_str(&format!("<h1>{}</h1>\n", esc_html(&title)));
    h.push_str(&notices_html);
    h.push_str(&format!("\n<form method=\"post\" action=\"options.php\" novalidate=\"novalidate\">\n{}\n<table class=\"form-table\" role=\"presentation\">\n\n", ctx.settings_fields("general")));

    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"blogname\">{}</label></th>\n<td><input name=\"blogname\" type=\"text\" id=\"blogname\" value=\"{}\" class=\"regular-text\" /></td>\n</tr>\n\n",
        tr("Site Title"),
        form_option(ctx, "blogname")
    ));
    let tagline = rustpress_core::i18n::sprintf(&tr("In a few words, explain what this site is about. Example: &#8220;%s.&#8221;"), &[&tr("Just another WordPress site")]);
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"blogdescription\">{}</label></th>\n<td><input name=\"blogdescription\" type=\"text\" id=\"blogdescription\" aria-describedby=\"tagline-description\" value=\"{}\" class=\"regular-text\" />\n<p class=\"description\" id=\"tagline-description\">{tagline}</p></td>\n</tr>\n\n",
        tr("Tagline"),
        form_option(ctx, "blogdescription")
    ));

    if ctx.can("upload_files") {
        h.push_str(&site_icon_section(ctx).await);
    }

    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"siteurl\">{}</label></th>\n<td><input name=\"siteurl\" type=\"url\" id=\"siteurl\" value=\"{}\" class=\"regular-text code\" /></td>\n</tr>\n\n",
        tr("WordPress Address (URL)"),
        form_option(ctx, "siteurl")
    ));
    let home_help = rustpress_core::i18n::sprintf(&tr("Enter the same address here unless you <a href=\"%s\">want your site home page to be different from your WordPress installation directory</a>."), &[&tr("https://developer.wordpress.org/advanced-administration/server/wordpress-in-directory/")]);
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"home\">{}</label></th>\n<td><input name=\"home\" type=\"url\" id=\"home\" aria-describedby=\"home-description\" value=\"{}\" class=\"regular-text code\" />\n\t<p class=\"description\" id=\"home-description\">\n\t\t{home_help}</p>\n</td>\n</tr>\n\n",
        tr("Site Address (URL)"),
        form_option(ctx, "home")
    ));

    let admin_email = site.options.string("admin_email", "");
    let mut pending = String::new();
    let new_admin_email = site.options.string("new_admin_email", "");
    if !new_admin_email.is_empty() && new_admin_email != admin_email {
        let message = rustpress_core::i18n::sprintf(&tr("There is a pending change of the admin email to %s."), &[&format!("<code>{}</code>", esc_html(&new_admin_email))]);
        let dismiss = esc_url(&format!("{}&_wpnonce={}", ctx.admin_url("options.php?dismiss=new_admin_email"), ctx.create_nonce("dismiss-1-new_admin_email")));
        pending = notices::notice(&format!("{message} <a href=\"{dismiss}\">{}</a>", tr("Cancel")), "", false, "", &["updated", "inline"], true);
    }
    h.push_str(&format!(
        "\n<tr>\n<th scope=\"row\"><label for=\"new_admin_email\">{}</label></th>\n<td><input name=\"new_admin_email\" type=\"email\" id=\"new_admin_email\" aria-describedby=\"new-admin-email-description\" value=\"{}\" class=\"regular-text ltr\" />\n<p class=\"description\" id=\"new-admin-email-description\">{}</p>\n{pending}</td>\n</tr>\n\n",
        tr("Administration Email Address"),
        esc_attr(&admin_email),
        tr("This address is used for admin purposes. If you change this, an email will be sent to your new address to confirm it. <strong>The new address will not become active until confirmed.</strong>")
    ));

    let membership = tr("Membership");
    let registered = site.options.get("users_can_register") == Some("1");
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\">{membership}</th>\n<td><fieldset><legend class=\"screen-reader-text\"><span>{membership}</span></legend><label for=\"users_can_register\">\n<input name=\"users_can_register\" type=\"checkbox\" id=\"users_can_register\" value=\"1\" {} />\n\t{}</label>\n</fieldset></td>\n</tr>\n\n",
        if registered { "checked='checked'" } else { "" },
        tr("Anyone can register")
    ));

    let default_role = site.options.string("default_role", "subscriber");
    let mut roles = role_names(ctx).await;
    roles.reverse();
    roles.retain(|(slug, _)| !(matches!(slug.as_str(), "administrator" | "editor") && *slug != default_role));
    h.push_str(&format!(
        "<tr>\n<th scope=\"row\"><label for=\"default_role\">{}</label></th>\n<td>\n\t\t<select name=\"default_role\" id=\"default_role\">{}</select>\n</td>\n</tr>\n\n",
        tr("New User Default Role"),
        dropdown_roles(ctx, &roles, &default_role)
    ));

    let locale = site.locale();
    let locale = if ctx.env.languages.iter().any(|l| l.locale == locale) { locale } else { String::new() };
    h.push_str(&format!(
        "\t\t<tr>\n\t\t<th scope=\"row\"><label for=\"WPLANG\">{}<span class=\"dashicons dashicons-translation\" aria-hidden=\"true\"></span></label></th>\n\t\t<td>\n\t\t\t{}\t\t</td>\n\t</tr>\n",
        tr("Site Language"),
        dropdown_languages(ctx, &locale)
    ));

    // Timezone
    let offset = site.options.string("gmt_offset", "");
    let mut tzstring = site.options.string("timezone_string", "");
    if tzstring.contains("Etc/GMT") {
        tzstring.clear();
    }
    let mut check_zone_info = true;
    if tzstring.is_empty() {
        check_zone_info = false;
        let value: f64 = offset.parse().unwrap_or(0.0);
        tzstring = if value == 0.0 { "UTC+0".to_owned() } else if value < 0.0 { format!("UTC{offset}") } else { format!("UTC+{offset}") };
    }
    let format = site.trx("Y-m-d H:i:s", "timezone date format");
    h.push_str(&format!(
        "\t<tr>\n<th scope=\"row\"><label for=\"timezone_string\">{}</label></th>\n<td>\n\n<select id=\"timezone_string\" name=\"timezone_string\" aria-describedby=\"timezone-description\">\n\t{}</select>\n\n<p class=\"description\" id=\"timezone-description\">\n{}</p>\n\n",
        tr("Timezone"),
        timezones::choice(site, &tzstring),
        rustpress_core::i18n::sprintf(&tr("Choose either a city in the same timezone as you or a %s (Coordinated Universal Time) time offset."), &["<abbr>UTC</abbr>"])
    ));
    h.push_str(&format!("<p class=\"timezone-info\">\n\t<span id=\"utc-time\">\n\t{}\t</span>\n", rustpress_core::i18n::sprintf(&tr("Universal time is %s."), &[&format!("<code>{}</code>", site.date_utc(&format, ctx.env.now))])));
    if !site.options.string("timezone_string", "").is_empty() || !offset.is_empty() && offset.parse::<f64>().unwrap_or(0.0) != 0.0 {
        h.push_str(&format!("	<span id=\"local-time\">\n\t{}\t</span>\n", rustpress_core::i18n::sprintf(&tr("Local time is %s."), &[&format!("<code>{}</code>", site.date(&format, ctx.env.now))])));
    }
    h.push_str("</p>\n\n");
    if check_zone_info && let Ok(zone) = tzstring.parse::<chrono_tz::Tz>() {
        let now = ctx.env.now;
        let dst_now = {
            use chrono::TimeZone;
            use chrono_tz::OffsetComponents;
            zone.timestamp_opt(now, 0).single().is_some_and(|t| t.offset().dst_offset().num_seconds() != 0)
        };
        let status = if dst_now { tr("This timezone is currently in daylight saving time.") } else { tr("This timezone is currently in standard time.") };
        let next = match next_transition(zone, now) {
            Some((ts, begins_dst)) => {
                let message = if begins_dst { tr("Daylight saving time begins on: %s.") } else { tr("Standard time begins on: %s.") };
                format!(" {}", rustpress_core::i18n::sprintf(&message, &[&format!("<code>{}</code>", site.date(&tr("F j, Y g:i a"), ts))]))
            }
            None => tr("This timezone does not observe daylight saving time."),
        };
        h.push_str(&format!("<p class=\"timezone-info\">\n<span>\n\t{status}\t<br />\n\t{next}\t</span>\n</p>\n"));
    }
    h.push_str("</td>\n</tr>\n\n");

    // Date and time format
    h.push_str(&format_rows(ctx, "date_format", &tr("Date Format"), &[tr("F j, Y"), "Y-m-d".into(), "m/d/Y".into(), "d/m/Y".into(), "d.m.Y".into()], false));
    h.push_str(&format_rows(ctx, "time_format", &tr("Time Format"), &[tr("g:i a"), "g:i A".into(), "H:i".into()], true));

    // Week starts on
    let start = site.options.string("start_of_week", "0").parse::<i64>().unwrap_or(0);
    let days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    let mut weekdays = String::new();
    for (index, day) in days.iter().enumerate() {
        let selected = if start == index as i64 { "selected=\"selected\"" } else { "" };
        weekdays.push_str(&format!("\n\t<option value='{index}' {selected}>{}</option>", site.locale_data().weekday[index]));
        let _ = day;
    }
    h.push_str(&format!("<tr>\n<th scope=\"row\"><label for=\"start_of_week\">{}</label></th>\n<td><select name=\"start_of_week\" id=\"start_of_week\">\n{weekdays}</select></td>\n</tr>\n", tr("Week Starts On")));
    h.push_str("</table>\n\n\n");
    h.push_str(&format!("<p class=\"submit\"><input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button button-primary\" value=\"{}\"  /></p></form>\n\n</div>\n\n", esc_attr(&tr("Save Changes"))));

    let screen = Screen { file: "options-general.php".into(), parent_file: "options-general.php".into(), submenu_file: Some("options-general.php".into()), title, help: Some(help(ctx)), ..Default::default() };
    // The language drop-down sets the global `$locale` to '' when the site's language is not installed.
    let installed = ctx.env.languages.iter().any(|l| l.locale == ctx.site().locale());
    let mut assets = Assets { head: options_general_js(ctx), empty_locale: !installed, ..Default::default() };
    if ctx.can("upload_files") {
        super::media_ui::enqueue(ctx, &mut assets, 0).await;
        // `wp_enqueue_media()` runs in the page, after the header was printed.
        assets.post_header_scripts = std::mem::take(&mut assets.scripts);
        assets.post_header_styles = std::mem::take(&mut assets.styles);
        assets.post_header_scripts.push("site-icon".into());
    }
    render(ctx, &screen, &h, &assets).await
}

/// The radio buttons of a date or time format.
fn format_rows(ctx: &Ctx<'_>, option: &str, title: &str, formats: &[String], time: bool) -> String {
    let site = ctx.site();
    let current = site.options.string(option, "");
    let mut out = format!("<tr>\n<th scope=\"row\">{title}</th>\n<td>\n\t<fieldset><legend class=\"screen-reader-text\"><span>{title}</span></legend>\n");
    let mut seen: Vec<&String> = Vec::new();
    let mut custom = true;
    for format in formats {
        if seen.contains(&format) {
            continue;
        }
        seen.push(format);
        out.push_str(&format!("\t<label><input type='radio' name='{option}' value='{}'", esc_attr(format)));
        if current == *format {
            out.push_str(" checked='checked'");
            custom = false;
        }
        out.push_str(&format!(" /> <span class=\"date-time-text format-i18n\">{}</span><code>{}</code></label><br />\n", site.date(format, ctx.env.now), esc_html(format)));
    }
    let (what, enter, label) = if time {
        (ctx.tr("Custom:"), ctx.tr("enter a custom time format in the following field"), ctx.tr("Custom time format:"))
    } else {
        (ctx.tr("Custom:"), ctx.tr("enter a custom date format in the following field"), ctx.tr("Custom date format:"))
    };
    let kind = if time { "time" } else { "date" };
    out.push_str(&format!(
        "<label><input type=\"radio\" name=\"{option}\" id=\"{kind}_format_custom_radio\" value=\"\\c\\u\\s\\t\\o\\m\"{}/> <span class=\"date-time-text date-time-custom-text\">{what}<span class=\"screen-reader-text\"> {enter}</span></span></label><label for=\"{option}_custom\" class=\"screen-reader-text\">{label}</label><input type=\"text\" name=\"{option}_custom\" id=\"{option}_custom\" value=\"{}\" class=\"small-text\" /><br /><p><strong>{}</strong> <span class=\"example\">{}</span><span class='spinner'></span>\n</p>",
        if custom { " checked='checked'" } else { "" },
        esc_attr(&current),
        ctx.tr("Preview:"),
        site.date(&current, ctx.env.now)
    ));
    if time {
        out.push_str(&format!("\t<p class='date-time-doc'>{}</p>\n", ctx.tr("<a href=\"https://wordpress.org/documentation/article/customize-date-and-time-format/\">Documentation on date and time formatting</a>.")));
    }
    out.push_str("\t</fieldset>\n</td>\n</tr>\n");
    if !time {
        out.push('\n');
    }
    out
}

/// The Site Icon row.
async fn site_icon_section(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let id = site.options.u64("site_icon", 0);
    let mut url = String::new();
    if id != 0
        && let Ok(rows) = site.db.post_meta(id).await
    {
        let value = |key: &str| rows.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
        if let Some(file) = value("_wp_attached_file") {
            let full = rustpress_blocks::images::attachment_url(site, &file);
            url = value("_wp_attachment_metadata").and_then(|raw| rustpress_blocks::images::Metadata::parse(&raw)).and_then(|meta| rustpress_blocks::images::intermediate_url(&full, &meta, 512, 512)).unwrap_or(full);
        }
    }
    let has = !url.is_empty();
    let (wrapper, button_classes, change_classes) = if has { (" has-site-icon", "button", "upload-button button-hero button") } else { (" hidden", "upload-button button-hero button", "button") };
    let filename = url.rsplit('/').next().unwrap_or_default().to_owned();
    let (app_alt, browser_alt) = if id != 0 {
        (
            rustpress_core::i18n::sprintf(&tr("App icon preview: The current image has no alternative text. The file name is: %s"), &[&filename]),
            rustpress_core::i18n::sprintf(&tr("Browser icon preview: The current image has no alternative text. The file name is: %s"), &[&filename]),
        )
    } else {
        (String::new(), String::new())
    };
    let esc_url_value = esc_url(&url);
    format!(
        "<tr class=\"hide-if-no-js site-icon-section\">\n<th scope=\"row\">{icon}</th>\n<td>\n\t\n\t<style>\n\t:root {{\n\t\t--site-icon-url: url( '{esc_url_value}' );\n\t}}\n\t</style>\n\n\t<div id=\"site-icon-preview\" class=\"site-icon-preview settings {wrapper}\">\n\t\t<div class=\"direction-wrap\">\n\t\t\t<img id=\"app-icon-preview\" src=\"{esc_url_value}\" class=\"app-icon-preview\" alt=\"{}\" />\n\t\t\t<div class=\"site-icon-preview-browser\">\n\t\t\t\t<svg role=\"img\" aria-hidden=\"true\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\" class=\"browser-buttons\"><path fill-rule=\"evenodd\" clip-rule=\"evenodd\" d=\"M0 20a6 6 0 1 1 12 0 6 6 0 0 1-12 0Zm18 0a6 6 0 1 1 12 0 6 6 0 0 1-12 0Zm24-6a6 6 0 1 0 0 12 6 6 0 0 0 0-12Z\" /></svg>\n\t\t\t\t<div class=\"site-icon-preview-tab\">\n\t\t\t\t\t<img id=\"browser-icon-preview\" src=\"{esc_url_value}\" class=\"browser-icon-preview\" alt=\"{}\" />\n\t\t\t\t\t<div class=\"site-icon-preview-site-title\" id=\"site-icon-preview-site-title\" aria-hidden=\"true\">{}</div>\n\t\t\t\t\t\t<svg role=\"img\" aria-hidden=\"true\" fill=\"none\" xmlns=\"http://www.w3.org/2000/svg\" class=\"close-button\">\n\t\t\t\t\t\t\t<path d=\"M12 13.0607L15.7123 16.773L16.773 15.7123L13.0607 12L16.773 8.28772L15.7123 7.22706L12 10.9394L8.28771 7.22705L7.22705 8.28771L10.9394 12L7.22706 15.7123L8.28772 16.773L12 13.0607Z\" />\n\t\t\t\t\t\t</svg>\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n\t</div>\n\n\t<input type=\"hidden\" name=\"site_icon\" id=\"site_icon_hidden_field\" value=\"{}\" />\n\t<div class=\"site-icon-action-buttons\">\n\t\t<button type=\"button\"\n\t\t\tid=\"choose-from-library-button\"\n\t\t\tclass=\"{button_classes}\"\n\t\t\tdata-alt-classes=\"{change_classes}\"\n\t\t\tdata-size=\"512\"\n\t\t\tdata-choose-text=\"{}\"\n\t\t\tdata-update-text=\"{}\"\n\t\t\tdata-update=\"{}\"\n\t\t\tdata-state=\"{}\"\n\n\t\t>\n\t\t\t\t\t\t\t{}\t\t\t\t\t</button>\n\t\t<button\n\t\t\tid=\"js-remove-site-icon\"\n\t\t\ttype=\"button\"\n\t\t\tclass=\"button button-secondary reset remove-site-icon{}\"\t\t>\n\t\t\t{}\t\t</button>\n\t</div>\n\n\t<p class=\"description\">\n\t\t{}\t</p>\n\n</td>\n</tr>\n\n\t\n",
        esc_attr(&app_alt),
        esc_attr(&browser_alt),
        esc_html(&site.options.string("blogname", "")),
        form_option(ctx, "site_icon"),
        esc_attr(&tr("Choose a Site Icon")),
        esc_attr(&tr("Change Site Icon")),
        esc_attr(&tr("Set as Site Icon")),
        if has { "1" } else { "" },
        if has { tr("Change Site Icon") } else { tr("Choose a Site Icon") },
        if has { "" } else { " hidden" },
        tr("Remove Site Icon"),
        rustpress_core::i18n::sprintf(&tr("The Site Icon is what you see in browser tabs, bookmark bars, and within the WordPress mobile apps. It should be square and at least <strong>%1$s by %2$s</strong> pixels."), &["512", "512"]),
        icon = tr("Site Icon"),
    )
}

/// `options_general_add_js()`.
fn options_general_js(ctx: &Ctx<'_>) -> String {
    let home = ctx.site().settings.home.clone();
    format!(
        "<script>\n\tjQuery( function($) {{\n\t\tvar $siteName = $( '#wp-admin-bar-site-name' ).children( 'a' ).first(),\n\t\t\t$siteIconPreview = $('#site-icon-preview-site-title'),\n\t\t\thomeURL = ( {} || '' ).replace( /^(https?:\\/\\/)?(www\\.)?/, '' );\n\n\t\t$( '#blogname' ).on( 'input', function() {{\n\t\t\tvar title = $.trim( $( this ).val() ) || homeURL,\n\t\t\t\t$siteIcon = $siteName.children();\n\n\t\t\t// Truncate to 40 characters.\n\t\t\tif ( 40 < title.length ) {{\n\t\t\t\ttitle = title.substring( 0, 40 ) + '\\u2026';\n\t\t\t}}\n\n\t\t\t$siteName.text( title ).prepend( $siteIcon );\n\t\t\t$siteIconPreview.text( title );\n\t\t}});\n\n\t\t$( 'input[name=\"date_format\"]' ).on( 'click', function() {{\n\t\t\tif ( 'date_format_custom_radio' !== $(this).attr( 'id' ) )\n\t\t\t\t$( 'input[name=\"date_format_custom\"]' ).val( $( this ).val() ).closest( 'fieldset' ).find( '.example' ).text( $( this ).parent( 'label' ).children( '.format-i18n' ).text() );\n\t\t}});\n\n\t\t$( 'input[name=\"date_format_custom\"]' ).on( 'click input', function() {{\n\t\t\t$( '#date_format_custom_radio' ).prop( 'checked', true );\n\t\t}});\n\n\t\t$( 'input[name=\"time_format\"]' ).on( 'click', function() {{\n\t\t\tif ( 'time_format_custom_radio' !== $(this).attr( 'id' ) )\n\t\t\t\t$( 'input[name=\"time_format_custom\"]' ).val( $( this ).val() ).closest( 'fieldset' ).find( '.example' ).text( $( this ).parent( 'label' ).children( '.format-i18n' ).text() );\n\t\t}});\n\n\t\t$( 'input[name=\"time_format_custom\"]' ).on( 'click input', function() {{\n\t\t\t$( '#time_format_custom_radio' ).prop( 'checked', true );\n\t\t}});\n\n\t\t$( 'input[name=\"date_format_custom\"], input[name=\"time_format_custom\"]' ).on( 'input', function() {{\n\t\t\tvar format = $( this ),\n\t\t\t\tfieldset = format.closest( 'fieldset' ),\n\t\t\t\texample = fieldset.find( '.example' ),\n\t\t\t\tspinner = fieldset.find( '.spinner' );\n\n\t\t\t// Debounce the event callback while users are typing.\n\t\t\tclearTimeout( $.data( this, 'timer' ) );\n\t\t\t$( this ).data( 'timer', setTimeout( function() {{\n\t\t\t\t// If custom date is not empty.\n\t\t\t\tif ( format.val() ) {{\n\t\t\t\t\tspinner.addClass( 'is-active' );\n\n\t\t\t\t\t$.post( ajaxurl, {{\n\t\t\t\t\t\taction: 'date_format_custom' === format.attr( 'name' ) ? 'date_format' : 'time_format',\n\t\t\t\t\t\tdate \t: format.val()\n\t\t\t\t\t}}, function( d ) {{ spinner.removeClass( 'is-active' ); example.text( d ); }} );\n\t\t\t\t}}\n\t\t\t}}, 500 ) );\n\t\t}} );\n\n\t\tvar languageSelect = $( '#WPLANG' );\n\t\t$( 'form' ).on( 'submit', function() {{\n\t\t\t/*\n\t\t\t * Don't show a spinner for English and installed languages,\n\t\t\t * as there is nothing to download.\n\t\t\t */\n\t\t\tif ( ! languageSelect.find( 'option:selected' ).data( 'installed' ) ) {{\n\t\t\t\t$( '#submit', this ).after( '<span class=\"spinner language-install-spinner is-active\" />' );\n\t\t\t}}\n\t\t}});\n\t}} );\n</script>\n",
        serde_json::to_string(&home).unwrap_or_default()
    )
}

/// The contextual help of the screen.
fn help(ctx: &Ctx<'_>) -> crate::chrome::Help {
    let tr = |s: &str| ctx.tr(s);
    let mut overview = format!("<p>{}</p><p>{}</p>", tr("The fields on this screen determine some of the basics of your site setup."), tr("Most themes show the site title at the top of every page, in the title bar of the browser, and as the identifying name for syndicated feeds. Many themes also show the tagline."));
    overview.push_str(&format!(
        "<p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
        tr("Two terms you will want to know are the WordPress URL and the site URL. The WordPress URL is where the core WordPress installation files are, and the site URL is the address a visitor uses in the browser to go to your site."),
        rustpress_core::i18n::sprintf(
            &tr("Though the terms refer to two different concepts, in practice, they can be the same address or different. For example, you can have the core WordPress installation files in the root directory (<code>https://example.com</code>), in which case the two URLs would be the same. Or the <a href=\"%s\">WordPress files can be in a subdirectory</a> (<code>https://example.com/wordpress</code>). In that case, the WordPress URL and the site URL would be different."),
            &[&tr("https://developer.wordpress.org/advanced-administration/server/wordpress-in-directory/")]
        ),
        rustpress_core::i18n::sprintf(
            &tr("Both WordPress URL and site URL can start with either %1$s or %2$s. A URL starting with %2$s requires an SSL certificate, so be sure that you have one before changing to %2$s. With %2$s, a padlock will appear next to the address in the browser address bar. Both %2$s and the padlock signal that your site meets some basic security requirements, which can build trust with your visitors and with search engines."),
            &["<code>http://</code>", "<code>https://</code>"]
        ),
        tr("If you want site visitors to be able to register themselves, check the membership box. If you want the site administrator to register every new user, leave the box unchecked. In either case, you can set a default user role for all new users.")
    ));
    overview.push_str(&format!(
        "<p>{}</p><p>{}</p><p>{}</p>",
        tr("You can set the language, and WordPress will automatically download and install the translation files (available if your filesystem is writable)."),
        tr("UTC means Coordinated Universal Time."),
        tr("You must click the Save Changes button at the bottom of the screen for new settings to take effect.")
    ));
    crate::chrome::Help {
        tabs: vec![crate::chrome::HelpTab { id: "overview".into(), title: tr("Overview"), content: overview }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/documentation/article/settings-general-screen/\">Documentation on General Settings</a>"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}
