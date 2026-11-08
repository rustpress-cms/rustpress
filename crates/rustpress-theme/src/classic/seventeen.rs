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
    Document, EMOJI_CSS, INLINE_LIMIT, body_classes, discovery_links, document_title, emoji_detection_script, emoji_style, oembed_links, read, script_modules, wp_version,
};
use crate::global_styles::GlobalStyles;

const THEME: &str = "twentyseventeen";

fn sidebar(id: &'static str) -> Sidebar<'static> {
    Sidebar { id, before_widget: "<section id=\"%1$s\" class=\"widget %2$s\">", after_widget: "</section>", before_title: "<h2 class=\"widget-title\">", after_title: "</h2>" }
}

/// The sidebars `twentyseventeen_widgets_init()` registers.
pub fn registered_sidebars(site: &Site) -> Vec<widgets::RegisteredSidebar> {
    let t = |text: &str| site.i18n().gettext(THEME, text);
    [("sidebar-1", "Blog Sidebar", "Add widgets here to appear in your sidebar on blog posts and archive pages."), ("sidebar-2", "Footer 1", "Add widgets here to appear in your footer."), ("sidebar-3", "Footer 2", "Add widgets here to appear in your footer.")]
        .into_iter()
        .map(|(id, name, description)| {
            let wrap = sidebar(id);
            widgets::RegisteredSidebar {
                id: id.to_owned(),
                name: t(name),
                description: t(description),
                class: String::new(),
                before_widget: wrap.before_widget.to_owned(),
                after_widget: wrap.after_widget.to_owned(),
                before_title: wrap.before_title.to_owned(),
                after_title: wrap.after_title.to_owned(),
                show_in_rest: false,
            }
        })
        .collect()
}

/// `get_search_form()` for a widget rendered outside of a page of the theme.
pub fn widget_search_form(site: &Site, q: &rustpress_core::QueryResult, env: &rustpress_blocks::Env<'_>) -> String {
    search_form_markup(site, q, env)
}

/// `dynamic_sidebar()` for one of the theme's sidebars.
async fn sidebar_widgets(ctx: &Ctx<'_>, id: &'static str) -> Result<String> {
    let request_uri = ctx.request_uri();
    let search = || search_form(ctx);
    let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
    widgets::dynamic_sidebar(&services, &sidebar(id)).await
}

/// `twentyseventeen_get_svg()` and the rest of the social links: `twentyseventeen_social_links_icons()`.
const SOCIAL_ICONS: [(&str, &str); 41] = [
    ("behance.net", "behance"),
    ("codepen.io", "codepen"),
    ("deviantart.com", "deviantart"),
    ("digg.com", "digg"),
    ("docker.com", "dockerhub"),
    ("dribbble.com", "dribbble"),
    ("dropbox.com", "dropbox"),
    ("facebook.com", "facebook"),
    ("flickr.com", "flickr"),
    ("foursquare.com", "foursquare"),
    ("plus.google.com", "google-plus"),
    ("github.com", "github"),
    ("instagram.com", "instagram"),
    ("linkedin.com", "linkedin"),
    ("mailto:", "envelope-o"),
    ("medium.com", "medium"),
    ("pinterest.com", "pinterest-p"),
    ("pscp.tv", "periscope"),
    ("getpocket.com", "get-pocket"),
    ("reddit.com", "reddit-alien"),
    ("skype.com", "skype"),
    ("skype:", "skype"),
    ("slideshare.net", "slideshare"),
    ("snapchat.com", "snapchat-ghost"),
    ("soundcloud.com", "soundcloud"),
    ("spotify.com", "spotify"),
    ("stumbleupon.com", "stumbleupon"),
    ("t.me", "telegram"),
    ("telegram.me", "telegram"),
    ("tumblr.com", "tumblr"),
    ("twitch.tv", "twitch"),
    ("twitter.com", "twitter"),
    ("vimeo.com", "vimeo"),
    ("vine.co", "vine"),
    ("vk.com", "vk"),
    ("wa.me", "whatsapp"),
    ("whatsapp.com", "whatsapp"),
    ("wordpress.org", "wordpress"),
    ("wordpress.com", "wordpress"),
    ("yelp.com", "yelp"),
    ("youtube.com", "youtube"),
];

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

fn is_frontpage(ctx: &Ctx<'_>) -> bool {
    ctx.q.flags.is_front_page && !ctx.q.flags.is_home
}

async fn has_header_image(ctx: &Ctx<'_>) -> bool {
    header_image(ctx).await.0.is_some()
}

/// `get_header_image()`: the theme mod, else the theme's default image, in the scheme of the request
/// (`set_url_scheme()`; `is_ssl()` shows in the scheme of `siteurl`). For a random header also what
/// `_get_random_header_data()` picked, which `get_custom_header()` returns then.
async fn header_image(ctx: &Ctx<'_>) -> (Option<String>, Option<super::custom_header::RandomHeader>) {
    use super::custom_header as header;
    let uri = ctx.theme.template_directory_uri();
    let Some(support) = header::support(&ctx.theme.template, &uri) else { return (None, None) };
    let defaults = header::default_headers(ctx.site, &ctx.theme.template, &uri);
    let uploaded = if ctx.mod_str("header_image").as_deref() == Some("random-uploaded-image") {
        header::uploaded_header_images(ctx.site, &ctx.site.settings.stylesheet).await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let random = header::random_header(&ctx.mods, &support, &defaults, &uploaded).filter(|pick| header::is_random_header_image("any", &ctx.mods, &support, Some(pick)));
    let url = header::header_image(ctx.site, &ctx.mods, &support, random.as_ref()).filter(|url| !url.is_empty());
    (url, random)
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
    let (url, random) = header_image(ctx).await;
    // `is_header_video_active()`: the theme keeps the default callback, `is_front_page()`.
    if preview(ctx).is_some() && ctx.q.flags.is_front_page {
        ctx.env.with_state(|s| s.custom_header_script = true);
    }
    let Some(url) = url else {
        return Ok(if preview(ctx).is_some() { "<div id=\"wp-custom-header\" class=\"wp-custom-header\"></div>".to_owned() } else { String::new() });
    };
    let data = ctx.mods.get("header_image_data");
    let (mut width, mut height, mut attachment) = (
        data.and_then(|d| object_prop(d, "width")).and_then(Value::as_i64).unwrap_or(0).max(0) as u64,
        data.and_then(|d| object_prop(d, "height")).and_then(Value::as_i64).unwrap_or(0).max(0) as u64,
        data.and_then(|d| object_prop(d, "attachment_id")).and_then(Value::as_i64).unwrap_or(0).max(0) as u64,
    );
    if let Some(random) = &random {
        // `get_custom_header()` returns the random pick, with the size of the theme where it has none.
        let size = |value: &Option<Value>, default: u64| value.as_ref().and_then(Value::as_i64).map_or(default, |n| n.unsigned_abs());
        let support = super::custom_header::support(&ctx.theme.template, &ctx.theme.template_directory_uri());
        let (theme_width, theme_height) = support.map_or((0, 0), |s| (s.width, s.height));
        (width, height, attachment) = (size(&random.width, theme_width), size(&random.height, theme_height), random.attachment_id);
    }
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

/// `get_bloginfo( $show, 'display' )`: the filters of `bloginfo`.
fn bloginfo_display(ctx: &Ctx<'_>, value: &str) -> String {
    esc_html(&rustpress_core::formatting::convert_chars(&ctx.site.texturize(value)))
}

/// `the_custom_logo()`.
async fn custom_logo(ctx: &Ctx<'_>) -> Result<String> {
    let id = images::custom_logo_id(ctx.site).await?;
    if id == 0 {
        // A placeholder that the live preview can fill.
        return Ok(match preview(ctx) {
            Some(_) => format!("<a href=\"{}\" class=\"custom-logo-link\" style=\"display:none;\"><img class=\"custom-logo\" alt=\"\" /></a>", esc_url(&ctx.site.home_url("/"))),
            None => String::new(),
        });
    }
    let rows = ctx.site.db.post_meta(id).await?;
    let alt_meta = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    let mut attrs: Vec<(String, AttrValue)> = vec![("class".into(), AttrValue::Text("custom-logo".into())), ("loading".into(), AttrValue::Omit)];
    if alt_meta.is_empty() || alt_meta == "0" {
        attrs.push(("alt".into(), AttrValue::Text(bloginfo_display(ctx, &ctx.site.settings.blogname))));
    }
    let image = images::get_attachment_image(&ctx.env, id, "full", &ImageArgs { attrs, post_thumbnail: false, thumbnail_of: None, dimensions: None, requested: None }).await?;
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
    let name = &bloginfo_display(ctx, &site.settings.blogname);
    let title = if q.flags.is_front_page {
        format!("<h1 class=\"site-title\"><a href=\"{home}\" rel=\"home\" {current}>{name}</a></h1>")
    } else {
        format!("<p class=\"site-title\"><a href=\"{home}\" rel=\"home\" {current}>{name}</a></p>")
    };
    let description = &bloginfo_display(ctx, &site.settings.blogdescription);
    let description = if description.is_empty() && preview(ctx).is_none() { String::new() } else { format!("<p class=\"site-description\">{description}</p>") };
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
    args.customize = ctx.q.customize.clone();
    let title_hook = |title: &str, item: &MenuItem, _depth: usize| {
        // `twentyseventeen_dropdown_icon_to_menu_link()`.
        if item.classes.iter().any(|c| c == "menu-item-has-children" || c == "page_item_has_children") { format!("{title}{}", svg("angle-down", false)) } else { title.to_owned() }
    };
    let link_hook = |link: String, _item: &MenuItem, _depth: usize| link;
    args.use_ids_once(&ctx.env, &menu.items);
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

async fn header(ctx: &Ctx<'_>, classes: &[String], bar: &str) -> Result<String> {
    ctx.env.with_state(|s| s.media.in_header = true);
    let top = top_menu(ctx).await?;
    let has_top_menu = top.is_some();
    let mut out = format!(
        "<body class=\"{}\">\n{bar}<div id=\"page\" class=\"site\">\n\t<a class=\"skip-link screen-reader-text\" href=\"#content\">\n\t\t{}\t</a>\n\n\t<header id=\"masthead\" class=\"site-header\">\n\n\t\t",
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
        let image = images::get_attachment_image(&ctx.env, images::post_thumbnail_id(&ctx.env, q.queried_object_id()).await?, "twentyseventeen-featured-image", &ImageArgs { post_thumbnail: true, thumbnail_of: Some(q.queried_object_id()), ..ImageArgs::default() }).await?;
        out.push_str(&format!("<div class=\"single-featured-image-header\">{image}</div><!-- .single-featured-image-header -->"));
    }
    ctx.env.with_state(|s| s.media.in_header = false);
    out.push_str("\n\t<div class=\"site-content-contain\">\n\t\t<div id=\"content\" class=\"site-content\">\n");
    Ok(out)
}

// ---------------------------------------------------------------------- footer

async fn footer(ctx: &Ctx<'_>) -> Result<String> {
    let mut widgets_html = String::from("\n");
    // `is_active_sidebar( 'sidebar-2' ) || is_active_sidebar( 'sidebar-3' )`, then each again.
    let two = active_sidebar(ctx, "sidebar-2");
    let three = active_sidebar(ctx, "sidebar-3");
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
    let social = social_navigation(ctx).await?;
    let powered = rustpress_core::i18n::sprintf(&t(ctx, "Proudly powered by %s"), &["RustPress"]);
    Ok(format!(
        "\n\t\t</div><!-- #content -->\n\n\t\t<footer id=\"colophon\" class=\"site-footer\">\n\t\t\t<div class=\"wrap\">\n\t\t\t\t{widgets_html}{social}<div class=\"site-info\">\n\t{privacy}\t<a href=\"{imprint_url}\" class=\"imprint\">\n\t\t{powered}\t</a>\n</div><!-- .site-info -->\n\t\t\t</div><!-- .wrap -->\n\t\t</footer><!-- #colophon -->\n\t</div><!-- .site-content-contain -->\n</div><!-- #page -->\n"
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

/// `twentyseventeen_time_link()`. While a plugin makes `get_the_date()`, `get_the_time()` and
/// `get_the_modified_date()` return `false`, the dates are empty and `get_the_time( 'U' )` never equals the
/// (unfiltered) modification time.
async fn time_link(ctx: &Ctx<'_>, post: &Post, permalink: &str) -> Result<String> {
    let site = ctx.site;
    if site.plugins.hides_date(site, post).await? {
        let time_string = "<time class=\"entry-date published\" datetime=\"\"></time><time class=\"updated\" datetime=\"\"></time>";
        return Ok(rustpress_core::i18n::sprintf(&t(ctx, "<span class=\"screen-reader-text\">Posted on</span> %s"), &[&format!("<a href=\"{}\" rel=\"bookmark\">{time_string}</a>", esc_url(permalink))]));
    }
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
    Ok(rustpress_core::i18n::sprintf(&t(ctx, "<span class=\"screen-reader-text\">Posted on</span> %s"), &[&format!("<a href=\"{}\" rel=\"bookmark\">{time_string}</a>", esc_url(permalink))]))
}

/// `twentyseventeen_posted_on()`.
async fn posted_on(ctx: &Ctx<'_>, post: &Post, permalink: &str) -> Result<String> {
    let author = ctx.site.db.user(post.post_author).await?;
    let (name, url) = match &author {
        Some(user) => (user.display_name.clone(), ctx.site.author_url(user)),
        None => (String::new(), String::new()),
    };
    let byline = rustpress_core::i18n::sprintf(&t(ctx, "by %s"), &[&format!("<span class=\"author vcard\"><a class=\"url fn n\" href=\"{}\">{}</a></span>", esc_url(&url), name)]);
    Ok(format!("<span class=\"posted-on\">{}</span><span class=\"byline\"> {byline}</span>", time_link(ctx, post, permalink).await?))
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
    let edit = edit_link(ctx, post);
    if !((categorized && !categories.is_empty()) || !tag_list.is_empty() || !edit.is_empty()) {
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
    out.push_str(&edit);
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
/// `get_query_var( 'page' )` as `WP_Query::generate_postdata()` takes it.
fn current_page(ctx: &Ctx<'_>) -> u64 {
    ctx.q.vars.u64("page").filter(|page| *page != 0).unwrap_or(1)
}

/// `get_post_format()`: the template parts exist for these four, the others use `content.php`.
async fn post_format(ctx: &Ctx<'_>, post: &Post) -> Result<Option<String>> {
    let terms = ctx.site.db.terms_for_post(post.id, Some("post_format")).await?;
    Ok(terms.first().and_then(|t| t.slug.strip_prefix("post-format-")).filter(|f| matches!(*f, "audio" | "video" | "image" | "gallery")).map(str::to_owned))
}

/// `get_edit_post_link()` through `twentyseventeen_edit_link()`: empty for who may not edit the post.
fn edit_link(ctx: &Ctx<'_>, post: &Post) -> String {
    if preview(ctx).is_some() || !ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.can_edit_post(post)) {
        return String::new();
    }
    let text = rustpress_core::i18n::sprintf(&t(ctx, "Edit<span class=\"screen-reader-text\"> \"%s\"</span>"), &[&the_title_for(ctx.site, post)]);
    format!("<span class=\"edit-link\"><a class=\"post-edit-link\" href=\"{}\">{text}</a></span>", esc_url(&ctx.site.site_url(&format!("wp-admin/post.php?post={}&action=edit", post.id))))
}

/// `wp_link_pages()` with the arguments of the theme. The templates of posts wrap the numbers in a span.
async fn link_pages(ctx: &Ctx<'_>, post: &Post, number_span: bool) -> Result<String> {
    let pages = rustpress_blocks::content::post_pages(&post.post_content);
    if pages.len() < 2 {
        return Ok(String::new());
    }
    let site = ctx.site;
    let q = ctx.q;
    let page = current_page(ctx).min(pages.len() as u64);
    let more = (q.queried_object_id() == post.id && (q.flags.is_page || q.flags.is_single)) || q.flags.is_feed || page > 1;
    let permalink = site.permalink(post).await?;
    let (before, after) = if number_span { ("<span class=\"page-number\">", "</span>") } else { ("", "") };
    let mut out = format!("<div class=\"page-links\">{}", t(ctx, "Pages:"));
    for number in 1..=pages.len() as u64 {
        let label = format!("{before}{number}{after}");
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
    out.push_str("</div>");
    Ok(out)
}

/// `get_media_embedded_in_content()`.
fn embedded_media(content: &str, types: &[&str]) -> Vec<String> {
    let alternatives: Vec<String> = types.iter().map(|tag| format!(r"<{tag}[^<]*?(?:>[\s\S]*?</{tag}>|\s*/>)")).collect();
    let Ok(pattern) = regex::Regex::new(&alternatives.join("|")) else { return Vec::new() };
    pattern.find_iter(content).map(|m| m.as_str().to_owned()).collect()
}

/// `get_post_gallery()`: all galleries are built each time, the first one is used.
async fn post_gallery(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<Option<String>> {
    Ok(rustpress_blocks::shortcodes::post_galleries(&ctx.env, post).await?.into_iter().next().filter(|gallery| !gallery.is_empty()))
}

/// `template-parts/post/content.php` and the templates of the post formats `audio`, `video`, `image` and `gallery`.
async fn content_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let site = ctx.site;
    let q = ctx.q;
    let permalink = site.permalink(post).await?;
    let is_single = q.flags.is_single;
    let is_home_unpaged = q.flags.is_home && !q.flags.is_paged;
    let format = post_format(ctx, post).await?;
    let format = format.as_deref();
    let classes = post_class(&ctx.env, post, &[], is_home_unpaged, q.flags.is_singular()).await?;
    let sticky = if site.settings.sticky_posts.contains(&post.id) && q.flags.is_home && is_home_unpaged { svg("thumb-tack", false) } else { String::new() };
    let mut meta = String::new();
    if post.post_type == "post" {
        meta.push_str("<div class=\"entry-meta\">");
        if is_single {
            meta.push_str(&posted_on(ctx, post, &permalink).await?);
        } else {
            meta.push_str(&time_link(ctx, post, &permalink).await?);
            meta.push_str(&edit_link(ctx, post));
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

    let raw = get_the_content(&ctx.env, post, current_page(ctx), is_single).await?;
    // The audio and video templates look into the rendered content first.
    let mut media = Vec::new();
    if let Some(kind @ ("audio" | "video")) = format {
        let rendered = the_content(&ctx.env, post, &raw).await?;
        if !rendered.contains("wp-playlist-script") {
            media = embedded_media(&rendered, if kind == "audio" { &["audio"] } else { &["video", "object", "embed", "iframe"] });
        }
    }

    // `get_the_post_thumbnail()` runs in the conditions as well, and each call counts as media.
    let thumbnail_id = images::post_thumbnail_id(&ctx.env, post.id).await?;
    let args = ImageArgs { post_thumbnail: true, thumbnail_of: Some(post.id), ..ImageArgs::default() };
    let has_thumbnail = thumbnail_id != 0 && !images::get_attachment_image(&ctx.env, thumbnail_id, "twentyseventeen-featured-image", &args).await?.is_empty();
    let mut thumbnail = String::new();
    let shows_thumbnail = has_thumbnail
        && !is_single
        && match format {
            Some("video") => media.is_empty(),
            Some("gallery") => post_gallery(ctx, post).await?.is_none(),
            _ => true,
        };
    if shows_thumbnail {
        let image = images::get_attachment_image(&ctx.env, thumbnail_id, "twentyseventeen-featured-image", &args).await?;
        thumbnail = format!("\t\t<div class=\"post-thumbnail\">\n\t\t\t<a href=\"{}\">\n\t\t\t\t{image}\t\t\t</a>\n\t\t</div><!-- .post-thumbnail -->\n\t", esc_url(&permalink));
    }

    let mut body = String::new();
    let shows_content = match format {
        Some(kind @ ("audio" | "video")) => {
            if !is_single {
                for html in &media {
                    body.push_str(&if kind == "audio" { format!("<div class=\"entry-audio\">{html}</div><!-- .entry-audio -->") } else { format!("<div class=\"entry-video\">{html}</div>") });
                }
            }
            is_single || media.is_empty()
        }
        Some("image") => is_single || thumbnail_id == 0 || images::get_attachment_image(&ctx.env, thumbnail_id, "twentyseventeen-featured-image", &args).await?.is_empty(),
        Some("gallery") => {
            if !is_single && post_gallery(ctx, post).await?.is_some() {
                let gallery = post_gallery(ctx, post).await?.unwrap_or_default();
                body.push_str(&format!("<div class=\"entry-gallery\">{gallery}</div>"));
            }
            is_single || post_gallery(ctx, post).await?.is_none()
        }
        _ => true,
    };
    if shows_content {
        body.push_str(&the_content(&ctx.env, post, &raw).await?);
        body.push_str(&link_pages(ctx, post, true).await?);
    }
    let footer = if is_single { entry_footer(ctx, post).await? } else { String::new() };
    // The templates of the formats have a block of PHP or an empty line more.
    let before_thumbnail = if matches!(format, Some("audio" | "video")) { "\n\t" } else { "" };
    let before_body = if format.is_some() { "\n" } else { "" };
    let after_body = before_body;
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\t{sticky}\t<header class=\"entry-header\">\n\t\t{meta}{heading}\t</header><!-- .entry-header -->\n\n\t{before_thumbnail}{thumbnail}\n\t<div class=\"entry-content\">\n{before_body}\t\t{body}{after_body}\t</div><!-- .entry-content -->\n\n\t{footer}\n</article><!-- #post-{id} -->\n",
        id = post.id,
        classes = esc_attr(&classes.join(" ")),
    ))
}

/// `is_front_page()` with a page on the front: `front-page.php`. On the blog `twentyseventeen_front_page_template()`
/// goes back to `index.php`.
fn is_static_front_page(ctx: &Ctx<'_>) -> bool {
    ctx.q.flags.is_front_page && ctx.q.flags.is_page
}

/// The image of a panel, a percentage for its height.
async fn panel_image(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let id = images::post_thumbnail_id(&ctx.env, post.id).await?;
    let Some(attachment) = (id != 0).then_some(ctx.site.db.post(id).await?).flatten() else { return Ok(String::new()) };
    let (meta, attached) = images::load_metadata(ctx.site, &attachment).await?;
    let (Some(meta), Some(attached)) = (meta, attached) else { return Ok(String::new()) };
    let image = images::image_downsize(&images::attachment_url(ctx.site, &attached), &meta, "twentyseventeen-featured-image");
    // PHP prints a float with 14 digits.
    let ratio = rustpress_blocks::general::php_float(image.height as f64 / image.width as f64 * 100.0);
    Ok(format!(
        "\n\t\t<div class=\"panel-image\" style=\"background-image: url({});\">\n\t\t\t<div class=\"panel-image-prop\" style=\"padding-top: {}%\"></div>\n\t\t</div><!-- .panel-image -->\n\n\t",
        esc_url(&image.url),
        esc_attr(&ratio)
    ))
}

/// What `content-front-page.php` and `content-front-page-panels.php` share. `recent` is the list of the blog panel,
/// whose `wp_reset_postdata()` leaves the post of the page for `the_ID()` in the last line: `closing_id`.
async fn panel_article(ctx: &Ctx<'_>, post: &Arc<Post>, anchor: &str, recent: &str, closing_id: u64, lead: &str, with_links: bool) -> Result<String> {
    let classes = post_class(&ctx.env, post, &["twentyseventeen-panel", ""], false, true).await?;
    let image = panel_image(ctx, post).await?;
    let title = the_title_for(ctx.site, post);
    let heading = if title.is_empty() { String::new() } else { format!("<h2 class=\"entry-title\">{title}</h2>") };
    let edit = edit_link(ctx, post);
    let raw = get_the_content(&ctx.env, post, current_page(ctx), true).await?;
    let content = the_content(&ctx.env, post, &raw).await?;
    let links = if with_links { link_pages(ctx, post, false).await? } else { String::new() };
    Ok(format!(
        "{lead}<article id=\"{anchor}\" class=\"{classes}\" >\n\n\t{image}\n\t<div class=\"panel-content\">\n\t\t<div class=\"wrap\">\n\t\t\t<header class=\"entry-header\">\n\t\t\t\t{heading}\n\t\t\t\t{edit}\n\t\t\t</header><!-- .entry-header -->\n\n\t\t\t<div class=\"entry-content\">\n\t\t\t\t{content}{links}\t\t\t</div><!-- .entry-content -->\n\n{recent}\t\t</div><!-- .wrap -->\n\t</div><!-- .panel-content -->\n\n</article><!-- #post-{closing_id} -->\n",
        classes = esc_attr(&classes.join(" ")),
    ))
}

/// `template-parts/page/content-front-page.php`.
async fn front_page_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    panel_article(ctx, post, &format!("post-{}", post.id), "", post.id, "", true).await
}

/// The panels of `front-page.php`: `twentyseventeen_front_page_section()` for the sections that have a page.
async fn front_page_panels(ctx: &Ctx<'_>) -> Result<String> {
    let mut out = String::new();
    for number in 1..=4 {
        out.push_str(&front_page_section(ctx, number).await?);
    }
    Ok(out)
}

/// `twentyseventeen_front_page_section()`: a panel with its page, or in the Customizer a placeholder for an empty one.
async fn front_page_section(ctx: &Ctx<'_>, number: u32) -> Result<String> {
    let placeholder = || match preview(ctx) {
        Some(_) => format!(
            "<article class=\"panel-placeholder panel twentyseventeen-panel twentyseventeen-panel{number}\" id=\"panel{number}\"><span class=\"twentyseventeen-panel-title\">{}</span></article>",
            rustpress_core::i18n::sprintf(&t(ctx, "Front Page Section %s Placeholder"), &[&number.to_string()])
        ),
        None => String::new(),
    };
    let Some(id) = ctx.mod_str(&format!("panel_{number}")).and_then(|id| id.parse::<u64>().ok()).filter(|id| *id != 0) else { return Ok(placeholder()) };
    let post = ctx.site.db.post(id).await?;
    let Some(post) = post.filter(|post| matches!(post.post_status.as_str(), "publish" | "private")) else { return Ok(placeholder()) };
    let post = Arc::new(post);
    ctx.env.with_state(|s| s.looped_posts.push(post.id));
    let mut recent = String::from("\t\t\t\n");
    let mut closing_id = post.id;
    if ctx.site.settings.page_for_posts == post.id {
        let query = rustpress_db::PostQuery { limit: Some(3), ..rustpress_db::PostQuery::default() };
        let posts = ctx.site.db.posts(&query).await?;
        recent = String::from("\t\t\t\n\t\t\t\t\n\t\t\t\t");
        if !posts.is_empty() {
            recent.push_str("\n\t\t\t\t\t<div class=\"recent-posts\">\n\n\t\t\t\t\t\t");
            for item in posts {
                ctx.env.with_state(|s| s.looped_posts.push(item.id));
                recent.push_str(&excerpt_part(ctx, &Arc::new(item)).await?);
            }
            recent.push_str("\t\t\t\t\t</div><!-- .recent-posts -->\n\t\t\t\t");
            closing_id = ctx.q.queried_object_id();
        }
        recent.push_str("\t\t\t\n");
    }
    panel_article(ctx, &post, &format!("panel{number}"), &recent, closing_id, "\n", false).await
}

/// The social links menu of `footer.php`.
async fn social_navigation(ctx: &Ctx<'_>) -> Result<String> {
    let Some(id) = menu::menu_location(&ctx.mods, "social") else { return Ok(String::new()) };
    let Some(mut social) = menu::load_menu(ctx.site, id).await? else { return Ok(String::new()) };
    let uri = ctx.request_uri();
    menu::classes_by_context(ctx.site, ctx.q, &uri, &mut social.items).await?;
    let mut args = NavArgs::new("social");
    args.menu_class = "social-links-menu".into();
    args.customize = ctx.q.customize.clone();
    args.depth = 1;
    args.link_before = "<span class=\"screen-reader-text\">".into();
    args.link_after = format!("</span>{}", svg("chain", false));
    let link_after = args.link_after.clone();
    let title_hook = |title: &str, _: &MenuItem, _: usize| title.to_owned();
    // `twentyseventeen_nav_menu_social_icons()`.
    let link_hook = |link: String, _: &MenuItem, _: usize| match SOCIAL_ICONS.iter().find(|(address, _)| link.contains(address)) {
        Some((_, icon)) => link.replace(&link_after, &format!("</span>{}", svg(icon, false))),
        None => link,
    };
    args.use_ids_once(&ctx.env, &social.items);
    let html = menu::wp_nav_menu(ctx.site, &social, &social.items, &args, &title_hook, &link_hook);
    Ok(format!("\t\t\t\t\t<nav class=\"social-navigation\" aria-label=\"{}\">\n\t\t\t\t\t\t{html}\t\t\t\t\t</nav><!-- .social-navigation -->\n\t\t\t\t\t", esc_attr(&t(ctx, "Footer Social Links Menu"))))
}

/// `twentyseventeen_content_width()`.
fn content_width(ctx: &Ctx<'_>) -> u64 {
    let mut width = 525;
    if ctx.mod_str("page_layout").as_deref() == Some("one-column") {
        if is_frontpage(ctx) {
            width = 644;
        } else if ctx.q.flags.is_page {
            width = 740;
        }
    }
    if ctx.q.flags.is_single && !active_sidebar(ctx, "sidebar-1") {
        width = 740;
    }
    width
}

/// `twentyseventeen_colors_css_wrap()`.
fn custom_colors_css(ctx: &Ctx<'_>) -> String {
    if ctx.mod_str("colorscheme").as_deref() != Some("custom") && preview(ctx).is_none() {
        return String::new();
    }
    let hue = ctx.mod_str("colorscheme_hue").and_then(|hue| hue.trim().parse::<i64>().ok()).map_or(250, i64::abs).to_string();
    let css = include_str!("seventeen_colors.css").replace("{$hue}", &hue).replace("{$saturation}", "50%").replace("{$reduced_saturation}", "40%");
    let data_hue = if preview(ctx).is_some() { format!("data-hue=\"{hue}\"") } else { String::new() };
    format!("\t<style type=\"text/css\" id=\"custom-theme-colors\" {data_hue}>\n\t\t{css}\t</style>\n\t")
}

/// `twentyseventeen_header_style()`.
fn header_style(ctx: &Ctx<'_>) -> String {
    let Some(color) = ctx.mod_str("header_textcolor").filter(|color| !color.is_empty() && color != "ffffff") else { return String::new() };
    let rules = if color == "blank" {
        "\t\t.site-title,\n\t\t.site-description {\n\t\t\tposition: absolute;\n\t\t\tclip-path: inset(50%);\n\t\t}\n\t\t\t".to_owned()
    } else {
        let selectors = [
            ".site-title a",
            ".colors-dark .site-title a",
            ".colors-custom .site-title a",
            "body.has-header-image .site-title a",
            "body.has-header-video .site-title a",
            "body.has-header-image.colors-dark .site-title a",
            "body.has-header-video.colors-dark .site-title a",
            "body.has-header-image.colors-custom .site-title a",
            "body.has-header-video.colors-custom .site-title a",
            ".site-description",
            ".colors-dark .site-description",
            ".colors-custom .site-description",
            "body.has-header-image .site-description",
            "body.has-header-video .site-description",
            "body.has-header-image.colors-dark .site-description",
            "body.has-header-video.colors-dark .site-description",
            "body.has-header-image.colors-custom .site-description",
            "body.has-header-video.colors-custom .site-description",
        ];
        format!("\t\t{} {{\n\t\t\tcolor: #{};\n\t\t}}\n\t", selectors.join(",\n\t\t"), esc_attr(&color))
    };
    format!("\t\t<style id=\"twentyseventeen-custom-header-styles\" type=\"text/css\">\n\t\t{rules}\t</style>\n\t\t")
}

async fn pagination(ctx: &Ctx<'_>) -> String {
    tags::posts_pagination(
        ctx.site,
        ctx.q,
        &Pagination {
            prev_text: format!("{}<span class=\"screen-reader-text\">{}</span>", svg("arrow-left", false), t(ctx, "Previous page")),
            next_text: format!("<span class=\"screen-reader-text\">{}</span>{}", t(ctx, "Next page"), svg("arrow-right", false)),
            before_page_number: format!("<span class=\"meta-nav screen-reader-text\">{} </span>", t(ctx, "Page")),
            mid_size: 1,
        },
    )
}

async fn get_sidebar(ctx: &Ctx<'_>) -> Result<String> {
    if !active_sidebar(ctx, "sidebar-1") {
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
    search_form_markup(ctx.site, ctx.q, &ctx.env)
}

fn search_form_markup(site: &Site, q: &rustpress_core::QueryResult, env: &rustpress_blocks::Env<'_>) -> String {
    let tx = |text: &str, context: &str| site.i18n().pgettext(THEME, context, text);
    let unique = env.with_state(|s| s.unique_id("search-form-"));
    let query = esc_attr(q.vars.get("s").unwrap_or(""));
    format!(
        "\n\n<form role=\"search\" method=\"get\" class=\"search-form\" action=\"{}\">\n\t<label for=\"{unique}\">\n\t\t<span class=\"screen-reader-text\">\n\t\t\t{}\t\t</span>\n\t</label>\n\t<input type=\"search\" id=\"{unique}\" class=\"search-field\" placeholder=\"{}\" value=\"{query}\" name=\"s\" />\n\t<button type=\"submit\" class=\"search-submit\">{}<span class=\"screen-reader-text\">\n\t\t{}\t</span></button>\n</form>\n",
        esc_url(&site.home_url("/")),
        tx("Search for:", "label"),
        esc_attr(&tx("Search &hellip;", "placeholder")),
        svg("search", false),
        tx("Search", "submit button"),
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
            let prefix = if f.is_category {
                site.trx("Category:", "category archive title prefix")
            } else if f.is_tag {
                site.trx("Tag:", "tag archive title prefix")
            } else {
                rustpress_core::i18n::sprintf(&site.trx("%s:", "taxonomy term archive title prefix"), &[&site.taxonomy_singular_label(&term.taxonomy)])
            };
            (format!("{prefix} <span>{}</span>", term.name), term.description.clone())
        }
        Queried::User(user) if f.is_author => (format!("{} <span>{}</span>", site.trx("Author:", "author archive title prefix"), user.display_name), String::new()),
        Queried::PostType(post_type) if f.is_post_type_archive && !(f.is_year || f.is_month || f.is_day) => {
            (format!("{} <span>{}</span>", site.trx("Archives:", "post type archive title prefix"), site.post_type_label(post_type)), String::new())
        }
        _ if f.is_year || f.is_month || f.is_day => {
            let first = q.posts.first();
            let ts = first.and_then(|p| site.timezone.local_timestamp(&p.post_date)).unwrap_or(0);
            // `get_the_date()` of the global post: empty when a plugin ran `the_post` for it in `wp_head` and hides its
            // date from then on.
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
    let meta = if post.post_type == "post" { format!("\t\t\t<div class=\"entry-meta\">\n\t\t\t\t{}\t\t\t</div><!-- .entry-meta -->\n\t\t", time_link(ctx, post, &permalink).await?) } else { String::new() };
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
    let raw = get_the_content(&ctx.env, post, current_page(ctx), true).await?;
    let content = the_content(&ctx.env, post, &raw).await?;
    let links = link_pages(ctx, post, false).await?;
    let edit = edit_link(ctx, post);
    Ok(format!(
        "\n<article id=\"post-{id}\" class=\"{classes}\">\n\t<header class=\"entry-header\">\n\t\t<h1 class=\"entry-title\">{title}</h1>\t\t{edit}\t</header><!-- .entry-header -->\n\t<div class=\"entry-content\">\n\t\t{content}{links}\t</div><!-- .entry-content -->\n</article><!-- #post-{id} -->\n",
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
        ctx.env.with_state(|s| s.looped_posts.push(post.id));
        loop_html.push_str(&if is_static_front_page(ctx) {
            front_page_part(ctx, &post).await?
        } else if f.is_page {
            page_part(ctx, &post).await?
        } else if f.is_search {
            excerpt_part(ctx, &post).await?
        } else {
            content_part(ctx, &post).await?
        });
        if (f.is_single || f.is_page) && !is_static_front_page(ctx) && (comments::comments_open(&post) || post.comment_count > 0) {
            let args = comments::ListArgs { avatar_size: 100, reply_text: format!("{}{}", svg("mail-reply", false), t(ctx, "Reply")), short_ping: true, cpage: None };
            let text = |s: &str| t(ctx, s);
            let text_x = |s: &str, c: &str| tx(ctx, s, c);
            let pagination = comments::PaginationText {
                prev_text: format!("{}<span class=\"screen-reader-text\">{}</span>", svg("arrow-left", false), t(ctx, "Previous")),
                next_text: format!("<span class=\"screen-reader-text\">{}</span>{}", t(ctx, "Next"), svg("arrow-right", false)),
            };
            loop_html.push_str(&comments::comments_template(ctx.site, &ctx.env, &post, &args, &ctx.request_uri(), &pagination, &text, &text_x).await?);
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
    if is_static_front_page(ctx) {
        let panels = front_page_panels(ctx).await?;
        return Ok(format!("\n<div id=\"primary\" class=\"content-area\">\n\t<main id=\"main\" class=\"site-main\">\n\n\t\t{loop_html}\n\t\t{panels}\n\t</main><!-- #main -->\n</div><!-- #primary -->\n\n"));
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

    let crate::head::PluginHead { head: plugin_head, blocks: early_blocks } = crate::head::plugin_head(theme, site, q, registry, true, None).await?;
    out.push_str("<script>(function(html){html.className = html.className.replace(/\\bno-js\\b/,'js')})(document.documentElement);</script>\n");
    // Plugins hook their `wp_head` actions of priority 0 on `init`, after the theme's `twentyseventeen_javascript_detection()`.
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
    let plugin_assets = site.plugins.assets(site);
    out.push_str(&rustpress_core::native::resource_hints(site, &plugin_assets));
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
        // The common stylesheet stays the same in every direction here.
        sheets.push(Sheet { id: "wp-block-library".into(), path: common, url: String::new(), source: "/wp-includes/css/dist/block-library/common.min.css".into(), rtl: false });
    }
    // Blocks a plugin rendered in `wp_head` (after `wp_maybe_inline_styles()` ran there) have ordinary enqueued
    // stylesheets, printed as links after the theme's; the others load on demand.
    let mut early: Vec<String> = Vec::new();
    for name in &early_blocks {
        let Some(block_type) = registry.get(name) else { continue };
        for handle in &block_type.styles {
            if !early.contains(handle) {
                early.push(handle.clone());
            }
        }
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
            // The comment keeps naming the left-to-right file.
            let source = url.split('?').next().unwrap_or(&url).to_owned();
            let variant = crate::document::rtl_variant(rtl, &path, &url);
            let is_rtl = variant.is_some();
            let (path, url) = variant.unwrap_or((path, url));
            sheets.push(Sheet { id: handle.clone(), path, source, url, rtl: is_rtl });
        }
    }
    // Smallest files first until the inline budget is used up.
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
            // The core adds an (empty) inline style to the common block stylesheet.
            let extra = if sheet.id == "wp-block-library" { "\n" } else { "" };
            out.push_str(&format!("<style id=\"{}-inline-css\">\n{}\n{extra}/*# sourceURL={} */\n</style>\n", sheet.id, css, sheet.source));
        } else {
            out.push_str(&format!("<link rel='stylesheet' id='{}{}-css' href='{}' media='all' />\n", sheet.id, if sheet.rtl { "-rtl" } else { "" }, sheet.url));
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

    // The theme's own styles and scripts (`twentyseventeen_scripts()`).
    let fonts_on = tx(ctx, "on", "Libre Franklin font: on or off") != "off";
    out.push('\n');
    out.push_str(&crate::plugin_assets::StylePrinter::new(wp_root, site).links(&plugin_assets.styles));
    if fonts_on {
        out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-fonts-css' href='{theme_uri}/assets/fonts/font-libre-franklin.css?ver=20230328' media='all' />\n"));
    }
    out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-style-css' href='{theme_uri}/style.css?ver=20250415' media='all' />\n"));
    out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-block-style-css' href='{theme_uri}/assets/css/blocks.css?ver=20240729' media='all' />\n"));
    if ctx.mod_str("colorscheme").as_deref() == Some("dark") || preview.is_some() {
        out.push_str(&format!("<link rel='stylesheet' id='twentyseventeen-colors-dark-css' href='{theme_uri}/assets/css/colors-dark.css?ver=20240412' media='all' />\n"));
    }
    // The media widgets and the Text widget enqueue the players for widgets the preview may add.
    if preview.is_some() {
        out.push_str(&crate::customize_preview::media_styles(&includes_url, &version));
    }
    for handle in &early {
        let Some(slug) = handle.strip_prefix("wp-block-") else { continue };
        if rustpress_core::files::is_file(&registry.blocks_dir().join(slug).join("style.min.css")) {
            out.push_str(&format!("<link rel='stylesheet' id='{handle}-css' href='{includes_url}/blocks/{slug}/style.min.css?ver={version}' media='all' />\n"));
        }
    }
    // The plugins' scripts are queued first; the theme's global script needs jQuery.
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&includes_url, &script_version(wp_root, "jquery-core", "3.7.1"), &script_version(wp_root, "jquery-migrate", "3.4.1")).with_packages(wp_root, site);
    // `customize-preview`, queued first, needs jQuery (through `customize-base`).
    if preview.is_some() {
        out.push_str(&printer.core("jquery"));
    }
    // Scripts of a later `wp_enqueue_scripts` callback than the theme's are queued after its own.
    let (early_scripts, late_scripts): (Vec<_>, Vec<_>) = plugin_assets.scripts.iter().cloned().partition(|a| a.priority <= 10);
    out.push_str(&printer.scripts(&early_scripts, false));
    out.push_str(&printer.core("jquery"));
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
    out.push_str(&printer.scripts(&late_scripts, false));

    out.push_str(&discovery_links(site, q, &version, 0, !plugin_head.removed.contains(&"rel_canonical"), &crate::document::locale_stylesheet(theme, site)).await?);
    out.push_str(&plugin_head.late);
    // `twentyseventeen_pingback_header()`.
    if let (true, Queried::Post(post)) = (q.flags.is_singular(), &q.queried)
        && post.ping_status == "open"
    {
        out.push_str(&format!("<link rel=\"pingback\" href=\"{}\">\n", esc_url(&site.site_url("xmlrpc.php"))));
    }
    out.push_str(&custom_colors_css(ctx));
    // What plugins hook on `wp_head` on `init` (before the widgets register).
    out.push_str(&plugin_head.late_init);
    // `recent_comments_style()`: an active "Recent Comments" widget.
    if preview.is_some() || widgets::sidebars_widgets(site).iter().any(|(name, ids)| name != "wp_inactive_widgets" && ids.iter().any(|id| id.starts_with("recent-comments-"))) {
        out.push_str("<style>.recentcomments a{display:inline !important;padding:0 !important;margin:0 !important;}</style>");
    }
    out.push_str(&header_style(ctx));
    // The Customizer adds its own on `wp_loaded`.
    if let Some(preview) = preview {
        out.push_str(&crate::customize_preview::loading_style(preview));
    }
    let icon = super::wp_site_icon(ctx.site).await?;
    // The default icon in the preview, so that the live preview can replace it.
    out.push_str(if icon.is_empty() && preview.is_some() { "<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"32x32\" />\n" } else { &icon });
    match custom_css(ctx).await? {
        Some(css) => out.push_str(&format!("<style id=\"wp-custom-css\">\n{css}\n</style>\n")),
        None if preview.is_some() => out.push_str("<style id=\"wp-custom-css\">\n\n</style>\n"),
        None => {}
    }
    // `wp_hoist_late_printed_styles()`: what the content enqueued goes in front of `</head>`, with a line break more.
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

async fn wp_footer(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State) -> Result<String> {
    let site = ctx.site;
    let wp_root = &ctx.theme.wp_root;
    let version = wp_version(wp_root);
    let modules: Vec<String> = state.script_modules.iter().cloned().collect();
    let (importmap, preloads, module_scripts) = script_modules(wp_root, site, &modules);
    // A plugin that resets the query on `wp_head` runs `the_post` for the global post before the theme's header.
    let mut looped = state.looped_posts.clone();
    if site.plugins.resets_query_in_head()
        && let Some(id) = crate::document::global_post_id(ctx.q)
    {
        looped.insert(0, id);
    }
    let plugin_footer = site.plugins.wp_footer(site, &looped).await?;
    let preview = preview(ctx);
    // `wp_underscore_playlist_templates()` runs first (the Text widget asks for it in the preview), the speculation
    // rules are for visitors.
    let playlist = if state.playlist || preview.is_some() { rustpress_blocks::shortcodes::playlist_templates(site) } else { String::new() };
    let speculation = if ctx.q.viewer.is_some() || preview.is_some() { String::new() } else { crate::document::speculation_rules(site, ctx.theme) };
    let nav_menus = preview.map(crate::customize_preview::nav_menus_exports).unwrap_or_default();
    let mut out = format!("{playlist}{speculation}{nav_menus}{plugin_footer}{importmap}{module_scripts}{}{preloads}", state.interactivity.data_script().unwrap_or_default());
    if let Some(preview) = preview {
        out.push_str(&preview_footer_scripts(ctx, state, preview).await?);
        out.push_str(&emoji_detection_script(site, wp_root, &version));
        out.push_str(crate::customize_preview::FOOTER_EXPORTS);
        out.push_str(&read(&ctx.theme.template_dir().join("assets/images/svg-icons.svg")).unwrap_or_default());
        return Ok(out);
    }
    let plugin_assets = site.plugins.assets(site);
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&site.site_url("wp-includes"), &script_version(wp_root, "jquery-core", "3.7.1"), &script_version(wp_root, "jquery-migrate", "3.4.1")).with_packages(wp_root, site);
    if ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar) {
        let includes_url = site.site_url("wp-includes");
        out.push_str(&format!("<script id=\"hoverintent-js-js\" src=\"{includes_url}/js/hoverintent-js.min.js?ver=2.2.1\"></script>\n<script id=\"admin-bar-js\" src=\"{includes_url}/js/admin-bar.min.js?ver={version}\"></script>\n"));
    }
    let (early_scripts, late_scripts): (Vec<_>, Vec<_>) = plugin_assets.scripts.iter().cloned().partition(|a| a.priority <= 10);
    out.push_str(&printer.scripts(&early_scripts, true));
    out.push_str(&format!("<script data-wp-strategy=\"defer\" defer id=\"jquery-scrollto-js\" src=\"{}/assets/js/jquery.scrollTo.js?ver=2.1.3\"></script>\n", ctx.theme.template_directory_uri()));
    // `comment-reply`, enqueued by `twentyseventeen_scripts()` for singular pages with open comments.
    if let (true, Queried::Post(post)) = (ctx.q.flags.is_singular(), &ctx.q.queried)
        && post.comment_status == "open"
        && site.options.get("thread_comments") == Some("1")
    {
        out.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{}/js/comment-reply.min.js?ver={version}\"></script>\n", site.site_url("wp-includes")));
    }
    out.push_str(&printer.scripts(&late_scripts, true));
    let includes_url = site.site_url("wp-includes");
    if state.playlist {
        // The head has printed jQuery.
        printer.mark_printed(&["jquery"]);
        out.push_str(&printer.core("wp-util"));
        out.push_str(&printer.core("backbone"));
    }
    if state.mediaelement || state.playlist {
        out.push_str(&crate::document::mediaelement_scripts(site, wp_root, &includes_url, &version, state.vimeo, state.mediaelement));
    }
    if state.playlist {
        printer.mark_printed(&["mediaelement", "mediaelement-core", "mediaelement-migrate"]);
        out.push_str(&printer.core("wp-playlist"));
    }
    out.push_str(&emoji_detection_script(site, wp_root, &version));
    out.push_str(&read(&ctx.theme.template_dir().join("assets/images/svg-icons.svg")).unwrap_or_default());
    Ok(out)
}

/// The scripts of the footer in the preview, in the order of the queue: `customize-preview` (`wp_loaded`) and the
/// theme's script for it (`customize_preview_init`), the toolbar's (`template_redirect`), the plugins', the theme's
/// (`wp_enqueue_scripts`), the media widgets' and the Text widget's, the components' and what the header enqueued.
async fn preview_footer_scripts(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State, preview: &rustpress_core::customize::Preview) -> Result<String> {
    let site = ctx.site;
    let wp_root = &ctx.theme.wp_root;
    let theme_uri = ctx.theme.template_directory_uri();
    let theme_scripts = [
        crate::customize_preview::ThemeScript { handle: "twentyseventeen-customize-preview", src: format!("{theme_uri}/assets/js/customize-preview.js"), deps: &["customize-preview"], ver: "20161002", strategy: None },
        crate::customize_preview::ThemeScript { handle: "jquery-scrollto", src: format!("{theme_uri}/assets/js/jquery.scrollTo.js"), deps: &["jquery"], ver: "2.1.3", strategy: Some("defer") },
    ];
    let mut scripts = crate::customize_preview::FooterScripts::new(site, wp_root, &theme_scripts, &["jquery", "jquery-core", "jquery-migrate"]);
    scripts.enqueue("customize-preview");
    scripts.enqueue("twentyseventeen-customize-preview");
    let mut out = scripts.print();
    if ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar) {
        let includes_url = site.site_url("wp-includes");
        out.push_str(&format!("<script id=\"hoverintent-js-js\" src=\"{includes_url}/js/hoverintent-js.min.js?ver=2.2.1\"></script>\n<script id=\"admin-bar-js\" src=\"{includes_url}/js/admin-bar.min.js?ver={}\"></script>\n", wp_version(wp_root)));
    }
    let plugin_assets = site.plugins.assets(site);
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&site.site_url("wp-includes"), &script_version(wp_root, "jquery-core", "3.7.1"), &script_version(wp_root, "jquery-migrate", "3.4.1")).with_packages(wp_root, site);
    printer.mark_printed(&["jquery"]);
    out.push_str(&printer.scripts(&plugin_assets.scripts, true));
    scripts.enqueue("jquery-scrollto");
    if let (true, Queried::Post(post)) = (ctx.q.flags.is_singular(), &ctx.q.queried)
        && post.comment_status == "open"
        && site.options.get("thread_comments") == Some("1")
    {
        scripts.enqueue("comment-reply");
    }
    // `WP_Widget_Media_Audio`, `WP_Widget_Media_Video` and `WP_Widget_Text::enqueue_preview_scripts()`.
    scripts.enqueue("wp-mediaelement");
    scripts.enqueue("mediaelement-vimeo");
    scripts.enqueue("wp-playlist");
    scripts.enqueue("customize-selective-refresh");
    if preview.widgets {
        scripts.enqueue("customize-preview-widgets");
    }
    scripts.enqueue("customize-preview-nav-menus");
    if state.custom_header_script {
        scripts.enqueue("wp-custom-header");
        scripts.localize("wp-custom-header", "_wpCustomHeaderSettings", &header_video_settings(ctx).await?);
    }
    out.push_str(&scripts.print());
    Ok(out)
}

/// `get_header_video_settings()` with the theme's `twentyseventeen_video_controls()`.
async fn header_video_settings(ctx: &Ctx<'_>) -> Result<serde_json::Value> {
    let (image, random) = header_image(ctx).await;
    let data = ctx.mods.get("header_image_data");
    let support = super::custom_header::support(&ctx.theme.template, &ctx.theme.template_directory_uri());
    let size = |name: &str, default: u64| data.and_then(|d| object_prop(d, name)).and_then(Value::as_i64).map_or(default, i64::unsigned_abs);
    let (mut width, mut height) = (size("width", support.as_ref().map_or(0, |s| s.width)), size("height", support.as_ref().map_or(0, |s| s.height)));
    if let Some(random) = &random {
        let pick = |value: &Option<Value>, default: u64| value.as_ref().and_then(Value::as_i64).map_or(default, i64::unsigned_abs);
        (width, height) = (pick(&random.width, support.as_ref().map_or(0, |s| s.width)), pick(&random.height, support.as_ref().map_or(0, |s| s.height)));
    }
    let video = header_video_url(ctx).await?;
    let mime = if regex::Regex::new(r"^https?://(?:www\.)?(?:youtube\.com/watch|youtu\.be/)").is_ok_and(|re| re.is_match(&video)) {
        "video/x-youtube".to_owned()
    } else {
        video_mime_type(&video).to_owned()
    };
    let control = |text: &str, icon: &str| format!("<span class=\"screen-reader-text\">{}</span>{}", t(ctx, text), svg(icon, false));
    Ok(serde_json::json!({
        "mimeType": mime,
        "posterUrl": image.unwrap_or_default(),
        "videoUrl": video,
        "width": width,
        "height": height,
        "minWidth": 900,
        "minHeight": 500,
        "l10n": {
            "pause": control("Pause background video", "pause"),
            "play": control("Play background video", "play"),
            "pauseSpeak": ctx.site.tr("Video is paused."),
            "playSpeak": ctx.site.tr("Video is playing."),
        },
    }))
}

/// The video types of `wp_check_filetype( $url, wp_get_mime_types() )`.
fn video_mime_type(url: &str) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let extension = path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).unwrap_or_default();
    match extension.as_str() {
        "asf" | "asx" => "video/x-ms-asf",
        "wmv" => "video/x-ms-wmv",
        "wmx" => "video/x-ms-wmx",
        "wm" => "video/x-ms-wm",
        "avi" => "video/avi",
        "divx" => "video/divx",
        "flv" => "video/x-flv",
        "mov" | "qt" => "video/quicktime",
        "mpeg" | "mpg" | "mpe" => "video/mpeg",
        "mp4" | "m4v" => "video/mp4",
        "ogv" => "video/ogg",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "3gp" | "3gpp" => "video/3gpp",
        "3g2" | "3gp2" => "video/3gpp2",
        _ => "",
    }
}

/// `get_header_video_url()`: the uploaded video, else the external one.
async fn header_video_url(ctx: &Ctx<'_>) -> Result<String> {
    let id = ctx.mod_str("header_video").and_then(|id| id.trim().parse::<i64>().ok()).map_or(0, i64::unsigned_abs);
    let url = if id != 0 {
        match ctx.site.db.post(id).await? {
            Some(post) if post.post_type == "attachment" => images::load_metadata(ctx.site, &post).await?.1.map(|file| images::attachment_url(ctx.site, &file)).unwrap_or_default(),
            _ => String::new(),
        }
    } else {
        ctx.mod_str("external_header_video").unwrap_or_default()
    };
    Ok(if url.is_empty() { url } else { rustpress_core::formatting::esc_url_raw(&url) })
}

/// `get_header_video_settings()` for the response of the partials (`export_header_video_settings()`).
pub(super) async fn header_video_export(ctx: &Ctx<'_>) -> Result<serde_json::Value> {
    let mut settings = header_video_settings(ctx).await?;
    // `get_header_video_url()` gives `false` without a video.
    if settings.get("videoUrl").and_then(serde_json::Value::as_str) == Some("")
        && let Some(map) = settings.as_object_mut()
    {
        map.insert("videoUrl".into(), serde_json::Value::Bool(false));
    }
    Ok(settings)
}

// ----------------------------------------------------------------------- page

/// The callbacks of the partials (`WP_Customize_Partial::render()`): the theme's, and those of WordPress and its
/// components the theme shows. `None` for a partial that is not registered or may not be refreshed (`null`); a
/// callback that fails gives `false`.
pub async fn render_partial(ctx: &Ctx<'_>, id: &str, context: Option<&serde_json::Value>) -> Result<Option<serde_json::Value>> {
    let failed = || Ok(Some(serde_json::Value::Bool(false)));
    Ok(Some(serde_json::Value::String(match id {
        // `twentyseventeen_customize_partial_blogname()` and `_blogdescription()`.
        "blogname" => bloginfo_display(ctx, &ctx.site.settings.blogname),
        "blogdescription" => bloginfo_display(ctx, &ctx.site.settings.blogdescription),
        // `_render_custom_logo_partial()`.
        "custom_logo" => custom_logo(ctx).await?,
        "custom_header" => custom_header_markup(ctx).await?,
        "panel_1" | "panel_2" | "panel_3" | "panel_4" => front_page_section(ctx, id[6..].parse().unwrap_or(1)).await?,
        _ => {
            if let Some(widget) = id.strip_prefix("widget[").and_then(|rest| rest.strip_suffix(']')) {
                // `render_widget_partial()`: the widget alone in its sidebar.
                // The setting of the widget must exist.
                if widgets::split_id(widget).is_none_or(|(base, number)| widgets::instance(ctx.site, base, number).is_none()) {
                    return Ok(None);
                }
                let Some(sidebar_id) = context.and_then(|c| c.get("sidebar_id")).and_then(serde_json::Value::as_str) else { return failed() };
                let Some(registered) = ["sidebar-1", "sidebar-2", "sidebar-3"].into_iter().find(|s| *s == sidebar_id) else { return failed() };
                let number = context.and_then(|c| c.get("sidebar_instance_number")).and_then(serde_json::Value::as_i64);
                let request_uri = ctx.request_uri();
                let search = || search_form(ctx);
                let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
                widgets::render_widget_partial(&services, &sidebar(registered), widget, number).await?
            } else if id.strip_prefix("nav_menu_instance[").and_then(|rest| rest.strip_suffix(']')).is_some_and(|hmac| hmac.len() == 32 && hmac.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())) {
                // `render_nav_menu_partial()`: `false` unless the arguments carry their hash.
                match context {
                    Some(args) => match nav_menu_partial(ctx, args).await? {
                        Some(html) => html,
                        None => return failed(),
                    },
                    None => return failed(),
                }
            } else {
                return Ok(None);
            }
        }
    })))
}

/// `wp_nav_menu( $nav_menu_args )` of a partial, after its `args_hmac` was checked.
async fn nav_menu_partial(ctx: &Ctx<'_>, args: &serde_json::Value) -> Result<Option<String>> {
    let Some(map) = args.as_object() else { return Ok(None) };
    let Some(hmac) = map.get("args_hmac").and_then(serde_json::Value::as_str) else { return Ok(None) };
    let mut sorted: Vec<(&String, &serde_json::Value)> = map.iter().filter(|(k, _)| *k != "args_hmac").collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let php = rustpress_phpser::Value::Array(sorted.iter().map(|(k, v)| (rustpress_phpser::Key::Str(k.as_bytes().to_vec()), menu::json_to_php(v))).collect());
    let expected = rustpress_blocks::commenter::wp_hash(&String::from_utf8_lossy(&rustpress_phpser::serialize(&php)), "auth");
    if !rustpress_core::hash_equals(&expected, hmac) {
        return Ok(None);
    }
    let text = |key: &str| map.get(key).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
    let location = text("theme_location");
    let mut nav = NavArgs::new(&location);
    nav.menu_id = text("menu_id");
    nav.menu_class = text("menu_class");
    nav.depth = map.get("depth").and_then(serde_json::Value::as_u64).unwrap_or(0) as usize;
    nav.link_before = text("link_before");
    nav.link_after = text("link_after");
    nav.container = map.get("container").and_then(serde_json::Value::as_str).filter(|c| !c.is_empty()).map(|tag| (tag.to_owned(), Some(text("container_aria_label"))));
    nav.menu = map.get("menu").and_then(serde_json::Value::as_u64);
    nav.fallback_cb = if text("fallback_cb") == "wp_page_menu" { "wp_page_menu" } else { "" };
    nav.customize = ctx.q.customize.clone();
    let id = match nav.menu {
        Some(id) => Some(id),
        None => menu::menu_location(&ctx.mods, &location),
    };
    let Some(id) = id else { return Ok(Some(String::new())) };
    let Some(mut found) = menu::load_menu(ctx.site, id).await? else { return Ok(Some(String::new())) };
    let uri = ctx.request_uri();
    menu::classes_by_context(ctx.site, ctx.q, &uri, &mut found.items).await?;
    nav.use_ids_once(&ctx.env, &found.items);
    let link_after = nav.link_after.clone();
    let title_hook = |title: &str, item: &MenuItem, _depth: usize| {
        if location == "top" && item.classes.iter().any(|c| c == "menu-item-has-children" || c == "page_item_has_children") { format!("{title}{}", svg("angle-down", false)) } else { title.to_owned() }
    };
    let link_hook = |link: String, _: &MenuItem, _: usize| match SOCIAL_ICONS.iter().find(|(address, _)| location == "social" && link.contains(address)) {
        Some((_, icon)) => link.replace(&link_after, &format!("</span>{}", svg(icon, false))),
        None => link,
    };
    Ok(Some(menu::wp_nav_menu(ctx.site, &found, &found.items, &nav, &title_hook, &link_hook)))
}

/// `twentyseventeen_body_classes()`.
pub(super) async fn add_body_classes(ctx: &Ctx<'_>, classes: &mut Vec<String>) -> Result<()> {
    let site = ctx.site;
    let q = ctx.q;
    if is_multi_author(site).await? {
        classes.push("group-blog".into());
    }
    if !q.flags.is_singular() {
        classes.push("hfeed".into());
    }
    if preview(ctx).is_some() {
        classes.push("twentyseventeen-customizer".into());
    }
    if q.flags.is_front_page && site.options.string("show_on_front", "posts") != "posts" {
        classes.push("twentyseventeen-front-page".into());
    }
    if has_header_image(ctx).await {
        classes.push("has-header-image".into());
    }
    if active_sidebar(ctx, "sidebar-1") && !q.flags.is_page {
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
    Ok(())
}

/// `WP_REST_Widget_Types_Controller::render_legacy_widget_preview_iframe()`.
pub async fn legacy_widget_preview(ctx: &Ctx<'_>, registry: &Registry, id_base: &str, instance: Option<&Value>) -> Result<String> {
    let site = ctx.site;
    let mut classes = body_classes(ctx.theme, site, ctx.q).await?;
    add_body_classes(ctx, &mut classes).await?;
    // `render_block_core_legacy_widget()`: without a signed instance the widget has empty settings.
    let empty = Value::Array(Vec::new());
    let search = || search_form_markup(site, ctx.q, &ctx.env);
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

pub async fn render(ctx: &Ctx<'_>, registry: &Registry, extras: &crate::document::Extras) -> Result<Document> {
    let site = ctx.site;
    let q = ctx.q;
    // Body classes: core's, then the theme's (`twentyseventeen_body_classes()`).
    let mut classes = body_classes(ctx.theme, site, q).await?;
    add_body_classes(ctx, &mut classes).await?;

    let content_width = content_width(ctx);
    ctx.env.with_state(|s| s.content_width = content_width);
    let toolbar_on = ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar);
    // The toolbar has no template to edit in a classic theme.
    let bar = match (&extras.toolbar, toolbar_on) {
        // Between `get_header` and the loop images count as being in the viewport: the avatars are not lazy.
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
        "<!DOCTYPE html>\n<html {}lang=\"{}\" class=\"no-js no-svg\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<link rel=\"profile\" href=\"https://gmpg.org/xfn/11\">\n\n{head_html}</head>\n\n{body}{footer_html}\n</body>\n</html>\n",
        if site.is_rtl() { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
    );
    Ok(Document { status: q.status(), html })
}
