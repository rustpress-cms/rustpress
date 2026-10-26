//! `wp-admin/themes.php`: the installed themes and switching between them.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_html, esc_url, wptexturize};
use serde_json::{Map, Value, json};

use super::notices::notice;
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};
use crate::themes::{self, ThemeInfo};

const CARD: &str = include_str!("../data/theme_card.tpl");
const CARD_ACTIVE: &str = include_str!("../data/theme_card_active.tpl");

fn wp_root(ctx: &Ctx<'_>) -> Option<std::path::PathBuf> {
    ctx.env.loader.includes.parent().map(std::path::Path::to_path_buf)
}

/// The tags with the labels WordPress gives the known ones.
fn tags_text(theme: &ThemeInfo) -> String {
    theme.tags.iter().map(|t| themes::feature_label(t).map_or_else(|| t.clone(), str::to_owned)).collect::<Vec<_>>().join(", ")
}

/// The theme the way `wp_prepare_themes_for_js()` describes it.
fn theme_json(ctx: &Ctx<'_>, theme: &ThemeInfo, active: &str, parent_name: Option<String>, update: Option<&crate::updates::ThemeUpdate>) -> Value {
    let site = ctx.site();
    let siteurl = site.settings.siteurl.trim_end_matches('/');
    let is_active = theme.stylesheet == active;
    let author_and_uri = if theme.author_uri.is_empty() { wptexturize(&theme.author) } else { format!("<a href=\"{}\">{}</a>", esc_url(&theme.author_uri), wptexturize(&theme.author)) };
    let nonce_url = |action: &str, nonce_action: &str| {
        let url = ctx.admin_url(&format!("themes.php?action={action}&stylesheet={}", theme.stylesheet));
        esc_url(&format!("{url}&_wpnonce={}", ctx.create_nonce(nonce_action))).replace("&#038;", "&amp;")
    };
    let mut actions = Map::new();
    actions.insert("activate".into(), json!(nonce_url("activate", &format!("switch-theme_{}", theme.stylesheet))));
    actions.insert(
        "customize".into(),
        json!(if theme.block_theme {
            let back = rustpress_core::canonical::rawurlencode(&ctx.request.request_uri);
            let path = if is_active { format!("site-editor.php?return={back}") } else { format!("site-editor.php?wp_theme_preview={}&return={back}", theme.stylesheet) };
            esc_url(&ctx.admin_url(&path))
        } else {
            String::new()
        }),
    );
    actions.insert("delete".into(), json!(nonce_url("delete", &format!("delete-theme_{}", theme.stylesheet))));
    actions.insert("autoupdate".into(), json!(nonce_url("enable-auto-update", "updates")));
    let (has_update, has_package, update_html) = match update {
        Some(u) => {
            let tr = |s: &str| ctx.tr(s);
            let name = esc_html(&theme.name);
            let details = format!("{}?TB_iframe=true&#038;width=1024&#038;height=800", u.url);
            let html = format!(
                "<p><strong>{}</strong></p>",
                rustpress_core::i18n::sprintf(
                    &tr("There is a new version of %1$s available. <a href=\"%2$s\" %3$s>View version %4$s details</a> or <a href=\"%5$s\" %6$s>update now</a>."),
                    &[
                        &name,
                        &details,
                        &format!("class=\"thickbox open-plugin-details-modal\" aria-label=\"{}\"", rustpress_core::formatting::esc_attr(&rustpress_core::i18n::sprintf(&tr("View %1$s version %2$s details"), &[&theme.name, &u.new_version]))),
                        &u.new_version,
                        &nonce_url("upgrade-theme", "upgrade-theme_x").replace("themes.php?action=upgrade-theme&amp;stylesheet=", "update.php?action=upgrade-theme&amp;theme=").replace(&ctx.create_nonce("upgrade-theme_x"), &ctx.create_nonce(&format!("upgrade-theme_{}", theme.stylesheet))),
                        &format!("aria-label=\"{}\" id=\"update-theme\" data-slug=\"{}\"", rustpress_core::formatting::esc_attr(&rustpress_core::i18n::sprintf(&tr("Update %s now"), &[&theme.name])), theme.stylesheet),
                    ]
                )
            );
            (true, !u.package.is_empty(), html)
        }
        None => (false, false, String::new()),
    };
    let screenshot = theme.screenshot.as_ref().map(|f| json!(format!("{siteurl}/wp-content/themes/{}/{f}", theme.stylesheet))).into_iter().collect::<Vec<_>>();
    let mut out = Map::new();
    out.insert("id".into(), json!(theme.stylesheet));
    out.insert("name".into(), json!(wptexturize(&theme.name)));
    out.insert("screenshot".into(), Value::Array(screenshot));
    out.insert("description".into(), json!(wptexturize(&theme.description)));
    out.insert("author".into(), json!(wptexturize(&theme.author)));
    out.insert("authorAndUri".into(), json!(author_and_uri));
    out.insert("tags".into(), json!(tags_text(theme)));
    out.insert("version".into(), json!(theme.version));
    out.insert("compatibleWP".into(), json!(true));
    out.insert("compatiblePHP".into(), json!(true));
    out.insert("updateResponse".into(), json!({"compatibleWP": true, "compatiblePHP": true}));
    out.insert("parent".into(), parent_name.map_or(json!(false), Value::from));
    out.insert("active".into(), json!(is_active));
    out.insert("hasUpdate".into(), json!(has_update));
    out.insert("hasPackage".into(), json!(has_package));
    out.insert("update".into(), json!(update_html));
    out.insert("autoupdate".into(), json!({"enabled": false, "supported": true, "forced": null}));
    out.insert("actions".into(), Value::Object(actions));
    out.insert("blockTheme".into(), json!(theme.block_theme));
    Value::Object(out)
}

/// One card of the server-side list.
fn card(ctx: &Ctx<'_>, theme: &ThemeInfo, active: bool, update: bool) -> String {
    let template = if active { CARD_ACTIVE } else { CARD };
    let site = ctx.site().settings.siteurl.trim_end_matches('/').to_owned();
    let nonce = ctx.create_nonce(&format!("switch-theme_{}", theme.stylesheet));
    let notice = if update { format!("\t<div class=\"notice notice-warning notice-alt inline update-message\"><p>{} <button class=\"button-link\" type=\"button\">{}</button></p></div>\t", ctx.tr("New version available."), ctx.tr("Update now")) } else { String::new() };
    let mut html = template.replace("\r\n", "\n").replace("{{return}}", &rustpress_core::canonical::rawurlencode(&ctx.request.request_uri));
    for (needle, value) in [("{{id}}", theme.stylesheet.as_str()), ("{{name}}", &esc_html(&wptexturize(&theme.name))), ("{{version}}", &theme.version), ("{{author}}", &wptexturize(&theme.author)), ("{{nonce}}", &nonce), ("{{update_notice}}", &notice), ("{{site}}", &site)] {
        html = html.replace(needle, value);
    }
    // The few texts of the card.
    for text in ["Theme Details", "Customize", "Activate", "Live Preview", "Active:"] {
        html = html.replace(&format!(">{text}<"), &format!(">{}<", ctx.tr(text)));
    }
    html = html.replace("By ", &format!("{} ", ctx.tr("By %s").replace("%s", "").trim()));
    html
}

/// `switch_theme()`.
async fn switch(ctx: &Ctx<'_>, theme: &ThemeInfo) -> bool {
    let Some(writer) = ctx.env.writer else { return false };
    let site = ctx.site();
    let old = site.settings.stylesheet.clone();
    writer.update_option("template", &theme.template).await.or_log();
    writer.update_option("stylesheet", &theme.stylesheet).await.or_log();
    writer.update_option("current_theme", &theme.name).await.or_log();
    // The mods of the new theme exist (empty) from now on.
    let mods = format!("theme_mods_{}", theme.stylesheet);
    if ctx.env.db.option(&mods).await.ok().flatten().is_none() {
        writer.update_option_with_autoload(&mods, "a:0:{}", "yes").await.or_log();
    }
    let old_mods = format!("theme_mods_{old}");
    if let Ok(Some(value)) = ctx.env.db.option(&old_mods).await {
        writer.update_option_with_autoload(&old_mods, &value, "no").await.or_log();
    }
    writer.update_option("theme_switched", &old).await.or_log();
    true
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    if !ctx.can("switch_themes") && !ctx.can("edit_theme_options") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to edit theme options on this site.")));
    }
    let Some(root) = wp_root(ctx) else { return die(ctx.env, 500, "WordPress &rsaquo; Error", "<p>The WordPress files are not available.</p>") };
    let stylesheet = request.get("stylesheet").unwrap_or_default().to_owned();
    if ctx.can("switch_themes") {
        match request.get("action") {
            Some("activate") => {
                if !ctx.verify_nonce(&format!("switch-theme_{stylesheet}")) {
                    return die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
                }
                let Some(theme) = themes::find(&root, &stylesheet) else {
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("An error occurred."), tr("The requested theme does not exist.")));
                };
                if !switch(ctx, &theme).await {
                    return super::common::read_only(ctx);
                }
                return redirect(&ctx.admin_url("themes.php?activated=true"));
            }
            Some("delete") => {
                if !ctx.verify_nonce(&format!("delete-theme_{stylesheet}")) {
                    return die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
                }
                if !ctx.can("delete_themes") {
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to delete this item.")));
                }
                let Some(_theme) = themes::find(&root, &stylesheet) else {
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("An error occurred while deleting the theme."), tr("The requested theme does not exist.")));
                };
                if ctx.site().settings.template == stylesheet {
                    return redirect(&ctx.admin_url("themes.php?delete-active-child=true"));
                }
                if ctx.site().settings.stylesheet == stylesheet {
                    // WordPress only hides the link; deleting the theme in use would leave the site without one.
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("You cannot delete the active theme.")));
                }
                if ctx.env.writer.is_none() {
                    return super::common::read_only(ctx);
                }
                let _ = std::fs::remove_dir_all(root.join("wp-content").join("themes").join(&stylesheet));
                return redirect(&ctx.admin_url("themes.php?deleted=true"));
            }
            Some(action @ ("enable-auto-update" | "disable-auto-update")) => {
                if !ctx.can("update_themes") {
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", if action == "enable-auto-update" { tr("Sorry, you are not allowed to enable themes automatic updates.") } else { tr("Sorry, you are not allowed to disable themes automatic updates.") }));
                }
                if !ctx.verify_nonce("updates") {
                    return die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
                }
                let Some(writer) = ctx.env.writer else { return super::common::read_only(ctx) };
                let mut auto: Vec<String> = match ctx.site().options.value("auto_update_themes") {
                    Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
                    _ => Vec::new(),
                };
                if action == "enable-auto-update" {
                    if !auto.contains(&stylesheet) {
                        auto.push(stylesheet.clone());
                    }
                } else {
                    auto.retain(|t| t != &stylesheet);
                }
                let known = themes::installed(&root);
                auto.retain(|t| known.iter().any(|k| &k.stylesheet == t));
                let items = auto.iter().enumerate().map(|(i, t)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Str(t.clone().into_bytes()))).collect();
                writer.update_option("auto_update_themes", &String::from_utf8_lossy(&rustpress_phpser::serialize(&rustpress_phpser::Value::Array(items)))).await.or_log();
                return redirect(&ctx.admin_url(&format!("themes.php?{}=true", if action == "enable-auto-update" { "enabled-auto-update" } else { "disabled-auto-update" })));
            }
            _ => {}
        }
    }

    let site = ctx.site();
    let installed = themes::installed(&root);
    let active = site.settings.stylesheet.clone();
    let updates = crate::updates::theme_updates(site).await;
    // The active theme comes first, the others by name.
    let mut ordered: Vec<&ThemeInfo> = installed.iter().collect();
    ordered.sort_by_key(|t| (t.stylesheet != active, t.name.to_lowercase()));
    let name_of = |stylesheet: &str| installed.iter().find(|t| t.stylesheet == stylesheet).map(|t| t.name.clone());
    let theme_list: Vec<Value> = installed
        .iter()
        .map(|t| {
            let parent = if t.has_parent() { name_of(&t.template) } else { None };
            theme_json(ctx, t, &active, parent, updates.get(&t.stylesheet))
        })
        .collect();
    // The active theme leads the list of the script, too.
    let mut theme_list = theme_list;
    theme_list.sort_by_key(|t| t["id"].as_str() != Some(active.as_str()));

    let mut cards = String::new();
    for theme in &ordered {
        cards.push_str(&card(ctx, theme, theme.stylesheet == active, updates.contains_key(&theme.stylesheet)));
    }
    let mut notices = String::new();
    let home = esc_url(&site.home_url("/"));
    let flag = |name: &str| request.get(name).is_some();
    if flag("broken") {
        notices.push_str(&notice(&tr("The active theme is broken. Reverting to the default theme."), "", true, "message1", &["updated"], true));
    } else if flag("activated") {
        let message = if flag("previewed") { tr("Settings saved and theme activated.") } else { tr("New theme activated.") };
        notices.push_str(&notice(&format!("{message} <a href=\"{home}\">{}</a>", tr("Visit site")), "", true, "message2", &["updated"], true));
    } else if flag("deleted") {
        notices.push_str(&notice(&tr("Theme deleted."), "", true, "message3", &["updated"], true));
    } else if flag("delete-active-child") {
        notices.push_str(&notice(&tr("You cannot delete a theme while it has an active child theme."), "", false, "message4", &["error"], true));
    } else if flag("enabled-auto-update") {
        notices.push_str(&notice(&tr("Theme will be auto-updated."), "", true, "message7", &["updated"], true));
    } else if flag("disabled-auto-update") {
        notices.push_str(&notice(&tr("Theme will no longer be auto-updated."), "", true, "message8", &["updated"], true));
    }

    let baked = super::pages::load(ctx, "themes");
    let searching = request.get("search").is_some_and(|s| !s.is_empty());
    let count = if searching { ctx.tr("&hellip;") } else { installed.len().to_string() };
    let mut content = baked.content.replace("{{cards}}", &cards).replace("{{count}}", &count).replace("{{notices}}", &notices);
    if searching {
        content = content.replacen("class=\"theme-browser\"", "class=\"theme-browser search-loading\"", 1);
    }
    let settings = json!({
        "themes": theme_list,
        "settings": {
            "canInstall": ctx.can("install_themes"),
            "installURI": ctx.admin_url("theme-install.php"),
            "confirmDelete": tr("Are you sure you want to delete this theme?\n\nClick 'Cancel' to go back, 'OK' to confirm the delete."),
            "adminUrl": "/wp-admin/",
        },
        "l10n": {
            "addNew": tr("Add Theme"),
            "search": tr("Search installed themes"),
            "themesFound": tr("Number of Themes found: %d"),
            "noThemesFound": tr("No themes found. Try a different search."),
            "themeViewed": tr("Theme details: %s"),
        },
    });
    let assets = Assets { scripts: vec!["thickbox".into(), "theme".into(), "updates".into()], styles: vec!["thickbox".into()], localize: vec![("theme".into(), "_wpThemeSettings".into(), settings)], ..Default::default() };
    let screen = Screen { file: "themes.php".into(), parent_file: "themes.php".into(), submenu_file: Some("themes.php".into()), title: tr("Themes"), help: Some(baked.help), ..Default::default() };
    render(ctx, &screen, &content, &assets).await
}
