//! `plugin-install.php`, `theme-install.php` and `update.php`: adding plugins and themes from a zip file.
//!
//! RustPress does not talk to wordpress.org, so the directory tabs show the error WordPress shows when it cannot
//! reach it; uploading an archive works like in WordPress.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use super::notices::notice;
use super::{AdminResponse, Ctx, die, render};
use crate::chrome::{Assets, Screen};
use crate::plugins::{self, PluginInfo};
use crate::themes;

fn wp_root(ctx: &Ctx<'_>) -> Option<PathBuf> {
    ctx.env.loader.includes.parent().map(Path::to_path_buf)
}

/// The nonces the baked upload pages carry, put in for fresh ones.
fn fresh_nonces(ctx: &Ctx<'_>, content: &str, pairs: &[(&str, String)]) -> String {
    let mut out = content.to_owned();
    for (old, action) in pairs {
        out = out.replace(old, &ctx.create_nonce(action));
    }
    out
}

/// `plugin-install.php`.
pub async fn plugin_install(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("install_plugins") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to install plugins on this site.")));
    }
    let tab = ctx.request.get("tab").unwrap_or("featured").to_owned();
    let baked = super::pages::load(ctx, "plugin-install-upload");
    let mut content = fresh_nonces(ctx, &baked.content, &[("b896137a2a", "plugin-upload".to_owned())]).replace("value=\"/wp-admin/plugin-install.php?tab=upload\"", &format!("value=\"{}\"", esc_attr(&ctx.request.request_uri)));
    if tab != "upload" {
        // The browse tabs: the upload form is folded away and the list says why it is empty.
        let a = content.find("<div class=\"upload-plugin\">").unwrap_or(0);
        let b = content.find("\t\n\t<span class=\"spinner\"></span>").unwrap_or(content.len());
        let upload = content[a..b].trim_end().to_owned();
        let tabs = [("featured", tr("Featured")), ("popular", tr("Popular")), ("recommended", tr("Recommended")), ("favorites", tr("Favorites"))];
        let links: String = tabs
            .iter()
            .map(|(name, label)| format!("\t<li class='plugin-install-{name}'><a href=\"{}\"{}>{label}</a>{}</li>\n", ctx.admin_url(&format!("plugin-install.php?tab={name}")), if *name == tab { " class=\"current\" aria-current=\"page\"" } else { "" }, if *name == "favorites" { "" } else { " " }))
            .collect();
        let error = notice(
            &format!("<p>{}</p><p class=\"hide-if-no-js\"><button class=\"button try-again\">{}</button></p>", tr("An unexpected error occurred. Something may be wrong with WordPress.org or this server&#8217;s configuration. If you continue to have problems, please try the <a href=\"https://wordpress.org/support/forums/\">support forums</a>."), tr("Try Again")),
            "",
            false,
            "",
            &["inline", "error"],
            false,
        );
        let browse = format!(
            "<div class=\"wrap plugin-install-tab-{tab}\">\n<h1 class=\"wp-heading-inline\">\n{}</h1>\n\n <a href=\"{}\" class=\"upload-view-toggle page-title-action\"><span class=\"upload\">{}</span><span class=\"browse\">{}</span></a>\n<hr class=\"wp-header-end\">\n\t<div class=\"upload-plugin-wrap\">\n\t\t{upload}\n\t\t</div>\n\t<h2 class='screen-reader-text'>{}</h2><p>{}</p><div class=\"wp-filter\">\n\t<ul class=\"filter-links\">\n{links}\t</ul>\n\t\t\t<form class=\"search-form search-plugins\" method=\"get\">\n\t\t<input type=\"hidden\" name=\"tab\" value=\"search\" />\n\t\t<label for=\"search-plugins\">{}</label>\n\t\t<input type=\"search\" name=\"s\" id=\"search-plugins\" value=\"\" class=\"wp-filter-search\" />\n\t\t<input type=\"submit\" id=\"search-submit\" class=\"button hide-if-js\" value=\"{}\"  />\t</form>\n\t</div>\n\t\t<br>\t<form id=\"plugin-filter\" method=\"post\">\n\t\t<div class=\"wp-list-table widefat plugin-install\">\n\t\t<h2 class='screen-reader-text'>{}</h2>\t<div id=\"the-list\">\n\t\t{error}\t</div>\n\t</div>\n\t</form>\n",
            tr("Add Plugins"),
            ctx.admin_url("plugin-install.php?tab=upload"),
            tr("Upload Plugin"),
            tr("Browse Plugins"),
            tr("Filter plugins list"),
            tr("Plugins extend and expand the functionality of WordPress. You may install plugins from the <a href=\"https://wordpress.org/plugins/\">WordPress Plugin Directory</a> right on this page, or upload a plugin in .zip format by clicking the button above."),
            tr("Search Plugins"),
            tr("Search Plugins"),
            tr("Plugins list"),
        );
        let tail = content[content.find("\t<span class=\"spinner\"></span>").unwrap_or(content.len())..].to_owned();
        content = format!("{browse}{tail}");
    }
    let content = content.replace("{{site}}", ctx.site().settings.siteurl.trim_end_matches('/'));
    let screen = Screen { file: "plugin-install.php".into(), parent_file: "plugins.php".into(), submenu_file: Some("plugin-install.php".into()), title: tr("Add Plugins"), help: Some(baked.help), ..Default::default() };
    render(ctx, &screen, &content, &Assets { scripts: vec!["plugin-install".into(), "updates".into()], styles: vec!["thickbox".into()], ..Default::default() }).await
}

/// `theme-install.php`.
pub async fn theme_install(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("install_themes") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to install themes on this site.")));
    }
    let baked = super::pages::load(ctx, "theme-install-upload");
    let content = fresh_nonces(ctx, &baked.content, &[("111475f3f0", "theme-upload".to_owned()), ("5381e0bb1b", format!("save_wporg_username_{}", ctx.user.id))])
        .replace("value=\"/wp-admin/theme-install.php?tab=upload\"", &format!("value=\"{}\"", esc_attr(&ctx.request.request_uri)))
        .replace("{{site}}", ctx.site().settings.siteurl.trim_end_matches('/'));
    let screen = Screen { file: "theme-install.php".into(), parent_file: "themes.php".into(), submenu_file: Some("themes.php".into()), title: tr("Add Themes"), help: Some(baked.help), ..Default::default() };
    let installed: Vec<String> = wp_root(ctx).map(|root| themes::installed(&root).into_iter().map(|t| t.stylesheet).collect()).unwrap_or_default();
    let settings = serde_json::json!({
        "themes": "",
        "settings": {"isInstall": true, "canInstall": true, "installURI": ctx.admin_url("theme-install.php"), "adminUrl": "/wp-admin/"},
        "l10n": {
            "addNew": tr("Add Theme"),
            "search": tr("Search Themes"),
            "upload": tr("Upload Theme"),
            "back": tr("Back"),
            "error": tr("An unexpected error occurred. Something may be wrong with WordPress.org or this server&#8217;s configuration. If you continue to have problems, please try the <a href=\"https://wordpress.org/support/forums/\">support forums</a>."),
            "tryAgain": tr("Try Again"),
            "themesFound": tr("Number of Themes found: %d"),
            "noThemesFound": tr("No themes found. Try a different search."),
            "themeViewed": tr("Theme details: %s"),
            "collapseSidebar": tr("Collapse Sidebar"),
            "expandSidebar": tr("Expand Sidebar"),
            "selectFeatureFilter": tr("Select one or more Theme features to filter by"),
        },
        "installedThemes": installed,
        "activeTheme": ctx.site().settings.stylesheet,
    });
    let assets = Assets { scripts: vec!["theme".into(), "updates".into()], localize: vec![("theme".into(), "_wpThemeSettings".into(), settings)], ..Default::default() };
    render(ctx, &screen, &content, &assets).await
}

/// One finished screen of `update.php`: a heading and the lines the installer prints.
async fn feedback(ctx: &Ctx<'_>, parent: &str, submenu: &str, title: &str, heading: &str, lines: &[String]) -> AdminResponse {
    let content = format!("<div class=\"wrap\"><h1>{}</h1>{}\n</div>", heading, lines.join("\n"));
    let screen = Screen { file: "update.php".into(), parent_file: parent.into(), submenu_file: Some(submenu.into()), title: title.to_owned(), ..Default::default() };
    render(ctx, &screen, &content, &Assets::default()).await
}

/// The directory entries of an archive that are safe to unpack, with their contents.
fn read_zip(bytes: &[u8]) -> Result<Vec<(String, Option<Vec<u8>>)>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "Incompatible Archive.".to_owned())?;
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|_| "Incompatible Archive.".to_owned())?;
        let Some(name) = file.enclosed_name() else { return Err("Incompatible Archive.".to_owned()) };
        let name = name.to_string_lossy().replace('\\', "/");
        if name.starts_with("__MACOSX/") || name.is_empty() {
            continue;
        }
        if file.is_dir() {
            entries.push((name.trim_end_matches('/').to_owned(), None));
        } else {
            let mut data = Vec::new();
            file.read_to_end(&mut data).map_err(|_| "Incompatible Archive.".to_owned())?;
            entries.push((name, Some(data)));
        }
    }
    Ok(entries)
}

/// The folder an archive installs into and the entries below it: a single top-level folder is unwrapped, otherwise
/// the name of the file stands for it.
fn layout(entries: &[(String, Option<Vec<u8>>)], zip_name: &str) -> (String, Vec<(String, Option<Vec<u8>>)>) {
    let tops: std::collections::BTreeSet<&str> = entries.iter().map(|(n, _)| n.split('/').next().unwrap_or_default()).collect();
    let single_folder = tops.len() == 1 && entries.iter().any(|(n, d)| n.contains('/') || d.is_none());
    if single_folder {
        let top = tops.iter().next().copied().unwrap_or_default().to_owned();
        let rest = entries.iter().filter_map(|(n, d)| n.strip_prefix(&format!("{top}/")).map(|r| (r.to_owned(), d.clone()))).collect();
        (top, rest)
    } else {
        let stem = zip_name.trim_end_matches(".zip").trim_end_matches(".ZIP");
        let stem: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') { c } else { '-' }).collect();
        (stem, entries.to_vec())
    }
}

fn header(text: &str, name: &str) -> String {
    for line in text.lines().take(60) {
        let line = line.trim().trim_start_matches(['/', '*', '#', ' ']).trim();
        if let Some((key, value)) = line.split_once(':')
            && key.trim().eq_ignore_ascii_case(name)
        {
            return value.trim().trim_end_matches("*/").trim().to_owned();
        }
    }
    String::new()
}

fn write_tree(dest: &Path, entries: &[(String, Option<Vec<u8>>)]) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for (name, data) in entries {
        let target = dest.join(name);
        match data {
            None => std::fs::create_dir_all(&target)?,
            Some(bytes) => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, bytes)?;
            }
        }
    }
    Ok(())
}

pub async fn update_php(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let action = ctx.request.get("action").unwrap_or_default().to_owned();
    match action.as_str() {
        "upload-plugin" => upload(ctx, true).await,
        "upload-theme" => upload(ctx, false).await,
        _ => die(ctx.env, 501, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("RustPress does not install, update or fetch anything from WordPress.org."))),
    }
}

async fn upload(ctx: &Ctx<'_>, plugin: bool) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    if !ctx.can(if plugin { "install_plugins" } else { "install_themes" }) {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", if plugin { tr("Sorry, you are not allowed to install plugins on this site.") } else { tr("Sorry, you are not allowed to install themes on this site.") }));
    }
    if !request.post || !ctx.verify_nonce(if plugin { "plugin-upload" } else { "theme-upload" }) {
        return die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
    }
    let field = if plugin { "pluginzip" } else { "themezip" };
    let Some(file) = request.files.iter().find(|f| f.field == field) else {
        return die(ctx.env, 400, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Please select a file")));
    };
    if !file.filename.to_lowercase().ends_with(".zip") {
        return die(ctx.env, 400, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Only .zip archives may be uploaded.")));
    }
    if ctx.env.writer.is_none() {
        return super::common::read_only(ctx);
    }
    let Some(root) = wp_root(ctx) else { return die(ctx.env, 500, "WordPress &rsaquo; Error", "<p>The WordPress files are not available.</p>") };
    let filename = file.filename.rsplit(['/', '\\']).next().unwrap_or_default().to_owned();
    let heading = if plugin { rustpress_core::i18n::sprintf(&tr("Installing plugin from uploaded file: %s"), &[&esc_html(&filename)]) } else { rustpress_core::i18n::sprintf(&tr("Installing theme from uploaded file: %s"), &[&esc_html(&filename)]) };
    let (parent, submenu, title) = if plugin { ("plugins.php", "plugin-install.php", tr("Upload Plugin")) } else { ("themes.php", "theme-install.php", tr("Upload Theme")) };
    let mut lines = vec![format!("<p>{}</p>", tr("Unpacking the package&#8230;")), format!("<p>{}</p>", if plugin { tr("Installing the plugin&#8230;") } else { tr("Installing the theme&#8230;") })];
    let failed = |mut lines: Vec<String>, reason: String| {
        lines.push(format!("<p>{}</p>", rustpress_core::i18n::sprintf(&ctx.tr("The package could not be installed. %s"), &[&reason])));
        lines.push(format!("<p>{}</p>", if plugin { ctx.tr("Plugin installation failed.") } else { ctx.tr("Theme installation failed.") }));
        if plugin {
            lines.push(format!("<p><a href=\"{}\">{}</a></p>", ctx.admin_url("plugin-install.php"), ctx.tr("Go to Plugin Installer")));
        }
        lines
    };

    let entries = match read_zip(&file.bytes) {
        Ok(entries) => entries,
        Err(reason) => {
            lines.push(format!("<p>{}</p>", ctx.tr(&reason)));
            lines.push(format!("<p>{}</p>", if plugin { tr("Plugin installation failed.") } else { tr("Theme installation failed.") }));
            return feedback(ctx, parent, submenu, &title, &heading, &lines).await;
        }
    };
    let (folder, files) = layout(&entries, &filename);
    let read = |name: &str| files.iter().find(|(n, _)| n == name).and_then(|(_, d)| d.as_ref()).map(|d| String::from_utf8_lossy(d).into_owned());
    if plugin {
        let valid = files.iter().any(|(n, d)| !n.contains('/') && n.ends_with(".php") && d.as_ref().is_some_and(|d| !header(&String::from_utf8_lossy(d), "Plugin Name").is_empty()));
        if !valid {
            return feedback(ctx, parent, submenu, &title, &heading, &failed(lines, tr("No valid plugins were found."))).await;
        }
    } else {
        let Some(style) = read("style.css") else { return feedback(ctx, parent, submenu, &title, &heading, &failed(lines, tr("The theme is missing the style.css stylesheet."))).await };
        if header(&style, "Theme Name").is_empty() {
            return feedback(ctx, parent, submenu, &title, &heading, &failed(lines, tr("The style.css stylesheet doesn&#8217;t contain a valid theme header. Missing theme name."))).await;
        }
        let template = header(&style, "Template");
        let has_template = files.iter().any(|(n, d)| d.is_some() && matches!(n.as_str(), "templates/index.html" | "index.php"));
        if template.is_empty() && !has_template {
            return feedback(ctx, parent, submenu, &title, &heading, &failed(lines, tr("Template is missing. Standalone themes need to have a templates/index.html or index.php template file. Child themes need to have a Template header in the style.css stylesheet."))).await;
        }
    }
    let base = root.join("wp-content").join(if plugin { "plugins" } else { "themes" });
    let destination = base.join(&folder);
    if folder.is_empty() || folder.contains("..") || destination.exists() {
        // WordPress prints this one without the "could not be installed" prefix.
        lines.push(format!("<p>{} {}/</p>", tr("Destination folder already exists."), destination.display().to_string().replace('\\', "/")));
        lines.push(format!("<p>{}</p>", if plugin { tr("Plugin installation failed.") } else { tr("Theme installation failed.") }));
        if plugin {
            lines.push(format!("<p><a href=\"{}\">{}</a></p>", ctx.admin_url("plugin-install.php"), tr("Go to Plugin Installer")));
        }
        return feedback(ctx, parent, submenu, &title, &heading, &lines).await;
    }
    if write_tree(&destination, &files).is_err() {
        let _ = std::fs::remove_dir_all(&destination);
        return feedback(ctx, parent, submenu, &title, &heading, &failed(lines, tr("Could not create directory."))).await;
    }
    if plugin {
        lines.push(format!("<p>{}</p>", tr("Plugin installed successfully.")));
        let installed: Vec<PluginInfo> = plugins::installed(&root);
        let file = installed.iter().find(|p| p.file.starts_with(&format!("{folder}/")) || p.file == folder).map(|p| p.file.clone()).unwrap_or_else(|| format!("{folder}/{folder}.php"));
        let activate = format!("plugins.php?action=activate&amp;plugin={}&amp;_wpnonce={}", urlencode(&file), ctx.create_nonce(&format!("activate-plugin_{file}")));
        lines.push(format!("<p><a class=\"button button-primary\" href=\"{activate}\" target=\"_parent\">{}</a> <a href=\"{}\">{}</a></p>", tr("Activate Plugin"), ctx.admin_url("plugin-install.php"), tr("Go to Plugin Installer")));
    } else {
        lines.push(format!("<p>{}</p>", tr("Theme installed successfully.")));
        let name = themes::find(&root, &folder).map_or_else(|| folder.clone(), |t| t.name);
        let preview = format!("{}?wp_theme_preview={}&#038;return={}", ctx.admin_url("site-editor.php"), esc_attr(&folder), urlencode(&ctx.admin_url("themes.php")));
        let activate = format!("{}?action=activate&#038;template={folder}&#038;stylesheet={folder}&#038;_wpnonce={}", ctx.admin_url("themes.php"), ctx.create_nonce(&format!("switch-theme_{folder}")));
        let quoted = format!("&#8220;{}&#8221;", esc_html(&name));
        lines.push(format!(
            "<p><a href=\"{}\" class=\"hide-if-no-customize load-customize\"><span aria-hidden=\"true\">{}</span><span class=\"screen-reader-text\">{} {quoted}</span></a> | <a href=\"{activate}\" class=\"activatelink\"><span aria-hidden=\"true\">{}</span><span class=\"screen-reader-text\">{} {quoted}</span></a> | <a href=\"{}\" target=\"_parent\">{}</a></p>",
            esc_url(&preview).replace("&amp;", "&#038;"),
            tr("Live Preview"),
            tr("Live Preview"),
            tr("Activate"),
            tr("Activate"),
            ctx.admin_url("themes.php"),
            tr("Go to Themes page")
        ));
    }
    feedback(ctx, parent, submenu, &title, &heading, &lines).await
}

/// `urlencode()`.
fn urlencode(text: &str) -> String {
    text.bytes().map(|b| if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') { (b as char).to_string() } else if b == b' ' { "+".to_owned() } else { format!("%{b:02X}") }).collect()
}
