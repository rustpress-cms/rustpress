//! Twenty Twenty 3.3: the templates of the theme (`header.php`, `index.php`, `singular.php`, `404.php`, `footer.php`,
//! the page templates in `templates/` and the files in `template-parts/`) and what its `functions.php`, `inc/` and
//! `classes/` add to the page.
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

use super::Ctx;
use super::menu::{self, MenuItem, NavArgs};
use super::tags::{self, Pagination};
use super::twentytwenty_icons as icons;
use super::widgets::{self, Sidebar};
use crate::Result;
use crate::document::{Document, INLINE_LIMIT, body_classes, discovery_links, document_title, emoji_detection_script, emoji_style, oembed_links, read, script_modules, wp_version};
use crate::global_styles::GlobalStyles;

const THEME: &str = "twentytwenty";

/// `wp_get_theme()->get( 'Version' )`, the version of every asset of the theme.
const VERSION: &str = "3.3";

/// The `default-color` of the theme's `custom-background` support.
const BACKGROUND: &str = "f5efe0";

/// `twenty_twenty_toggle_duration()`.
const TOGGLE_DURATION: u32 = 250;

/// The two footer sidebars of `twentytwenty_sidebar_registration()`.
fn sidebar(id: &'static str) -> Sidebar<'static> {
    Sidebar { id, before_widget: "<div class=\"widget %2$s\"><div class=\"widget-content\">", after_widget: "</div></div>", before_title: "<h2 class=\"widget-title subheading heading-size-3\">", after_title: "</h2>" }
}

/// The sidebars `twentytwenty_sidebar_registration()` registers.
pub fn registered_sidebars(site: &Site) -> Vec<widgets::RegisteredSidebar> {
    let i18n = site.i18n();
    [
        ("sidebar-1", "Footer #1", "Widgets in this area will be displayed in the first column in the footer."),
        ("sidebar-2", "Footer #2", "Widgets in this area will be displayed in the second column in the footer."),
    ]
    .into_iter()
    .map(|(id, name, description)| {
        let wrap = sidebar(id);
        widgets::RegisteredSidebar {
            id: id.to_owned(),
            name: esc_html(&i18n.gettext(THEME, name)),
            description: esc_html(&i18n.gettext(THEME, description)),
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

/// Translates with the theme's text domain.
fn t(ctx: &Ctx<'_>, text: &str) -> String {
    ctx.site.i18n().gettext(THEME, text)
}

/// `_x()` with the theme's text domain.
fn tx(ctx: &Ctx<'_>, text: &str, context: &str) -> String {
    ctx.site.i18n().pgettext(THEME, context, text)
}

/// `twentytwenty_get_theme_svg()` of the `ui` group.
fn svg(icon: &str) -> &'static str {
    icons::UI.iter().find(|(name, _)| *name == icon).map_or("", |(_, svg)| *svg)
}

/// `TwentyTwenty_SVG_Icons::get_social_link_svg()`: the icon of the first network whose domains the address contains
/// (case-insensitively).
fn social_link_svg(uri: &str) -> Option<&'static str> {
    let uri = uri.to_lowercase();
    icons::SOCIAL.iter().find_map(|(icon, svg)| {
        let own = [format!("{icon}.com")];
        let domains: Vec<String> = match icons::SOCIAL_MAP.iter().find(|(name, _)| name == icon) {
            Some((_, domains)) => domains.iter().map(|d| d.trim().to_lowercase()).collect(),
            None => own.to_vec(),
        };
        domains.iter().any(|d| uri.contains(d.as_str())).then_some(*svg)
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

/// `get_theme_mod( $name, true )` as a condition.
fn mod_on(ctx: &Ctx<'_>, name: &str) -> bool {
    ctx.mods.get(name).is_none_or(php_truthy)
}

/// `true === get_theme_mod( 'enable_header_search', true )`.
fn header_search(ctx: &Ctx<'_>) -> bool {
    matches!(ctx.mods.get("enable_header_search"), None | Some(Value::Bool(true)))
}

/// `has_nav_menu()`.
async fn has_menu(ctx: &Ctx<'_>, location: &str) -> Result<bool> {
    menu::has_nav_menu(ctx.site, preview(ctx), &ctx.mods, location).await
}

/// `has_custom_logo()`.
async fn has_custom_logo(ctx: &Ctx<'_>) -> Result<bool> {
    Ok(images::custom_logo_id(ctx.site).await? != 0)
}

/// `get_bloginfo( $show, 'display' )`: the filters of `bloginfo`.
fn bloginfo_display(ctx: &Ctx<'_>, value: &str) -> String {
    esc_html(&rustpress_core::formatting::convert_chars(&ctx.site.texturize(value)))
}

/// `sanitize_hex_color()`: `None` for what is not a color (and `Some("")` for an empty value).
fn sanitize_hex_color(color: &str) -> Option<String> {
    if color.is_empty() {
        return Some(String::new());
    }
    let hex = color.strip_prefix('#')?;
    ((hex.len() == 3 || hex.len() == 6) && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| color.to_owned())
}

/// `twentytwenty_get_color_for_area()`.
fn color_for_area(mods: &Value, area: &str, context: &str) -> Option<String> {
    let text = |value: &Value| match value {
        Value::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
        Value::Int(i) => Some(i.to_string()),
        _ => None,
    };
    match mods.get("accent_accessible_colors") {
        // `isset()` is false for `null`.
        Some(settings) => settings.get(area).and_then(|colors| colors.get(context)).and_then(text),
        None => {
            let defaults = [("text", "#000000"), ("accent", "#cd2653"), ("secondary", "#6d6d6d"), ("borders", "#dcd7ca")];
            matches!(area, "content" | "header-footer").then(|| defaults.iter().find(|(name, _)| *name == context).map(|(_, color)| (*color).to_owned())).flatten()
        }
    }
}

/// `get_theme_mod( 'background_color', 'f5efe0' )`.
fn background_color(mods: &Value) -> String {
    match mods.get("background_color") {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => BACKGROUND.to_owned(),
    }
}

/// The editor palette and font sizes of `twentytwenty_block_editor_settings()` as the `settings` of the theme's
/// `theme.json` layer.
pub fn editor_presets(i18n: &rustpress_core::i18n::I18n, mods: &Value) -> (serde_json::Value, serde_json::Value) {
    use serde_json::json;
    let t = |text: &str| esc_html(&i18n.gettext(THEME, text));
    let x = |text: &str, context: &str| esc_html(&i18n.pgettext(THEME, context, text));
    let color = |context: &str| color_for_area(mods, "content", context).unwrap_or_default();
    // `get_theme_mod( 'background_color' )`, or the default of the theme support.
    let background = match mods.get("background_color") {
        Some(value) if php_truthy(value) => match value {
            Value::Str(s) => String::from_utf8_lossy(s).into_owned(),
            Value::Int(i) => i.to_string(),
            _ => BACKGROUND.to_owned(),
        },
        _ => BACKGROUND.to_owned(),
    };
    let palette = json!([
        { "name": t("Accent Color"), "slug": "accent", "color": color("accent") },
        { "name": x("Primary", "color"), "slug": "primary", "color": color("text") },
        { "name": x("Secondary", "color"), "slug": "secondary", "color": color("secondary") },
        { "name": t("Subtle Background"), "slug": "subtle-background", "color": color("borders") },
        { "name": t("Background Color"), "slug": "background", "color": format!("#{background}") },
    ]);
    let sizes = [
        ("Small", "Name of the small font size in the block editor", "S", "Short name of the small font size in the block editor.", "18px", "small"),
        ("Regular", "Name of the regular font size in the block editor", "M", "Short name of the regular font size in the block editor.", "21px", "normal"),
        ("Large", "Name of the large font size in the block editor", "L", "Short name of the large font size in the block editor.", "26.25px", "large"),
        ("Larger", "Name of the larger font size in the block editor", "XL", "Short name of the larger font size in the block editor.", "32px", "larger"),
    ];
    let sizes = sizes.iter().map(|(name, context, short, short_context, size, slug)| json!({ "name": x(name, context), "shortName": x(short, short_context), "size": size, "slug": slug })).collect();
    (palette, sizes)
}

// ------------------------------------------------------------------- template tags

/// `get_custom_logo()` with `twentytwenty_get_custom_logo()` (the retina logo at half its size).
async fn custom_logo(ctx: &Ctx<'_>) -> Result<String> {
    let id = images::custom_logo_id(ctx.site).await?;
    if id == 0 {
        return Ok(match preview(ctx) {
            Some(_) => format!("<a href=\"{}\" class=\"custom-logo-link\" style=\"display:none;\"><img class=\"custom-logo\" alt=\"\" /></a>", esc_url(&ctx.site.home_url("/"))),
            None => String::new(),
        });
    }
    let mut attrs: Vec<(String, AttrValue)> = vec![("class".into(), AttrValue::Text("custom-logo".into())), ("loading".into(), AttrValue::Omit)];
    let rows = ctx.site.db.post_meta(id).await?;
    let alt = rows.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    if alt.is_empty() || alt == "0" {
        attrs.push(("alt".into(), AttrValue::Text(bloginfo_display(ctx, &ctx.site.settings.blogname))));
    }
    let image = images::get_attachment_image(&ctx.env, id, "full", &ImageArgs { attrs, ..ImageArgs::default() }).await?;
    let q = ctx.q;
    let current = !q.flags.is_paged && (q.flags.is_front_page || (q.flags.is_home && ctx.site.settings.page_for_posts != q.queried_object_id()));
    let aria = if current { " aria-current=\"page\"" } else { "" };
    let html = format!("<a href=\"{}\" class=\"custom-logo-link\" rel=\"home\"{aria}>{image}</a>", esc_url(&ctx.site.home_url("/")));
    if image.is_empty() || !ctx.mods.get("retina_logo").is_some_and(php_truthy) {
        return Ok(html);
    }
    let Some(attachment) = ctx.site.db.post(id).await? else { return Ok(html) };
    let (Some(meta), _) = images::load_metadata(ctx.site, &attachment).await? else { return Ok(html) };
    let (width, height) = (meta.width() / 2, meta.height() / 2);
    static WIDTH: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r#"(?i)width="\d+""#).unwrap());
    static HEIGHT: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r#"(?i)height="\d+""#).unwrap());
    static STYLE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r#"(style="[^"]*)"#).unwrap());
    let html = WIDTH.replace_all(&html, format!("width=\"{width}\"").as_str()).into_owned();
    let html = HEIGHT.replace_all(&html, format!("height=\"{height}\"").as_str()).into_owned();
    Ok(if html.contains(" style=") { STYLE.replace_all(&html, format!("$1 height: {height}px;").as_str()).into_owned() } else { html.replace("src=", &format!("style=\"height: {height}px;\" src=")) })
}

/// `twentytwenty_site_logo()`.
async fn site_logo(ctx: &Ctx<'_>) -> Result<String> {
    let q = ctx.q;
    let f = &q.flags;
    let title = &ctx.site.settings.blogname;
    let (classname, contents) = if has_custom_logo(ctx).await? {
        ("site-logo", format!("{}<span class=\"screen-reader-text\">{}</span>", custom_logo(ctx).await?, esc_html(title)))
    } else {
        if title.is_empty() {
            return Ok(String::new());
        }
        let mut contents = format!("<a href=\"{}\" rel=\"home\">{}</a>", esc_url(&ctx.site.home_url("/")), esc_html(title));
        if (f.is_front_page || (f.is_home && ctx.site.settings.page_for_posts != q.queried_object_id())) && !f.is_paged {
            contents = contents.replace(" rel=", " aria-current=\"page\" rel=");
        }
        ("site-title", contents)
    };
    Ok(if (f.is_front_page || f.is_home) && !f.is_page { format!("<h1 class=\"{classname}\">{contents}</h1>") } else { format!("<div class=\"{classname} faux-heading\">{contents}</div>") })
}

/// `twentytwenty_site_description()`.
fn site_description(ctx: &Ctx<'_>) -> String {
    let description = &ctx.site.settings.blogdescription;
    if description.is_empty() {
        return String::new();
    }
    format!("<div class=\"site-description\">{}</div><!-- .site-description -->", esc_html(description))
}

/// The sub-menu toggle of `twentytwenty_add_sub_toggles_to_main_menu()` and `TwentyTwenty_Walker_Page`.
fn sub_menu_toggle(ctx: &Ctx<'_>, target: &str) -> String {
    format!(
        "<button class=\"toggle sub-menu-toggle fill-children-current-color\" data-toggle-target=\"{target}\" data-toggle-type=\"slidetoggle\" data-toggle-duration=\"{TOGGLE_DURATION}\" aria-expanded=\"false\"><span class=\"screen-reader-text\">{}</span>{}</button>",
        t(ctx, "Show sub menu"),
        svg("chevron-down")
    )
}

/// `wp_nav_menu()` for one of the theme's locations, with `container` and `items_wrap` empty.
async fn nav_menu(ctx: &Ctx<'_>, location: &str, show_toggles: bool) -> Result<String> {
    let Some(id) = menu::menu_location(&ctx.mods, location) else { return Ok(String::new()) };
    let Some(mut found) = menu::load_menu(ctx.site, preview(ctx), id).await? else { return Ok(String::new()) };
    let uri = ctx.request_uri();
    menu::classes_by_context(ctx.site, ctx.q, &uri, &mut found.items).await?;
    let mut args = NavArgs::new(location);
    args.container = None;
    args.items_wrap = "%3$s".into();
    args.customize = ctx.q.customize.clone();
    let social = location == "social";
    if location == "footer" || social {
        args.depth = 1;
    }
    if social {
        args.fallback_cb = "";
        args.menu_class = String::new();
        args.link_before = "<span class=\"screen-reader-text\">".into();
        args.link_after = "</span>".into();
    }
    let title_hook = |title: &str, _: &MenuItem, _: usize| title.to_owned();
    // `twentytwenty_add_sub_toggles_to_main_menu()` on `nav_menu_item_args` (`before` and `after`) and
    // `twentytwenty_nav_menu_social_icons()` on `walker_nav_menu_start_el`.
    let link_hook = |link: String, item: &MenuItem, _: usize| {
        let has_children = item.classes.iter().any(|c| c == "menu-item-has-children");
        let mut output = if show_toggles {
            let toggle = if has_children { sub_menu_toggle(ctx, &format!(".menu-modal .menu-item-{} > .sub-menu", item.id)) } else { String::new() };
            format!("<div class=\"ancestor-wrapper\">{link}{toggle}</div><!-- .ancestor-wrapper -->")
        } else if location == "primary" && has_children {
            format!("{link}<span class=\"icon\"></span>")
        } else {
            link
        };
        if social {
            output = output.replace("</span>", &format!("</span>{}", social_link_svg(&item.url).unwrap_or_else(|| svg("link"))));
        }
        output
    };
    args.use_ids_once(&ctx.env, &found.items);
    Ok(menu::wp_nav_menu(ctx.site, &found, &found.items, &args, &title_hook, &link_hook))
}

/// `wp_list_pages()` with `TwentyTwenty_Walker_Page`, `match_menu_classes` and either `show_toggles` or
/// `show_sub_menu_icons`: the fallback of the theme's menus.
async fn list_pages(ctx: &Ctx<'_>, show_toggles: bool) -> Result<String> {
    use rustpress_db::{Order, OrderBy, PostQuery};
    let site = ctx.site;
    let q = ctx.q;
    let mut pages = site.db.posts(&PostQuery { post_types: vec!["page".into()], order_by: OrderBy::Title, order: Order::Asc, ..PostQuery::default() }).await?;
    // `sort_column` `menu_order, post_title`.
    pages.sort_by_key(|p| p.menu_order);
    // The current page of `wp_list_pages()`.
    let current = match &q.queried {
        Queried::Post(post) if q.flags.is_page || q.flags.is_attachment || q.flags.is_posts_page => Some(post.as_ref().clone()),
        Queried::Post(post) if q.flags.is_singular() && site.types.post_type(&post.post_type).is_some_and(|t| t.hierarchical) => Some(post.as_ref().clone()),
        _ => None,
    };
    let mut ancestors: Vec<u64> = Vec::new();
    if let Some(current) = &current {
        let mut parent = current.post_parent;
        while parent != 0 && !ancestors.contains(&parent) {
            ancestors.push(parent);
            parent = site.db.post(parent).await?.map_or(0, |p| p.post_parent);
        }
    }
    let ids: Vec<u64> = pages.iter().map(|p| p.id).collect();
    let with_children: Vec<u64> = pages.iter().filter(|p| p.post_parent != 0).map(|p| p.post_parent).collect();
    let mut links = std::collections::HashMap::new();
    for page in &pages {
        links.insert(page.id, site.permalink(page).await?);
    }
    struct Walk<'a> {
        ctx: &'a Ctx<'a>,
        pages: &'a [Post],
        current: Option<&'a Post>,
        ancestors: &'a [u64],
        with_children: &'a [u64],
        links: &'a std::collections::HashMap<u64, String>,
        show_toggles: bool,
    }
    fn element(w: &Walk<'_>, page: &Post, depth: usize, out: &mut String) {
        let ctx = w.ctx;
        let has_children = w.with_children.contains(&page.id);
        let mut classes = vec!["page_item".to_owned(), format!("page-item-{}", page.id)];
        if has_children {
            classes.push("page_item_has_children".into());
        }
        let is_current = w.current.is_some_and(|c| c.id == page.id);
        if let Some(current) = w.current {
            if w.ancestors.contains(&page.id) {
                classes.push("current_page_ancestor".into());
            }
            if is_current {
                classes.push("current_page_item".into());
            } else if page.id == current.post_parent {
                classes.push("current_page_parent".into());
            }
        }
        // (`get_option( 'page_for_posts' ) === $page->ID` compares a string with a number, so it never holds.)
        // `twentytwenty_filter_wp_list_pages_item_classes()`.
        if is_current {
            classes.push("current-menu-item".into());
        }
        if has_children {
            classes.push("menu-item-has-children".into());
        }
        let title = if page.post_title.is_empty() { sprintf(&ctx.site.i18n().gettext(THEME, "#%d (no title)"), &[&page.id.to_string()]) } else { page.post_title.clone() };
        let mut attributes = format!(" href=\"{}\"", esc_url(w.links.get(&page.id).map_or("", String::as_str)));
        if is_current {
            attributes.push_str(" aria-current=\"page\"");
        }
        let (before, after) = if w.show_toggles {
            let toggle = if has_children { sub_menu_toggle(ctx, &format!(".menu-modal .page-item-{} > ul", page.id)) } else { String::new() };
            ("<div class=\"ancestor-wrapper\">", format!("{toggle}</div><!-- .ancestor-wrapper -->"))
        } else {
            ("", if has_children { "<span class=\"icon\"></span>".to_owned() } else { String::new() })
        };
        out.push_str(&format!("{}<li class=\"{}\">{before}<a{attributes}>{}</a>{after}", "\t".repeat(depth), esc_attr(&classes.join(" ")), menu::the_title_filters(ctx.site, &title)));
    }
    fn walk(w: &Walk<'_>, parent: u64, depth: usize, out: &mut String) {
        for page in w.pages.iter().filter(|p| p.post_parent == parent) {
            element(w, page, depth, out);
            if w.pages.iter().any(|c| c.post_parent == page.id) {
                let indent = "\t".repeat(depth);
                out.push_str(&format!("\n{indent}<ul class='children'>\n"));
                walk(w, page.id, depth + 1, out);
                out.push_str(&format!("{indent}</ul>\n"));
            }
            out.push_str("</li>\n");
        }
    }
    let w = Walk { ctx, pages: &pages, current: current.as_ref(), ancestors: &ancestors, with_children: &with_children, links: &links, show_toggles };
    let mut out = String::new();
    walk(&w, 0, 0, &mut out);
    // `Walker::walk()`: pages whose parent is not listed follow at the top, with their children.
    for page in pages.iter().filter(|p| p.post_parent != 0 && !ids.contains(&p.post_parent)) {
        element(&w, page, 0, &mut out);
        if pages.iter().any(|c| c.post_parent == page.id) {
            out.push_str("\n<ul class='children'>\n");
            walk(&w, page.id, 1, &mut out);
            out.push_str("</ul>\n");
        }
        out.push_str("</li>\n");
    }
    Ok(out)
}

// ---------------------------------------------------------------------- header

/// `searchform.php`; `aria_label` is the argument of `get_search_form()`.
fn search_form_markup(site: &Site, q: &rustpress_core::QueryResult, env: &rustpress_blocks::Env<'_>, aria_label: &str) -> String {
    let unique = env.with_state(|s| s.unique_id("search-form-"));
    let aria = if aria_label.is_empty() { String::new() } else { format!("aria-label=\"{}\"", esc_attr(aria_label)) };
    let i18n = site.i18n();
    format!(
        "<form role=\"search\" {aria} method=\"get\" class=\"search-form\" action=\"{}\">\n\t<label for=\"{id}\">\n\t\t<span class=\"screen-reader-text\">\n\t\t\t{}\t\t</span>\n\t\t<input type=\"search\" id=\"{id}\" class=\"search-field\" placeholder=\"{}\" value=\"{}\" name=\"s\" />\n\t</label>\n\t<input type=\"submit\" class=\"search-submit\" value=\"{}\" />\n</form>\n",
        esc_url(&site.home_url("/")),
        i18n.gettext(THEME, "Search for:"),
        esc_attr(&i18n.pgettext(THEME, "placeholder", "Search &hellip;")),
        esc_attr(q.vars.get("s").unwrap_or("")),
        esc_attr(&i18n.pgettext(THEME, "submit button", "Search")),
        id = esc_attr(&unique),
    )
}

/// `get_search_form()` for a widget rendered outside of a page of the theme.
pub fn widget_search_form(site: &Site, q: &rustpress_core::QueryResult, env: &rustpress_blocks::Env<'_>) -> String {
    search_form_markup(site, q, env, "")
}

/// `template-parts/modal-search.php`.
fn modal_search(ctx: &Ctx<'_>) -> String {
    format!(
        "<div class=\"search-modal cover-modal header-footer-group\" data-modal-target-string=\".search-modal\" role=\"dialog\" aria-modal=\"true\" aria-label=\"{}\">\n\n\t<div class=\"search-modal-inner modal-inner\">\n\n\t\t<div class=\"section-inner\">\n\n\t\t\t{}\n\t\t\t<button class=\"toggle search-untoggle close-search-toggle fill-children-current-color\" data-toggle-target=\".search-modal\" data-toggle-body-class=\"showing-search-modal\" data-set-focus=\".search-modal .search-field\">\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t\t{}\t\t\t</button><!-- .search-toggle -->\n\n\t\t</div><!-- .section-inner -->\n\n\t</div><!-- .search-modal-inner -->\n\n</div><!-- .menu-modal -->\n",
        esc_attr(&t(ctx, "Search")),
        search_form_markup(ctx.site, ctx.q, &ctx.env, &t(ctx, "Search for:")),
        t(ctx, "Close search"),
        svg("cross"),
    )
}

/// `template-parts/modal-menu.php`.
async fn modal_menu(ctx: &Ctx<'_>) -> Result<String> {
    let (mobile, primary, expanded) = (has_menu(ctx, "mobile").await?, has_menu(ctx, "primary").await?, has_menu(ctx, "expanded").await?);
    let location = if mobile {
        "mobile"
    } else if primary {
        "primary"
    } else if expanded {
        "expanded"
    } else {
        ""
    };
    let mut out = format!(
        "\n<div class=\"menu-modal cover-modal header-footer-group\" data-modal-target-string=\".menu-modal\">\n\n\t<div class=\"menu-modal-inner modal-inner\">\n\n\t\t<div class=\"menu-wrapper section-inner\">\n\n\t\t\t<div class=\"menu-top\">\n\n\t\t\t\t<button class=\"toggle close-nav-toggle fill-children-current-color\" data-toggle-target=\".menu-modal\" data-toggle-body-class=\"showing-menu-modal\" data-set-focus=\".menu-modal\">\n\t\t\t\t\t<span class=\"toggle-text\">{}</span>\n\t\t\t\t\t{}\t\t\t\t</button><!-- .nav-toggle -->\n\n\t\t\t\t",
        t(ctx, "Close Menu"),
        svg("cross")
    );
    if expanded {
        out.push_str(&format!(
            "\n\t\t\t\t\t<nav class=\"expanded-menu{}\" aria-label=\"{}\">\n\n\t\t\t\t\t\t<ul class=\"modal-menu reset-list-style\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</ul>\n\n\t\t\t\t\t</nav>\n\n\t\t\t\t\t",
            if location == "expanded" { " mobile-menu" } else { "" },
            esc_attr(&tx(ctx, "Expanded", "menu")),
            nav_menu(ctx, "expanded", true).await?
        ));
    }
    if location != "expanded" {
        let list = if location.is_empty() { list_pages(ctx, true).await? } else { nav_menu(ctx, location, true).await? };
        out.push_str(&format!(
            "\n\t\t\t\t\t<nav class=\"mobile-menu\" aria-label=\"{}\">\n\n\t\t\t\t\t\t<ul class=\"modal-menu reset-list-style\">\n\n\t\t\t\t\t\t{list}\n\t\t\t\t\t\t</ul>\n\n\t\t\t\t\t</nav>\n\n\t\t\t\t\t",
            esc_attr(&tx(ctx, "Mobile", "menu"))
        ));
    }
    out.push_str("\n\t\t\t</div><!-- .menu-top -->\n\n\t\t\t<div class=\"menu-bottom\">\n\n\t\t\t\t");
    if has_menu(ctx, "social").await? {
        out.push_str(&format!(
            "\n\t\t\t\t\t<nav aria-label=\"{}\">\n\t\t\t\t\t\t<ul class=\"social-menu reset-list-style social-icons fill-children-current-color\">\n\n\t\t\t\t\t\t\t{}\n\t\t\t\t\t\t</ul>\n\t\t\t\t\t</nav><!-- .social-menu -->\n\n\t\t\t\t",
            esc_attr(&t(ctx, "Expanded Social links")),
            nav_menu(ctx, "social", false).await?
        ));
    }
    out.push_str("\n\t\t\t</div><!-- .menu-bottom -->\n\n\t\t</div><!-- .menu-wrapper -->\n\n\t</div><!-- .menu-modal-inner -->\n\n</div><!-- .menu-modal -->\n");
    Ok(out)
}

/// `header.php` from `wp_body_open()` on.
async fn header(ctx: &Ctx<'_>) -> Result<String> {
    ctx.env.with_state(|s| s.media.in_header = true);
    let search = header_search(ctx);
    let search_text = tx(ctx, "Search", "toggle text");
    let mobile_search = if search {
        format!(
            "\n\t\t\t\t\t\t<button class=\"toggle search-toggle mobile-search-toggle\" data-toggle-target=\".search-modal\" data-toggle-body-class=\"showing-search-modal\" data-set-focus=\".search-modal .search-field\" aria-expanded=\"false\">\n\t\t\t\t\t\t\t<span class=\"toggle-inner\">\n\t\t\t\t\t\t\t\t<span class=\"toggle-icon\">\n\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t\t<span class=\"toggle-text\">{search_text}</span>\n\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t</button><!-- .search-toggle -->\n\n\t\t\t\t\t",
            svg("search")
        )
    } else {
        String::new()
    };
    let logo = site_logo(ctx).await?;
    let description = site_description(ctx);
    let menu_text = t(ctx, "Menu");
    let (primary, expanded) = (has_menu(ctx, "primary").await?, has_menu(ctx, "expanded").await?);
    let mut nav = String::new();
    if primary || !expanded {
        let list = if primary { nav_menu(ctx, "primary", false).await? } else { list_pages(ctx, false).await? };
        nav.push_str(&format!(
            "\n\t\t\t\t\t\t\t<nav class=\"primary-menu-wrapper\" aria-label=\"{}\">\n\n\t\t\t\t\t\t\t\t<ul class=\"primary-menu reset-list-style\">\n\n\t\t\t\t\t\t\t\t{list}\n\t\t\t\t\t\t\t\t</ul>\n\n\t\t\t\t\t\t\t</nav><!-- .primary-menu-wrapper -->\n\n\t\t\t\t\t\t",
            esc_attr(&tx(ctx, "Horizontal", "menu"))
        ));
    }
    if search || expanded {
        nav.push_str("\n\t\t\t\t\t\t<div class=\"header-toggles hide-no-js\">\n\n\t\t\t\t\t\t");
        if expanded {
            nav.push_str(&format!(
                "\n\t\t\t\t\t\t\t<div class=\"toggle-wrapper nav-toggle-wrapper has-expanded-menu\">\n\n\t\t\t\t\t\t\t\t<button class=\"toggle nav-toggle desktop-nav-toggle\" data-toggle-target=\".menu-modal\" data-toggle-body-class=\"showing-menu-modal\" aria-expanded=\"false\" data-set-focus=\".close-nav-toggle\">\n\t\t\t\t\t\t\t\t\t<span class=\"toggle-inner\">\n\t\t\t\t\t\t\t\t\t\t<span class=\"toggle-text\">{menu_text}</span>\n\t\t\t\t\t\t\t\t\t\t<span class=\"toggle-icon\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t\t</button><!-- .nav-toggle -->\n\n\t\t\t\t\t\t\t</div><!-- .nav-toggle-wrapper -->\n\n\t\t\t\t\t\t\t",
                svg("ellipsis")
            ));
        }
        if search {
            nav.push_str(&format!(
                "\n\t\t\t\t\t\t\t<div class=\"toggle-wrapper search-toggle-wrapper\">\n\n\t\t\t\t\t\t\t\t<button class=\"toggle search-toggle desktop-search-toggle\" data-toggle-target=\".search-modal\" data-toggle-body-class=\"showing-search-modal\" data-set-focus=\".search-modal .search-field\" aria-expanded=\"false\">\n\t\t\t\t\t\t\t\t\t<span class=\"toggle-inner\">\n\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t<span class=\"toggle-text\">{search_text}</span>\n\t\t\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t\t</button><!-- .search-toggle -->\n\n\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t\t",
                svg("search")
            ));
        }
        nav.push_str("\n\t\t\t\t\t\t</div><!-- .header-toggles -->\n\t\t\t\t\t\t");
    }
    let modal = if search { modal_search(ctx) } else { String::new() };
    let menu_modal = modal_menu(ctx).await?;
    ctx.env.with_state(|s| s.media.in_header = false);
    Ok(format!(
        "\n\t\t<header id=\"site-header\" class=\"header-footer-group\">\n\n\t\t\t<div class=\"header-inner section-inner\">\n\n\t\t\t\t<div class=\"header-titles-wrapper\">\n\n\t\t\t\t\t{mobile_search}\n\t\t\t\t\t<div class=\"header-titles\">\n\n\t\t\t\t\t\t{logo}{description}\n\t\t\t\t\t</div><!-- .header-titles -->\n\n\t\t\t\t\t<button class=\"toggle nav-toggle mobile-nav-toggle\" data-toggle-target=\".menu-modal\"  data-toggle-body-class=\"showing-menu-modal\" aria-expanded=\"false\" data-set-focus=\".close-nav-toggle\">\n\t\t\t\t\t\t<span class=\"toggle-inner\">\n\t\t\t\t\t\t\t<span class=\"toggle-icon\">\n\t\t\t\t\t\t\t\t{ellipsis}\t\t\t\t\t\t\t</span>\n\t\t\t\t\t\t\t<span class=\"toggle-text\">{menu_text}</span>\n\t\t\t\t\t\t</span>\n\t\t\t\t\t</button><!-- .nav-toggle -->\n\n\t\t\t\t</div><!-- .header-titles-wrapper -->\n\n\t\t\t\t<div class=\"header-navigation-wrapper\">\n\n\t\t\t\t\t{nav}\n\t\t\t\t</div><!-- .header-navigation-wrapper -->\n\n\t\t\t</div><!-- .header-inner -->\n\n\t\t\t{modal}\n\t\t</header><!-- #site-header -->\n\n\t\t{menu_modal}",
        ellipsis = svg("ellipsis"),
    ))
}

// ------------------------------------------------------------------- the loop

/// `get_query_var( 'page' )` as `WP_Query::generate_postdata()` takes it.
fn current_page(ctx: &Ctx<'_>) -> u64 {
    ctx.q.vars.u64("page").filter(|page| *page != 0).unwrap_or(1)
}

/// `post_class()`.
async fn classes_of(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let q = ctx.q;
    let classes = post_class(&ctx.env, post, &[], q.flags.is_home && !q.flags.is_paged, q.flags.is_singular()).await?;
    Ok(esc_attr(&classes.join(" ")))
}

/// `twentytwenty_read_more_tag()` on `the_content_more_link`: the link of `the_content()` as a button.
pub fn read_more_tag(permalink: &str, post_id: u64, title: &str, text: &str) -> String {
    format!(" <div class=\"read-more-button-wrap\"><a href=\"{permalink}#more-{post_id}\" class=\"more-link\"><span class=\"faux-button\">{text}</span> <span class=\"screen-reader-text\">\"{title}\"</span></a></div>")
}

/// `post_type_supports( $type, 'author' )`.
fn supports_author(ctx: &Ctx<'_>, post: &Post) -> bool {
    match post.post_type.as_str() {
        "post" | "page" | "attachment" => true,
        other => ctx.site.types.post_type(other).is_some_and(|t| t.supports.as_ref().is_some_and(|s| s.iter().any(|s| s == "author"))),
    }
}

/// `has_category()` and `has_tag()`.
async fn has_terms(ctx: &Ctx<'_>, post: &Post, taxonomy: &str) -> Result<bool> {
    Ok(!ctx.site.db.terms_for_post(post.id, Some(taxonomy)).await?.is_empty())
}

/// `get_edit_post_link()`: empty for who may not edit the post.
fn edit_post_url(ctx: &Ctx<'_>, post: &Post) -> Option<String> {
    if !ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.can_edit_post(post)) {
        return None;
    }
    Some(ctx.site.site_url(&format!("wp-admin/post.php?post={}&action=edit", post.id)))
}

/// `edit_post_link()` through `twentytwenty_edit_post_link()`.
fn edit_link(ctx: &Ctx<'_>, post: &Post) -> String {
    let Some(url) = edit_post_url(ctx, post) else { return String::new() };
    let text = sprintf(&rustpress_core::kses::kses_post(&t(ctx, "Edit <span class=\"screen-reader-text\">%s</span>")), &[&the_title_for(ctx.site, post)]);
    format!(
        "<div class=\"post-meta-wrapper post-meta-edit-link-wrapper\"><ul class=\"post-meta\"><li class=\"post-edit meta-wrapper\"><span class=\"meta-icon\">{}</span><span class=\"meta-text\"><a href=\"{}\">{text}</a></span></li></ul><!-- .post-meta --></div><!-- .post-meta-wrapper -->",
        svg("edit"),
        esc_url(&url)
    )
}

/// `comments_popup_link()` with its defaults.
async fn comments_popup_link(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let site = ctx.site;
    let viewer = ctx.q.viewer.as_ref();
    let number = site.comments_number(post, viewer);
    let title = the_title_for(site, post);
    let permalink = site.permalink(post).await?;
    if number == 0 && !site.comments_open(post, viewer) && !site.pings_open(post, viewer) {
        return Ok(format!("<span>{}</span>", sprintf(&site.tr("Comments Off<span class=\"screen-reader-text\"> on %s</span>"), &[&title])));
    }
    if rustpress_blocks::content::post_password_required(post) {
        return Ok(site.tr("Enter your password to view comments."));
    }
    let href = if number == 0 { format!("{permalink}#respond") } else { esc_url(&format!("{permalink}#comments")) };
    let text = if number > 1 {
        let more = site.i18n().ngettext("default", "%1$s Comment<span class=\"screen-reader-text\"> on %2$s</span>", "%1$s Comments<span class=\"screen-reader-text\"> on %2$s</span>", number);
        sprintf(&more, &[&site.number_i18n(number as u64), &title])
    } else if number == 0 {
        sprintf(&site.tr("No Comments<span class=\"screen-reader-text\"> on %s</span>"), &[&title])
    } else {
        sprintf(&site.tr("1 Comment<span class=\"screen-reader-text\"> on %s</span>"), &[&title])
    };
    Ok(format!("<a href=\"{href}\">{text}</a>"))
}

/// One item of the post meta list.
fn meta_item(class: &str, screen_reader: Option<&str>, icon: &str, text: &str) -> String {
    let label = match screen_reader {
        Some(label) => format!("\n\t\t\t\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t\t\t\t{label}\t\t\t\t\t\t\t</span>"),
        None => String::new(),
    };
    format!("\t\t\t\t\t<li class=\"{class} meta-wrapper\">\n\t\t\t\t\t\t<span class=\"meta-icon\">{label}\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</span>\n\t\t\t\t\t\t<span class=\"meta-text\">\n\t\t\t\t\t\t\t{text}\t\t\t\t\t\t</span>\n\t\t\t\t\t</li>\n\t\t\t\t\t", svg(icon))
}

/// `twentytwenty_get_post_meta()` for `single-top` or `single-bottom`.
async fn post_meta(ctx: &Ctx<'_>, post: &Post, top: bool) -> Result<String> {
    if post.post_type == "page" {
        return Ok(String::new());
    }
    let site = ctx.site;
    let mut items = String::new();
    if top {
        if supports_author(ctx, post) {
            let (url, name) = match site.db.user(post.post_author).await? {
                Some(user) => (site.author_url(&user), user.display_name),
                None => (if site.uses_pretty_permalinks() { site.home_url(&site.user_trailingslashit("/author/")) } else { site.home_url(&format!("?author={}", post.post_author)) }, String::new()),
            };
            let by = sprintf(&t(ctx, "By %s"), &[&format!("<a href=\"{}\">{}</a>", esc_url(&url), esc_html(&name))]);
            items.push_str(&meta_item("post-author", Some(&t(ctx, "Post author")), "user", &by));
        }
        let posted = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
        let date = format!("<a href=\"{}\">{}</a>\n", esc_url(&site.permalink(post).await?), site.date(&site.options.string("date_format", "F j, Y"), posted));
        items.push_str(&meta_item("post-date", Some(&t(ctx, "Post date")), "calendar", &date));
        let viewer = ctx.q.viewer.as_ref();
        if !rustpress_blocks::content::post_password_required(post) && (site.comments_open(post, viewer) || site.comments_number(post, viewer) != 0) {
            items.push_str(&meta_item("post-comment-link", None, "comment", &comments_popup_link(ctx, post).await?));
        }
        if post.post_type == "post" && site.settings.sticky_posts.contains(&post.id) {
            items.push_str(&meta_item("post-sticky", None, "bookmark", &t(ctx, "Sticky post")));
        }
    } else if has_terms(ctx, post, "post_tag").await? {
        items.push_str(&meta_item("post-tags", Some(&t(ctx, "Tags")), "tag", &tags::tag_list(site, post, ", ").await?));
    }
    if items.is_empty() {
        return Ok(String::new());
    }
    let classes = if top { " post-meta-single post-meta-single-top" } else { " post-meta-single post-meta-single-bottom" };
    Ok(format!("\n\t\t<div class=\"post-meta-wrapper{classes}\">\n\n\t\t\t<ul class=\"post-meta\">\n\n\t\t\t\t{items}\n\t\t\t</ul><!-- .post-meta -->\n\n\t\t</div><!-- .post-meta-wrapper -->\n\n\t\t"))
}

/// The categories of the entry header (`the_category( ' ' )`).
async fn entry_categories(ctx: &Ctx<'_>, post: &Post, indent: &str) -> Result<String> {
    if !has_terms(ctx, post, "category").await? {
        return Ok(String::new());
    }
    Ok(format!(
        "\n{indent}<div class=\"entry-categories\">\n{indent}\t<span class=\"screen-reader-text\">\n{indent}\t\t{}{indent}\t</span>\n{indent}\t<div class=\"entry-categories-inner\">\n{indent}\t\t{}{indent}\t</div><!-- .entry-categories-inner -->\n{indent}</div><!-- .entry-categories -->\n\n{indent}",
        t(ctx, "Categories"),
        tags::category_list(ctx.site, post, " ").await?
    ))
}

/// `the_excerpt()`.
async fn the_excerpt(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let excerpt = rustpress_blocks::excerpt::get_the_excerpt(&ctx.env, post).await?;
    Ok(rustpress_core::formatting::wpautop(&rustpress_core::formatting::convert_chars(&ctx.site.texturize(&excerpt))))
}

/// `has_excerpt()`.
fn has_excerpt(post: &Post) -> bool {
    !post.post_excerpt.is_empty()
}

/// `template-parts/entry-header.php`.
async fn entry_header(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let singular = ctx.q.flags.is_singular();
    let title = the_title_for(ctx.site, post);
    let heading = if title.is_empty() {
        String::new()
    } else if singular {
        format!("<h1 class=\"entry-title\">{title}</h1>")
    } else {
        format!("<h2 class=\"entry-title heading-size-1\"><a href=\"{}\">{title}</a></h2>", esc_url(&ctx.site.permalink(post).await?))
    };
    let excerpt = if has_excerpt(post) && singular { format!("\n\t\t\t<div class=\"intro-text section-inner max-percentage small\">\n\t\t\t\t{}\t\t\t</div>\n\n\t\t\t", the_excerpt(ctx, post).await?) } else { String::new() };
    Ok(format!(
        "\n<header class=\"entry-header has-text-align-center{}\">\n\n\t<div class=\"entry-header-inner section-inner medium\">\n\n\t\t{}{heading}{excerpt}{}\n\t</div><!-- .entry-header-inner -->\n\n</header><!-- .entry-header -->\n",
        if singular { " header-footer-group" } else { "" },
        entry_categories(ctx, post, "\t\t\t").await?,
        post_meta(ctx, post, true).await?,
    ))
}

/// `the_post_thumbnail()`.
async fn post_thumbnail_html(ctx: &Ctx<'_>, post: &Post, id: u64) -> Result<String> {
    let args = ImageArgs { post_thumbnail: true, thumbnail_of: Some(post.id), ..ImageArgs::default() };
    Ok(images::get_attachment_image(&ctx.env, id, "post-thumbnail", &args).await?)
}

/// `template-parts/featured-image.php`.
async fn featured_image(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let id = images::post_thumbnail_id(&ctx.env, post.id).await?;
    if id == 0 || rustpress_blocks::content::post_password_required(post) {
        return Ok(String::new());
    }
    let image = post_thumbnail_html(ctx, post, id).await?;
    // `get_the_post_thumbnail_caption()`.
    let caption = match ctx.site.db.post(id).await? {
        Some(attachment) if attachment.post_type == "attachment" => attachment.post_excerpt,
        _ => String::new(),
    };
    let caption = if caption.is_empty() { String::new() } else { format!("\n\t\t\t\t<figcaption class=\"wp-caption-text\">{}</figcaption>\n\n\t\t\t\t", rustpress_core::kses::kses_post(&caption)) };
    Ok(format!(
        "\n\t<figure class=\"featured-media\">\n\n\t\t<div class=\"featured-media-inner section-inner{}\">\n\n\t\t\t{image}{caption}\n\t\t</div><!-- .featured-media-inner -->\n\n\t</figure><!-- .featured-media -->\n\n\t",
        if ctx.q.flags.is_singular() { "" } else { " medium" }
    ))
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
    let mut out = format!("<nav class=\"post-nav-links bg-light-background\" aria-label=\"{}\"><span class=\"label\">{}</span>", esc_attr(&t(ctx, "Page")), t(ctx, "Pages:"));
    for number in 1..=pages.len() as u64 {
        let label = format!("<span class=\"page-number\">{number}</span>");
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

/// `template-parts/entry-author-bio.php`.
async fn author_bio(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let description = ctx.site.db.user_meta(post.post_author, "description").await?.unwrap_or_default();
    if description.is_empty() || description == "0" || !mod_on(ctx, "show_author_bio") {
        return Ok(String::new());
    }
    let user = ctx.site.db.user(post.post_author).await?;
    let (name, email, url) = match &user {
        Some(user) => (user.display_name.clone(), user.user_email.clone(), ctx.site.author_url(user)),
        None => (String::new(), String::new(), String::new()),
    };
    let avatar = rustpress_blocks::content::get_avatar(&ctx.env, &email, 160, "", "", "");
    Ok(format!(
        "<div class=\"author-bio\">\n\t<div class=\"author-title-wrapper\">\n\t\t<div class=\"author-avatar vcard\">\n\t\t\t{avatar}\t\t</div>\n\t\t<h2 class=\"author-title heading-size-4\">\n\t\t\t{}\t\t</h2>\n\t</div><!-- .author-name -->\n\t<div class=\"author-description\">\n\t\t{}\t\t<a class=\"author-link\" href=\"{}\" rel=\"author\">\n\t\t\t{}\t\t</a>\n\t</div><!-- .author-description -->\n</div><!-- .author-bio -->\n\t",
        sprintf(&t(ctx, "By %s"), &[&esc_html(&name)]),
        rustpress_core::kses::kses_post(&rustpress_core::formatting::wpautop(&description)),
        esc_url(&url),
        t(ctx, "View Archive <span aria-hidden=\"true\">&rarr;</span>"),
    ))
}

/// `get_next_post()` or `get_previous_post()`.
async fn adjacent_post(ctx: &Ctx<'_>, post: &Post, previous: bool) -> Result<Option<Post>> {
    if post.post_type == "attachment" {
        // Attachments are not published, so `get_adjacent_post()` finds none.
        return Ok(None);
    }
    Ok(match ctx.site.db.adjacent_post_id(&post.post_type, &post.post_date, post.id, previous).await? {
        Some(id) => ctx.site.db.post(id).await?.filter(|other| other.post_status == "publish"),
        None => None,
    })
}

/// `template-parts/navigation.php`.
async fn navigation(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let next = adjacent_post(ctx, post, false).await?;
    let prev = adjacent_post(ctx, post, true).await?;
    if next.is_none() && prev.is_none() {
        return Ok(String::new());
    }
    let classes = if next.is_none() {
        " only-one only-prev"
    } else if prev.is_none() {
        " only-one only-next"
    } else {
        ""
    };
    let site = ctx.site;
    let mut out = format!(
        "\n\t<nav class=\"pagination-single section-inner{classes}\" aria-label=\"{}\">\n\n\t\t<hr class=\"styled-separator is-style-wide\" aria-hidden=\"true\" />\n\n\t\t<div class=\"pagination-single-inner\">\n\n\t\t\t",
        esc_attr(&t(ctx, "Post"))
    );
    if let Some(prev) = &prev {
        out.push_str(&format!(
            "\n\t\t\t\t<a class=\"previous-post\" href=\"{}\">\n\t\t\t\t\t<span class=\"arrow\" aria-hidden=\"true\">&larr;</span>\n\t\t\t\t\t<span class=\"title\"><span class=\"title-inner\">{}</span></span>\n\t\t\t\t</a>\n\n\t\t\t\t",
            esc_url(&site.permalink(prev).await?),
            rustpress_core::kses::kses_post(&the_title_for(site, prev))
        ));
    }
    if let Some(next) = &next {
        out.push_str(&format!(
            "\n\t\t\t\t<a class=\"next-post\" href=\"{}\">\n\t\t\t\t\t<span class=\"arrow\" aria-hidden=\"true\">&rarr;</span>\n\t\t\t\t\t\t<span class=\"title\"><span class=\"title-inner\">{}</span></span>\n\t\t\t\t</a>\n\t\t\t\t",
            esc_url(&site.permalink(next).await?),
            rustpress_core::kses::kses_post(&the_title_for(site, next))
        ));
    }
    out.push_str("\n\t\t</div><!-- .pagination-single-inner -->\n\n\t\t<hr class=\"styled-separator is-style-wide\" aria-hidden=\"true\" />\n\n\t</nav><!-- .pagination-single -->\n\n\t");
    Ok(out)
}

/// `comments.php`.
async fn comments_template(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    if rustpress_blocks::content::post_password_required(post) {
        return Ok(String::new());
    }
    let site = ctx.site;
    let env = &ctx.env;
    let viewer = ctx.q.viewer.as_ref();
    let page = rustpress_blocks::comment_pages::comments_for_template(env, post.id).await?;
    let request_uri = ctx.request_uri();
    let mut out = String::new();
    let has_comments = !page.comments.is_empty();
    if has_comments {
        let title = the_title_for(site, post);
        let heading = if site.comments_number_is_one(post, viewer) {
            sprintf(&tx(ctx, "One reply on &ldquo;%s&rdquo;", "comments title"), &[&title])
        } else {
            let count = site.comments_number(post, viewer);
            let template = site.i18n().npgettext(THEME, "comments title", "%1$s reply on &ldquo;%2$s&rdquo;", "%1$s replies on &ldquo;%2$s&rdquo;", count);
            sprintf(&template, &[&site.number_i18n(count.max(0) as u64), &title])
        };
        let texts = rustpress_blocks::legacy_comments::TwentyTwentyTexts {
            says: t(ctx, "says:"),
            date_at_time: t(ctx, "%1$s at %2$s"),
            edit: t(ctx, "Edit"),
            awaiting: t(ctx, "Your comment is awaiting moderation."),
            by_post_author: t(ctx, "By Post Author"),
        };
        let args = rustpress_blocks::legacy_comments::ListArgs { avatar_size: 120, reply_text: site.tr("Reply"), short_ping: false, cpage: Some(page.page), div_style: true, twentytwenty: Some(texts) };
        let list = rustpress_blocks::legacy_comments::list_comments(site, env, post, &page.comments, &args).await?;
        let links = tags::paginate_comments_links(
            site,
            &site.permalink(post).await?,
            &request_uri,
            page.page,
            page.max_pages,
            &format!("<span aria-hidden=\"true\">&larr;</span> {}", t(ctx, "Older Comments")),
            &format!("{} <span aria-hidden=\"true\">&rarr;</span>", t(ctx, "Newer Comments")),
            0,
            "",
        );
        let pagination = if links.is_empty() {
            String::new()
        } else {
            format!(
                "\n\t\t\t\t<nav class=\"comments-pagination pagination{}\" aria-label=\"{}\">\n\t\t\t\t\t{}\t\t\t\t</nav>\n\n\t\t\t\t",
                if links.contains("prev page-numbers") { "" } else { " only-next" },
                esc_attr(&t(ctx, "Comments")),
                rustpress_core::kses::kses_post(&links)
            )
        };
        out.push_str(&format!(
            "\n\t<div class=\"comments\" id=\"comments\">\n\n\t\t\n\t\t<div class=\"comments-header section-inner small max-percentage\">\n\n\t\t\t<h2 class=\"comment-reply-title\">\n\t\t\t{heading}\t\t\t</h2><!-- .comments-title -->\n\n\t\t</div><!-- .comments-header -->\n\n\t\t<div class=\"comments-inner section-inner thin max-percentage\">\n\n\t\t\t{list}{pagination}\n\t\t</div><!-- .comments-inner -->\n\n\t</div><!-- comments -->\n\n\t"
        ));
    }
    let separator = if has_comments { "<hr class=\"styled-separator is-style-wide\" aria-hidden=\"true\" />" } else { "" };
    if site.comments_open(post, viewer) || site.pings_open(post, viewer) {
        out.push_str(separator);
        let form = rustpress_blocks::legacy_comments::FormArgs {
            title_reply_before: "<h2 id=\"reply-title\" class=\"comment-reply-title\">".into(),
            title_reply_after: "</h2>".into(),
            class_form: "section-inner thin max-percentage".into(),
            ..Default::default()
        };
        out.push_str(&rustpress_blocks::legacy_comments::comment_form_with(site, viewer, post, &request_uri, false, &form).await?);
    } else if ctx.q.flags.is_single {
        out.push_str(separator);
        out.push_str(&format!("\n\t<div class=\"comment-respond\" id=\"respond\">\n\n\t\t<p class=\"comments-closed\">{}</p>\n\n\t</div><!-- #respond -->\n\n\t", t(ctx, "Comments are closed.")));
    }
    Ok(out)
}

/// What `template-parts/content.php` and `content-cover.php` print after the content: the page links, the edit link,
/// the post meta at the bottom and the author's biography.
async fn after_content(ctx: &Ctx<'_>, post: &Post) -> Result<String> {
    let mut out = link_pages(ctx, post).await?;
    out.push_str(&edit_link(ctx, post));
    out.push_str(&post_meta(ctx, post, false).await?);
    if supports_author(ctx, post) && ctx.q.flags.is_single {
        out.push_str(&author_bio(ctx, post).await?);
    }
    Ok(out)
}

/// The navigation between posts and the comments at the end of `template-parts/content.php` and `content-cover.php`.
async fn article_end(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let f = &ctx.q.flags;
    let site = ctx.site;
    let viewer = ctx.q.viewer.as_ref();
    let mut out = String::new();
    if f.is_single {
        out.push_str(&navigation(ctx, post).await?);
    }
    if (f.is_single || f.is_page) && (site.comments_open(post, viewer) || site.comments_number(post, viewer) != 0) && !rustpress_blocks::content::post_password_required(post) {
        out.push_str(&format!("\n\t\t<div class=\"comments-wrapper section-inner\">\n\n\t\t\t{}\n\t\t</div><!-- .comments-wrapper -->\n\n\t\t", comments_template(ctx, post).await?));
    }
    Ok(out)
}

/// `get_page_template_slug()` of the queried post (`''` for the default template).
async fn page_template(ctx: &Ctx<'_>) -> Result<String> {
    let Queried::Post(post) = &ctx.q.queried else { return Ok(String::new()) };
    if !ctx.q.flags.is_singular() {
        return Ok(String::new());
    }
    let slug = ctx.site.db.post_meta_value(post.id, "_wp_page_template").await?.unwrap_or_default();
    Ok(if slug == "default" { String::new() } else { slug })
}

/// Whether the page template file is one of the theme's (`locate_template()` finds it).
fn is_theme_template(slug: &str) -> bool {
    matches!(slug, "templates/template-cover.php" | "templates/template-full-width.php")
}

/// `template-parts/content.php`.
async fn content_part(ctx: &Ctx<'_>, post: &Arc<Post>, full_width: bool) -> Result<String> {
    let f = &ctx.q.flags;
    let classes = classes_of(ctx, post).await?;
    let header = entry_header(ctx, post).await?;
    let featured = if f.is_search { String::new() } else { featured_image(ctx, post).await? };
    let content = if f.is_search || (!f.is_singular() && ctx.mod_str("blog_content").as_deref() == Some("summary")) {
        the_excerpt(ctx, post).await?
    } else {
        let raw = get_the_content(&ctx.env, post, current_page(ctx), f.is_singular()).await?;
        the_content(&ctx.env, post, &raw).await?
    };
    let after = after_content(ctx, post).await?;
    let end = article_end(ctx, post).await?;
    Ok(format!(
        "\n<article class=\"{classes}\" id=\"post-{id}\">\n\n\t{header}{featured}\n\t<div class=\"post-inner {}\">\n\n\t\t<div class=\"entry-content\">\n\n\t\t\t{content}\n\t\t</div><!-- .entry-content -->\n\n\t</div><!-- .post-inner -->\n\n\t<div class=\"section-inner\">\n\t\t{after}\n\t</div><!-- .section-inner -->\n\n\t{end}\n</article><!-- .post -->\n",
        if full_width { " " } else { "thin " },
        id = post.id,
    ))
}

/// `template-parts/content-cover.php`.
async fn cover_part(ctx: &Ctx<'_>, post: &Arc<Post>) -> Result<String> {
    let site = ctx.site;
    let f = &ctx.q.flags;
    let classes = classes_of(ctx, post).await?;
    let mut header_style = String::new();
    let mut header_classes = String::new();
    if !rustpress_blocks::content::post_password_required(post) {
        let id = images::post_thumbnail_id(&ctx.env, post.id).await?;
        if id != 0
            && let Some(url) = images::attachment_image_url(&ctx.env, id, "twentytwenty-fullscreen").await?
        {
            header_style = format!(" style=\"background-image: url( {} );\"", esc_url(&url));
            header_classes = " bg-image".into();
        }
    }
    let overlay_color = ctx.mod_str("cover_template_overlay_background_color").filter(|c| !c.is_empty() && c != "0");
    let overlay_style = overlay_color.map(|c| format!(" style=\"color: {};\"", esc_attr(&c))).unwrap_or_default();
    if mod_on(ctx, "cover_template_fixed_background") {
        header_classes.push_str(" bg-attachment-fixed");
    }
    let opacity = match ctx.mods.get("cover_template_overlay_opacity") {
        None | Some(Value::Bool(false)) => "80".to_owned(),
        Some(_) => ctx.mod_str("cover_template_overlay_opacity").unwrap_or_default(),
    };
    let title = the_title_for(site, post);
    let heading = if title.is_empty() { String::new() } else { format!("<h1 class=\"entry-title\">{title}</h1>") };
    let below = if f.is_page {
        format!(
            "\n\t\t\t\t\t\t\t\t<div class=\"to-the-content-wrapper\">\n\n\t\t\t\t\t\t\t\t\t<a href=\"#post-inner\" class=\"to-the-content fill-children-current-color\">\n\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t<div class=\"screen-reader-text\">\n\t\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t\t</div>\n\t\t\t\t\t\t\t\t\t</a><!-- .to-the-content -->\n\n\t\t\t\t\t\t\t\t</div><!-- .to-the-content-wrapper -->\n\n\t\t\t\t\t\t\t\t",
            svg("arrow-down"),
            t(ctx, "Scroll Down")
        )
    } else {
        let mut below = String::new();
        if has_excerpt(post) {
            below.push_str(&format!(
                "\n\t\t\t\t\t\t\t\t\t<div class=\"intro-text section-inner max-percentage{}\">\n\t\t\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t\t\t\t",
                esc_attr(if f.is_singular() { " small" } else { " thin" }),
                the_excerpt(ctx, post).await?
            ));
        }
        below.push_str(&post_meta(ctx, post, true).await?);
        below
    };
    let raw = get_the_content(&ctx.env, post, current_page(ctx), f.is_singular()).await?;
    let content = the_content(&ctx.env, post, &raw).await?;
    let after = after_content(ctx, post).await?;
    let end = article_end(ctx, post).await?;
    Ok(format!(
        "\n<article class=\"{classes}\" id=\"post-{id}\">\n\t\n\t<div class=\"cover-header {header_classes}\"{header_style}>\n\t\t<div class=\"cover-header-inner-wrapper screen-height\">\n\t\t\t<div class=\"cover-header-inner\">\n\t\t\t\t<div class=\"cover-color-overlay color-accent{}\"{overlay_style}></div>\n\n\t\t\t\t\t<header class=\"entry-header has-text-align-center\">\n\t\t\t\t\t\t<div class=\"entry-header-inner section-inner medium\">\n\n\t\t\t\t\t\t\t{}{heading}{below}\n\t\t\t\t\t\t</div><!-- .entry-header-inner -->\n\t\t\t\t\t</header><!-- .entry-header -->\n\n\t\t\t</div><!-- .cover-header-inner -->\n\t\t</div><!-- .cover-header-inner-wrapper -->\n\t</div><!-- .cover-header -->\n\n\t<div class=\"post-inner\" id=\"post-inner\">\n\n\t\t<div class=\"entry-content\">\n\n\t\t{content}\n\t\t</div><!-- .entry-content -->\n\t\t{after}\n\t</div><!-- .post-inner -->\n\n\t{end}\n</article><!-- .post -->\n",
        esc_attr(&format!(" opacity-{opacity}")),
        entry_categories(ctx, post, "\t\t\t\t\t\t\t\t").await?,
        id = post.id,
    ))
}

/// `template-parts/pagination.php`.
fn pagination(ctx: &Ctx<'_>) -> String {
    let desc = ctx.q.vars.get("order").map(str::to_uppercase).is_none_or(|order| order != "ASC");
    let newer = t(ctx, "Newer <span class=\"nav-short\">Posts</span>");
    let older = t(ctx, "Older <span class=\"nav-short\">Posts</span>");
    let prev_text = format!("<span aria-hidden=\"true\">&larr;</span> <span class=\"nav-prev-text\">{}</span>", if desc { &newer } else { &older });
    let next_text = format!("<span class=\"nav-next-text\">{}</span> <span aria-hidden=\"true\">&rarr;</span>", if desc { &older } else { &newer });
    let mut links = tags::posts_pagination(ctx.site, ctx.q, &Pagination { prev_text: prev_text.clone(), next_text: next_text.clone(), before_page_number: String::new(), mid_size: 1 });
    if !links.contains("prev page-numbers") {
        links = links.replace("<div class=\"nav-links\">", &format!("<div class=\"nav-links\"><span class=\"prev page-numbers placeholder\" aria-hidden=\"true\">{prev_text}</span>"));
    }
    if !links.contains("next page-numbers") {
        links = links.replace("</div>", &format!("<span class=\"next page-numbers placeholder\" aria-hidden=\"true\">{next_text}</span></div>"));
    }
    if links.is_empty() {
        return String::new();
    }
    format!("\n\t<div class=\"pagination-wrapper section-inner\">\n\n\t\t<hr class=\"styled-separator pagination-separator is-style-wide\" aria-hidden=\"true\" />\n\n\t\t{links}\n\t</div><!-- .pagination-wrapper -->\n\n\t")
}

/// `get_the_archive_title()` with `twentytwenty_get_the_archive_title()`, and `get_the_archive_description()`.
async fn archive_title(ctx: &Ctx<'_>) -> Result<(String, String)> {
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
                sprintf(&site.trx("%s:", "taxonomy term archive title prefix"), &[&site.taxonomy_singular_label(&term.taxonomy)])
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
    // `/(\A[^\:]+\:)/`: the text up to the first colon, if there is one.
    let title = match title.find(':') {
        Some(at) if at > 0 => format!("<span class=\"color-accent\">{}</span>{}", &title[..=at], &title[at + 1..]),
        _ => title,
    };
    Ok((title, description))
}

/// `index.php`, `singular.php` and the page templates, between `get_header()` and `get_footer()`.
async fn main_template(ctx: &Ctx<'_>) -> Result<String> {
    let q = ctx.q;
    let f = &q.flags;
    let site = ctx.site;
    let mut out = String::from("\n<main id=\"site-content\">\n\n\t");
    if f.is_404 {
        // `404.php`.
        out.push_str(&format!(
            "<div class=\"section-inner thin error404-content\">\n\n\t\t<h1 class=\"entry-title\">{}</h1>\n\n\t\t<div class=\"intro-text\"><p>{}</p></div>\n\n\t\t{}\n\t</div><!-- .section-inner -->\n\n</main><!-- #site-content -->\n\n",
            t(ctx, "Page Not Found"),
            t(ctx, "The page you were looking for could not be found. It might have been removed, renamed, or did not exist in the first place."),
            search_form_markup(site, q, &ctx.env, &t(ctx, "404 not found"))
        ));
        return Ok(out);
    }
    ctx.env.with_state(|s| s.media.in_main_loop = true);
    if f.is_singular() {
        // `singular.php`, `templates/template-cover.php` and `templates/template-full-width.php`.
        let template = page_template(ctx).await?;
        let template = if is_theme_template(&template) { template } else { String::new() };
        for post in &q.posts {
            let post = Arc::new(post.clone());
            ctx.env.with_state(|s| s.looped_posts.push(post.id));
            if template == "templates/template-cover.php" {
                out.push_str(&cover_part(ctx, &post).await?);
            } else {
                out.push_str(&content_part(ctx, &post, template == "templates/template-full-width.php").await?);
            }
        }
        ctx.env.with_state(|s| s.media.in_main_loop = false);
        out.push_str("\n</main><!-- #site-content -->\n\n");
        return Ok(out);
    }
    let (title, subtitle) = if f.is_search {
        let title = format!("<span class=\"color-accent\">{}</span> &ldquo;{}&rdquo;", t(ctx, "Search:"), esc_html(q.vars.get("s").unwrap_or("")));
        let found = q.found_posts;
        let subtitle = if found > 0 {
            sprintf(&site.i18n().ngettext(THEME, "We found %s result for your search.", "We found %s results for your search.", found as i64), &[&site.number_i18n(found)])
        } else {
            t(ctx, "We could not find any results for your search. You can give it another try through the search form below.")
        };
        (title, subtitle)
    } else if f.is_archive && q.posts.is_empty() {
        (t(ctx, "Nothing Found"), String::new())
    } else if !f.is_home {
        archive_title(ctx).await?
    } else {
        (String::new(), String::new())
    };
    if !title.is_empty() || !subtitle.is_empty() {
        out.push_str("\n\t\t<header class=\"archive-header has-text-align-center header-footer-group\">\n\n\t\t\t<div class=\"archive-header-inner section-inner medium\">\n\n\t\t\t\t");
        if !title.is_empty() {
            out.push_str(&format!("\t\t\t\t\t<h1 class=\"archive-title\">{}</h1>\n\t\t\t\t", rustpress_core::kses::kses_post(&title)));
        }
        out.push_str("\n\t\t\t\t");
        if !subtitle.is_empty() {
            out.push_str(&format!(
                "\t\t\t\t\t<div class=\"archive-subtitle section-inner thin max-percentage intro-text\">{}</div>\n\t\t\t\t",
                rustpress_core::kses::kses_post(&rustpress_core::formatting::wpautop(&subtitle))
            ));
        }
        out.push_str("\n\t\t\t</div><!-- .archive-header-inner -->\n\n\t\t</header><!-- .archive-header -->\n\n\t\t");
    }
    for (i, post) in q.posts.iter().enumerate() {
        if i > 0 {
            out.push_str("<hr class=\"post-separator styled-separator is-style-wide section-inner\" aria-hidden=\"true\" />");
        }
        let post = Arc::new(post.clone());
        ctx.env.with_state(|s| s.looped_posts.push(post.id));
        out.push_str(&content_part(ctx, &post, false).await?);
    }
    ctx.env.with_state(|s| s.media.in_main_loop = false);
    if q.posts.is_empty() && f.is_search {
        out.push_str(&format!(
            "\n\t\t<div class=\"no-search-results-form section-inner thin\">\n\n\t\t\t{}\n\t\t</div><!-- .no-search-results -->\n\n\t\t",
            search_form_markup(site, q, &ctx.env, &t(ctx, "search again"))
        ));
    }
    out.push_str("\n\t");
    out.push_str(&pagination(ctx));
    out.push_str("\n</main><!-- #site-content -->\n\n");
    Ok(out)
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

/// `template-parts/footer-menus-widgets.php`.
async fn footer_menus_widgets(ctx: &Ctx<'_>) -> Result<String> {
    let (footer_menu, social) = (has_menu(ctx, "footer").await?, has_menu(ctx, "social").await?);
    let (first, second) = (active_sidebar(ctx, "sidebar-1"), active_sidebar(ctx, "sidebar-2"));
    if !(footer_menu || social || first || second) {
        return Ok(String::new());
    }
    let mut out = String::from("\n\t<div class=\"footer-nav-widgets-wrapper header-footer-group\">\n\n\t\t<div class=\"footer-inner section-inner\">\n\n\t\t\t");
    if footer_menu || social {
        let classes = format!("{}{}", if footer_menu { " has-footer-menu" } else { "" }, if social { " has-social-menu" } else { "" });
        out.push_str(&format!("\t\t\t\t<div class=\"footer-top{classes}\">\n\t\t\t\t\t"));
        if footer_menu {
            out.push_str(&format!(
                "\n\t\t\t\t\t\t<nav aria-label=\"{}\" class=\"footer-menu-wrapper\">\n\n\t\t\t\t\t\t\t<ul class=\"footer-menu reset-list-style\">\n\t\t\t\t\t\t\t\t{}\t\t\t\t\t\t\t</ul>\n\n\t\t\t\t\t\t</nav><!-- .site-nav -->\n\n\t\t\t\t\t",
                esc_attr(&t(ctx, "Footer")),
                nav_menu(ctx, "footer", false).await?
            ));
        }
        out.push_str("\t\t\t\t\t");
        if social {
            out.push_str(&format!(
                "\n\t\t\t\t\t\t<nav aria-label=\"{}\" class=\"footer-social-wrapper\">\n\n\t\t\t\t\t\t\t<ul class=\"social-menu footer-social reset-list-style social-icons fill-children-current-color\">\n\n\t\t\t\t\t\t\t\t{}\n\t\t\t\t\t\t\t</ul><!-- .footer-social -->\n\n\t\t\t\t\t\t</nav><!-- .footer-social-wrapper -->\n\n\t\t\t\t\t",
                esc_attr(&t(ctx, "Social links")),
                nav_menu(ctx, "social", false).await?
            ));
        }
        out.push_str("\t\t\t\t</div><!-- .footer-top -->\n\n\t\t\t");
    }
    out.push_str("\n\t\t\t");
    if first || second {
        let request_uri = ctx.request_uri();
        let search = || search_form_markup(ctx.site, ctx.q, &ctx.env, "");
        out.push_str("\n\t\t\t\t<aside class=\"footer-widgets-outer-wrapper\">\n\n\t\t\t\t\t<div class=\"footer-widgets-wrapper\">\n\n\t\t\t\t\t\t");
        if first {
            let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
            let html = widgets::dynamic_sidebar(&services, &sidebar("sidebar-1")).await?;
            out.push_str(&format!("\n\t\t\t\t\t\t\t<div class=\"footer-widgets column-one grid-item\">\n\t\t\t\t\t\t\t\t{html}\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t"));
        }
        out.push_str("\n\t\t\t\t\t\t");
        if second {
            let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
            let html = widgets::dynamic_sidebar(&services, &sidebar("sidebar-2")).await?;
            out.push_str(&format!("\n\t\t\t\t\t\t\t<div class=\"footer-widgets column-two grid-item\">\n\t\t\t\t\t\t\t\t{html}\t\t\t\t\t\t\t</div>\n\n\t\t\t\t\t\t"));
        }
        out.push_str("\n\t\t\t\t\t</div><!-- .footer-widgets-wrapper -->\n\n\t\t\t\t</aside><!-- .footer-widgets-outer-wrapper -->\n\n\t\t\t");
    }
    out.push_str("\n\t\t</div><!-- .footer-inner -->\n\n\t</div><!-- .footer-nav-widgets-wrapper -->\n\n\t");
    Ok(out)
}

/// `footer.php` up to `wp_footer()`.
async fn footer(ctx: &Ctx<'_>) -> Result<String> {
    let site = ctx.site;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let year = site.date(&tx(ctx, "Y", "copyright date format"), now);
    let privacy = privacy_policy_link(ctx, "<p class=\"privacy-policy\">", "</p>").await?;
    // The credit names RustPress (the compat run ignores it); it points at the site itself.
    let powered = t(ctx, "Powered by WordPress").replace("WordPress", "RustPress");
    let arrow = "<span class=\"arrow\" aria-hidden=\"true\">&uarr;</span>";
    Ok(format!(
        "\t\t\t<footer id=\"site-footer\" class=\"header-footer-group\">\n\n\t\t\t\t<div class=\"section-inner\">\n\n\t\t\t\t\t<div class=\"footer-credits\">\n\n\t\t\t\t\t\t<p class=\"footer-copyright\">&copy;\n\t\t\t\t\t\t\t{year}\t\t\t\t\t\t\t<a href=\"{home}\">{name}</a>\n\t\t\t\t\t\t</p><!-- .footer-copyright -->\n\n\t\t\t\t\t\t{privacy}\n\t\t\t\t\t\t<p class=\"powered-by-wordpress\">\n\t\t\t\t\t\t\t<a href=\"{home}\">\n\t\t\t\t\t\t\t\t{powered}\t\t\t\t\t\t\t</a>\n\t\t\t\t\t\t</p><!-- .powered-by-wordpress -->\n\n\t\t\t\t\t</div><!-- .footer-credits -->\n\n\t\t\t\t\t<a class=\"to-the-top\" href=\"#site-header\">\n\t\t\t\t\t\t<span class=\"to-the-top-long\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</span><!-- .to-the-top-long -->\n\t\t\t\t\t\t<span class=\"to-the-top-short\">\n\t\t\t\t\t\t\t{}\t\t\t\t\t\t</span><!-- .to-the-top-short -->\n\t\t\t\t\t</a><!-- .to-the-top -->\n\n\t\t\t\t</div><!-- .section-inner -->\n\n\t\t\t</footer><!-- #site-footer -->\n\n\t\t",
        sprintf(&t(ctx, "To the top %s"), &[arrow]),
        sprintf(&t(ctx, "Up %s"), &[arrow]),
        home = esc_url(&site.home_url("/")),
        name = bloginfo_display(ctx, &site.settings.blogname),
    ))
}

// ------------------------------------------------------------------------ head

/// `TwentyTwenty_Non_Latin_Languages::get_non_latin_css( 'front-end' )`.
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
        "body", "input", "textarea", "button", ".button", ".faux-button", ".faux-button.more-link", ".wp-block-button__link", ".wp-block-file__button", ".has-drop-cap:not(:focus)::first-letter", ".entry-content .wp-block-archives",
        ".entry-content .wp-block-categories", ".entry-content .wp-block-cover-image", ".entry-content .wp-block-cover-image p", ".entry-content .wp-block-latest-comments", ".entry-content .wp-block-latest-posts", ".entry-content .wp-block-pullquote",
        ".entry-content .wp-block-quote.is-large", ".entry-content .wp-block-quote.is-style-large", ".entry-content .wp-block-archives *", ".entry-content .wp-block-categories *", ".entry-content .wp-block-latest-posts *", ".entry-content .wp-block-latest-comments *",
        ".entry-content", ".entry-content h1", ".entry-content h2", ".entry-content h3", ".entry-content h4", ".entry-content h5", ".entry-content h6", ".entry-content p", ".entry-content ol", ".entry-content ul", ".entry-content dl", ".entry-content dt",
        ".entry-content cite", ".entry-content figcaption", ".entry-content table", ".entry-content address", ".entry-content .wp-caption-text", ".entry-content .wp-block-file", ".comment-content p", ".comment-content ol", ".comment-content ul",
        ".comment-content dl", ".comment-content dt", ".comment-content cite", ".comment-content figcaption", ".comment-content .wp-caption-text", ".widget_text p", ".widget_text ol", ".widget_text ul", ".widget_text dl", ".widget_text dt",
        ".widget-content .rssSummary", ".widget-content cite", ".widget-content figcaption", ".widget-content .wp-caption-text",
    ];
    // `twentytwenty_generate_css()`.
    format!("{} {{ font-family: {}; }}", ELEMENTS.join(","), fonts.join(","))
}

/// `twentytwenty_get_elements_array()`: the selectors of each color, by area, color and property.
type Elements = &'static [(&'static str, &'static [(&'static str, &'static [(&'static str, &'static [&'static str])])])];

const ELEMENTS: Elements = &[
    (
        "content",
        &[
            (
                "accent",
                &[
                    ("color", &[".color-accent", ".color-accent-hover:hover", ".color-accent-hover:focus", ":root .has-accent-color", ".has-drop-cap:not(:focus):first-letter", ".wp-block-button.is-style-outline", "a"]),
                    ("border-color", &["blockquote", ".border-color-accent", ".border-color-accent-hover:hover", ".border-color-accent-hover:focus"]),
                    (
                        "background-color",
                        &["button", ".button", ".faux-button", ".wp-block-button__link", ".wp-block-file .wp-block-file__button", "input[type=\"button\"]", "input[type=\"reset\"]", "input[type=\"submit\"]", ".bg-accent", ".bg-accent-hover:hover", ".bg-accent-hover:focus", ":root .has-accent-background-color", ".comment-reply-link"],
                    ),
                    ("fill", &[".fill-children-accent", ".fill-children-accent *"]),
                ],
            ),
            (
                "background",
                &[
                    (
                        "color",
                        &[
                            ":root .has-background-color", "button", ".button", ".faux-button", ".wp-block-button__link", ".wp-block-file__button", "input[type=\"button\"]", "input[type=\"reset\"]", "input[type=\"submit\"]", ".wp-block-button", ".comment-reply-link",
                            ".has-background.has-primary-background-color:not(.has-text-color)", ".has-background.has-primary-background-color *:not(.has-text-color)", ".has-background.has-accent-background-color:not(.has-text-color)",
                            ".has-background.has-accent-background-color *:not(.has-text-color)",
                        ],
                    ),
                    ("background-color", &[":root .has-background-background-color"]),
                ],
            ),
            ("text", &[("color", &["body", ".entry-title a", ":root .has-primary-color"]), ("background-color", &[":root .has-primary-background-color"])]),
            (
                "secondary",
                &[
                    (
                        "color",
                        &[
                            "cite", "figcaption", ".wp-caption-text", ".post-meta", ".entry-content .wp-block-archives li", ".entry-content .wp-block-categories li", ".entry-content .wp-block-latest-posts li", ".wp-block-latest-comments__comment-date",
                            ".wp-block-latest-posts__post-date", ".wp-block-embed figcaption", ".wp-block-image figcaption", ".wp-block-pullquote cite", ".comment-metadata", ".comment-respond .comment-notes", ".comment-respond .logged-in-as", ".pagination .dots",
                            ".entry-content hr:not(.has-background)", "hr.styled-separator", ":root .has-secondary-color",
                        ],
                    ),
                    ("background-color", &[":root .has-secondary-background-color"]),
                ],
            ),
            (
                "borders",
                &[
                    ("border-color", &["pre", "fieldset", "input", "textarea", "table", "table *", "hr"]),
                    ("background-color", &["caption", "code", "code", "kbd", "samp", ".wp-block-table.is-style-stripes tbody tr:nth-child(odd)", ":root .has-subtle-background-background-color"]),
                    ("border-bottom-color", &[".wp-block-table.is-style-stripes"]),
                    ("border-top-color", &[".wp-block-latest-posts.is-grid li"]),
                    ("color", &[":root .has-subtle-background-color"]),
                ],
            ),
        ],
    ),
    (
        "header-footer",
        &[
            (
                "accent",
                &[
                    (
                        "color",
                        &[
                            "body:not(.overlay-header) .primary-menu > li > a", "body:not(.overlay-header) .primary-menu > li > .icon", ".modal-menu a", ".footer-menu a, .footer-widgets a:where(:not(.wp-block-button__link))", "#site-footer .wp-block-button.is-style-outline",
                            ".wp-block-pullquote:before", ".singular:not(.overlay-header) .entry-header a", ".archive-header a", ".header-footer-group .color-accent", ".header-footer-group .color-accent-hover:hover",
                        ],
                    ),
                    (
                        "background-color",
                        &[".social-icons a", "#site-footer button:not(.toggle)", "#site-footer .button", "#site-footer .faux-button", "#site-footer .wp-block-button__link", "#site-footer .wp-block-file__button", "#site-footer input[type=\"button\"]", "#site-footer input[type=\"reset\"]", "#site-footer input[type=\"submit\"]"],
                    ),
                ],
            ),
            (
                "background",
                &[
                    (
                        "color",
                        &[
                            ".social-icons a", "body:not(.overlay-header) .primary-menu ul", ".header-footer-group button", ".header-footer-group .button", ".header-footer-group .faux-button", ".header-footer-group .wp-block-button:not(.is-style-outline) .wp-block-button__link",
                            ".header-footer-group .wp-block-file__button", ".header-footer-group input[type=\"button\"]", ".header-footer-group input[type=\"reset\"]", ".header-footer-group input[type=\"submit\"]",
                        ],
                    ),
                    (
                        "background-color",
                        &["#site-header", ".footer-nav-widgets-wrapper", "#site-footer", ".menu-modal", ".menu-modal-inner", ".search-modal-inner", ".archive-header", ".singular .entry-header", ".singular .featured-media:before", ".wp-block-pullquote:before"],
                    ),
                ],
            ),
            (
                "text",
                &[
                    ("color", &[".header-footer-group", "body:not(.overlay-header) #site-header .toggle", ".menu-modal .toggle"]),
                    ("background-color", &["body:not(.overlay-header) .primary-menu ul"]),
                    ("border-bottom-color", &["body:not(.overlay-header) .primary-menu > li > ul:after"]),
                    ("border-left-color", &["body:not(.overlay-header) .primary-menu ul ul:after"]),
                ],
            ),
            (
                "secondary",
                &[(
                    "color",
                    &[
                        ".site-description", "body:not(.overlay-header) .toggle-inner .toggle-text", ".widget .post-date", ".widget .rss-date", ".widget_archive li", ".widget_categories li", ".widget cite", ".widget_pages li", ".widget_meta li", ".widget_nav_menu li",
                        ".powered-by-wordpress", ".footer-credits .privacy-policy", ".to-the-top", ".singular .entry-header .post-meta", ".singular:not(.overlay-header) .entry-header .post-meta a",
                    ],
                )],
            ),
            (
                "borders",
                &[
                    (
                        "border-color",
                        &[".header-footer-group pre", ".header-footer-group fieldset", ".header-footer-group input", ".header-footer-group textarea", ".header-footer-group table", ".header-footer-group table *", ".footer-nav-widgets-wrapper", "#site-footer", ".menu-modal nav *", ".footer-widgets-outer-wrapper", ".footer-top"],
                    ),
                    ("background-color", &[".header-footer-group table caption", "body:not(.overlay-header) .header-inner .toggle-wrapper::before"]),
                ],
            ),
        ],
    ),
];

/// `twentytwenty_get_customizer_css( 'front-end' )`.
fn customizer_css(mods: &Value) -> String {
    let mut css = String::new();
    for (area, colors) in ELEMENTS {
        for (key, properties) in *colors {
            for (property, selectors) in *properties {
                if let Some(value) = color_for_area(mods, area, key).filter(|v| !v.is_empty() && v != "0") {
                    css.push_str(&format!("{} {{ {property}: {value}; }}", selectors.join(",")));
                }
            }
        }
    }
    let cover = mods.get("cover_template_overlay_text_color").and_then(|v| match v {
        Value::Str(s) => sanitize_hex_color(&String::from_utf8_lossy(s)),
        _ => None,
    });
    if let Some(cover) = cover.filter(|c| !c.is_empty() && c != "#ffffff") {
        css.push_str(&format!(".overlay-header .header-inner {{ color: {cover}; }}.cover-header .entry-header * {{ color: {cover}; }}"));
    }
    css
}

/// `get_background_color()`: the mod, or the `default-color` of the theme's `custom-background` support.
fn custom_background_color(mods: &Value) -> String {
    match mods.get("background_color") {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => BACKGROUND.to_owned(),
    }
}

/// `_custom_background_cb()` with the theme's default color.
fn custom_background_style(ctx: &Ctx<'_>) -> String {
    let image = ctx.mod_str("background_image").unwrap_or_default();
    let mut color = custom_background_color(&ctx.mods);
    if color == BACKGROUND {
        color.clear();
    }
    if image.is_empty() && color.is_empty() {
        return if preview(ctx).is_some() { "<style id=\"custom-background-css\"></style>".to_owned() } else { String::new() };
    }
    // `maybe_hash_hex_color()`.
    let hex = (color.len() == 3 || color.len() == 6) && color.chars().all(|c| c.is_ascii_hexdigit());
    let mut style = if color.is_empty() { String::new() } else if hex { format!("background-color: #{color};") } else { format!("background-color: {color};") };
    if !image.is_empty() {
        style.push_str(&format!(" background-image: url(\"{}\");", rustpress_core::formatting::esc_url_raw(&image)));
        let pick = |name: &str, default: &str, allowed: &[&str]| ctx.mod_str(name).filter(|v| allowed.contains(&v.as_str())).unwrap_or_else(|| default.to_owned());
        style.push_str(&format!(" background-position: {} {};", pick("background_position_x", "left", &["center", "right", "left"]), pick("background_position_y", "top", &["center", "top", "bottom"])));
        style.push_str(&format!(" background-size: {};", pick("background_size", "auto", &["auto", "contain", "cover"])));
        style.push_str(&format!(" background-repeat: {};", pick("background_repeat", "repeat", &["repeat-x", "repeat-y", "repeat", "no-repeat"])));
        style.push_str(&format!(" background-attachment: {};", pick("background_attachment", "scroll", &["fixed"])));
    }
    format!("<style id=\"custom-background-css\">\nbody.custom-background {{ {} }}\n</style>\n", style.trim())
}

/// `body.custom-background` of `get_body_class()`: a background color other than the theme's default, or an image.
pub fn has_custom_background(mods: &Value) -> bool {
    custom_background_color(mods) != BACKGROUND || mods.get("background_image").is_some_and(php_truthy)
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

/// `wp_head()`, and what the plugins print on `wp_body_open` and in `wp_footer` (they work it out with the head).
/// The last part is what `wp_hoist_late_printed_styles()` puts right before `</head>`.
async fn head(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State, registry: &Registry) -> Result<(String, String, (String, String), String)> {
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
    let plugin_assets = site.plugins.assets_for(site, Some(q), None).await;
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
    // `wp_hoist_late_printed_styles()`: the enqueued handles in the order of the style handles of all registered blocks.
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
    // `wp_maybe_inline_styles()`.
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

    // The theme's styles (`twentytwenty_register_styles()`, `twentytwenty_non_latin_languages()`).
    out.push('\n');
    out.push_str(&crate::plugin_assets::StylePrinter::new(wp_root, site).links(&plugin_assets.styles));
    if rtl {
        out.push_str(&format!("<link rel='stylesheet' id='twentytwenty-style-rtl-css' href='{theme_uri}/style-rtl.css?ver={VERSION}' media='all' />\n"));
    } else {
        out.push_str(&format!("<link rel='stylesheet' id='twentytwenty-style-css' href='{}/style.css?ver={VERSION}' media='all' />\n", theme.stylesheet_directory_uri()));
    }
    let inline_css: Vec<String> = [customizer_css(&ctx.mods), non_latin_css(site)].into_iter().filter(|css| !css.is_empty()).collect();
    if !inline_css.is_empty() {
        out.push_str(&format!("<style id=\"twentytwenty-style-inline-css\">\n{}\n/*# sourceURL=twentytwenty-style-inline-css */\n</style>\n", inline_css.join("\n")));
    }
    out.push_str(&format!("<link rel='stylesheet' id='twentytwenty-fonts-css' href='{}/assets/css/font-inter.css?ver={VERSION}' media='all' />\n", theme.stylesheet_directory_uri()));
    out.push_str(&format!("<link rel='stylesheet' id='twentytwenty-print-style-css' href='{theme_uri}/print.css?ver={VERSION}' media='print' />\n"));
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
    let (early_scripts, late_scripts): (Vec<_>, Vec<_>) = plugin_assets.scripts.iter().cloned().partition(|a| a.priority <= 10);
    out.push_str(&printer.scripts(&early_scripts, false));
    // `twentytwenty_register_scripts()`.
    out.push_str(&format!("<script data-wp-strategy=\"defer\" defer id=\"twentytwenty-js-js\" src=\"{theme_uri}/assets/js/index.js?ver={VERSION}\"></script>\n"));
    out.push_str(&printer.scripts(&late_scripts, false));

    out.push_str(&discovery_links(site, q, &version, 0, !plugin_head.removed.contains(&"rel_canonical"), &crate::document::locale_stylesheet(theme, site)).await?);
    // The plugins' actions of priority 10, before the theme's.
    out.push_str(&plugin_head.late);
    // `twentytwenty_no_js_class()`.
    out.push_str("<script>\ndocument.documentElement.className = document.documentElement.className.replace( 'no-js', 'js' );\n//# sourceURL=twentytwenty_no_js_class\n</script>\n");
    // What plugins hook on `wp_head` on `init`.
    out.push_str(&plugin_head.late_init);
    // `recent_comments_style()`: an active "Recent Comments" widget.
    if (preview.is_some() || widgets::sidebars_widgets(site).iter().any(|(name, ids)| name != "wp_inactive_widgets" && ids.iter().any(|id| id.starts_with("recent-comments-")))) && !site.plugins.comments_off_everywhere(q.viewer.as_ref()) {
        out.push_str("<style>.recentcomments a{display:inline !important;padding:0 !important;margin:0 !important;}</style>");
    }
    out.push_str(&custom_background_style(ctx));
    if let Some(preview) = preview {
        out.push_str(&crate::customize_preview::loading_style(preview));
    }
    // What plugins hook on `wp_head` on `wp`.
    out.push_str(&plugin_head.on_wp);
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
        late.push('\n');
    }
    Ok((out, plugin_head.body_open, (plugin_head.footer, plugin_head.footer_on_wp), late))
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

/// `wp_footer()`.
async fn wp_footer(ctx: &Ctx<'_>, state: &rustpress_blocks::env::State, plugin_head_footer: &(String, String)) -> Result<String> {
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
    let mut out = format!("{playlist}{speculation}{nav_menus}{plugin_footer}{}{importmap}{module_scripts}{preloads}{}{}", plugin_head_footer.0, state.interactivity.data_script().unwrap_or_default(), plugin_head_footer.1);
    let includes_url = site.site_url("wp-includes");
    let plugin_assets = site.plugins.assets_for(site, Some(ctx.q), None).await;
    let mut printer = crate::plugin_assets::ScriptPrinter::new(&includes_url, &script_version(wp_root, "jquery-core", "3.7.1"), &script_version(wp_root, "jquery-migrate", "3.4.1")).with_packages(wp_root, site);
    if ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar) {
        out.push_str(&format!("<script id=\"hoverintent-js-js\" src=\"{includes_url}/js/hoverintent-js.min.js?ver=2.2.1\"></script>\n<script id=\"admin-bar-js\" src=\"{includes_url}/js/admin-bar.min.js?ver={version}\"></script>\n"));
    }
    let (early_scripts, late_scripts): (Vec<_>, Vec<_>) = plugin_assets.scripts.iter().cloned().partition(|a| a.priority <= 10);
    out.push_str(&printer.scripts(&early_scripts, true));
    // `comment-reply`, enqueued by `twentytwenty_register_scripts()` for singular pages with open comments.
    if let (true, Queried::Post(post)) = (ctx.q.flags.is_singular(), &ctx.q.queried)
        && site.comments_open(post, ctx.q.viewer.as_ref())
        && !site.plugins.closes_singular_comments(post, ctx.q.viewer.as_ref())
        && site.options.get("thread_comments") == Some("1")
    {
        out.push_str(&format!("<script async data-wp-strategy=\"async\" fetchpriority=\"low\" id=\"comment-reply-js\" src=\"{includes_url}/js/comment-reply.min.js?ver={version}\"></script>\n"));
    }
    out.push_str(&printer.scripts(&late_scripts, true));
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
        // `twentytwenty_customize_partial_blogname()` and `_blogdescription()`: `bloginfo()`.
        "blogname" => bloginfo_display(ctx, &ctx.site.settings.blogname),
        "blogdescription" => bloginfo_display(ctx, &ctx.site.settings.blogdescription),
        // `twentytwenty_customize_partial_site_logo()`.
        "custom_logo" | "retina_logo" => site_logo(ctx).await?,
        _ => {
            let Some(widget) = id.strip_prefix("widget[").and_then(|rest| rest.strip_suffix(']')) else { return Ok(None) };
            if widgets::split_id(widget).is_none_or(|(base, number)| widgets::instance(ctx.site, base, number).is_none()) {
                return Ok(None);
            }
            let Some(sidebar_id) = context.and_then(|c| c.get("sidebar_id")).and_then(serde_json::Value::as_str) else { return failed() };
            let wrap = match sidebar_id {
                "sidebar-1" => sidebar("sidebar-1"),
                "sidebar-2" => sidebar("sidebar-2"),
                _ => return failed(),
            };
            let number = context.and_then(|c| c.get("sidebar_instance_number")).and_then(serde_json::Value::as_i64);
            let request_uri = ctx.request_uri();
            let search = || search_form_markup(ctx.site, ctx.q, &ctx.env, "");
            let services = widgets::Services { site: ctx.site, q: ctx.q, env: &ctx.env, stylesheet: &ctx.theme.stylesheet, request_uri: &request_uri, search_form: &search };
            widgets::render_widget_partial(&services, &wrap, widget, number).await?
        }
    })))
}

/// The classes `get_body_class()` gets from `twentytwenty_body_classes()`.
pub(super) async fn add_body_classes(ctx: &Ctx<'_>, classes: &mut Vec<String>) -> Result<()> {
    let q = ctx.q;
    let f = &q.flags;
    let site = ctx.site;
    let viewer = q.viewer.as_ref();
    let template = page_template(ctx).await?;
    if f.is_singular() {
        classes.push("singular".into());
    }
    if template == "templates/template-cover.php" {
        classes.push("overlay-header".into());
    }
    if template == "templates/template-full-width.php" {
        classes.push("has-full-width-content".into());
    }
    if header_search(ctx) {
        classes.push("enable-search-modal".into());
    }
    if f.is_singular() {
        let thumbnail = match &q.queried {
            Queried::Post(post) => images::post_thumbnail_id(&ctx.env, post.id).await? != 0,
            _ => false,
        };
        classes.push(if thumbnail { "has-post-thumbnail".into() } else { "missing-post-thumbnail".into() });
    }
    if preview(ctx).is_some() {
        classes.push("customizer-preview".into());
    }
    let single_pagination = match (&q.queried, f.is_single) {
        (Queried::Post(post), true) => adjacent_post(ctx, post, false).await?.is_some() || adjacent_post(ctx, post, true).await?.is_some(),
        _ => false,
    };
    classes.push(if single_pagination { "has-single-pagination".into() } else { "has-no-pagination".into() });
    // The global `$post`: the first post of the main query.
    let showing = q.posts.first().is_some_and(|post| (post.post_type == "post" || site.comments_open(post, viewer) || site.comments_number(post, viewer) != 0) && !rustpress_blocks::content::post_password_required(post));
    classes.push(if showing { "showing-comments".into() } else { "not-showing-comments".into() });
    classes.push(if site.show_avatars() { "show-avatars".into() } else { "hide-avatars".into() });
    if !template.is_empty() {
        let base = template.rsplit('/').next().unwrap_or(&template);
        classes.push(base.strip_suffix(".php").unwrap_or(base).to_owned());
    }
    let top = has_menu(ctx, "footer").await? || has_menu(ctx, "social").await? || active_sidebar(ctx, "sidebar-1") || active_sidebar(ctx, "sidebar-2");
    classes.push(if top { "footer-top-visible".into() } else { "footer-top-hidden".into() });
    let header_footer = ctx.mod_str("header_footer_background_color").unwrap_or_else(|| "#ffffff".to_owned());
    let header_footer = format!("#{}", header_footer.trim_start_matches('#')).to_lowercase();
    let background = format!("#{}", background_color(&ctx.mods).trim_start_matches('#')).to_lowercase();
    if background == header_footer {
        classes.push("reduced-spacing".into());
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
    let (head_html, _, plugin_footer, late) = head(ctx, &state, registry).await?;
    let footer_html = wp_footer(ctx, &state, &plugin_footer).await?;
    Ok(format!(
        "\t\t<!doctype html>\n\t\t<html {}lang=\"{}\">\n\t\t<head>\n\t\t\t<meta charset=\"UTF-8\" />\n\t\t\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\t\t\t<link rel=\"profile\" href=\"https://gmpg.org/xfn/11\" />\n\t\t\t{head_html}\t\t\t<style>\n\t\t\t\t/* Reset theme styles */\n\t\t\t\thtml, body, #page, #content {{\n\t\t\t\t\tpadding: 0 !important;\n\t\t\t\t\tmargin: 0 !important;\n\t\t\t\t}}\n\t\t\t</style>\n\t\t{late}</head>\n\t\t<body class=\"{}\">\n\t\t<div id=\"page\" class=\"site\">\n\t\t\t<div id=\"content\" class=\"site-content\">\n\t\t\t\t{widget}\t\t\t</div><!-- #content -->\n\t\t</div><!-- #page -->\n\t\t{footer_html}\t\t</body>\n\t\t</html>\n\t\t",
        if site.is_rtl() { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
        esc_attr(&classes.join(" ")),
    ))
}

pub async fn render(ctx: &Ctx<'_>, registry: &Registry, extras: &crate::document::Extras) -> Result<Document> {
    let site = ctx.site;
    let q = ctx.q;
    let mut classes = body_classes(ctx.theme, site, q).await?;
    add_body_classes(ctx, &mut classes).await?;
    ctx.env.with_state(|s| s.content_width = 580);
    let toolbar_on = ctx.q.viewer.as_ref().is_some_and(|viewer| viewer.show_admin_bar);
    let bar = match (&extras.toolbar, toolbar_on) {
        (Some(toolbar), true) => toolbar(&crate::document::ToolbarInfo { template_slug: "", query: ctx.q, site: ctx.site }).replace(" loading='lazy'", ""),
        _ => String::new(),
    };
    let header_html = header(ctx).await?;
    let main = main_template(ctx).await?;
    let widgets_html = footer_menus_widgets(ctx).await?;
    let footer_html = footer(ctx).await?;
    let state = ctx.env.take_state();
    let (head_html, body_open, plugin_footer, late) = head(ctx, &state, registry).await?;
    let wp_footer_html = wp_footer(ctx, &state, &plugin_footer).await?;
    // `wp_body_open`: the toolbar, what plugins print, and `twentytwenty_skip_link()` at priority 5.
    let skip = format!("<a class=\"skip-link screen-reader-text\" href=\"#site-content\">{}</a>", t(ctx, "Skip to the content"));
    let html = format!(
        "<!DOCTYPE html>\n\n<html class=\"no-js\" {}lang=\"{}\">\n\n\t<head>\n\n\t\t<meta charset=\"UTF-8\">\n\t\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n\n\t\t<link rel=\"profile\" href=\"https://gmpg.org/xfn/11\">\n\n\t\t{head_html}\n\t{late}</head>\n\n\t<body class=\"{}\">\n\n\t\t{bar}{body_open}{skip}{header_html}{main}{widgets_html}\n{footer_html}{wp_footer_html}\n\t</body>\n</html>\n",
        if site.is_rtl() { "dir=\"rtl\" " } else { "" },
        esc_attr(&site.language_tag()),
        esc_attr(&classes.join(" ")),
    );
    Ok(Document { status: q.status(), html })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_come_through_kses() {
        assert!(svg("search").starts_with("<svg class=\"svg-icon\" aria-hidden=\"true\" role=\"img\" focusable=\"false\""));
        assert_eq!(svg("missing"), "");
    }

    #[test]
    fn social_links_match_their_domains() {
        assert!(social_link_svg("https://github.com/rustpress-cms").is_some());
        assert!(social_link_svg("https://example.org/").is_none());
    }

    #[test]
    fn the_more_link_becomes_a_button() {
        assert_eq!(
            read_more_tag("http://x/a/", 5, "A", "Continue reading"),
            " <div class=\"read-more-button-wrap\"><a href=\"http://x/a/#more-5\" class=\"more-link\"><span class=\"faux-button\">Continue reading</span> <span class=\"screen-reader-text\">\"A\"</span></a></div>"
        );
    }

    #[test]
    fn default_colors_fill_the_customizer_css() {
        let css = customizer_css(&Value::Null);
        assert!(css.starts_with(".color-accent,.color-accent-hover:hover,.color-accent-hover:focus,:root .has-accent-color,.has-drop-cap:not(:focus):first-letter,.wp-block-button.is-style-outline,a { color: #cd2653; }"));
        // No background color is set by default, so its rules are left out.
        assert!(!css.contains(":root .has-background-background-color {"));
    }

    #[test]
    fn hex_colors_are_checked() {
        assert_eq!(sanitize_hex_color("#abc").as_deref(), Some("#abc"));
        assert_eq!(sanitize_hex_color("abc"), None);
        assert_eq!(sanitize_hex_color("").as_deref(), Some(""));
    }
}
