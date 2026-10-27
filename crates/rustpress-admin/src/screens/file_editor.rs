//! `theme-editor.php` and `plugin-editor.php`: the file editors of themes and plugins, and the Ajax action that saves
//! a file (`wp_edit_theme_plugin_file()`).

use std::path::{Path, PathBuf};

use rustpress_core::canonical::rawurlencode;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url, wptexturize};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::Value;

use super::notices::notice;
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::plugins::{self as plugin_scan, PluginInfo};
use crate::themes::{self, ThemeInfo};

const EDITABLE_TYPES: [&str; 33] = [
    "bash", "conf", "css", "diff", "htm", "html", "http", "inc", "include", "js", "mjs", "json", "jsx", "less", "md", "patch", "php", "php3", "php4", "php5", "php7", "phps", "phtml", "sass", "scss", "sh", "sql", "svg", "text", "txt",
    "xml", "yaml", "yml",
];

/// An error of `WP_Error`: code, message and the data of `wp_send_json_error()`.
pub struct EditError {
    pub code: &'static str,
    pub message: String,
}

fn error(code: &'static str, message: impl Into<String>) -> EditError {
    EditError { code, message: message.into() }
}

fn wp_root(env: &AdminEnv<'_>) -> Option<PathBuf> {
    env.loader.includes.parent().map(Path::to_path_buf)
}

/// `validate_file()`: 0 for a name that is fine, 1 for `../`, 2 for a drive, 3 for a file that is not allowed.
fn validate_file(file: &str, allowed: &[String]) -> u8 {
    if file.is_empty() {
        return 0;
    }
    if file == "../" {
        return 1;
    }
    if file.matches("../").count() > 1 {
        return 1;
    }
    if file.contains("../") && !file.ends_with("../") {
        return 1;
    }
    if !allowed.is_empty() && !allowed.iter().any(|a| a == file) {
        return 3;
    }
    if file.chars().nth(1) == Some(':') {
        return 2;
    }
    0
}

/// `WP_Theme::scandir()`: the files with one of the extensions, below `path`, by name.
fn scandir(path: &Path, extension: &str, depth: i32, relative: &str) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(path) else { return Vec::new() };
    let mut names: Vec<String> = entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for name in names {
        if name.starts_with('.') || ["CVS", "node_modules", "vendor", "bower_components"].contains(&name.as_str()) {
            continue;
        }
        let full = path.join(&name);
        if full.is_dir() {
            if depth == 0 {
                continue;
            }
            files.extend(scandir(&full, extension, depth - 1, &format!("{relative}{name}/")));
        } else if name.ends_with(&format!(".{extension}")) {
            files.push((format!("{relative}{name}"), full));
        }
    }
    files
}

/// `$allowed_files += ...`: what is already there stays.
fn union(into: &mut Vec<(String, PathBuf)>, more: Vec<(String, PathBuf)>) {
    for item in more {
        if !into.iter().any(|(k, _)| *k == item.0) {
            into.push(item);
        }
    }
}

/// The files of the theme the editor offers, `style.css` and `functions.php` first.
fn theme_files(theme: &ThemeInfo) -> Vec<(String, PathBuf)> {
    let mut allowed: Vec<(String, PathBuf)> = Vec::new();
    for kind in EDITABLE_TYPES {
        let found = scandir(&theme.dir, kind, -1, "");
        if kind == "css"
            && let Some(style) = found.iter().find(|(k, _)| k == "style.css").cloned() {
                union(&mut allowed, vec![style]);
            }
        union(&mut allowed, found);
    }
    if let Some(at) = allowed.iter().position(|(k, _)| k == "functions.php") {
        let item = allowed.remove(at);
        allowed.insert(0, item);
    }
    if let Some(at) = allowed.iter().position(|(k, _)| k == "style.css") {
        let item = allowed.remove(at);
        allowed.insert(0, item);
    }
    allowed
}

/// `list_files()` of a plugin folder: every file below it (hidden files and the usual folders left out).
fn list_files(folder: &Path, levels: u32, exclusions: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(folder) else { return out };
    let mut names: Vec<String> = entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    for name in names {
        if name.starts_with('.') || exclusions.contains(&name.as_str()) {
            continue;
        }
        let full = folder.join(&name);
        if full.is_dir() {
            let inner = if levels > 1 { list_files(&full, levels - 1, &[]) } else { Vec::new() };
            if inner.is_empty() {
                out.push(format!("{}/{name}/", folder.to_string_lossy().replace('\\', "/").trim_end_matches('/')));
            } else {
                out.extend(inner);
            }
        } else {
            out.push(format!("{}/{name}", folder.to_string_lossy().replace('\\', "/").trim_end_matches('/')));
        }
    }
    out
}

/// `get_plugin_files()`: the file of the plugin and, for one in a folder, every file in the folder.
fn plugin_files(root: &Path, plugin: &str) -> Vec<String> {
    let plugins_dir = root.join("wp-content").join("plugins");
    let plugin_file = plugins_dir.join(plugin);
    let mut files = vec![plugin.to_owned()];
    if let Some(dir) = plugin_file.parent()
        && dir.is_dir()
        && dir != plugins_dir
    {
        let base = plugins_dir.to_string_lossy().replace('\\', "/");
        for file in list_files(dir, 100, &["CVS", "node_modules", "vendor", "bower_components"]) {
            let relative = file.strip_prefix(&base).map_or(file.clone(), |r| r.trim_start_matches('/').to_owned());
            if !files.contains(&relative) {
                files.push(relative);
            }
        }
    }
    files
}

fn extension_of(file: &str) -> Option<String> {
    let name = file.rsplit('/').next().unwrap_or(file);
    let (stem, ext) = name.rsplit_once('.')?;
    (!ext.is_empty() && !stem.is_empty() || name.starts_with('.') && !ext.is_empty()).then(|| ext.to_owned())
}

/// `wp_edit_theme_plugin_file()` for a file of a theme or a plugin.
pub async fn edit_file(env: &AdminEnv<'_>, request: &AdminRequest, user: &rustpress_auth::CurrentUser, token: &str, field: &(dyn Fn(&str) -> Option<String> + Sync)) -> Result<(), EditError> {
    let Some(file) = field("file").filter(|f| !f.is_empty()) else { return Err(error("missing_file", "")) };
    if validate_file(&file, &[]) != 0 {
        return Err(error("bad_file", ""));
    }
    let Some(content) = field("newcontent") else { return Err(error("missing_content", "")) };
    let Some(nonce) = field("nonce") else { return Err(error("missing_nonce", "")) };
    let tr = |s: &str| env.site.tr(s);
    let _ = request;
    let nonce_ok = |action: &str| env.auth.verify_nonce(&nonce, action, user.id, token, env.now).is_some();
    let Some(root) = wp_root(env) else { return Err(error("missing_theme_or_plugin", "")) };
    let real_file: PathBuf;
    if let Some(plugin) = field("plugin").filter(|p| !p.is_empty()) {
        if !user.can("edit_plugins") {
            return Err(error("unauthorized", tr("Sorry, you are not allowed to edit plugins for this site.")));
        }
        if !nonce_ok(&format!("edit-plugin_{file}")) {
            return Err(error("nonce_failure", ""));
        }
        if !plugin_scan::installed(&root).iter().any(|p| p.file == plugin) {
            return Err(error("invalid_plugin", ""));
        }
        if validate_file(&file, &plugin_files(&root, &plugin)) != 0 {
            return Err(error("bad_plugin_file_path", tr("Sorry, that file cannot be edited.")));
        }
        real_file = root.join("wp-content").join("plugins").join(&file);
    } else if let Some(stylesheet) = field("theme").filter(|t| !t.is_empty()) {
        if validate_file(&stylesheet, &[]) != 0 {
            return Err(error("bad_theme_path", ""));
        }
        if !user.can("edit_themes") {
            return Err(error("unauthorized", tr("Sorry, you are not allowed to edit templates for this site.")));
        }
        let Some(theme) = themes::find(&root, &stylesheet) else { return Err(error("non_existent_theme", tr("The requested theme does not exist."))) };
        if !nonce_ok(&format!("edit-theme_{stylesheet}_{file}")) {
            return Err(error("nonce_failure", ""));
        }
        let allowed: Vec<String> = theme_files(&theme).into_iter().map(|(k, _)| k).collect();
        if validate_file(&file, &allowed) != 0 {
            return Err(error("disallowed_theme_file", tr("Sorry, that file cannot be edited.")));
        }
        real_file = theme.dir.join(&file);
    } else {
        return Err(error("missing_theme_or_plugin", ""));
    }
    if !real_file.is_file() {
        return Err(error("file_does_not_exist", tr("File does not exist! Please double check the name and try again.")));
    }
    if let Some(extension) = extension_of(&real_file.to_string_lossy()).map(|e| e.to_lowercase())
        && !EDITABLE_TYPES.contains(&extension.as_str())
    {
        return Err(error("illegal_file_type", tr("Files of this type are not editable.")));
    }
    if std::fs::metadata(&real_file).map_or(true, |m| m.permissions().readonly()) {
        return Err(error("file_not_writable", ""));
    }
    if env.writer.is_none() {
        return Err(error("file_not_writable", ""));
    }
    if std::fs::write(&real_file, content.as_bytes()).is_err() {
        return Err(error("unable_to_write", tr("Unable to write to file.")));
    }
    Ok(())
}

/// The Ajax action `edit-theme-plugin-file`.
pub async fn ajax_edit(env: &AdminEnv<'_>, request: &AdminRequest, user: &rustpress_auth::CurrentUser, token: &str) -> AdminResponse {
    let field = |name: &str| request.post_field(name).map(str::to_owned);
    match edit_file(env, request, user, token, &field).await {
        Ok(()) => super::ajax::json(200, &serde_json::json!({"success": true, "data": {"message": env.site.tr("File edited successfully.")}})),
        Err(e) => super::ajax::json(200, &serde_json::json!({"success": false, "data": {"code": e.code, "message": e.message}})),
    }
}

/// The tree of the files: directories hold files and further directories in the order they appear.
enum Entry {
    File(String),
    Dir(Vec<(String, Entry)>),
}

fn insert(tree: &mut Vec<(String, Entry)>, parts: &[&str], file: &str) {
    let Some((first, rest)) = parts.split_first() else { return };
    if rest.is_empty() {
        match tree.iter_mut().find(|(k, _)| k == first) {
            Some(slot) => slot.1 = Entry::File(file.to_owned()),
            None => tree.push(((*first).to_owned(), Entry::File(file.to_owned()))),
        }
        return;
    }
    let position = match tree.iter().position(|(k, e)| k == first && matches!(e, Entry::Dir(_))) {
        Some(p) => p,
        None => {
            tree.retain(|(k, _)| k != first);
            tree.push(((*first).to_owned(), Entry::Dir(Vec::new())));
            tree.len() - 1
        }
    };
    if let Entry::Dir(children) = &mut tree[position].1 {
        insert(children, rest, file);
    }
}

/// `get_file_description()`.
fn file_description(ctx: &Ctx<'_>, file: &str, path: &Path) -> String {
    let tr = |s: &str| ctx.tr(s);
    let basename = file.rsplit('/').next().unwrap_or(file);
    let dirname = file.rsplit_once('/').map_or(".", |(d, _)| d);
    let known = match basename {
        "functions.php" => Some(tr("Theme Functions")),
        "header.php" => Some(tr("Theme Header")),
        "footer.php" => Some(tr("Theme Footer")),
        "sidebar.php" => Some(tr("Sidebar")),
        "comments.php" => Some(tr("Comments")),
        "searchform.php" => Some(tr("Search Form")),
        "404.php" => Some(tr("404 Template")),
        "link.php" => Some(tr("Links Template")),
        "theme.json" => Some(tr("Theme Styles & Block Settings")),
        "index.php" => Some(tr("Main Index Template")),
        "archive.php" => Some(tr("Archives")),
        "author.php" => Some(tr("Author Template")),
        "taxonomy.php" => Some(tr("Taxonomy Template")),
        "category.php" => Some(tr("Category Template")),
        "tag.php" => Some(tr("Tag Template")),
        "home.php" => Some(tr("Posts Page")),
        "search.php" => Some(tr("Search Results")),
        "date.php" => Some(tr("Date Template")),
        "singular.php" => Some(tr("Singular Template")),
        "single.php" => Some(tr("Single Post")),
        "page.php" => Some(tr("Single Page")),
        "front-page.php" => Some(tr("Homepage")),
        "privacy-policy.php" => Some(tr("Privacy Policy Page")),
        "attachment.php" => Some(tr("Attachment Template")),
        "image.php" => Some(tr("Image Attachment Template")),
        "video.php" => Some(tr("Video Attachment Template")),
        "audio.php" => Some(tr("Audio Attachment Template")),
        "application.php" => Some(tr("Application Attachment Template")),
        "embed.php" => Some(tr("Embed Template")),
        "embed-404.php" => Some(tr("Embed 404 Template")),
        "embed-content.php" => Some(tr("Embed Content Template")),
        "header-embed.php" => Some(tr("Embed Header Template")),
        "footer-embed.php" => Some(tr("Embed Footer Template")),
        "style.css" => Some(tr("Stylesheet")),
        "editor-style.css" => Some(tr("Visual Editor Stylesheet")),
        "editor-style-rtl.css" => Some(tr("Visual Editor RTL Stylesheet")),
        "rtl.css" => Some(tr("RTL Stylesheet")),
        "my-hacks.php" => Some(tr("my-hacks.php (legacy hacks support)")),
        ".htaccess" => Some(tr(".htaccess (for rewrite rules )")),
        "wp-layout.css" => Some(tr("Stylesheet")),
        "wp-comments.php" => Some(tr("Comments Template")),
        "wp-comments-popup.php" => Some(tr("Popup Comments Template")),
        "comments-popup.php" => Some(tr("Popup Comments")),
        _ => None,
    };
    if let Some(known) = known
        && dirname == "."
    {
        return known;
    }
    if let Ok(text) = std::fs::read_to_string(path) {
        // `|Template Name:(.*)$|mi`
        for line in text.lines() {
            if let Some(at) = line.to_lowercase().find("template name:") {
                let name = line[at + "template name:".len()..].trim().trim_end_matches("*/").trim().to_owned();
                return sprintf(&tr("%s Page Template"), &[&name]);
            }
        }
    }
    basename.trim().to_owned()
}

/// `wp_print_theme_file_tree()`.
fn print_theme_tree(ctx: &Ctx<'_>, tree: &[(String, Entry)], paths: &[(String, PathBuf)], relative_file: &str, stylesheet: &str, level: usize, out: &mut String) {
    let size = tree.len();
    for (position, (label, entry)) in tree.iter().enumerate() {
        let index = position + 1;
        match entry {
            Entry::Dir(children) => {
                out.push_str(&format!(
                    "\t\t\t<li role=\"treeitem\" aria-expanded=\"true\" tabindex=\"-1\"\n\t\t\t\taria-level=\"{level}\"\n\t\t\t\taria-setsize=\"{size}\"\n\t\t\t\taria-posinset=\"{index}\">\n\t\t\t\t<span class=\"folder-label\">{} <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span><span aria-hidden=\"true\" class=\"icon\"></span></span>\n\t\t\t\t<ul role=\"group\" class=\"tree-folder\">",
                    esc_html(label),
                    ctx.tr("folder")
                ));
                print_theme_tree(ctx, children, paths, relative_file, stylesheet, level + 1, out);
                out.push_str("</ul>\n\t\t\t</li>\n\t\t\t");
            }
            Entry::File(filename) => {
                // The arguments of the recursive call are swapped: the set size is the position, the position the size.
                let url = format!("{}?file={}&theme={}", ctx.admin_url("theme-editor.php"), rawurlencode(filename), rawurlencode(stylesheet));
                let current = relative_file == filename;
                let path = paths.iter().find(|(k, _)| k == filename).map(|(_, p)| p.clone()).unwrap_or_default();
                let mut description = esc_html(&file_description(ctx, filename, &path));
                let basename = filename.rsplit('/').next().unwrap_or(filename);
                if description != *filename && basename != description {
                    description.push_str(&format!("<br /><span class=\"nonessential\">({})</span>", esc_html(filename)));
                }
                let description = if current { format!("<span class=\"notice notice-info\">{description}</span>") } else { description };
                out.push_str(&format!(
                    "\t\t<li role=\"none\" class=\"{}\">\n\t\t\t<a role=\"treeitem\" tabindex=\"{}\"\n\t\t\t\thref=\"{}\"\n\t\t\t\taria-level=\"{level}\"\n\t\t\t\taria-setsize=\"{index}\"\n\t\t\t\taria-posinset=\"{size}\">\n\t\t\t\t{description}\t\t\t</a>\n\t\t</li>\n\t\t",
                    if current { "current-file" } else { "" },
                    if current { "0" } else { "-1" },
                    esc_url(&url)
                ));
            }
        }
    }
}

/// `wp_print_plugin_file_tree()`.
fn print_plugin_tree(ctx: &Ctx<'_>, tree: &[(String, Entry)], current: &str, plugin: &str, level: usize, out: &mut String) {
    let size = tree.len();
    for (position, (label, entry)) in tree.iter().enumerate() {
        let index = position + 1;
        match entry {
            Entry::Dir(children) => {
                out.push_str(&format!(
                    "\t\t\t<li role=\"treeitem\" aria-expanded=\"true\" tabindex=\"-1\"\n\t\t\t\taria-level=\"{level}\"\n\t\t\t\taria-setsize=\"{size}\"\n\t\t\t\taria-posinset=\"{index}\">\n\t\t\t\t<span class=\"folder-label\">{} <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span><span aria-hidden=\"true\" class=\"icon\"></span></span>\n\t\t\t\t<ul role=\"group\" class=\"tree-folder\">",
                    esc_html(label),
                    ctx.tr("folder")
                ));
                print_plugin_tree(ctx, children, current, plugin, level + 1, out);
                out.push_str("</ul>\n\t\t\t</li>\n\t\t\t");
            }
            Entry::File(plugin_file) => {
                let url = format!("{}?file={}&plugin={}", ctx.admin_url("plugin-editor.php"), rawurlencode(plugin_file), rawurlencode(plugin));
                let is_current = current == plugin_file;
                out.push_str(&format!(
                    "\t\t<li role=\"none\" class=\"{}\">\n\t\t\t<a role=\"treeitem\" tabindex=\"{}\"\n\t\t\t\thref=\"{}\"\n\t\t\t\taria-level=\"{level}\"\n\t\t\t\taria-setsize=\"{index}\"\n\t\t\t\taria-posinset=\"{size}\">\n\t\t\t\t{}{}\t\t\t</a>\n\t\t</li>\n\t\t",
                    if is_current { "current-file" } else { "" },
                    if is_current { "0" } else { "-1" },
                    esc_url(&url),
                    if is_current { "<span class=\"notice notice-info\">" } else { "" },
                    format!("{}{}", esc_html(label), if is_current { "</span>" } else { "" })
                ));
            }
        }
    }
}

/// `wp_doc_link_parse()`: the functions a PHP file calls, without the ones it defines itself.
fn doc_link_parse(content: &str) -> Vec<String> {
    let bytes = content.as_bytes();
    let mut index = 0;
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut functions: Vec<String> = Vec::new();
    let mut ignore: Vec<String> = Vec::new();
    let mut in_php = false;
    let is_ident_start = |b: u8| b.is_ascii_alphabetic() || b == b'_' || b >= 0x7f;
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b >= 0x7f;
    let keywords = [
        "abstract", "and", "array", "as", "break", "callable", "case", "catch", "class", "clone", "const", "continue", "declare", "default", "die", "do", "echo", "else", "elseif", "empty", "enddeclare", "endfor", "endforeach", "endif", "endswitch",
        "endwhile", "eval", "exit", "extends", "final", "finally", "fn", "for", "foreach", "function", "global", "goto", "if", "implements", "include", "include_once", "instanceof", "insteadof", "interface", "isset", "list", "match", "namespace", "new",
        "or", "print", "private", "protected", "public", "readonly", "require", "require_once", "return", "static", "switch", "throw", "trait", "try", "unset", "use", "var", "while", "xor", "yield",
    ];
    let mut previous: Vec<String> = Vec::new();
    let mut previous_object_operator = false;
    while index < bytes.len() {
        if !in_php {
            match content[index..].find("<?php").or_else(|| content[index..].find("<?=")) {
                Some(at) => {
                    index += at + if content[index + at..].starts_with("<?php") { 5 } else { 3 };
                    in_php = true;
                    continue;
                }
                None => break,
            }
        }
        let b = bytes[index];
        if content[index..].starts_with("?>") {
            in_php = false;
            index += 2;
            continue;
        }
        if b == b'#' || content[index..].starts_with("//") {
            index += content[index..].find('\n').unwrap_or(bytes.len() - index);
            continue;
        }
        if content[index..].starts_with("/*") {
            index += content[index..].find("*/").map_or(bytes.len() - index, |e| e + 2);
            continue;
        }
        if b == b'\'' || b == b'"' {
            index += 1;
            while index < bytes.len() && bytes[index] != b {
                if bytes[index] == b'\\' {
                    index += 1;
                }
                index += 1;
            }
            index += 1;
            previous.push(String::new());
            continue;
        }
        if content[index..].starts_with("->") {
            previous_object_operator = true;
            index += 2;
            continue;
        }
        if is_ident_start(b) {
            let start = index;
            while index < bytes.len() && is_ident(bytes[index]) {
                index += 1;
            }
            let word = &content[start..index];
            let after = content[index..].trim_start_matches([' ', '\t', '\r', '\n']);
            let is_call = after.starts_with('(') && !keywords.contains(&word.to_lowercase().as_str());
            if is_call {
                let defined = previous.last().is_some_and(|p| p == "function" || p == "class") || previous_object_operator;
                if defined {
                    ignore.push(word.to_owned());
                }
                functions.push(word.to_owned());
            }
            previous_object_operator = false;
            previous.push(word.to_owned());
            tokens.push((start, index));
            continue;
        }
        if !b.is_ascii_whitespace() {
            previous_object_operator = false;
        }
        index += 1;
    }
    functions.sort();
    functions.dedup();
    functions.into_iter().filter(|f| !ignore.contains(f)).collect()
}

/// `esc_textarea()`.
fn esc_textarea(text: &str) -> String {
    super::post_edit::esc_textarea_pub(text)
}

/// `wp_get_code_editor_settings()` for a file.
fn code_editor_settings(ctx: &Ctx<'_>, file: &str) -> serde_json::Value {
    use serde_json::{Map, Value as Json, json};
    let site = ctx.site();
    let mut codemirror = json!({
        "indentUnit": 4,
        "indentWithTabs": true,
        "inputStyle": "contenteditable",
        "lineNumbers": true,
        "lineWrapping": true,
        "styleActiveLine": true,
        "continueComments": true,
        "extraKeys": {"Ctrl-Space": "autocomplete", "Ctrl-/": "toggleComment", "Cmd-/": "toggleComment", "Alt-F": "findPersistent", "Ctrl-F": "findPersistent", "Cmd-F": "findPersistent"},
        "direction": "ltr",
        "gutters": []
    });
    let includes_url = format!("{}/wp-includes/", site.settings.siteurl.trim_end_matches('/'));
    let mut htmlhint = json!({
        "tagname-lowercase": true, "attr-lowercase": true, "attr-value-double-quotes": false, "doctype-first": false, "tag-pair": true, "spec-char-escape": true, "id-unique": true,
        "src-not-empty": true, "attr-no-duplication": true, "alt-require": true, "space-tab-mixed-disabled": "tab", "attr-unsafe-chars": true
    });
    let extension = if file.rsplit('/').next().is_some_and(|b| b.contains('.')) { file.rsplit('.').next().unwrap_or_default().to_lowercase() } else { String::new() };
    // `wp_get_mime_types()` first, then the types it does not have.
    let kind = match extension.as_str() {
        "css" => "text/css",
        "htm" | "html" => "text/html",
        "js" => "application/javascript",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "xml" => "application/xml",
        "conf" => "text/nginx",
        "diff" | "patch" => "text/x-diff",
        "http" => "message/http",
        "mjs" => "text/javascript",
        "jsx" => "text/jsx",
        "less" => "text/x-less",
        "md" => "text/x-gfm",
        "php" | "phtml" | "php3" | "php4" | "php5" | "php7" | "phps" => "application/x-httpd-php",
        "scss" => "text/x-scss",
        "sass" => "text/x-sass",
        "sh" | "bash" => "text/x-sh",
        "sql" => "text/x-sql",
        "yml" | "yaml" => "text/x-yaml",
        _ => "text/plain",
    };
    let mut merge = |extra: Json| {
        if let (Some(base), Some(extra)) = (codemirror.as_object_mut(), extra.as_object()) {
            for (k, v) in extra {
                base.insert(k.clone(), v.clone());
            }
        }
    };
    match kind {
        "text/css" | "text/x-scss" | "text/x-less" | "text/x-sass" => merge(json!({"mode": kind, "lint": false, "autoCloseBrackets": true, "matchBrackets": true})),
        "text/x-diff" => merge(json!({"mode": "diff"})),
        "text/html" => {
            merge(json!({"mode": "htmlmixed", "lint": true, "autoCloseBrackets": true, "autoCloseTags": true, "matchTags": {"bothTags": true}}));
            let _ = &mut htmlhint;
        }
        "text/x-gfm" => merge(json!({"mode": "gfm", "highlightFormatting": true})),
        "application/javascript" | "text/javascript" => merge(json!({"mode": "javascript", "lint": true, "autoCloseBrackets": true, "matchBrackets": true})),
        k if k.contains("json") => merge(json!({"mode": {"name": "javascript", "json": true}, "lint": true, "autoCloseBrackets": true, "matchBrackets": true})),
        k if k.contains("jsx") => merge(json!({"mode": "jsx", "autoCloseBrackets": true, "matchBrackets": true})),
        "text/nginx" => merge(json!({"mode": "nginx"})),
        "application/x-httpd-php" => merge(json!({"mode": "php", "autoCloseBrackets": true, "autoCloseTags": true, "matchBrackets": true, "matchTags": {"bothTags": true}})),
        "text/x-sql" | "text/x-mysql" => merge(json!({"mode": "sql", "autoCloseBrackets": true, "matchBrackets": true})),
        k if k.contains("xml") => merge(json!({"mode": "xml", "autoCloseBrackets": true, "autoCloseTags": true, "matchTags": {"bothTags": true}})),
        "text/x-yaml" => merge(json!({"mode": "yaml"})),
        other => merge(json!({"mode": other})),
    }
    if codemirror.get("lint").and_then(Json::as_bool).unwrap_or(false)
        && let Some(gutters) = codemirror.get_mut("gutters").and_then(Json::as_array_mut)
    {
        gutters.push(json!("CodeMirror-lint-markers"));
    }
    let mut jshint = Map::new();
    jshint.insert("esversion".into(), json!(11));
    jshint.insert("module".into(), json!(file.ends_with(".mjs")));
    jshint.insert("espreeModuleUrl".into(), json!(format!("{includes_url}js/codemirror/espree.min.js?ver=9.6.1")));
    for (k, v) in [("boss", json!(true)), ("curly", json!(true)), ("eqeqeq", json!(true)), ("eqnull", json!(true)), ("expr", json!(true)), ("immed", json!(true)), ("noarg", json!(true)), ("nonbsp", json!(true)), ("quotmark", json!("single")), ("undef", json!(true)), ("unused", json!(true)), ("browser", json!(true))] {
        jshint.insert(k.into(), v);
    }
    jshint.insert(
        "globals".into(),
        json!({"_": false, "Backbone": false, "jQuery": false, "JSON": false, "wp": false, "export": false, "module": false, "require": false, "WorkerGlobalScope": false, "self": false, "OffscreenCanvas": false, "Promise": false}),
    );
    json!({
        "codemirror": codemirror,
        "csslint": {"errors": true, "box-model": true, "display-property-grouping": true, "duplicate-properties": true, "known-properties": true, "outline-none": true},
        "jshint": Json::Object(jshint),
        "htmlhint": htmlhint
    })
}

/// `wp_json_encode( $value, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`.
fn json_hex_tag(value: &serde_json::Value) -> String {
    let mut out = String::new();
    for c in value.to_string().chars() {
        match c {
            '<' => out.push_str("\\u003C"),
            '>' => out.push_str("\\u003E"),
            c if c.is_ascii() => out.push(c),
            c => {
                let mut units = [0u16; 2];
                for unit in c.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
        }
    }
    out
}

/// The scripts, styles and settings of the editor page (`wp_enqueue_code_editor()`).
fn editor_assets(ctx: &Ctx<'_>, file: &str, kind: &str) -> Assets {
    let settings = code_editor_settings(ctx, file);
    let mode = settings["codemirror"]["mode"].clone();
    let mode_name = mode.as_str().map(str::to_owned).or_else(|| mode["name"].as_str().map(str::to_owned)).unwrap_or_default();
    let mut assets = Assets { scripts: vec!["code-editor".into(), "wp-theme-plugin-editor".into()], styles: vec!["code-editor".into()], ..Default::default() };
    if settings["codemirror"]["lint"].as_bool().unwrap_or(false) {
        match mode_name.as_str() {
            "css" | "text/css" | "text/x-scss" | "text/x-less" => assets.scripts.insert(1, "csslint".into()),
            "htmlmixed" | "text/html" | "php" | "application/x-httpd-php" | "text/x-php" => {
                assets.scripts.insert(1, "htmlhint".into());
                assets.scripts.insert(2, "csslint".into());
            }
            "javascript" | "application/ecmascript" | "application/json" | "application/javascript" | "application/ld+json" | "text/typescript" | "application/typescript" => assets.scripts.insert(1, "jsonlint".into()),
            _ => {}
        }
    }
    assets.inline.push(("code-editor".into(), format!("jQuery.extend( wp.codeEditor.defaultSettings, {} );", json_hex_tag(&settings)), "after".into()));
    let page_settings = serde_json::json!({"codeEditor": settings});
    assets.inline.push(("wp-theme-plugin-editor".into(), format!("jQuery( function( $ ) {{ wp.themePluginEditor.init( $( \"#template\" ), {} ); }} )", json_hex_tag(&page_settings)), "after".into()));
    assets.inline.push(("wp-theme-plugin-editor".into(), format!("wp.themePluginEditor.themeOrPlugin = \"{kind}\";"), "after".into()));
    assets
}

/// The warning dialog of the file editors, until it is dismissed.
async fn warning_dialog(ctx: &Ctx<'_>, pointer: &str, theme: Option<&ThemeInfo>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let dismissed = ctx.env.db.user_meta(ctx.user.id, "dismissed_wp_pointers").await.ok().flatten().unwrap_or_default();
    if dismissed.split(',').any(|p| p == pointer) {
        return String::new();
    }
    let return_url = super::common::wp_get_referer(ctx)
        .filter(|r| !r.contains("theme-editor.php") && !r.contains("plugin-editor.php") && !r.contains("wp-login.php"))
        .unwrap_or_else(|| ctx.admin_url(""));
    let mut h = String::from("\t<div id=\"file-editor-warning\" class=\"notification-dialog-wrap file-editor-warning hide-if-no-js hidden\">\n\t\t<div class=\"notification-dialog-background\"></div>\n\t\t<div class=\"notification-dialog\">\n\t\t\t<div class=\"file-editor-warning-content\">\n\t\t\t\t<div class=\"file-editor-warning-message\">\n");
    h.push_str(&format!("\t\t\t\t\t<h1>{}</h1>\n", tr("Heads up!")));
    if let Some(theme) = theme {
        h.push_str(&format!(
            "\t\t\t\t\t<p>\n\t\t\t\t\t\t{}\t\t\t\t\t</p>\n\t\t\t\t\t\t",
            tr("You appear to be making direct edits to your theme in the WordPress dashboard. It is not recommended! Editing your theme directly could break your site and your changes may be lost in future updates.")
        ));
        if !theme.has_parent() {
            h.push_str(&format!(
                "<p>{}</p>",
                sprintf(&tr("If you need to tweak more than your theme&#8217;s CSS, you might want to try <a href=\"%s\">making a child theme</a>."), &[&esc_url(&tr("https://developer.wordpress.org/themes/advanced-topics/child-themes/"))])
            ));
        }
        h.push_str(&format!(
            "\t\t\t\t\t<p>{}</p>\n\t\t\t\t</div>\n",
            tr("If you decide to go ahead with direct edits anyway, use a file manager to create a copy with a new name and hang on to the original. That way, you can re-enable a functional version if something goes wrong.")
        ));
    } else {
        h.push_str(&format!(
            "\t\t\t\t\t<p>{}</p>\n\t\t\t\t\t<p>{}</p>\n\t\t\t\t</div>\n",
            tr("You appear to be making direct edits to your plugin in the WordPress dashboard. Editing plugins directly is not recommended as it may introduce incompatibilities that break your site and your changes may be lost in future updates."),
            tr("If you absolutely have to make direct edits to this plugin, use a file manager to create a copy with a new name and hang on to the original. That way, you can re-enable a functional version if something goes wrong.")
        ));
    }
    h.push_str(&format!(
        "\t\t\t\t<p>\n\t\t\t\t\t<a class=\"button file-editor-warning-go-back\" href=\"{}\">{}</a>\n\t\t\t\t\t<button type=\"button\" class=\"file-editor-warning-dismiss button button-primary\">{}</button>\n\t\t\t\t</p>\n\t\t\t</div>\n\t\t</div>\n\t</div>\n\t",
        esc_url(&return_url),
        tr("Go back"),
        tr("I understand")
    ));
    h
}

fn editor_notices_script() -> String {
    String::new()
}

/// `wp_print_file_editor_templates()`.
fn editor_templates(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    format!(
        "\t<script type=\"text/html\" id=\"tmpl-wp-file-editor-notice\">\n\t\t<div class=\"notice inline notice-{{{{ data.type || 'info' }}}} {{{{ data.alt ? 'notice-alt' : '' }}}} {{{{ data.dismissible ? 'is-dismissible' : '' }}}} {{{{ data.classes || '' }}}}\">\n\t\t\t<# if ( 'php_error' === data.code ) {{ #>\n\t\t\t\t<p>\n\t\t\t\t\t{}\t\t\t\t</p>\n\t\t\t\t<pre>{{{{ data.message }}}}</pre>\n\t\t\t<# }} else if ( 'file_not_writable' === data.code ) {{ #>\n\t\t\t\t<p>\n\t\t\t\t\t{}\t\t\t\t</p>\n\t\t\t<# }} else {{ #>\n\t\t\t\t<p>{{{{ data.message || data.code }}}}</p>\n\n\t\t\t\t<# if ( 'lint_errors' === data.code ) {{ #>\n\t\t\t\t\t<p>\n\t\t\t\t\t\t<# var elementId = 'el-' + String( Math.random() ); #>\n\t\t\t\t\t\t<input id=\"{{{{ elementId }}}}\"  type=\"checkbox\">\n\t\t\t\t\t\t<label for=\"{{{{ elementId }}}}\">{}</label>\n\t\t\t\t\t</p>\n\t\t\t\t<# }} #>\n\t\t\t<# }} #>\n\t\t\t<# if ( data.dismissible ) {{ #>\n\t\t\t\t<button type=\"button\" class=\"notice-dismiss\"><span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></button>\n\t\t\t<# }} #>\n\t\t</div>\n\t</script>\n\t",
        sprintf(&tr("Your PHP code changes were not applied due to an error on line %1$s of file %2$s. Please fix and try saving again."), &["{{ data.line }}", "{{ data.file }}"]),
        sprintf(&tr("You need to make this file writable before you can save your changes. See <a href=\"%s\">Changing File Permissions</a> for more information."), &[&tr("https://developer.wordpress.org/advanced-administration/server/file-permissions/")]),
        tr("Update anyway, even though it might break your site?"),
        tr("Dismiss")
    )
}

fn doc_select(ctx: &Ctx<'_>, functions: &[String]) -> String {
    let mut select = format!("<select name=\"docs-list\" id=\"docs-list\"><option value=\"\">{}</option>", esc_html(&ctx.tr("Function Name&hellip;")));
    for function in functions {
        select.push_str(&format!("<option value=\"{}\">{}()</option>", esc_attr(function), esc_html(function)));
    }
    select.push_str("</select>");
    select
}

fn documentation(ctx: &Ctx<'_>, select: &str, indent: &str, inner: &str) -> String {
    let version = ctx.site().options.string("wp_version_reference", "7.1.3");
    let version = if version.is_empty() { "7.1.3".to_owned() } else { version };
    let outer = &indent[..indent.len() - 1];
    format!(
        "{indent}<div id=\"documentation\" class=\"hide-if-no-js\">\n{inner}<label for=\"docs-list\">{}</label>\n{inner}{select}{inner}<input disabled id=\"docs-lookup\" type=\"button\" class=\"button\" value=\"{}\" onclick=\"if ( '' !== jQuery('#docs-list').val() ) {{ window.open( 'https://api.wordpress.org/core/handbook/1.0/?function=' + escape( jQuery( '#docs-list' ).val() ) + '&amp;locale={}&amp;version={}&amp;redirect=true'); }}\" />\n{indent}</div>\n{outer}",
        ctx.tr("Documentation:"),
        esc_attr(&ctx.tr("Look Up")),
        rawurlencode(&ctx.site().locale()),
        rawurlencode(&version)
    )
}

fn help_theme(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let content = format!(
        "<p>{}</p><p>{}</p><p>{}</p><p id=\"editor-keyboard-trap-help-1\">{}</p><ul><li id=\"editor-keyboard-trap-help-2\">{}</li><li id=\"editor-keyboard-trap-help-3\">{}</li><li id=\"editor-keyboard-trap-help-4\">{}</li></ul><p>{}</p><p>{}</p><p>{}</p>",
        tr("You can use the theme file editor to edit the individual CSS and PHP files which make up your theme."),
        tr("Begin by choosing a theme to edit from the dropdown menu and clicking the Select button. A list then appears of the theme&#8217;s template files. Clicking once on any file name causes the file to appear in the large Editor box."),
        tr("For PHP files, you can use the documentation dropdown to select from functions recognized in that file. Look Up takes you to a web page with reference material about that particular function."),
        tr("When using a keyboard to navigate:"),
        tr("In the editing area, the Tab key enters a tab character."),
        tr("To move away from this area, press the Esc key followed by the Tab key."),
        tr("Screen reader users: when in forms mode, you may need to press the Esc key twice."),
        tr("After typing in your edits, click Update File."),
        tr("<strong>Advice:</strong> Think very carefully about your site crashing if you are live-editing the theme currently in use."),
        sprintf(&tr("Upgrading to a newer version of the same theme will override changes made here. To avoid this, consider creating a <a href=\"%s\">child theme</a> instead."), &[&tr("https://developer.wordpress.org/themes/advanced-topics/child-themes/")])
    );
    let sidebar = format!(
        "<p><strong>{}</strong></p><p>{}</p><p>{}</p><p>{}</p><p>{}</p><p>{}</p>",
        tr("For more information:"),
        tr("<a href=\"https://developer.wordpress.org/themes/\">Documentation on Theme Development</a>"),
        tr("<a href=\"https://wordpress.org/documentation/article/appearance-theme-file-editor-screen/\">Documentation on Editing Themes</a>"),
        tr("<a href=\"https://developer.wordpress.org/advanced-administration/wordpress/edit-files/\">Documentation on Editing Files</a>"),
        tr("<a href=\"https://developer.wordpress.org/themes/basics/template-tags/\">Documentation on Template Tags</a>"),
        tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
    );
    Help { tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content }], sidebar }
}

fn help_plugin(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let content = format!(
        "<p>{}</p><p>{}</p><p>{}</p><p id=\"editor-keyboard-trap-help-1\">{}</p><ul><li id=\"editor-keyboard-trap-help-2\">{}</li><li id=\"editor-keyboard-trap-help-3\">{}</li><li id=\"editor-keyboard-trap-help-4\">{}</li></ul><p>{}</p>",
        tr("You can use the plugin file editor to make changes to any of your plugins&#8217; individual PHP files. Be aware that if you make changes, plugins updates will overwrite your customizations."),
        tr("Choose a plugin to edit from the dropdown menu and click the Select button. Click once on any file name to load it in the editor, and make your changes. Do not forget to save your changes (Update File) when you are finished."),
        tr("The documentation menu below the editor lists the PHP functions recognized in the plugin file. Clicking Look Up takes you to a web page about that particular function."),
        tr("When using a keyboard to navigate:"),
        tr("In the editing area, the Tab key enters a tab character."),
        tr("To move away from this area, press the Esc key followed by the Tab key."),
        tr("Screen reader users: when in forms mode, you may need to press the Esc key twice."),
        tr("If you want to make changes but do not want them to be overwritten when the plugin is updated, you may be ready to think about writing your own plugin. For information on how to edit plugins, write your own from scratch, or just better understand their anatomy, check out the links below.")
    );
    let sidebar = format!(
        "<p><strong>{}</strong></p><p>{}</p><p>{}</p><p>{}</p>",
        tr("For more information:"),
        tr("<a href=\"https://developer.wordpress.org/advanced-administration/plugins/editor-screen/\">Documentation on Editing Plugins</a>"),
        tr("<a href=\"https://developer.wordpress.org/plugins/\">Documentation on Writing Plugins</a>"),
        tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
    );
    Help { tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content }], sidebar }
}

fn error_message(code: &str, message: &str) -> String {
    esc_html(if message.is_empty() { code } else { message })
}

fn sanitize_text_field(text: &str) -> String {
    rustpress_core::formatting::strip_tags(text).replace(['\r', '\n', '\t'], " ").trim().to_owned()
}

/// `theme-editor.php`.
pub async fn theme_editor(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    if !ctx.can("edit_themes") {
        return die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit templates for this site.")));
    }
    let Some(root) = wp_root(ctx.env) else { return die(ctx.env, 500, "WordPress &rsaquo; Error", "<p>The WordPress files are not available.</p>") };
    let site = ctx.site();
    let active = site.options.string("stylesheet", "");
    let requested = request.request("theme").map(sanitize_text_field).unwrap_or_default();
    let stylesheet = if requested.is_empty() { active.clone() } else { requested };
    let Some(theme) = themes::find(&root, &stylesheet) else { return super::common::die_plain(ctx, "The requested theme does not exist.") };
    let allowed = theme_files(&theme);
    let file_param = request.request("file").map(sanitize_text_field).unwrap_or_default();
    let (relative_file, file): (String, PathBuf) = if file_param.is_empty() {
        let Some((_, path)) = allowed.iter().find(|(k, _)| k == "style.css") else { return super::common::die_plain(ctx, "Sorry, that file cannot be edited.") };
        ("style.css".to_owned(), path.clone())
    } else {
        (file_param.clone(), theme.dir.join(&file_param))
    };
    // `validate_file_to_edit()`: the file has to be one of the allowed ones.
    let allowed_names: Vec<String> = allowed.iter().map(|(k, _)| k.clone()).collect();
    if validate_file(&relative_file, &allowed_names) != 0 {
        return super::common::die_plain(ctx, "Sorry, that file cannot be edited.");
    }

    // The fallback of a browser without JavaScript.
    let mut edit_error: Option<EditError> = None;
    let mut posted_content: Option<String> = None;
    if request.post {
        let field = |name: &str| request.post_field(name).map(str::to_owned);
        match edit_file(ctx.env, request, &ctx.user, &ctx.token, &field).await {
            Err(e) => {
                if ctx.env.auth.verify_nonce(request.post_field("nonce").unwrap_or_default(), &format!("edit-theme_{stylesheet}_{relative_file}"), ctx.user.id, &ctx.token, ctx.env.now).is_some() {
                    posted_content = request.post_field("newcontent").map(str::to_owned);
                }
                edit_error = Some(e);
            }
            Ok(()) => {
                let location = format!("{}?a=1&theme={}&file={}", ctx.admin_url("theme-editor.php"), rawurlencode(&stylesheet), rawurlencode(&relative_file));
                return redirect(&location);
            }
        }
    }

    let mut error = request.request("error").is_some_and(|e| !e.is_empty());
    if !file.is_file() {
        error = true;
    }
    let mut content = String::new();
    let mut functions: Vec<String> = Vec::new();
    if let Some(posted) = posted_content.filter(|p| !p.is_empty()) {
        content = posted;
    } else if !error && let Ok(bytes) = std::fs::read(&file) && !bytes.is_empty() {
        content = String::from_utf8_lossy(&bytes).into_owned();
        if file.to_string_lossy().ends_with(".php") {
            functions = doc_link_parse(&content);
        }
        content = esc_textarea(&content);
    }
    let file_show = allowed.iter().find(|(_, p)| *p == file).map(|(k, _)| k.clone()).unwrap_or_default();

    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", esc_html(&tr("Edit Themes")));
    if request.get("a").is_some() {
        h.push_str(notice(&tr("File edited successfully."), "", true, "message", &["updated"], true).trim_end_matches('\n'));
    } else if let Some(e) = &edit_error {
        let message = format!("<p>{}</p>\n\t\t<pre>{}</pre>", tr("There was an error while trying to update the file. You may need to fix something and try updating again."), error_message(e.code, &e.message));
        h.push_str(notice(&message, "error", false, "message", &[], true).trim_end_matches('\n'));
    }
    let active_theme = themes::find(&root, &active);
    let active_block = active_theme.as_ref().is_some_and(|t| t.block_theme);
    if file.to_string_lossy().ends_with(".css") {
        let info = |url: &str| {
            let message = format!(
                "<p><strong>{}</strong></p><p>{}</p>",
                tr("Did you know?"),
                sprintf(&tr("There is no need to change your CSS here &mdash; you can edit and live preview CSS changes in the <a href=\"%s\">built-in CSS editor</a>."), &[&esc_url(url).replace("&#038;", "&amp;")])
            );
            notice(&message, "info", false, "message", &[], true).trim_end_matches('\n').to_owned()
        };
        if !active_block && ctx.can("customize") {
            h.push_str(&info(&format!("{}?autofocus%5Bsection%5D=custom_css", ctx.admin_url("customize.php"))));
        } else if active_block && ctx.can("edit_theme_options") {
            h.push_str(&info(&format!("{}?p=%2Fstyles&section=%2Fcss", ctx.admin_url("site-editor.php"))));
        }
        let minified = Path::new(&file.to_string_lossy().trim_end_matches(".css").to_owned()).with_extension("").to_string_lossy().into_owned() + ".min.css";
        if Path::new(&minified).exists() {
            let message = format!("<p><strong>{}</strong></p><p>{}</p>", tr("There is a minified version of this stylesheet."), tr("It is likely that this unminified stylesheet will not be served to visitors."));
            h.push_str(notice(&message, "warning", false, "wp-css-min-warning", &[], true).trim_end_matches('\n'));
        }
    }
    let is_active_name = active_theme.as_ref().is_some_and(|t| t.name == theme.name);
    let name = format!("<strong>{}</strong>", wptexturize(&theme.name));
    h.push_str(&format!(
        "\n<div class=\"fileedit-sub\">\n<div class=\"alignleft\">\n<h2>\n\t{}</h2>\n <span><strong>{}</strong></span></div>\n<div class=\"alignright\">\n\t<form action=\"theme-editor.php\" method=\"get\">\n\t\t<label for=\"theme\" id=\"theme-plugin-editor-selector\">{} </label>\n\t\t<select name=\"theme\" id=\"theme\">\n\t\t",
        sprintf(&tr(if is_active_name { "Editing %s (active)" } else { "Editing %s (inactive)" }), &[&name]),
        sprintf(&tr("File: %s"), &[&esc_html(&file_show)]),
        tr("Select theme to edit:")
    ));
    for a_theme in themes::installed(&root) {
        h.push_str(&format!(
            "\n\t<option value=\"{}\"{}>{}</option>",
            esc_attr(&a_theme.stylesheet),
            if a_theme.stylesheet == stylesheet { " selected=\"selected\"" } else { "" },
            wptexturize(&a_theme.name)
        ));
    }
    h.push_str(&format!(
        "\t\t</select>\n\t\t{}\t</form>\n</div>\n<br class=\"clear\" />\n</div>\n\n\n<div id=\"templateside\">\n\t<h2 id=\"theme-files-label\">{}</h2>\n\t<ul role=\"tree\" aria-labelledby=\"theme-files-label\">\n\t\t",
        format!("<input type=\"submit\" name=\"Submit\" id=\"Submit\" class=\"button\" value=\"{}\"  />", esc_attr(&tr("Select"))),
        tr("Theme Files")
    ));
    if theme.has_parent() {
        let parent_name = themes::find(&root, &theme.template).map(|p| p.name).unwrap_or_default();
        h.push_str(&format!(
            "\t\t\t<li class=\"howto\">\n\t\t\t\t{}\t\t\t</li>\n\t\t",
            sprintf(&tr("This child theme inherits templates from a parent theme, %s."), &[&format!("<a href=\"{}\">{}</a>", ctx.admin_url(&format!("theme-editor.php?theme={}", urlencode(&theme.template))), wptexturize(&parent_name))])
        ));
    }
    let mut tree: Vec<(String, Entry)> = Vec::new();
    for (name, _) in &allowed {
        let parts: Vec<&str> = name.split('/').collect();
        insert(&mut tree, &parts, name);
    }
    h.push_str("\t\t<li role=\"treeitem\" tabindex=\"-1\" aria-expanded=\"true\" aria-level=\"1\" aria-posinset=\"1\" aria-setsize=\"1\">\n\t\t\t<ul role=\"group\">\n\t\t\t\t");
    print_theme_tree(ctx, &tree, &allowed, &relative_file, &stylesheet, 2, &mut h);
    h.push_str("\t\t\t</ul>\n\t\t</li>\n\t</ul>\n</div>\n\n");
    if error {
        h.push_str(&notice(&tr("File does not exist! Please double check the name and try again."), "", false, "", &["error"], true));
    } else {
        let writable = std::fs::metadata(&file).is_ok_and(|m| !m.permissions().readonly());
        let child_notice = if active_theme.as_ref().is_some_and(|t| t.has_parent()) && theme.stylesheet == active_theme.as_ref().map_or("", |t| t.template.as_str()) {
            let message = format!("{}{}", if writable { format!("<strong>{}</strong> ", tr("Caution:")) } else { String::new() }, tr("This is a file in your current parent theme."));
            notice(&message, "warning", false, "", &["inline"], true)
        } else {
            String::new()
        };
        h.push_str(&format!(
            "\t<form name=\"template\" id=\"template\" action=\"theme-editor.php\" method=\"post\">\n\t\t{}\t\t<div>\n\t\t\t<label for=\"newcontent\" id=\"theme-plugin-editor-label\">{}</label>\n\t\t\t<textarea cols=\"70\" rows=\"30\" name=\"newcontent\" id=\"newcontent\" aria-describedby=\"editor-keyboard-trap-help-1 editor-keyboard-trap-help-2 editor-keyboard-trap-help-3 editor-keyboard-trap-help-4\">{content}</textarea>\n\t\t\t<input type=\"hidden\" name=\"action\" value=\"update\" />\n\t\t\t<input type=\"hidden\" name=\"file\" value=\"{}\" />\n\t\t\t<input type=\"hidden\" name=\"theme\" value=\"{}\" />\n\t\t</div>\n\n\t\t",
            ctx.nonce_field_named(&format!("edit-theme_{stylesheet}_{relative_file}"), "nonce"),
            tr("Selected file content:"),
            esc_attr(&relative_file),
            esc_attr(&theme.stylesheet)
        ));
        if !functions.is_empty() {
            h.push_str(&documentation(ctx, &doc_select(ctx, &functions), "\t\t\t", "\t\t\t\t"));
        }
        h.push_str(&format!("\n\t\t<div>\n\t\t\t<div class=\"editor-notices\">\n\t\t\t\t{child_notice}\t\t\t</div>\n\t\t\t"));
        if writable {
            h.push_str(&format!(
                "\t\t\t\t<p class=\"submit\">\n\t\t\t\t\t{}\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t</p>\n\t\t\t\t",
                crate::list_table::submit_button(&tr("Update File"), "primary", "submit", "")
            ));
        } else {
            h.push_str(&format!(
                "\t\t\t\t<p>\n\t\t\t\t\t{}\t\t\t\t</p>\n\t\t\t\t",
                sprintf(&tr("You need to make this file writable before you can save your changes. See <a href=\"%s\">Changing File Permissions</a> for more information."), &[&tr("https://developer.wordpress.org/advanced-administration/server/file-permissions/")])
            ));
        }
        h.push_str("\t\t</div>\n\n\t\t");
        h.push_str(&editor_templates(ctx));
        h.push_str("\t</form>\n\t");
    }
    h.push_str("<br class=\"clear\" />\n</div>\n");
    h.push_str(&warning_dialog(ctx, "theme_editor_notice", Some(&theme)).await);
    let screen = Screen { file: "theme-editor.php".into(), content_inline: true, parent_file: "tools.php".into(), submenu_file: Some("theme-editor.php".into()), title: tr("Edit Themes"), help: Some(help_theme(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &editor_assets(ctx, &file.to_string_lossy(), "theme")).await
}

fn urlencode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `plugin-editor.php`.
pub async fn plugin_editor(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let request = ctx.request;
    if !ctx.can("edit_plugins") {
        return super::common::die_plain(ctx, "Sorry, you are not allowed to edit plugins for this site.");
    }
    let Some(root) = wp_root(ctx.env) else { return die(ctx.env, 500, "WordPress &rsaquo; Error", "<p>The WordPress files are not available.</p>") };
    let plugins: Vec<PluginInfo> = plugin_scan::installed(&root);
    let screen = |title: String, help: Option<Help>| Screen { file: "plugin-editor.php".into(), content_inline: true, parent_file: "tools.php".into(), submenu_file: Some("plugin-editor.php".into()), title, help, ..Default::default() };
    if plugins.is_empty() {
        let h = format!("<div class=\"wrap\">\n\t\t<h1>{}</h1>\n\t\t{}\t</div>\n", esc_html(&tr("Edit Plugins")), notice(&tr("No plugins are currently available."), "", false, "message", &["error"], true));
        return render(ctx, &screen(tr("Edit Plugins"), None), &h, &Assets::default()).await;
    }
    let mut file = request.request("file").unwrap_or_default().to_owned();
    let mut plugin = request.request("plugin").unwrap_or_default().to_owned();
    if plugin.is_empty() {
        if !file.is_empty() {
            let file_dirname = file.rsplit_once('/').map_or(".", |(d, _)| d).to_owned();
            for candidate in &plugins {
                let candidate_dir = candidate.file.rsplit_once('/').map_or(".", |(d, _)| d);
                if candidate.file == file || (file_dirname != "." && candidate_dir == file_dirname) {
                    plugin = candidate.file.clone();
                    break;
                }
            }
            if plugin.is_empty() {
                plugin = file.clone();
            }
        } else {
            plugin = plugins[0].file.clone();
        }
    }
    let files = plugin_files(&root, &plugin);
    if file.is_empty() {
        file = files[0].clone();
    }
    if validate_file(&file, &files) != 0 {
        return super::common::die_plain(ctx, "Sorry, that file cannot be edited.");
    }
    let real_file = root.join("wp-content").join("plugins").join(&file);
    let plugin_name = plugins.iter().find(|p| p.file == files[0]).map_or_else(|| files[0].clone(), |p| p.name.clone());

    let mut edit_error: Option<EditError> = None;
    let mut posted_content: Option<String> = None;
    if request.post {
        let field = |name: &str| request.post_field(name).map(str::to_owned);
        match edit_file(ctx.env, request, &ctx.user, &ctx.token, &field).await {
            Err(e) => {
                if ctx.env.auth.verify_nonce(request.post_field("nonce").unwrap_or_default(), &format!("edit-plugin_{file}"), ctx.user.id, &ctx.token, ctx.env.now).is_some() {
                    posted_content = request.post_field("newcontent").map(str::to_owned);
                }
                edit_error = Some(e);
            }
            Ok(()) => {
                let location = format!("{}?a=1&plugin={}&file={}", ctx.admin_url("plugin-editor.php"), rawurlencode(&plugin), rawurlencode(&file));
                return redirect(&location);
            }
        }
    }
    if !real_file.is_file() {
        return super::common::die_plain_html(ctx, &format!("<p>{}</p>", tr("File does not exist! Please double check the name and try again.")));
    }
    if let Some(extension) = extension_of(&real_file.to_string_lossy()).map(|e| e.to_lowercase())
        && !EDITABLE_TYPES.contains(&extension.as_str())
    {
        return super::common::die_plain_html(ctx, &format!("<p>{}</p>", tr("Files of this type are not editable.")));
    }
    let content_raw = match posted_content.filter(|p| !p.is_empty()) {
        Some(posted) => posted,
        None => String::from_utf8_lossy(&std::fs::read(&real_file).unwrap_or_default()).into_owned(),
    };
    let functions = if real_file.to_string_lossy().ends_with(".php") { doc_link_parse(&content_raw) } else { Vec::new() };
    let content = esc_textarea(&content_raw);
    let writable = std::fs::metadata(&real_file).is_ok_and(|m| !m.permissions().readonly());
    let active_plugins: Vec<String> = match ctx.site().options.value("active_plugins") {
        Some(Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    };
    let is_active = active_plugins.contains(&plugin);

    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", esc_html(&tr("Edit Plugins")));
    if request.get("a").is_some() {
        h.push_str(&notice(&tr("File edited successfully."), "", false, "message", &["updated", "is-dismissible"], true));
    } else if let Some(e) = &edit_error {
        let message = format!("<p>{}</p>\n\t\t<pre>{}</pre>", tr("There was an error while trying to update the file. You may need to fix something and try updating again."), error_message(e.code, &e.message));
        h.push_str(&notice(&message, "error", false, "message", &[], false));
    }
    let heading = match (is_active, writable) {
        (true, true) => "Editing %s (active)",
        (true, false) => "Browsing %s (active)",
        (false, true) => "Editing %s (inactive)",
        (false, false) => "Browsing %s (inactive)",
    };
    h.push_str(&format!(
        "\n<div class=\"fileedit-sub\">\n<div class=\"alignleft\">\n<h2>\n\t{}</h2>\n <span><strong>{}</strong></span>\n</div>\n<div class=\"alignright\">\n\t<form action=\"plugin-editor.php\" method=\"get\">\n\t\t<label for=\"plugin\" id=\"theme-plugin-editor-selector\">{} </label>\n\t\t<select name=\"plugin\" id=\"plugin\">\n\t\t\t",
        sprintf(&tr(heading), &[&format!("<strong>{}</strong>", esc_html(&plugin_name))]),
        sprintf(&tr("File: %s"), &[&esc_html(&file)]),
        tr("Select plugin to edit:")
    ));
    for a_plugin in &plugins {
        h.push_str(&format!("\n\t<option value=\"{}\" {}>{}</option>", esc_attr(&a_plugin.file), if a_plugin.file == plugin { " selected='selected'" } else { "" }, esc_attr(&a_plugin.name)));
    }
    h.push_str(&format!(
        "\t\t</select>\n\t\t{}\t</form>\n</div>\n<br class=\"clear\" />\n</div>\n\n<div id=\"templateside\">\n\t<h2 id=\"plugin-files-label\">{}</h2>\n\n\t",
        format!("<input type=\"submit\" name=\"Submit\" id=\"Submit\" class=\"button\" value=\"{}\"  />", esc_attr(&tr("Select"))),
        tr("Plugin Files")
    ));
    let mut tree: Vec<(String, Entry)> = Vec::new();
    for plugin_file in files.iter().filter(|f| extension_of(f).is_some_and(|e| EDITABLE_TYPES.contains(&e.as_str()))) {
        // `preg_replace( '#^.+?/#', '', $plugin_file )`: without the folder of the plugin.
        let trimmed = plugin_file.split_once('/').map_or(plugin_file.as_str(), |(_, rest)| rest);
        let parts: Vec<&str> = trimmed.split('/').collect();
        insert(&mut tree, &parts, plugin_file);
    }
    h.push_str("\t<ul role=\"tree\" aria-labelledby=\"plugin-files-label\">\n\t<li role=\"treeitem\" tabindex=\"-1\" aria-expanded=\"true\" aria-level=\"1\" aria-posinset=\"1\" aria-setsize=\"1\">\n\t\t<ul role=\"group\">\n\t\t\t");
    print_plugin_tree(ctx, &tree, &file, &plugin, 2, &mut h);
    h.push_str("\t\t</ul>\n\t</li>\n\t</ul>\n</div>\n\n");
    h.push_str(&format!(
        "<form name=\"template\" id=\"template\" action=\"plugin-editor.php\" method=\"post\">\n\t{}\t<div>\n\t\t<label for=\"newcontent\" id=\"theme-plugin-editor-label\">{}</label>\n\t\t<textarea cols=\"70\" rows=\"25\" name=\"newcontent\" id=\"newcontent\" aria-describedby=\"editor-keyboard-trap-help-1 editor-keyboard-trap-help-2 editor-keyboard-trap-help-3 editor-keyboard-trap-help-4\">{content}</textarea>\n\t\t<input type=\"hidden\" name=\"action\" value=\"update\" />\n\t\t<input type=\"hidden\" name=\"file\" value=\"{}\" />\n\t\t<input type=\"hidden\" name=\"plugin\" value=\"{}\" />\n\t</div>\n\n\t",
        ctx.nonce_field_named(&format!("edit-plugin_{file}"), "nonce"),
        tr("Selected file content:"),
        esc_attr(&file),
        esc_attr(&plugin)
    ));
    if !functions.is_empty() {
        h.push_str(&documentation(ctx, &doc_select(ctx, &functions), "\t\t", "\t\t\t"));
    }
    h.push_str("\n\t");
    if writable {
        let warning = if active_plugins.contains(&plugin) {
            notice(&tr("<strong>Warning:</strong> Making changes to active plugins is not recommended."), "warning", false, "", &["inline", "active-plugin-edit-warning"], true)
        } else {
            String::new()
        };
        h.push_str(&format!(
            "\t\t<div class=\"editor-notices\">\n\t\t{warning}\t\t</div>\n\t\t<p class=\"submit\">\n\t\t\t{}\t\t\t<span class=\"spinner\"></span>\n\t\t</p>\n\t",
            crate::list_table::submit_button(&tr("Update File"), "primary", "submit", "")
        ));
    } else {
        h.push_str(&format!(
            "\t\t<p>\n\t\t\t{}\t\t</p>\n\t",
            sprintf(&tr("You need to make this file writable before you can save your changes. See <a href=\"%s\">Changing File Permissions</a> for more information."), &[&tr("https://developer.wordpress.org/advanced-administration/server/file-permissions/")])
        ));
    }
    h.push_str("\n\t");
    h.push_str(&editor_templates(ctx));
    h.push_str("</form>\n<br class=\"clear\" />\n</div>\n");
    h.push_str(&warning_dialog(ctx, "plugin_editor_notice", None).await);
    let _ = editor_notices_script();
    render(ctx, &screen(tr("Edit Plugins"), Some(help_plugin(ctx))), &h, &editor_assets(ctx, &real_file.to_string_lossy(), "plugin")).await
}
