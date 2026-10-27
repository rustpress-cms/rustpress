//! `core/query`, `core/post-template`, `core/query-title`, `core/query-no-results`
//! and the pagination blocks.

use std::sync::Arc;

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::query::parse_search_terms;
use rustpress_core::rewrite::QueryVars;
use rustpress_core::urls::add_query_arg;
use rustpress_db::{Order, OrderBy, Post, PostQuery};
use serde_json::{Map, Value};

use crate::Result;
use crate::content::post_class;
use crate::env::Context;
use crate::parser::Content;
use crate::render::BlockCtx;

/// The posts a query block iterates over and the numbers for pagination.
#[derive(Debug, Clone)]
pub(crate) struct LoopData {
    pub posts: Vec<Post>,
    pub max_pages: u64,
    /// `found_posts` and `posts_per_page` of the query.
    pub found: u64,
    pub per_page: u64,
}

fn query_ctx<'a>(bc: &'a BlockCtx<'_>) -> Option<&'a Map<String, Value>> {
    bc.context.get("query").and_then(Value::as_object)
}

fn is_inherit(bc: &BlockCtx<'_>) -> bool {
    query_ctx(bc).and_then(|q| q.get("inherit")).is_some_and(|v| !crate::style_engine::is_empty(v))
}

fn page_key(bc: &BlockCtx<'_>) -> String {
    match bc.context.get("queryId") {
        Some(id) => format!("query-{}-page", id.as_u64().map_or_else(|| id.to_string(), |n| n.to_string())),
        None => "query-page".to_owned(),
    }
}

/// `empty( $_GET[ $page_key ] ) ? 1 : (int) $_GET[ $page_key ]`.
fn custom_page(bc: &BlockCtx<'_>) -> u64 {
    QueryVars::parse(&bc.env.query.raw_query).u64(&page_key(bc)).filter(|p| *p > 0).unwrap_or(1)
}

/// [`run_custom_query`], once per render for the same query, page and current post: the post template, the
/// total, the no-results block and every pagination block of a query ask for the same loop.
async fn custom_query(bc: &BlockCtx<'_>, page: u64) -> Result<LoopData> {
    let key = format!("{}|{page}|{}", query_ctx(bc).map(|q| Value::Object(q.clone()).to_string()).unwrap_or_default(), bc.context.post.as_ref().map_or(0, |p| p.id));
    if let Some(data) = bc.env.with_state(|s| s.query_loops.get(&key).cloned()) {
        return Ok(data);
    }
    let data = run_custom_query(bc, page).await?;
    bc.env.with_state(|s| s.query_loops.insert(key, data.clone()));
    Ok(data)
}

/// `build_query_vars_from_query_block` + running the query.
async fn run_custom_query(bc: &BlockCtx<'_>, page: u64) -> Result<LoopData> {
    let site = bc.env.site;
    let q = query_ctx(bc).cloned().unwrap_or_default();
    let get = |k: &str| q.get(k).filter(|v| !crate::style_engine::is_empty(v));

    let mut query = PostQuery {
        post_types: vec![get("postType").and_then(Value::as_str).unwrap_or("post").to_owned()],
        ..PostQuery::default()
    };
    let mut ignore_sticky = false;
    let sticky = &site.settings.sticky_posts;
    match get("sticky").and_then(Value::as_str) {
        Some("only") => {
            query.ids = if sticky.is_empty() { vec![0] } else { sticky.clone() };
            ignore_sticky = true;
        }
        Some("exclude") => query.exclude_ids.extend(sticky.iter().copied()),
        Some("ignore") => ignore_sticky = true,
        _ => {}
    }
    if let Some(excluded) = get("exclude").and_then(Value::as_array) {
        query.exclude_ids.extend(excluded.iter().filter_map(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).filter(|id| *id != 0));
    }
    if get("excludeCurrent").is_some()
        && let Some(post) = &bc.context.post {
            query.exclude_ids.push(post.id);
        }
    let per_page = q.get("perPage").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
    let effective_per_page = per_page.unwrap_or(site.settings.posts_per_page).max(1);
    let offset = q.get("offset").and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))).unwrap_or(0);
    query.limit = Some(effective_per_page);
    if per_page.is_some() {
        query.offset = effective_per_page.saturating_mul(page.saturating_sub(1)).saturating_add(offset);
    }
    // Taxonomy filters (term IDs).
    let mut has_tax = false;
    if let Some(tax_query) = get("taxQuery").and_then(Value::as_object) {
        for (taxonomy, terms) in tax_query {
            let ids: Vec<u64> = terms.as_array().map(|a| a.iter().filter_map(Value::as_u64).collect()).unwrap_or_default();
            let mut group = Vec::new();
            for id in ids {
                if let Some(term) = site.db.term_by_id(Some(taxonomy), id).await? {
                    group.push(term.term_taxonomy_id);
                }
            }
            if !group.is_empty() {
                query.tax_groups.push(group);
                has_tax = true;
            }
        }
    }
    match q.get("order").and_then(Value::as_str).map(str::to_ascii_uppercase).as_deref() {
        Some("ASC") => query.order = Order::Asc,
        _ => query.order = Order::Desc,
    }
    query.order_by = match q.get("orderBy").and_then(Value::as_str) {
        Some("title") => OrderBy::Title,
        Some("modified") => OrderBy::Modified,
        Some("ID" | "id") => OrderBy::Id,
        Some("menu_order") => OrderBy::MenuOrder,
        _ => OrderBy::Date,
    };
    let mut has_author = false;
    match q.get("author") {
        Some(Value::String(s)) if !s.is_empty() => {
            // Multiple authors are not supported by the DB layer yet: the first one wins.
            if let Some(id) = s.split(',').filter_map(|p| p.trim().parse::<u64>().ok()).find(|id| *id != 0) {
                query.author = Some(id);
                has_author = true;
            }
        }
        Some(Value::Number(n)) if n.as_u64().is_some_and(|n| n > 0) => {
            query.author = n.as_u64();
            has_author = true;
        }
        _ => {}
    }
    let mut has_search = false;
    if let Some(search) = get("search").and_then(Value::as_str) {
        query.search = parse_search_terms(search, false);
        query.search_phrase = Some(search.to_owned());
        query.order_by = OrderBy::Relevance;
        has_search = true;
    }
    if let Some(parents) = get("parents").and_then(Value::as_array)
        && let Some(parent) = parents.iter().filter_map(Value::as_u64).next() {
            query.parent = Some(parent);
        }
    // `WP_Query` without a status: a signed-in viewer also gets the private posts they may read.
    if let Some(viewer) = bc.env.query.viewer.as_ref().filter(|v| v.id != 0) {
        query.private_types = query.post_types.iter().filter(|t| viewer.can_read_private(t)).cloned().collect();
        query.private_author = Some(viewer.id);
    }

    let found = site.db.count_posts(&query).await?;
    let mut posts = site.db.posts(&query).await?;
    site.db.prime_loop(&posts).await?;
    // Custom queries without archive/search flags behave like the blog home: sticky posts first.
    if !ignore_sticky && !has_tax && !has_author && !has_search && page <= 1 && !sticky.is_empty() && query.post_types == ["post"] {
        let (mut front, rest): (Vec<Post>, Vec<Post>) = posts.into_iter().partition(|p| sticky.contains(&p.id));
        let missing: Vec<u64> = sticky.iter().copied().filter(|id| !front.iter().any(|p| p.id == *id) && !query.exclude_ids.contains(id)).collect();
        if !missing.is_empty() {
            front.extend(site.db.posts(&PostQuery { ids: missing, ..PostQuery::default() }).await?);
        }
        front.extend(rest);
        posts = front;
    }
    let max_pages = found.div_ceil(effective_per_page);
    Ok(LoopData { posts, max_pages, found, per_page: effective_per_page })
}

pub(super) async fn loop_data(bc: &BlockCtx<'_>) -> Result<LoopData> {
    if is_inherit(bc) {
        let q = bc.env.query;
        return Ok(LoopData { posts: q.posts.clone(), max_pages: q.max_pages, found: q.found_posts, per_page: q.per_page });
    }
    custom_query(bc, custom_page(bc)).await
}

/// `core/query`: the saved wrapper plus, for enhanced pagination, router directives.
pub(super) fn query(bc: &BlockCtx<'_>) -> Result<String> {
    let interactive = bc.attrs.get("enhancedPagination").and_then(Value::as_bool) == Some(true) && bc.attrs.contains_key("queryId");
    if !interactive {
        return Ok(bc.content.clone());
    }
    bc.env.with_state(|s| s.script_modules.insert("@wordpress/block-library/query/view".to_owned()));
    let Some(mut editor) = crate::html::TagEditor::first(&bc.content) else { return Ok(bc.content.clone()) };
    let id = bc.attrs.get("queryId").map(|v| v.as_u64().map_or_else(|| v.to_string(), |n| n.to_string())).unwrap_or_default();
    editor.set_attribute("data-wp-interactive", "core/query");
    editor.set_attribute("data-wp-router-region", &format!("query-{id}"));
    editor.set_attribute("data-wp-context", "{}");
    editor.set_attribute("data-wp-key", &id);
    Ok(editor.finish())
}

pub(super) async fn post_template(bc: &BlockCtx<'_>) -> Result<String> {
    let data = loop_data(bc).await?;
    if data.posts.is_empty() {
        return Ok(String::new());
    }
    let mut classnames = String::new();
    if let (Some(display), Some(_)) = (bc.context.get("displayLayout"), bc.context.get("query"))
        && display.get("type").and_then(Value::as_str) == Some("flex") {
            classnames = format!("is-flex-container columns-{}", display.get("columns").map(|c| c.as_u64().map_or_else(|| c.to_string(), |n| n.to_string())).unwrap_or_default());
        }
    if bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null()) {
        classnames.push_str(" has-link-color");
    }
    let layout = bc.attrs.get("layout").and_then(Value::as_object);
    if layout.and_then(|l| l.get("type")).and_then(Value::as_str) == Some("grid")
        && let Some(count) = layout.and_then(|l| l.get("columnCount")).filter(|v| !crate::style_engine::is_empty(v)) {
            classnames.push_str(&format!(" {}", rustpress_core::slug::sanitize_title(&format!("columns-{}", count.as_u64().map_or_else(|| count.to_string(), |n| n.to_string())))));
            if layout.and_then(|l| l.get("minimumColumnWidth")).is_some_and(|v| !crate::style_engine::is_empty(v)) {
                classnames.push_str(" has-native-responsive-grid");
            }
        }
    let wrapper = bc.wrapper_attributes(&[("class", classnames.trim().to_owned())]);

    let enhanced = bc.context.get("enhancedPagination").and_then(Value::as_bool).unwrap_or(false);
    let flags = &bc.env.query.flags;
    let mut items = String::new();
    // An inheriting block iterates the main query itself, so `in_the_loop()` is true meanwhile
    // (a nested one clones it, which leaves the flag as it was).
    let inherit = is_inherit(bc);
    let was_in_loop = bc.env.with_state(|s| {
        let before = s.media.in_main_loop;
        s.media.in_main_loop = before || inherit;
        before
    });
    for post in &data.posts {
        let post = Arc::new(post.clone());
        let mut ctx = Context::default();
        ctx.values.insert("postType".into(), Value::from(post.post_type.clone()));
        ctx.values.insert("postId".into(), Value::from(post.id));
        ctx.post = Some(post.clone());

        let mut inner = String::new();
        let mut index = 0;
        for chunk in &bc.block.inner_content {
            match chunk {
                Content::Html(html) => inner.push_str(html),
                Content::InnerBlock => {
                    if let Some(block) = bc.block.inner_blocks.get(index) {
                        inner.push_str(&bc.env.render_block(block, &ctx, Some(bc.block)).await?);
                    }
                    index += 1;
                }
            }
        }
        let classes = post_class(bc.env, &post, &["wp-block-post"], flags.is_home && !flags.is_paged, flags.is_singular()).await?;
        let directives = if enhanced { format!(" data-wp-key=\"post-template-item-{}\"", post.id) } else { String::new() };
        items.push_str(&format!("<li{directives} class=\"{}\">{inner}</li>", esc_attr(&classes.join(" "))));
    }
    bc.env.with_state(|s| s.media.in_main_loop = was_in_loop);
    Ok(format!("<ul {wrapper}>{items}</ul>"))
}

/// `core/query-total`.
pub(super) async fn query_total(bc: &BlockCtx<'_>) -> Result<String> {
    let data = loop_data(bc).await?;
    let current_page = if is_inherit(bc) { bc.env.query.vars.u64("paged").unwrap_or(1).max(1) } else { custom_page(bc) } as i64;
    let (found, per_page) = (data.found as i64, data.per_page as i64);
    let site = bc.env.site;
    let start = if found == 0 { 0 } else { (current_page - 1) * per_page + 1 };
    let end = (start + per_page - 1).min(found);
    let output = if bc.attr_str("displayType") == Some("range-display") {
        if start == end {
            rustpress_core::i18n::sprintf(&site.tr("Displaying %1$s of %2$s"), &[&start.to_string(), &found.to_string()])
        } else {
            rustpress_core::i18n::sprintf(&site.tr("Displaying %1$s – %2$s of %3$s"), &[&start.to_string(), &end.to_string(), &found.to_string()])
        }
    } else {
        rustpress_core::i18n::sprintf(&site.trn("%d result found", "%d results found", found), &[&found.to_string()])
    };
    Ok(format!("<div {}>{output}</div>", bc.wrapper_attributes(&[])))
}

pub(super) async fn query_no_results(bc: &BlockCtx<'_>) -> Result<String> {
    if bc.content.trim().is_empty() {
        return Ok(String::new());
    }
    let count = if is_inherit(bc) { bc.env.query.posts.len() } else { custom_query(bc, custom_page(bc)).await?.posts.len() };
    if count > 0 {
        return Ok(String::new());
    }
    let class = if bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null()) { "has-link-color" } else { "" };
    let wrapper = bc.wrapper_attributes(&[("class", class.to_owned())]);
    Ok(format!("<div {wrapper}>{}</div>", bc.content))
}

// ----------------------------------------------------------------- query title

pub(super) async fn query_title(bc: &BlockCtx<'_>) -> Result<String> {
    let q = bc.env.query;
    let kind = bc.attr_str("type");
    let is_archive = q.flags.is_archive;
    let is_search = q.flags.is_search;
    let Some(kind) = kind else { return Ok(String::new()) };
    if (kind == "archive" && !is_archive) || (kind == "search" && !is_search) {
        return Ok(String::new());
    }
    let mut title = String::new();
    if is_archive {
        title = archive_title(bc);
    }
    if is_search {
        title = bc.env.site.tr("Search results");
        if bc.attrs.get("showSearchTerm").is_some_and(|v| !crate::style_engine::is_empty(v)) {
            let term = q.vars.get("s").unwrap_or("");
            title = rustpress_core::i18n::sprintf(&bc.env.site.tr("Search results for: &#8220;%s&#8221;"), &[&esc_attr(term)]);
        }
    }
    if kind == "post-type" {
        return Ok(String::new());
    }
    let level = bc.attrs.get("level").and_then(Value::as_u64).unwrap_or(1);
    let tag = if level == 0 { "p".to_owned() } else { format!("h{level}") };
    let align = bc.attr_str("textAlign").filter(|a| !a.is_empty()).map(|a| format!("has-text-align-{a}")).unwrap_or_default();
    let wrapper = bc.wrapper_attributes(&[("class", align)]);
    Ok(format!("<{tag} {wrapper}>{title}</{tag}>"))
}

/// `get_the_archive_title()` for the standard archives.
fn archive_title(bc: &BlockCtx<'_>) -> String {
    use rustpress_core::Queried;
    let q = bc.env.query;
    let f = &q.flags;
    let site = bc.env.site;
    let show_prefix = bc.attrs.get("showPrefix").and_then(Value::as_bool).unwrap_or(true);
    let (title, prefix) = match &q.queried {
        Queried::Term(term) if f.is_category => (term.name.clone(), site.trx("Category:", "category archive title prefix")),
        Queried::Term(term) if f.is_tag => (term.name.clone(), site.trx("Tag:", "tag archive title prefix")),
        Queried::User(user) if f.is_author => (user.display_name.clone(), site.trx("Author:", "author archive title prefix")),
        Queried::PostType(post_type) if f.is_post_type_archive => (site.post_type_label(post_type), site.trx("Archives:", "post type archive title prefix")),
        Queried::Term(term) if f.is_tax => (term.name.clone(), rustpress_core::i18n::sprintf(&site.trx("%s:", "taxonomy term archive title prefix"), &[&site.taxonomy_singular_label(&term.taxonomy)])),
        _ if f.is_year || f.is_month || f.is_day => {
            let vars = &q.vars;
            let year = vars.u64("year").unwrap_or(0) as u32;
            let month = vars.u64("monthnum").unwrap_or(0) as u32;
            let day = vars.u64("day").unwrap_or(0) as u32;
            let first = q.posts.first();
            let date_format = if f.is_day { site.trx("F j, Y", "daily archives date format") } else if f.is_month { site.trx("F Y", "monthly archives date format") } else { site.trx("Y", "yearly archives date format") };
            let ts = first
                .and_then(|p| site.timezone.local_timestamp(&p.post_date))
                .or_else(|| site.timezone.local_timestamp(&format!("{year:04}-{:02}-{:02} 00:00:00", month.max(1), day.max(1))));
            let title = ts.map(|ts| site.date(&date_format, ts)).unwrap_or_default();
            (title, site.trx(if f.is_day { "Day:" } else if f.is_month { "Month:" } else { "Year:" }, "date archive title prefix"))
        }
        _ => (site.tr("Archives"), String::new()),
    };
    if show_prefix && !prefix.is_empty() {
        format!("{prefix} <span>{}</span>", esc_html_title(&title))
    } else if !prefix.is_empty() {
        format!("<span>{}</span>", esc_html_title(&title))
    } else {
        esc_html_title(&title)
    }
}

fn esc_html_title(text: &str) -> String {
    esc_html(text)
}

// ------------------------------------------------------------------ pagination

pub(super) fn query_pagination(bc: &BlockCtx<'_>) -> Result<String> {
    if bc.content.trim().is_empty() {
        return Ok(String::new());
    }
    let class = if bc.attrs.get("style").and_then(|s| s.get("elements")).and_then(|e| e.get("link")).and_then(|l| l.get("color")).and_then(|c| c.get("text")).is_some_and(|v| !v.is_null()) { "has-link-color" } else { "" };
    let wrapper = bc.wrapper_attributes(&[("aria-label", bc.env.site.tr("Pagination")), ("class", class.to_owned())]);
    Ok(format!("<nav {wrapper}>{}</nav>", bc.content))
}

/// `get_query_pagination_arrow`.
fn pagination_arrow(bc: &BlockCtx<'_>, is_next: bool) -> Option<String> {
    let kind = bc.context.get("paginationArrow").and_then(Value::as_str)?;
    let arrow = match (kind, is_next) {
        ("arrow", true) => "→",
        ("arrow", false) => "←",
        ("chevron", true) => "»",
        ("chevron", false) => "«",
        _ => return None,
    };
    let direction = if is_next { "next" } else { "previous" };
    Some(format!("<span class='wp-block-query-pagination-{direction}-arrow is-arrow-{kind}' aria-hidden='true'>{arrow}</span>"))
}

fn label_for(bc: &BlockCtx<'_>, default: &str) -> (String, String) {
    let show = bc.context.get("showLabel").and_then(Value::as_bool).unwrap_or(true);
    let text = bc.attr_str("label").filter(|l| !l.is_empty()).map(esc_html).unwrap_or_else(|| default.to_owned());
    let label = if show { text.clone() } else { String::new() };
    (label, text)
}

fn replace_ampersands(label: &str) -> String {
    rustpress_core::formatting::convert_chars(label).replace("&#038;#", "&#")
}

pub(super) async fn pagination_link(bc: &BlockCtx<'_>, is_next: bool) -> Result<String> {
    let key = page_key(bc);
    let direction = if is_next { "next" } else { "previous" };
    let mut wrapper = bc.wrapper_attributes(&[]);
    let (mut label, text) = label_for(bc, &bc.env.site.tr(if is_next { "Next Page" } else { "Previous Page" }));
    let arrow = pagination_arrow(bc, is_next);
    if label.is_empty() {
        wrapper.push_str(&format!(" aria-label=\"{text}\""));
    }
    if let Some(a) = arrow {
        label = if is_next { format!("{label}{a}") } else { format!("{a}{label}") };
    }
    let max_page = query_ctx(bc).and_then(|q| q.get("pages")).and_then(Value::as_u64).unwrap_or(0);
    let site = bc.env.site;
    let q = bc.env.query;
    let request = format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) });

    let mut content = String::new();
    if is_inherit(bc) {
        let paged = q.paged.max(1);
        let target = if is_next { paged + 1 } else { paged.saturating_sub(1) };
        let max = if is_next {
            if max_page != 0 && max_page < q.max_pages { max_page } else { q.max_pages }
        } else {
            0
        };
        let available = if is_next { !q.flags.is_single && target <= max } else { !q.flags.is_single && paged > 1 };
        if available {
            content = format!("<a href=\"{}\" {wrapper}>{}</a>", esc_url(&site.pagenum_link(&request, target.max(1))), replace_ampersands(&label));
        }
    } else {
        let page = custom_page(bc);
        let data = custom_query(bc, page).await?;
        let total = if max_page == 0 || max_page > data.max_pages { data.max_pages } else { max_page };
        let available = if is_next { (max_page == 0 || max_page > page) && data.max_pages != 0 && data.max_pages != page } else { page > 1 && page <= total };
        if available {
            let target = if is_next { page + 1 } else { page - 1 };
            content = format!("<a href=\"{}\" {wrapper}>{label}</a>", esc_url(&add_query_arg(&request, &key, &target.to_string())));
        }
    }
    if bc.context.get("enhancedPagination").and_then(Value::as_bool).unwrap_or(false) && !content.is_empty()
        && let Some(mut editor) = crate::html::TagEditor::first(&content)
            && editor.has_class(&format!("wp-block-query-pagination-{direction}")) {
                editor.set_attribute("data-wp-key", &format!("query-pagination-{direction}"));
                editor.set_attribute("data-wp-on--click", "core/query::actions.navigate");
                editor.set_attribute("data-wp-on--mouseenter", "core/query::actions.prefetch");
                editor.set_attribute("data-wp-watch", "core/query::callbacks.prefetch");
                content = editor.finish();
            }
    Ok(content)
}

fn number_format(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `paginate_links()` with `prev_next = false`.
pub(super) fn paginate_links(base: &str, format: &str, total: u64, current: u64, mid_size: u64, add_args: &[(String, String)], fragment: &str) -> Option<String> {
    if total < 2 {
        return None;
    }
    let end_size = 1;
    let mut links: Vec<String> = Vec::new();
    let mut dots = false;
    for n in 1..=total {
        if n == current {
            links.push(format!("<span aria-current=\"page\" class=\"page-numbers current\">{}</span>", number_format(n)));
            dots = true;
        } else if n <= end_size || (current > 0 && n + mid_size >= current && n <= current + mid_size) || n > total.saturating_sub(end_size) {
            let mut link = base.replace("%_%", if n == 1 { "" } else { format });
            link = link.replace("%#%", &n.to_string());
            for (k, v) in add_args {
                link = add_query_arg(&link, k, v);
            }
            link.push_str(fragment);
            links.push(format!("<a class=\"page-numbers\" href=\"{}\">{}</a>", esc_url(&link), number_format(n)));
            dots = true;
        } else if dots {
            links.push("<span class=\"page-numbers dots\">&hellip;</span>".to_owned());
            dots = false;
        }
    }
    Some(links.join("\n"))
}

/// `urlencode_deep()`: PHP's `urlencode()`, spaces become `+`.
fn url_encode(value: &str) -> String {
    rustpress_core::canonical::php_urlencode(value)
}

pub(super) async fn pagination_numbers(bc: &BlockCtx<'_>) -> Result<String> {
    let max_page = query_ctx(bc).and_then(|q| q.get("pages")).and_then(Value::as_u64).unwrap_or(0);
    let mid_size = bc.attrs.get("midSize").and_then(Value::as_u64).unwrap_or(2);
    let site = bc.env.site;
    let q = bc.env.query;
    let wrapper = bc.wrapper_attributes(&[]);

    let content = if is_inherit(bc) {
        let total = if max_page == 0 || max_page > q.max_pages { q.max_pages } else { max_page };
        let request = format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) });
        let first = site.pagenum_link(&request, 1);
        let (path, query) = first.split_once('?').unwrap_or((first.as_str(), ""));
        let pretty = site.uses_pretty_permalinks();
        let base = if pretty && !site.settings.permalink_structure.ends_with('/') { rustpress_core::urls::untrailingslashit(path) } else { rustpress_core::urls::trailingslashit(path) };
        let base = format!("{base}%_%");
        let index = if site.using_index_permalinks() && !base.contains("index.php") { "index.php/" } else { "" };
        let format = if pretty {
            let f = format!("{index}{}", site.user_trailingslashit("page/%#%"));
            if site.settings.permalink_structure.ends_with('/') { f } else { format!("/{}", f.trim_start_matches('/')) }
        } else {
            "?paged=%#%".to_owned()
        };
        let args: Vec<(String, String)> = rustpress_core::canonical::parse_qs(query).into_iter().filter(|(k, _)| k != "paged").map(|(k, v)| (k, url_encode(&v))).collect();
        paginate_links(&base, &format, total, q.paged.max(1), mid_size, &args, "")
    } else {
        let page = custom_page(bc);
        let data = custom_query(bc, page).await?;
        let total = if max_page == 0 || max_page > data.max_pages { data.max_pages } else { max_page };
        let key = page_key(bc);
        let mut args = Vec::new();
        if page != 1 {
            args.push(("cst".to_owned(), String::new()));
        }
        if let Some(paged) = QueryVars::parse(&q.raw_query).u64("paged") {
            args = vec![("paged".to_owned(), paged.to_string())];
        }
        // `paginate_links()` also carries the other query arguments of the requested URL
        // (`get_pagenum_link()` without `paged`), except the page key itself.
        for (k, v) in rustpress_core::canonical::parse_qs(&q.raw_query) {
            if k == "paged" || k == key {
                continue;
            }
            let value = url_encode(&v);
            match args.iter_mut().find(|(existing, _)| *existing == k) {
                Some(slot) => slot.1 = value,
                None => args.push((k, value)),
            }
        }
        paginate_links("%_%", &format!("?{key}=%#%"), total, page.max(1), mid_size, &args, "")
    };
    let Some(mut content) = content else { return Ok(String::new()) };
    if content.is_empty() {
        return Ok(String::new());
    }
    if bc.context.get("enhancedPagination").and_then(Value::as_bool).unwrap_or(false) {
        content = enhance_numbers(&content);
    }
    Ok(format!("<div {wrapper}>{content}</div>"))
}

fn enhance_numbers(content: &str) -> String {
    let mut out = content.to_owned();
    let mut index = 0;
    let mut from = 0;
    loop {
        let all = crate::html::tags(&out, from);
        let Some(tag) = all.into_iter().find(|t| !t.is_closer && t.attr("class").is_some_and(|c| c.split_whitespace().any(|x| x == "page-numbers"))) else { break };
        let mut editor = crate::html::TagEditor::from_tag(&out, tag.clone());
        if editor.get_attribute("data-wp-key").is_none() {
            editor.set_attribute("data-wp-key", &format!("index-{index}"));
            index += 1;
        }
        if tag.name == "a" {
            editor.set_attribute("data-wp-on--click", "core/query::actions.navigate");
        }
        out = editor.finish();
        from = tag.start + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_get_thousand_separators() {
        assert_eq!(number_format(7), "7");
        assert_eq!(number_format(1000), "1,000");
        assert_eq!(number_format(1234567), "1,234,567");
    }

    #[test]
    fn paginate_links_with_current_page_and_dots() {
        let out = paginate_links("http://x/%_%", "page/%#%/", 10, 5, 1, &[], "").unwrap();
        assert_eq!(
            out,
            "<a class=\"page-numbers\" href=\"http://x/\">1</a>\n<span class=\"page-numbers dots\">&hellip;</span>\n<a class=\"page-numbers\" href=\"http://x/page/4/\">4</a>\n<span aria-current=\"page\" class=\"page-numbers current\">5</span>\n<a class=\"page-numbers\" href=\"http://x/page/6/\">6</a>\n<span class=\"page-numbers dots\">&hellip;</span>\n<a class=\"page-numbers\" href=\"http://x/page/10/\">10</a>"
        );
    }

    #[test]
    fn paginate_links_keeps_extra_query_arguments() {
        let out = paginate_links("http://x/%_%", "page/%#%/", 2, 1, 2, &[("s".to_owned(), "Beitrag".to_owned())], "").unwrap();
        assert!(out.contains("href=\"http://x/page/2/?s=Beitrag\""), "{out}");
        assert!(paginate_links("http://x/%_%", "page/%#%/", 1, 1, 2, &[], "").is_none());
    }

    #[test]
    fn query_string_values_are_encoded() {
        assert_eq!(url_encode("a b&c"), "a+b%26c");
    }
}
