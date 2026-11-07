//! `wp-admin/customize.php`: the Customizer of a classic theme, `WP_Customize_Manager` with its widgets and menus. The
//! Customizer itself is WordPress' JavaScript; this module prints the page it starts on, with the same settings, controls,
//! sections and panels.

mod assets;
mod controls;
mod manager;
mod nav_menus;
mod register;
mod templates;
mod widgets;

use rustpress_core::formatting::{esc_attr, esc_html, esc_url, sanitize_html_class};
use rustpress_core::i18n::sprintf;
use rustpress_phpser::Value as Php;
use rustpress_theme::classic::custom_header::{self as header, Support};
use serde_json::json;

use super::block_editor::inline_json;
use super::{AdminResponse, Ctx, render};
use crate::chrome::{CustomizeParts, Screen};
use manager::Manager;

const TEMPLATES: &str = include_str!("../../data/customize_templates.html");
const MENU_TEMPLATES: &str = include_str!("../../data/customize_menus.html");
const NOTICE_TEMPLATES: &str = include_str!("../../data/customize_notices.html");
const THEME_TEMPLATES: &str = include_str!("../../data/customize_themes.html");
const HEADER_TEMPLATES: &str = include_str!("../../data/customize_header.html");

/// What the parts of the Customizer read.
pub(super) struct State<'a> {
    pub ctx: &'a Ctx<'a>,
    /// `get_theme_mods()`.
    pub mods: Php,
    /// `get_theme_support( 'custom-header' )`.
    pub support: Option<Support>,
    pub template_uri: String,
    pub stylesheet_uri: String,
    pub stylesheet: String,
    /// `wp_get_theme()->display( 'Name' )`.
    pub theme_name: String,
    /// `get_theme_support( 'custom-logo' )`: width, height, flexible width and height.
    pub custom_logo: Option<(u64, u64, bool, bool)>,
    /// `WP_Customize_Manager::has_published_pages()`.
    pub has_published_pages: bool,
    /// The user did not turn off syntax highlighting.
    pub syntax_highlighting: bool,
    /// `wp_get_custom_css()`.
    pub custom_css: String,
}

/// Prints a baked template with the addresses of the site, in its language.
fn baked(ctx: &Ctx<'_>, template: &str) -> String {
    let site = ctx.site();
    let text = template.replace("{{admin}}", &ctx.admin_url("")).replace("{{site}}", site.settings.siteurl.trim_end_matches('/'));
    super::template_text::translate(site, &text)
}

/// `Theme Name:` of the active theme's `style.css`.
fn theme_name(ctx: &Ctx<'_>) -> String {
    let slug = &ctx.site().settings.stylesheet;
    let css = std::fs::read(ctx.env.theme.stylesheet_dir().join("style.css")).unwrap_or_default();
    let head = String::from_utf8_lossy(&css[..css.len().min(8192)]).into_owned();
    head.lines().find_map(|line| line.trim_start_matches([' ', '*', '\t']).strip_prefix("Theme Name:").map(|name| name.trim().to_owned())).unwrap_or_else(|| slug.clone())
}

/// The markup of `customize.php` between `<body>` and the footer scripts.
fn body(ctx: &Ctx<'_>, return_url: &str) -> String {
    let tr = |text: &str| ctx.tr(text);
    let trx = |text: &str, context: &str| ctx.site().trx(text, context);
    let name = esc_html(&ctx.site().settings.blogname);
    let devices = [("desktop", tr("Desktop"), true), ("tablet", tr("Tablet"), false), ("mobile", tr("Mobile"), false)];
    let mut buttons = String::new();
    for (device, label, active) in devices {
        let class = if active { format!("preview-{device} active") } else { format!("preview-{device}") };
        buttons.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t<button type=\"button\" class=\"{}\" aria-pressed=\"{}\" data-device=\"{device}\">\n\t\t\t\t\t\t\t<span class=\"devices__preview-label\">{}</span>\n\t\t\t\t\t\t</button>\n",
            esc_attr(&class),
            if active { "1" } else { "" },
            esc_html(&label)
        ));
    }
    format!(
        "<div class=\"wp-full-overlay expanded\">\n\t<form id=\"customize-controls\" class=\"wrap wp-full-overlay-sidebar\">\n\t\t<div id=\"customize-header-actions\" class=\"wp-full-overlay-header\">\n\t\t\t\t\t\t\t\t\t\t\t\t\t\t<div id=\"customize-save-button-wrapper\" class=\"customize-save-button-wrapper\" >\n\t\t\t\t\t<input type=\"submit\" name=\"save\" id=\"save\" class=\"button button-primary button-compact save\" value=\"{publish}\"  />\t\t\t\t\t<button id=\"publish-settings\" class=\"publish-settings button-primary button-compact button dashicons dashicons-admin-generic\" aria-label=\"{settings}\" aria-expanded=\"false\" disabled></button>\n\t\t\t\t</div>\n\t\t\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t<button type=\"button\" class=\"customize-controls-preview-toggle\">\n\t\t\t\t<span class=\"controls\">{customize}</span>\n\t\t\t\t<span class=\"preview\">{preview}</span>\n\t\t\t</button>\n\t\t\t<a class=\"customize-controls-close\" href=\"{close_url}\">\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t{close}\t\t\t\t</span>\n\t\t\t</a>\n\t\t</div>\n\n\t\t<div id=\"customize-sidebar-outer-content\">\n\t\t\t<div id=\"customize-outer-theme-controls\">\n\t\t\t\t<ul class=\"customize-outer-pane-parent\"></ul>\n\t\t\t</div>\n\t\t</div>\n\n\t\t<div id=\"widgets-right\" class=\"wp-clearfix\"><!-- For Widget Customizer, many widgets try to look for instances under div#widgets-right, so we have to add that ID to a container div in the Customizer for compat -->\n\t\t\t<div id=\"customize-notifications-area\" class=\"customize-control-notifications-container\">\n\t\t\t\t<ul></ul>\n\t\t\t</div>\n\t\t\t<div class=\"wp-full-overlay-sidebar-content\" tabindex=\"-1\">\n\t\t\t\t<div id=\"customize-info\" class=\"accordion-section customize-info\" data-block-theme=\"{block_theme}\">\n\t\t\t\t\t<div class=\"accordion-section-title\">\n\t\t\t\t\t\t<h2 class=\"preview-notice\">\n\t\t\t\t\t\t{customizing}\t\t\t\t\t\t</h2>\n\t\t\t\t\t\t<button type=\"button\" class=\"customize-help-toggle dashicons dashicons-editor-help\" aria-expanded=\"false\"><span class=\"screen-reader-text\">\n\t\t\t\t\t\t\t{help}\t\t\t\t\t\t</span></button>\n\t\t\t\t\t</div>\n\t\t\t\t\t<div class=\"customize-panel-description\">\n\t\t\t\t\t\t<p>\n\t\t\t\t\t\t\t{about}\t\t\t\t\t\t</p>\n\t\t\t\t\t\t<p>\n\t\t\t\t\t\t\t{documentation}\t\t\t\t\t\t</p>\n\t\t\t\t\t</div>\n\t\t\t\t</div>\n\n\t\t\t\t<div id=\"customize-theme-controls\">\n\t\t\t\t\t<ul class=\"customize-pane-parent\"></ul>\n\t\t\t\t</div>\n\t\t\t</div>\n\t\t</div>\n\n\t\t<div id=\"customize-footer-actions\" class=\"wp-full-overlay-footer\">\n\t\t\t<button type=\"button\" class=\"collapse-sidebar button\" aria-expanded=\"true\" aria-label=\"{hide_label}\">\n\t\t\t\t<span class=\"collapse-sidebar-arrow\" aria-hidden=\"true\"></span>\n\t\t\t\t<span class=\"collapse-sidebar-label\">{hide}</span>\n\t\t\t</button>\n\t\t\t\t\t\t\t\t\t<div class=\"devices-wrapper\" role=\"group\" aria-label=\"{responsive}\">\n\t\t\t\t<div class=\"devices\">\n{buttons}\t\t\t\t\t\t\t\t\t</div>\n\t\t\t</div>\n\t\t\t\t\t</div>\n\t</form>\n\t<div id=\"customize-preview\" class=\"wp-full-overlay-main\"></div>\n\t",
        publish = esc_attr(&tr("Publish")),
        settings = esc_attr(&tr("Publish Settings")),
        customize = tr("Customize"),
        preview = esc_html(&trx("Preview", "noun")),
        close_url = esc_url(return_url),
        close = tr("Close the Customizer and go back to the previous page"),
        block_theme = i32::from(ctx.env.block_theme),
        customizing = sprintf(&tr("You are customizing %s"), &[&format!("<strong class=\"panel-title site-title\">{name}</strong>")]),
        help = tr("Help"),
        about = tr("The Customizer allows you to preview changes to your site before publishing them. You can navigate to different pages on your site within the preview. Edit shortcuts are shown for some editable elements. The Customizer is intended for use with non-block themes."),
        documentation = tr("<a href=\"https://wordpress.org/documentation/article/customizer/\">Documentation on Customizer</a>"),
        hide_label = esc_attr(&trx("Hide Controls", "label for hide controls button without length constraints")),
        hide = trx("Hide Controls", "short (~12 characters) label for hide controls button"),
        responsive = esc_attr(&trx("Responsive Views", "label for responsive previews group")),
    )
}

/// The addresses `customize.php` takes: `set_preview_url()` of `url`, `set_return_url()` of `return`, and what
/// `get_preview_url()` and `get_return_url()` make of them.
struct Urls {
    preview: String,
    close: String,
}

fn urls(ctx: &Ctx<'_>) -> Urls {
    let site = ctx.site();
    let home = site.home_url("/");
    let argument = |name: &str| ctx.request.request(name).filter(|value| !value.is_empty()).map(esc_url_raw).filter(|value| !value.is_empty());
    let preview = argument("url").map(|url| crate::login::validate_redirect(site, &esc_url_raw(&url), &home));
    let returned = argument("return").map(|url| {
        let mut url = esc_url_raw(&url);
        for arg in crate::chrome::REMOVABLE_QUERY_ARGS {
            url = rustpress_core::urls::remove_query_arg(&url, arg);
        }
        crate::login::validate_redirect(site, &url, "")
    });
    let close = match returned.filter(|url| !url.is_empty()) {
        Some(url) => {
            // A page of the Appearance menu that the theme does not add sends back to the themes.
            let path = url.split(['?', '#']).next().unwrap_or_default();
            let page = url.split_once('?').map(|(_, query)| query.split('#').next().unwrap_or_default()).and_then(|query| rustpress_core::canonical::parse_qs(query).into_iter().find(|(k, _)| k == "page").map(|(_, v)| v));
            match page {
                Some(page) if path.rsplit('/').next() == Some("themes.php") && !(page == "custom-header" && super::custom_header::available(ctx)) => ctx.admin_url("themes.php"),
                _ => url,
            }
        }
        None => match super::common::wp_get_referer(ctx) {
            Some(referer) if !matches!(referer.split(['?', '#']).next().unwrap_or_default().rsplit('/').next(), Some("customize.php" | "wp-login.php")) => referer,
            _ => preview.clone().unwrap_or_else(|| home.clone()),
        },
    };
    Urls { preview: preview.unwrap_or(home), close }
}

/// `set_autofocus()` of `autofocus[...]`: the panel, section and control to open.
fn autofocus(ctx: &Ctx<'_>) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for kind in ["panel", "section", "control"] {
        if let Some(value) = ctx.request.request(&format!("autofocus[{kind}]")) {
            out.insert(kind.to_owned(), json!(crate::sanitize::text_field(value)));
        }
    }
    if out.is_empty() { json!([]) } else { serde_json::Value::Object(out) }
}

/// The data of `_wpCustomizeHeader` (`WP_Customize_Header_Image_Control::enqueue()`), for a theme with a custom header.
async fn header_data(state: &State<'_>) -> Option<serde_json::Value> {
    let ctx = state.ctx;
    let support = state.support.as_ref()?;
    let site = ctx.site();
    let mut uploads = serde_json::Map::new();
    for uploaded in header::uploaded_header_images(site, &state.stylesheet).await.unwrap_or_default() {
        let meta = ctx.env.db.post_meta(uploaded.attachment_id).await.unwrap_or_default();
        let value = |name: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(name)).and_then(|m| m.meta_value.clone()).unwrap_or_default();
        let php = |v: &Php| rustpress_core::options::php_to_json(v);
        let mut entry = serde_json::Map::new();
        entry.insert("attachment_id".into(), json!(uploaded.attachment_id));
        entry.insert("url".into(), json!(uploaded.url));
        entry.insert("thumbnail_url".into(), json!(uploaded.url));
        entry.insert("alt_text".into(), json!(value("_wp_attachment_image_alt")));
        entry.insert("attachment_parent".into(), php(&uploaded.attachment_parent));
        if let Some(width) = &uploaded.width {
            entry.insert("width".into(), php(width));
        }
        if let Some(height) = &uploaded.height {
            entry.insert("height".into(), php(height));
        }
        entry.insert("timestamp".into(), json!(value(&format!("_wp_attachment_custom_header_last_used_{}", state.stylesheet))));
        uploads.insert(uploaded.attachment_id.to_string(), serde_json::Value::Object(entry));
    }
    let mut defaults = serde_json::Map::new();
    for default in header::default_headers(site, &ctx.env.theme.template, &state.template_uri) {
        defaults.insert(default.key.clone(), json!({"url": default.url, "thumbnail_url": default.thumbnail_url, "description": default.description}));
    }
    Some(json!({
        "data": {"width": support.width, "height": support.height, "flex-width": u8::from(support.flex_width), "flex-height": u8::from(support.flex_height), "currentImgSrc": null},
        "nonces": {"add": ctx.create_nonce("header-add"), "remove": ctx.create_nonce("header-remove")},
        "uploads": if uploads.is_empty() { json!([]) } else { serde_json::Value::Object(uploads) },
        "defaults": if defaults.is_empty() { json!([]) } else { serde_json::Value::Object(defaults) },
    }))
}

/// `customize_pane_settings()`.
fn pane_settings(state: &State<'_>, m: &Manager, preview_url: &str, return_url: &str, now: &str, autofocus: serde_json::Value) -> String {
    let ctx = state.ctx;
    let tr = |text: &str| ctx.tr(text);
    let site = ctx.site();
    let home = site.home_url("/");
    let can_publish = ctx.can("customize");
    let mut choices = Vec::new();
    if can_publish {
        choices.push(json!({"status": "publish", "label": tr("Publish")}));
    }
    choices.push(json!({"status": "draft", "label": tr("Save Draft")}));
    if can_publish {
        choices.push(json!({"status": "future", "label": site.trx("Schedule", "customizer changeset action/button label")}));
    }
    let (panels, sections) = m.containers(&tr);
    let mut nonces = serde_json::Map::new();
    nonces.insert("save".into(), json!(ctx.create_nonce(&format!("save-customize_{}", state.stylesheet))));
    nonces.insert("preview".into(), json!(ctx.create_nonce(&format!("preview-customize_{}", state.stylesheet))));
    nonces.insert("switch_themes".into(), json!(ctx.create_nonce("switch_themes")));
    nonces.insert("dismiss_autosave_or_lock".into(), json!(ctx.create_nonce("customize_dismiss_autosave_or_lock")));
    nonces.insert("override_lock".into(), json!(ctx.create_nonce("customize_override_changeset_lock")));
    nonces.insert("trash".into(), json!(ctx.create_nonce("trash_customize_changeset")));
    if !ctx.env.block_theme {
        nonces.insert("update-widget".into(), json!(ctx.create_nonce("update-widget")));
    }
    nonces.insert("customize-menus".into(), json!(ctx.create_nonce("customize-menus")));
    let uuid = uuid4();
    let millis = ctx.env.now * 1000;
    let settings = json!({
        "changeset": {
            "uuid": uuid,
            "branching": false,
            "autosaved": false,
            "hasAutosaveRevision": false,
            "latestAutoDraftUuid": null,
            "status": "",
            "currentUserCanPublish": can_publish,
            "publishDate": now,
            "statusChoices": choices,
            "lockUser": null,
        },
        "initialServerDate": now,
        "dateFormat": site.options.string("date_format", ""),
        "timeFormat": site.options.string("time_format", ""),
        "initialServerTimestamp": millis,
        "initialClientTimestamp": -1,
        "timeouts": {"windowRefresh": 250, "changesetAutoSave": 60000, "keepAliveCheck": 2500, "reflowPaneContents": 100, "previewFrameSensitivity": 2000},
        "theme": {"stylesheet": state.stylesheet, "active": true, "_canInstall": ctx.can("install_themes")},
        "url": {
            "preview": esc_url_raw(preview_url),
            "return": esc_url_raw(return_url),
            "parent": esc_url_raw(&ctx.admin_url("")),
            "activated": esc_url_raw(&home),
            "ajax": "/wp-admin/admin-ajax.php",
            "allowed": [esc_url_raw(&home)],
            "isCrossDomain": false,
            "home": esc_url_raw(&home),
            "login": esc_url_raw(&format!("{}?interim-login=1&customize-login=1", site.site_url("wp-login.php"))),
        },
        "browser": {"mobile": false, "ios": false},
        "panels": panels,
        "sections": sections,
        "nonce": nonces,
        "autofocus": autofocus,
        "documentTitleTmpl": rustpress_core::formatting::html_entity_decode(&tr("Customize: %s")),
        "previewableDevices": {"desktop": {"label": tr("Desktop"), "default": true}, "tablet": {"label": tr("Tablet")}, "mobile": {"label": tr("Mobile")}},
        "l10n": {
            "confirmDeleteTheme": tr("Are you sure you want to delete this theme?"),
            "themeSearchResults": tr("%d themes found"),
            "announceThemeCount": tr("Displaying %d themes"),
            "announceThemeDetails": tr("Theme details: %s"),
        },
    });
    let script = format!(
        "\n\t\t\tvar _wpCustomizeSettings = {};\n\t\t\t_wpCustomizeSettings.initialClientTimestamp = _.now();\n\t\t\t_wpCustomizeSettings.controls = {{}};\n\t\t\t_wpCustomizeSettings.settings = {{}};\n\t\t\t{}\t\t\n//# sourceURL=WP_Customize_Manager%3A%3Acustomize_pane_settings",
        inline_json(&settings),
        m.statements()
    );
    format!("<script>{script}\n</script>\n")
}

fn esc_url_raw(url: &str) -> String {
    rustpress_core::formatting::esc_url_raw(url)
}

/// `wp_generate_uuid4()`.
fn uuid4() -> String {
    let bytes: [u8; 16] = rand::random();
    let hex = |range: std::ops::Range<usize>| bytes[range].iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut out = format!("{}-{}-", hex(0..4), hex(4..6));
    out.push_str(&format!("{:04x}-", (u16::from_be_bytes([bytes[6], bytes[7]]) & 0x0fff) | 0x4000));
    out.push_str(&format!("{:04x}-", (u16::from_be_bytes([bytes[8], bytes[9]]) & 0x3fff) | 0x8000));
    out.push_str(&hex(10..16));
    out
}

/// What the page reads before the parts register.
async fn state<'a>(ctx: &'a Ctx<'a>) -> State<'a> {
    let theme = ctx.env.theme;
    let site = ctx.site();
    let stylesheet = site.settings.stylesheet.clone();
    let mods = rustpress_theme::classic::menu::theme_mod(site, &stylesheet);
    let template_uri = rustpress_blocks::env::ThemeAccess::template_directory_uri(theme);
    let stylesheet_uri = rustpress_blocks::env::ThemeAccess::stylesheet_directory_uri(theme);
    let support = if ctx.env.block_theme { None } else { header::support(&theme.template, &template_uri) };
    let custom_logo = (theme.template == "twentyseventeen").then_some((250, 250, true, false));
    let pages = rustpress_db::PostQuery { post_types: vec!["page".into()], statuses: vec!["publish".into()], limit: Some(1), ..Default::default() };
    let has_published_pages = ctx.env.db.count_posts(&pages).await.unwrap_or(0) > 0;
    let syntax_highlighting = ctx.env.db.user_meta(ctx.user.id, "syntax_highlighting").await.ok().flatten().as_deref() != Some("false");
    let custom_css = rustpress_theme::classic::wp_get_custom_css(site, &mods).await.unwrap_or_default();
    State { ctx, mods, support, template_uri, stylesheet_uri, stylesheet, theme_name: theme_name(ctx), custom_logo, has_published_pages, syntax_highlighting, custom_css }
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |text: &str| ctx.tr(text);
    let site = ctx.site();
    let state = state(ctx).await;
    let widgets_on = !ctx.env.block_theme;
    let menus = nav_menus::menus(&state).await;
    let locations = if ctx.env.theme.template == "twentyseventeen" { super::nav_menus::registered_locations(ctx) } else { Vec::new() };
    let mut m = Manager::default();
    // `customize_register`: the widgets at priority 1, the settings of WordPress and of the theme, then the menus.
    let widget_state = if widgets_on { Some(widgets::load(&state).await) } else { None };
    if let Some(w) = &widget_state {
        widgets::register(&state, &mut m, w);
    }
    register::core(&state, &mut m).await;
    if state.ctx.env.theme.template == "twentyseventeen" {
        register::twentyseventeen(&state, &mut m);
    }
    nav_menus::register(&state, &mut m, &menus, &locations, widgets_on).await;
    m.prepare(&|capability| ctx.can(capability));
    register::render_contents(&state, &mut m).await;
    register::control_json(&state, &mut m).await;

    let Urls { preview: preview_url, close: return_url } = urls(ctx);
    // `current_time( 'mysql' )`.
    let now = rustpress_core::datetime::SiteTimezone::from_options(&site.options).format("Y-m-d H:i:s", ctx.env.now);
    let menus_title = m.panel("nav_menus").map(|p| p.title.clone()).unwrap_or_default();
    let widgets_title = m.panel("widgets").map(|p| p.title.clone()).unwrap_or_else(|| tr("Widgets"));
    let header_data = header_data(&state).await;
    let mut assets = assets::assets(
        &state,
        &m,
        widget_state.as_ref().map(|w| widgets::settings_data(&state, w)),
        nav_menus::scripts_data(&state, &menus, &locations, &menus_title),
        header_data,
    )
    .await;

    // The control templates: `render_control_templates()` makes a control of each type, after the real ones.
    let blogname = site.settings.blogname.replace('\\', "\\\\").replace('\'', "\\'");
    let hidden_columns = super::common::user_option(ctx, "managenav-menuscolumnshidden").await.map(|value| rustpress_core::options::maybe_unserialize(&value)).and_then(|value| value.as_array().map(|items| items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect::<Vec<_>>())).unwrap_or_default();
    let templates = baked(ctx, &TEMPLATES.replace("{{blogname_js}}", &blogname))
        .replace("{{screen_options}}", &templates::screen_options(ctx, &hidden_columns))
        .replace("{{date_time}}", &templates::date_time(ctx, m.control_count() + 11))
        .replace("{{menu_locations}}", &templates::menu_locations(ctx, &locations));
    let mut footer = String::new();
    if let Some(w) = &widget_state {
        // `WP_Customize_Widgets::print_footer_scripts()`: the editor's settings and link dialog, the widget templates.
        let rich_editing = ctx.env.db.user_meta(ctx.user.id, "rich_editing").await.ok().flatten();
        footer.push_str(&super::editor::default_editor_scripts(ctx, super::editor::can_richedit(ctx, rich_editing.as_deref())));
        let dismissed = ctx.env.db.user_meta(ctx.user.id, "dismissed_wp_pointers").await.ok().flatten().unwrap_or_default();
        footer.push_str(&super::widgets::control_templates(ctx, &dismissed, true));
        footer.push_str(&widgets::control_templates(&state, w, &menus, &widgets_title));
    }
    footer.push_str(&baked(ctx, MENU_TEMPLATES));
    footer.push_str(&nav_menus::available_items(&state, &menus_title));
    footer.push_str(&baked(ctx, NOTICE_TEMPLATES));
    footer.push_str(&baked(ctx, THEME_TEMPLATES));
    // `_wp_footer_scripts()` of `customize.php`, unless the widgets printed the scripts already.
    if widget_state.is_none() {
        footer.push_str(crate::chrome::CUSTOMIZE_FOOTER_SCRIPTS);
    }
    footer.push_str(&super::media_ui::templates(ctx));
    if state.support.is_some() {
        footer.push_str(&baked(ctx, HEADER_TEMPLATES));
    }
    footer.push_str(&pane_settings(&state, &m, &preview_url, &return_url, &now, autofocus(ctx)));

    let locale = site.locale().replace('_', "-").to_lowercase();
    let mut body_class = format!("wp-core-ui wp-customizer js locale-{} admin-color-modern", sanitize_html_class(&locale));
    // `wp_use_widgets_block_editor()`.
    body_class.push_str(" wp-embed-responsive");
    assets.customize = Some(Box::new(CustomizeParts {
        title: esc_html(&sprintf(&rustpress_core::formatting::html_entity_decode(&tr("Customize: %s")), &[&tr("Loading&hellip;")])),
        body_class,
        body: body(ctx, &return_url),
        templates,
        footer,
        admin_hooks: widget_state.is_some(),
    }));
    let screen = Screen { file: "customize.php".into(), hook_suffix: Some("widgets.php".into()), title: String::new(), ..Default::default() };
    render(ctx, &screen, "", &assets).await
}
