//! Sidebars and widgets (`wp_get_sidebars_widgets()`, `dynamic_sidebar()` and the core widgets).

use rustpress_blocks::{Env, general};
use rustpress_core::formatting::{esc_attr, esc_url, strip_tags};
use rustpress_core::{Queried, QueryResult, Site};
use rustpress_db::{CommentQuery, Order, OrderBy, PostQuery};
use rustpress_phpser::Value;

use super::menu::{self, NavArgs};
use crate::Result;

/// How a theme registered a sidebar (`register_sidebar()`).
#[derive(Debug, Clone)]
pub struct Sidebar<'a> {
    pub id: &'a str,
    pub before_widget: &'a str,
    pub after_widget: &'a str,
    pub before_title: &'a str,
    pub after_title: &'a str,
}

/// What the widgets need from the page and the theme.
pub struct Services<'a> {
    pub site: &'a Site,
    pub q: &'a QueryResult,
    pub env: &'a Env<'a>,
    pub stylesheet: &'a str,
    pub request_uri: &'a str,
    /// `get_search_form()` of the theme.
    pub search_form: &'a (dyn Fn() -> String + Sync),
}

/// A sidebar as `register_sidebar()` keeps it in `$wp_registered_sidebars`.
#[derive(Debug, Clone, Default)]
pub struct RegisteredSidebar {
    pub id: String,
    pub name: String,
    pub description: String,
    pub class: String,
    pub before_widget: String,
    pub after_widget: String,
    pub before_title: String,
    pub after_title: String,
    pub show_in_rest: bool,
}

/// `$wp_registered_sidebars` of the active theme: the sidebars of a ported classic theme, or the classic sidebars a
/// block theme kept from the theme before it (`_wp_block_theme_register_classic_sidebars()`).
pub fn registered_sidebars(theme: &crate::theme::Theme, site: &Site) -> Vec<RegisteredSidebar> {
    if super::is_ported(theme) {
        return if super::is_twentyone(theme) {
            super::twentyone::registered_sidebars(site)
        } else if super::is_twentytwenty(theme) {
            super::twenty::registered_sidebars(site)
        } else {
            super::seventeen::registered_sidebars(site)
        };
    }
    let mods = super::menu::theme_mod(site, &theme.stylesheet);
    let Some(list) = mods.get("wp_classic_sidebars").and_then(Value::as_array) else { return Vec::new() };
    list.iter()
        .filter_map(|(_, sidebar)| {
            sidebar.as_array()?;
            let text = |key: &str| setting_str(sidebar, key);
            Some(RegisteredSidebar {
                id: text("id"),
                name: text("name"),
                description: text("description"),
                class: text("class"),
                before_widget: text("before_widget"),
                after_widget: text("after_widget"),
                before_title: text("before_title"),
                after_title: text("after_title"),
                show_in_rest: setting_flag(sidebar, "show_in_rest"),
            })
        })
        .collect()
}

/// `wp_get_sidebars_widgets()`: sidebar id -> widget ids.
pub fn sidebars_widgets(site: &Site) -> Vec<(String, Vec<String>)> {
    let Some(value) = site.options.value("sidebars_widgets") else { return Vec::new() };
    let Some(entries) = value.as_array() else { return Vec::new() };
    entries
        .iter()
        .filter_map(|(key, list)| {
            let rustpress_phpser::Key::Str(name) = key else { return None };
            let ids = list.as_array()?.iter().filter_map(|(_, id)| id.as_str().map(str::to_owned)).collect();
            Some((String::from_utf8_lossy(name).into_owned(), ids))
        })
        .collect()
}

/// `is_active_sidebar()`.
pub fn is_active_sidebar(site: &Site, id: &str) -> bool {
    sidebars_widgets(site).iter().any(|(name, ids)| name == id && !ids.is_empty())
}

/// The widget class name WordPress puts on the wrapper (`widget_ops['classname']`).
fn classname(base: &str) -> String {
    match base {
        "recent-posts" => "widget_recent_entries".into(),
        "recent-comments" => "widget_recent_comments".into(),
        "archives" => "widget_archive".into(),
        "nav_menu" => "widget_nav_menu".into(),
        "tag_cloud" => "widget_tag_cloud".into(),
        other => format!("widget_{}", other.replace('-', "_")),
    }
}

/// Splits `recent-posts-6` into `recent-posts` and `6`.
pub(super) fn split_id(id: &str) -> Option<(&str, u64)> {
    let (base, number) = id.rsplit_once('-')?;
    Some((base, number.parse().ok()?))
}

/// The settings of one widget instance from the `widget_<base>` option.
pub(super) fn instance(site: &Site, base: &str, number: u64) -> Option<Value> {
    let option = site.options.value(&format!("widget_{base}"))?;
    let entries = option.as_array()?;
    entries.iter().find_map(|(key, value)| match key {
        rustpress_phpser::Key::Int(n) if *n as u64 == number => Some(value.clone()),
        _ => None,
    })
}

pub(super) fn setting_str(instance: &Value, key: &str) -> String {
    match instance.get(key) {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => String::new(),
    }
}

pub(super) fn setting_int(instance: &Value, key: &str) -> i64 {
    instance.get(key).and_then(Value::as_i64).unwrap_or(0)
}

pub(super) fn setting_flag(instance: &Value, key: &str) -> bool {
    instance.get(key).is_some_and(|v| matches!(v, Value::Bool(true)) || v.as_i64().is_some_and(|n| n != 0))
}

/// The title of a widget as printed: `widget_title` runs `wptexturize()`, `convert_chars()` and `esc_html()`.
pub(super) fn widget_title(site: &Site, instance: &Value, default: Option<&str>) -> (String, String) {
    let title = setting_str(instance, "title");
    let default_title = default.map(|d| site.tr(d)).unwrap_or_default();
    let shown = if title.is_empty() { default_title.clone() } else { title };
    (rustpress_core::formatting::esc_html(&site.texturize(&shown)), default_title)
}

/// `dynamic_sidebar()`: the widgets of a sidebar, wrapped the way the theme registered it.
pub async fn dynamic_sidebar(s: &Services<'_>, sidebar: &Sidebar<'_>) -> Result<String> {
    let mut out = String::new();
    // `WP_Customize_Widgets` in the preview: `dynamic_sidebar_has_widgets` counts the sidebar, and with selective
    // refresh `dynamic_sidebar_before` and `_after` mark where its widgets are.
    let preview = s.q.customize.as_deref().filter(|p| p.widgets_previewed());
    let marked = preview.filter(|p| p.selective_refresh_widgets);
    let instance_number = marked.map(|p| p.start_sidebar(sidebar.id));
    if let (Some(_), Some(number)) = (marked, instance_number) {
        out.push_str(&rustpress_core::customize::sidebar_marker("before", sidebar.id, number));
    }
    if let Some((_, ids)) = sidebars_widgets(s.site).into_iter().find(|(name, _)| name == sidebar.id) {
        for id in ids {
            let Some((base, number)) = split_id(&id) else { continue };
            let Some(settings) = instance(s.site, base, number) else { continue };
            if let Some(preview) = preview {
                preview.tally_widget(&id);
            }
            match instance_number {
                Some(number) => {
                    // `filter_dynamic_sidebar_params()`: the attributes of the partial on the widget's element.
                    let before = sidebar.before_widget.replacen("%1$s", &id, 1).replacen("%2$s", &classname(base), 1);
                    let context = format!("{{\"sidebar_id\":{},\"sidebar_instance_number\":{number}}}", rustpress_blocks::php_json::encode(&serde_json::Value::String(sidebar.id.to_owned())));
                    let attributes = format!(
                        " data-customize-partial-id=\"{}\" data-customize-partial-type=\"widget\" data-customize-partial-placement-context=\"{}\" data-customize-widget-id=\"{}\"",
                        esc_attr(&format!("widget[{id}]")),
                        esc_attr(&context),
                        esc_attr(&id)
                    );
                    let name_end = before.char_indices().skip(1).find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_')).map_or(before.len(), |(i, _)| i);
                    let before = if before.starts_with('<') && name_end > 1 { format!("{} {attributes}{}", &before[..name_end], &before[name_end..]) } else { before };
                    out.push_str(&display(s, sidebar, before, base, id.rsplit('-').next().unwrap_or_default(), &settings).await?);
                }
                None => out.push_str(&render_widget(s, sidebar, &id, base, &settings).await?),
            }
        }
    }
    if let (Some(_), Some(number)) = (marked, instance_number) {
        out.push_str(&rustpress_core::customize::sidebar_marker("after", sidebar.id, number));
    }
    Ok(out)
}

/// `WP_Customize_Widgets::render_widget_partial()`: `dynamic_sidebar()` with only this widget in the sidebar, with
/// the attributes of its partial (the sidebar's call number comes from the request) and without the markers.
pub async fn render_widget_partial(s: &Services<'_>, sidebar: &Sidebar<'_>, id: &str, instance_number: Option<i64>) -> Result<String> {
    let Some(preview) = s.q.customize.as_deref() else { return Ok(String::new()) };
    let counted = preview.start_sidebar(sidebar.id);
    let Some((base, number)) = split_id(id) else { return Ok(String::new()) };
    let Some(settings) = instance(s.site, base, number) else { return Ok(String::new()) };
    preview.tally_widget(id);
    let before = sidebar.before_widget.replacen("%1$s", id, 1).replacen("%2$s", &classname(base), 1);
    let context = format!(
        "{{\"sidebar_id\":{},\"sidebar_instance_number\":{}}}",
        rustpress_blocks::php_json::encode(&serde_json::Value::String(sidebar.id.to_owned())),
        instance_number.unwrap_or(i64::from(counted))
    );
    let attributes = format!(
        " data-customize-partial-id=\"{}\" data-customize-partial-type=\"widget\" data-customize-partial-placement-context=\"{}\" data-customize-widget-id=\"{}\"",
        esc_attr(&format!("widget[{id}]")),
        esc_attr(&context),
        esc_attr(id)
    );
    let name_end = before.char_indices().skip(1).find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '_')).map_or(before.len(), |(i, _)| i);
    let before = if before.starts_with('<') && name_end > 1 { format!("{} {attributes}{}", &before[..name_end], &before[name_end..]) } else { before };
    display(s, sidebar, before, base, id.rsplit('-').next().unwrap_or_default(), &settings).await
}

/// `wp_render_widget()`: one widget with the settings the caller holds.
pub async fn render_widget(s: &Services<'_>, sidebar: &Sidebar<'_>, id: &str, base: &str, settings: &Value) -> Result<String> {
    let before = sidebar.before_widget.replacen("%1$s", id, 1).replacen("%2$s", &classname(base), 1);
    display(s, sidebar, before, base, id.rsplit('-').next().unwrap_or_default(), settings).await
}

/// `the_widget()` with its default wrapper: the preview of a widget that is being edited.
pub async fn the_widget(s: &Services<'_>, base: &str, settings: &Value) -> Result<String> {
    let sidebar = Sidebar { id: "", before_widget: "<div class=\"widget %s\">", after_widget: "</div>", before_title: "<h2 class=\"widgettitle\">", after_title: "</h2>" };
    let before = sidebar.before_widget.replacen("%s", &classname(base), 1);
    display(s, &sidebar, before, base, "-1", settings).await
}

/// `WP_Widget::widget()` of the core widgets.
async fn display(s: &Services<'_>, sidebar: &Sidebar<'_>, mut before: String, base: &str, number: &str, settings: &Value) -> Result<String> {
    let body = match base {
        "recent-posts" => recent_posts(s, sidebar, &before, settings).await?,
        "search" => Some(search(s, sidebar, &before, settings)),
        "meta" => Some(meta(s, sidebar, &before, settings)),
        "text" => Some(text(s, sidebar, &before, settings)),
        "custom_html" => Some(custom_html(s, sidebar, &before, settings)),
        "nav_menu" => nav_menu(s, sidebar, &before, settings).await?,
        // `unregister_widget( 'WP_Widget_Recent_Comments' )` of a plugin.
        "recent-comments" if s.site.plugins.comments_off_everywhere(s.q.viewer.as_ref()) => None,
        "recent-comments" => Some(recent_comments(s, sidebar, &before, settings).await?),
        "pages" => Some(pages(s, sidebar, &before, settings).await?),
        "tag_cloud" => tag_cloud(s, sidebar, &before, settings).await?,
        "archives" => Some(archives(s, sidebar, &before, number, settings).await?),
        "calendar" => Some(calendar_widget(s, sidebar, &before, settings).await?),
        "categories" => Some(categories(s, sidebar, &before, number, settings).await?),
        "media_image" | "media_audio" | "media_video" | "media_gallery" => super::media_widgets::display(s, sidebar, &before, base, settings).await?,
        "block" => {
            let content = setting_str(settings, "content");
            let blocks = rustpress_blocks::parser::parse_blocks(&content);
            let class = match blocks.first().and_then(|b| b.name.as_deref()) {
                Some("core/paragraph") => "widget_block widget_text",
                Some("core/calendar") => "widget_block widget_calendar",
                Some("core/search") => "widget_block widget_search",
                Some("core/html") => "widget_block widget_custom_html",
                Some("core/archives") => "widget_block widget_archive",
                Some("core/latest-posts") => "widget_block widget_recent_entries",
                Some("core/latest-comments") => "widget_block widget_recent_comments",
                Some("core/tag-cloud") => "widget_block widget_tag_cloud",
                _ => "widget_block",
            };
            before = before.replace("widget_block", class);
            let rendered = s.env.render_markup(&content, &s.env.root_context()).await?;
            Some(format!("{before}{rendered}{}", sidebar.after_widget))
        }
        _ => None,
    };
    Ok(body.unwrap_or_default())
}

pub(super) fn heading(sidebar: &Sidebar<'_>, title: &str) -> String {
    if title.is_empty() { String::new() } else { format!("{}{title}{}", sidebar.before_title, sidebar.after_title) }
}

/// The `<nav aria-label>` of the widgets of a theme with `navigation-widgets` support.
fn nav_open(title: &str, default_title: &str) -> String {
    let plain = strip_tags(title).trim().to_owned();
    format!("<nav aria-label=\"{}\">", esc_attr(if plain.is_empty() { default_title } else { &plain }))
}

/// `WP_Widget_Recent_Posts::widget()`.
async fn recent_posts(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> Result<Option<String>> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Recent Posts"));
    let number = match setting_int(settings, "number") {
        n if n > 0 => n as u64,
        _ => 5,
    };
    let show_date = setting_flag(settings, "show_date");
    let query = PostQuery { post_types: vec!["post".into()], limit: Some(number), order_by: OrderBy::Date, order: Order::Desc, ..PostQuery::default() };
    let posts = site.db.posts(&query).await?;
    if posts.is_empty() {
        return Ok(None);
    }
    let queried_id = match &s.q.queried {
        Queried::Post(p) => p.id,
        _ => 0,
    };
    let mut out = format!("\n\t\t{before}\n\t\t{}{}\n\t\t<ul>\n\t\t\t", heading(sidebar, &title), nav_open(&title, &default_title));
    for post in &posts {
        let post_title = rustpress_blocks::content::the_title_for(site, post);
        let text = if post_title.is_empty() { site.tr("(no title)") } else { post_title };
        let current = if queried_id == post.id { " aria-current=\"page\"" } else { "" };
        let date = if show_date {
            let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
            format!("\t\t\t\t\t\t<span class=\"post-date\">{}</span>\n\t\t\t\t\t", site.date(&site.options.string("date_format", "F j, Y"), ts))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "\t\t\t\t\t\t\t\t<li>\n\t\t\t\t\t<a href=\"{}\"{current}>{text}</a>\n\t\t\t\t\t{date}\t\t\t\t</li>\n\t\t\t",
            esc_url(&site.permalink(post).await?)
        ));
    }
    out.push_str(&format!("\t\t</ul>\n\n\t\t</nav>{}", sidebar.after_widget));
    Ok(Some(out))
}

/// `WP_Widget_Search::widget()`.
fn search(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> String {
    let (title, _) = widget_title(s.site, settings, None);
    format!("{before}{}{}{}", heading(sidebar, &title), (s.search_form)(), sidebar.after_widget)
}

/// `WP_Widget_Meta::widget()`.
fn meta(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> String {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Meta"));
    let pretty = site.uses_pretty_permalinks();
    let feed = if pretty { site.user_trailingslashit(&site.home_url(&format!("{}feed", site.root()))) } else { site.home_url("?feed=rss2") };
    let comments = if pretty { site.user_trailingslashit(&site.home_url(&format!("{}comments/feed", site.root()))) } else { site.home_url("?feed=comments-rss2") };
    // `wp_register()`.
    let register = match &s.q.viewer {
        Some(viewer) if viewer.can("read") => format!("<li><a href=\"{}\">{}</a></li>", site.site_url("wp-admin/"), site.tr("Site Admin")),
        None if site.options.get("users_can_register") == Some("1") => format!("<li><a href=\"{}\">{}</a></li>", esc_url(&site.site_url("wp-login.php?action=register")), site.tr("Register")),
        _ => String::new(),
    };
    // `wp_loginout()` with the `loginout` filter.
    let login = match &s.q.viewer {
        Some(viewer) => format!("<a href=\"{}\">{}</a>", esc_url(&rustpress_core::urls::add_query_arg(&site.site_url("wp-login.php?action=logout"), "_wpnonce", &viewer.create_nonce("log-out"))), site.tr("Log out")),
        None => format!("<a href=\"{}\">{}</a>", esc_url(&site.site_url("wp-login.php")), site.tr("Log in")),
    };
    let login = site.plugins.loginout(login);
    format!(
        "{before}{}{}\n\t\t<ul>\n\t\t\t{register}\t\t\t<li>{login}</li>\n\t\t\t<li><a href=\"{}\">{}</a></li>\n\t\t\t<li><a href=\"{}\">{}</a></li>\n\n\t\t\t<li><a href=\"{}\">{}</a></li>\n\t\t</ul>\n\n\t\t</nav>{}",
        heading(sidebar, &title),
        nav_open(&title, &default_title),
        esc_url(&feed),
        site.tr("Entries feed"),
        esc_url(&comments),
        site.tr("Comments feed"),
        esc_url(&site.tr("https://wordpress.org/")),
        site.tr("WordPress.org"),
        sidebar.after_widget
    )
}

/// `WP_Widget_Text::widget()`.
fn text(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> String {
    let site = s.site;
    let (title, _) = widget_title(site, settings, None);
    let raw = setting_str(settings, "text");
    let content = if setting_flag(settings, "visual") {
        let text = rustpress_core::formatting::capital_p_dangit(&site.texturize(&raw), false);
        rustpress_core::formatting::wpautop(&text)
    } else if setting_flag(settings, "filter") {
        rustpress_core::formatting::wpautop(&raw)
    } else {
        raw
    };
    format!("{before}{}\t\t\t<div class=\"textwidget\">{content}</div>\n\t\t{}", heading(sidebar, &title), sidebar.after_widget)
}

/// `WP_Widget_Custom_HTML::widget()`.
fn custom_html(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> String {
    let (title, _) = widget_title(s.site, settings, None);
    let content = setting_str(settings, "content");
    // The class of the Text widget goes in front of the classes, for themes that style it.
    let before = before.replacen(" class=\"", " class=\"widget_text ", 1);
    format!("{before}{}<div class=\"textwidget custom-html-widget\">{content}</div>{}", heading(sidebar, &title), sidebar.after_widget)
}

/// `WP_Nav_Menu_Widget::widget()`.
async fn nav_menu(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> Result<Option<String>> {
    let site = s.site;
    let id = setting_int(settings, "nav_menu");
    if id == 0 {
        return Ok(None);
    }
    let Some(mut menu) = menu::load_menu(site, s.q.customize.as_deref(), id).await? else { return Ok(None) };
    let (title, _) = widget_title(site, settings, None);
    menu::classes_by_context(site, s.q, s.request_uri, &mut menu.items).await?;
    let mut args = NavArgs::new("");
    args.container = Some(("nav".into(), Some(strip_tags(&title).trim().to_owned())));
    args.menu = Some(menu.term_id);
    args.fallback_cb = "";
    args.customize = s.q.customize.clone().filter(|p| p.menus_previewed());
    args.use_ids_once(s.env, &menu.items);
    let title_hook = |t: &str, _: &menu::MenuItem, _: usize| t.to_owned();
    let link_hook = |l: String, _: &menu::MenuItem, _: usize| l;
    let html = menu::wp_nav_menu(site, &menu, &menu.items, &args, &title_hook, &link_hook);
    Ok(Some(format!("{before}{}{html}{}", heading(sidebar, &title), sidebar.after_widget)))
}

/// `WP_Widget_Recent_Comments::widget()`.
async fn recent_comments(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Recent Comments"));
    let number = match setting_int(settings, "number") {
        n if n > 0 => n as u64,
        _ => 5,
    };
    let comments = site.db.comments_query(&CommentQuery { limit: Some(number), types: Vec::new(), ..CommentQuery::default() }).await?;
    let mut items = String::new();
    for comment in &comments {
        let Some(post) = site.db.post(comment.comment_post_id).await? else { continue };
        let name = if comment.comment_author.is_empty() { site.tr("Anonymous") } else { comment.comment_author.clone() };
        let url = comment.comment_author_url.trim();
        let author = if url.is_empty() || url == "http://" || url == "https://" { name } else { format!("<a href=\"{}\" class=\"url\" rel=\"ugc external nofollow\">{name}</a>", esc_url(url)) };
        let link = rustpress_blocks::comment_pages::comment_link(s.env, comment, None).await?;
        let line = rustpress_core::i18n::sprintf(&site.trx("%1$s on %2$s", "widgets"), &[&format!("<span class=\"comment-author-link\">{author}</span>"), &format!("<a href=\"{}\">{}</a>", esc_url(&link), rustpress_blocks::content::the_title_for(site, &post))]);
        items.push_str(&format!("<li class=\"recentcomments\">{line}</li>"));
    }
    Ok(format!("{before}{}{}<ul id=\"recentcomments\">{items}</ul></nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `WP_Widget_Pages::widget()` with `wp_list_pages()`.
async fn pages(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Pages"));
    let exclude: Vec<u64> = setting_str(settings, "exclude").split(',').filter_map(|i| i.trim().parse().ok()).collect();
    let sort = setting_str(settings, "sortby");
    let order_by = if sort == "ID" { OrderBy::Id } else { OrderBy::Title };
    let mut all = site.db.posts(&PostQuery { post_types: vec!["page".into()], order_by, order: Order::Asc, ..PostQuery::default() }).await?;
    // `menu_order, post_title`, also without a setting.
    if sort.is_empty() || sort == "menu_order" {
        all.sort_by_key(|p| p.menu_order);
    }
    all.retain(|p| !exclude.contains(&p.id));
    let queried = match &s.q.queried {
        Queried::Post(p) if s.q.flags.is_page || s.q.flags.is_posts_page => Some(p.as_ref().clone()),
        _ => None,
    };
    // Ancestors of the current page and the page for posts.
    let mut ancestors: Vec<u64> = Vec::new();
    if let Some(page) = &queried {
        let mut parent = page.post_parent;
        while parent != 0 && !ancestors.contains(&parent) {
            ancestors.push(parent);
            parent = all.iter().find(|p| p.id == parent).map_or(0, |p| p.post_parent);
        }
    }
    let page_for_posts = site.settings.page_for_posts;
    struct Walk<'a> {
        site: &'a Site,
        all: &'a [rustpress_db::Post],
        queried: Option<&'a rustpress_db::Post>,
        ancestors: &'a [u64],
        page_for_posts: u64,
    }
    fn walk<'a>(w: &'a Walk<'a>, parent: u64, depth: usize, out: &'a mut String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let Walk { site, all, queried, ancestors, page_for_posts } = *w;
            for page in all.iter().filter(|p| p.post_parent == parent || (parent == 0 && !all.iter().any(|a| a.id == p.post_parent))) {
                if parent == 0 && page.post_parent != 0 && all.iter().any(|a| a.id == page.post_parent) {
                    continue;
                }
                let has_children = all.iter().any(|c| c.post_parent == page.id);
                let mut css = format!("page_item page-item-{}", page.id);
                if has_children {
                    css.push_str(" page_item_has_children");
                }
                let mut current = false;
                if let Some(q) = queried {
                    if q.id == page.id {
                        css.push_str(" current_page_item");
                        current = true;
                    } else if q.post_parent == page.id {
                        css.push_str(" current_page_parent");
                    }
                    if ancestors.contains(&page.id) {
                        css.push_str(" current_page_ancestor");
                    }
                } else if page.id == page_for_posts && page_for_posts != 0 {
                    css.push_str(" current_page_parent");
                }
                let mut title = super::menu::the_title_filters(site, &page.post_title);
                if title.is_empty() {
                    title = rustpress_core::i18n::sprintf(&site.tr("#%d (no title)"), &[&page.id.to_string()]);
                }
                let aria = if current { " aria-current=\"page\"" } else { "" };
                let indent = "\t".repeat(depth);
                out.push_str(&format!("{indent}<li class=\"{css}\"><a href=\"{}\"{aria}>{title}</a>", esc_url(&site.permalink(page).await?)));
                if has_children {
                    out.push_str(&format!("\n{indent}<ul class='children'>\n"));
                    walk(w, page.id, depth + 1, out).await?;
                    out.push_str(&format!("{indent}</ul>\n"));
                }
                out.push_str("</li>\n");
            }
            Ok(())
        })
    }
    let mut list = String::new();
    walk(&Walk { site, all: &all, queried: queried.as_ref(), ancestors: &ancestors, page_for_posts }, 0, 0, &mut list).await?;
    let list = list.trim_end_matches('\n').to_owned();
    Ok(format!("{before}{}{}\n\t\t\t<ul>\n\t\t\t\t{list}\n\t\t\t</ul>\n\n\t\t\t</nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `WP_Widget_Tag_Cloud::widget()` with the theme's arguments (one size, a list).
async fn tag_cloud(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> Result<Option<String>> {
    let site = s.site;
    // `_get_current_taxonomy()`.
    let wanted = setting_str(settings, "taxonomy");
    let taxonomy = if !wanted.is_empty() && matches!(wanted.as_str(), "category" | "post_tag" | "link_category") || site.types.taxonomy(&wanted).is_some() { wanted } else { "post_tag".to_owned() };
    let default = match taxonomy.as_str() {
        "post_tag" => site.tr("Tags"),
        "link_category" => site.tr("Link Categories"),
        other => site.taxonomy_label(other),
    };
    let title = setting_str(settings, "title");
    let title = rustpress_core::formatting::esc_html(&site.texturize(if title.is_empty() { &default } else { &title }));
    // `twentyseventeen_widget_tag_cloud_args()`; other themes keep the defaults.
    let args = if s.stylesheet == "twentyseventeen" {
        general::TagCloud { taxonomy, smallest: 1.0, largest: 1.0, unit: "em".into(), list: true, show_count: setting_flag(settings, "count"), ..general::TagCloud::new() }
    } else {
        general::TagCloud { taxonomy, show_count: setting_flag(settings, "count"), ..general::TagCloud::new() }
    };
    let cloud = general::wp_tag_cloud(s.env, &args).await?;
    if cloud.is_empty() {
        return Ok(None);
    }
    Ok(Some(format!("{before}{}{}<div class=\"tagcloud\">{cloud}</div>\n</nav>{}", heading(sidebar, &title), nav_open(&title, &default), sidebar.after_widget)))
}

/// `WP_Widget_Archives::widget()` as a list of months.
async fn archives(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, number: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Archives"));
    if setting_flag(settings, "dropdown") {
        let id = format!("archives-dropdown-{number}");
        let args = general::Archives { format: "option".into(), show_post_count: setting_flag(settings, "count"), ..general::Archives::new() };
        let items = general::wp_get_archives(s.env, &args).await?;
        let script = dropdown_script(ARCHIVES_SCRIPT, &id, "WP_Widget_Archives%3A%3Awidget");
        return Ok(format!(
            "{before}{}\t\t<label class=\"screen-reader-text\" for=\"{id}\">{title}</label>\n\t\t<select id=\"{id}\" name=\"archive-dropdown\">\n\t\t\t\n\t\t\t<option value=\"\">{}</option>\n\t\t\t{items}\n\t\t</select>\n\n\t\t\t{script}{}",
            heading(sidebar, &title),
            esc_attr(&site.tr("Select Month")),
            sidebar.after_widget
        ));
    }
    let args = general::Archives { show_post_count: setting_flag(settings, "count"), ..general::Archives::new() };
    let items = general::wp_get_archives(s.env, &args).await?;
    Ok(format!("{before}{}{}\n\t\t\t<ul>\n\t\t\t\t{items}\t\t\t</ul>\n\n\t\t\t</nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `WP_Widget_Categories::widget()` as a (hierarchical) list.
async fn categories(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, number: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Categories"));
    let mut args = general::ListCategories::new(s.env, "category");
    args.show_count = setting_flag(settings, "count");
    args.terms.hierarchical = setting_flag(settings, "hierarchical");
    if setting_flag(settings, "dropdown") {
        let first = s.env.with_state(|state| !std::mem::replace(&mut state.categories_dropdown, true));
        let id = if first { "cat".to_owned() } else { format!("categories-dropdown-{number}") };
        let dropdown = general::DropdownCategories {
            terms: args.terms.clone(),
            show_count: args.show_count,
            depth: 0,
            show_option_none: site.tr("Select Category"),
            show_option_all: String::new(),
            option_none_value: "-1".into(),
            name: "cat".into(),
            id: id.clone(),
            class: "postform".into(),
            value_field_slug: false,
            selected: "0".into(),
            hide_if_empty: false,
            required: false,
        };
        let select = general::wp_dropdown_categories(s.env, &dropdown).await?;
        let script = dropdown_script(CATEGORIES_SCRIPT, &id, "WP_Widget_Categories%3A%3Awidget");
        return Ok(format!(
            "{before}{}<form action=\"{}\" method=\"get\"><label class=\"screen-reader-text\" for=\"{}\">{title}</label>{select}</form>{script}{}",
            heading(sidebar, &title),
            esc_url(&site.home_url("")),
            esc_attr(&id),
            sidebar.after_widget
        ));
    }
    args.title_li = String::new();
    let list = general::wp_list_categories(s.env, &args).await?;
    Ok(format!("{before}{}{}\n\t\t\t<ul>\n\t\t\t\t{list}\t\t\t</ul>\n\n\t\t\t</nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// The scripts of the two widgets with a drop-down: both follow the selection, but not after the Escape key.
const SCRIPT_END: &str = "\tfunction onKeyUp( event ) {\n\t\tif ( 'Escape' === event.key ) {\n\t\t\tdropdown.dataset.lastkey = 'escape';\n\t\t} else {\n\t\t\tdelete dropdown.dataset.lastkey;\n\t\t}\n\t}\n\tfunction onClick() {\n\t\tdelete dropdown.dataset.lastkey;\n\t}\n\tdropdown.addEventListener( 'keyup', onKeyUp );\n\tdropdown.addEventListener( 'click', onClick );\n\tdropdown.addEventListener( 'change', onSelectChange );\n";
const CATEGORIES_SCRIPT: &str = "\t\t\tif ( dropdown.value && parseInt( dropdown.value ) > 0 && dropdown instanceof HTMLSelectElement ) {\n\t\t\t\tdropdown.parentElement.submit();\n\t\t\t}\n";
const ARCHIVES_SCRIPT: &str = "\t\t\tif ( dropdown.value ) {\n\t\t\t\tdocument.location.href = dropdown.value;\n\t\t\t}\n";

fn dropdown_script(action: &str, id: &str, source: &str) -> String {
    format!(
        "<script>\n( ( dropdownId ) => {{\n\tconst dropdown = document.getElementById( dropdownId );\n\tfunction onSelectChange() {{\n\t\tsetTimeout( () => {{\n\t\t\tif ( 'escape' === dropdown.dataset.lastkey ) {{\n\t\t\t\treturn;\n\t\t\t}}\n{action}\t\t}}, 250 );\n\t}}\n{SCRIPT_END}}})( \"{id}\" );\n\n//# sourceURL={source}\n</script>\n"
    )
}

/// `WP_Widget_Calendar::widget()` with `get_calendar()`.
async fn calendar_widget(s: &Services<'_>, sidebar: &Sidebar<'_>, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, _) = widget_title(site, settings, None);
    // `WP_Widget_Calendar::$instance` counts for the whole request.
    let first = s.env.with_state(|state| {
        state.calendars += 1;
        state.calendars == 1
    });
    let wrap = if first { "<div id=\"calendar_wrap\" class=\"calendar_wrap\">" } else { "<div class=\"calendar_wrap\">" };
    let calendar = general::get_calendar(s.env, true, None).await?;
    Ok(format!("{before}{}{wrap}{calendar}</div>{}", heading(sidebar, &title), sidebar.after_widget))
}