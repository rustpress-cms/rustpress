//! `wp-admin/plugins.php`: the installed plugins. RustPress runs no PHP plugins; the screen lists them and
//! records which ones are switched on in `active_plugins`, the way WordPress would.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use super::notices::notice;
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};
use crate::deps::md5_hex;
use crate::plugins::{self, PluginInfo};

/// The Bytes of a query value (`rawurlencode()`).
fn urlenc(text: &str) -> String {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

fn active_plugins(ctx: &Ctx<'_>) -> Vec<String> {
    match ctx.site().options.value("active_plugins") {
        Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

fn auto_updates(ctx: &Ctx<'_>) -> Vec<String> {
    match ctx.site().options.value("auto_update_plugins") {
        Some(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

/// `get_option( 'recently_activated' )` (not autoloaded): plugin file => time of the deactivation.
async fn recently_activated(ctx: &Ctx<'_>) -> Vec<(String, i64)> {
    let raw = ctx.site().options.fetch(ctx.env.db, "recently_activated").await.ok().flatten().unwrap_or_default();
    match rustpress_phpser::unserialize(raw.as_bytes()).ok() {
        Some(rustpress_phpser::Value::Array(items)) => items
            .iter()
            .filter_map(|(k, v)| match k {
                rustpress_phpser::Key::Str(file) => Some((String::from_utf8_lossy(file).into_owned(), v.as_i64().unwrap_or(0))),
                rustpress_phpser::Key::Int(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `update_option( 'recently_activated', $recent, false )`.
async fn save_recently_activated(ctx: &Ctx<'_>, recent: &[(String, i64)]) {
    let Some(writer) = ctx.env.writer else { return };
    let value = rustpress_phpser::Value::Array(recent.iter().map(|(file, time)| (rustpress_phpser::Key::Str(file.as_bytes().to_vec()), rustpress_phpser::Value::Int(*time))).collect());
    writer.update_option_with_autoload("recently_activated", &String::from_utf8_lossy(&rustpress_phpser::serialize(&value)), "off").await.or_log();
}

/// `plugins.php` after (de)activating: deactivated plugins go to the front of `recently_activated`, activated ones
/// leave it.
async fn note_recently_activated(ctx: &Ctx<'_>, deactivated: &[String], activated: &[String]) {
    let mut recent: Vec<(String, i64)> = deactivated.iter().map(|file| (file.clone(), ctx.env.now)).collect();
    recent.extend(recently_activated(ctx).await.into_iter().filter(|(file, _)| !deactivated.contains(file) && !activated.contains(file)));
    save_recently_activated(ctx, &recent).await;
}

/// `serialize( array( ... ) )` of a list of strings.
fn serialize_list(items: &[String]) -> String {
    let value = rustpress_phpser::Value::Array(items.iter().enumerate().map(|(i, s)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Str(s.as_bytes().to_vec()))).collect());
    String::from_utf8_lossy(&rustpress_phpser::serialize(&value)).into_owned()
}

/// `is_uninstallable_plugin()`: the plugin has an `uninstall.php` or registered an uninstall hook.
fn is_uninstallable(ctx: &Ctx<'_>, root: &std::path::Path, file: &str) -> bool {
    let registered = ctx.site().options.value("uninstall_plugins").is_some_and(|v| v.get(file).is_some());
    let dir = file.rsplit_once('/').map_or(".", |(dir, _)| dir);
    registered || rustpress_core::files::is_file(&root.join("wp-content").join("plugins").join(dir).join("uninstall.php"))
}

/// What a row knows of the plugin besides its headers.
struct RowInfo<'a> {
    active: bool,
    auto_update: bool,
    status: &'a str,
    /// The slug WordPress.org knows the plugin by (from the transient `update_plugins`).
    api_slug: Option<&'a str>,
    uninstallable: bool,
}

/// One row of the list.
fn row(ctx: &Ctx<'_>, plugin: &PluginInfo, info: &RowInfo<'_>) -> String {
    let (active, auto_update, status) = (info.active, info.auto_update, info.status);
    // The search of the list is part of the links (`add_query_arg()` writes an empty `s` without an equals sign).
    let s_arg = match ctx.request.request("s").map(str::trim).filter(|s| !s.is_empty()) {
        Some(search) => format!("s={}", urlenc(search)),
        None => "s".to_owned(),
    };
    let tr = |s: &str| ctx.tr(s);
    let name = esc_html(&plugin.name);
    // `$plugin_data['slug'] ?? sanitize_title( $plugin_data['Name'] )`.
    let slug = info.api_slug.map_or_else(|| rustpress_core::slug::sanitize_title(&plugin.name), str::to_owned);
    let state = if active { "active" } else { "inactive" };
    let class = if info.uninstallable { format!("{state} is-uninstallable") } else { state.to_owned() };
    let file_q = urlenc(&plugin.file);
    let checkbox = format!("checkbox_{}", md5_hex(&plugin.file));
    let activate_nonce = ctx.create_nonce(&format!("activate-plugin_{}", plugin.file));
    let deactivate_nonce = ctx.create_nonce(&format!("deactivate-plugin_{}", plugin.file));
    let bulk_nonce = ctx.create_nonce("bulk-plugins");
    let updates_nonce = ctx.create_nonce("updates");
    let mut links: Vec<(String, String)> = Vec::new();
    if active {
        links.push(("deactivate".into(), format!("<a href=\"plugins.php?action=deactivate&amp;plugin={file_q}&amp;plugin_status={status}&amp;paged=1&amp;{s_arg}&amp;_wpnonce={deactivate_nonce}\" id=\"deactivate-{slug}\" aria-label=\"{}\">{}</a>", esc_attr(&rustpress_core::i18n::sprintf(&tr("Deactivate %s"), &[&plugin.name])), tr("Deactivate"))));
        // `plugin_action_links_{$plugin_file}` of the plugin itself: its links come first, keyed by number.
        let own = ctx.site().plugins.plugin_action_links(ctx.site(), &plugin.file, Vec::new());
        links.splice(0..0, own.into_iter().enumerate().map(|(i, link)| (i.to_string(), link)));
    } else {
        links.push(("activate".into(), format!("<a href=\"plugins.php?action=activate&amp;plugin={file_q}&amp;plugin_status={status}&amp;paged=1&amp;{s_arg}&amp;_wpnonce={activate_nonce}\" id=\"activate-{slug}\" class=\"edit\" aria-label=\"{}\">{}</a>", esc_attr(&rustpress_core::i18n::sprintf(&tr("Activate %s"), &[&plugin.name])), tr("Activate"))));
        links.push(("delete".into(), format!("<a href=\"plugins.php?action=delete-selected&amp;checked%5B0%5D={file_q}&amp;plugin_status={status}&amp;paged=1&amp;{s_arg}&amp;_wpnonce={bulk_nonce}\" id=\"delete-{slug}\" class=\"delete\" aria-label=\"{}\">{}</a>", esc_attr(&rustpress_core::i18n::sprintf(&tr("Delete %s"), &[&plugin.name])), tr("Delete"))));
    }
    // `WP_List_Table::row_actions()`.
    let count = links.len();
    let actions: String = links.iter().enumerate().map(|(i, (key, link))| format!("<span class='{key}'>{link}{}</span>", if i + 1 < count { " | " } else { "" })).collect();
    let description = if plugin.description.is_empty() { "&nbsp;".to_owned() } else { plugins::kses_description(&plugin.description) };
    let mut meta = Vec::new();
    if !plugin.version.is_empty() {
        meta.push(rustpress_core::i18n::sprintf(&tr("Version %s"), &[&plugin.version]));
    }
    if !plugin.author.is_empty() {
        let author = if plugin.author_uri.is_empty() { esc_html(&plugin.author) } else { format!("<a href=\"{}\">{}</a>", esc_url(&plugin.author_uri), esc_html(&plugin.author)) };
        meta.push(rustpress_core::i18n::sprintf(&tr("By %s"), &[&author]));
    }
    let site = ctx.site().settings.siteurl.trim_end_matches('/').to_owned();
    // The details from WordPress.org for a plugin it knows, else the plugin's own site.
    if let Some(api_slug) = info.api_slug.filter(|_| ctx.can("install_plugins")) {
        meta.push(format!("<a href=\"{site}/wp-admin/plugin-install.php?tab=plugin-information&#038;plugin={api_slug}&#038;TB_iframe=true&#038;width=600&#038;height=550\" class=\"thickbox open-plugin-details-modal\" aria-label=\"{}\" data-title=\"{name}\">{}</a>", esc_attr(&rustpress_core::i18n::sprintf(&tr("More information about %s"), &[&plugin.name])), tr("View details")));
    } else if !plugin.plugin_uri.is_empty() {
        meta.push(format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&plugin.plugin_uri), esc_attr(&rustpress_core::i18n::sprintf(&tr("Visit plugin site for %s"), &[&plugin.name])), tr("Visit plugin site")));
    }
    // `plugin_row_meta` of the active plugins.
    let meta = ctx.site().plugins.plugin_row_meta(ctx.site(), &plugin.file, meta);
    // A plugin WordPress.org does not know cannot be updated automatically.
    let auto = if info.api_slug.is_none() {
        "<span class=\"label\"></span><div class=\"notice notice-error notice-alt inline hidden\"><p></p></div>".to_owned()
    } else if auto_update {
        format!("<a href=\"plugins.php?action=disable-auto-update&amp;plugin={file_q}&amp;paged=1&amp;plugin_status={status}&amp;_wpnonce={updates_nonce}\" class=\"toggle-auto-update aria-button-if-js\" data-wp-action=\"disable\"><span class=\"dashicons dashicons-update spin hidden\" aria-hidden=\"true\"></span><span class=\"label\">{}</span></a><div class=\"notice notice-error notice-alt inline hidden\"><p></p></div>", tr("Disable auto-updates"))
    } else {
        format!("<a href=\"plugins.php?action=enable-auto-update&amp;plugin={file_q}&amp;paged=1&amp;plugin_status={status}&amp;_wpnonce={updates_nonce}\" class=\"toggle-auto-update aria-button-if-js\" data-wp-action=\"enable\"><span class=\"dashicons dashicons-update spin hidden\" aria-hidden=\"true\"></span><span class=\"label\">{}</span></a><div class=\"notice notice-error notice-alt inline hidden\"><p></p></div>", tr("Enable auto-updates"))
    };
    format!(
        "<tr class=\"{class}\" data-slug=\"{slug}\" data-plugin=\"{}\"><td class='check-column'><label class=\"label-covers-full-cell\" for=\"{checkbox}\"><span class=\"screen-reader-text\">{}</span></label><input type=\"checkbox\" name=\"checked[]\" value=\"{}\" id=\"{checkbox}\" /></td><th scope='row' class='plugin-title column-primary' aria-label='{}'><strong>{name}</strong><div class=\"row-actions visible\">{actions}</div><button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button></th><td class='column-description desc'>\n\t\t\t\t\t\t<div class='plugin-description'><p>{description}</p></div>\n\t\t\t\t\t\t<div class='{class} second plugin-version-author-uri'>{}</div></td><td class='column-auto-updates'>{auto}</td></tr>",
        esc_attr(&plugin.file),
        rustpress_core::i18n::sprintf(&tr("Select %s"), &[&plugin.name]),
        esc_attr(&plugin.file),
        esc_attr(&plugin.name),
        tr("Show more details"),
        meta.join(" | ")
    )
}

/// The must-use rows (no actions).
fn mu_row(ctx: &Ctx<'_>, plugin: &PluginInfo) -> String {
    let tr = |s: &str| ctx.tr(s);
    let name = esc_html(&plugin.name);
    let description = if plugin.description.is_empty() { "&nbsp;".to_owned() } else { plugins::kses_description(&plugin.description) };
    let mut meta = Vec::new();
    if !plugin.version.is_empty() {
        meta.push(rustpress_core::i18n::sprintf(&tr("Version %s"), &[&plugin.version]));
    }
    if !plugin.author.is_empty() {
        let author = if plugin.author_uri.is_empty() { esc_html(&plugin.author) } else { format!("<a href=\"{}\">{}</a>", esc_url(&plugin.author_uri), esc_html(&plugin.author)) };
        meta.push(rustpress_core::i18n::sprintf(&tr("By %s"), &[&author]));
    }
    format!(
        "<tr class=\"active\" data-slug=\"{}\" data-plugin=\"{}\"><td class='check-column'></td><th scope='row' class='plugin-title column-primary' aria-label='{}'><strong>{name}</strong></th><td class='column-description desc'>\n\t\t\t\t\t\t<div class='plugin-description'><p>{description}</p></div>\n\t\t\t\t\t\t<div class='active second plugin-version-author-uri'>{}</div></td></tr>",
        plugins_slug(plugin),
        esc_attr(&plugin.file),
        esc_attr(&plugin.name),
        meta.join(" | ")
    )
}

fn plugins_slug(plugin: &PluginInfo) -> String {
    plugin.slug()
}

/// `validate_file()`: no way out of the plugins directory.
fn valid_plugin_file(file: &str) -> bool {
    !file.is_empty() && !file.contains("..") && !file.contains("./") && !file.starts_with('/') && !file.contains('\\') && !file.contains(':')
}

/// `delete_plugins()`: a plugin in a folder takes the folder along, a single file goes alone.
fn delete_plugin_files(root: &std::path::Path, file: &str) -> std::io::Result<()> {
    let plugins = root.join("wp-content").join("plugins");
    match file.split_once('/') {
        Some((folder, _)) => std::fs::remove_dir_all(plugins.join(folder)),
        None => std::fs::remove_file(plugins.join(file)),
    }
}

/// The actions of `plugins.php` that change something: activate, deactivate, delete, auto-updates.
async fn actions(ctx: &Ctx<'_>, action: &str, installed: &[PluginInfo], active: &mut Vec<String>, root: &std::path::Path) -> Option<AdminResponse> {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let expired = || die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
    let status = request.request("plugin_status").unwrap_or("all").to_owned();
    let page = request.request("paged").unwrap_or("1").to_owned();
    let search = urlenc(request.request("s").unwrap_or_default());
    let back_url = |extra: &str| ctx.admin_url(&format!("plugins.php?{extra}plugin_status={status}&paged={page}&s={search}"));
    let back = |extra: &str| redirect(&back_url(extra));
    let checked: Vec<String> = request.form.iter().chain(request.query.iter()).filter(|(k, _)| k == "checked[]").map(|(_, v)| v.clone()).collect();
    let file = request.request("plugin").unwrap_or_default().to_owned();
    let is_installed = |f: &str| installed.iter().any(|p| p.file == f);
    match action {
        "activate" | "deactivate" => {
            if !ctx.verify_nonce(&format!("{action}-plugin_{file}")) {
                return Some(expired());
            }
            let Some(writer) = ctx.env.writer else { return Some(super::common::read_only(ctx)) };
            if action == "activate" && is_installed(&file) && !active.contains(&file) {
                active.push(file.clone());
            } else if action == "deactivate" {
                active.retain(|p| p != &file);
            }
            writer.update_option("active_plugins", &serialize_list(active)).await.or_log();
            if action == "activate" {
                note_recently_activated(ctx, &[], std::slice::from_ref(&file)).await;
            } else {
                note_recently_activated(ctx, std::slice::from_ref(&file), &[]).await;
            }
            Some(back(&format!("{}=true&", if action == "activate" { "activate" } else { "deactivate" })))
        }
        "activate-selected" | "deactivate-selected" => {
            if !ctx.can("activate_plugins") {
                return Some(die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", if action == "activate-selected" { tr("Sorry, you are not allowed to activate plugins for this site.") } else { tr("Sorry, you are not allowed to deactivate plugins for this site.") })));
            }
            if !ctx.verify_nonce("bulk-plugins") {
                return Some(expired());
            }
            let Some(writer) = ctx.env.writer else { return Some(super::common::read_only(ctx)) };
            let selected: Vec<String> = checked.iter().filter(|f| if action == "activate-selected" { !active.contains(f) && is_installed(f) } else { active.contains(f) }).cloned().collect();
            if selected.is_empty() {
                return Some(back(""));
            }
            if action == "activate-selected" {
                active.extend(selected.iter().cloned());
            } else {
                active.retain(|p| !selected.contains(p));
            }
            writer.update_option("active_plugins", &serialize_list(active)).await.or_log();
            if action == "activate-selected" {
                note_recently_activated(ctx, &[], &selected).await;
            } else {
                note_recently_activated(ctx, &selected, &[]).await;
            }
            Some(back(&format!("{}=true&", if action == "activate-selected" { "activate-multi" } else { "deactivate-multi" })))
        }
        "enable-auto-update" | "disable-auto-update" | "enable-auto-update-selected" | "disable-auto-update-selected" => {
            if !ctx.can("update_plugins") {
                return Some(die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to manage plugins automatic updates."))));
            }
            let single = !action.ends_with("-selected");
            if single && file.is_empty() || !single && checked.is_empty() {
                return Some(back(""));
            }
            if !ctx.verify_nonce(if single { "updates" } else { "bulk-plugins" }) {
                return Some(expired());
            }
            let Some(writer) = ctx.env.writer else { return Some(super::common::read_only(ctx)) };
            let mut auto = auto_updates(ctx);
            let before = auto.clone();
            let targets = if single { vec![file] } else { checked.clone() };
            if action.starts_with("enable") {
                for target in targets {
                    if !auto.contains(&target) {
                        auto.push(target);
                    }
                }
            } else {
                auto.retain(|p| !targets.contains(p));
            }
            if !single && auto == before {
                return Some(back(""));
            }
            auto.retain(|p| is_installed(p));
            writer.update_option("auto_update_plugins", &serialize_list(&auto)).await.or_log();
            let flag = match action {
                "enable-auto-update" => "enabled-auto-update",
                "disable-auto-update" => "disabled-auto-update",
                "enable-auto-update-selected" => "enabled-auto-update-multi",
                _ => "disabled-auto-update-multi",
            };
            // `add_query_arg()` writes an empty `s` without an equals sign.
            let base = back_url("");
            let base = base.strip_suffix("&s=").map_or_else(|| base.clone(), |b| format!("{b}&s"));
            Some(redirect(&format!("{base}&{flag}=true")))
        }
        "delete-selected" => {
            if !ctx.can("delete_plugins") {
                return Some(die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to delete plugins for this site."))));
            }
            if !ctx.verify_nonce("bulk-plugins") {
                return Some(expired());
            }
            if checked.is_empty() {
                return Some(back(""));
            }
            let plugins: Vec<String> = checked.iter().filter(|f| !active.contains(f)).cloned().collect();
            if plugins.is_empty() {
                return Some(back("error=true&main=true&"));
            }
            if plugins.iter().any(|f| !valid_plugin_file(f)) {
                return Some(back(""));
            }
            if request.request("verify-delete").is_none() {
                return Some(delete_confirmation(ctx, &plugins, root).await);
            }
            let count = plugins.len();
            if ctx.env.writer.is_none() {
                return Some(super::common::read_only(ctx));
            }
            for plugin in &plugins {
                if delete_plugin_files(root, plugin).is_err() {
                    return Some(die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Plugin could not be deleted."))));
                }
            }
            Some(back(&format!("deleted={count}&")))
        }
        _ => None,
    }
}

/// The question before `delete-selected` removes files.
async fn delete_confirmation(ctx: &Ctx<'_>, plugins: &[String], root: &std::path::Path) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let installed = plugins::installed(root);
    let mut items = String::new();
    for file in plugins {
        let folder = file.split_once('/').map(|(f, _)| f);
        for plugin in installed.iter().filter(|p| match folder {
            Some(folder) => p.file.starts_with(&format!("{folder}/")),
            None => &p.file == file,
        }) {
            let author = if plugin.author.is_empty() { String::new() } else { plugin.author.clone() };
            items.push_str(&format!("<li>{}</li>", rustpress_core::i18n::sprintf(&ctx.site().trx("%1$s by %2$s", "plugin"), &[&format!("<strong>{}</strong>", esc_html(&plugin.name)), &format!("<em>{}</em>", esc_html(&author))])));
        }
    }
    let many = plugins.len() != 1;
    let current = ctx.request.request_uri.clone();
    let hidden: String = plugins.iter().map(|p| format!("<input type=\"hidden\" name=\"checked[]\" value=\"{}\" />", esc_attr(p))).collect();
    let nonce = ctx.nonce_field("bulk-plugins");
    let content = format!(
        "<div class=\"wrap\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t<h1>{}</h1>\n\t\t\t\t\t\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t\t\t\t\t<ul class=\"ul-disc\">\n\t\t\t\t\t\t{items}\t\t\t\t\t</ul>\n\t\t\t\t<p>\n\t\t\t\t{}\t\t\t\t</p>\n\t\t\t\t<form method=\"post\" action=\"{}\" style=\"display:inline;\">\n\t\t\t\t\t<input type=\"hidden\" name=\"verify-delete\" value=\"1\" />\n\t\t\t\t\t<input type=\"hidden\" name=\"action\" value=\"delete-selected\" />\n\t\t\t\t\t{hidden}\t\t\t\t\t{nonce}\t\t\t\t\t<input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button\" value=\"{}\"  />\t\t\t\t</form>\n\t\t\t\t\t\t\t\t<form method=\"post\" action=\"\" style=\"display:inline;\">\n\t\t\t\t\t<input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button\" value=\"{}\"  />\t\t\t\t</form>\n\t\t\t\t</div>\n\t\t\t\t",
        if many { tr("Delete Plugins") } else { tr("Delete Plugin") },
        if many { tr("You are about to remove the following plugins:") } else { tr("You are about to remove the following plugin:") },
        tr("Are you sure you want to delete these files?"),
        esc_url(&current),
        esc_attr(&tr("Yes, delete these files")),
        esc_attr(&tr("No, return me to the plugin list"))
    );
    let screen = Screen { file: "plugins.php".into(), parent_file: "plugins.php".into(), submenu_file: Some("plugins.php".into()), title: tr("Plugins"), ..Default::default() };
    render(ctx, &screen, &content, &Assets { scripts: vec!["jquery".into()], ..Default::default() }).await
}

/// `activate_plugin()` for the Ajax action: the plugin goes into `active_plugins`.
pub(super) async fn activate_one(ctx: &Ctx<'_>, file: &str) -> Result<(), String> {
    let Some(root) = ctx.env.loader.includes.parent().map(std::path::Path::to_path_buf) else { return Err(ctx.tr("Plugin file does not exist.")) };
    if !plugins::installed(&root).iter().any(|p| p.file == file) {
        return Err(ctx.tr("Plugin file does not exist."));
    }
    let mut active = active_plugins(ctx);
    if !active.iter().any(|p| p == file) {
        active.push(file.to_owned());
        let Some(writer) = ctx.env.writer else { return Err(ctx.tr("The site is read only.")) };
        writer.update_option("active_plugins", &serialize_list(&active)).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    if !ctx.can("activate_plugins") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to manage plugins for this site.")));
    }
    let Some(root) = ctx.env.loader.includes.parent().map(std::path::Path::to_path_buf) else { return die(ctx.env, 500, "WordPress &rsaquo; Error", "<p>The WordPress files are not available.</p>") };
    let installed = plugins::installed(&root);
    let mut active = active_plugins(ctx);
    let mut notices = String::new();
    // "Clear List" of the recently active plugins; the page shows right after.
    if request.request("clear-recent-list").is_some() {
        save_recently_activated(ctx, &[]).await;
    }
    let action = {
        let first = request.request("action").unwrap_or_default();
        if first.is_empty() || first == "-1" { request.request("action2").unwrap_or_default().to_owned() } else { first.to_owned() }
    };
    if let Some(response) = actions(ctx, &action, &installed, &mut active, &root).await {
        return response;
    }
    let updated = |message: String| notice(&message, "", true, "message", &["updated"], true);
    if request.get("error").is_some() {
        let message = if request.get("main").is_some() { tr("You cannot delete a plugin while it is active on the main site.") } else { tr("Plugin could not be activated because it triggered a <strong>fatal error</strong>.") };
        notices.push_str(&notice(&message, "", false, "message", &["error"], true));
    } else if let Some(deleted) = request.get("deleted") {
        notices.push_str(&updated(if deleted == "1" { tr("The selected plugin has been deleted.") } else { tr("The selected plugins have been deleted.") }));
    } else if request.get("activate").is_some() {
        notices.push_str(&updated(tr("Plugin activated.")));
    } else if request.get("activate-multi").is_some() {
        notices.push_str(&updated(tr("Selected plugins activated.")));
    } else if request.get("deactivate").is_some() {
        notices.push_str(&updated(tr("Plugin deactivated.")));
    } else if request.get("deactivate-multi").is_some() {
        notices.push_str(&updated(tr("Selected plugins deactivated.")));
    } else if request.get("enabled-auto-update").is_some() {
        notices.push_str(&updated(tr("Plugin will be auto-updated.")));
    } else if request.get("disabled-auto-update").is_some() {
        notices.push_str(&updated(tr("Plugin will no longer be auto-updated.")));
    } else if request.get("enabled-auto-update-multi").is_some() {
        notices.push_str(&updated(tr("Selected plugins will be auto-updated.")));
    } else if request.get("disabled-auto-update-multi").is_some() {
        notices.push_str(&updated(tr("Selected plugins will no longer be auto-updated.")));
    }

    let searching = request.request("s").is_some_and(|s| !s.trim().is_empty());
    let status = ["all", "active", "inactive", "recently_activated", "mustuse", "auto-update-enabled", "auto-update-disabled"].iter().find(|s| request.get("plugin_status") == Some(**s)).copied().unwrap_or("all");
    // Deactivations older than a week are forgotten.
    let stored_recent = recently_activated(ctx).await;
    let recent: Vec<(String, i64)> = stored_recent.iter().filter(|(_, time)| time + 7 * 86_400 >= ctx.env.now).cloned().collect();
    if recent.len() != stored_recent.len() {
        save_recently_activated(ctx, &recent).await;
    }
    let is_recent = |p: &PluginInfo| recent.iter().any(|(file, _)| *file == p.file);
    let mu = plugins::must_use(&root);
    let auto = auto_updates(ctx);
    let is_active = |p: &PluginInfo| active.contains(&p.file);
    let counts = [
        ("all", installed.len()),
        ("active", installed.iter().filter(|p| is_active(p)).count()),
        ("inactive", installed.iter().filter(|p| !is_active(p)).count()),
        ("recently_activated", installed.iter().filter(|p| !is_active(p) && is_recent(p)).count()),
        ("mustuse", mu.len()),
        ("auto-update-enabled", installed.iter().filter(|p| auto.contains(&p.file)).count()),
        ("auto-update-disabled", installed.iter().filter(|p| !auto.contains(&p.file)).count()),
    ];
    // A list with no plugins in it is the list of all plugins (`$plugins[ $status ]` is empty).
    let status = if status != "all" && counts.iter().find(|(name, _)| *name == status).map_or(0, |(_, n)| *n) == 0 { "all" } else { status };
    // `$status = 'search'` for a search; no view is the current one then.
    let status = if searching { "search" } else { status };
    let label = |s: &str| match s {
        "all" => tr("All"),
        "active" => tr("Active"),
        "inactive" => tr("Inactive"),
        "recently_activated" => tr("Recently Active"),
        "mustuse" => tr("Must-Use"),
        "auto-update-enabled" => tr("Auto-updates Enabled"),
        _ => tr("Auto-updates Disabled"),
    };
    let site = ctx.site();
    let mut views = Vec::new();
    for (name, count) in counts {
        // `all` always shows; the others only with plugins in them.
        if name != "all" && count == 0 {
            continue;
        }
        let current = if name == status { " class=\"current\" aria-current=\"page\"" } else { "" };
        views.push(format!("\t<li class='{name}'><a href=\"plugins.php?plugin_status={name}\"{current}>{} <span class=\"count\">({})</span></a>", label(name), site.number_i18n(count as u64)));
    }
    let last = views.len();
    let views_html = format!("<h2 class='screen-reader-text'>{}</h2><ul class='subsubsub'>\n{}</ul>", tr("Filter plugins list"), views.iter().enumerate().map(|(i, v)| format!("{v}{}\n", if i + 1 == last { "</li>" } else { " |</li>" })).collect::<String>());

    let search = request.request("s").unwrap_or_default().trim().to_lowercase();
    let list: Vec<&PluginInfo> = installed
        .iter()
        .filter(|p| match status {
            "active" => is_active(p),
            "inactive" => !is_active(p),
            "recently_activated" => !is_active(p) && is_recent(p),
            "auto-update-enabled" => auto.contains(&p.file),
            "auto-update-disabled" => !auto.contains(&p.file),
            _ => true,
        })
        .filter(|p| search.is_empty() || [&p.name, &p.description, &p.author].iter().any(|f| f.to_lowercase().contains(&search)))
        .collect();
    let must_use = status == "mustuse";
    let total = if must_use { mu.len() } else { list.len() };
    let mut rows = String::new();
    if must_use {
        for p in &mu {
            rows.push_str(&mu_row(ctx, p));
        }
    } else {
        let slugs = crate::updates::plugin_slugs(site).await;
        for p in &list {
            let api_slug = slugs.iter().find(|(file, _)| *file == p.file).map(|(_, slug)| slug.as_str());
            let info = RowInfo { active: is_active(p), auto_update: auto.contains(&p.file), status, api_slug, uninstallable: is_uninstallable(ctx, &root, &p.file) };
            rows.push_str(&row(ctx, p, &info));
        }
    }
    if rows.is_empty() {
        let message = if status == "mustuse" { tr("No items found.") } else { tr("No plugins found.") };
        rows = format!("<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"{}\">{message}</td></tr>", if must_use { 3 } else { 4 });
    }
    let count_text = rustpress_core::i18n::sprintf(&site.trn("%s item", "%s items", total as i64), &[&site.number_i18n(total as u64)]);
    let pagination_top = format!(
        "<div class='tablenav-pages one-page'><span class=\"displaying-num\">{count_text}</span>\n<span class='pagination-links'><span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&laquo;</span>\n<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&lsaquo;</span>\n<span class=\"paging-input\"><label for=\"current-page-selector\" class=\"screen-reader-text\">{}</label><input class='current-page' id='current-page-selector' type='text'\n\t\t\t\t\tname='paged' value='1' size='1' aria-describedby='table-paging' /><span class='tablenav-paging-text'> {} <span class='total-pages'>1</span></span></span>\n<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&rsaquo;</span>\n<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&raquo;</span></span></div>",
        tr("Current Page"),
        rustpress_core::i18n::sprintf(&tr("of %s"), &[""]).trim_end()
    ).replace(" of  <span", " of <span");
    let pagination_bottom = format!(
        "<div class='tablenav-pages one-page'><span class=\"displaying-num\">{count_text}</span>\n<span class='pagination-links'><span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&laquo;</span>\n<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&lsaquo;</span>\n<span class=\"screen-reader-text\">{}</span><span id=\"table-paging\" class=\"paging-input\"><span class=\"tablenav-paging-text\">1 {} <span class='total-pages'>1</span></span></span>\n<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&rsaquo;</span>\n<span class=\"tablenav-pages-navspan button disabled\" aria-hidden=\"true\">&raquo;</span></span></div>",
        tr("Current Page"),
        rustpress_core::i18n::sprintf(&tr("of %s"), &[""]).trim().to_owned()
    );
    let bulk = |which: &str, name: &str| {
        if must_use {
            return "\t\t\t<div class=\"alignleft actions bulkactions\">\n\t\t\t\t\t</div>\n".to_owned();
        }
        let options: String = [("activate-selected", "Activate"), ("deactivate-selected", "Deactivate"), ("update-selected", "Update"), ("delete-selected", "Delete"), ("enable-auto-update-selected", "Enable Auto-updates"), ("disable-auto-update-selected", "Disable Auto-updates")]
            .iter()
            .filter(|(v, _)| !(status == "inactive" && *v == "deactivate-selected") && !(status == "active" && *v == "activate-selected"))
            .map(|(v, l)| format!("\t<option value=\"{v}\">{}</option>\n", tr(l)))
            .collect();
        format!(
            "\t\t\t<div class=\"alignleft actions bulkactions\">\n\t\t\t<label for=\"bulk-action-selector-{which}\" class=\"screen-reader-text\">{}</label><select name=\"{name}\" id=\"bulk-action-selector-{which}\">\n<option value=\"-1\">{}</option>\n{options}</select>\n<input type=\"submit\" name=\"bulk_action\" id=\"doaction{}\" class=\"button action button-compact\" value=\"{}\"  />\n\t\t</div>\n",
            tr("Select bulk action"),
            tr("Bulk actions"),
            if which == "top" { "" } else { "2" },
            tr("Apply")
        )
    };
    let mu_note_top = if must_use { format!("<div class=\"alignleft actions\"><p>{}</p></div>", rustpress_core::i18n::sprintf(&tr("Files in the %s directory are executed automatically."), &["<code>/wp-content/mu-plugins</code>"])) } else { String::new() };
    let mu_note_bottom = if must_use { "<div class=\"alignleft actions\"></div>".to_owned() } else { String::new() };
    // `extra_tablenav()` of the list of recently active plugins.
    let (mu_note_top, mu_note_bottom) = if status == "recently_activated" {
        let clear = format!("<div class=\"alignleft actions\"><input type=\"submit\" name=\"clear-recent-list\" id=\"clear-recent-list\" class=\"button\" value=\"{}\"  /></div>", esc_attr(&tr("Clear List")));
        (clear.clone(), clear)
    } else {
        (mu_note_top, mu_note_bottom)
    };
    // Screen Options: the description and the automatic updates can be hidden, `plugins_per_page` defaults to 999.
    let hidden = super::common::hidden_columns(ctx, "plugins").await;
    let is_hidden = |column: &str| if hidden.iter().any(|h| h == column) { " hidden" } else { "" };
    let mut option_columns = vec![("description".to_owned(), tr("Description"), !is_hidden("description").is_empty())];
    if !must_use {
        option_columns.push(("auto-updates".to_owned(), tr("Automatic Updates"), !is_hidden("auto-updates").is_empty()));
    }
    let screen_options = crate::chrome::ScreenOptions { columns: option_columns, per_page: Some(("plugins_per_page".into(), super::common::items_per_page(ctx, "plugins_per_page", 999).await)), ..Default::default() };
    let rows = rows.replace("<td class='column-description desc'>", &format!("<td class='column-description desc{}'>", is_hidden("description"))).replace("<td class='column-auto-updates'>", &format!("<td class='column-auto-updates{}'>", is_hidden("auto-updates")));
    let checkbox_head = |id: &str, tfoot: bool| {
        if must_use {
            return format!("<td  {} class='manage-column column-cb check-column'  ></td>", if tfoot { String::new() } else { "id='cb'".to_owned() });
        }
        format!("<td  {} class='manage-column column-cb check-column'  ><input id=\"cb-select-all-{id}\" type=\"checkbox\" />\n\t\t\t<label for=\"cb-select-all-{id}\"><span class=\"screen-reader-text\">{}</span></label></td>", if tfoot { String::new() } else { "id='cb'".to_owned() }, tr("Select All"))
    };
    let heads = |tfoot: bool| {
        let id = |n: &str| if tfoot { " ".to_owned() } else { format!("id='{n}' ") };
        let mut out = format!("<th scope=\"col\" {}class='manage-column column-name column-primary'  >{}</th><th scope=\"col\" {}class='manage-column column-description{}'  >{}</th>", id("name"), tr("Plugin"), id("description"), is_hidden("description"), tr("Description"));
        if !must_use {
            out.push_str(&format!("<th scope=\"col\" {}class='manage-column column-auto-updates{}'  >{}</th>", id("auto-updates"), is_hidden("auto-updates"), tr("Automatic Updates")));
        }
        out
    };
    // Temporary arguments of the notices do not belong into the address the forms come back to.
    let mut request_uri = request.request_uri.clone();
    for arg in ["error", "deleted", "activate", "activate-multi", "deactivate", "deactivate-multi", "enabled-auto-update", "disabled-auto-update", "enabled-auto-update-multi", "disabled-auto-update-multi", "_error_nonce"] {
        request_uri = rustpress_core::urls::remove_query_arg(&request_uri, arg);
    }
    let referer = esc_url(&request_uri);
    let nonce_field = format!("<input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{referer}\" />", ctx.create_nonce("bulk-plugins"));
    let table = format!(
        "<form method=\"post\" id=\"bulk-action-form\">\n\n<input type=\"hidden\" name=\"plugin_status\" value=\"{status}\" />\n<input type=\"hidden\" name=\"paged\" value=\"1\" />\n\n{nonce_field}\t<div class=\"tablenav top\">\n\n\t\t\t\t{}\t\t{mu_note_top}{pagination_top}\n\t\t<br class=\"clear\" />\n\t</div>\n\t\t<h2 class='screen-reader-text'>{}</h2><table class=\"wp-list-table widefat plugins\">\n\t\t\t<thead>\n\t<tr>\n\t\t{}{}\t</tr>\n\t</thead>\n\n\t<tbody id=\"the-list\"\n\t\t\t\t>\n\t\t{rows}\t</tbody>\n\n\t<tfoot>\n\t<tr>\n\t\t{}{}\t</tr>\n\t</tfoot>\n\n</table>\n\t\t\t<div class=\"tablenav bottom\">\n\n\t\t\t\t{}\t\t{mu_note_bottom}{pagination_bottom}\n\t\t<br class=\"clear\" />\n\t</div>\n\t\t</form>\n",
        bulk("top", "action").trim_start_matches('\t'),
        tr("Plugins list"),
        checkbox_head("1", false),
        heads(false),
        checkbox_head("2", true),
        heads(true),
        bulk("bottom", "action2").trim_start_matches('\t'),
    );
    let baked = super::pages::load(ctx, "plugins");
    let _ = baked.content;
    let search_form = format!(
        "<form class=\"search-form search-plugins\" method=\"get\">\n\t\t<p class=\"search-box\">\n\t\t\t<label for=\"plugin-search-input\">{}</label>\n\t\t\t<input type=\"search\" id=\"plugin-search-input\" class=\"wp-filter-search\" name=\"s\" value=\"{}\" />\n\t\t\t<input type=\"submit\" id=\"search-submit\" class=\"button hide-if-js\" value=\"{}\"  />\t\t</p>\n\t\t</form>\n\n",
        tr("Search installed plugins"),
        esc_attr(request.request("s").unwrap_or_default()),
        tr("Search installed plugins")
    );
    let site_url = site.settings.siteurl.trim_end_matches('/');
    let subtitle = if searching {
        format!("	<span class=\"subtitle\">{}
</span>
", rustpress_core::i18n::sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(request.request("s").unwrap_or_default()))]))
    } else {
        "	
".to_owned()
    };
    let heading = format!("{notices}<div class=\"wrap\">\n<h1 class=\"wp-heading-inline\">\n{}</h1>\n\n\t<a href=\"{site_url}/wp-admin/plugin-install.php\" class=\"page-title-action\">{}</a>\n{subtitle}<hr class=\"wp-header-end\">\n\n\n", tr("Plugins"), tr("Add Plugin"));
    // The templates of the updates script follow the form; they are the tail of the baked page.
    let tail_at = baked.content.find("\t<span class=\"spinner\"></span>").unwrap_or(0);
    let tail = &baked.content[tail_at..];
    let content = format!("{heading}{views_html}\n{search_form}{table}\n{tail}");
    let screen = Screen { file: "plugins.php".into(), parent_file: "plugins.php".into(), submenu_file: Some("plugins.php".into()), title: tr("Plugins"), help: Some(baked.help), options: Some(screen_options), ..Default::default() };
    // `_wpUpdatesItemCounts`: the plugin files of every view and the update counts.
    let upgrades = crate::updates::plugin_updates(site).await;
    let files = |filter: &dyn Fn(&PluginInfo) -> bool| serde_json::Value::from(installed.iter().filter(|p| filter(p)).map(|p| p.file.clone()).collect::<Vec<_>>());
    let item_counts = serde_json::json!({
        "plugins": {
            "all": files(&|_| true),
            "search": if searching { files(&|p| [&p.name, &p.description, &p.author].iter().any(|f| f.to_lowercase().contains(&search))) } else { serde_json::json!([]) },
            "active": files(&|p| is_active(p)),
            "inactive": files(&|p| !is_active(p)),
            "recently_activated": files(&|p| !is_active(p) && is_recent(p)),
            "upgrade": files(&|p| upgrades.contains(&p.file)),
            "mustuse": mu.iter().map(|p| p.file.clone()).collect::<Vec<_>>(),
            "dropins": [],
            "paused": [],
            "auto-update-enabled": files(&|p| auto.contains(&p.file)),
            "auto-update-disabled": files(&|p| !auto.contains(&p.file)),
        },
        "totals": crate::updates::update_data(site, crate::updates::counts(site).await),
    });
    let assets = Assets { scripts: vec!["updates".into(), "plugin-install".into()], styles: vec!["thickbox".into()], localize: vec![("updates".into(), "_wpUpdatesItemCounts".into(), item_counts)], ..Default::default() };
    render(ctx, &screen, &content, &assets).await
}
