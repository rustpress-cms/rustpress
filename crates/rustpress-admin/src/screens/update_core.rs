//! `wp-admin/update-core.php`: what the update transients announce for WordPress, plugins, themes and translations.
//! RustPress does not fetch or install packages; the screen shows the state like WordPress does.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::{Key, Value};

use super::{AdminResponse, Ctx, die, render};
use crate::chrome::{Assets, Screen};

/// A property of an object or an entry of an array.
fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    let props = match value {
        Value::Object { props, .. } => props,
        Value::Array(items) => items,
        _ => return None,
    };
    props.iter().find_map(|(k, v)| match k {
        Key::Str(s) if s == name.as_bytes() => Some(v),
        _ => None,
    })
}

fn text(value: &Value, name: &str) -> String {
    field(value, name).and_then(Value::as_str).unwrap_or_default().to_owned()
}

async fn transient(ctx: &Ctx<'_>, name: &str) -> Option<Value> {
    let site = ctx.site();
    let raw = site.options.fetch(&site.db, &format!("_site_transient_{name}")).await.ok().flatten()?;
    rustpress_phpser::unserialize(raw.as_bytes()).ok()
}

pub(super) fn wp_root(ctx: &Ctx<'_>) -> std::path::PathBuf {
    ctx.env.loader.includes.parent().map(std::path::Path::to_path_buf).unwrap_or_default()
}

fn nonce_fields(ctx: &Ctx<'_>) -> String {
    format!("<input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />", ctx.create_nonce("upgrade-core"), esc_url(&ctx.request.request_uri))
}

/// `list_core_update()` for one offer.
fn list_core_update(ctx: &Ctx<'_>, update: &Value, version: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let response = text(update, "response");
    let current = text(update, "current");
    let locale = text(update, "locale");
    let site_locale = ctx.site().locale();
    let version_string = if locale == "en_US" && site_locale == "en_US" { current.clone() } else { format!("{current}&ndash;{site_locale}") };
    let is_current = response.is_empty() || response == "latest";
    let (mut message, mut action, mut submit) = (String::new(), "update-core.php?action=do-core-upgrade", sprintf(&tr("Update to version %s"), &[&version_string]));
    if response == "development" {
        message = tr("You can update to the latest nightly build manually:");
    } else if is_current {
        submit = sprintf(&tr("Re-install version %s"), &[&version_string]);
        action = "update-core.php?action=do-core-reinstall";
    } else {
        let version_url = format!("https://wordpress.org/documentation/wordpress-version/version-{}/", rustpress_core::slug::sanitize_title(&current));
        message = sprintf(&tr("You can update from WordPress %1$s to <a href=\"%2$s\">WordPress %3$s</a> manually:"), &[version, &esc_url(&version_url), &version_string]);
    }
    let mut out = format!("<p>{message}</p><form method=\"post\" action=\"{}\" name=\"upgrade\" class=\"upgrade\">{}", esc_url(action), nonce_fields(ctx));
    out.push_str(&format!("<p><input name=\"version\" value=\"{}\" type=\"hidden\" /><input name=\"locale\" value=\"{}\" type=\"hidden\" />", esc_attr(&current), esc_attr(&locale)));
    out.push_str(&format!("<input type=\"submit\" name=\"upgrade\" id=\"upgrade\" class=\"button\" value=\"{}\"  /></p>", esc_attr(&submit)));
    out.push_str("</form>");
    out
}

/// The version the first core offer upgrades to, when there is one.
pub(super) async fn core_upgrade_version(ctx: &Ctx<'_>) -> Option<String> {
    let core = transient(ctx, "update_core").await?;
    let Some(Value::Array(updates)) = field(&core, "updates") else { return None };
    updates.iter().map(|(_, u)| u).find(|u| text(u, "response") == "upgrade").map(|u| text(u, "current"))
}

/// `core_upgrade_preamble()`.
async fn core_section(ctx: &Ctx<'_>, version: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let core = transient(ctx, "update_core").await;
    let updates: Vec<&Value> = core.as_ref().and_then(|c| field(c, "updates")).map(|u| match u {
        Value::Array(items) => items.iter().map(|(_, v)| v).collect(),
        _ => Vec::new(),
    }).unwrap_or_default();
    let newer = updates.first().filter(|u| text(u, "response") == "upgrade");
    let mut out = String::new();
    if newer.is_some() {
        out.push_str(&format!("<h2 class=\"response\">{}</h2>", tr("An updated version of WordPress is available.")));
        let message = sprintf(
            &tr("<strong>Important:</strong> Before updating, please <a href=\"%1$s\">back up your database and files</a>. For help with updates, visit the <a href=\"%2$s\">Updating WordPress</a> documentation page."),
            &[&tr("https://developer.wordpress.org/advanced-administration/security/backup/"), &tr("https://wordpress.org/documentation/article/updating-wordpress/")],
        );
        out.push_str(&format!("<div class=\"notice notice-warning inline\"><p>{message}</p></div>\n"));
    } else if version.contains("alpha") || version.contains("beta") || version.contains("RC") {
        out.push_str(&format!("<h2 class=\"response\">{}</h2>", tr("You are using a development version of WordPress.")));
    } else {
        out.push_str(&format!("<h2 class=\"response\">{}</h2>", tr("You have the latest version of WordPress.")));
    }
    out.push_str("<ul class=\"core-updates\">");
    for update in &updates {
        out.push_str("<li>");
        out.push_str(&list_core_update(ctx, update, version));
        out.push_str("</li>");
    }
    out.push_str("</ul>");
    if !updates.is_empty() && (updates.len() > 1 || text(updates[0], "response") != "latest") {
        out.push_str(&format!("<p>{}</p>", tr("While your site is being updated, it will be in maintenance mode. As soon as your updates are complete, this mode will be deactivated.")));
    } else if updates.is_empty() {
        let normalized = version.split('-').next().unwrap_or(version);
        out.push_str(&format!("<p>{}</p>", sprintf(&tr("<a href=\"%1$s\">Learn more about WordPress %2$s</a>."), &[&esc_url(&ctx.admin_url("about.php")), normalized])));
    }
    out
}

/// `core_auto_updates_settings()`: the paragraph about automatic updates.
fn auto_update_status(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let minor = site.options.string("auto_update_core_minor", "enabled") == "enabled";
    let major = site.options.string("auto_update_core_major", "unset") == "enabled";
    let action_url = ctx.admin_url("update-core.php?action=core-major-auto-updates-settings");
    let link = |value: &str, class: &str, label: String| {
        format!("<br /><a href=\"{}\" class=\"core-auto-update-settings-link {class}\">{label}</a>", esc_url(&format!("{action_url}&value={value}&_wpnonce={}", ctx.create_nonce("core-major-auto-updates-nonce"))).replace("&#038;", "&amp;"))
    };
    let body = if major {
        format!("{}{}", tr("This site is automatically kept up to date with each new version of WordPress."), link("disable", "core-auto-update-settings-link-disable", tr("Switch to automatic updates for maintenance and security releases only.")))
    } else if minor {
        format!("{}{}", tr("This site is automatically kept up to date with maintenance and security releases of WordPress only."), link("enable", "core-auto-update-settings-link-enable", tr("Enable automatic updates for all new versions of WordPress.")))
    } else {
        tr("This site will not receive automatic updates for new versions of WordPress.")
    };
    format!("\t<p class=\"auto-update-status\">\n\t\t{body}\t</p>\n")
}

/// `list_plugin_updates()`.
async fn plugin_section(ctx: &Ctx<'_>, version: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let Some(transient) = transient(ctx, "update_plugins").await else { return format!("<h2>{}</h2><p>{}</p>", tr("Plugins"), tr("Your plugins are all up to date.")) };
    let entries: Vec<(String, &Value)> = match field(&transient, "response") {
        Some(Value::Array(items)) => items.iter().filter_map(|(k, v)| if let Key::Str(name) = k { Some((String::from_utf8_lossy(name).into_owned(), v)) } else { None }).collect(),
        _ => Vec::new(),
    };
    if entries.is_empty() {
        return format!("<h2>{}</h2><p>{}</p>", tr("Plugins"), tr("Your plugins are all up to date."));
    }
    let installed = crate::plugins::installed(&wp_root(ctx));
    let current_version = version.split('-').next().unwrap_or(version).to_owned();
    let mut rows = String::new();
    for (file, update) in &entries {
        let info = installed.iter().find(|p| &p.file == file);
        let name = info.map_or(file.clone(), |p| p.name.clone());
        let installed_version = info.map(|p| p.version.clone()).unwrap_or_default();
        let new_version = text(update, "new_version");
        let slug = text(update, "slug");
        let icon = ["svg", "2x", "1x", "default"].iter().find_map(|k| field(update, "icons").and_then(|i| field(i, k)).and_then(Value::as_str).filter(|s| !s.is_empty()).map(|s| format!("<img src=\"{}\" alt=\"\" class=\"plugin-icon\" />", esc_url(s)))).unwrap_or_else(|| "<span class=\"dashicons dashicons-admin-plugins\"></span>".to_owned());
        let tested = text(update, "tested");
        let compat = if !tested.is_empty() && tested >= current_version { sprintf(&tr("Compatibility with WordPress %s: Yes (according to its author)"), &[&current_version]) } else { sprintf(&tr("Compatibility with WordPress %s: Not tested"), &[&current_version]) };
        let details_url = ctx.admin_url(&format!("plugin-install.php?tab=plugin-information&plugin={slug}&section=changelog&TB_iframe=true&width=640&height=662"));
        let details = format!(
            "<a href=\"{}\" class=\"thickbox open-plugin-details-modal\" aria-label=\"{}\">{}</a>",
            esc_url(&details_url),
            esc_attr(&sprintf(&tr("View %1$s version %2$s details"), &[&name, &new_version])),
            sprintf(&tr("View version %s details."), &[&new_version])
        );
        let notice = field(update, "upgrade_notice").and_then(Value::as_str).map(|n| format!("<br />{}", rustpress_core::formatting::strip_tags(n))).unwrap_or_default();
        let checkbox_id = format!("checkbox_{}", crate::deps::md5_hex(&name));
        rows.push_str(&format!(
            "\t<tr>\n\t\t<td class=\"check-column\">\n\t\t\t\t\t\t\t<input type=\"checkbox\" name=\"checked[]\" id=\"{checkbox_id}\" value=\"{}\" />\n\t\t\t\t<label for=\"{checkbox_id}\">\n\t\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t</label>\n\t\t\t\t\t</td>\n\t\t<td class=\"plugin-title\"><p>\n\t\t\t{icon}\n\t\t\t<strong>{name}</strong>\n\t\t\t{} {details}<br />{compat}{notice}\t\t</p></td>\n\t</tr>\n\t\t\t",
            esc_attr(file),
            sprintf(&tr("Select %s"), &[&name]),
            sprintf(&tr("You have version %1$s installed. Update to %2$s."), &[&installed_version, &new_version])
        ));
    }
    format!(
        "<h2>\n\t{} <span class=\"count\">({})</span></h2>\n<p>{}</p>\n<form method=\"post\" action=\"update-core.php?action=do-plugin-upgrade\" name=\"upgrade-plugins\" class=\"upgrade\">\n\t{}<p><input id=\"upgrade-plugins\" class=\"button\" type=\"submit\" value=\"{}\" name=\"upgrade\" /></p>\n<table class=\"widefat updates-table\" id=\"update-plugins-table\">\n\t<thead>\n\t<tr>\n\t\t<td class=\"manage-column check-column\"><input type=\"checkbox\" id=\"plugins-select-all\" /></td>\n\t\t<td class=\"manage-column\"><label for=\"plugins-select-all\">{}</label></td>\n\t</tr>\n\t</thead>\n\n\t<tbody class=\"plugins\">\n{rows}\t</tbody>\n\n\t<tfoot>\n\t<tr>\n\t\t<td class=\"manage-column check-column\"><input type=\"checkbox\" id=\"plugins-select-all-2\" /></td>\n\t\t<td class=\"manage-column\"><label for=\"plugins-select-all-2\">{}</label></td>\n\t</tr>\n\t</tfoot>\n</table>\n<p><input id=\"upgrade-plugins-2\" class=\"button\" type=\"submit\" value=\"{}\" name=\"upgrade\" /></p>\n</form>\n",
        tr("Plugins"),
        entries.len(),
        tr("The following plugins have new versions available. Check the ones you want to update and then click &#8220;Update Plugins&#8221;."),
        nonce_fields(ctx),
        esc_attr(&tr("Update Plugins")),
        tr("Select All"),
        tr("Select All"),
        esc_attr(&tr("Update Plugins"))
    )
}

/// `list_theme_updates()`.
async fn theme_section(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let updates = crate::updates::theme_updates(ctx.site()).await;
    let installed = crate::themes::installed(&wp_root(ctx));
    let mut entries: Vec<(&crate::themes::ThemeInfo, &crate::updates::ThemeUpdate)> = installed.iter().filter_map(|t| updates.get(&t.stylesheet).map(|u| (t, u))).collect();
    entries.sort_by_key(|(t, _)| t.name.to_lowercase());
    if entries.is_empty() {
        return format!("<h2>{}</h2><p>{}</p>", tr("Themes"), tr("Your themes are all up to date."));
    }
    let siteurl = ctx.site().settings.siteurl.trim_end_matches('/').to_owned();
    let mut rows = String::new();
    for (theme, update) in &entries {
        let checkbox_id = format!("checkbox_{}", crate::deps::md5_hex(&theme.name));
        let screenshot = theme.screenshot.as_ref().map(|file| format!("\t\t\t<img src=\"{siteurl}/wp-content/themes/{}/{file}?ver={}\" width=\"85\" height=\"64\" class=\"updates-table-screenshot\" alt=\"\" />\n", theme.stylesheet, theme.version)).unwrap_or_default();
        rows.push_str(&format!(
            "\t\t<tr>\n\t\t<td class=\"check-column\">\n\t\t\t\t\t\t\t<input type=\"checkbox\" name=\"checked[]\" id=\"{checkbox_id}\" value=\"{}\" />\n\t\t\t\t<label for=\"{checkbox_id}\">\n\t\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t</label>\n\t\t\t\t\t</td>\n\t\t<td class=\"plugin-title\"><p>\n{screenshot}\t\t\t<strong>{}</strong>\n\t\t\t{} \t\t</p></td>\n\t</tr>\n\t\t\t",
            esc_attr(&theme.stylesheet),
            sprintf(&tr("Select %s"), &[&theme.name]),
            esc_html(&theme.name),
            sprintf(&tr("You have version %1$s installed. Update to %2$s."), &[&theme.version, &update.new_version])
        ));
    }
    format!(
        "<h2>\n\t{} <span class=\"count\">({})</span></h2>\n<p>{}</p>\n<p>\n\t{}</p>\n<form method=\"post\" action=\"update-core.php?action=do-theme-upgrade\" name=\"upgrade-themes\" class=\"upgrade\">\n\t{}<p><input id=\"upgrade-themes\" class=\"button\" type=\"submit\" value=\"{}\" name=\"upgrade\" /></p>\n<table class=\"widefat updates-table\" id=\"update-themes-table\">\n\t<thead>\n\t<tr>\n\t\t<td class=\"manage-column check-column\"><input type=\"checkbox\" id=\"themes-select-all\" /></td>\n\t\t<td class=\"manage-column\"><label for=\"themes-select-all\">{}</label></td>\n\t</tr>\n\t</thead>\n\n\t<tbody class=\"plugins\">\n{rows}\t</tbody>\n\n\t<tfoot>\n\t<tr>\n\t\t<td class=\"manage-column check-column\"><input type=\"checkbox\" id=\"themes-select-all-2\" /></td>\n\t\t<td class=\"manage-column\"><label for=\"themes-select-all-2\">{}</label></td>\n\t</tr>\n\t</tfoot>\n</table>\n<p><input id=\"upgrade-themes-2\" class=\"button\" type=\"submit\" value=\"{}\" name=\"upgrade\" /></p>\n</form>\n",
        tr("Themes"),
        entries.len(),
        tr("The following themes have new versions available. Check the ones you want to update and then click &#8220;Update Themes&#8221;."),
        sprintf(&tr("<strong>Please Note:</strong> Any customizations you have made to theme files will be lost. Please consider using <a href=\"%s\">child themes</a> for modifications."), &[&tr("https://developer.wordpress.org/themes/advanced-topics/child-themes/")]),
        nonce_fields(ctx),
        esc_attr(&tr("Update Themes")),
        tr("Select All"),
        tr("Select All"),
        esc_attr(&tr("Update Themes"))
    )
}

/// `list_translation_updates()`.
fn translation_section(ctx: &Ctx<'_>, any: bool) -> String {
    if !any {
        return String::new();
    }
    let tr = |s: &str| ctx.tr(s);
    format!(
        "\t\t<h2>{}</h2>\n\t<form method=\"post\" action=\"update-core.php?action=do-translation-upgrade\" name=\"upgrade-translations\" class=\"upgrade\">\n\t\t<p>{}</p>\n\t\t{}\t\t<p><input class=\"button\" type=\"submit\" value=\"{}\" name=\"upgrade\" /></p>\n\t</form>\n",
        tr("Translations"),
        tr("New translations are available."),
        nonce_fields(ctx),
        esc_attr(&tr("Update Translations"))
    )
}

/// `date_i18n( 'g:i a T' )`: a site with a UTC offset instead of a timezone shows `GMT+0200`.
fn time_with_zone(ctx: &Ctx<'_>, timestamp: i64) -> String {
    let site = ctx.site();
    let format = tr_format(ctx);
    if site.options.string("timezone_string", "").is_empty() {
        let offset = site.options.string("gmt_offset", "0").parse::<f64>().unwrap_or(0.0);
        let zone = format!("GMT{}", rustpress_core::datetime::offset_string((offset * 3600.0) as i32, false));
        return format!("{} {zone}", site.date(&format.replace(" T", ""), timestamp));
    }
    site.date(&format, timestamp)
}

fn tr_format(ctx: &Ctx<'_>) -> String {
    ctx.tr("g:i a T")
}

async fn has_translations(ctx: &Ctx<'_>, name: &str) -> bool {
    transient(ctx, name).await.is_some_and(|t| matches!(field(&t, "translations"), Some(Value::Array(items)) if !items.is_empty()))
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !(ctx.can("update_core") || ctx.can("update_themes") || ctx.can("update_plugins") || ctx.can("update_languages")) {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to update this site.")));
    }
    let site = ctx.site();
    let version = ctx.env.loader.version.clone();
    let core = transient(ctx, "update_core").await;
    let last_checked = core.as_ref().and_then(|c| field(c, "last_checked")).and_then(Value::as_i64).unwrap_or(0);
    let last = if last_checked > 0 {
        let link = format!("<a href=\"{}\">{}</a>", esc_url(&ctx.admin_url("update-core.php?force-check=1")), tr("Check again."));
        format!("<p class=\"update-last-checked\">{} {link}</p>", sprintf(&tr("Last checked on %1$s at %2$s."), &[&site.date(&tr("F j, Y"), last_checked), &time_with_zone(ctx, last_checked)]))
    } else {
        String::new()
    };
    let translations = has_translations(ctx, "update_core").await || has_translations(ctx, "update_plugins").await || has_translations(ctx, "update_themes").await;
    let content = format!(
        "<div class=\"wrap\">\n\t<h1>{}</h1>\n\t<p>{}</p>\n\n\t<h2 class=\"wp-current-version\">{}</h2>{last}\n{}\n\t{}{}{}{}\n\t</div>\n",
        tr("WordPress Updates"),
        tr("Updates may take several minutes to complete. If there is no feedback after 5 minutes, or if there are errors please refer to the Help section above."),
        sprintf(&tr("Current version: %s"), &[&version]),
        auto_update_status(ctx),
        core_section(ctx, &version).await,
        plugin_section(ctx, &version).await,
        theme_section(ctx).await,
        translation_section(ctx, translations)
    );
    let screen = Screen { file: "update-core.php".into(), parent_file: "index.php".into(), submenu_file: Some("update-core.php".into()), title: tr("WordPress Updates"), help: Some(super::pages::load(ctx, "update-core").help), ..Default::default() };
    let counts = crate::updates::counts(site).await;
    let totals = serde_json::json!({"counts": {"plugins": counts.plugins, "themes": counts.themes, "wordpress": counts.wordpress, "translations": counts.translations, "total": counts.total()}, "title": ""});
    let assets = Assets {
        scripts: vec!["plugin-install".into(), "updates".into(), "thickbox".into()],
        styles: vec!["plugin-install".into(), "thickbox".into()],
        localize: vec![("updates".into(), "_wpUpdatesItemCounts".into(), serde_json::json!({"totals": totals}))],
        no_auth_check: true,
        ..Default::default()
    };
    render(ctx, &screen, &content, &assets).await
}
