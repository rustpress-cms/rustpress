//! Sidebars and widgets (`wp_get_sidebars_widgets()`, `dynamic_sidebar()` and the core widgets).

use rustpress_blocks::Env;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url, strip_tags};
use rustpress_core::{Queried, QueryResult, Site};
use rustpress_db::{CommentQuery, Order, OrderBy, PostQuery};
use rustpress_phpser::Value;

use super::menu::{self, NavArgs};
use crate::Result;

/// How a theme registered a sidebar (`register_sidebar()`).
#[derive(Debug, Clone)]
pub struct Sidebar {
    pub id: &'static str,
    pub before_widget: &'static str,
    pub after_widget: &'static str,
    pub before_title: &'static str,
    pub after_title: &'static str,
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
fn split_id(id: &str) -> Option<(&str, u64)> {
    let (base, number) = id.rsplit_once('-')?;
    Some((base, number.parse().ok()?))
}

/// The settings of one widget instance from the `widget_<base>` option.
fn instance(site: &Site, base: &str, number: u64) -> Option<Value> {
    let option = site.options.value(&format!("widget_{base}"))?;
    let entries = option.as_array()?;
    entries.iter().find_map(|(key, value)| match key {
        rustpress_phpser::Key::Int(n) if *n as u64 == number => Some(value.clone()),
        _ => None,
    })
}

fn setting_str(instance: &Value, key: &str) -> String {
    match instance.get(key) {
        Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
        Some(Value::Int(i)) => i.to_string(),
        _ => String::new(),
    }
}

fn setting_int(instance: &Value, key: &str) -> i64 {
    instance.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn setting_flag(instance: &Value, key: &str) -> bool {
    instance.get(key).is_some_and(|v| matches!(v, Value::Bool(true)) || v.as_i64().is_some_and(|n| n != 0))
}

/// The title of a widget as printed: `widget_title` filters (none that change plain text).
fn widget_title(site: &Site, instance: &Value, default: Option<&str>) -> (String, String) {
    let title = setting_str(instance, "title");
    let default_title = default.map(|d| site.tr(d)).unwrap_or_default();
    let shown = if title.is_empty() { default_title.clone() } else { title };
    (shown, default_title)
}

/// `dynamic_sidebar()`: the widgets of a sidebar, wrapped the way the theme registered it.
pub async fn dynamic_sidebar(s: &Services<'_>, sidebar: &Sidebar) -> Result<String> {
    let mut out = String::new();
    let mut calendars = 0u32;
    let Some((_, ids)) = sidebars_widgets(s.site).into_iter().find(|(name, _)| name == sidebar.id) else { return Ok(out) };
    for id in ids {
        let Some((base, number)) = split_id(&id) else { continue };
        let Some(settings) = instance(s.site, base, number) else { continue };
        let mut before = sidebar.before_widget.replacen("%1$s", &id, 1).replacen("%2$s", &classname(base), 1);
        let body = match base {
            "recent-posts" => recent_posts(s, sidebar, &before, &settings).await?,
            "search" => Some(search(s, sidebar, &before, &settings)),
            "meta" => Some(meta(s, sidebar, &before, &settings)),
            "text" => Some(text(s, sidebar, &before, &settings)),
            "nav_menu" => nav_menu(s, sidebar, &before, &settings).await?,
            "recent-comments" => Some(recent_comments(s, sidebar, &before, &settings).await?),
            "pages" => Some(pages(s, sidebar, &before, &settings).await?),
            "tag_cloud" => Some(tag_cloud(s, sidebar, &before, &settings).await?),
            "archives" => Some(archives(s, sidebar, &before, &settings).await?),
            "calendar" => Some(calendar_widget(s, sidebar, &before, &settings, &mut calendars).await?),
            "categories" => Some(categories(s, sidebar, &before, &settings).await?),
            "block" => {
                let content = setting_str(&settings, "content");
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
        if let Some(body) = body {
            out.push_str(&body);
        }
    }
    Ok(out)
}

fn heading(sidebar: &Sidebar, title: &str) -> String {
    if title.is_empty() { String::new() } else { format!("{}{title}{}", sidebar.before_title, sidebar.after_title) }
}

/// The `<nav aria-label>` of the widgets of a theme with `navigation-widgets` support.
fn nav_open(title: &str, default_title: &str) -> String {
    let plain = strip_tags(title).trim().to_owned();
    format!("<nav aria-label=\"{}\">", esc_attr(if plain.is_empty() { default_title } else { &plain }))
}

/// `WP_Widget_Recent_Posts::widget()`.
async fn recent_posts(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<Option<String>> {
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
            format!("\t<span class=\"post-date\">{}</span>\n\t\t\t\t\t", site.date(&site.options.string("date_format", "F j, Y"), ts))
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
fn search(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> String {
    let (title, _) = widget_title(s.site, settings, None);
    format!("{before}{}{}{}", heading(sidebar, &title), (s.search_form)(), sidebar.after_widget)
}

/// `WP_Widget_Meta::widget()`.
fn meta(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> String {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Meta"));
    let pretty = site.uses_pretty_permalinks();
    let feed = if pretty { site.user_trailingslashit(&site.home_url("feed")) } else { site.home_url("?feed=rss2") };
    let comments = if pretty { site.user_trailingslashit(&site.home_url("comments/feed")) } else { site.home_url("?feed=comments-rss2") };
    format!(
        "{before}{}{}\n\t\t<ul>\n\t\t\t\t\t\t<li><a href=\"{}\">{}</a></li>\n\t\t\t<li><a href=\"{}\">{}</a></li>\n\t\t\t<li><a href=\"{}\">{}</a></li>\n\n\t\t\t<li><a href=\"{}\">{}</a></li>\n\t\t</ul>\n\n\t\t</nav>{}",
        heading(sidebar, &title),
        nav_open(&title, &default_title),
        esc_url(&site.site_url("wp-login.php")),
        site.tr("Log in"),
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
fn text(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> String {
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

/// `WP_Nav_Menu_Widget::widget()`.
async fn nav_menu(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<Option<String>> {
    let site = s.site;
    let id = setting_int(settings, "nav_menu");
    if id <= 0 {
        return Ok(None);
    }
    let Some(mut menu) = menu::load_menu(site, id as u64).await? else { return Ok(None) };
    let (title, _) = widget_title(site, settings, None);
    menu::classes_by_context(site, s.q, s.request_uri, &mut menu.items).await?;
    let mut args = NavArgs::new("");
    args.container = Some(("nav".into(), Some(strip_tags(&title).trim().to_owned())));
    args.item_ids = false;
    let title_hook = |t: &str, _: &menu::MenuItem, _: usize| t.to_owned();
    let link_hook = |l: String, _: &menu::MenuItem, _: usize| l;
    let html = menu::wp_nav_menu(site, &menu, &menu.items, &args, &title_hook, &link_hook);
    Ok(Some(format!("{before}{}{html}{}", heading(sidebar, &title), sidebar.after_widget)))
}

/// `WP_Widget_Recent_Comments::widget()`.
async fn recent_comments(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Recent Comments"));
    let number = match setting_int(settings, "number") {
        n if n > 0 => n as u64,
        _ => 5,
    };
    let comments = site.db.comments_query(&CommentQuery { limit: Some(number), ..CommentQuery::default() }).await?;
    let mut items = String::new();
    for comment in &comments {
        let Some(post) = site.db.post(comment.comment_post_id).await? else { continue };
        let name = if comment.comment_author.is_empty() { site.tr("Anonymous") } else { comment.comment_author.clone() };
        let url = comment.comment_author_url.trim();
        let author = if url.is_empty() || url == "http://" || url == "https://" { name } else { format!("<a href=\"{}\" class=\"url\" rel=\"ugc external nofollow\">{name}</a>", esc_url(url)) };
        let link = format!("{}#comment-{}", site.permalink(&post).await?, comment.comment_id);
        let line = rustpress_core::i18n::sprintf(&site.trx("%1$s on %2$s", "widgets"), &[&format!("<span class=\"comment-author-link\">{author}</span>"), &format!("<a href=\"{}\">{}</a>", esc_url(&link), rustpress_blocks::content::the_title_for(site, &post))]);
        items.push_str(&format!("<li class=\"recentcomments\">{line}</li>"));
    }
    Ok(format!("{before}{}{}<ul id=\"recentcomments\">{items}</ul></nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `WP_Widget_Pages::widget()` with `wp_list_pages()`.
async fn pages(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Pages"));
    let exclude: Vec<u64> = setting_str(settings, "exclude").split(',').filter_map(|i| i.trim().parse().ok()).collect();
    let sort = setting_str(settings, "sortby");
    let order_by = match sort.as_str() {
        "menu_order" => OrderBy::MenuOrder,
        "ID" => OrderBy::Id,
        _ => OrderBy::Title,
    };
    let mut all = site.db.posts(&PostQuery { post_types: vec!["page".into()], order_by, order: Order::Asc, ..PostQuery::default() }).await?;
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
async fn tag_cloud(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let taxonomy = match setting_str(settings, "taxonomy").as_str() {
        "" => "post_tag".to_owned(),
        other => other.to_owned(),
    };
    // Other taxonomies would use their registered label, which needs a plugin; only tags exist here.
    let default = site.tr("Tags");
    let (title, default_title) = {
        let t = setting_str(settings, "title");
        (if t.is_empty() { default.clone() } else { t }, default)
    };
    let mut terms: Vec<_> = site.db.terms(&taxonomy).await?.into_iter().filter(|t| t.count > 0).collect();
    // `wp_generate_tag_cloud()` sorts the names with `natcasesort()`.
    terms.sort_by_key(|t| t.name.to_ascii_lowercase());
    terms.truncate(45);
    let mut items = String::new();
    for (i, term) in terms.iter().enumerate() {
        items.push_str(&format!(
            "\t<li><a href=\"{}\" class=\"tag-cloud-link tag-link-{} tag-link-position-{}\" style=\"font-size: 1em;\">{}</a></li>\n",
            esc_url(&site.term_link(term).await?),
            term.term_id,
            i + 1,
            esc_html(&term.name)
        ));
    }
    let cloud = if terms.is_empty() { String::new() } else { format!("<div class=\"tagcloud\"><ul class='wp-tag-cloud' role='list'>\n{items}</ul>\n</div>\n") };
    Ok(format!("{before}{}{}{cloud}</nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `WP_Widget_Archives::widget()` as a list of months.
async fn archives(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Archives"));
    let count = setting_flag(settings, "count");
    let locale = site.locale_data();
    let mut items = String::new();
    for (year, month, posts) in site.db.monthly_archives().await? {
        let name = locale.month.get(month as usize - 1).cloned().unwrap_or_default();
        let label = rustpress_core::i18n::sprintf(&site.tr("%1$s %2$d"), &[&name, &year.to_string()]);
        let link = site.date_link(year, month, 0);
        let after = if count { format!("&nbsp;({posts})") } else { String::new() };
        items.push_str(&format!("\t<li><a href='{}'>{label}</a>{after}</li>\n", esc_url(&link)));
    }
    Ok(format!("{before}{}{}\n\t\t\t<ul>\n\t\t\t\t{items}\t\t\t</ul>\n\n\t\t\t</nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `WP_Widget_Categories::widget()` as a (hierarchical) list.
async fn categories(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value) -> Result<String> {
    let site = s.site;
    let (title, default_title) = widget_title(site, settings, Some("Categories"));
    let count = setting_flag(settings, "count");
    let hierarchical = setting_flag(settings, "hierarchical");
    let mut terms: Vec<_> = site.db.terms("category").await?.into_iter().filter(|t| t.count > 0).collect();
    terms.sort_by_key(|t| t.name.to_lowercase());
    // `Walker_Category`: the queried category, its parent and its ancestors are marked.
    let mut current: Option<(u64, u64)> = None;
    let mut ancestors = Vec::new();
    if let Queried::Term(term) = &s.q.queried
        && (s.q.flags.is_category || s.q.flags.is_tax || s.q.flags.is_tag)
        && term.taxonomy == "category"
    {
        current = Some((term.term_id, term.parent));
        let mut parent = term.parent;
        while parent != 0 && !ancestors.contains(&parent) {
            ancestors.push(parent);
            parent = site.db.term_by_id(Some("category"), parent).await?.map_or(0, |t| t.parent);
        }
    }
    struct Walk<'a> {
        site: &'a Site,
        terms: &'a [rustpress_db::Term],
        hierarchical: bool,
        count: bool,
        current: Option<(u64, u64)>,
        ancestors: &'a [u64],
    }
    fn walk<'a>(w: &'a Walk<'a>, parent: u64, depth: usize, out: &'a mut String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let Walk { site, terms, hierarchical, count, current, ancestors } = *w;
            for term in terms.iter().filter(|t| if hierarchical { t.parent == parent } else { parent == 0 }) {
                let indent = "\t".repeat(depth + 1);
                let n = if count { format!(" ({})", site.number_i18n(term.count.max(0) as u64)) } else { String::new() };
                let mut classes = format!("cat-item cat-item-{}", term.term_id);
                let mut aria = "";
                if let Some((id, current_parent)) = current {
                    if term.term_id == id {
                        classes.push_str(" current-cat");
                        aria = " aria-current=\"page\"";
                    } else if term.term_id == current_parent {
                        classes.push_str(" current-cat-parent");
                    }
                    if ancestors.contains(&term.term_id) {
                        classes.push_str(" current-cat-ancestor");
                    }
                }
                out.push_str(&format!("{indent}<li class=\"{classes}\"><a{aria} href=\"{}\">{}</a>{n}\n", esc_url(&site.term_link(term).await?), esc_html(&term.name)));
                if hierarchical && terms.iter().any(|c| c.parent == term.term_id) {
                    out.push_str("<ul class='children'>\n");
                    walk(w, term.term_id, depth + 1, out).await?;
                    out.push_str(&format!("{indent}</ul>\n"));
                }
                out.push_str("</li>\n");
            }
            Ok(())
        })
    }
    let mut list = String::new();
    walk(&Walk { site, terms: &terms, hierarchical, count, current, ancestors: &ancestors }, 0, 0, &mut list).await?;
    Ok(format!("{before}{}{}\n\t\t\t<ul>\n\t\t\t\t{list}\t\t\t</ul>\n\n\t\t\t</nav>{}", heading(sidebar, &title), nav_open(&title, &default_title), sidebar.after_widget))
}

/// `gmdate()` for the date formats of the calendar (English names, as WordPress prints them there).
fn gmdate_english(format: &str, year: u32, month: u32, day: u32) -> String {
    const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        match c {
            'F' => out.push_str(MONTHS[month as usize - 1]),
            'M' => out.push_str(&MONTHS[month as usize - 1][..3]),
            'j' => out.push_str(&day.to_string()),
            'd' => out.push_str(&format!("{day:02}")),
            'n' => out.push_str(&month.to_string()),
            'm' => out.push_str(&format!("{month:02}")),
            'Y' => out.push_str(&year.to_string()),
            'y' => out.push_str(&format!("{:02}", year % 100)),
            '\\' => out.extend(chars.next()),
            other => out.push(other),
        }
    }
    out
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => 29,
        _ => 28,
    }
}

/// Weekday of a date, 0 = Sunday.
fn weekday(year: u32, month: u32, day: u32) -> u32 {
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + t[month as usize - 1] + day) % 7
}

/// `WP_Widget_Calendar::widget()` with `get_calendar()`.
async fn calendar_widget(s: &Services<'_>, sidebar: &Sidebar, before: &str, settings: &Value, instances: &mut u32) -> Result<String> {
    let site = s.site;
    let (title, _) = widget_title(site, settings, None);
    let wrap = if *instances == 0 { "<div id=\"calendar_wrap\" class=\"calendar_wrap\">" } else { "<div class=\"calendar_wrap\">" };
    *instances += 1;
    let calendar = get_calendar(s).await?;
    Ok(format!("{before}{}{wrap}{calendar}</div>{}", heading(sidebar, &title), sidebar.after_widget))
}

/// `get_calendar()`.
async fn get_calendar(s: &Services<'_>) -> Result<String> {
    let site = s.site;
    let q = s.q;
    let locale = site.locale_data();
    let now = chrono::Utc::now().timestamp();
    let today = |code: &str| site.timezone.format(code, now).parse::<u32>().unwrap_or(0);
    let vars = &q.vars;
    let (this_year, this_month) = match (vars.u64("monthnum"), vars.u64("year"), vars.get("m")) {
        (Some(month), Some(year), _) if month > 0 && year > 0 => (year as u32, month as u32),
        (_, _, Some(m)) if !m.is_empty() => (m.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0), if m.len() < 6 { 1 } else { m.get(4..6).and_then(|v| v.parse().ok()).unwrap_or(1) }),
        _ => (today("Y"), today("n")),
    };
    // `mktime( 0, 0, 0, $thismonth, 1, $thisyear )` normalizes an out-of-range month into the year; the caption
    // still prints the month name for the raw value (empty if there is none).
    let raw_month = this_month;
    let months = this_year as i64 * 12 + this_month as i64 - 1;
    let (this_year, this_month) = ((months.div_euclid(12)).max(1) as u32, months.rem_euclid(12) as u32 + 1);
    let week_begins = site.options.u64("start_of_week", 0) as u32;
    let last_day = days_in_month(this_year, this_month);
    // WordPress compares `post_date` with the raw string `$thisyear-$thismonth-01`; MariaDB reads an invalid month
    // as no date: nothing is "before" it and every post is "after" it (observed with `?m=201713`).
    let valid = (1..=12).contains(&raw_month);
    let previous = if valid { site.db.calendar_neighbor(this_year, this_month, last_day, true).await? } else { None };
    let next = if valid { site.db.calendar_neighbor(this_year, this_month, last_day, false).await? } else { site.db.calendar_neighbor(0, 1, 31, false).await? };
    let caption = rustpress_core::i18n::sprintf(&site.trx("%1$s %2$s", "calendar caption"), &[locale.month.get((raw_month as usize).wrapping_sub(1)).map_or("", String::as_str), &this_year.to_string()]);
    let mut out = format!("<table id=\"wp-calendar\" class=\"wp-calendar-table\">\n\t<caption>{caption}</caption>\n\t<thead>\n\t<tr>");
    for i in 0..7 {
        let wd = ((i + week_begins) % 7) as usize;
        out.push_str(&format!("\n\t\t<th scope=\"col\" aria-label=\"{}\">{}</th>", esc_attr(&locale.weekday[wd]), locale.weekday_initial[wd]));
    }
    out.push_str("\n\t</tr>\n\t</thead>\n\t<tbody>\n\t<tr>");
    let with_posts = site.db.calendar_days(this_year, this_month, last_day).await?;
    let week_mod = |n: i64| n.rem_euclid(7);
    let pad = week_mod(weekday(this_year, this_month, 1) as i64 - week_begins as i64);
    if pad > 0 {
        out.push_str(&format!("\n\t\t<td colspan=\"{pad}\" class=\"pad\">&nbsp;</td>"));
    }
    let mut new_row = false;
    for day in 1..=last_day {
        if new_row {
            out.push_str("\n\t</tr>\n\t<tr>\n\t\t");
        }
        new_row = false;
        out.push_str(if today("j") == day && today("n") == this_month && today("Y") == this_year { "<td id=\"today\">" } else { "<td>" });
        if with_posts.contains(&day) {
            let date = gmdate_english(&site.trx("F j, Y", "daily archives date format"), this_year, this_month, day);
            let label = rustpress_core::i18n::sprintf(&site.tr("Posts published on %s"), &[&date]);
            out.push_str(&format!("<a href=\"{}\" aria-label=\"{}\">{day}</a>", site.date_link(this_year, this_month, day), esc_attr(&label)));
        } else {
            out.push_str(&day.to_string());
        }
        out.push_str("</td>");
        if week_mod(weekday(this_year, this_month, day) as i64 - week_begins as i64) == 6 {
            new_row = true;
        }
    }
    // `$day` is one past the last day here.
    let pad = 7 - week_mod(weekday(this_year, this_month, last_day) as i64 + 1 - week_begins as i64);
    if (1..7).contains(&pad) {
        out.push_str(&format!("\n\t\t<td class=\"pad\" colspan=\"{pad}\">&nbsp;</td>"));
    }
    out.push_str("\n\t</tr>\n\t</tbody>\n\t</table>");
    out.push_str(&format!("<nav aria-label=\"{}\" class=\"wp-calendar-nav\">", site.tr("Previous and next months")));
    let abbrev = |month: u32| locale.month_abbrev[month as usize - 1].clone();
    match previous {
        Some((y, m)) => out.push_str(&format!("\n\t\t<span class=\"wp-calendar-nav-prev\"><a href=\"{}\">&laquo; {}</a></span>", site.date_link(y, m, 0), abbrev(m))),
        None => out.push_str("\n\t\t<span class=\"wp-calendar-nav-prev\">&nbsp;</span>"),
    }
    out.push_str("\n\t\t<span class=\"pad\">&nbsp;</span>");
    match next {
        Some((y, m)) => out.push_str(&format!("\n\t\t<span class=\"wp-calendar-nav-next\"><a href=\"{}\">{} &raquo;</a></span>", site.date_link(y, m, 0), abbrev(m))),
        None => out.push_str("\n\t\t<span class=\"wp-calendar-nav-next\">&nbsp;</span>"),
    }
    out.push_str("\n\t</nav>");
    Ok(out)
}