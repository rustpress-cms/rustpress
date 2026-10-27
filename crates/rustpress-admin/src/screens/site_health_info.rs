//! The Info tab of `site-health.php` (`WP_Debug_Data`): what the site is made of and what it runs on.

use std::path::{Path, PathBuf};

use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::Value;

use super::Ctx;
use crate::plugins as plugin_scan;
use crate::themes::{self, ThemeInfo};

/// The value of a field: text or a list of `name: value` pairs (WordPress has list fields; none of those RustPress
/// fills is one yet).
#[derive(Clone)]
#[allow(dead_code)]
enum Val {
    Text(String),
    List(Vec<(String, String)>),
}

/// What the clipboard text shows for a field instead of the value.
#[derive(Clone)]
#[allow(dead_code)]
enum Debug {
    Text(String),
    Bool(bool),
    List(Vec<(String, String)>),
}

struct Field {
    key: String,
    label: String,
    value: Val,
    debug: Option<Debug>,
    private: bool,
}

struct Section {
    id: &'static str,
    label: String,
    description: String,
    show_count: bool,
    fields: Vec<Field>,
}

fn field(key: &str, label: String, value: impl Into<String>) -> Field {
    Field { key: key.to_owned(), label, value: Val::Text(value.into()), debug: None, private: false }
}

impl Field {
    fn debug(mut self, debug: impl Into<String>) -> Self {
        self.debug = Some(Debug::Text(debug.into()));
        self
    }

    fn debug_bool(mut self, debug: bool) -> Self {
        self.debug = Some(Debug::Bool(debug));
        self
    }

    fn private(mut self) -> Self {
        self.private = true;
        self
    }
}

/// `WP_Debug_Data::format( $info, 'debug' )`: the text of the copy button.
fn format_debug(info: &[Section]) -> String {
    let mut out = String::from("`\n");
    for section in info {
        if section.fields.is_empty() {
            continue;
        }
        out.push_str(&format!("### {}{} ###\n\n", section.id, if section.show_count { format!(" ({})", section.fields.len()) } else { String::new() }));
        for field in &section.fields {
            if field.private {
                continue;
            }
            let value = match (&field.debug, &field.value) {
                (Some(Debug::List(items)), _) | (None, Val::List(items)) => items.iter().map(|(k, v)| format!("\n\t{k}: {v}")).collect::<String>(),
                (Some(Debug::Bool(b)), _) => (if *b { "true" } else { "false" }).to_owned(),
                (Some(Debug::Text(t)), _) | (None, Val::Text(t)) => {
                    if t.is_empty() { "undefined".to_owned() } else { t.clone() }
                }
            };
            out.push_str(&format!("{}: {value}\n", field.key));
        }
        out.push('\n');
    }
    out.push('`');
    out
}

fn timezone_string(ctx: &Ctx<'_>) -> String {
    let site = ctx.site();
    let zone = site.options.string("timezone_string", "");
    if !zone.is_empty() {
        return zone;
    }
    let offset: f64 = site.options.string("gmt_offset", "0").parse().unwrap_or(0.0);
    let sign = if offset < 0.0 { '-' } else { '+' };
    let abs = offset.abs();
    format!("{sign}{:02}:{:02}", abs.trunc() as u64, ((abs - abs.trunc()) * 60.0).round() as u64)
}

fn version_string(ctx: &Ctx<'_>, version: &str, author: &str) -> (String, String) {
    let tr = |s: &str| ctx.tr(s);
    match (version.is_empty(), author.is_empty()) {
        (false, false) => (sprintf(&tr("Version %1$s by %2$s"), &[version, author]), format!("version: {version}, author: {author}")),
        (true, false) => (sprintf(&tr("By %s"), &[author]), format!("author: {author}, version: (undefined)")),
        (false, true) => (sprintf(&tr("Version %s"), &[version]), format!("author: (undefined), version: {version}")),
        (true, true) => (tr("No version or author information is available."), "author: (undefined), version: (undefined)".to_owned()),
    }
}

fn option_list(ctx: &Ctx<'_>, name: &str) -> Vec<String> {
    match ctx.site().options.value(name) {
        Some(Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_owned()
}

fn writable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| !m.permissions().readonly())
}

/// The directory for uploads and its fonts directory.
fn uploads_dir(ctx: &Ctx<'_>, root: &Path) -> PathBuf {
    let configured = ctx.site().options.string("upload_path", "");
    if configured.is_empty() {
        root.join("wp-content").join("uploads")
    } else if Path::new(&configured).is_absolute() {
        PathBuf::from(configured)
    } else {
        root.join(configured)
    }
}

fn theme_fields(ctx: &Ctx<'_>, theme: &ThemeInfo, parent: &ThemeInfo, auto: &[String], active: bool) -> Vec<Field> {
    let tr = |s: &str| ctx.tr(s);
    let name_value = sprintf(&tr("%1$s (%2$s)"), &[&theme.name, &theme.stylesheet]);
    let mut fields = vec![
        field("name", tr("Name"), name_value),
        field("version", tr("Version"), theme.version.clone()),
        field("author", tr("Author"), rustpress_core::formatting::strip_tags(&theme.author)),
        {
            let uri = theme.author_uri.clone();
            if uri.is_empty() { field("author_website", tr("Author website"), tr("Undefined")).debug("(undefined)") } else { field("author_website", tr("Author website"), uri) }
        },
    ];
    if active {
        if theme.has_parent() {
            let value = sprintf(&tr("%1$s (%2$s)"), &[&parent.name, &theme.template]);
            fields.push(field("parent_theme", tr("Parent theme"), value.clone()).debug(format!("{} ({})", parent.name, theme.template)));
        } else {
            fields.push(field("parent_theme", tr("Parent theme"), tr("None")).debug("none"));
        }
        let defaults: &[&str] = if theme.block_theme {
            &["core-block-patterns", "post-thumbnails", "responsive-embeds", "editor-styles", "html5", "automatic-feed-links", "widgets-block-editor", "block-templates"]
        } else {
            &["post-thumbnails", "html5", "automatic-feed-links", "title-tag"]
        };
        let mut features: Vec<String> = defaults.iter().map(|f| (*f).to_owned()).collect();
        // `$_wp_theme_features` gets the theme's own features after the defaults, in the order the theme adds them
        // (the child's `functions.php` runs before the parent's).
        let themes = ctx.env.loader.includes.parent().map(|root| root.join("wp-content").join("themes")).unwrap_or_default();
        let mut dirs = vec![themes.join(&theme.stylesheet)];
        if theme.has_parent() {
            dirs.push(themes.join(&theme.template));
        }
        for dir in dirs {
            let Some(php) = rustpress_core::files::read(&dir.join("functions.php")) else { continue };
            let calls = rustpress_core::formatting::cached_plain_regex(r#"(add_theme_support)\(\s*['"]([^'"]+)['"]|(add_editor_style)\("#);
            for found in calls.captures_iter(&php) {
                let feature = if found.get(3).is_some() { "editor-style".to_owned() } else { found[2].to_owned() };
                if !features.contains(&feature) {
                    features.push(feature);
                }
            }
        }
        fields.push(field("theme_features", tr("Theme features"), features.join(", ")));
    }
    fields.push(field("theme_path", tr("Theme directory location"), path_string(&theme.dir)));
    let enabled = auto.contains(&theme.stylesheet);
    fields.push(field("auto_update", tr(if active { "Auto-updates" } else { "Auto-update" }), tr(if enabled { "Enabled" } else { "Disabled" })).debug(tr(if enabled { "Enabled" } else { "Disabled" })));
    fields
}

/// The sections of the page.
async fn debug_data(ctx: &Ctx<'_>) -> Vec<Section> {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let root = ctx.env.loader.includes.parent().map(Path::to_path_buf).unwrap_or_default();
    let content = root.join("wp-content");
    let mut info: Vec<Section> = Vec::new();

    // WordPress
    let permalink = site.options.string("permalink_structure", "");
    let https = site.settings.siteurl.starts_with("https://");
    let registration = site.options.string("users_can_register", "0") == "1";
    let blog_public = site.options.string("blog_public", "1") != "0";
    let comment_status = site.options.string("default_comment_status", "open");
    let environment = site.environment_type();
    let user_count = ctx.env.db.list_users(&rustpress_db::UserListQuery { limit: 1, orderby: "ID".into(), ..Default::default() }).await.map(|(_, n)| n).unwrap_or(0);
    let user_locale = ctx.env.db.user_meta(ctx.user.id, "locale").await.ok().flatten().filter(|l| !l.is_empty()).unwrap_or_else(|| site.locale());
    info.push(Section {
        id: "wp-core",
        label: tr("WordPress"),
        description: String::new(),
        show_count: false,
        fields: vec![
            field("version", tr("Version"), ctx.env.loader.version.clone()),
            field("site_language", tr("Site Language"), site.locale()),
            field("user_language", tr("User Language"), user_locale),
            field("timezone", tr("Timezone"), timezone_string(ctx)),
            field("home_url", tr("Home URL"), site.settings.home.clone()).private(),
            field("site_url", tr("Site URL"), site.settings.siteurl.clone()).private(),
            field("permalink", tr("Permalink structure"), if permalink.is_empty() { tr("No permalink structure set") } else { permalink.clone() }).debug(permalink.clone()),
            field("https_status", tr("Is this site using HTTPS?"), tr(if https { "Yes" } else { "No" })).debug_bool(https),
            field("multisite", tr("Is this a multisite?"), tr("No")).debug_bool(false),
            field("user_registration", tr("Can anyone register on this site?"), tr(if registration { "Yes" } else { "No" })).debug(if registration { "1" } else { "0" }),
            field("blog_public", tr("Is this site discouraging search engines?"), tr(if blog_public { "No" } else { "Yes" })).debug(if blog_public { "1" } else { "0" }),
            field("default_comment_status", tr("Default comment status"), site.trx(if comment_status == "open" { "Open" } else { "Closed" }, "comment status")).debug(comment_status),
            field("environment_type", tr("Environment type"), environment.clone()).debug(environment),
            field("user_count", tr("User count"), user_count.to_string()),
            field("dotorg_communication", tr("Communication with WordPress.org"), tr("RustPress does not communicate with WordPress.org")).debug("false"),
        ],
    });

    // Directories and sizes
    let uploads = uploads_dir(ctx, &root);
    let loading = tr("Loading&hellip;");
    let mut paths = Vec::new();
    for (key, label, path, size_key, size_label) in [
        ("wordpress_path", "WordPress directory location", path_string(&root), "wordpress_size", "WordPress directory size"),
        ("uploads_path", "Uploads directory location", path_string(&uploads), "uploads_size", "Uploads directory size"),
        ("themes_path", "Themes directory location", path_string(&content.join("themes")), "themes_size", "Themes directory size"),
        ("plugins_path", "Plugins directory location", path_string(&content.join("plugins")), "plugins_size", "Plugins directory size"),
        ("fonts_path", "Fonts directory location", path_string(&uploads.join("fonts")), "fonts_size", "Fonts directory size"),
    ] {
        paths.push(field(key, tr(label), path));
        paths.push(field(size_key, tr(size_label), loading.clone()).debug("loading..."));
    }
    paths.push(field("database_size", tr("Database size"), loading.clone()).debug("loading..."));
    paths.push(field("total_size", tr("Total installation size"), loading.clone()).debug("loading..."));
    info.push(Section { id: "wp-paths-sizes", label: tr("Directories and Sizes"), description: String::new(), show_count: false, fields: paths });

    // Themes
    let installed = themes::installed(&root);
    let active_stylesheet = site.options.string("stylesheet", "");
    let auto_themes = option_list(ctx, "auto_update_themes");
    let active_theme = installed.iter().find(|t| t.stylesheet == active_stylesheet).cloned();
    let parent_theme = active_theme.as_ref().and_then(|t| if t.has_parent() { installed.iter().find(|p| p.stylesheet == t.template).cloned() } else { None });
    if let Some(theme) = &active_theme {
        info.push(Section { id: "wp-active-theme", label: tr("Active Theme"), description: String::new(), show_count: false, fields: theme_fields(ctx, theme, parent_theme.as_ref().unwrap_or(theme), &auto_themes, true) });
    }
    info.push(Section {
        id: "wp-parent-theme",
        label: tr("Parent Theme"),
        description: String::new(),
        show_count: false,
        fields: parent_theme
            .as_ref()
            .map(|p| {
                let mut fields = theme_fields(ctx, p, p, &auto_themes, false);
                fields.retain(|f| f.key != "parent_theme");
                fields
            })
            .unwrap_or_default(),
    });
    let mut inactive_themes = Vec::new();
    for theme in installed.iter().filter(|t| t.stylesheet != active_stylesheet && parent_theme.as_ref().is_none_or(|p| p.stylesheet != t.stylesheet)) {
        let author = rustpress_core::formatting::strip_tags(&theme.author);
        let (mut value, mut debug) = version_string(ctx, &theme.version, &author);
        if debug.starts_with("author: (undefined), version: (undefined)") {
            debug = "undefined".into();
        }
        let enabled = auto_themes.contains(&theme.stylesheet);
        let auto = tr(if enabled { "Auto-updates enabled" } else { "Auto-updates disabled" });
        value.push_str(&format!(" | {auto}"));
        debug.push_str(&format!(", {auto}"));
        inactive_themes.push(field(&rustpress_core::formatting::strip_tags(&theme.name), sprintf(&tr("%1$s (%2$s)"), &[&theme.name, &theme.stylesheet]), value).debug(debug));
    }
    info.push(Section { id: "wp-themes-inactive", label: tr("Inactive Themes"), description: String::new(), show_count: true, fields: inactive_themes });

    // Plugins
    let mut mu = Vec::new();
    for plugin in plugin_scan::must_use(&root) {
        let (value, debug) = version_string(ctx, &plugin.version, &plugin.author);
        mu.push(field(&plugin.name, plugin.name.clone(), value).debug(debug));
    }
    info.push(Section { id: "wp-mu-plugins", label: tr("Must Use Plugins"), description: String::new(), show_count: true, fields: mu });
    let active_plugins = option_list(ctx, "active_plugins");
    let auto_plugins = option_list(ctx, "auto_update_plugins");
    let (mut active, mut inactive) = (Vec::new(), Vec::new());
    for plugin in plugin_scan::installed(&root) {
        let (mut value, mut debug) = version_string(ctx, &plugin.version, &plugin.author);
        let enabled = auto_plugins.contains(&plugin.file);
        let auto = tr(if enabled { "Auto-updates enabled" } else { "Auto-updates disabled" });
        value.push_str(&format!(" | {auto}"));
        debug.push_str(&format!(", {auto}"));
        let entry = field(&rustpress_core::formatting::strip_tags(&plugin.name), plugin.name.clone(), value).debug(debug);
        if active_plugins.contains(&plugin.file) { active.push(entry) } else { inactive.push(entry) }
    }
    info.push(Section { id: "wp-plugins-active", label: tr("Active Plugins"), description: String::new(), show_count: true, fields: active });
    info.push(Section { id: "wp-plugins-inactive", label: tr("Inactive Plugins"), description: String::new(), show_count: true, fields: inactive });

    // Media handling
    let max = super::media_ui::max_upload();
    info.push(Section {
        id: "wp-media",
        label: tr("Media Handling"),
        description: String::new(),
        show_count: false,
        fields: vec![
            field("image_editor", tr("Active editor"), "RustPress"),
            field("file_uploads", tr("File uploads"), tr("Enabled")).debug("1"),
            field("upload_max_filesize", tr("Max size of an uploaded file"), format!("{}M", max / (1024 * 1024))),
            field("max_effective_size", tr("Max effective file size"), super::ajax_media::size_format(max)),
        ],
    });

    // Server
    let permalink_supported = !permalink.is_empty();
    let now = ctx.env.now;
    let utc = |format: &str| rustpress_core::datetime::SiteTimezone::utc().format(format, now);
    info.push(Section {
        id: "wp-server",
        label: tr("Server"),
        description: tr("The options shown below relate to your server setup. If changes are required, you may need your web host&#8217;s assistance."),
        show_count: false,
        fields: vec![
            field("server_architecture", tr("Server architecture"), format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)),
            field("httpd_software", tr("Web server"), format!("RustPress/{}", env!("CARGO_PKG_VERSION"))),
            field("pretty_permalinks", tr("Are pretty permalinks supported?"), tr(if permalink_supported { "Yes" } else { "No" })).debug_bool(permalink_supported),
            field("static_robotstxt_file", tr("robots.txt"), sprintf(&tr("Your site is using the dynamic %s file which is generated by WordPress."), &["robots.txt"])).debug_bool(false),
            field("current", tr("Current time"), utc("c")),
            field("utc-time", tr("Current UTC time"), utc("l, d-M-y H:i:s \\U\\T\\C")),
            field("server-time", tr("Current Server time"), site.date("c", now)),
        ],
    });

    // Database
    let server = ctx.env.db.server_info().await.unwrap_or_default();
    info.push(Section {
        id: "wp-database",
        label: tr("Database"),
        description: String::new(),
        show_count: false,
        fields: vec![
            field("extension", tr("Database Extension"), "sqlx (MySQL protocol)"),
            field("server_version", tr("Server version"), server.version.clone()),
            field("client_version", tr("Client version"), format!("sqlx {}", "0.9")),
            field("database_user", tr("Database username"), server.user.clone()).private(),
            field("database_host", tr("Database host"), server.host.clone()).private(),
            field("database_name", tr("Database name"), server.name.clone()).private(),
            field("database_prefix", tr("Table prefix"), ctx.env.db.prefix().as_str().to_owned()).private(),
            field("database_charset", tr("Database charset"), server.charset.clone()).private(),
            field("database_collate", tr("Database collation"), server.collation.clone()).private(),
            field("max_allowed_packet", tr("Max allowed packet size"), server.max_allowed_packet.to_string()),
            field("max_connections", tr("Max connections number"), server.max_connections.to_string()),
        ],
    });

    // Constants
    let undefined = || tr("Undefined");
    let dir = |p: PathBuf| path_string(&p);
    info.push(Section {
        id: "wp-constants",
        label: tr("WordPress Constants"),
        description: String::new(),
        show_count: false,
        fields: vec![
            field("ABSPATH", "ABSPATH".into(), format!("{}/", path_string(&root))).private(),
            field("WP_HOME", "WP_HOME".into(), undefined()).debug("undefined"),
            field("WP_SITEURL", "WP_SITEURL".into(), undefined()).debug("undefined"),
            field("WP_CONTENT_DIR", "WP_CONTENT_DIR".into(), dir(content.clone())),
            field("WP_PLUGIN_DIR", "WP_PLUGIN_DIR".into(), dir(content.join("plugins"))),
            field("WP_MEMORY_LIMIT", "WP_MEMORY_LIMIT".into(), undefined()).debug("undefined"),
            field("WP_MAX_MEMORY_LIMIT", "WP_MAX_MEMORY_LIMIT".into(), undefined()).debug("undefined"),
            field("WP_DEBUG", "WP_DEBUG".into(), tr("Disabled")).debug("false"),
            field("WP_DEBUG_DISPLAY", "WP_DEBUG_DISPLAY".into(), tr("Enabled")).debug("true"),
            field("WP_DEBUG_LOG", "WP_DEBUG_LOG".into(), tr("Disabled")).debug("false"),
            field("SCRIPT_DEBUG", "SCRIPT_DEBUG".into(), tr("Disabled")).debug("false"),
            field("WP_CACHE", "WP_CACHE".into(), tr("Disabled")).debug("false"),
            field("CONCATENATE_SCRIPTS", "CONCATENATE_SCRIPTS".into(), undefined()).debug("undefined"),
            field("COMPRESS_SCRIPTS", "COMPRESS_SCRIPTS".into(), undefined()).debug("undefined"),
            field("COMPRESS_CSS", "COMPRESS_CSS".into(), undefined()).debug("undefined"),
            field("WP_ENVIRONMENT_TYPE", "WP_ENVIRONMENT_TYPE".into(), undefined()).debug("undefined"),
            field("WP_DEVELOPMENT_MODE", "WP_DEVELOPMENT_MODE".into(), tr("Disabled")).debug("undefined"),
            field("DB_CHARSET", "DB_CHARSET".into(), server.charset.clone()),
            field("DB_COLLATE", "DB_COLLATE".into(), tr("Empty value")).debug(""),
            field("EMPTY_TRASH_DAYS", "EMPTY_TRASH_DAYS".into(), "30"),
        ],
    });

    // Filesystem
    let access = |ok: bool| tr(if ok { "Writable" } else { "Not writable" });
    info.push(Section {
        id: "wp-filesystem",
        label: tr("Filesystem Permissions"),
        description: tr("Shows whether WordPress is able to write to the directories it needs access to."),
        show_count: false,
        fields: vec![
            field("wordpress", tr("The main WordPress directory"), access(writable(&root))).debug(if writable(&root) { "writable" } else { "not writable" }),
            field("wp-content", tr("The wp-content directory"), access(writable(&content))).debug(if writable(&content) { "writable" } else { "not writable" }),
            field("uploads", tr("The uploads directory"), access(writable(&uploads))).debug(if writable(&uploads) { "writable" } else { "not writable" }),
            field("plugins", tr("The plugins directory"), access(writable(&content.join("plugins")))).debug(if writable(&content.join("plugins")) { "writable" } else { "not writable" }),
            field("themes", tr("The themes directory"), access(writable(&content.join("themes")))).debug(if writable(&content.join("themes")) { "writable" } else { "not writable" }),
            field("fonts", tr("The fonts directory"), access(writable(&uploads.join("fonts")))).debug(if writable(&uploads.join("fonts")) { "writable" } else { "not writable" }),
            field("mu-plugins", tr("The must use plugins directory"), access(writable(&content.join("mu-plugins")))).debug(if writable(&content.join("mu-plugins")) { "writable" } else { "not writable" }),
        ],
    });
    info
}

/// The body of the Info tab, from `site-health-info.php`.
pub async fn body(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let info = debug_data(ctx).await;
    let dump = format_debug(&info);
    let mut h = format!(
        "<div class=\"health-check-body health-check-debug-tab hide-if-no-js\">\n\t\n\t<h2>\n\t\t{}\t</h2>\n\n\t<p>\n\t\t{}\t</p>\n\t<p>\n\t\t{}\t</p>\n\n\t<div class=\"site-health-copy-buttons\">\n\t\t<div class=\"copy-button-wrapper\">\n\t\t\t<button type=\"button\" class=\"button copy-button\" data-clipboard-text=\"{}\">\n\t\t\t\t{}\t\t\t</button>\n\t\t\t<span class=\"success hidden\" aria-hidden=\"true\">{}</span>\n\t\t</div>\n\t</div>\n\n\t<div id=\"health-check-debug\" class=\"health-check-accordion\">\n\n\t\t",
        tr("Site Health Info"),
        sprintf(&tr("This page can show you every detail about the configuration of your WordPress website. For any improvements that could be made, see the <a href=\"%s\">Site Health Status</a> page."), &[&rustpress_core::formatting::esc_url(&ctx.admin_url("site-health.php"))]),
        tr("If you want to export a handy list of all the information on this page, you can use the button below to copy it to the clipboard. You can then paste it in a text file and save it to your device, or paste it in an email exchange with a support engineer or theme/plugin developer for example."),
        esc_attr(&dump),
        tr("Copy site info to clipboard"),
        tr("Copied!")
    );
    let sizes = ["uploads_size", "themes_size", "plugins_size", "fonts_size", "wordpress_size", "database_size", "total_size"];
    for section in &info {
        if section.fields.is_empty() {
            continue;
        }
        let count = if section.show_count { format!("({})", ctx.site().number_i18n(section.fields.len() as u64)) } else { String::new() };
        let spinner = if section.id == "wp-paths-sizes" { "\t\t\t\t\t\t<span class=\"health-check-wp-paths-sizes spinner\"></span>\n\t\t\t\t\t\t" } else { "" };
        let description = if section.description.is_empty() { String::new() } else { format!("<p>{}</p>", section.description) };
        let mut rows = String::new();
        for f in &section.fields {
            let values = match &f.value {
                Val::List(items) => format!("<ul>{}</ul>", items.iter().map(|(k, v)| format!("<li>{}: {}</li>", esc_html(k), esc_html(v))).collect::<String>()),
                Val::Text(t) => esc_html(t),
            };
            if sizes.contains(&f.key.as_str()) {
                rows.push_str(&format!("<tr><th scope=\"row\">{}</th><td class=\"{}\">{values}</td></tr>", esc_html(&f.label), esc_attr(&f.key)));
            } else {
                rows.push_str(&format!("<tr><th scope=\"row\">{}</th><td>{values}</td></tr>", esc_html(&f.label)));
            }
        }
        h.push_str(&format!(
            "\t\t\t<h3 class=\"health-check-accordion-heading\">\n\t\t\t\t<button id=\"health-check-section-{id}\" aria-expanded=\"false\" class=\"health-check-accordion-trigger\" aria-controls=\"health-check-accordion-block-{id}\" type=\"button\">\n\t\t\t\t\t<span class=\"title\">\n\t\t\t\t\t\t{}\t\t\t\t\t\t{count}\t\t\t\t\t</span>\n\t\t\t\t\t{spinner}\t\t\t\t\t<span class=\"icon\"></span>\n\t\t\t\t</button>\n\t\t\t</h3>\n\t\t\t<div id=\"health-check-accordion-block-{id}\" class=\"health-check-accordion-panel\" hidden=\"hidden\">\n\t\t\t\t{description}\t\t\t\t<table class=\"widefat striped health-check-table\">\n\t\t\t\t\t<tbody>\n\t\t\t\t\t{rows}\t\t\t\t\t</tbody>\n\t\t\t\t</table>\n\t\t\t</div>\n\t\t",
            esc_html(&section.label),
            id = section.id
        ));
    }
    h.push_str("\t</div>\n</div>\n");
    h
}

/// The Ajax action `health-check-get-sizes`: the sizes of the directories and the database.
pub async fn sizes(env: &super::AdminEnv<'_>) -> serde_json::Value {
    let root = env.loader.includes.parent().map(Path::to_path_buf).unwrap_or_default();
    serde_json::json!({"success": true, "data": rustpress_core::health::directory_sizes(env.site, &root).await})
}

#[allow(dead_code)]
async fn sizes_old(env: &super::AdminEnv<'_>) -> serde_json::Value {
    use serde_json::{Map, Value as Json, json};
    let root = env.loader.includes.parent().map(Path::to_path_buf).unwrap_or_default();
    let content = root.join("wp-content");
    let configured = env.site.options.string("upload_path", "");
    let uploads = if configured.is_empty() { content.join("uploads") } else if Path::new(&configured).is_absolute() { PathBuf::from(configured) } else { root.join(configured) };
    fn size_of(path: &Path) -> u64 {
        let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
        if meta.is_file() {
            return meta.len();
        }
        if !meta.is_dir() {
            return 0;
        }
        std::fs::read_dir(path).map(|entries| entries.flatten().map(|e| size_of(&e.path())).sum()).unwrap_or(0)
    }
    let fonts = uploads.join("fonts");
    let (themes, plugins, uploads_all, fonts_size) = (size_of(&content.join("themes")), size_of(&content.join("plugins")), size_of(&uploads), size_of(&fonts));
    // The WordPress directory is what the other directories do not hold.
    let wordpress = size_of(&root).saturating_sub(themes + plugins + uploads_all);
    let entries: Vec<(&str, u64)> = vec![
        ("wordpress_size", wordpress),
        ("themes_size", themes),
        ("plugins_size", plugins),
        ("uploads_size", uploads_all.saturating_sub(fonts_size)),
        ("fonts_size", fonts_size),
        ("database_size", env.db.database_size().await.unwrap_or(0)),
    ];
    let format = |bytes: u64| -> Json {
        let size = super::ajax_media::size_format(bytes);
        json!({"size": size, "debug": format!("{size} ({} bytes)", space_separated(bytes))})
    };
    let mut data = Map::new();
    let mut raw = 0u64;
    for (name, bytes) in &entries {
        raw += *bytes;
        let _ = name;
        data.insert((*name).to_owned(), format(*bytes));
    }
    data.insert("total_size".into(), format(raw));
    let mut ordered = Map::new();
    ordered.insert("raw".into(), json!(raw));
    for (key, value) in data {
        ordered.insert(key, value);
    }
    json!({"success": true, "data": Json::Object(ordered)})
}

/// `space_separated_number()`: 1 234 567.
fn space_separated(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}
