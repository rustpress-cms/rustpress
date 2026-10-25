//! The frame of every administration screen: head, toolbar, main menu, footer and the scripts.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

use crate::adminbar::{self, BarContext};
use crate::menu::AdminMenu;
use crate::registry::{LoaderEnv, default_scripts, default_styles};

/// The screen being shown (`$pagenow`, `$parent_file`, `$submenu_file`, `$typenow`).
#[derive(Debug, Clone, Default)]
pub struct Screen {
    /// `options-general.php`
    pub file: String,
    /// The top-level entry the screen belongs to; defaults to `file`.
    pub parent_file: String,
    /// The highlighted submenu entry; defaults to `file`.
    pub submenu_file: Option<String>,
    pub post_type: String,
    /// `$title`, in plain text.
    pub title: String,
    /// The contextual help (`get_current_screen()->add_help_tab()`).
    pub help: Option<Help>,
    /// The "View Post" entry of the toolbar: `(label, address)`.
    pub view_item: Option<(String, String)>,
    /// The block editor takes the whole window (`is-fullscreen-mode`).
    pub fullscreen: bool,
    /// More classes of the `body` element.
    pub body_classes: Vec<String>,
}

/// One tab of the contextual help.
#[derive(Debug, Clone)]
pub struct HelpTab {
    pub id: String,
    pub title: String,
    /// HTML.
    pub content: String,
}

/// The contextual help of a screen.
#[derive(Debug, Clone, Default)]
pub struct Help {
    pub tabs: Vec<HelpTab>,
    /// HTML of the sidebar ("For more information").
    pub sidebar: String,
}

/// `WP_Screen::render_screen_meta()` with the help tabs and the Help button.
fn screen_meta(help: &Help, label: &str) -> String {
    if help.tabs.is_empty() {
        // A screen without help tabs: the empty wrapper and no Help button.
        return "		<div id=\"screen-meta\" class=\"metabox-prefs\">

			<div id=\"contextual-help-wrap\" class=\"hidden no-sidebar\" tabindex=\"-1\" aria-label=\"Contextual Help Tab\">
				<div id=\"contextual-help-back\"></div>
				<div id=\"contextual-help-columns\">
					<div class=\"contextual-help-tabs\">
						<ul>
												</ul>
					</div>

					
					<div class=\"contextual-help-tabs-wrap\">
											</div>
				</div>
			</div>
				</div>
		".to_owned();
    }
    let mut tabs = String::new();
    let mut panels = String::new();
    for (index, tab) in help.tabs.iter().enumerate() {
        let active = if index == 0 { " class=\"active\"" } else { "" };
        let panel_active = if index == 0 { " active" } else { "" };
        tabs.push_str(&format!(
            "
							<li id=\"tab-link-{id}\"{active}>
								<a href=\"#tab-panel-{id}\" aria-controls=\"tab-panel-{id}\">
									{title}								</a>
							</li>
",
            id = esc_attr(&tab.id),
            title = esc_html(&tab.title)
        ));
        panels.push_str(&format!("
							<div id=\"tab-panel-{}\" class=\"help-tab-content{panel_active}\">
								{}							</div>
", esc_attr(&tab.id), tab.content));
    }
    let sidebar = if help.sidebar.is_empty() { String::new() } else { format!("										<div class=\"contextual-help-sidebar\">
						{}					</div>
", help.sidebar) };
    format!(
        "		<div id=\"screen-meta\" class=\"metabox-prefs\">

			<div id=\"contextual-help-wrap\" class=\"hidden\" tabindex=\"-1\" aria-label=\"Contextual Help Tab\">
				<div id=\"contextual-help-back\"></div>
				<div id=\"contextual-help-columns\">
					<div class=\"contextual-help-tabs\">
						<ul>
						{tabs}													</ul>
					</div>

{sidebar}					
					<div class=\"contextual-help-tabs-wrap\">
						{panels}												</div>
				</div>
			</div>
				</div>
				<div id=\"screen-meta-links\">
					<div id=\"contextual-help-link-wrap\" class=\"hide-if-no-js screen-meta-toggle\">
			<button type=\"button\" id=\"contextual-help-link\" class=\"button button-compact show-settings\" aria-controls=\"contextual-help-wrap\" aria-expanded=\"false\">{}</button>
			</div>
				</div>
		
",
        esc_html(label)
    )
}

/// What the frame needs besides the screen.
pub struct ChromeEnv<'a> {
    pub site: &'a Site,
    pub loader: LoaderEnv,
    pub user: &'a CurrentUser,
    pub menu: &'a AdminMenu,
    pub bar: BarContext,
    /// `get_stylesheet()`, for the `wp-theme-…` body class.
    pub stylesheet: String,
    /// `COOKIEPATH`, `time()`, whether the sign-in is over TLS: `userSettings`.
    pub cookie_path: String,
    pub now: i64,
    pub secure: bool,
}

/// Extra assets of a screen.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub styles: Vec<String>,
    pub scripts: Vec<String>,
    /// Scripts a page enqueues before `admin-header.php` runs: they come before `svg-painter` and `heartbeat`.
    pub early_scripts: Vec<String>,
    /// Printed in the head after the scripts (`admin_head`).
    pub head: String,
    /// Printed after the scripts of the footer (`admin_print_footer_scripts`, late).
    pub footer: String,
    /// Printed before the scripts of the footer (`admin_footer`: the media templates).
    pub pre_footer: String,
    /// `wp_localize_script()`: `(handle, object name, data)`.
    pub localize: Vec<(String, String, serde_json::Value)>,
    /// `wp_add_inline_script()`: `(handle, code, position)`.
    pub inline: Vec<(String, String, String)>,
    /// The screen runs the block editor: its script modules (math blocks, routing) are loaded too.
    pub editor_modules: bool,
}

/// The `version` of a script module (`wp-includes/js/dist/script-modules/<path>.min.asset.php`).
fn module_version(includes: &std::path::Path, path: &str) -> String {
    let asset = std::fs::read_to_string(includes.join("js/dist/script-modules").join(format!("{path}.min.asset.php"))).unwrap_or_default();
    asset.split("'version' => '").nth(1).and_then(|rest| rest.split('\'').next()).unwrap_or_default().to_owned()
}

/// `WP_Script_Modules::print_import_map()` and `print_enqueued_script_modules()`.
fn script_modules(env: &ChromeEnv<'_>, editor: bool) -> String {
    let root = env.loader.site_url.trim_end_matches('/').to_owned();
    let includes = &env.loader.includes;
    let url = |path: &str| format!("{root}/wp-includes/js/dist/script-modules/{path}.min.js?ver={}", module_version(includes, path));
    let mut imports = vec![("@wordpress/video-conversion/worker", "video-conversion/worker"), ("@wordpress/vips/worker", "vips/worker")];
    if editor {
        imports.push(("@wordpress/latex-to-mathml", "latex-to-mathml/index"));
        imports.push(("@wordpress/route", "route/index"));
    }
    let map: serde_json::Map<String, serde_json::Value> = imports.into_iter().map(|(id, path)| (id.to_owned(), serde_json::Value::from(url(path)))).collect();
    let json = rustpress_blocks::php_json::encode_with(&serde_json::json!({"imports": map}), rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: true });
    let mut out = format!("<script id=\"wp-importmap\" type=\"importmap\">\n{json}\n</script>\n");
    if editor {
        out.push_str(&format!("<script id=\"@wordpress/latex-to-mathml/loader-js-module\" src=\"{}\" type=\"module\"></script>\n", url("latex-to-mathml/loader")));
    }
    out
}

/// `wp-admin/admin-header.php` to `admin-footer.php` around `content` (the inside of `.wrap`, with
/// its own `<div class="wrap">`).
pub fn render(env: &ChromeEnv<'_>, screen: &Screen, content: &str, assets: &Assets) -> String {
    let site = env.site;
    let tr = |s: &str| site.tr(s);
    let version = env.loader.version.clone();
    let rtl = site.is_rtl();
    let page = screen.file.trim_end_matches(".php");
    let parent_file = if screen.parent_file.is_empty() { screen.file.clone() } else { screen.parent_file.clone() };
    let submenu_file = screen.submenu_file.clone().or_else(|| Some(screen.file.clone()));
    let blog_name = site.options.string("blogname", "");
    let admin_title = if blog_name.is_empty() { rustpress_core::formatting::wptexturize(&site.options.string("siteurl", "")) } else { rustpress_core::formatting::wptexturize(&blog_name) };
    let title = format!("{} &lsaquo; {} &#8212; WordPress", esc_html(&screen.title), esc_html(&admin_title).replace('\'', "&#039;"));

    // Styles of the head.
    let mut styles = default_styles(&env.loader);
    // The styles a screen asks for (`wp_enqueue_media()`) are enqueued before `admin-header.php` adds its own.
    if !assets.styles.is_empty() {
        styles.enqueue("admin-bar");
        for handle in &assets.styles {
            styles.enqueue(handle);
        }
    }
    for handle in [
        "dashicons", "admin-bar", "common", "forms", "admin-menu", "dashboard", "list-tables", "edit", "revisions", "media", "themes", "about", "nav-menus", "wp-pointer", "widgets", "site-icon", "l10n", "wp-base-styles", "wp-tooltip", "buttons",
        "wp-auth-check", "wp-theme", "wp-components", "wp-commands",
    ] {
        styles.enqueue(handle);
    }
    for handle in &assets.styles {
        styles.enqueue(handle);
    }
    let admin_styles = styles.print_admin_styles(true);
    let colors = format!("<link rel='stylesheet' id='colors-css' href='{}/wp-admin/css/colors/modern/colors.min.css?ver={version}' media='all' />\n", env.loader.site_url);

    // Scripts of the head (`utils` with `userSettings`).
    let mut scripts = default_scripts(&env.loader);
    scripts.localize(
        "utils",
        "userSettings",
        &serde_json::json!({"url": env.cookie_path, "uid": env.user.id, "time": env.now, "secure": if env.secure { "1" } else { "" }}),
    );
    // The editors enqueue their own scripts before `admin-header.php`, and `utils` comes after the first two.
    if assets.early_scripts.is_empty() {
        scripts.enqueue("utils");
    }
    // `wp_heartbeat_settings()` and `wp_heartbeat_set_suspension()`.
    let mut heartbeat = serde_json::Map::new();
    if let Some(nonce) = env.loader.nonce.as_ref() {
        heartbeat.insert("nonce".into(), serde_json::Value::String(nonce("heartbeat-nonce")));
    }
    if matches!(screen.file.as_str(), "post.php" | "post-new.php") {
        heartbeat.insert("suspension".into(), serde_json::Value::String("disable".into()));
    }
    scripts.relocalize("heartbeat", "heartbeatSettings", &serde_json::Value::Object(heartbeat));
    let (first, rest) = assets.early_scripts.split_at(assets.early_scripts.len().min(2));
    for handle in ["common", "admin-bar"].into_iter().chain(first.iter().map(String::as_str)).chain(if assets.early_scripts.is_empty() { None } else { Some("utils") }).chain(rest.iter().map(String::as_str)).chain(["svg-painter", "heartbeat", "wp-auth-check"]) {
        scripts.enqueue(handle);
    }
    for (handle, object, data) in &assets.localize {
        scripts.localize(handle, object, data);
    }
    for (handle, code, position) in &assets.inline {
        scripts.add_inline_script(handle, code, position);
    }
    for handle in &assets.scripts {
        scripts.enqueue(handle);
    }
    let head_scripts = scripts.print_head_scripts(true);
    let footer_scripts = scripts.print_footer_scripts(true);

    let mut classes = vec!["wp-admin".to_owned(), "wp-core-ui".to_owned(), "no-js".to_owned()];
    if screen.fullscreen {
        classes.push("is-fullscreen-mode".to_owned());
    }
    classes.push(format!("{}-php", page));
    classes.push("auto-fold".to_owned());
    classes.push("admin-bar".to_owned());
    if !screen.post_type.is_empty() {
        classes.push(format!("post-type-{}", rustpress_core::formatting::sanitize_html_class(&screen.post_type)));
    }
    let mut parts = version.split('.');
    let (major, minor) = (parts.next().unwrap_or("0"), parts.next().unwrap_or("0"));
    classes.push(format!("branch-{major}-{minor}"));
    classes.push(format!("version-{}", version.replace('.', "-")));
    classes.push("admin-color-modern".to_owned());
    if rtl {
        classes.push("rtl".to_owned());
    }
    classes.push(format!("locale-{}", rustpress_core::formatting::sanitize_html_class(&site.locale().replace('_', "-").to_lowercase())));
    classes.push("no-customize-support".to_owned());
    classes.push("svg".to_owned());
    classes.extend(screen.body_classes.iter().cloned());
    classes.push(format!("wp-theme-{}", rustpress_core::formatting::sanitize_html_class(&env.stylesheet)));

    let locale_data = site.locale_data();
    let thousands = (locale_data.thousands_sep.clone(), locale_data.decimal_point.clone());
    let ajax = esc_url(&site.site_url("wp-admin/admin-ajax.php"));
    let ajax_path = ajax.strip_prefix(&env.loader.site_url).unwrap_or(&ajax).to_owned();
    let canonical = site.site_url(&format!("wp-admin/{}", screen.file));

    let mut out = String::new();
    out.push_str(&format!(
        "<!DOCTYPE html>\n<html class=\"wp-toolbar\"\n\t{}>\n<head>\n<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\" />\n\t<title>{title}</title>\n",
        if rtl { format!("dir=\"rtl\" lang=\"{}\"", esc_attr(&site.language_tag())) } else { format!("lang=\"{}\"", esc_attr(&site.language_tag())) }
    ));
    out.push_str(&format!(
        "<script>\naddLoadEvent = function(func){{if(typeof jQuery!=='undefined')jQuery(function(){{func();}});else if(typeof wpOnload!=='function'){{wpOnload=func;}}else{{var oldonload=wpOnload;wpOnload=function(){{oldonload();func();}}}}}};\nvar ajaxurl = '{ajax_path}',\n\tpagenow = '{page}',\n\ttypenow = '{}',\n\tadminpage = '{}',\n\tthousandsSeparator = '{}',\n\tdecimalPoint = '{}',\n\tisRtl = {};\n</script>\n",
        screen.post_type,
        page.to_owned() + "-php",
        thousands.0,
        thousands.1,
        i32::from(rtl)
    ));
    out.push_str(&admin_styles);
    out.push_str(&colors);
    out.push_str(&head_scripts);
    out.push_str("<script>var _wpColorScheme = {\"icons\":{\"base\":\"#f3f1f1\",\"focus\":\"#fff\",\"current\":\"#fff\"}};</script>\n");
    out.push_str(&format!(
        "\t<link id=\"wp-admin-canonical\" rel=\"canonical\" href=\"{}\" />\n\t<script>\n\t\tif ( window.history.replaceState ) {{\n\t\t\twindow.history.replaceState( null, null, document.getElementById( 'wp-admin-canonical' ).href + window.location.hash );\n\t\t}}\n\t</script>\n\t<meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\">",
        esc_url(&canonical)
    ));
    out.push_str(&assets.head);
    out.push_str("\n\t</head>\n");
    out.push_str(&format!("<body class=\"{}\">\n", esc_attr(&classes.join(" "))));
    out.push_str("<script>\n\tdocument.body.className = document.body.className.replace('no-js','js');\n</script>\n\n");
    out.push_str("<script>\n\t\t(function() {\n\t\t\tvar request, b = document.body, c = 'className', cs = 'customize-support', rcs = new RegExp('(^|\\\\s+)(no-)?'+cs+'(\\\\s+|$)');\n\n\t\t\t\trequest = true;\n\t\n\t\t\tb[c] = b[c].replace( rcs, ' ' );\n\t\t\t// The customizer requires postMessage and CORS (if the site is cross domain).\n\t\t\tb[c] += ( window.postMessage && request ? ' ' : ' no-' ) + cs;\n\t\t}());\n\t\n//# sourceURL=wp_customize_support_script\n</script>\n\n");
    out.push_str("<div id=\"wpwrap\">\n\n");
    out.push_str(&format!(
        "<div id=\"adminmenumain\" role=\"navigation\" aria-label=\"{}\">\n<a href=\"#wpbody-content\" class=\"screen-reader-shortcut\">{}</a>\n<a href=\"#wp-toolbar\" class=\"screen-reader-shortcut\">{}</a>\n<div id=\"adminmenuback\"></div>\n<div id=\"adminmenuwrap\">\n<ul id=\"adminmenu\">\n\n",
        esc_attr(&tr("Main menu")),
        tr("Skip to main content"),
        tr("Skip to toolbar")
    ));
    out.push_str(&env.menu.render(site, &screen.file, &parent_file, submenu_file.as_deref(), &screen.post_type));
    out.push_str("</ul>\n</div>\n</div>\n");
    out.push_str("<div id=\"wpcontent\">\n");
    let mut bar = env.bar.clone();
    bar.view_item = screen.view_item.clone();
    out.push_str(&adminbar::render_admin(site, env.user, &bar));
    out.push_str("\n<div id=\"wpbody\" role=\"main\">\n\n<div id=\"wpbody-content\">\n");
    match &screen.help {
        Some(help) => out.push_str(&screen_meta(help, &tr("Help"))),
        None => out.push_str("\t\t"),
    }
    out.push_str(content);
    out.push_str("\n<div class=\"clear\"></div></div><!-- wpbody-content -->\n<div class=\"clear\"></div></div><!-- wpbody -->\n<div class=\"clear\"></div></div><!-- wpcontent -->\n\n");
    out.push_str(&format!(
        "<div id=\"wpfooter\" role=\"contentinfo\">\n\t\t<p id=\"footer-left\" class=\"alignleft\">\n\t\t<span id=\"footer-thankyou\">{}</span>\t</p>\n\t<p id=\"footer-upgrade\" class=\"alignright\">\n\t\t{}\t</p>\n\t<div class=\"clear\"></div>\n</div>\n\n",
        rustpress_core::i18n::sprintf(&tr("Thank you for creating with <a href=\"%s\">WordPress</a>."), &["https://wordpress.org/"]),
        rustpress_core::i18n::sprintf(&tr("Version %s"), &[&version])
    ));
    out.push_str(&assets.pre_footer);
    out.push_str(&format!(
        "\t\t<div id=\"wp-auth-check-wrap\" class=\"hidden\">\n\t<div id=\"wp-auth-check-bg\"></div>\n\t<div id=\"wp-auth-check\">\n\t<button type=\"button\" class=\"wp-auth-check-close button-link\"><span class=\"screen-reader-text\">\n\t\t{close}\t</span></button>\n\t\t\t<div id=\"wp-auth-check-form\" class=\"loading\" data-src=\"{src}\"></div>\n\t\t\t<div class=\"wp-auth-fallback\">\n\t\t<p><b class=\"wp-auth-fallback-expired\" tabindex=\"0\">{expired}</b></p>\n\t\t<p><a href=\"{login}\" target=\"_blank\">{again}</a>\n\t\t{note}</p>\n\t</div>\n\t</div>\n\t</div>\n\t",
        close = tr("Close dialog"),
        src = esc_url(&format!("{}?interim-login=1&wp_lang={}", site.site_url("wp-login.php"), if site.locale() == "en_US" { String::new() } else { site.locale() })),
        expired = tr("Session expired"),
        login = esc_url(&site.site_url("wp-login.php")),
        again = tr("Please log in again."),
        note = tr("The login page will open in a new tab. After logging in you can close it and return to this page.")
    ));
    out.push_str(&script_modules(env, assets.editor_modules));
    out.push_str(&footer_scripts);
    out.push_str(&assets.footer);
    out.push_str("\n</body>\n</html>");
    out
}
