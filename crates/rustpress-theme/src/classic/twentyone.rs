//! Twenty Twenty-One 2.9: the templates of the theme (`header.php`, `index.php`, `single.php`, `footer.php`, the files
//! in `template-parts/`) and what its `functions.php`, `inc/` and `classes/` add to the page.
//!
//! The text between the PHP tags is kept as it is in the theme's files, including the quirks of PHP's output (the
//! newline right after `?>` is dropped), because the result is compared byte for byte with the real theme.

use std::sync::Arc;

use rustpress_blocks::content::{get_the_content, post_class, the_content, the_title_for};
use rustpress_blocks::images::{self, AttrValue, ImageArgs};
use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::{Queried, Site};
use rustpress_db::Post;
use rustpress_phpser::Value;

use super::comments;
use super::menu::{self, MenuItem, NavArgs};
use super::tags::{self, Pagination};
use super::twentyone_icons as icons;
use super::widgets::{self, Sidebar};
use super::Ctx;
use crate::Result;
use crate::document::{Document, INLINE_LIMIT, body_classes, discovery_links, document_title, emoji_detection_script, emoji_style, oembed_links, read, script_modules, wp_version};
use crate::global_styles::GlobalStyles;

const THEME: &str = "twentytwentyone";

/// `wp_get_theme()->get( 'Version' )`, the version of every asset of the theme.
const VERSION: &str = "2.9";

/// The default of the `background_color` theme mod (`get_theme_mod( 'background_color', 'D1E4DD' )`).
const BACKGROUND: &str = "D1E4DD";

fn sidebar() -> Sidebar<'static> {
    Sidebar { id: "sidebar-1", before_widget: "<section id=\"%1$s\" class=\"widget %2$s\">", after_widget: "</section>", before_title: "<h2 class=\"widget-title\">", after_title: "</h2>" }
}

/// The sidebar `twenty_twenty_one_widgets_init()` registers.
pub fn registered_sidebars(site: &Site) -> Vec<widgets::RegisteredSidebar> {
    let wrap = sidebar();
    vec![widgets::RegisteredSidebar {
        id: wrap.id.to_owned(),
        name: esc_html(&site.i18n().gettext(THEME, "Footer")),
        description: esc_html(&site.i18n().gettext(THEME, "Add widgets here to appear in your footer.")),
        class: String::new(),
        before_widget: wrap.before_widget.to_owned(),
        after_widget: wrap.after_widget.to_owned(),
        before_title: wrap.before_title.to_owned(),
        after_title: wrap.after_title.to_owned(),
        show_in_rest: false,
    }]
}

/// Translates with the theme's text domain.
fn t(ctx: &Ctx<'_>, text: &str) -> String {
    ctx.site.i18n().gettext(THEME, text)
}

/// `esc_html__()` with the theme's text domain.
fn th(ctx: &Ctx<'_>, text: &str) -> String {
    esc_html(&t(ctx, text))
}

fn tx(ctx: &Ctx<'_>, text: &str, context: &str) -> String {
    ctx.site.i18n().pgettext(THEME, context, text)
}

/// `Twenty_Twenty_One_SVG_Icons::get_svg()`.
fn svg(group: &str, icon: &str, size: u32) -> String {
    let list = match group {
        "ui" => icons::UI,
        "social" => icons::SOCIAL,
        _ => &[],
    };
    let Some((_, source)) = list.iter().find(|(name, _)| *name == icon) else { return String::new() };
    let source = source.trim();
    match source.strip_prefix("<svg ") {
        Some(rest) => format!("<svg class=\"svg-icon\" width=\"{size}\" height=\"{size}\" aria-hidden=\"true\" role=\"img\" focusable=\"false\" {rest}"),
        None => source.to_owned(),
    }
}

/// `twenty_twenty_one_get_icon_svg( 'ui', $icon )`.
fn ui(icon: &str) -> String {
    svg("ui", icon, 24)
}

/// `Twenty_Twenty_One_SVG_Icons::get_social_link_svg()`: the icon of the first network whose domains the address
/// contains (case-insensitively), and the opening of the screen reader text that replaces the `<span>` of the link.
fn social_link_svg(uri: &str, size: u32) -> Option<String> {
    let uri = uri.to_lowercase();
    icons::SOCIAL.iter().find_map(|(icon, _)| {
        let own = [format!("{icon}.com")];
        let domains: Vec<String> = match icons::SOCIAL_MAP.iter().find(|(name, _)| name == icon) {
            Some((_, domains)) => domains.iter().map(|d| d.trim().to_lowercase()).collect(),
            None => own.to_vec(),
        };
        domains.iter().any(|d| uri.contains(d.as_str())).then(|| format!("{}<span class=\"screen-reader-text\">", svg("social", icon, size)))
    })
}

// ---------------------------------------------------------------- conditionals

/// `is_customize_preview()`.
fn preview<'c>(ctx: &'c Ctx<'_>) -> Option<&'c rustpress_core::customize::Preview> {
    ctx.q.customize.as_deref()
}

/// `is_active_sidebar()`, which the Customizer counts as a sidebar of the page.
fn active_sidebar(ctx: &Ctx<'_>, id: &str) -> bool {
    if let Some(preview) = preview(ctx) {
        preview.tally_sidebar(id);
    }
    widgets::is_active_sidebar(ctx.site, id)
}

/// `true === get_theme_mod( 'display_title_and_tagline', true )`.
fn shows_title(ctx: &Ctx<'_>) -> bool {
    match ctx.mods.get("display_title_and_tagline") {
        None => true,
        Some(value) => matches!(value, Value::Bool(true)),
    }
}

/// `get_theme_mod( 'display_title_and_tagline', true )` as a condition.
fn shows_title_loosely(ctx: &Ctx<'_>) -> bool {
    match ctx.mods.get("display_title_and_tagline") {
        None => true,
        Some(value) => php_truthy(value),
    }
}

fn php_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !(s.is_empty() || s.as_slice() == b"0"),
        Value::Array(a) => !a.is_empty(),
        _ => true,
    }
}

/// `get_theme_mod( 'background_color', 'D1E4DD' )`.
fn background_color(mods: &Value) -> String {
    match mods.get("background_color") {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => BACKGROUND.to_owned(),
    }
}

/// `get_background_color()`: the mod, or the lowercase `default-color` of the theme's `custom-background` support.
fn custom_background_color(mods: &Value) -> String {
    match mods.get("background_color") {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => BACKGROUND.to_lowercase(),
    }
}

/// `Twenty_Twenty_One_Custom_Colors::get_relative_luminance_from_hex()`.
fn luminance(hex: &str) -> i64 {
    let mut hex = hex.trim_start_matches('#').to_owned();
    if hex.len() == 3 {
        hex = hex.chars().flat_map(|c| [c, c]).collect();
    }
    // `hexdec()` skips what is not a hexadecimal digit.
    let channel = |from: usize| -> f64 {
        let part: String = hex.chars().skip(from).take(2).filter(char::is_ascii_hexdigit).collect();
        i64::from_str_radix(&part, 16).unwrap_or(0) as f64
    };
    let lum = 0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4);
    lum.round() as i64
}

/// `Twenty_Twenty_One_Custom_Colors::custom_get_readable_color()`.
fn readable_color(background: &str) -> &'static str {
    if 127 < luminance(background) { "#000" } else { "#fff" }
}

/// `Twenty_Twenty_One_Dark_Mode::switch_should_render()`.
fn dark_mode(ctx: &Ctx<'_>) -> bool {
    ctx.mods.get("respect_user_color_preference").is_some_and(php_truthy) && 127 <= luminance(&background_color(&ctx.mods))
}

/// `has_custom_logo()`.
async fn has_custom_logo(ctx: &Ctx<'_>) -> Result<bool> {
    Ok(images::custom_logo_id(ctx.site).await? != 0)
}

/// `get_bloginfo( $show, 'display' )`: the filters of `bloginfo`.
fn bloginfo_display(ctx: &Ctx<'_>, value: &str) -> String {
    esc_html(&rustpress_core::formatting::convert_chars(&ctx.site.texturize(value)))
}

/// `is_front_page() && ! is_paged()`.
fn front_unpaged(ctx: &Ctx<'_>) -> bool {
    ctx.q.flags.is_front_page && !ctx.q.flags.is_paged
}

/// `the_custom_logo()` with `unlink-homepage-logo`.
async fn custom_logo(ctx: &Ctx<'_>) -> Result<String> {
    let id = images::custom_logo_id(ctx.site).await?;
    if id == 0 {
        return Ok(match preview(ctx) {
            Some(_) => format!("<a href=\"{}\" class=\"custom-logo-link\" style=\"display:none;\"><img class=\"custom-logo\" alt=\"\" /></a>", esc_url(&ctx.site.home_url("/"))),
            None => String::new(),
        });
    }
    let unlinked = front_unpaged(ctx);
    let mut attrs: Vec<(String, AttrValue)> = vec![("class".into(), AttrValue::Text("custom-logo".into())), ("loading".into(), AttrValue::Omit)];
    if unlinked {
        attrs.push(("alt".into(), AttrValue::Text(String::new())));
    } else {
        let rows = ctx.site.db.post_meta(id).await?;
        let alt = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
        if alt.is_empty() || alt == "0" {
            attrs.push(("alt".into(), AttrValue::Text(bloginfo_display(ctx, &ctx.site.settings.blogname))));
        }
    }
    let image = images::get_attachment_image(&ctx.env, id, "full", &ImageArgs { attrs, ..ImageArgs::default() }).await?;
    if image.is_empty() {
        return Ok(String::new());
    }
    if unlinked {
        return Ok(format!("<span class=\"custom-logo-link\">{image}</span>"));
    }
    let q = ctx.q;
    let current = !q.flags.is_paged && (q.flags.is_front_page || (q.flags.is_home && ctx.site.settings.page_for_posts != q.queried_object_id()));
    let aria = if current { " aria-current=\"page\"" } else { "" };
    Ok(format!("<a href=\"{}\" class=\"custom-logo-link\" rel=\"home\"{aria}>{image}</a>", esc_url(&ctx.site.home_url("/"))))
}

// ---------------------------------------------------------------------- header

/// `template-parts/header/site-branding.php`.
async fn site_branding(ctx: &Ctx<'_>) -> Result<String> {
    let site = ctx.site;
    let q = ctx.q;
    let show_title = shows_title(ctx);
    let class = if show_title { "site-title" } else { "screen-reader-text" };
    let logo = if has_custom_logo(ctx).await? { Some(custom_logo(ctx).await?) } else { None };
    let mut out = String::from("\n");
    if let (Some(logo), true) = (&logo, show_title) {
        out.push_str(&format!("\t<div class=\"site-logo\">{logo}</div>\n"));
    }
    out.push_str("\n<div class=\"site-branding\">\n\n\t");
    if let (Some(logo), false) = (&logo, show_title) {
        out.push_str(&format!("\t\t<div class=\"site-logo\">{logo}</div>\n\t"));
    }
    out.push_str("\n\t");
    // `get_bloginfo( 'name' )` without the display filters, escaped once.
    let name = &site.settings.blogname;
    if !name.is_empty() {
        out.push_str("\t\t");
        let home = esc_url(&site.home_url("/"));
        let name = esc_html(name);
        if front_unpaged(ctx) {
            out.push_str(&format!("\t\t\t<h1 class=\"{class}\">{name}</h1>\n\t\t"));
        } else if q.flags.is_front_page && !q.flags.is_home {
            out.push_str(&format!("\t\t\t<h1 class=\"{class}\"><a href=\"{home}\" rel=\"home\">{name}</a></h1>\n\t\t"));
        } else {
            out.push_str(&format!("\t\t\t<p class=\"{class}\"><a href=\"{home}\" rel=\"home\">{name}</a></p>\n\t\t"));
        }
        out.push('\t');
    }
    out.push_str("\n\t");
    let description = bloginfo_display(ctx, &site.settings.blogdescription);
    if !description.is_empty() && show_title {
        out.push_str(&format!("\t\t<p class=\"site-description\">\n\t\t\t{description}\t\t</p>\n\t"));
    }
    out.push_str("</div><!-- .site-branding -->\n");
    Ok(out)
}

/// `twenty_twenty_one_change_calendar_nav_arrows()` on `get_calendar`.
pub fn calendar_nav_arrows(calendar: String, rtl: bool) -> String {
    calendar.replace("&laquo; ", &ui(if rtl { "arrow_right" } else { "arrow_left" })).replace(" &raquo;", &ui(if rtl { "arrow_left" } else { "arrow_right" }))
}

/// `twenty_twenty_one_add_sub_menu_toggle()` and `twenty_twenty_one_nav_menu_social_icons()` on
/// `walker_nav_menu_start_el`.
fn start_el(ctx: &Ctx<'_>, location: &str, link_before: &str, output: String, item: &MenuItem, depth: usize) -> String {
    let mut output = output;
    if location == "primary" && depth == 0 && item.classes.iter().any(|c| c == "menu-item-has-children") {
        output.push_str(&format!(
            "<button class=\"sub-menu-toggle\" aria-expanded=\"false\" onClick=\"twentytwentyoneExpandSubMenu(this)\"><span class=\"icon-plus\">{}</span><span class=\"icon-minus\">{}</span><span class=\"screen-reader-text\">{}</span></button>",
            svg("ui", "plus", 18),
            svg("ui", "minus", 18),
            th(ctx, "Open menu")
        ));
    }
    if location == "footer"
        && let Some(icon) = social_link_svg(&item.url, 24)
    {
        output = output.replace(link_before, &icon);
    }
    output
}

/// `twenty_twenty_one_add_menu_description_args()` on `nav_menu_item_args`.
fn menu_item_args(link_after: &mut String, item: &MenuItem, depth: usize) {
    if link_after != "</span>" {
        link_after.clear();
    }
    if depth == 0 && !item.description.is_empty() {
        *link_after = format!("<p class=\"menu-item-description\"><span>{}</span></p>", item.description);
    }
}

/// `wp_nav_menu()` for one of the theme's locations.
async fn nav_menu(ctx: &Ctx<'_>, location: &str) -> Result<String> {
    let Some(id) = menu::menu_location(&ctx.mods, location) else { return Ok(String::new()) };
    let Some(mut found) = menu::load_menu(ctx.site, id).await? else { return Ok(String::new()) };
    let uri = ctx.request_uri();
    menu::classes_by_context(ctx.site, ctx.q, &uri, &mut found.items).await?;
    let mut args = NavArgs::new(location);
    args.fallback_cb = "";
    args.customize = ctx.q.customize.clone();
    args.item_args = Some(menu_item_args);
    if location == "primary" {
        args.menu_class = "menu-wrapper".into();
        args.container_class = "primary-menu-container".into();
        args.items_wrap = "<ul id=\"primary-menu-list\" class=\"%2$s\">%3$s</ul>".into();
    } else {
        args.items_wrap = "%3$s".into();
        args.container = None;
        args.depth = 1;
        args.link_before = "<span>".into();
        args.link_after = "</span>".into();
    }
    let title_hook = |title: &str, _: &MenuItem, _: usize| title.to_owned();
    let link_before = args.link_before.clone();
    let link_hook = |link: String, item: &MenuItem, depth: usize| start_el(ctx, location, &link_before, link, item, depth);
    args.use_ids_once(&ctx.env, &found.items);
    Ok(menu::wp_nav_menu(ctx.site, &found, &found.items, &args, &title_hook, &link_hook))
}

/// `template-parts/header/site-nav.php`.
async fn site_nav(ctx: &Ctx<'_>) -> Result<String> {
    if !menu::has_nav_menu(ctx.site, &ctx.mods, "primary").await? {
        return Ok("\n".to_owned());
    }
    Ok(format!(
        "\n\t<nav id=\"site-navigation\" class=\"primary-navigation\" aria-label=\"{label}\">\n\t\t<div class=\"menu-button-container\">\n\t\t\t<button id=\"primary-mobile-menu\" class=\"button\" aria-controls=\"primary-menu-list\" aria-expanded=\"false\">\n\t\t\t\t<span class=\"dropdown-icon open\">{menu}\t\t\t\t\t{menu_icon}\t\t\t\t</span>\n\t\t\t\t<span class=\"dropdown-icon close\">{close}\t\t\t\t\t{close_icon}\t\t\t\t</span>\n\t\t\t</button><!-- #primary-mobile-menu -->\n\t\t</div><!-- .menu-button-container -->\n\t\t{nav}\t</nav><!-- #site-navigation -->\n\t",
        label = esc_attr(&t(ctx, "Primary menu")),
        menu = th(ctx, "Menu"),
        menu_icon = ui("menu"),
        close = th(ctx, "Close"),
        close_icon = ui("close"),
        nav = nav_menu(ctx, "primary").await?,
    ))
}

/// `header.php` from `<body>` to `<main>`, with `template-parts/header/site-header.php`.
async fn header(ctx: &Ctx<'_>, classes: &[String], bar: &str) -> Result<String> {
    ctx.env.with_state(|s| s.media.in_header = true);
    let mut wrapper = String::from("site-header");
    if has_custom_logo(ctx).await? {
        wrapper.push_str(" has-logo");
    }
    if shows_title(ctx) {
        wrapper.push_str(" has-title-and-tagline");
    }
    if menu::has_nav_menu(ctx.site, &ctx.mods, "primary").await? {
        wrapper.push_str(" has-menu");
    }
    let branding = site_branding(ctx).await?;
    let nav = site_nav(ctx).await?;
    ctx.env.with_state(|s| s.media.in_header = false);
    Ok(format!(
        "<body class=\"{}\">\n{bar}<div id=\"page\" class=\"site\">\n\t<a class=\"skip-link screen-reader-text\" href=\"#content\">\n\t\t{}\t</a>\n\n\t\n<header id=\"masthead\" class=\"{}\">\n\n\t{branding}\t{nav}\n</header><!-- #masthead -->\n\n\t<div id=\"content\" class=\"site-content\">\n\t\t<div id=\"primary\" class=\"content-area\">\n\t\t\t<main id=\"main\" class=\"site-main\">\n",
        esc_attr(&classes.join(" ")),
        th(ctx, "Skip to content"),
        esc_attr(&wrapper),
    ))
}

// ---------------------------------------------------------------------- footer

/// `the_privacy_policy_link( $before, $after )`.
async fn privacy_policy_link(ctx: &Ctx<'_>, before: &str, after: &str) -> Result<String> {
    let id = ctx.site.options.u64("wp_page_for_privacy_policy", 0);
    if id == 0 {
        return Ok(String::new());
    }
    let Some(page) = ctx.site.db.post(id).await? else { return Ok(String::new()) };
    if page.post_status != "publish" {
        return Ok(String::new());
    }
    let title = the_title_for(ctx.site, &page);
    if title.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("{before}<a class=\"privacy-policy-link\" href=\"{}\" rel=\"privacy-policy\">{title}</a>{after}", esc_url(&ctx.site.permalink(&page).await?)))
}

/// `footer.php` with `template-parts/footer/footer-widgets.php`.
async fn footer(ctx: &Ctx<'_>) -> Result<String> {
    let mut out = String::from("\t\t\t</main><!-- #main -->\n\t\t</div><!-- #primary -->\n\t</div><!-- #content -->\n\n\t");
    if active_sidebar(ctx, "sidebar-1") {
        let request_uri = ctx.request_uri();
        let search = || search_form_markup(ctx.site, ctx.q, &ctx.env, "");
        let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
        let widgets_html = widgets::dynamic_sidebar(&services, &sidebar()).await?;
        out.push_str(&format!("\n\t<aside class=\"widget-area\">\n\t\t{widgets_html}\t</aside><!-- .widget-area -->\n\n\t"));
    }
    out.push_str("\n\t<footer id=\"colophon\" class=\"site-footer\">\n\n\t\t");
    if menu::has_nav_menu(ctx.site, &ctx.mods, "footer").await? {
        out.push_str(&format!(
            "\t\t\t<nav aria-label=\"{}\" class=\"footer-navigation\">\n\t\t\t\t<ul class=\"footer-navigation-wrapper\">\n\t\t\t\t\t{}\t\t\t\t</ul><!-- .footer-navigation-wrapper -->\n\t\t\t</nav><!-- .footer-navigation -->\n\t\t",
            esc_attr(&t(ctx, "Secondary menu")),
            nav_menu(ctx, "footer").await?
        ));
    }
    out.push_str("\t\t<div class=\"site-info\">\n\t\t\t<div class=\"site-name\">\n\t\t\t\t");
    if has_custom_logo(ctx).await? {
        out.push_str(&format!("\t\t\t\t\t<div class=\"site-logo\">{}</div>\n\t\t\t\t", custom_logo(ctx).await?));
    } else {
        out.push_str("\t\t\t\t\t");
        if !ctx.site.settings.blogname.is_empty() && shows_title_loosely(ctx) {
            let name = bloginfo_display(ctx, &ctx.site.settings.blogname);
            out.push_str("\t\t\t\t\t\t\t\t\t\t\t\t\t");
            if front_unpaged(ctx) {
                out.push_str(&name);
            } else {
                out.push_str(&format!("<a href=\"{}\">{name}</a>\n", esc_url(&ctx.site.home_url("/"))));
            }
            out.push_str("\t\t\t\t\t\t\t\t\t\t\t");
        }
        out.push_str("\t\t\t\t");
    }
    let privacy = privacy_policy_link(ctx, "<div class=\"privacy-policy\">", "</div>").await?;
    // The credit names RustPress (the compat run ignores it); it points at the site itself.
    let powered = sprintf(&th(ctx, "Proudly powered by %s."), &[&format!("<a href=\"{}\">RustPress</a>", esc_url(&ctx.site.home_url("/")))]);
    out.push_str(&format!(
        "\t\t\t</div><!-- .site-name -->\n\n\t\t\t{privacy}\n\t\t\t<div class=\"powered-by\">\n\t\t\t\t{powered}\t\t\t</div><!-- .powered-by -->\n\n\t\t</div><!-- .site-info -->\n\t</footer><!-- #colophon -->\n\n</div><!-- #page -->\n\n"
    ));
    Ok(out)
}

// ------------------------------------------------------------------- the loop

/// `get_query_var( 'page' )` as `WP_Query::generate_postdata()` takes it.
fn current_page(ctx: &Ctx<'_>) -> u64 {
    ctx.q.vars.u64("page").filter(|page| *page != 0).unwrap_or(1)
}

/// `get_post_format()`.
async fn post_format(ctx: &Ctx<'_>, post: &Post) -> Result<Option<String>> {
    if post.post_type != "post" {
        return Ok(None);
    }
    let terms = ctx.site.db.terms_for_post(post.id, Some("post_format")).await?;
    Ok(terms.first().and_then(|t| t.slug.strip_prefix("post-format-")).map(str::to_owned))
}

/// `post_class()` (`twenty_twenty_one_post_classes()` adds `entry` through the theme's filter).
async fn classes_of(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let q = ctx.q;
    let classes = post_class(&ctx.env, post, &[], q.flags.is_home && !q.flags.is_paged, q.flags.is_singular()).await?;
    Ok(esc_attr(&classes.join(" ")))
}

/// `twenty_twenty_one_continue_reading_text()` for a title.
fn continue_reading_text(i18n: &rustpress_core::i18n::I18n, title: &str) -> String {
    sprintf(&esc_html(&i18n.gettext(THEME, "Continue reading %s")), &[&format!("<span class=\"screen-reader-text\">{title}</span>")])
}

/// `twenty_twenty_one_continue_reading_link_excerpt()` as a template (`%1$s` permalink, `%2$s` title).
pub fn excerpt_more_template(i18n: &rustpress_core::i18n::I18n) -> String {
    format!("&hellip; <a class=\"more-link\" href=\"%1$s\">{}</a>", continue_reading_text(i18n, "%2$s"))
}

/// `twenty_twenty_one_continue_reading_link()` on `the_content_more_link`.
pub fn content_more_link(i18n: &rustpress_core::i18n::I18n, permalink: &str, post_id: u64, title: &str) -> String {
    format!("<div class=\"more-link-container\"><a class=\"more-link\" href=\"{permalink}#more-{post_id}\">{}</a></div>", continue_reading_text(i18n, title))
}

/// `twenty_twenty_one_password_form()` on `the_password_form`.
pub fn password_form(site: &Site, post: &Post) -> String {
    let i18n = site.i18n();
    let label = format!("pwbox-{}", post.id);
    format!(
        "<p class=\"post-password-message\">{}</p>\n\t<form action=\"{}\" class=\"post-password-form\" method=\"post\">\n\t<label class=\"post-password-form__label\" for=\"{label}\">{}</label><input class=\"post-password-form__input\" name=\"post_password\" id=\"{label}\" type=\"password\" spellcheck=\"false\" size=\"20\" /><input type=\"submit\" class=\"post-password-form__submit\" name=\"{}\" value=\"{}\" /></form>\n\t",
        esc_html(&i18n.gettext(THEME, "This content is password protected. Please enter a password to view.")),
        esc_url(&site.site_url("wp-login.php?action=postpass")),
        esc_html(&i18n.pgettext(THEME, "Post password form", "Password")),
        esc_attr(&i18n.pgettext(THEME, "Post password form", "Submit")),
        esc_attr(&i18n.pgettext(THEME, "Post password form", "Enter")),
    )
}

/// `twenty_twenty_one_get_attachment_image_attributes()` on `wp_get_attachment_image_attributes`.
pub fn attachment_image_attributes(attrs: &mut Vec<(String, AttrValue)>, full: Option<(u64, u64)>, requested: Option<(u64, u64)>) {
    let text = |attrs: &[(String, AttrValue)], key: &str| attrs.iter().find(|(k, _)| k == key).and_then(|(_, v)| if let AttrValue::Text(t) = v { Some(t.clone()) } else { None });
    if text(attrs, "class").is_some_and(|class| class.contains("custom-logo")) {
        return;
    }
    let Some((width, height)) = requested.or(full) else { return };
    if width == 0 || height == 0 {
        return;
    }
    let ratio = (100.0 * height as f64 / width as f64 * 100.0).round() / 100.0;
    let style = format!("width:100%;height:{}%;max-width:{width}px;{}", rustpress_blocks::general::php_float(ratio), text(attrs, "style").unwrap_or_default());
    match attrs.iter_mut().find(|(k, _)| k == "style") {
        Some(slot) => slot.1 = AttrValue::Text(style),
        None => attrs.push(("style".into(), AttrValue::Text(style))),
    }
}

/// `twenty_twenty_one_can_show_post_thumbnail()`.
async fn can_show_thumbnail(ctx: &Ctx<'_>, post: &Post) -> Result<bool> {
    Ok(!rustpress_blocks::content::post_password_required(post) && !ctx.q.flags.is_attachment && images::post_thumbnail_id(&ctx.env, post.id).await? != 0)
}

/// `wp_get_attachment_caption()`.
async fn attachment_caption(ctx: &Ctx<'_>, id: u64) -> Result<String> {
    let Some(attachment) = ctx.site.db.post(id).await? else { return Ok(String::new()) };
    if attachment.post_type != "attachment" {
        return Ok(String::new());
    }
    Ok(attachment.post_excerpt)
}

/// `twenty_twenty_one_post_thumbnail()`.
async fn post_thumbnail(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    if !can_show_thumbnail(ctx, post).await? {
        return Ok(String::new());
    }
    let id = images::post_thumbnail_id(&ctx.env, post.id).await?;
    let caption = attachment_caption(ctx, id).await?;
    let caption = if caption.is_empty() {
        String::new()
    } else {
        format!("\t\t\t\t\t<figcaption class=\"wp-caption-text\">{}</figcaption>\n\t\t\t\t", rustpress_core::kses::kses_post(&caption))
    };
    if ctx.q.flags.is_singular() {
        let args = ImageArgs { attrs: vec![("loading".into(), AttrValue::Omit)], post_thumbnail: true, thumbnail_of: Some(post.id), ..ImageArgs::default() };
        let image = images::get_attachment_image(&ctx.env, id, "post-thumbnail", &args).await?;
        Ok(format!("\n\t\t\n\t\t\t<figure class=\"post-thumbnail\">\n\t\t\t\t{image}\t\t\t\t{caption}\t\t\t</figure><!-- .post-thumbnail -->\n\n\t\t\t\t"))
    } else {
        let args = ImageArgs { post_thumbnail: true, thumbnail_of: Some(post.id), ..ImageArgs::default() };
        let image = images::get_attachment_image(&ctx.env, id, "post-thumbnail", &args).await?;
        Ok(format!(
            "\n\t\t\n\t\t\t<figure class=\"post-thumbnail\">\n\t\t\t\t<a class=\"post-thumbnail-inner alignwide\" href=\"{}\" aria-hidden=\"true\" tabindex=\"-1\">\n\t\t\t\t\t{image}\t\t\t\t</a>\n\t\t\t\t{caption}\t\t\t</figure><!-- .post-thumbnail -->\n\n\t\t\t\t",
            esc_url(&ctx.site.permalink(post).await?)
        ))
    }
}

/// `get_edit_post_link()`: empty for who may not edit the post.
fn edit_post_url(ctx: &Ctx<'_>, post: &Post) -> Option<String> {
    if !ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.can_edit_post(post)) {
        return None;
    }
    Some(ctx.site.site_url(&format!("wp-admin/post.php?post={}&action=edit", post.id)))
}

/// `edit_post_link( sprintf( 'Edit %s', ... ), $before, $after )`.
fn edit_link(ctx: &Ctx<'_>, post: &Post, before: &str, after: &str) -> String {
    let Some(url) = edit_post_url(ctx, post) else { return String::new() };
    let text = sprintf(&th(ctx, "Edit %s"), &[&format!("<span class=\"screen-reader-text\">{}</span>", the_title_for(ctx.site, post))]);
    format!("{before}<a class=\"post-edit-link\" href=\"{}\">{text}</a>{after}", esc_url(&url))
}

/// `twenty_twenty_one_posted_on()`.
async fn posted_on(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let site = ctx.site;
    let (machine, human) = if site.plugins.hides_date(site, post).await? {
        (String::new(), String::new())
    } else {
        let posted = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
        (site.date("c", posted), site.date(&site.options.string("date_format", "F j, Y"), posted))
    };
    let time = format!("<time class=\"entry-date published updated\" datetime=\"{}\">{}</time>", esc_attr(&machine), esc_html(&human));
    Ok(format!("<span class=\"posted-on\">{}</span>", sprintf(&th(ctx, "Published %s"), &[&time])))
}

/// The author's `description` (`get_the_author_meta( 'description' )`).
async fn author_description(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    Ok(ctx.site.db.user_meta(post.post_author, "description").await?.unwrap_or_default())
}

/// `post_type_supports( $type, 'author' )`.
fn supports_author(ctx: &Ctx<'_>, post: &Post) -> bool {
    match post.post_type.as_str() {
        "post" | "page" | "attachment" => true,
        other => ctx.site.types.post_type(other).is_some_and(|t| t.supports.as_ref().is_some_and(|s| s.iter().any(|s| s == "author"))),
    }
}

/// `twenty_twenty_one_posted_by()`.
async fn posted_by(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    if !author_description(ctx, post).await?.is_empty() || !supports_author(ctx, post) {
        return Ok(String::new());
    }
    let Some(user) = ctx.site.db.user(post.post_author).await? else {
        // `get_author_posts_url()` of a user that does not exist: the author base without a name.
        let site = ctx.site;
        let url = if site.uses_pretty_permalinks() { site.home_url(&site.user_trailingslashit("/author/")) } else { site.home_url(&format!("?author={}", post.post_author)) };
        let link = format!("<a href=\"{}\" rel=\"author\"></a>", esc_url(&url));
        return Ok(format!("<span class=\"byline\">{}</span>", sprintf(&th(ctx, "By %s"), &[&link])));
    };
    let link = format!("<a href=\"{}\" rel=\"author\">{}</a>", esc_url(&ctx.site.author_url(&user)), esc_html(&user.display_name));
    Ok(format!("<span class=\"byline\">{}</span>", sprintf(&th(ctx, "By %s"), &[&link])))
}

/// The categories and tags of `twenty_twenty_one_entry_meta_footer()`.
async fn taxonomies(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let site = ctx.site;
    let has_category = !site.db.terms_for_post(post.id, Some("category")).await?.is_empty();
    let has_tag = !site.db.terms_for_post(post.id, Some("post_tag")).await?.is_empty();
    if !has_category && !has_tag {
        return Ok(String::new());
    }
    // `wp_get_list_item_separator()`: core's, since WordPress has it.
    let separator = site.tr(", ");
    let mut out = String::from("<div class=\"post-taxonomies\">");
    let categories = tags::category_list(site, post, &separator).await?;
    if !categories.is_empty() {
        out.push_str(&sprintf(&format!("<span class=\"cat-links\">{} </span>", th(ctx, "Categorized as %s")), &[&categories]));
    }
    let tag_list = tags::tag_list(site, post, &separator).await?;
    if !tag_list.is_empty() {
        out.push_str(&sprintf(&format!("<span class=\"tags-links\">{}</span>", th(ctx, "Tagged %s")), &[&tag_list]));
    }
    out.push_str("</div>");
    Ok(out)
}

/// `twenty_twenty_one_entry_meta_footer()`.
async fn entry_meta_footer(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    if post.post_type != "post" {
        return Ok(String::new());
    }
    let site = ctx.site;
    let mut out = String::new();
    if !ctx.q.flags.is_single {
        if site.settings.sticky_posts.contains(&post.id) {
            out.push_str(&format!("<p>{}</p>", esc_html(&tx(ctx, "Featured post", "Label for sticky posts"))));
        }
        if matches!(post_format(ctx, post).await?.as_deref(), Some("aside" | "status")) {
            out.push_str(&format!("<p><a href=\"{}\">{}</a></p>", esc_url(&site.permalink(post).await?), continue_reading_text(&site.i18n(), &the_title_for(site, post))));
        }
        out.push_str(&posted_on(ctx, post).await?);
        out.push_str(&edit_link(ctx, post, "<span class=\"edit-link\">", "</span><br>"));
    } else {
        out.push_str("<div class=\"posted-by\">");
        out.push_str(&posted_on(ctx, post).await?);
        out.push_str(&posted_by(ctx, post).await?);
        out.push_str(&edit_link(ctx, post, "<span class=\"edit-link\">", "</span>"));
        out.push_str("</div>");
    }
    out.push_str(&taxonomies(ctx, post).await?);
    Ok(out)
}

/// `wp_link_pages()` with the arguments of the theme.
async fn link_pages(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let pages = rustpress_blocks::content::post_pages(&post.post_content);
    if pages.len() < 2 {
        return Ok(String::new());
    }
    let site = ctx.site;
    let q = ctx.q;
    let page = current_page(ctx).min(pages.len() as u64);
    let more = (q.queried_object_id() == post.id && (q.flags.is_page || q.flags.is_single)) || q.flags.is_feed || page > 1;
    let permalink = site.permalink(post).await?;
    let pagelink = th(ctx, "Page %");
    let mut out = format!("<nav class=\"page-links\" aria-label=\"{}\">", esc_attr(&t(ctx, "Page")));
    for number in 1..=pages.len() as u64 {
        let label = pagelink.replace('%', &number.to_string());
        let link = if number != page || (!more && page == 1) {
            // `_wp_link_page()`.
            let url = if number == 1 {
                permalink.clone()
            } else if !site.uses_pretty_permalinks() || matches!(post.post_status.as_str(), "draft" | "pending") {
                rustpress_core::urls::add_query_arg(&permalink, "page", &number.to_string())
            } else if site.settings.show_on_front == rustpress_core::settings::ShowOnFront::Page && site.settings.page_on_front == post.id {
                format!("{}/{}", permalink.trim_end_matches('/'), site.user_trailingslashit(&format!("page/{number}")))
            } else {
                format!("{}/{}", permalink.trim_end_matches('/'), site.user_trailingslashit(&number.to_string()))
            };
            format!("<a href=\"{}\" class=\"post-page-numbers\">{label}</a>", esc_url(&url))
        } else {
            format!("<span class=\"post-page-numbers current\" aria-current=\"page\">{label}</span>")
        };
        out.push(' ');
        out.push_str(&link);
    }
    out.push_str("</nav>");
    Ok(out)
}

/// `the_content()` of the post in the loop and its `wp_link_pages()`.
async fn content_and_pages(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let raw = get_the_content(&ctx.env, post, current_page(ctx), ctx.q.flags.is_singular()).await?;
    let mut out = the_content(&ctx.env, post, &raw).await?;
    out.push_str(&link_pages(ctx, post).await?);
    Ok(out)
}

/// `template-parts/content/content.php`: a post with its full content (`display_excerpt_or_full_post` is `full`).
async fn content_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let title = the_title_for(ctx.site, post);
    let heading = if title.is_empty() {
        String::new()
    } else if ctx.q.flags.is_singular() {
        format!("<h1 class=\"entry-title default-max-width\">{title}</h1>")
    } else {
        format!("<h2 class=\"entry-title default-max-width\"><a href=\"{}\">{title}</a></h2>", esc_url(&ctx.site.permalink(post).await?))
    };
    let classes = classes_of(ctx, post).await?;
    let thumbnail = post_thumbnail(ctx, post).await?;
    let content = content_and_pages(ctx, post).await?;
    let meta = entry_meta_footer(ctx, post).await?;
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\t<header class=\"entry-header\">\n\t\t\t\t\t{heading}\t\t\n\t\t{thumbnail}\t</header><!-- .entry-header -->\n\n\t<div class=\"entry-content\">\n\t\t{content}\t</div><!-- .entry-content -->\n\n\t<footer class=\"entry-footer default-max-width\">\n\t\t{meta}\t</footer><!-- .entry-footer -->\n</article><!-- #post-{id} -->\n",
        id = post.id,
    ))
}

/// `twenty_twenty_one_print_first_instance_of_block()`: the first `$instances` top-level blocks of a name (a name ending
/// in `*` matches by prefix), rendered and passed through `the_content`. `None` when there is none.
async fn first_instance_of_block(ctx: &Ctx<'_>, post: &Arc<Post>, content: &str, name: &str, instances: usize) -> Result<Option<String>> {
    let blocks = rustpress_blocks::parser::parse_blocks(content);
    let mut picked = String::new();
    let mut count = 0;
    for block in &blocks {
        let Some(block_name) = block.name.as_deref() else { continue };
        let matching = match name.strip_suffix('*') {
            Some(prefix) => block_name.starts_with(prefix),
            None => block_name == name,
        };
        if matching {
            count += 1;
            picked.push_str(&rustpress_blocks::serializer::serialize_block(block));
            if count >= instances {
                break;
            }
        }
    }
    if picked.is_empty() {
        return Ok(None);
    }
    Ok(Some(the_content(&ctx.env, post, &picked).await?))
}

/// `has_block( $name, $content )`.
fn has_block(content: &str, name: &str) -> bool {
    let short = name.strip_prefix("core/").unwrap_or(name);
    content.contains(&format!("<!-- wp:{short} ")) || content.contains(&format!("<!-- wp:{short}-->")) || content.contains(&format!("<!-- wp:{short} -->")) || content.contains(&format!("<!-- wp:{name} "))
}

/// `the_excerpt()`.
async fn the_excerpt(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let excerpt = rustpress_blocks::excerpt::get_the_excerpt(&ctx.env, post).await?;
    Ok(rustpress_core::formatting::wpautop(&rustpress_core::formatting::convert_chars(&ctx.site.texturize(&excerpt))))
}

/// `template-parts/excerpt/excerpt-*.php`.
async fn excerpt_body(ctx: &Ctx<'_>, post: &Arc<Post>, format: Option<&str>) -> Result<String> {
    // `get_the_content()` of the post in the loop.
    let content = get_the_content(&ctx.env, post, current_page(ctx), ctx.q.flags.is_singular()).await?;
    let full = || async { the_content(&ctx.env, post, &content).await };
    Ok(match format {
        Some("aside" | "status") => full().await?,
        Some(kind @ ("audio" | "video")) => {
            let core = if kind == "audio" { "core/audio" } else { "core/video" };
            let first = if has_block(&content, core) {
                first_instance_of_block(ctx, post, &content, core, 1).await?
            } else if has_block(&content, "core/embed") {
                first_instance_of_block(ctx, post, &content, "core/embed", 1).await?
            } else {
                first_instance_of_block(ctx, post, &content, "core-embed/*", 1).await?
            };
            format!("{}{}", first.unwrap_or_default(), the_excerpt(ctx, post).await?)
        }
        Some("chat") => {
            if has_block(&content, "core/paragraph") {
                first_instance_of_block(ctx, post, &content, "core/paragraph", 2).await?.unwrap_or_default()
            } else {
                the_excerpt(ctx, post).await?
            }
        }
        Some("gallery") => {
            let first = if has_block(&content, "core/gallery") { first_instance_of_block(ctx, post, &content, "core/gallery", 1).await? } else { None };
            format!("{}{}", first.unwrap_or_default(), the_excerpt(ctx, post).await?)
        }
        Some("image") => {
            let first = if !can_show_thumbnail(ctx, post).await? && has_block(&content, "core/image") { first_instance_of_block(ctx, post, &content, "core/image", 1).await? } else { None };
            format!("{}{}", first.unwrap_or_default(), the_excerpt(ctx, post).await?)
        }
        Some("link") => {
            if has_block(&content, "core/paragraph") {
                first_instance_of_block(ctx, post, &content, "core/paragraph", 1).await?.unwrap_or_default()
            } else {
                full().await?
            }
        }
        Some("quote") => {
            let first = if has_block(&content, "core/quote") {
                first_instance_of_block(ctx, post, &content, "core/quote", 1).await?
            } else if has_block(&content, "core/pullquote") {
                first_instance_of_block(ctx, post, &content, "core/pullquote", 1).await?
            } else {
                None
            };
            match first {
                Some(html) => html,
                None if has_block(&content, "core/quote") || has_block(&content, "core/pullquote") => String::new(),
                None => the_excerpt(ctx, post).await?,
            }
        }
        _ => the_excerpt(ctx, post).await?,
    })
}

/// `template-parts/content/content-excerpt.php` with `template-parts/header/excerpt-header.php`.
async fn excerpt_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let format = post_format(ctx, post).await?;
    let format = format.as_deref();
    let classes = classes_of(ctx, post).await?;
    let header = if matches!(format, Some("aside" | "status")) {
        String::new()
    } else {
        let title = the_title_for(ctx.site, post);
        let heading = if title.is_empty() { String::new() } else { format!("<h2 class=\"entry-title default-max-width\"><a href=\"{}\">{title}</a></h2>", esc_url(&ctx.site.permalink(post).await?)) };
        format!("\n<header class=\"entry-header\">\n\t{heading}{}</header><!-- .entry-header -->\n", post_thumbnail(ctx, post).await?)
    };
    let body = excerpt_body(ctx, post, format).await?;
    let meta = entry_meta_footer(ctx, post).await?;
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\n\t{header}\n\t<div class=\"entry-content\">\n\t\t{body}\t</div><!-- .entry-content -->\n\n\t<footer class=\"entry-footer default-max-width\">\n\t\t{meta}\t</footer><!-- .entry-footer -->\n</article><!-- #post-${{ID}} -->\n",
        id = post.id,
    ))
}

/// `template-parts/post/author-bio.php`.
async fn author_bio(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let description = author_description(ctx, post).await?;
    if description.is_empty() || !supports_author(ctx, post) {
        return Ok(String::new());
    }
    let user = ctx.site.db.user(post.post_author).await?;
    let (name, email, url) = match &user {
        Some(user) => (user.display_name.clone(), user.user_email.clone(), ctx.site.author_url(user)),
        None => (String::new(), String::new(), String::new()),
    };
    let avatars = ctx.site.options.get("show_avatars").is_some_and(|v| !v.is_empty() && v != "0");
    let avatar = rustpress_blocks::content::get_avatar(&ctx.env, &email, 85, "", "", "");
    Ok(format!(
        "\t<div class=\"author-bio {}\">\n\t\t{avatar}\t\t<div class=\"author-bio-content\">\n\t\t\t<h2 class=\"author-title\">\n\t\t\t{}\t\t\t</h2><!-- .author-title -->\n\t\t\t<p class=\"author-description\"> {description}</p><!-- .author-description -->\n\t\t\t<a class=\"author-link\" href=\"{}\" rel=\"author\">{}</a>\t\t</div><!-- .author-bio-content -->\n\t</div><!-- .author-bio -->\n\t",
        if avatars { "show-avatars" } else { "" },
        sprintf(&th(ctx, "By %s"), &[&name]),
        esc_url(&url),
        sprintf(&th(ctx, "View all of %s's posts."), &[&name]),
    ))
}

/// `template-parts/content/content-single.php`.
async fn single_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let classes = classes_of(ctx, post).await?;
    let title = the_title_for(ctx.site, post);
    let heading = if title.is_empty() { String::new() } else { format!("<h1 class=\"entry-title\">{title}</h1>") };
    let thumbnail = post_thumbnail(ctx, post).await?;
    let content = content_and_pages(ctx, post).await?;
    let meta = entry_meta_footer(ctx, post).await?;
    let bio = if post.post_type == "attachment" { String::new() } else { format!("\t\t{}\t", author_bio(ctx, post).await?) };
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\n\t<header class=\"entry-header alignwide\">\n\t\t{heading}\t\t{thumbnail}\t</header><!-- .entry-header -->\n\n\t<div class=\"entry-content\">\n\t\t{content}\t</div><!-- .entry-content -->\n\n\t<footer class=\"entry-footer default-max-width\">\n\t\t{meta}\t</footer><!-- .entry-footer -->\n\n\t{bio}\n</article><!-- #post-{id} -->\n",
        id = post.id,
    ))
}

/// `template-parts/content/content-page.php`.
async fn page_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let classes = classes_of(ctx, post).await?;
    let mut header = String::new();
    if !ctx.q.flags.is_front_page {
        let title = the_title_for(ctx.site, post);
        let heading = if title.is_empty() { String::new() } else { format!("<h1 class=\"entry-title\">{title}</h1>") };
        header = format!("\t\t<header class=\"entry-header alignwide\">\n\t\t\t{heading}\t\t\t{}\t\t</header><!-- .entry-header -->\n\t", post_thumbnail(ctx, post).await?);
    } else if images::post_thumbnail_id(&ctx.env, post.id).await? != 0 {
        header = format!("\t\t<header class=\"entry-header alignwide\">\n\t\t\t{}\t\t</header><!-- .entry-header -->\n\t", post_thumbnail(ctx, post).await?);
    }
    let content = content_and_pages(ctx, post).await?;
    let edit = if edit_post_url(ctx, post).is_some() {
        format!("\t\t<footer class=\"entry-footer default-max-width\">\n\t\t\t{}\t\t</footer><!-- .entry-footer -->\n\t", edit_link(ctx, post, "<span class=\"edit-link\">", "</span>"))
    } else {
        String::new()
    };
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\n\t{header}\n\t<div class=\"entry-content\">\n\t\t{content}\t</div><!-- .entry-content -->\n\n\t{edit}</article><!-- #post-{id} -->\n",
        id = post.id,
    ))
}

/// `image.php`: an image attachment.
async fn image_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let site = ctx.site;
    let classes = classes_of(ctx, post).await?;
    let title = the_title_for(site, post);
    let heading = if title.is_empty() { String::new() } else { format!("<h1 class=\"entry-title\">{title}</h1>") };
    let image = images::get_attachment_image(&ctx.env, post.id, "full", &ImageArgs::default()).await?;
    let caption = if post.post_excerpt.is_empty() {
        String::new()
    } else {
        format!("\t\t\t\t\t<figcaption class=\"wp-caption-text\">{}</figcaption>\n\t\t\t\t", rustpress_core::kses::kses_post(&post.post_excerpt))
    };
    let content = content_and_pages(ctx, post).await?;
    let mut footer = String::new();
    let parent = if post.post_parent != 0 { site.db.post(post.post_parent).await? } else { None };
    match &parent {
        Some(parent) => {
            let link = format!("<a href=\"{}\">{}</a>", esc_url(&site.permalink(parent).await?), esc_html(&the_title_for(site, parent)));
            footer.push_str(&format!("<span class=\"posted-on\">{}</span>", sprintf(&th(ctx, "Published in %s"), &[&link])));
        }
        None => footer.push_str(&edit_link(ctx, post, "<span class=\"edit-link\">", "</span>")),
    }
    let (meta, attached) = images::load_metadata(site, post).await?;
    if let Some(meta) = meta {
        footer.push_str(&format!(
            "<span class=\"full-size-link\"><span class=\"screen-reader-text\">{}</span><a href=\"{}\">{} &times; {}</a></span>",
            esc_html(&tx(ctx, "Full size", "Used before full size attachment link.")),
            esc_url(&attached.map(|file| images::attachment_url(site, &file)).unwrap_or_default()),
            meta.width(),
            meta.height()
        ));
    }
    if parent.is_some() {
        footer.push_str(&edit_link(ctx, post, "<span class=\"edit-link\">", "</span><br>"));
    }
    Ok(format!(
        "\t<article id=\"post-{id}\" class=\"{classes}\">\n\t\t<header class=\"entry-header alignwide\">\n\t\t\t{heading}\t\t</header><!-- .entry-header -->\n\n\t\t<div class=\"entry-content\">\n\t\t\t<figure class=\"wp-block-image\">\n\t\t\t\t{image}\n\t\t\t\t{caption}\t\t\t</figure><!-- .wp-block-image -->\n\n\t\t\t{content}\t\t</div><!-- .entry-content -->\n\n\t\t<footer class=\"entry-footer default-max-width\">\n\t\t\t{footer}\t\t</footer><!-- .entry-footer -->\n\t</article><!-- #post-{id} -->\n\t",
        id = post.id,
    ))
}

/// `get_search_form()` with the theme's `searchform.php`.
fn search_form(ctx: &Ctx<'_>) -> String {
    search_form_markup(ctx.site, ctx.q, &ctx.env, "")
}

/// `searchform.php`; `aria_label` is the argument of `get_search_form()`.
fn search_form_markup(site: &Site, q: &rustpress_core::QueryResult, env: &rustpress_blocks::Env<'_>, aria_label: &str) -> String {
    let unique = env.with_state(|s| s.unique_id("search-form-"));
    let aria = if aria_label.is_empty() { String::new() } else { format!("aria-label=\"{}\"", esc_attr(aria_label)) };
    format!(
        "<form role=\"search\" {aria} method=\"get\" class=\"search-form\" action=\"{}\">\n\t<label for=\"{}\">{}</label>\n\t<input type=\"search\" id=\"{}\" class=\"search-field\" value=\"{}\" name=\"s\" />\n\t<input type=\"submit\" class=\"search-submit\" value=\"{}\" />\n</form>\n",
        esc_url(&site.home_url("/")),
        esc_attr(&unique),
        site.i18n().gettext(THEME, "Search&hellip;"),
        esc_attr(&unique),
        esc_attr(q.vars.get("s").unwrap_or("")),
        esc_attr(&site.i18n().pgettext(THEME, "submit button", "Search")),
    )
}

/// `get_search_form()` for a widget rendered outside of a page of the theme.
pub fn widget_search_form(site: &Site, q: &rustpress_core::QueryResult, env: &rustpress_blocks::Env<'_>) -> String {
    search_form_markup(site, q, env, "")
}

/// `<span class="page-description search-term">` with the search query.
fn search_term(ctx: &Ctx<'_>) -> String {
    format!("<span class=\"page-description search-term\">{}</span>", esc_html(ctx.q.vars.get("s").unwrap_or("")))
}

/// `template-parts/content/content-none.php`.
fn content_none(ctx: &Ctx<'_>) -> String {
    let f = &ctx.q.flags;
    let title = if f.is_search {
        format!("\n\t\t\t<h1 class=\"page-title\">\n\t\t\t\t{}\t\t\t</h1>\n\n\t\t", sprintf(&th(ctx, "Results for \"%s\""), &[&search_term(ctx)]))
    } else {
        format!("\n\t\t\t<h1 class=\"page-title\">{}</h1>\n\n\t\t", th(ctx, "Nothing here"))
    };
    let can_publish = ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.can("publish_posts"));
    let body = if f.is_home && can_publish {
        let text = t(ctx, "Ready to publish your first post? <a href=\"%s\">Get started here</a>.");
        format!("\n\t\t\t<p>{}</p>\n\t\t", sprintf(&text, &[&esc_url(&ctx.site.site_url("wp-admin/post-new.php"))]))
    } else if f.is_search {
        format!("\n\t\t\t<p>{}</p>\n\t\t\t{}\n\t\t", th(ctx, "Sorry, but nothing matched your search terms. Please try again with some different keywords."), search_form(ctx))
    } else {
        format!("\n\t\t\t<p>{}</p>\n\t\t\t{}\n\t\t", th(ctx, "It seems we can&rsquo;t find what you&rsquo;re looking for. Perhaps searching can help."), search_form(ctx))
    };
    format!("\n<section class=\"no-results not-found\">\n\t<header class=\"page-header alignwide\">\n\t\t{title}\t</header><!-- .page-header -->\n\n\t<div class=\"page-content default-max-width\">\n\n\t\t{body}\t</div><!-- .page-content -->\n</section><!-- .no-results -->\n")
}

/// `twenty_twenty_one_the_posts_navigation()`.
fn posts_navigation(ctx: &Ctx<'_>) -> String {
    let order = ctx.q.vars.get("order").map(str::to_uppercase).filter(|o| o == "ASC").unwrap_or_else(|| "DESC".to_owned());
    let newer = t(ctx, "Newer <span class=\"nav-short\">posts</span>");
    let older = t(ctx, "Older <span class=\"nav-short\">posts</span>");
    let (prev, next) = if order == "DESC" { (newer, older) } else { (older, newer) };
    let rtl = ctx.site.is_rtl();
    tags::posts_pagination(
        ctx.site,
        ctx.q,
        &Pagination {
            prev_text: format!("{} <span class=\"nav-prev-text\">{prev}</span>", ui(if rtl { "arrow_right" } else { "arrow_left" })),
            next_text: format!("<span class=\"nav-next-text\">{next}</span> {}", ui(if rtl { "arrow_left" } else { "arrow_right" })),
            before_page_number: format!("{} ", th(ctx, "Page")),
            mid_size: 0,
        },
    )
}

/// `comments.php`.
async fn comments_template(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    if rustpress_blocks::content::post_password_required(post) {
        return Ok(String::new());
    }
    let site = ctx.site;
    let env = &ctx.env;
    let page = rustpress_blocks::comment_pages::comments_for_template(env, post.id).await?;
    let avatars = site.options.get("show_avatars").is_some_and(|v| !v.is_empty() && v != "0");
    let mut out = format!("\n<div id=\"comments\" class=\"comments-area default-max-width {}\">\n\n\t", if avatars { "show-avatars" } else { "" });
    let request_uri = ctx.request_uri();
    if !page.comments.is_empty() {
        let count = post.comment_count;
        let title = if count == 1 {
            th(ctx, "1 comment")
        } else {
            let template = site.i18n().npgettext(THEME, "Comments title", "%s comment", "%s comments", count);
            sprintf(&esc_html(&template), &[&esc_html(&site.number_i18n(count.max(0) as u64))])
        };
        out.push_str(&format!("\t\t<h2 class=\"comments-title\">\n\t\t\t\t\t\t\t{title}\t\t\t\t\t</h2><!-- .comments-title -->\n\n\t\t<ol class=\"comment-list\">\n\t\t\t"));
        let args = comments::ListArgs { avatar_size: 60, reply_text: site.trx("Reply", "verb"), short_ping: true, cpage: Some(page.page) };
        out.push_str(&rustpress_blocks::legacy_comments::list_comments(site, env, post, &page.comments, &args).await?);
        let rtl = site.is_rtl();
        let prev = format!("{} <span class=\"nav-prev-text\">{}</span>", ui(if rtl { "arrow_right" } else { "arrow_left" }), th(ctx, "Older comments"));
        let next = format!("<span class=\"nav-next-text\">{}</span> {}", th(ctx, "Newer comments"), ui(if rtl { "arrow_left" } else { "arrow_right" }));
        let pagination = tags::comments_pagination_with(site, &site.permalink(post).await?, &request_uri, page.page, page.max_pages, &prev, &next, 0, &format!("{} ", th(ctx, "Page")));
        out.push_str(&format!("\t\t</ol><!-- .comment-list -->\n\n\t\t{pagination}\n\t\t"));
        if !comments::comments_open(post) {
            out.push_str(&format!("\t\t\t<p class=\"no-comments\">{}</p>\n\t\t", th(ctx, "Comments are closed.")));
        }
        out.push('\t');
    }
    let form = rustpress_blocks::legacy_comments::FormArgs { title_reply: th(ctx, "Leave a comment"), title_reply_before: "<h2 id=\"reply-title\" class=\"comment-reply-title\">".into(), title_reply_after: "</h2>".into(), rows: 5 };
    out.push_str(&format!("\n\t{}\n</div><!-- #comments -->\n", rustpress_blocks::legacy_comments::comment_form_with(site, ctx.q.viewer.as_ref(), post, &request_uri, false, &form).await?));
    Ok(out)
}

/// `the_post_navigation()`; `prev_text` and `next_text` contain `%title`.
async fn post_navigation(ctx: &Ctx<'_>, post: &Post, prev_text: &str, next_text: Option<&str>) -> Result<String> {
    let site = ctx.site;
    let mut links = String::new();
    for previous in [true, false] {
        let text = if previous { prev_text } else { next_text.unwrap_or("%title") };
        let other = if post.post_type == "attachment" {
            // An attachment's previous post is its parent.
            if previous && post.post_parent != 0 { site.db.post(post.post_parent).await? } else { None }
        } else {
            match site.db.adjacent_post_id(&post.post_type, &post.post_date, post.id, previous).await? {
                Some(id) => site.db.post(id).await?,
                None => None,
            }
        };
        let Some(other) = other else { continue };
        if post.post_type != "attachment" && other.post_status != "publish" {
            continue;
        }
        let mut title = other.post_title.clone();
        if title.is_empty() {
            title = if previous { site.tr("Previous Post") } else { site.tr("Next Post") };
        }
        let title = super::menu::the_title_filters(site, &title);
        let inner = text.replace("%title", &title);
        let anchor = format!("<a href=\"{}\" rel=\"{}\">{inner}</a>", esc_url(&site.permalink(&other).await?), if previous { "prev" } else { "next" });
        links.push_str(&format!("<div class=\"nav-{}\">{anchor}</div>", if previous { "previous" } else { "next" }));
    }
    if links.is_empty() {
        return Ok(String::new());
    }
    Ok(tags::navigation_markup(&links, "post-navigation", &site.tr("Post navigation"), &site.tr("Posts")))
}

/// `single_post_title( '', false )` of the posts page.
fn posts_page_title(ctx: &Ctx<'_>) -> String {
    match &ctx.q.queried {
        Queried::Post(page) if !page.post_title.is_empty() => the_title_for(ctx.site, page),
        _ => String::new(),
    }
}

/// The archive title (`get_the_archive_title()`) and description.
async fn archive_header(ctx: &Ctx<'_>) -> Result<(String, String)> {
    let q = ctx.q;
    let site = ctx.site;
    let f = &q.flags;
    Ok(match &q.queried {
        Queried::Term(term) if f.is_tax && term.taxonomy == "post_format" => {
            let title = match term.slug.as_str() {
                "post-format-aside" => site.trx("Asides", "post format archive title"),
                "post-format-gallery" => site.trx("Galleries", "post format archive title"),
                "post-format-image" => site.trx("Images", "post format archive title"),
                "post-format-video" => site.trx("Videos", "post format archive title"),
                "post-format-quote" => site.trx("Quotes", "post format archive title"),
                "post-format-link" => site.trx("Links", "post format archive title"),
                "post-format-status" => site.trx("Statuses", "post format archive title"),
                "post-format-audio" => site.trx("Audio", "post format archive title"),
                "post-format-chat" => site.trx("Chats", "post format archive title"),
                _ => term.name.clone(),
            };
            (title, term.description.clone())
        }
        Queried::Term(term) if f.is_category || f.is_tag || f.is_tax => {
            let prefix = if f.is_category {
                site.trx("Category:", "category archive title prefix")
            } else if f.is_tag {
                site.trx("Tag:", "tag archive title prefix")
            } else {
                rustpress_core::i18n::sprintf(&site.trx("%s:", "taxonomy term archive title prefix"), &[&site.taxonomy_singular_label(&term.taxonomy)])
            };
            (format!("{prefix} <span>{}</span>", term.name), term.description.clone())
        }
        Queried::User(user) if f.is_author => {
            let description = site.db.user_meta(user.id, "description").await?.unwrap_or_default();
            (format!("{} <span>{}</span>", site.trx("Author:", "author archive title prefix"), user.display_name), description)
        }
        Queried::PostType(post_type) if f.is_post_type_archive && !(f.is_year || f.is_month || f.is_day) => {
            (format!("{} <span>{}</span>", site.trx("Archives:", "post type archive title prefix"), site.post_type_label(post_type)), String::new())
        }
        _ if f.is_year || f.is_month || f.is_day => {
            let first = q.posts.first();
            let ts = first.and_then(|p| site.timezone.local_timestamp(&p.post_date)).unwrap_or(0);
            let hidden = match first {
                Some(post) if site.plugins.resets_query_in_head() => site.plugins.hides_date(site, post).await?,
                _ => false,
            };
            let (prefix, format) = if f.is_day {
                (site.trx("Day:", "date archive title prefix"), site.trx("F j, Y", "daily archives date format"))
            } else if f.is_month {
                (site.trx("Month:", "date archive title prefix"), site.trx("F Y", "monthly archives date format"))
            } else {
                (site.trx("Year:", "date archive title prefix"), site.trx("Y", "yearly archives date format"))
            };
            (format!("{prefix} <span>{}</span>", if hidden { String::new() } else { site.date(&format, ts) }), String::new())
        }
        _ => (site.tr("Archives"), String::new()),
    })
}

/// The body of the main template, between `get_header()` and `get_footer()`.
async fn main_template(ctx: &Ctx<'_>) -> Result<String> {
    let q = ctx.q;
    let f = &q.flags;
    let site = ctx.site;
    if f.is_404 {
        // `404.php`.
        return Ok(format!(
            "\n\t<header class=\"page-header alignwide\">\n\t\t<h1 class=\"page-title\">{}</h1>\n\t</header><!-- .page-header -->\n\n\t<div class=\"error-404 not-found default-max-width\">\n\t\t<div class=\"page-content\">\n\t\t\t<p>{}</p>\n\t\t\t{}\t\t</div><!-- .page-content -->\n\t</div><!-- .error-404 -->\n\n",
            th(ctx, "Nothing here"),
            th(ctx, "It looks like nothing was found at this location. Maybe try a search?"),
            search_form(ctx)
        ));
    }
    ctx.env.with_state(|s| s.media.in_main_loop = true);
    let mut out = String::new();
    if f.is_singular() {
        for post in &q.posts {
            let post = Arc::new(post.clone());
            ctx.env.with_state(|s| s.looped_posts.push(post.id));
            if f.is_attachment && post.post_mime_type.starts_with("image/") {
                out.push_str(&image_part(ctx, &post).await?);
                if comments::comments_open(&post) || post.comment_count > 0 {
                    out.push_str(&comments_template(ctx, &post).await?);
                }
                continue;
            }
            if f.is_page {
                out.push_str(&page_part(ctx, &post).await?);
                if comments::comments_open(&post) || post.comment_count > 0 {
                    out.push_str(&comments_template(ctx, &post).await?);
                }
                continue;
            }
            out.push_str(&single_part(ctx, &post).await?);
            if f.is_attachment {
                let text = sprintf(&t(ctx, "<span class=\"meta-nav\">Published in</span><span class=\"post-title\">%s</span>"), &["%title"]);
                out.push_str(&post_navigation(ctx, &post, &text, None).await?);
            }
            if comments::comments_open(&post) || post.comment_count > 0 {
                out.push_str(&comments_template(ctx, &post).await?);
            }
            let rtl = site.is_rtl();
            let next_icon = ui(if rtl { "arrow_left" } else { "arrow_right" });
            let prev_icon = ui(if rtl { "arrow_right" } else { "arrow_left" });
            let next = format!("<p class=\"meta-nav\">{}{next_icon}</p><p class=\"post-title\">%title</p>", th(ctx, "Next post"));
            let prev = format!("<p class=\"meta-nav\">{prev_icon}{}</p><p class=\"post-title\">%title</p>", th(ctx, "Previous post"));
            out.push_str(&post_navigation(ctx, &post, &prev, Some(&next)).await?);
        }
        ctx.env.with_state(|s| s.media.in_main_loop = false);
        return Ok(out);
    }
    let full = ctx.mod_str("display_excerpt_or_full_post").as_deref() == Some("full");
    let mut parts = Vec::new();
    for post in &q.posts {
        let post = Arc::new(post.clone());
        ctx.env.with_state(|s| s.looped_posts.push(post.id));
        parts.push(if f.is_search || !full { excerpt_part(ctx, &post).await? } else { content_part(ctx, &post).await? });
    }
    let loop_html = parts.concat();
    ctx.env.with_state(|s| s.media.in_main_loop = false);
    if f.is_search {
        // `search.php`.
        if q.posts.is_empty() {
            return Ok(content_none(ctx));
        }
        let found = q.found_posts;
        let count = sprintf(&esc_html(&site.i18n().ngettext(THEME, "We found %d result for your search.", "We found %d results for your search.", found as i64)), &[&found.to_string()]);
        return Ok(format!(
            "\t<header class=\"page-header alignwide\">\n\t\t<h1 class=\"page-title\">\n\t\t\t{}\t\t</h1>\n\t</header><!-- .page-header -->\n\n\t<div class=\"search-result-count default-max-width\">\n\t\t{count}\t</div><!-- .search-result-count -->\n\t{loop_html}{}",
            sprintf(&th(ctx, "Results for \"%s\""), &[&search_term(ctx)]),
            posts_navigation(ctx)
        ));
    }
    if f.is_archive {
        // `archive.php`.
        if q.posts.is_empty() {
            return Ok(format!("\n\t{}\n", content_none(ctx)));
        }
        let (title, description) = archive_header(ctx).await?;
        let description = if description.is_empty() { String::new() } else { format!("\t\t\t<div class=\"archive-description\">{}</div>\n\t\t", rustpress_core::kses::kses_post(&rustpress_core::formatting::wpautop(&description))) };
        return Ok(format!("\n\n\t<header class=\"page-header alignwide\">\n\t\t<h1 class=\"page-title\">{title}</h1>\t\t{description}\t</header><!-- .page-header -->\n\n\t{}\n\t{}\n\n", parts.iter().map(|part| format!("\t\t\t\t{part}\t")).collect::<String>(), posts_navigation(ctx)));
    }
    // `index.php`.
    let mut out = String::from("\n");
    if f.is_home && !f.is_front_page {
        let title = posts_page_title(ctx);
        if !title.is_empty() {
            out.push_str(&format!("\t<header class=\"page-header alignwide\">\n\t\t<h1 class=\"page-title\">{title}</h1>\n\t</header><!-- .page-header -->\n"));
        }
    }
    out.push('\n');
    if q.posts.is_empty() {
        out.push_str(&content_none(ctx));
    } else {
        out.push_str(&loop_html);
        out.push_str(&posts_navigation(ctx));
    }
    Ok(out)
}

// ------------------------------------------------------------------------ head

/// `twenty_twenty_one_get_non_latin_css( 'front-end' )`.
fn non_latin_css(site: &Site) -> String {
    let fonts: &[&str] = match site.language_tag().as_str() {
        "ar" | "ary" | "azb" | "ckb" | "fa-IR" | "haz" | "ps" => &["Tahoma", "Arial", "sans-serif"],
        "zh-CN" => &["'PingFang SC'", "'Helvetica Neue'", "'Microsoft YaHei New'", "'STHeiti Light'", "sans-serif"],
        "zh-TW" => &["'PingFang TC'", "'Helvetica Neue'", "'Microsoft YaHei New'", "'STHeiti Light'", "sans-serif"],
        "zh-HK" => &["'PingFang HK'", "'Helvetica Neue'", "'Microsoft YaHei New'", "'STHeiti Light'", "sans-serif"],
        "bel" | "bg-BG" | "kk" | "mk-MK" | "mn" | "ru-RU" | "sah" | "sr-RS" | "tt-RU" | "uk" => &["'Helvetica Neue'", "Helvetica", "'Segoe UI'", "Arial", "sans-serif"],
        "bn-BD" | "hi-IN" | "mr" | "ne-NP" | "gu" => &["Arial", "sans-serif"],
        "el" => &["'Helvetica Neue', Helvetica, Arial, sans-serif"],
        "he-IL" => &["'Arial Hebrew'", "Arial", "sans-serif"],
        "ja" => &["sans-serif"],
        "ko-KR" => &["'Apple SD Gothic Neo'", "'Malgun Gothic'", "'Nanum Gothic'", "Dotum", "sans-serif"],
        "th" => &["'Sukhumvit Set'", "'Helvetica Neue'", "Helvetica", "Arial", "sans-serif"],
        "vi" => &["'Libre Franklin'", "sans-serif"],
        _ => return String::new(),
    };
    const ELEMENTS: &[&str] = &[
        "body", "input", "textarea", "button", ".button", ".faux-button", ".wp-block-button__link", ".wp-block-file__button", ".has-drop-cap:not(:focus)::first-letter", ".entry-content .wp-block-archives", ".entry-content .wp-block-categories",
        ".entry-content .wp-block-cover-image", ".entry-content .wp-block-latest-comments", ".entry-content .wp-block-latest-posts", ".entry-content .wp-block-pullquote", ".entry-content .wp-block-quote.is-large", ".entry-content .wp-block-quote.is-style-large",
        ".entry-content .wp-block-archives *", ".entry-content .wp-block-categories *", ".entry-content .wp-block-latest-posts *", ".entry-content .wp-block-latest-comments *", ".entry-content p", ".entry-content ol", ".entry-content ul", ".entry-content dl",
        ".entry-content dt", ".entry-content cite", ".entry-content figcaption", ".entry-content .wp-caption-text", ".comment-content p", ".comment-content ol", ".comment-content ul", ".comment-content dl", ".comment-content dt", ".comment-content cite",
        ".comment-content figcaption", ".comment-content .wp-caption-text", ".widget_text p", ".widget_text ol", ".widget_text ul", ".widget_text dl", ".widget_text dt", ".widget-content .rssSummary", ".widget-content cite", ".widget-content figcaption",
        ".widget-content .wp-caption-text",
    ];
    // `twenty_twenty_one_generate_css()`.
    format!("{} {{ font-family: {}; }}", ELEMENTS.join(","), fonts.join(","))
}

/// `Twenty_Twenty_One_Custom_Colors::generate_custom_color_variables()` for a background that is not the default.
fn custom_color_variables(background: &str) -> String {
    let readable = readable_color(background);
    let mut css = format!(
        ":root{{--global--color-background: #{background};--global--color-primary: {readable};--global--color-secondary: {readable};--button--color-background: {readable};--button--color-text-hover: {readable};"
    );
    if readable == "#fff" {
        css.push_str("--table--stripes-border-color: rgba(240, 240, 240, 0.15);--table--stripes-background-color: rgba(240, 240, 240, 0.15);");
    }
    css.push('}');
    css
}

/// `_custom_background_cb()` with the theme's default color.
fn custom_background_style(ctx: &Ctx<'_>) -> String {
    let image = ctx.mod_str("background_image").unwrap_or_default();
    let mut color = custom_background_color(&ctx.mods);
    if color == BACKGROUND.to_lowercase() {
        color.clear();
    }
    if image.is_empty() && color.is_empty() {
        return if preview(ctx).is_some() { "<style id=\"custom-background-css\"></style>".to_owned() } else { String::new() };
    }
    // `maybe_hash_hex_color()`.
    let hex = (color.len() == 3 || color.len() == 6) && color.chars().all(|c| c.is_ascii_hexdigit());
    let mut style = if color.is_empty() { String::new() } else if hex { format!("background-color: #{color};") } else { format!("background-color: {color};") };
    if !image.is_empty() {
        let url = format!(" background-image: url(\"{}\");", rustpress_core::formatting::esc_url_raw(&image));
        style.push_str(&url);
        let position_x = ctx.mod_str("background_position_x").unwrap_or_else(|| "left".to_owned());
        let position_y = ctx.mod_str("background_position_y").unwrap_or_else(|| "top".to_owned());
        let position_x = if matches!(position_x.as_str(), "center" | "right" | "left") { position_x } else { "left".to_owned() };
        let position_y = if matches!(position_y.as_str(), "center" | "top" | "bottom") { position_y } else { "top".to_owned() };
        style.push_str(&format!(" background-position: {position_x} {position_y};"));
        let size = ctx.mod_str("background_size").unwrap_or_else(|| "auto".to_owned());
        let size = if matches!(size.as_str(), "auto" | "contain" | "cover") { size } else { "auto".to_owned() };
        style.push_str(&format!(" background-size: {size};"));
        let repeat = ctx.mod_str("background_repeat").unwrap_or_else(|| "repeat".to_owned());
        let repeat = if matches!(repeat.as_str(), "repeat-x" | "repeat-y" | "repeat" | "no-repeat") { repeat } else { "repeat".to_owned() };
        style.push_str(&format!(" background-repeat: {repeat};"));
        let attachment = ctx.mod_str("background_attachment").unwrap_or_else(|| "scroll".to_owned());
        let attachment = if attachment == "fixed" { attachment } else { "scroll".to_owned() };
        style.push_str(&format!(" background-attachment: {attachment};"));
    }
    format!("<style id=\"custom-background-css\">\nbody.custom-background {{ {} }}\n</style>\n", style.trim())
}

/// `body.custom-background` of `get_body_class()`: a background color other than the theme's default, or an image.
pub fn has_custom_background(mods: &Value) -> bool {
    let color = custom_background_color(mods);
    let image = mods.get("background_image").is_some_and(php_truthy);
    color != BACKGROUND.to_lowercase() || image
}

/// jQuery's versions as `script-loader.php` registers them.
fn script_version(wp_root: &std::path::Path, handle: &str, default: &str) -> String {
    let text = read(&wp_root.join("wp-includes").join("script-loader.php")).unwrap_or_default();
    let marker = format!("'{handle}'");
    for line in text.lines() {
        if line.trim_start().starts_with(&format!("$scripts->add( {marker},"))
            && let Some(rest) = line.rsplit("array(),").next()
            && let Some(v) = rest.split('\'').nth(1)
        {
            return v.to_owned();
        }
    }
    default.to_owned()
}

/// `wp_head()`.
async fn head(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State, registry: &Registry) -> Result<String> {
    let site = ctx.site;
    let theme = ctx.theme;
    let q = ctx.q;
    let wp_root = &theme.wp_root;
    let version = wp_version(wp_root);
    let includes_url = site.site_url("wp-includes");
    let theme_uri = theme.template_directory_uri();
    let mut out = String::new();

    let crate::head::PluginHead { head: plugin_head, blocks: early_blocks } = crate::head::plugin_head(theme, site, q, registry, true, None).await?;
    // The plugins' actions of priority 0 come first.
    out.push_str(&plugin_head.early);
    out.push_str(&plugin_head.first);
    if !plugin_head.removed.contains(&"_wp_render_title_tag") {
        let title = match plugin_head.title {
            Some(title) => title,
            None => document_title(site, q).await?,
        };
        out.push_str(&format!("<title>{}</title>\n", esc_html(&title)));
    }
    out.push_str(&rustpress_core::native::robots_tag(plugin_head.robots.as_deref().unwrap_or_default()));
    out.push_str(&plugin_head.html);
    out.push_str(&crate::head::feed_links(site, q).await?);
    out.push_str(&oembed_links(site, q).await?);

    // --- styles in the order WordPress prints them
    struct Sheet {
        id: String,
        path: std::path::PathBuf,
        url: String,
        source: String,
        rtl: bool,
    }
    let mut sheets: Vec<Sheet> = Vec::new();
    let rtl = site.is_rtl();
    let common = wp_root.join("wp-includes/css/dist/block-library/common.min.css");
    let common_present = rustpress_core::files::is_file(&common);
    if common_present {
        sheets.push(Sheet { id: "wp-block-library".into(), path: common, url: String::new(), source: "/wp-includes/css/dist/block-library/common.min.css".into(), rtl: false });
    }
    let mut early: Vec<String> = Vec::new();
    for name in &early_blocks {
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !early.contains(handle) {
                early.push(handle.clone());
            }
        }
    }
    let mut queue: Vec<String> = Vec::new();
    for name in &state.used_blocks {
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !queue.contains(handle) {
                queue.push(handle.clone());
            }
        }
    }
    // `wp_hoist_late_printed_styles()`: the enqueued handles in the order of the style handles of all registered
    // blocks (a handle another block lists earlier, like `wp-block-buttons` of `core/post-comments-form`, comes there).
    let mut handles: Vec<String> = Vec::new();
    for name in registry.registration_order() {
        let Some(block_type) = registry.get(&name) else { continue };
        for handle in &block_type.styles {
            if queue.contains(handle) && !handles.contains(handle) {
                handles.push(handle.clone());
            }
        }
    }
    for handle in queue {
        if !handles.contains(&handle) {
            handles.push(handle);
        }
    }
    let mut queued_handles = handles.clone();
    for handle in &early {
        if !queued_handles.contains(handle) {
            queued_handles.push(handle.clone());
        }
    }
    handles.retain(|h| !early.contains(h));
    for handle in &handles {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        let path = registry.blocks_dir().join(slug).join("style.min.css");
        if rustpress_core::files::is_file(&path) {
            let url = format!("{includes_url}/blocks/{slug}/style.min.css?ver={version}");
            let source = url.split('?').next().unwrap_or(&url).to_owned();
            let variant = crate::document::rtl_variant(rtl, &path, &url);
            let is_rtl = variant.is_some();
            let (path, url) = variant.unwrap_or((path, url));
            sheets.push(Sheet { id: handle.clone(), path, source, url, rtl: is_rtl });
        }
    }
    // `wp_maybe_inline_styles()` runs twice: in `wp_head` for the common block styles alone (which gives them an
    // empty `src`), and in the footer for the block styles enqueued while the page rendered.
    let mut by_size: Vec<(usize, u64)> = sheets.iter().enumerate().filter(|(_, s)| s.id != "wp-block-library").filter_map(|(i, s)| rustpress_core::files::size(&s.path).map(|len| (i, len))).collect();
    by_size.sort_by_key(|(_, size)| *size);
    let mut budget = 0u64;
    let mut inline: Vec<usize> = sheets.iter().position(|s| s.id == "wp-block-library").into_iter().collect();
    for (i, size) in by_size {
        if budget + size > INLINE_LIMIT {
            break;
        }
        budget += size;
        inline.push(i);
    }
    let preview = preview(ctx);
    if preview.is_some_and(|p| p.widgets) {
        out.push_str(crate::customize_preview::WIDGETS_PREVIEW_CSS);
    }
    out.push_str("<style id=\"wp-img-auto-sizes-contain-inline-css\">\nimg:is([sizes=auto i],[sizes^=\"auto,\" i]){contain-intrinsic-size:3000px 1500px}\n/*# sourceURL=wp-img-auto-sizes-contain-inline-css */\n</style>\n");
    let toolbar = q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar);
    if preview.is_some() {
        out.push_str(&crate::customize_preview::styles(&includes_url, &version, rtl, toolbar));
    } else if toolbar {
        out.push_str(&crate::document::toolbar_styles(&includes_url, &version, rtl));
    }
    out.push_str(&emoji_style());
    for (i, sheet) in sheets.iter().enumerate() {
        if inline.contains(&i) {
            let css = read(&sheet.path).unwrap_or_default();
            let extra = if sheet.id == "wp-block-library" { "\n" } else { "" };
            out.push_str(&format!("<style id=\"{}-inline-css\">\n{}\n{extra}/*# sourceURL={} */\n</style>\n", sheet.id, css, sheet.source));
        } else {
            out.push_str(&format!("<link rel='stylesheet' id='{}{}-css' href='{}' media='all' />\n", sheet.id, if sheet.rtl { "-rtl" } else { "" }, sheet.url));
        }
    }
    if sheets.len() > usize::from(common_present) {
        out.push('\n');
    }
    if let Some(css) = read(&wp_root.join("wp-includes/css/classic-themes.min.css")) {
        out.push_str(&format!("<style id=\"classic-theme-styles-inline-css\">\n{}\n/*# sourceURL=/wp-includes/css/classic-themes.min.css */\n</style>\n", css));
    }
    let mut queued = queued_handles.clone();
    for name in &state.used_blocks {
        if let Some(short) = name.strip_prefix("core/") {
            let handle = format!("wp-block-{short}");
            if !queued.contains(&handle) {
                queued.push(handle);
            }
        }
    }
    let global = GlobalStyles::new(&theme.json, registry).with_other_blocks(ctx.site.plugins.block_names()).classic_chunks(Some(&queued)).join("\n");
    out.push_str(&format!("\n<style id=\"global-styles-inline-css\">\n{global}\n/*# sourceURL=global-styles-inline-css */\n</style>\n"));

    // The theme's styles (`twenty_twenty_one_scripts()`, `Twenty_Twenty_One_Dark_Mode::enqueue_scripts()`).
    out.push('\n');
    let plugin_assets = site.plugins.assets_for(site, Some(q), None).await;
    out.push_str(&crate::plugin_assets::StylePrinter::new(wp_root, site).links(&plugin_assets.styles));
    let style = if rtl { "style-rtl.css" } else { "style.css" };
    out.push_str(&format!("<link rel='stylesheet' id='twenty-twenty-one-style-css' href='{theme_uri}/{style}?ver={VERSION}' media='all' />\n"));
    let background = background_color(&ctx.mods);
    let mut inline_css = Vec::new();
    let non_latin = non_latin_css(site);
    if !non_latin.is_empty() {
        inline_css.push(non_latin);
    }
    if background.to_lowercase() != BACKGROUND.to_lowercase() {
        inline_css.push(custom_color_variables(&background));
    }
    if !inline_css.is_empty() {
        out.push_str(&format!("<style id=\"twenty-twenty-one-style-inline-css\">\n{}\n/*# sourceURL=twenty-twenty-one-style-inline-css */\n</style>\n", inline_css.join("\n")));
    }
    out.push_str(&format!("<link rel='stylesheet' id='twenty-twenty-one-print-style-css' href='{theme_uri}/assets/css/print.css?ver={VERSION}' media='print' />\n"));
    if dark_mode(ctx) {
        let file = if rtl { "style-dark-mode-rtl.css" } else { "style-dark-mode.css" };
        out.push_str(&format!("<link rel='stylesheet' id='tt1-dark-mode-css' href='{theme_uri}/assets/css/{file}?ver={VERSION}' media='all' />\n"));
    }
    if preview.is_some() {
        out.push_str(&crate::customize_preview::media_styles(&includes_url, &version));
    }
    for handle in &early {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        if rustpress_core::files::is_file(&registry.blocks_dir().join(slug).join("style.min.css")) {
            out.push_str(&format!("<link rel='stylesheet' id='{handle}-css' href='{includes_url}/blocks/{slug}/style.min.css?ver={version}' media='all' />\n"));
        }
    }
    // The scripts of the head.
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&includes_url, &script_version(wp_root, "jquery-core", "3.7.1"), &script_version(wp_root, "jquery-migrate", "3.4.1")).with_packages(wp_root, site);
    if preview.is_some() {
        out.push_str(&printer.core("jquery"));
    }
    out.push_str(&printer.scripts(&plugin_assets.scripts, false));
    if menu::has_nav_menu(site, &ctx.mods, "primary").await? {
        out.push_str(&format!("<script data-wp-strategy=\"defer\" defer id=\"twenty-twenty-one-primary-navigation-script-js\" src=\"{theme_uri}/assets/js/primary-navigation.js?ver={VERSION}\"></script>\n"));
    }

    out.push_str(&discovery_links(site, q, &version, 0, !plugin_head.removed.contains(&"rel_canonical"), &crate::document::locale_stylesheet(theme, site)).await?);
    // The plugins' actions of priority 10, before the theme's.
    out.push_str(&plugin_head.late);
    // `twenty_twenty_one_pingback_header()`.
    if let (true, Queried::Post(post)) = (q.flags.is_singular(), &q.queried)
        && post.ping_status == "open"
    {
        out.push_str(&format!("<link rel=\"pingback\" href=\"{}\">", esc_url(&site.site_url("xmlrpc.php"))));
    }
    // `recent_comments_style()`: an active "Recent Comments" widget.
    if preview.is_some() || widgets::sidebars_widgets(site).iter().any(|(name, ids)| name != "wp_inactive_widgets" && ids.iter().any(|id| id.starts_with("recent-comments-"))) {
        out.push_str("<style>.recentcomments a{display:inline !important;padding:0 !important;margin:0 !important;}</style>");
    }
    out.push_str(&custom_background_style(ctx));
    if let Some(preview) = preview {
        out.push_str(&crate::customize_preview::loading_style(preview));
    }
    let icon = super::wp_site_icon(ctx.site).await?;
    out.push_str(if icon.is_empty() && preview.is_some() { "<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"32x32\" />\n" } else { &icon });
    match custom_css(ctx).await? {
        Some(css) => out.push_str(&format!("<style id=\"wp-custom-css\">\n{css}\n</style>\n")),
        None if preview.is_some() => out.push_str("<style id=\"wp-custom-css\">\n\n</style>\n"),
        None => {}
    }
    // `wp_hoist_late_printed_styles()`.
    let mut late = String::new();
    if (state.mediaelement || state.playlist) && preview.is_none() {
        late.push_str(&format!("<link rel='stylesheet' id='mediaelement-css' href='{includes_url}/js/mediaelement/mediaelementplayer-legacy.min.css?ver=4.2.17' media='all' />\n<link rel='stylesheet' id='wp-mediaelement-css' href='{includes_url}/js/mediaelement/wp-mediaelement.min.css?ver={version}' media='all' />\n"));
    }
    let supports = state.block_supports_css.compile();
    if !supports.is_empty() {
        late.push_str(&format!("<style id=\"core-block-supports-inline-css\">\n{supports}\n/*# sourceURL=core-block-supports-inline-css */\n</style>\n"));
    }
    if !late.is_empty() {
        out.push_str(&late);
        out.push('\n');
    }
    Ok(out)
}

/// `wp_get_custom_css()`.
async fn custom_css(ctx: &Ctx<'_>) -> Result<Option<String>> {
    let css = match preview(ctx).and_then(|p| p.custom_css.clone()) {
        Some(css) => css,
        None => super::wp_get_custom_css(ctx.site, &ctx.mods).await?,
    };
    let css = css.trim().to_owned();
    Ok(if css.is_empty() { None } else { Some(css) })
}

// --------------------------------------------------------------------- footer

/// `Twenty_Twenty_One_Dark_Mode::the_switch()`.
fn dark_mode_switch(ctx: &Ctx<'_>) -> String {
    let side = if ctx.site.is_rtl() { "right" } else { "left" };
    let script = read(&ctx.theme.template_dir().join("assets/js/dark-mode-toggler.js")).unwrap_or_default();
    format!(
        "<button id=\"dark-mode-toggler\" class=\"fixed-bottom\" aria-pressed=\"false\" onClick=\"toggleDarkMode()\">{}</button>\t\t<style>\n\t\t\t#dark-mode-toggler > span {{\n\t\t\t\tmargin-{side}: 5px;\n\t\t\t}}\n\t\t\t#dark-mode-toggler > span::before {{\n\t\t\t\tcontent: '{}';\n\t\t\t}}\n\t\t\t#dark-mode-toggler[aria-pressed=\"true\"] > span::before {{\n\t\t\t\tcontent: '{}';\n\t\t\t}}\n\t\t\t\t\t</style>\n\n\t\t<script>\n{}\n//# sourceURL={}\n</script>\n",
        sprintf(&th(ctx, "Dark Mode: %s"), &["<span aria-hidden=\"true\"></span>"]),
        esc_attr(&t(ctx, "Off")),
        esc_attr(&t(ctx, "On")),
        script.trim_end(),
        rustpress_core::formatting::esc_url_raw(&format!("{}/assets/js/dark-mode-toggler.js", ctx.theme.template_directory_uri())),
    )
}

/// `wp_footer()`.
async fn wp_footer(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State) -> Result<String> {
    let site = ctx.site;
    let wp_root = &ctx.theme.wp_root;
    let version = wp_version(wp_root);
    let modules: Vec<String> = state.script_modules.iter().cloned().collect();
    let (importmap, preloads, module_scripts) = script_modules(wp_root, site, &modules);
    let mut looped = state.looped_posts.clone();
    if site.plugins.resets_query_in_head()
        && let Some(id) = crate::document::global_post_id(ctx.q)
    {
        looped.insert(0, id);
    }
    let plugin_footer = site.plugins.wp_footer(site, ctx.q, &looped).await?;
    let preview = preview(ctx);
    let playlist = if state.playlist || preview.is_some() { rustpress_blocks::shortcodes::playlist_templates(site) } else { String::new() };
    let speculation = if ctx.q.viewer.is_some() || preview.is_some() { String::new() } else { crate::document::speculation_rules(site, ctx.theme) };
    let nav_menus = preview.map(crate::customize_preview::nav_menus_exports).unwrap_or_default();
    // `twenty_twenty_one_supports_js()` and the switch of the dark mode run on `wp_footer` before the scripts.
    let supports_js = "<script>\ndocument.body.classList.remove('no-js');\n//# sourceURL=twenty_twenty_one_supports_js\n</script>\n";
    let switch = if dark_mode(ctx) { dark_mode_switch(ctx) } else { String::new() };
    let mut out = format!("{playlist}{speculation}{nav_menus}{plugin_footer}{supports_js}{switch}{importmap}{module_scripts}{preloads}{}", state.interactivity.data_script().unwrap_or_default());
    let includes_url = site.site_url("wp-includes");
    let plugin_assets = site.plugins.assets_for(site, Some(ctx.q), None).await;
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&includes_url, &script_version(wp_root, "jquery-core", "3.7.1"), &script_version(wp_root, "jquery-migrate", "3.4.1")).with_packages(wp_root, site);
    if ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar) {
        out.push_str(&format!("<script id=\"hoverintent-js-js\" src=\"{includes_url}/js/hoverintent-js.min.js?ver=2.2.1\"></script>\n<script id=\"admin-bar-js\" src=\"{includes_url}/js/admin-bar.min.js?ver={version}\"></script>\n"));
    }
    out.push_str(&printer.scripts(&plugin_assets.scripts, true));
    // `comment-reply`, enqueued by `twenty_twenty_one_scripts()` for singular pages with open comments.
    if let (true, Queried::Post(post)) = (ctx.q.flags.is_singular(), &ctx.q.queried)
        && post.comment_status == "open"
        && site.options.get("thread_comments") == Some("1")
    {
        out.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{includes_url}/js/comment-reply.min.js?ver={version}\"></script>\n"));
    }
    out.push_str(&format!("<script id=\"twenty-twenty-one-responsive-embeds-script-js\" src=\"{}/assets/js/responsive-embeds.js?ver={VERSION}\"></script>\n", ctx.theme.template_directory_uri()));
    if state.playlist {
        out.push_str(&printer.core("jquery"));
        out.push_str(&printer.core("wp-util"));
        out.push_str(&printer.core("backbone"));
    }
    if state.mediaelement || state.playlist {
        if !state.playlist {
            out.push_str(&printer.core("jquery"));
        }
        printer.mark_printed(&["jquery"]);
        out.push_str(&crate::document::mediaelement_scripts(site, wp_root, &includes_url, &version, state.vimeo, state.mediaelement));
    }
    if state.playlist {
        printer.mark_printed(&["mediaelement", "mediaelement-core", "mediaelement-migrate"]);
        out.push_str(&printer.core("wp-playlist"));
    }
    out.push_str(&emoji_detection_script(site, wp_root, &version));
    out.push_str(&site.plugins.print_footer_scripts(site, ctx.q));
    Ok(out)
}

// ----------------------------------------------------------------------- page

/// The callbacks of the partials the theme and WordPress register for the Customizer's preview.
pub async fn render_partial(ctx: &Ctx<'_>, id: &str, context: Option<&serde_json::Value>) -> Result<Option<serde_json::Value>> {
    let failed = || Ok(Some(serde_json::Value::Bool(false)));
    Ok(Some(serde_json::Value::String(match id {
        "custom_logo" => custom_logo(ctx).await?,
        _ => {
            if let Some(widget) = id.strip_prefix("widget[").and_then(|rest| rest.strip_suffix(']')) {
                if widgets::split_id(widget).is_none_or(|(base, number)| widgets::instance(ctx.site, base, number).is_none()) {
                    return Ok(None);
                }
                let Some(sidebar_id) = context.and_then(|c| c.get("sidebar_id")).and_then(serde_json::Value::as_str) else { return failed() };
                if sidebar_id != "sidebar-1" {
                    return failed();
                }
                let number = context.and_then(|c| c.get("sidebar_instance_number")).and_then(serde_json::Value::as_i64);
                let request_uri = ctx.request_uri();
                let search = || search_form(ctx);
                let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
                widgets::render_widget_partial(&services, &sidebar(), widget, number).await?
            } else {
                return Ok(None);
            }
        }
    })))
}

/// The classes `get_body_class()` gets from the theme: `Twenty_Twenty_One_Custom_Colors::body_class()` and
/// `twenty_twenty_one_body_classes()`.
pub(super) async fn add_body_classes(ctx: &Ctx<'_>, classes: &mut Vec<String>) -> Result<()> {
    let luminance = luminance(&background_color(&ctx.mods));
    classes.push(if 127 > luminance { "is-dark-theme".into() } else { "is-light-theme".into() });
    if 225 <= luminance {
        classes.push("has-background-white".into());
    }
    classes.push("no-js".into());
    classes.push(if ctx.q.flags.is_singular() { "singular".into() } else { "hfeed".into() });
    if menu::has_nav_menu(ctx.site, &ctx.mods, "primary").await? {
        classes.push("has-main-navigation".into());
    }
    if !active_sidebar(ctx, "sidebar-1") {
        classes.push("no-widgets".into());
    }
    Ok(())
}

/// `WP_REST_Widget_Types_Controller::render_legacy_widget_preview_iframe()`.
pub async fn legacy_widget_preview(ctx: &Ctx<'_>, registry: &Registry, id_base: &str, instance: Option<&Value>) -> Result<String> {
    let site = ctx.site;
    let mut classes = body_classes(ctx.theme, site, ctx.q).await?;
    add_body_classes(ctx, &mut classes).await?;
    let empty = Value::Array(Vec::new());
    let search = || search_form_markup(site, ctx.q, &ctx.env, "");
    let services = widgets::Services { site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: "", search_form: &search };
    let widget = widgets::the_widget(&services, id_base, instance.unwrap_or(&empty)).await?;
    let state = ctx.env.take_state();
    let head_html = head(ctx, &state, registry).await?;
    let footer_html = wp_footer(ctx, &state).await?;
    Ok(format!(
        "\t\t<!doctype html>\n\t\t<html {}lang=\"{}\">\n\t\t<head>\n\t\t\t<meta charset=\"UTF-8\" />\n\t\t\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\t\t\t<link rel=\"profile\" href=\"https://gmpg.org/xfn/11\" />\n\t\t\t{head_html}\t\t\t<style>\n\t\t\t\t/* Reset theme styles */\n\t\t\t\thtml, body, #page, #content {{\n\t\t\t\t\tpadding: 0 !important;\n\t\t\t\t\tmargin: 0 !important;\n\t\t\t\t}}\n\t\t\t</style>\n\t\t</head>\n\t\t<body class=\"{}\">\n\t\t<div id=\"page\" class=\"site\">\n\t\t\t<div id=\"content\" class=\"site-content\">\n\t\t\t\t{widget}\t\t\t</div><!-- #content -->\n\t\t</div><!-- #page -->\n\t\t{footer_html}\t\t</body>\n\t\t</html>\n\t\t",
        if site.is_rtl() { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
        esc_attr(&classes.join(" ")),
    ))
}

/// `twentytwentyone_the_html_classes()` with `Twenty_Twenty_One_Dark_Mode::html_classes()`.
fn html_classes(ctx: &Ctx<'_>) -> String {
    if dark_mode(ctx) { "class=\"respect-color-scheme-preference\"".to_owned() } else { String::new() }
}

pub async fn render(ctx: &Ctx<'_>, registry: &Registry, extras: &crate::document::Extras) -> Result<Document> {
    let site = ctx.site;
    let q = ctx.q;
    let mut classes = body_classes(ctx.theme, site, q).await?;
    add_body_classes(ctx, &mut classes).await?;
    ctx.env.with_state(|s| s.content_width = 750);
    let toolbar_on = ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar);
    let bar = match (&extras.toolbar, toolbar_on) {
        (Some(toolbar), true) => toolbar(&crate::document::ToolbarInfo { template_slug: "", query: ctx.q }).replace(" loading='lazy'", ""),
        _ => String::new(),
    };
    let mut body = header(ctx, &classes, &bar).await?;
    body.push_str(&main_template(ctx).await?);
    body.push_str(&footer(ctx).await?);
    let state = ctx.env.take_state();
    let head_html = head(ctx, &state, registry).await?;
    let footer_html = wp_footer(ctx, &state).await?;
    let html = format!(
        "<!doctype html>\n<html {}lang=\"{}\" {}>\n<head>\n\t<meta charset=\"UTF-8\" />\n\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n\t{head_html}</head>\n\n{body}{footer_html}\n</body>\n</html>\n",
        if site.is_rtl() { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
        html_classes(ctx),
    );
    Ok(Document { status: q.status(), html })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luminance_follows_the_theme() {
        assert_eq!(luminance("D1E4DD"), 223);
        assert_eq!(luminance("#fff"), 255);
        assert_eq!(luminance("000000"), 0);
        assert_eq!(readable_color("28303D"), "#fff");
    }

    #[test]
    fn icons_get_the_extra_attributes() {
        assert!(ui("menu").starts_with("<svg class=\"svg-icon\" width=\"24\" height=\"24\" aria-hidden=\"true\" role=\"img\" focusable=\"false\" viewBox=\"0 0 24 24\""));
        assert_eq!(svg("ui", "missing", 24), "");
    }

    #[test]
    fn social_links_match_their_domains() {
        assert!(social_link_svg("https://github.com/rustpress-cms", 24).is_some_and(|svg| svg.ends_with("<span class=\"screen-reader-text\">")));
        assert!(social_link_svg("https://mastodon.social/@x", 24).is_some());
        assert!(social_link_svg("https://example.org/", 24).is_none());
    }

    #[test]
    fn image_style_uses_the_original_size() {
        let mut attrs = vec![("class".to_owned(), AttrValue::Text("attachment-full".into()))];
        attachment_image_attributes(&mut attrs, Some((1500, 703)), None);
        assert!(matches!(&attrs[1], (k, AttrValue::Text(v)) if k == "style" && v == "width:100%;height:46.87%;max-width:1500px;"));
        let mut logo = vec![("class".to_owned(), AttrValue::Text("custom-logo".into()))];
        attachment_image_attributes(&mut logo, Some((10, 10)), None);
        assert_eq!(logo.len(), 1);
    }
}
