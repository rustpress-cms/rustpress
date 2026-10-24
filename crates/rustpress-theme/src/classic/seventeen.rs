//! Twenty Seventeen: the templates of the theme (`header.php`, `index.php`, `footer.php`, the
//! files in `template-parts/`) and what its `functions.php` adds to `wp_head()`/`wp_footer()`.
//!
//! The text between the PHP tags is kept as it is in the theme's files, including the quirks of
//! PHP's output (the newline right after `?>` is dropped), because the result is compared byte
//! for byte with the real theme.

use std::sync::Arc;

use rustpress_blocks::content::{get_the_content, post_class, the_content, the_title_for};
use rustpress_blocks::images::{self, AttrValue, ImageArgs, LoadingInput};
use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::{Queried, Site};
use rustpress_db::Post;
use rustpress_phpser::Value;

use super::comments;
use super::menu::{self, MenuItem, NavArgs};
use super::tags::{self, Pagination};
use super::widgets::{self, Sidebar};
use super::Ctx;
use crate::Result;
use crate::document::{
    Document, EMOJI_CSS, INLINE_LIMIT, body_classes, discovery_links, document_title, emoji_detection_script, emoji_style, oembed_links, read, robots_meta, script_modules, wp_version,
};
use crate::global_styles::GlobalStyles;

const THEME: &str = "twentyseventeen";

fn sidebar(id: &'static str) -> Sidebar {
    Sidebar { id, before_widget: "<section id=\"%1$s\" class=\"widget %2$s\">", after_widget: "</section>", before_title: "<h2 class=\"widget-title\">", after_title: "</h2>" }
}

/// `dynamic_sidebar()` for one of the theme's sidebars.
async fn sidebar_widgets(ctx: &Ctx<'_>, id: &'static str) -> Result<String> {
    let request_uri = ctx.request_uri();
    let search = || search_form(ctx);
    let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
    widgets::dynamic_sidebar(&services, &sidebar(id)).await
}

/// Translates with the theme's text domain.
fn t(ctx: &Ctx<'_>, text: &str) -> String {
    ctx.site.i18n().gettext(THEME, text)
}

fn tx(ctx: &Ctx<'_>, text: &str, context: &str) -> String {
    ctx.site.i18n().pgettext(THEME, context, text)
}

/// `twentyseventeen_get_svg()` without a title.
fn svg(icon: &str, fallback: bool) -> String {
    let icon = esc_attr(icon);
    let spare = if fallback { format!("<span class=\"svg-fallback icon-{icon}\"></span>") } else { String::new() };
    format!("<svg class=\"icon icon-{icon}\" aria-hidden=\"true\" role=\"img\"> <use href=\"#icon-{icon}\" xlink:href=\"#icon-{icon}\"></use> {spare}</svg>")
}

// ---------------------------------------------------------------- conditionals

fn is_frontpage(ctx: &Ctx<'_>) -> bool {
    ctx.q.flags.is_front_page && !ctx.q.flags.is_home
}

async fn has_header_image(ctx: &Ctx<'_>) -> bool {
    header_image_url(ctx).is_some()
}

/// `get_header_image()`: the theme mod, else the theme's default image.
fn header_image_url(ctx: &Ctx<'_>) -> Option<String> {
    let url = match ctx.mod_str("header_image") {
        Some(url) => url,
        None => format!("{}/assets/images/header.jpg", ctx.theme.template_directory_uri()),
    };
    if url == "remove-header" || url.is_empty() { None } else { Some(url) }
}

/// A property of a `stdClass` stored in a theme mod.
fn object_prop<'v>(value: &'v Value, name: &str) -> Option<&'v Value> {
    match value {
        Value::Object { props, .. } => props.iter().find_map(|(k, v)| match k {
            rustpress_phpser::Key::Str(s) if s == name.as_bytes() => Some(v),
            _ => None,
        }),
        Value::Array(_) => value.get(name),
        _ => None,
    }
}

async fn is_multi_author(site: &Site) -> Result<bool> {
    Ok(site.db.count_users_with_published_posts(&["post"]).await? > 1)
}

// ---------------------------------------------------------------------- header

/// `the_custom_header_markup()`.
async fn custom_header_markup(ctx: &Ctx<'_>) -> Result<String> {
    let Some(url) = header_image_url(ctx) else { return Ok(String::new()) };
    let data = ctx.mods.get("header_image_data");
    let width = data.and_then(|d| object_prop(d, "width")).and_then(Value::as_i64).unwrap_or(0).max(0) as u64;
    let height = data.and_then(|d| object_prop(d, "height")).and_then(Value::as_i64).unwrap_or(0).max(0) as u64;
    let attachment = data.and_then(|d| object_prop(d, "attachment_id")).and_then(Value::as_i64).unwrap_or(0).max(0) as u64;
    let mut alt = String::new();
    let mut attrs: Vec<(String, String)> = vec![("src".into(), url.clone()), ("width".into(), width.to_string()), ("height".into(), height.to_string())];
    let mut srcset = None;
    if attachment != 0 {
        let rows = ctx.site.db.post_meta(attachment).await?;
        if let Some(a) = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()) {
            alt = a;
        }
        if let Some(meta) = images::attachment_metadata(&ctx.env, attachment).await? {
            srcset = images::srcset(ctx.site, width, height, &url, &meta);
        }
    }
    attrs.push(("alt".into(), alt));
    if let Some(srcset) = srcset {
        attrs.push(("srcset".into(), srcset));
        // `twentyseventeen_header_image_tag()` replaces the generated sizes.
        attrs.push(("sizes".into(), "(max-width: 767px) 200vw, 100vw".into()));
    }
    let input = LoadingInput { width: Some(width), height: Some(height), ..LoadingInput::default() };
    attrs.extend(images::loading_attributes(&ctx.env, "img", &input, "get_header_image_tag"));
    let mut html = String::from("<img");
    for (name, value) in &attrs {
        html.push_str(&format!(" {name}=\"{}\"", esc_attr(value)));
    }
    html.push_str(" />");
    Ok(format!("<div id=\"wp-custom-header\" class=\"wp-custom-header\">{html}</div>"))
}

/// `the_custom_logo()`.
async fn custom_logo(ctx: &Ctx<'_>) -> Result<String> {
    let id = images::custom_logo_id(ctx.site).await?;
    if id == 0 {
        return Ok(String::new());
    }
    let rows = ctx.site.db.post_meta(id).await?;
    let alt_meta = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    let mut attrs: Vec<(String, AttrValue)> = vec![("class".into(), AttrValue::Text("custom-logo".into())), ("loading".into(), AttrValue::Omit)];
    if alt_meta.is_empty() || alt_meta == "0" {
        attrs.push(("alt".into(), AttrValue::Text(ctx.site.settings.blogname.clone())));
    }
    let image = images::get_attachment_image(&ctx.env, id, "full", &ImageArgs { attrs, post_thumbnail: false, dimensions: None }).await?;
    let q = ctx.q;
    let current = !q.flags.is_paged && (q.flags.is_front_page || (q.flags.is_home && ctx.site.settings.page_for_posts != q.queried_object_id()));
    let aria = if current { " aria-current=\"page\"" } else { "" };
    Ok(format!("<a href=\"{}\" class=\"custom-logo-link\" rel=\"home\"{aria}>{image}</a>", esc_url(&ctx.site.home_url("/"))))
}

async fn site_branding(ctx: &Ctx<'_>, has_top_menu: bool) -> Result<String> {
    let q = ctx.q;
    let site = ctx.site;
    let is_front = !q.flags.is_paged && (q.flags.is_front_page || (q.flags.is_home && site.settings.page_for_posts != q.queried_object_id()));
    let current = if is_front { "aria-current=\"page\"" } else { "" };
    let home = esc_url(&site.home_url("/"));
    let name = &site.settings.blogname;
    let title = if q.flags.is_front_page {
        format!("<h1 class=\"site-title\"><a href=\"{home}\" rel=\"home\" {current}>{name}</a></h1>")
    } else {
        format!("<p class=\"site-title\"><a href=\"{home}\" rel=\"home\" {current}>{name}</a></p>")
    };
    let description = &site.settings.blogdescription;
    let description = if description.is_empty() { String::new() } else { format!("<p class=\"site-description\">{description}</p>") };
    let scroll = if (is_frontpage(ctx) || (q.flags.is_home && q.flags.is_front_page)) && !has_top_menu {
        format!(
            "\t\t<a href=\"#content\" class=\"menu-scroll-down\">{}<span class=\"screen-reader-text\">\n\t\t\t{}\t\t</span></a>\n\t",
            svg("arrow-right", false),
            t(ctx, "Scroll down to content")
        )
    } else {
        String::new()
    };
    Ok(format!(
        "<div class=\"site-branding\">\n\t<div class=\"wrap\">\n\n\t\t{logo}\n\t\t<div class=\"site-branding-text\">\n\t\t\t\t\t\t\t\t\t\t{title}\n\t\t\t\n\t\t\t\t\t\t\t{description}\n\t\t\t\t\t</div><!-- .site-branding-text -->\n\n\t\t{scroll}\n\t</div><!-- .wrap -->\n</div><!-- .site-branding -->",
        logo = custom_logo(ctx).await?,
    ))
}

/// `wp_nav_menu( [ 'theme_location' => 'top', 'menu_id' => 'top-menu' ] )`.
async fn top_menu(ctx: &Ctx<'_>) -> Result<Option<String>> {
    let Some(id) = menu::menu_location(&ctx.mods, "top") else { return Ok(None) };
    let Some(mut menu) = menu::load_menu(ctx.site, id).await? else { return Ok(None) };
    let uri = ctx.request_uri();
    menu::classes_by_context(ctx.site, ctx.q, &uri, &mut menu.items).await?;
    let mut args = NavArgs::new("top");
    args.menu_id = "top-menu".into();
    let title_hook = |title: &str, item: &MenuItem, _depth: usize| {
        // `twentyseventeen_dropdown_icon_to_menu_link()`.
        if item.classes.iter().any(|c| c == "menu-item-has-children" || c == "page_item_has_children") { format!("{title}{}", svg("angle-down", false)) } else { title.to_owned() }
    };
    let link_hook = |link: String, _item: &MenuItem, _depth: usize| link;
    Ok(Some(menu::wp_nav_menu(ctx.site, &menu, &menu.items, &args, &title_hook, &link_hook)))
}

async fn navigation_top(ctx: &Ctx<'_>, menu_html: &str) -> Result<String> {
    let scroll = if (is_frontpage(ctx) || (ctx.q.flags.is_home && ctx.q.flags.is_front_page)) && has_header_image(ctx).await {
        format!(
            "\t\t<a href=\"#content\" class=\"menu-scroll-down\">{}<span class=\"screen-reader-text\">\n\t\t\t{}\t\t</span></a>\n\t",
            svg("arrow-right", false),
            t(ctx, "Scroll down to content")
        )
    } else {
        String::new()
    };
    Ok(format!(
        "<nav id=\"site-navigation\" class=\"main-navigation\" aria-label=\"{label}\">\n\t<button class=\"menu-toggle\" aria-controls=\"top-menu\" aria-expanded=\"false\">\n\t\t{bars}{close}{menu}\t</button>\n\n\t{menu_html}\n\t{scroll}</nav><!-- #site-navigation -->\n",
        label = esc_attr(&t(ctx, "Top Menu")),
        bars = svg("bars", false),
        close = svg("close", false),
        menu = t(ctx, "Menu"),
    ))
}

async fn header(ctx: &Ctx<'_>, classes: &[String]) -> Result<String> {
    ctx.env.with_state(|s| s.media.in_header = true);
    let top = top_menu(ctx).await?;
    let has_top_menu = top.is_some();
    let mut out = format!(
        "<body class=\"{}\">\n<div id=\"page\" class=\"site\">\n\t<a class=\"skip-link screen-reader-text\" href=\"#content\">\n\t\t{}\t</a>\n\n\t<header id=\"masthead\" class=\"site-header\">\n\n\t\t",
        esc_attr(&classes.join(" ")),
        t(ctx, "Skip to content")
    );
    out.push_str(&format!(
        "<div class=\"custom-header\">\n\n\t\t<div class=\"custom-header-media\">\n\t\t\t{}\t\t</div>\n\n\t{}\n\n</div><!-- .custom-header -->\n\n\t\t",
        custom_header_markup(ctx).await?,
        site_branding(ctx, has_top_menu).await?
    ));
    if let Some(menu_html) = top {
        out.push_str(&format!(
            "\t\t\t<div class=\"navigation-top\">\n\t\t\t\t<div class=\"wrap\">\n\t\t\t\t\t{}\t\t\t\t</div><!-- .wrap -->\n\t\t\t</div><!-- .navigation-top -->\n\t\t",
            navigation_top(ctx, &menu_html).await?
        ));
    }
    out.push_str("\n\t</header><!-- #masthead -->\n\n\t");
    // `twentyseventeen_should_show_featured_image()`.
    let q = ctx.q;
    if (q.flags.is_single || (q.flags.is_page && !is_frontpage(ctx))) && images::post_thumbnail_id(&ctx.env, q.queried_object_id()).await? != 0 {
        let image = images::get_attachment_image(&ctx.env, images::post_thumbnail_id(&ctx.env, q.queried_object_id()).await?, "twentyseventeen-featured-image", &ImageArgs { post_thumbnail: true, ..ImageArgs::default() }).await?;
        out.push_str(&format!("<div class=\"single-featured-image-header\">{image}</div><!-- .single-featured-image-header -->"));
    }
    ctx.env.with_state(|s| s.media.in_header = false);
    out.push_str("\n\t<div class=\"site-content-contain\">\n\t\t<div id=\"content\" class=\"site-content\">\n");
    Ok(out)
}

// ---------------------------------------------------------------------- footer

async fn footer(ctx: &Ctx<'_>) -> Result<String> {
    let mut widgets_html = String::from("\n");
    let two = widgets::is_active_sidebar(ctx.site, "sidebar-2");
    let three = widgets::is_active_sidebar(ctx.site, "sidebar-3");
    if two || three {
        widgets_html = format!("\n\n\t<aside class=\"widget-area\" aria-label=\"{}\">\n\t\t", esc_attr(&t(ctx, "Footer")));
        if two {
            widgets_html.push_str(&format!("\t\t\t<div class=\"widget-column footer-widget-1\">\n\t\t\t\t{}\t\t\t</div>\n\t\t\t", sidebar_widgets(ctx, "sidebar-2").await?));
        }
        if three {
            widgets_html.push_str(&format!("\t\t\t<div class=\"widget-column footer-widget-2\">\n\t\t\t\t{}\t\t\t</div>\n\t\t", sidebar_widgets(ctx, "sidebar-3").await?));
        }
        widgets_html.push_str("\t</aside><!-- .widget-area -->\n\n\t");
    }
    // `the_privacy_policy_link()` is empty without a privacy policy page.
    let privacy = privacy_policy_link(ctx).await?;
    // The credit names RustPress (the compat run ignores this link); it points at the site itself.
    let imprint_url = esc_url(&ctx.site.home_url("/"));
    let powered = rustpress_core::i18n::sprintf(&t(ctx, "Proudly powered by %s"), &["RustPress"]);
    Ok(format!(
        "\n\t\t</div><!-- #content -->\n\n\t\t<footer id=\"colophon\" class=\"site-footer\">\n\t\t\t<div class=\"wrap\">\n\t\t\t\t{widgets_html}<div class=\"site-info\">\n\t{privacy}\t<a href=\"{imprint_url}\" class=\"imprint\">\n\t\t{powered}\t</a>\n</div><!-- .site-info -->\n\t\t\t</div><!-- .wrap -->\n\t\t</footer><!-- #colophon -->\n\t</div><!-- .site-content-contain -->\n</div><!-- #page -->\n"
    ))
}

async fn privacy_policy_link(ctx: &Ctx<'_>) -> Result<String> {
    let id = ctx.site.options.u64("wp_page_for_privacy_policy", 0);
    if id == 0 {
        return Ok(String::new());
    }
    let Some(page) = ctx.site.db.post(id).await? else { return Ok(String::new()) };
    if page.post_status != "publish" {
        return Ok(String::new());
    }
    Ok(format!("<a class=\"privacy-policy-link\" href=\"{}\" rel=\"privacy-policy\">{}</a><span role=\"separator\" aria-hidden=\"true\"></span>", esc_url(&ctx.site.permalink(&page).await?), esc_html(&page.post_title)))
}

// ------------------------------------------------------------------- the loop

/// `twentyseventeen_time_link()`.
fn time_link(ctx: &Ctx<'_>, post: &Post, permalink: &str) -> String {
    let site = ctx.site;
    let posted = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
    let modified = site.timezone.local_timestamp(&post.post_modified).unwrap_or(0);
    let date_format = site.options.string("date_format", "F j, Y");
    let time_string = if posted != modified {
        format!(
            "<time class=\"entry-date published\" datetime=\"{}\">{}</time><time class=\"updated\" datetime=\"{}\">{}</time>",
            site.date("c", posted),
            site.date(&date_format, posted),
            site.date("c", modified),
            site.date(&date_format, modified)
        )
    } else {
        format!("<time class=\"entry-date published updated\" datetime=\"{}\">{}</time>", site.date("c", posted), site.date(&date_format, posted))
    };
    rustpress_core::i18n::sprintf(&t(ctx, "<span class=\"screen-reader-text\">Posted on</span> %s"), &[&format!("<a href=\"{}\" rel=\"bookmark\">{time_string}</a>", esc_url(permalink))])
}

/// `twentyseventeen_posted_on()`.
async fn posted_on(ctx: &Ctx<'_>, post: &Post, permalink: &str) -> Result<String> {
    let author = ctx.site.db.user(post.post_author).await?;
    let (name, url) = match &author {
        Some(user) => (user.display_name.clone(), ctx.site.author_url(user)),
        None => (String::new(), String::new()),
    };
    let byline = rustpress_core::i18n::sprintf(&t(ctx, "by %s"), &[&format!("<span class=\"author vcard\"><a class=\"url fn n\" href=\"{}\">{}</a></span>", esc_url(&url), name)]);
    Ok(format!("<span class=\"posted-on\">{}</span><span class=\"byline\"> {byline}</span>", time_link(ctx, post, permalink)))
}

/// `twentyseventeen_categorized_blog()`.
async fn categorized_blog(ctx: &Ctx<'_>) -> Result<bool> {
    let categories = ctx.site.db.terms("category").await?;
    Ok(categories.iter().filter(|c| c.count > 0).count() > 1)
}

/// `twentyseventeen_entry_footer()`.
async fn entry_footer(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let separator = t(ctx, ", ");
    let categories = tags::category_list(ctx.site, post, &separator).await?;
    let tag_list = tags::tag_list(ctx.site, post, &separator).await?;
    let categorized = categorized_blog(ctx).await?;
    if !(((categorized && !categories.is_empty()) || !tag_list.is_empty()) || false) {
        return Ok(String::new());
    }
    let mut out = String::from("<footer class=\"entry-footer\">");
    if post.post_type == "post" {
        out.push_str("<span class=\"cat-tags-links\">");
        if categorized && !categories.is_empty() {
            out.push_str(&format!("<span class=\"cat-links\">{}<span class=\"screen-reader-text\">{}</span>{categories}</span>", svg("folder-open", false), t(ctx, "Categories")));
        }
        if !tag_list.is_empty() {
            out.push_str(&format!("<span class=\"tags-links\">{}<span class=\"screen-reader-text\">{}</span>{tag_list}</span>", svg("hashtag", false), t(ctx, "Tags")));
        }
        out.push_str("</span>");
    }
    out.push_str("</footer> <!-- .entry-footer -->");
    Ok(out)
}

/// `twentyseventeen_content_image_sizes_attr()` (filter `wp_calculate_image_sizes`).
pub fn calculated_sizes(site: &Site, q: &rustpress_core::QueryResult, stylesheet: &str, sizes: Option<String>, width: u64) -> Option<String> {
    let mut sizes = sizes;
    if 740 <= width {
        sizes = Some("(max-width: 706px) 89vw, (max-width: 767px) 82vw, 740px".to_owned());
    }
    let mods = menu::theme_mod(site, stylesheet);
    let one_column_page = q.flags.is_page && mods.get("page_options").and_then(Value::as_str) == Some("one-column");
    if (widgets::is_active_sidebar(site, "sidebar-1") || q.flags.is_archive || q.flags.is_search || q.flags.is_home || q.flags.is_page) && !one_column_page && 767 <= width {
        sizes = Some("(max-width: 767px) 89vw, (max-width: 1000px) 54vw, (max-width: 1071px) 543px, 580px".to_owned());
    }
    sizes
}

/// `twentyseventeen_excerpt_more()` as a template (`%1$s` permalink, `%2$s` title).
pub fn excerpt_more_template(i18n: &rustpress_core::i18n::I18n) -> String {
    let text = i18n.gettext(THEME, "Continue reading<span class=\"screen-reader-text\"> \"%s\"</span>").replace("%s", "%2$s");
    format!(" &hellip; <p class=\"link-more\"><a href=\"%1$s\" class=\"more-link\">{text}</a></p>")
}

/// `twentyseventeen_post_thumbnail_sizes_attr()` (filter `wp_get_attachment_image_attributes`).
pub fn attachment_sizes_attr(_site: &Site, q: &rustpress_core::QueryResult) -> String {
    if q.flags.is_archive || q.flags.is_search || q.flags.is_home { "(max-width: 767px) 89vw, (max-width: 1000px) 54vw, (max-width: 1071px) 543px, 580px".to_owned() } else { "100vw".to_owned() }
}
/// One article of the loop: `template-parts/post/content.php`.
async fn content_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let site = ctx.site;
    let q = ctx.q;
    let permalink = site.permalink(post).await?;
    let is_single = q.flags.is_single;
    let is_home_unpaged = q.flags.is_home && !q.flags.is_paged;
    let classes = post_class(&ctx.env, post, &[], is_home_unpaged, q.flags.is_singular()).await?;
    let sticky = if site.settings.sticky_posts.contains(&post.id) && q.flags.is_home && is_home_unpaged { svg("thumb-tack", false) } else { String::new() };
    let mut meta = String::new();
    if post.post_type == "post" {
        meta.push_str("<div class=\"entry-meta\">");
        if is_single {
            meta.push_str(&posted_on(ctx, post, &permalink).await?);
        } else {
            meta.push_str(&time_link(ctx, post, &permalink));
        }
        meta.push_str("</div><!-- .entry-meta -->");
    }
    let title = the_title_for(site, post);
    let heading = if title.is_empty() {
        String::new()
    } else if is_single {
        format!("<h1 class=\"entry-title\">{title}</h1>")
    } else if q.flags.is_front_page && q.flags.is_home {
        format!("<h3 class=\"entry-title\"><a href=\"{}\" rel=\"bookmark\">{title}</a></h3>", esc_url(&permalink))
    } else {
        format!("<h2 class=\"entry-title\"><a href=\"{}\" rel=\"bookmark\">{title}</a></h2>", esc_url(&permalink))
    };
    // The thumbnail of archives and the blog. `get_the_post_thumbnail()` is evaluated in the condition
    // even where `! is_single()` then rules the block out, and counts as media each time.
    let mut thumbnail = String::new();
    {
        let id = images::post_thumbnail_id(&ctx.env, post.id).await?;
        if id != 0 && is_single {
            images::get_attachment_image(&ctx.env, id, "twentyseventeen-featured-image", &ImageArgs { post_thumbnail: true, ..ImageArgs::default() }).await?;
        } else if id != 0 {
            // `get_the_post_thumbnail()` runs in the condition and again in `the_post_thumbnail()`:
            // both calls count as media, and the second one is printed.
            let args = ImageArgs { post_thumbnail: true, ..ImageArgs::default() };
            let first = images::get_attachment_image(&ctx.env, id, "twentyseventeen-featured-image", &args).await?;
            let image = images::get_attachment_image(&ctx.env, id, "twentyseventeen-featured-image", &args).await?;
            if !first.is_empty() && !image.is_empty() {
                thumbnail = format!("\t\t<div class=\"post-thumbnail\">\n\t\t\t<a href=\"{}\">\n\t\t\t\t{image}\t\t\t</a>\n\t\t</div><!-- .post-thumbnail -->\n\t", esc_url(&permalink));
            }
        }
    }
    let raw = get_the_content(&ctx.env, post, 1, is_single);
    let content = the_content(&ctx.env, post, &raw).await?;
    let footer = if is_single { entry_footer(ctx, post).await? } else { String::new() };
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\t{sticky}\t<header class=\"entry-header\">\n\t\t{meta}{heading}\t</header><!-- .entry-header -->\n\n\t{thumbnail}\n\t<div class=\"entry-content\">\n\t\t{content}\t</div><!-- .entry-content -->\n\n\t{footer}\n</article><!-- #post-{id} -->\n",
        id = post.id,
        classes = esc_attr(&classes.join(" ")),
    ))
}

async fn pagination(ctx: &Ctx<'_>) -> String {
    tags::posts_pagination(
        ctx.site,
        ctx.q,
        &Pagination {
            prev_text: format!("{}<span class=\"screen-reader-text\">{}</span>", svg("arrow-left", false), t(ctx, "Previous page")),
            next_text: format!("<span class=\"screen-reader-text\">{}</span>{}", t(ctx, "Next page"), svg("arrow-right", false)),
            before_page_number: format!("<span class=\"meta-nav screen-reader-text\">{} </span>", t(ctx, "Page")),
        },
    )
}

async fn get_sidebar(ctx: &Ctx<'_>) -> Result<String> {
    if !widgets::is_active_sidebar(ctx.site, "sidebar-1") {
        return Ok(String::new());
    }
    Ok(format!(
        "\n<aside id=\"secondary\" class=\"widget-area\" aria-label=\"{}\">\n\t{}</aside><!-- #secondary -->\n",
        esc_attr(&t(ctx, "Blog Sidebar")),
        sidebar_widgets(ctx, "sidebar-1").await?
    ))
}

/// `get_search_form()` with the theme's `searchform.php`.
fn search_form(ctx: &Ctx<'_>) -> String {
    let unique = ctx.env.with_state(|s| s.unique_id("search-form-"));
    let query = esc_attr(ctx.q.vars.get("s").unwrap_or(""));
    format!(
        "\n\n<form role=\"search\" method=\"get\" class=\"search-form\" action=\"{}\">\n\t<label for=\"{unique}\">\n\t\t<span class=\"screen-reader-text\">\n\t\t\t{}\t\t</span>\n\t</label>\n\t<input type=\"search\" id=\"{unique}\" class=\"search-field\" placeholder=\"{}\" value=\"{query}\" name=\"s\" />\n\t<button type=\"submit\" class=\"search-submit\">{}<span class=\"screen-reader-text\">\n\t\t{}\t</span></button>\n</form>\n",
        esc_url(&ctx.site.home_url("/")),
        tx(ctx, "Search for:", "label"),
        esc_attr(&tx(ctx, "Search &hellip;", "placeholder")),
        svg("search", false),
        tx(ctx, "Search", "submit button"),
    )
}

/// The archive title and description (`the_archive_title()` and `the_archive_description()`).
async fn archive_header(ctx: &Ctx<'_>) -> Result<String> {
    let q = ctx.q;
    let site = ctx.site;
    let f = &q.flags;
    let (title, description) = match &q.queried {
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
            let prefix = if f.is_category { site.trx("Category:", "category archive title prefix") } else if f.is_tag { site.trx("Tag:", "tag archive title prefix") } else { format!("{}:", term.taxonomy) };
            (format!("{prefix} <span>{}</span>", term.name), term.description.clone())
        }
        Queried::User(user) if f.is_author => (format!("{} <span>{}</span>", site.trx("Author:", "author archive title prefix"), user.display_name), String::new()),
        _ if f.is_year || f.is_month || f.is_day => {
            let first = q.posts.first();
            let ts = first.and_then(|p| site.timezone.local_timestamp(&p.post_date)).unwrap_or(0);
            let (prefix, format) = if f.is_day {
                (site.trx("Day:", "date archive title prefix"), site.trx("F j, Y", "daily archives date format"))
            } else if f.is_month {
                (site.trx("Month:", "date archive title prefix"), site.trx("F Y", "monthly archives date format"))
            } else {
                (site.trx("Year:", "date archive title prefix"), site.trx("Y", "yearly archives date format"))
            };
            (format!("{prefix} <span>{}</span>", site.date(&format, ts)), String::new())
        }
        _ => (site.tr("Archives"), String::new()),
    };
    let mut out = format!("<h1 class=\"page-title\">{title}</h1>");
    if !description.is_empty() {
        out.push_str(&format!("<div class=\"taxonomy-description\">{description}</div>"));
    }
    Ok(out)
}

/// `template-parts/post/content-excerpt.php`.
async fn excerpt_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let site = ctx.site;
    let q = ctx.q;
    let permalink = site.permalink(post).await?;
    let classes = post_class(&ctx.env, post, &[], q.flags.is_home && !q.flags.is_paged, q.flags.is_singular()).await?;
    let meta = if post.post_type == "post" { format!("\t\t\t<div class=\"entry-meta\">\n\t\t\t\t{}\t\t\t</div><!-- .entry-meta -->\n\t\t", time_link(ctx, post, &permalink)) } else { String::new() };
    let title = the_title_for(site, post);
    let heading = if title.is_empty() {
        String::new()
    } else if q.flags.is_front_page && !q.flags.is_home {
        format!("<h3 class=\"entry-title\"><a href=\"{}\" rel=\"bookmark\">{title}</a></h3>", esc_url(&permalink))
    } else {
        format!("<h2 class=\"entry-title\"><a href=\"{}\" rel=\"bookmark\">{title}</a></h2>", esc_url(&permalink))
    };
    let excerpt = rustpress_blocks::excerpt::get_the_excerpt(&ctx.env, post).await?;
    let excerpt = rustpress_core::formatting::wpautop(&rustpress_core::formatting::convert_chars(&site.texturize(&excerpt)));
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\n\t<header class=\"entry-header\">\n\t\t{meta}\n\t\t{heading}\t</header><!-- .entry-header -->\n\n\t<div class=\"entry-summary\">\n\t\t{excerpt}\t</div><!-- .entry-summary -->\n\n</article><!-- #post-{id} -->\n",
        id = post.id,
        classes = esc_attr(&classes.join(" ")),
    ))
}

/// `template-parts/page/content-page.php`.
async fn page_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let classes = post_class(&ctx.env, post, &[], false, true).await?;
    let title = the_title_for(ctx.site, post);
    let raw = get_the_content(&ctx.env, post, 1, true);
    let content = the_content(&ctx.env, post, &raw).await?;
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\t<header class=\"entry-header\">\n\t\t<h1 class=\"entry-title\">{title}</h1>\t\t\t</header><!-- .entry-header -->\n\t<div class=\"entry-content\">\n\t\t{content}\t</div><!-- .entry-content -->\n</article><!-- #post-{id} -->\n",
        id = post.id,
        classes = esc_attr(&classes.join(" ")),
    ))
}

/// `the_post_navigation()` of `single.php`.
async fn post_navigation(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let site = ctx.site;
    let mut links = String::new();
    for previous in [true, false] {
        let Some(id) = site.db.adjacent_post_id(&post.post_type, &post.post_date, post.id, previous).await? else { continue };
        let Some(other) = site.db.post(id).await? else { continue };
        if other.post_status != "publish" {
            continue;
        }
        let mut title = other.post_title.clone();
        if title.is_empty() {
            title = if previous { site.tr("Previous Post") } else { site.tr("Next Post") };
        }
        let title = super::menu::the_title_filters(site, &title);
        let template = if previous {
            format!("<span class=\"screen-reader-text\">{}</span><span aria-hidden=\"true\" class=\"nav-subtitle\">{}</span> <span class=\"nav-title\"><span class=\"nav-title-icon-wrapper\">{}</span>%title</span>", t(ctx, "Previous Post"), t(ctx, "Previous"), svg("arrow-left", false))
        } else {
            format!("<span class=\"screen-reader-text\">{}</span><span aria-hidden=\"true\" class=\"nav-subtitle\">{}</span> <span class=\"nav-title\">%title<span class=\"nav-title-icon-wrapper\">{}</span></span>", t(ctx, "Next Post"), t(ctx, "Next"), svg("arrow-right", false))
        };
        let inner = template.replace("%title", &title);
        let anchor = format!("<a href=\"{}\" rel=\"{}\">{inner}</a>", esc_url(&site.permalink(&other).await?), if previous { "prev" } else { "next" });
        links.push_str(&format!("<div class=\"nav-{}\">{anchor}</div>", if previous { "previous" } else { "next" }));
    }
    if links.is_empty() {
        return Ok(String::new());
    }
    Ok(tags::navigation_markup(&links, "post-navigation", &site.tr("Post navigation"), &site.tr("Posts")))
}

/// The body of the main template (`index.php`, `archive.php`, `search.php`, ...).
async fn main_template(ctx: &Ctx<'_>) -> Result<String> {
    let q = ctx.q;
    let f = &q.flags;
    ctx.env.with_state(|s| s.media.in_main_loop = true);
    let mut loop_html = String::new();
    for post in &q.posts {
        let post = Arc::new(post.clone());
        if f.is_404 {
            break;
        }
        loop_html.push_str(&if f.is_page {
            page_part(ctx, &post).await?
        } else if f.is_search {
            excerpt_part(ctx, &post).await?
        } else {
            content_part(ctx, &post).await?
        });
        if (f.is_single || f.is_page) && (comments::comments_open(&post) || post.comment_count > 0) {
            let args = comments::ListArgs { avatar_size: 100, reply_text: format!("{}{}", svg("mail-reply", false), t(ctx, "Reply")), short_ping: true };
            let text = |s: &str| t(ctx, s);
            let text_x = |s: &str, c: &str| tx(ctx, s, c);
            loop_html.push_str(&comments::comments_template(ctx.site, &ctx.env, &post, &args, &ctx.request_uri(), &text, &text_x).await?);
        }
        if f.is_single {
            loop_html.push_str(&post_navigation(ctx, &post).await?);
        }
    }
    ctx.env.with_state(|s| s.media.in_main_loop = false);

    if f.is_404 {
        return Ok(format!(
            "\n<div class=\"wrap\">\n\t<div id=\"primary\" class=\"content-area\">\n\t\t<main id=\"main\" class=\"site-main\">\n\n\t\t\t<section class=\"error-404 not-found\">\n\t\t\t\t<header class=\"page-header\">\n\t\t\t\t\t<h1 class=\"page-title\">{}</h1>\n\t\t\t\t</header><!-- .page-header -->\n\t\t\t\t<div class=\"page-content\">\n\t\t\t\t\t<p>{}</p>\n\n\t\t\t\t\t{}\n\t\t\t\t</div><!-- .page-content -->\n\t\t\t</section><!-- .error-404 -->\n\t\t</main><!-- #main -->\n\t</div><!-- #primary -->\n</div><!-- .wrap -->\n\n",
            t(ctx, "Oops! That page can&rsquo;t be found."),
            t(ctx, "It looks like nothing was found at this location. Maybe try a search?"),
            search_form(ctx)
        ));
    }
    if f.is_singular() {
        let tail = if f.is_page { "</div><!-- #primary -->\n" } else { "</div><!-- #primary -->\n\t" };
        let sidebar = if f.is_page { String::new() } else { get_sidebar(ctx).await? };
        return Ok(format!(
            "\n<div class=\"wrap\">\n\t<div id=\"primary\" class=\"content-area\">\n\t\t<main id=\"main\" class=\"site-main\">\n\n\t\t\t{loop_html}\n\t\t</main><!-- #main -->\n\t{tail}{sidebar}</div><!-- .wrap -->\n\n",
        ));
    }
    let pagination_html = pagination(ctx).await;
    let lead = if f.is_search || f.is_archive { "\n" } else { "" };
    let header = if f.is_search {
        let title = if q.posts.is_empty() {
            format!("\t\t\t<h1 class=\"page-title\">{}</h1>\n\t\t", t(ctx, "Nothing Found"))
        } else {
            format!(
                "\t\t\t<h1 class=\"page-title\">\n\t\t\t{}\t\t\t</h1>\n\t\t",
                rustpress_core::i18n::sprintf(&t(ctx, "Search Results for: %s"), &[&format!("<span>{}</span>", esc_html(q.vars.get("s").unwrap_or("")))])
            )
        };
        format!("\t<header class=\"page-header\">\n\t\t{title}\t</header><!-- .page-header -->\n")
    } else if f.is_archive {
        if q.posts.is_empty() { "\t".to_owned() } else { format!("\t\t\t<header class=\"page-header\">\n\t\t\t{}\t\t</header><!-- .page-header -->\n\t", archive_header(ctx).await?) }
    } else if f.is_home && !f.is_front_page {
        let page_title = match &q.queried {
            Queried::Post(page) => the_title_for(ctx.site, page),
            _ => String::new(),
        };
        format!("\t\t\t<header class=\"page-header\">\n\t\t\t<h1 class=\"page-title\">{page_title}</h1>\n\t\t</header>\n\t")
    } else {
        format!("\t\t<header class=\"page-header\">\n\t\t<h2 class=\"page-title\">{}</h2>\n\t</header>\n\t", t(ctx, "Posts"))
    };
    let main_indent = if f.is_search { "\t\t" } else if f.is_archive { "\t\t\t\t\t" } else { "\t\t\t" };
    let body = if q.posts.is_empty() && f.is_search {
        format!("\n\t\t\t<p>{}</p>\n\t\t\t{}", t(ctx, "Sorry, but nothing matched your search terms. Please try again with some different keywords."), search_form(ctx))
    } else if q.posts.is_empty() {
        format!("\n\n<section class=\"no-results not-found\">\n\t<header class=\"page-header\">\n\t\t<h1 class=\"page-title\">{}</h1>\n\t</header>\n\n\t<div class=\"page-content\">\n\t\t\t<p>{}</p>\n\t\t\t{}\t</div><!-- .page-content -->\n</section><!-- .no-results -->\n", t(ctx, "Nothing Found"), t(ctx, "It seems we can&rsquo;t find what you&rsquo;re looking for. Perhaps searching can help."), search_form(ctx))
    } else {
        format!("{loop_html}{pagination_html}")
    };
    // The sidebar comes after the content, so `wp_unique_id()` counts in WordPress order.
    let sidebar_html = get_sidebar(ctx).await?;
    Ok(format!(
        "\n<div class=\"wrap\">\n{lead}{header}\n\t<div id=\"primary\" class=\"content-area\">\n\t\t<main id=\"main\" class=\"site-main\">\n\n{main_indent}{body}\n\t\t</main><!-- #main -->\n\t</div><!-- #primary -->\n\t{sidebar_html}</div><!-- .wrap -->\n\n"
    ))
}
// ------------------------------------------------------------------------ head

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

async fn head(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State, registry: &Registry) -> Result<String> {
    let site = ctx.site;
    let theme = ctx.theme;
    let q = ctx.q;
    let wp_root = &theme.wp_root;
    let version = wp_version(wp_root);
    let includes_url = site.site_url("wp-includes");
    let theme_uri = theme.template_directory_uri();
    let mut out = String::new();

    out.push_str("<script>(function(html){html.className = html.className.replace(/\\bno-js\\b/,'js')})(document.documentElement);</script>\n");
    out.push_str(&format!("<title>{}</title>\n", esc_html(&document_title(site, q).await?)));
    out.push_str(&robots_meta(site, q));
    out.push_str(&crate::head::feed_links(site, q).await?);
    out.push_str(&oembed_links(site, q).await?);

    // --- styles in the order WordPress prints them
    struct Sheet {
        id: String,
        path: std::path::PathBuf,
        url: String,
        source: String,
    }
    let mut sheets: Vec<Sheet> = Vec::new();
    let common = wp_root.join("wp-includes/css/dist/block-library/common.min.css");
    let common_present = common.is_file();
    if common.is_file() {
        sheets.push(Sheet { id: "wp-block-library".into(), path: common, url: String::new(), source: "/wp-includes/css/dist/block-library/common.min.css".into() });
    }
    let mut handles: Vec<String> = Vec::new();
    for name in &state.used_blocks {
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !handles.contains(handle) {
                handles.push(handle.clone());
            }
        }
    }
    handles.sort();
    for handle in &handles {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        let path = registry.blocks_dir().join(slug).join("style.min.css");
        if path.is_file() {
            let url = format!("{includes_url}/blocks/{slug}/style.min.css?ver={version}");
            sheets.push(Sheet { id: handle.clone(), path, source: url.split('?').next().unwrap_or(&url).to_owned(), url });
        }
    }
    // Smallest files first until the inline budget is used up.
    let mut by_size: Vec<(usize, u64)> = sheets.iter().enumerate().filter_map(|(i, s)| std::fs::metadata(&s.path).ok().map(|m| (i, m.len()))).collect();
    by_size.sort_by_key(|(_, size)| *size);
    let mut budget = 0u64;
    let mut inline: Vec<usize> = Vec::new();
    for (i, size) in by_size {
        if budget + size > INLINE_LIMIT {
            break;
        }
        budget += size;
        inline.push(i);
    }
    out.push_str("<style id=\"wp-img-auto-sizes-contain-inline-css\">\nimg:is([sizes=auto i],[sizes^=\"auto,\" i]){contain-intrinsic-size:3000px 1500px}\n/*# sourceURL=wp-img-auto-sizes-contain-inline-css */\n</style>\n");
    out.push_str(&emoji_style());
    for (i, sheet) in sheets.iter().enumerate() {
        if inline.contains(&i) {
            let css = read(&sheet.path).unwrap_or_default();
            // The core adds an (empty) inline style to the common block stylesheet.
            let extra = if sheet.id == "wp-block-library" { "\n" } else { "" };
            out.push_str(&format!("<style id=\"{}-inline-css\">\n{}\n{extra}/*# sourceURL={} */\n</style>\n", sheet.id, css, sheet.source));
        } else {
            out.push_str(&format!("<link rel='stylesheet' id='{}-css' href='{}' media='all' />\n", sheet.id, sheet.url));
        }
    }
    let _ = EMOJI_CSS;
    // The print pass of the on-demand block styles ends with a newline.
    if sheets.len() > usize::from(common_present) {
        out.push('\n');
    }
    if let Some(css) = read(&wp_root.join("wp-includes/css/classic-themes.min.css")) {
        out.push_str(&format!("<style id=\"classic-theme-styles-inline-css\">\n{}\n/*# sourceURL=/wp-includes/css/classic-themes.min.css */\n</style>\n", css));
    }
    // Global styles of the core theme.json: presets and the layout rules of the blocks in use.
    let mut queued = handles.clone();
    for name in &state.used_blocks {
        if let Some(short) = name.strip_prefix("core/") {
            let handle = format!("wp-block-{short}");
            if !queued.contains(&handle) {
                queued.push(handle);
            }
        }
    }
    let global = GlobalStyles::new(&theme.json, registry).classic_chunks(Some(&queued)).join("\n");
    out.push_str(&format!("\n<style id=\"global-styles-inline-css\">\n{global}\n/*# sourceURL=global-styles-inline-css */\n</style>\n"));

    // The theme's own styles and scripts (`twentyseventeen_scripts()`).
    let fonts_on = tx(ctx, "on", "Libre Franklin font: on or off") != "off";
    out.push('\n');
    if fonts_on {
        out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-fonts-css' href='{theme_uri}/assets/fonts/font-libre-franklin.css?ver=20230328' media='all' />\n"));
    }
    out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-style-css' href='{theme_uri}/style.css?ver=20250415' media='all' />\n"));
    out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-block-style-css' href='{theme_uri}/assets/css/blocks.css?ver=20240729' media='all' />\n"));
    if ctx.mod_str("colorscheme").as_deref() == Some("dark") {
        out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-colors-dark-css' href='{theme_uri}/assets/css/colors-dark.css?ver=20240412' media='all' />\n"));
    }
    let jquery = script_version(wp_root, "jquery-core", "3.7.1");
    let migrate = script_version(wp_root, "jquery-migrate", "3.4.1");
    out.push_str(&format!("<script id=\"jquery-core-js\" src=\"{includes_url}/js/jquery/jquery.min.js?ver={jquery}\"></script>\n"));
    out.push_str(&format!("<script id=\"jquery-migrate-js\" src=\"{includes_url}/js/jquery/jquery-migrate.min.js?ver={migrate}\"></script>\n"));
    let has_menu = menu::has_nav_menu(site, &ctx.mods, "top").await?;
    let mut l10n = vec![("quote", svg("quote-right", false))];
    if has_menu {
        l10n.push(("expand", t(ctx, "Expand child menu")));
        l10n.push(("collapse", t(ctx, "Collapse child menu")));
        l10n.push(("icon", svg("angle-down", true)));
    }
    let json = serde_json::Value::Object(l10n.into_iter().map(|(k, v)| (k.to_owned(), serde_json::Value::String(v))).collect());
    let json = rustpress_blocks::php_json::encode(&json).replace("\\/", "/").replace('<', "\\u003C").replace('>', "\\u003E");
    out.push_str(&format!("<script id=\"twentyseventeen-global-js-extra\">\nvar twentyseventeenScreenReaderText = {json};\n//# sourceURL=twentyseventeen-global-js-extra\n</script>\n"));
    out.push_str(&format!("<script data-wp-strategy=\"defer\" defer id=\"twentyseventeen-global-js\" src=\"{theme_uri}/assets/js/global.js?ver=20211130\"></script>\n"));
    if has_menu {
        out.push_str(&format!("<script data-wp-strategy=\"defer\" defer id=\"twentyseventeen-navigation-js\" src=\"{theme_uri}/assets/js/navigation.js?ver=20210122\"></script>\n"));
    }

    out.push_str(&discovery_links(site, q, &version).await?);
    // `twentyseventeen_pingback_header()`.
    if let (true, Queried::Post(post)) = (q.flags.is_singular(), &q.queried)
        && post.ping_status == "open"
    {
        out.push_str(&format!("<link rel=\"pingback\" href=\"{}\">\n", esc_url(&site.site_url("xmlrpc.php"))));
    }
    // `recent_comments_style()`: an active "Recent Comments" widget.
    if widgets::sidebars_widgets(site).iter().any(|(name, ids)| name != "wp_inactive_widgets" && ids.iter().any(|id| id.starts_with("recent-comments-"))) {
        out.push_str("<style>.recentcomments a{display:inline !important;padding:0 !important;margin:0 !important;}</style>");
    }
    out.push_str(&site_icon(ctx).await?);
    if let Some(css) = custom_css(ctx).await? {
        out.push_str(&format!("<style id=\"wp-custom-css\">\n{css}\n</style>\n"));
    }
    let supports = state.block_supports_css.compile();
    if !supports.is_empty() {
        out.push_str(&format!("<style id=\"core-block-supports-inline-css\">\n{supports}\n/*# sourceURL=core-block-supports-inline-css */\n</style>\n\n"));
    }
    Ok(out)
}

/// `wp_site_icon()`.
async fn site_icon(ctx: &Ctx<'_>) -> Result<String> {
    let id = ctx.site.options.u64("site_icon", 0);
    if id == 0 {
        return Ok(String::new());
    }
    let Some(post) = ctx.site.db.post(id).await? else { return Ok(String::new()) };
    let (meta, _) = images::load_metadata(ctx.site, &post).await?;
    let Some(meta) = meta else { return Ok(String::new()) };
    let Some(file) = meta.file() else { return Ok(String::new()) };
    let dir = file.rsplit_once('/').map_or("", |(d, _)| d);
    let base = images::upload_baseurl(ctx.site);
    let sized = |size: u64| -> Option<String> {
        let name = meta.sizes()?.values().find_map(|s| (s.get("width").and_then(Value2::as_u64) == Some(size) && s.get("height").and_then(Value2::as_u64) == Some(size)).then(|| s.get("file").and_then(Value2::as_str).map(str::to_owned)).flatten())?;
        Some(if dir.is_empty() { format!("{base}/{name}") } else { format!("{base}/{dir}/{name}") })
    };
    let mut out = String::new();
    for (rel, size) in [("icon", 32u64), ("icon", 192), ("apple-touch-icon", 180)] {
        if let Some(url) = sized(size) {
            if rel == "icon" {
                out.push_str(&format!("<link rel=\"icon\" href=\"{}\" sizes=\"{size}x{size}\" />\n", esc_url(&url)));
            } else {
                out.push_str(&format!("<link rel=\"apple-touch-icon\" href=\"{}\" />\n", esc_url(&url)));
            }
        }
    }
    if let Some(url) = sized(270) {
        out.push_str(&format!("<meta name=\"msapplication-TileImage\" content=\"{}\" />\n", esc_url(&url)));
    }
    Ok(out)
}

use serde_json::Value as Value2;

/// `wp_get_custom_css()`.
async fn custom_css(ctx: &Ctx<'_>) -> Result<Option<String>> {
    let Some(id) = ctx.mods.get("custom_css_post_id").and_then(Value::as_i64).filter(|id| *id > 0) else { return Ok(None) };
    let Some(post) = ctx.site.db.post(id as u64).await? else { return Ok(None) };
    if post.post_type != "custom_css" || post.post_status != "publish" {
        return Ok(None);
    }
    let css = post.post_content.trim().to_owned();
    Ok(if css.is_empty() { None } else { Some(css) })
}

// --------------------------------------------------------------------- footer

async fn wp_footer(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State) -> String {
    let site = ctx.site;
    let wp_root = &ctx.theme.wp_root;
    let version = wp_version(wp_root);
    let modules: Vec<String> = state.script_modules.iter().cloned().collect();
    let (importmap, preloads, module_scripts) = script_modules(wp_root, site, &modules);
    let mut out = format!("{importmap}{module_scripts}{preloads}");
    out.push_str(&format!("<script data-wp-strategy=\"defer\" defer id=\"jquery-scrollto-js\" src=\"{}/assets/js/jquery.scrollTo.js?ver=2.1.3\"></script>\n", ctx.theme.template_directory_uri()));
    // `comment-reply`, enqueued by `twentyseventeen_scripts()` for singular pages with open comments.
    if let (true, Queried::Post(post)) = (ctx.q.flags.is_singular(), &ctx.q.queried)
        && post.comment_status == "open"
        && site.options.get("thread_comments") == Some("1")
    {
        out.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{}/js/comment-reply.min.js?ver={version}\"></script>\n", site.site_url("wp-includes")));
    }
    out.push_str(&emoji_detection_script(site, wp_root, &version));
    out.push_str(&read(&ctx.theme.template_dir().join("assets/images/svg-icons.svg")).unwrap_or_default());
    out
}

// ----------------------------------------------------------------------- page

pub async fn render(ctx: &Ctx<'_>, registry: &Registry) -> Result<Document> {
    let site = ctx.site;
    let q = ctx.q;
    // Body classes: core's, then the theme's (`twentyseventeen_body_classes()`).
    let mut classes = body_classes(ctx.theme, site, q).await?;
    if is_multi_author(site).await? {
        classes.push("group-blog".into());
    }
    if !q.flags.is_singular() {
        classes.push("hfeed".into());
    }
    if q.flags.is_front_page && site.options.string("show_on_front", "posts") != "posts" {
        classes.push("twentyseventeen-front-page".into());
    }
    if has_header_image(ctx).await {
        classes.push("has-header-image".into());
    }
    if widgets::is_active_sidebar(site, "sidebar-1") && !q.flags.is_page {
        classes.push("has-sidebar".into());
    }
    if q.flags.is_page || q.flags.is_archive {
        classes.push(if ctx.mod_str("page_layout").as_deref() == Some("one-column") { "page-one-column".into() } else { "page-two-column".into() });
    }
    if ctx.mod_str("header_textcolor").as_deref() == Some("blank") {
        classes.push("title-tagline-hidden".into());
    }
    let scheme = match ctx.mod_str("colorscheme").as_deref() {
        Some("dark") => "dark",
        Some("custom") => "custom",
        _ => "light",
    };
    classes.push(format!("colors-{scheme}"));

    let mut body = header(ctx, &classes).await?;
    body.push_str(&main_template(ctx).await?);
    body.push_str(&footer(ctx).await?);
    let state = ctx.env.take_state();
    let head_html = head(ctx, &state, registry).await?;
    let footer_html = wp_footer(ctx, &state).await;
    let html = format!(
        "<!DOCTYPE html>\n<html {}lang=\"{}\" class=\"no-js no-svg\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<link rel=\"profile\" href=\"https://gmpg.org/xfn/11\">\n\n{head_html}</head>\n\n{body}{footer_html}\n</body>\n</html>\n",
        if site.is_rtl() { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
    );
    Ok(Document { status: q.status(), html })
}
