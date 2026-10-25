//! `wp-admin/edit.php`: the list of posts, pages and other post types.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use rustpress_db::{OrderBy, Order, Post, PostQuery};

use super::notices;
use super::{AdminResponse, Ctx, redirect, render};
use crate::chrome::{Assets, Screen};
use crate::content::Content;
use crate::list_table::{Column, Table, row_actions, submit_button, view_link, views};

/// The statuses `show_in_admin_all_list` leaves in "All".
const ALL_STATUSES: [&str; 5] = ["publish", "future", "draft", "pending", "private"];

/// A post type the list works with.
pub struct TypeInfo {
    pub name: String,
    pub hierarchical: bool,
    pub label: String,
    pub singular: String,
}

pub fn type_info(ctx: &Ctx<'_>, name: &str) -> Option<TypeInfo> {
    let site = ctx.site();
    match name {
        "post" => Some(TypeInfo { name: "post".into(), hierarchical: false, label: site.post_type_label("post"), singular: site.trx("Post", "post type singular name") }),
        "page" => Some(TypeInfo { name: "page".into(), hierarchical: true, label: site.post_type_label("page"), singular: site.trx("Page", "post type singular name") }),
        other => site.types.post_type(other).filter(|t| t.public || t.queryable).map(|t| TypeInfo { name: other.to_owned(), hierarchical: t.hierarchical, label: t.plural_label(), singular: t.singular_label() }),
    }
}

/// `post_class()` for a list row.
fn post_class(post: &Post, categories: &[rustpress_db::Term], tags: &[rustpress_db::Term], sticky: bool, thumbnail: bool) -> String {
    let mut classes = vec![format!("post-{}", post.id), format!("type-{}", post.post_type), format!("status-{}", post.post_status)];
    if post.post_type == "post" {
        classes.push("format-standard".to_owned());
    }
    if thumbnail {
        classes.push("has-post-thumbnail".to_owned());
    }
    if sticky {
        classes.push("status-sticky".to_owned());
    }
    classes.push("hentry".to_owned());
    for term in categories {
        classes.push(format!("category-{}", rustpress_core::formatting::sanitize_html_class(&term.slug)));
    }
    for term in tags {
        classes.push(format!("tag-{}", rustpress_core::formatting::sanitize_html_class(&term.slug)));
    }
    classes.join(" ")
}

/// `_draft_or_post_title()`.
fn draft_or_title(ctx: &Ctx<'_>, post: &Post) -> String {
    if post.post_title.is_empty() { ctx.tr("(no title)") } else { esc_html(&post.post_title) }
}

fn sticky_ids(ctx: &Ctx<'_>) -> Vec<u64> {
    let raw = ctx.site().options.string("sticky_posts", "");
    match rustpress_phpser::unserialize(raw.as_bytes()) {
        Ok(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_i64()).map(|i| i as u64).collect(),
        _ => Vec::new(),
    }
}

/// The IDs of a category and everything below it, as `term_taxonomy_id`s.
async fn category_tt_ids(ctx: &Ctx<'_>, cat: u64) -> Vec<u64> {
    let terms = ctx.env.db.terms("category").await.unwrap_or_default();
    let mut ids = vec![cat];
    loop {
        let before = ids.len();
        for term in &terms {
            if ids.contains(&term.parent) && !ids.contains(&term.term_id) {
                ids.push(term.term_id);
            }
        }
        if ids.len() == before {
            break;
        }
    }
    terms.iter().filter(|t| ids.contains(&t.term_id)).map(|t| t.term_taxonomy_id).collect()
}

/// `wp_dropdown_categories()` with `show_option_all` and a hierarchy.
async fn categories_dropdown(ctx: &Ctx<'_>, selected: u64) -> String {
    let terms = ctx.env.db.terms("category").await.unwrap_or_default();
    let mut out = format!("<label class=\"screen-reader-text\" for=\"cat\">{}</label>", ctx.site().trx("Filter by category", "category filter label"));
    out.push_str("<select  name='cat' id='cat' class='postform'>\n");
    out.push_str(&format!("\t<option value='0'>{}</option>\n", ctx.site().trx("All Categories", "all categories")));
    fn walk(terms: &[rustpress_db::Term], parent: u64, level: usize, selected: u64, out: &mut String) {
        let mut children: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.parent == parent).collect();
        children.sort_by_key(|t| t.name.to_lowercase());
        for term in children {
            let pad = "&nbsp;".repeat(level * 3);
            let sel = if term.term_id == selected { " selected=\"selected\"" } else { "" };
            out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\"{sel}>{pad}{}</option>\n", term.term_id, esc_html(&term.name)));
            walk(terms, term.term_id, level + 1, selected, out);
        }
    }
    walk(&terms, 0, 0, selected, &mut out);
    out.push_str("</select>\n");
    out
}

/// The notice after a bulk action (`bulk_post_updated_messages`).
fn result_notice(ctx: &Ctx<'_>, info: &TypeInfo) -> String {
    let site = ctx.site();
    let count = |name: &str| ctx.request.get(name).and_then(|v| v.parse::<i64>().ok());
    let (post, posts) = if info.name == "page" { (("%s page updated.", "%s pages updated."), ()) } else { (("%s post updated.", "%s posts updated."), ()) };
    let _ = posts;
    let mut messages: Vec<String> = Vec::new();
    let number = |n: i64| site.number_i18n(n.unsigned_abs());
    if let Some(n) = count("updated").filter(|n| *n > 0) {
        messages.push(rustpress_core::i18n::sprintf(&site.trn(post.0, post.1, n), &[&number(n)]));
    }
    if let Some(n) = count("deleted").filter(|n| *n > 0) {
        let singular = if info.name == "page" { "%s page permanently deleted." } else { "%s post permanently deleted." };
        let plural = if info.name == "page" { "%s pages permanently deleted." } else { "%s posts permanently deleted." };
        messages.push(rustpress_core::i18n::sprintf(&site.trn(singular, plural, n), &[&number(n)]));
    }
    if let Some(n) = count("trashed").filter(|n| *n > 0) {
        let singular = if info.name == "page" { "%s page moved to the Trash." } else { "%s post moved to the Trash." };
        let plural = if info.name == "page" { "%s pages moved to the Trash." } else { "%s posts moved to the Trash." };
        let mut message = rustpress_core::i18n::sprintf(&site.trn(singular, plural, n), &[&number(n)]);
        if let Some(ids) = ctx.request.get("ids").filter(|i| !i.is_empty()) {
            let url = format!("{}&_wpnonce={}", ctx.admin_url(&format!("edit.php?doaction=undo&action=untrash&ids={}", rustpress_core::canonical::php_urlencode(ids))), ctx.create_nonce("bulk-posts"));
            message.push_str(&format!(" <a href=\"{}\">{}</a>", esc_url(&url), ctx.tr("Undo")));
        }
        messages.push(message);
    }
    if let Some(n) = count("untrashed").filter(|n| *n > 0) {
        let singular = if info.name == "page" { "%s page restored from the Trash." } else { "%s post restored from the Trash." };
        let plural = if info.name == "page" { "%s pages restored from the Trash." } else { "%s posts restored from the Trash." };
        messages.push(rustpress_core::i18n::sprintf(&site.trn(singular, plural, n), &[&number(n)]));
    }
    if messages.is_empty() {
        return String::new();
    }
    notices::notice(&messages.join(" "), "success", true, "message", &["updated"], true).replace("<div id=\"message\" class=\"notice notice-success is-dismissible updated\">", "<div id=\"message\" class=\"updated notice notice-success is-dismissible\">")
}

/// Handles a bulk action (and `doaction=undo`); returns the redirect.
async fn bulk_action(ctx: &Ctx<'_>, info: &TypeInfo) -> Option<AdminResponse> {
    let request = ctx.request;
    let mut action = request.request("action").filter(|a| *a != "-1").map(str::to_owned);
    if action.is_none() {
        action = request.request("action2").filter(|a| *a != "-1").map(str::to_owned);
    }
    if request.request("delete_all").is_some() || request.request("delete_all2").is_some() {
        action = Some("delete_all".to_owned());
    }
    if request.request("filter_action").is_some() {
        return None;
    }
    let action = action?;
    let writer = ctx.env.writer?;
    if !ctx.verify_nonce("bulk-posts") {
        return Some(super::die(ctx.env, 403, &ctx.tr("WordPress Failure Notice"), &ctx.tr("The link you followed has expired.")));
    }
    let mut ids: Vec<u64> = request.query.iter().chain(request.form.iter()).filter(|(k, _)| k == "post[]" || k == "post").filter_map(|(_, v)| v.parse().ok()).collect();
    if let Some(list) = request.request("ids") {
        ids.extend(list.split(',').filter_map(|i| i.trim().parse::<u64>().ok()));
    }
    let mut sendback = format!("edit.php?post_type={}", info.name);
    if info.name == "post" {
        sendback = "edit.php".to_owned();
    }
    if let Some(status) = request.request("post_status").filter(|s| !s.is_empty() && *s != "all") {
        sendback = add_query_arg(&sendback, "post_status", status);
    }
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let mut done = 0i64;
    let key;
    match action.as_str() {
        "trash" => {
            key = "trashed";
            for id in &ids {
                if content.trash_post(*id).await.unwrap_or(false) {
                    done += 1;
                }
            }
            sendback = add_query_arg(&sendback, "ids", &ids.iter().map(u64::to_string).collect::<Vec<_>>().join(","));
        }
        "untrash" => {
            key = "untrashed";
            for id in &ids {
                if content.untrash_post(*id).await.unwrap_or(false) {
                    done += 1;
                }
            }
        }
        "delete" => {
            key = "deleted";
            for id in &ids {
                if content.delete_post(*id, true).await.unwrap_or(false) {
                    done += 1;
                }
            }
        }
        "delete_all" => {
            key = "deleted";
            let trashed = PostQuery { post_types: vec![info.name.clone()], statuses: vec!["trash".into()], ..Default::default() };
            for post in ctx.env.db.posts(&trashed).await.unwrap_or_default() {
                if content.delete_post(post.id, true).await.unwrap_or(false) {
                    done += 1;
                }
            }
        }
        _ => return None,
    }
    sendback = add_query_arg(&sendback, key, &done.to_string());
    Some(redirect(&ctx.admin_url(&sendback)))
}

/// Flattens pages into the order of a hierarchical list: parents first, children below them.
fn hierarchical_order(posts: Vec<Post>) -> Vec<(Post, usize)> {
    let ids: Vec<u64> = posts.iter().map(|p| p.id).collect();
    let mut out: Vec<(Post, usize)> = Vec::new();
    fn walk(posts: &[Post], parent: u64, level: usize, out: &mut Vec<(Post, usize)>) {
        for post in posts.iter().filter(|p| p.post_parent == parent) {
            out.push((post.clone(), level));
            walk(posts, post.id, level + 1, out);
        }
    }
    // Roots are the pages whose parent is not in the list.
    for post in posts.iter().filter(|p| p.post_parent == 0 || !ids.contains(&p.post_parent)) {
        out.push((post.clone(), 0));
        walk(&posts, post.id, 1, &mut out);
    }
    out
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let site = ctx.site();
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let post_type = request.get("post_type").filter(|t| !t.is_empty()).unwrap_or("post").to_owned();
    let Some(info) = type_info(ctx, &post_type) else {
        return super::die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Invalid post type.")));
    };
    let edit_cap = if info.name == "page" { "edit_pages" } else { "edit_posts" };
    if !ctx.can(edit_cap) {
        return super::die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), if info.name == "page" { tr("Sorry, you are not allowed to edit pages.") } else { tr("Sorry, you are not allowed to edit posts.") }));
    }
    if let Some(response) = bulk_action(ctx, &info).await {
        return response;
    }
    let db = ctx.env.db;
    let edit_base = if info.name == "post" { "edit.php".to_owned() } else { format!("edit.php?post_type={}", info.name) };
    let origin = {
        let siteurl = &site.settings.siteurl;
        match siteurl.split_once("://") {
            Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
            None => siteurl.clone(),
        }
    };
    let current_url = format!("{origin}{}", request.request_uri);

    // Parameters
    let post_status = request.get("post_status").filter(|s| ["publish", "future", "draft", "pending", "private", "trash", "auto-draft", "inherit"].contains(s)).map(str::to_owned);
    let status_param = request.get("post_status").unwrap_or("all").to_owned();
    let is_trash = post_status.as_deref() == Some("trash");
    let author = request.get("author").and_then(|a| a.parse::<u64>().ok()).filter(|a| *a != 0);
    let mut cat = request.get("cat").and_then(|a| a.parse::<u64>().ok()).unwrap_or(0);
    // `category_name=slug` (the links of the category column) becomes the `cat` of the query.
    if cat == 0
        && let Some(slug) = request.get("category_name").and_then(|s| s.rsplit('/').next()).filter(|s| !s.is_empty())
        && let Ok(Some(term)) = db.term_by_slug("category", slug).await {
            cat = term.term_id;
        }
    let tag_term = match request.get("tag").filter(|s| !s.is_empty()) {
        Some(slug) => db.term_by_slug("post_tag", slug).await.ok().flatten(),
        None => None,
    };
    let month = request.get("m").and_then(|a| a.parse::<u32>().ok()).unwrap_or(0);
    let search = request.get("s").map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let paged = request.get("paged").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
    let per_page = {
        let key = format!("{}edit_{}_per_page", db.table_prefix(), info.name);
        db.user_meta(ctx.user.id, &key).await.ok().flatten().and_then(|v| v.parse::<u64>().ok()).filter(|v| *v >= 1).unwrap_or(20)
    };

    // Counts
    let counts = db.post_status_counts(&info.name).await.unwrap_or_default();
    let count_of = |status: &str| counts.iter().find(|(s, _)| s == status).map_or(0, |(_, n)| *n);
    let total_all: u64 = ALL_STATUSES.iter().map(|s| count_of(s)).sum();
    let mine = db.count_user_posts(&info.name, ctx.user.id, &["trash", "auto-draft"]).await.unwrap_or(0);
    let sticky = if info.name == "post" { sticky_ids(ctx) } else { Vec::new() };

    // The status links
    let mut links: Vec<(String, String)> = Vec::new();
    let mut all_args = vec![("post_type", info.name.clone())];
    let mut mine_link = None;
    let mine_current = request.get("author").and_then(|a| a.parse::<u64>().ok()) == Some(ctx.user.id);
    if mine > 0 && mine != total_all {
        let url = add_query_arg(&add_query_arg(&edit_base, "post_type", &info.name), "author", &ctx.user.id.to_string());
        let label = rustpress_core::i18n::sprintf(&site.trnx("Mine <span class=\"count\">(%s)</span>", "Mine <span class=\"count\">(%s)</span>", mine as i64, "posts"), &[&site.number_i18n(mine)]);
        mine_link = Some(("mine".to_owned(), view_link(&url, &label, mine_current)));
        all_args.push(("all_posts", "1".to_owned()));
    }
    let is_base_request = {
        let keys: Vec<&str> = request.query.iter().map(|(k, _)| k.as_str()).filter(|k| *k != "paged").collect();
        keys.is_empty() || (keys == ["post_type"] && request.get("post_type") == Some(info.name.as_str())) || keys == ["mode"]
    };
    let all_url = all_args.iter().fold("edit.php".to_owned(), |url, (k, v)| add_query_arg(&url, k, v));
    let all_label = rustpress_core::i18n::sprintf(&site.trnx("All <span class=\"count\">(%s)</span>", "All <span class=\"count\">(%s)</span>", total_all as i64, "posts"), &[&site.number_i18n(total_all)]);
    links.push(("all".to_owned(), view_link(&all_url, &all_label, !mine_current && (is_base_request || request.get("all_posts").is_some()))));
    if let Some(link) = mine_link {
        links.push(link);
    }
    let status_labels: [(&str, &str, &str); 6] = [
        ("publish", "Published <span class=\"count\">(%s)</span>", "Published <span class=\"count\">(%s)</span>"),
        ("future", "Scheduled <span class=\"count\">(%s)</span>", "Scheduled <span class=\"count\">(%s)</span>"),
        ("draft", "Draft <span class=\"count\">(%s)</span>", "Drafts <span class=\"count\">(%s)</span>"),
        ("pending", "Pending <span class=\"count\">(%s)</span>", "Pending <span class=\"count\">(%s)</span>"),
        ("private", "Private <span class=\"count\">(%s)</span>", "Private <span class=\"count\">(%s)</span>"),
        ("trash", "Trash <span class=\"count\">(%s)</span>", "Trash <span class=\"count\">(%s)</span>"),
    ];
    for (status, one, many) in status_labels {
        let n = count_of(status);
        if n == 0 {
            continue;
        }
        let url = add_query_arg(&add_query_arg("edit.php", "post_status", status), "post_type", &info.name);
        let label = rustpress_core::i18n::sprintf(&site.trn(one, many, n as i64), &[&site.number_i18n(n)]);
        links.push((status.to_owned(), view_link(&url, &label, post_status.as_deref() == Some(status))));
        if status == "publish" && !sticky.is_empty() {
            let sticky_count = sticky.len() as u64;
            let url = add_query_arg(&add_query_arg("edit.php", "post_type", &info.name), "show_sticky", "1");
            let label = rustpress_core::i18n::sprintf(&site.trnx("Sticky <span class=\"count\">(%s)</span>", "Sticky <span class=\"count\">(%s)</span>", sticky_count as i64, "posts"), &[&site.number_i18n(sticky_count)]);
            links.push(("sticky".to_owned(), view_link(&url, &label, request.get("show_sticky").is_some_and(|v| !v.is_empty() && v != "0"))));
        }
    }
    if post_status.is_none() && !sticky.is_empty() && !links.iter().any(|(c, _)| c == "sticky") {
        // Sticky posts without a published status entry still get their link after "All".
    }

    // The query
    let orderby_param = request.get("orderby").unwrap_or_default().to_owned();
    let order_param = request.get("order").unwrap_or_default().to_lowercase();
    let hierarchical_display = info.hierarchical && orderby_param.is_empty();
    let order_by = match orderby_param.as_str() {
        "title" => OrderBy::Title,
        "date" => OrderBy::Date,
        "comment_count" => OrderBy::CommentCount,
        "author" => OrderBy::Author,
        "modified" => OrderBy::Modified,
        "parent" => OrderBy::Parent,
        "menu_order" => OrderBy::MenuOrder,
        _ if hierarchical_display => OrderBy::MenuOrderTitle,
        _ if matches!(post_status.as_deref(), Some("pending" | "draft")) && request.get("orderby").is_none() => OrderBy::Modified,
        _ => OrderBy::Date,
    };
    let order = if order_param == "asc" || (hierarchical_display) || (order_param.is_empty() && post_status.as_deref() == Some("pending")) { Order::Asc } else { Order::Desc };
    let mut query = PostQuery {
        post_types: vec![info.name.clone()],
        statuses: match &post_status {
            Some(status) => vec![status.clone()],
            None => ALL_STATUSES.iter().map(|s| (*s).to_owned()).collect(),
        },
        author,
        order_by,
        order,
        ..Default::default()
    };
    if !ctx.can("edit_others_posts") && !ctx.can("edit_others_pages") {
        query.author = Some(ctx.user.id);
    }
    if let Some(search) = &search {
        query.search = search.split_whitespace().map(str::to_owned).collect();
        query.search_phrase = Some(search.clone());
    }
    if cat != 0 {
        query.tax_groups.push(category_tt_ids(ctx, cat).await);
    }
    if request.get("tag").is_some_and(|slug| !slug.is_empty()) {
        // A tag that does not exist matches nothing.
        query.tax_groups.push(vec![tag_term.as_ref().map_or(0, |term| term.term_taxonomy_id)]);
    }
    if month >= 190_000 {
        query.year = Some(month / 100);
        query.month = Some(month % 100);
    }
    if request.get("show_sticky").is_some_and(|v| !v.is_empty() && v != "0") {
        query.ids = sticky.clone();
        if query.ids.is_empty() {
            query.ids = vec![0];
        }
    }
    let total_items = db.count_posts(&query).await.unwrap_or(0);
    let (posts, levels): (Vec<Post>, Vec<usize>) = if hierarchical_display {
        let all = db.posts(&query).await.unwrap_or_default();
        let flat = hierarchical_order(all);
        let start = ((paged - 1) * per_page) as usize;
        let slice: Vec<(Post, usize)> = flat.into_iter().skip(start).take(per_page as usize).collect();
        (slice.iter().map(|(p, _)| p.clone()).collect(), slice.iter().map(|(_, l)| *l).collect())
    } else {
        query.limit = Some(per_page);
        query.offset = (paged - 1) * per_page;
        let posts = db.posts(&query).await.unwrap_or_default();
        let levels = vec![0; posts.len()];
        (posts, levels)
    };

    // Related data of the rows
    let ids: Vec<u64> = posts.iter().map(|p| p.id).collect();
    let pending = db.pending_comment_counts(&ids).await.unwrap_or_default();
    let author_ids: Vec<u64> = posts.iter().map(|p| p.post_author).collect();
    let authors = db.users_by_ids(&author_ids).await.unwrap_or_default();
    let parent_ids: Vec<u64> = posts.iter().map(|p| p.post_parent).filter(|p| *p != 0).collect();
    let mut parents: Vec<Post> = Vec::new();
    for id in parent_ids {
        if let Ok(Some(parent)) = db.post(id).await {
            parents.push(parent);
        }
    }

    // The table
    let supports_author = site.post_type_supports(&info.name, "author");
    let taxonomies = Content { site, db, writer: match ctx.env.writer { Some(w) => w, None => return super::die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>") }, user: &ctx.user, now: ctx.env.now }.object_taxonomies(&info.name);
    let mut columns: Vec<Column> = vec![Column::new("cb", "<input type=\"checkbox\" />"), Column::new("title", &site.trx("Title", "column name")).sortable("title", false, Some(&tr("Title")), if info.hierarchical { Some("asc") } else { None })];
    if supports_author {
        columns.push(Column::new("author", &tr("Author")));
    }
    for (taxonomy, _) in &taxonomies {
        if taxonomy == "category" {
            columns.push(Column::new("categories", &site.taxonomy_label("category")));
        } else if taxonomy == "post_tag" {
            columns.push(Column::new("tags", &site.taxonomy_label("post_tag")));
        }
    }
    if site.post_type_supports(&info.name, "comments") && !matches!(status_param.as_str(), "pending" | "draft" | "future") {
        columns.push(
            Column::new("comments", &format!("<span class=\"vers comment-grey-bubble\" title=\"{}\" aria-hidden=\"true\"></span><span class=\"screen-reader-text\">{}</span>", esc_attr(&tr("Comments")), tr("Comments")))
                .sortable("comment_count", false, Some(&tr("Comments")), None),
        );
    }
    columns.push(Column::new("date", &tr("Date")).sortable("date", true, Some(&tr("Date")), if info.hierarchical { None } else { Some("desc") }));

    let bulk_actions = {
        let mut list: Vec<(String, String)> = Vec::new();
        if is_trash {
            list.push(("untrash".into(), tr("Restore")));
        } else {
            list.push(("edit".into(), site.trx("Bulk edit", "verb")));
        }
        list.push(if is_trash { ("delete".into(), tr("Delete permanently")) } else { ("trash".into(), tr("Move to Trash")) });
        list
    };
    let table = Table {
        site,
        plural: if info.hierarchical { "pages".into() } else { "posts".into() },
        singular: String::new(),
        columns: columns.clone(),
        primary: "title".into(),
        current_url: current_url.clone(),
        orderby: orderby_param.clone(),
        order: if order_param == "desc" { "desc".into() } else { "asc".into() },
        total_items,
        per_page,
        current_page: paged,
        bulk_actions,
        has_items: !posts.is_empty(),
    };

    // Rows
    let mut rows = String::new();
    let all_categories = db.terms("category").await.unwrap_or_default();
    let _ = all_categories;
    if posts.is_empty() {
        let not_found = if is_trash {
            if info.name == "page" { tr("No pages found in Trash.") } else { tr("No posts found in Trash.") }
        } else if info.name == "page" {
            tr("No pages found.")
        } else {
            tr("No posts found.")
        };
        rows.push_str(&format!("<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"{}\">{not_found}</td></tr>", columns.len()));
    }
    for (post, level) in posts.iter().zip(levels.iter()) {
        let categories = db.terms_for_post(post.id, Some("category")).await.unwrap_or_default();
        let tags = db.terms_for_post(post.id, Some("post_tag")).await.unwrap_or_default();
        let is_sticky = sticky.contains(&post.id);
        let thumbnail = db.post_meta(post.id).await.unwrap_or_default().iter().any(|m| m.meta_key.as_deref() == Some("_thumbnail_id") && m.meta_value.as_deref().and_then(|v| v.parse::<u64>().ok()).is_some_and(|id| id > 0));
        let author_row = authors.iter().find(|u| u.id == post.post_author);
        let class = format!(
            "iedit author-{} level-{level} {}",
            if post.post_author == ctx.user.id { "self" } else { "other" },
            post_class(post, &categories, &tags, is_sticky, thumbnail)
        );
        rows.push_str(&format!("\t\t{}<tr id=\"post-{}\" class=\"{class}\">\n\t\t\t", if hierarchical_display { "\t" } else { "" }, post.id));
        let title = draft_or_title(ctx, post);
        let can_edit = ctx.can(edit_cap) && (post.post_author == ctx.user.id || ctx.can("edit_others_posts") || ctx.can("edit_others_pages"));
        for column in &columns {
            match column.key.as_str() {
                "cb" => {
                    if can_edit {
                        rows.push_str(&format!(
                            "<td class=\"check-column\">\t\t\t<input id=\"cb-select-{id}\" type=\"checkbox\" name=\"post[]\" value=\"{id}\" />\n\t\t\t<label for=\"cb-select-{id}\">\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</label>\n\t\t\t<div class=\"locked-indicator\">\n\t\t\t\t<span class=\"locked-indicator-icon\" aria-hidden=\"true\"></span>\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</div>\n\t\t\t</td>",
                            rustpress_core::i18n::sprintf(&tr("Select %s"), &[&title]),
                            rustpress_core::i18n::sprintf(&tr("&#8220;%s&#8221; is locked"), &[&title]),
                            id = post.id
                        ));
                    } else {
                        rows.push_str("<td class=\"check-column\"></td>");
                    }
                }
                "title" => {
                    let aria = if post.post_title.is_empty() { tr("no title") } else { post.post_title.clone() };
                    let pad = "<span aria-hidden=\"true\">&#8212;</span> ".repeat(*level);
                    let parent = parents.iter().find(|p| p.id == post.post_parent);
                    let (described, hierarchy_linked, hierarchy_nolink) = match parent {
                        Some(parent) => {
                            let hierarchy_id = format!("post-hierarchy-{}", post.id);
                            let text = esc_html(&rustpress_core::i18n::sprintf(&tr("Child of %s"), &[&rustpress_core::formatting::wp_strip_all_tags(&esc_html(&parent.post_title))]));
                            (format!(" aria-describedby=\"{hierarchy_id}\""), format!("<span id=\"{hierarchy_id}\" class=\"hidden\">{text}</span>"), format!("<span id=\"{hierarchy_id}\" class=\"screen-reader-text\"> ({text})</span>"))
                        }
                        None => (String::new(), String::new(), String::new()),
                    };
                    let edit_link = ctx.admin_url(&format!("post.php?post={}&action=edit", post.id)).replace('&', "&amp;");
                    let states = post_states(ctx, post, &status_param, is_sticky);
                    let name_html = if can_edit && post.post_status != "trash" {
                        format!("{pad}<a class=\"row-title\" href=\"{edit_link}\"{described}>{title}</a>{hierarchy_linked}")
                    } else {
                        format!("{pad}<span>{title}{hierarchy_nolink}</span>")
                    };
                    let inline = if can_edit { inline_data(ctx, post, &info, &taxonomies, is_sticky).await } else { String::new() };
                    rows.push_str(&format!(
                        "<th scope=\"row\" class=\"title column-title has-row-actions column-primary page-title\" data-colname=\"{}\" aria-label=\"{}\"><div class=\"locked-info\"><span class=\"locked-avatar\"></span> <span class=\"locked-text\"></span></div>\n<strong>{name_html}{states}</strong>\n{inline}{}<button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button></th>",
                        esc_attr(&column.label),
                        esc_attr(&aria),
                        row_actions_for(ctx, post, &title, can_edit, &info).await,
                        tr("Show more details")
                    ));
                }
                "author" => {
                    let cell = match author_row {
                        Some(user) if !user.display_name.is_empty() => {
                            let mut url = add_query_arg("edit.php", "post_type", &post.post_type);
                            url = add_query_arg(&url, "author", &user.id.to_string());
                            format!("<a href=\"{}\">{}</a>", esc_url(&url), esc_html(&user.display_name))
                        }
                        _ => format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{}</span>", tr("(no author)")),
                    };
                    rows.push_str(&format!("<td class='author column-author' data-colname=\"{}\">{cell}</td>", tr("Author")));
                }
                "categories" | "tags" => {
                    let (terms, label, none, query_var) = if column.key == "categories" { (&categories, site.taxonomy_label("category"), tr("No categories"), "category_name") } else { (&tags, site.taxonomy_label("post_tag"), tr("No tags"), "tag") };
                    let cell = if terms.is_empty() {
                        format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{none}</span>")
                    } else {
                        let links: Vec<String> = terms
                            .iter()
                            .map(|t| {
                                let mut url = "edit.php".to_owned();
                                if post.post_type != "post" {
                                    url = add_query_arg(&url, "post_type", &post.post_type);
                                }
                                url = add_query_arg(&url, query_var, &t.slug);
                                format!("<a href=\"{}\">{}</a>", esc_url(&url), esc_html(&t.name))
                            })
                            .collect();
                        links.join(&site.trx(", ", "tag delimiter"))
                    };
                    rows.push_str(&format!("<td class='{} column-{}' data-colname=\"{}\">{cell}</td>", column.key, column.key, esc_attr(&label)));
                }
                "comments" => {
                    let pending_count = pending.iter().find(|(id, _)| *id == post.id).map_or(0, |(_, n)| *n);
                    rows.push_str(&format!("<td class='comments column-comments' data-colname=\"{}\">\t\t<div class=\"post-com-count-wrapper\">\n\t\t{}\t\t</div>\n\t\t</td>", esc_attr(&strip_tags_label(&column.label, ctx)), comments_bubble(ctx, post, pending_count)));
                }
                "date" => {
                    rows.push_str(&format!("<td class='date column-date' data-colname=\"{}\">{}</td>", tr("Date"), date_cell(ctx, post)));
                }
                _ => {}
            }
        }
        rows.push_str("\t\t</tr>\n\t\t");
    }

    // Extra navigation: months, categories, filter button, "Empty Trash".
    let months = db.post_months(&info.name, is_trash).await.unwrap_or_default();
    let extra_top = {
        let mut filters = String::new();
        if !(months.is_empty() || (months.len() == 1 && months[0].1 == 0)) {
            filters.push_str(&format!("		<label for=\"filter-by-date\" class=\"screen-reader-text\">{}</label>
		<select name=\"m\" id=\"filter-by-date\">
", site.trx("Filter by date", "date filter label")));
            filters.push_str(&format!("			<option{} value=\"0\">{}</option>
		", if month == 0 { " selected='selected'" } else { "" }, tr("All dates")));
            for (year, month_number) in &months {
                if *year == 0 {
                    continue;
                }
                let value = format!("{year}{month_number:02}");
                let name = site.locale_data().month[(*month_number as usize).saturating_sub(1).min(11)].clone();
                let label = rustpress_core::i18n::sprintf(&tr("%1$s %2$d"), &[&name, &year.to_string()]);
                filters.push_str(&format!("<option {} value='{value}'>{}</option>
", if month.to_string() == value { "selected='selected'" } else { "" }, esc_html(&label)));
            }
            filters.push_str("		</select>
		");
        }
        if taxonomies.iter().any(|(n, _)| n == "category") {
            filters.push_str(&categories_dropdown(ctx, cat).await);
        }
        let mut out = String::from("		<div class=\"alignleft actions\">
		");
        if !filters.is_empty() {
            out.push_str(&filters);
            out.push_str(&submit_button(&tr("Filter"), "compact", "filter_action", "post-query-submit"));
            out.push_str("		");
        }
        if is_trash && !posts.is_empty() && ctx.can("edit_others_posts") {
            out.push_str(&submit_button(&tr("Empty Trash"), "apply", "delete_all", "delete_all"));
        }
        out.push_str("</div>
		");
        out
    };
    let extra_bottom = {
        let mut out = String::from("		<div class=\"alignleft actions\">
		");
        if is_trash && !posts.is_empty() && ctx.can("edit_others_posts") {
            out.push_str(&submit_button(&tr("Empty Trash"), "apply", "delete_all2", "delete_all2"));
        }
        out.push_str("		</div>
		");
        out
    };

    let nonce_field = ctx.nonce_field("bulk-posts");
    let caption = {
        let (ordered_by, direction) = match (orderby_param.as_str(), order_param.as_str()) {
            ("title", o) => (tr("Table ordered by Title."), o == "desc"),
            ("comment_count", o) => (tr("Table ordered by Comments."), o == "desc"),
            ("", _) if info.hierarchical => (tr("Table ordered by Hierarchical Menu Order and Title."), false),
            (_, o) => (tr("Table ordered by Date."), o != "asc"),
        };
        format!("{ordered_by} {}", if direction { tr("Descending.") } else { tr("Ascending.") })
    };
    let list_heading = if info.hierarchical { tr("Pages list") } else { tr("Posts list") };
    let table_html = table.display(&nonce_field, &extra_top, &extra_bottom, &caption, &rows, &list_heading);

    // The page
    let add_new = if info.name == "post" { ctx.admin_url("post-new.php") } else { ctx.admin_url(&format!("post-new.php?post_type={}", info.name)) };
    let subtitle = match &search {
        Some(term) => format!("<span class=\"subtitle\">{}</span>", rustpress_core::i18n::sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(term))])),
        None => String::new(),
    };
    let heading = format!("<h1 class=\"wp-heading-inline\">\n{}</h1>\n\n <a href=\"{}\" class=\"page-title-action\">{}</a>{subtitle}\n<hr class=\"wp-header-end\">\n\n", esc_html(&info.label), esc_url(&add_new), if info.hierarchical { tr("Add Page") } else { tr("Add Post") });
    let search_box = if search.is_some() || !posts.is_empty() {
        let mut hidden = String::new();
        if !orderby_param.is_empty() {
            hidden.push_str(&format!("<input type=\"hidden\" name=\"orderby\" value=\"{}\" />", esc_attr(&orderby_param)));
        }
        if !order_param.is_empty() {
            hidden.push_str(&format!("<input type=\"hidden\" name=\"order\" value=\"{}\" />", esc_attr(&order_param)));
        }
        let text = if info.hierarchical { tr("Search Pages") } else { tr("Search Posts") };
        format!(
            "{hidden}\n<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"post-search-input\">{text}:</label>\n\t<input type=\"search\" id=\"post-search-input\" name=\"s\" value=\"{}\" />\n\t\t{}</p>\n\t\t\n",
            esc_attr(&search.clone().unwrap_or_default()),
            submit_button(&text, "compact", "", "search-submit")
        )
    } else {
        String::new()
    };
    let filter_heading = if info.hierarchical { tr("Filter pages list") } else { tr("Filter posts list") };
    let hidden_fields = format!(
        "<input type=\"hidden\" name=\"post_status\" class=\"post_status_page\" value=\"{}\" />\n<input type=\"hidden\" name=\"post_type\" class=\"post_type_page\" value=\"{}\" />\n",
        esc_attr(&status_param),
        esc_attr(&info.name)
    );
    let notice = result_notice(ctx, &info);
    // `if ( $wp_list_table->has_items() )`: an empty list has no Quick Edit.
    let inline = if posts.is_empty() { String::new() } else { super::inline_edit::inline_edit(ctx, &info, posts.first(), columns.len(), taxonomies.iter().any(|(_, hierarchical)| *hierarchical)).await };
    let content = format!(
        "<div class=\"wrap\">\n{heading}{notice}\n{}\n<form id=\"posts-filter\" method=\"get\">\n\n{search_box}{hidden_fields}\n\n\n{table_html}\n</form>\n\n\n{inline}<div id=\"ajax-response\"></div>\n<div class=\"clear\"></div>\n</div>\n\n",
        views(&filter_heading, &links)
    );
    let screen = Screen {
        file: "edit.php".into(),
        parent_file: edit_base.clone(),
        submenu_file: Some(edit_base),
        post_type: info.name.clone(),
        title: info.label.clone(),
        help: None,
        view_item: None,
        ..Default::default()
    };
    let _ = remove_query_arg("", "");
    render(ctx, &screen, &content, &Assets { scripts: vec![], ..Default::default() }).await
}

fn strip_tags_label(label: &str, ctx: &Ctx<'_>) -> String {
    let _ = label;
    ctx.tr("Comments")
}

/// `esc_textarea()`.
fn esc_textarea(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#039;")
}

/// `get_inline_data()`: what the quick edit form reads from the row.
async fn inline_data(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, taxonomies: &[(String, bool)], sticky: bool) -> String {
    let site = ctx.site();
    let db = ctx.env.db;
    let meta = db.post_meta(post.id).await.unwrap_or_default();
    let template = meta.iter().find(|m| m.meta_key.as_deref() == Some("_wp_page_template")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
    let date = |range: std::ops::Range<usize>| post.post_date.get(range).unwrap_or_default().to_owned();
    let slug = String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(post.post_name.as_bytes())).into_owned();
    let mut out = format!(
        "\n<div class=\"hidden\" id=\"inline_{id}\">\n\t<div class=\"post_title\">{}</div>\n\t<div class=\"post_name\">{}</div>\n\t<div class=\"post_author\">{}</div>\n\t<div class=\"comment_status\">{}</div>\n\t<div class=\"ping_status\">{}</div>\n\t<div class=\"_status\">{}</div>\n\t<div class=\"jj\">{}</div>\n\t<div class=\"mm\">{}</div>\n\t<div class=\"aa\">{}</div>\n\t<div class=\"hh\">{}</div>\n\t<div class=\"mn\">{}</div>\n\t<div class=\"ss\">{}</div>\n\t<div class=\"post_password\">{}</div>",
        esc_textarea(post.post_title.trim()),
        esc_textarea(&slug),
        post.post_author,
        esc_html(&post.comment_status),
        esc_html(&post.ping_status),
        esc_html(&post.post_status),
        date(8..10),
        date(5..7),
        date(0..4),
        date(11..13),
        date(14..16),
        date(17..19),
        esc_html(&post.post_password),
        id = post.id
    );
    if info.hierarchical {
        out.push_str(&format!("<div class=\"post_parent\">{}</div>", post.post_parent));
    }
    out.push_str(&format!("<div class=\"page_template\">{}</div>", if template.is_empty() { "default".to_owned() } else { esc_html(&template) }));
    if site.post_type_supports(&post.post_type, "page-attributes") {
        out.push_str(&format!("<div class=\"menu_order\">{}</div>", post.menu_order));
    }
    for (name, hierarchical) in taxonomies {
        if name == "post_format" {
            continue;
        }
        let terms = db.terms_for_post(post.id, Some(name)).await.unwrap_or_default();
        if *hierarchical {
            let ids: Vec<String> = terms.iter().map(|t| t.term_id.to_string()).collect();
            out.push_str(&format!("<div class=\"post_category\" id=\"{name}_{}\">{}</div>", post.id, ids.join(",")));
        } else {
            let names: Vec<String> = terms.iter().map(|t| t.name.clone()).collect();
            out.push_str(&format!("<div class=\"tags_input\" id=\"{name}_{}\">{}</div>", post.id, esc_html(&esc_attr(&names.join(",")).replace(',', ", "))));
        }
    }
    if !info.hierarchical {
        out.push_str(&format!("<div class=\"sticky\">{}</div>", if sticky { "sticky" } else { "" }));
    }
    if site.post_type_supports(&post.post_type, "post-formats") {
        let formats = db.terms_for_post(post.id, Some("post_format")).await.unwrap_or_default();
        let format = formats.first().and_then(|t| t.slug.strip_prefix("post-format-")).unwrap_or_default().to_owned();
        out.push_str(&format!("<div class=\"post_format\">{}</div>", esc_html(&format)));
    }
    out.push_str("</div>");
    out
}

/// `get_post_states()` as `_post_states()` prints it.
fn post_states(ctx: &Ctx<'_>, post: &Post, status_param: &str, sticky: bool) -> String {
    let site = ctx.site();
    let mut states: Vec<String> = Vec::new();
    if !post.post_password.is_empty() {
        states.push(site.trx("Password protected", "post status"));
    }
    if post.post_status == "private" && status_param != "private" {
        states.push(site.trx("Private", "post status"));
    }
    if post.post_status == "draft" && status_param != "draft" {
        states.push(site.trx("Draft", "post status"));
    }
    if post.post_status == "pending" && status_param != "pending" {
        states.push(site.trx("Pending", "post status"));
    }
    if sticky {
        states.push(site.trx("Sticky", "post status"));
    }
    if post.post_status == "future" {
        states.push(site.trx("Scheduled", "post status"));
    }
    if site.options.get("show_on_front") == Some("page") {
        if site.options.u64("page_on_front", 0) == post.id {
            states.push(site.trx("Front Page", "page label"));
        }
        if site.options.u64("page_for_posts", 0) == post.id {
            states.push(site.trx("Posts Page", "page label"));
        }
    }
    if site.options.u64("wp_page_for_privacy_policy", 0) == post.id {
        states.push(site.trx("Privacy Policy Page", "page label"));
    }
    if states.is_empty() {
        return String::new();
    }
    let separator = site.trx(", ", "list item separator");
    let count = states.len();
    let mut out = String::from(" &mdash; ");
    for (index, state) in states.iter().enumerate() {
        let suffix = if index + 1 < count { separator.as_str() } else { "" };
        out.push_str(&format!("<span class='post-state'>{state}{suffix}</span>"));
    }
    out
}

/// `handle_row_actions()`.
async fn row_actions_for(ctx: &Ctx<'_>, post: &Post, title: &str, can_edit: bool, info: &TypeInfo) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let mut actions: Vec<(String, String)> = Vec::new();
    let edit_url = ctx.admin_url(&format!("post.php?post={}&action=edit", post.id)).replace('&', "&amp;");
    if can_edit && post.post_status != "trash" {
        actions.push(("edit".into(), format!("<a href=\"{edit_url}\" aria-label=\"{}\">{}</a>", esc_attr(&rustpress_core::i18n::sprintf(&tr("Edit &#8220;%s&#8221;"), &[title])), tr("Edit"))));
        actions.push(("inline hide-if-no-js".into(), format!("<button type=\"button\" class=\"button-link editinline\" aria-label=\"{}\" aria-expanded=\"false\">{}</button>", esc_attr(&rustpress_core::i18n::sprintf(&tr("Quick edit &#8220;%s&#8221; inline"), &[title])), tr("Quick&nbsp;Edit"))));
    }
    let can_delete = ctx.can(if info.name == "page" { "delete_pages" } else { "delete_posts" }) && (post.post_author == ctx.user.id || ctx.can("delete_others_posts") || ctx.can("delete_others_pages"));
    if can_delete {
        let nonce_url = |action: &str| {
            let url = ctx.admin_url(&format!("post.php?post={}&action={action}", post.id));
            format!("{}&amp;_wpnonce={}", url.replace('&', "&amp;"), ctx.create_nonce(&format!("{action}-post_{}", post.id)))
        };
        if post.post_status == "trash" {
            actions.push(("untrash".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", nonce_url("untrash"), esc_attr(&rustpress_core::i18n::sprintf(&tr("Restore &#8220;%s&#8221; from the Trash"), &[title])), tr("Restore"))));
            actions.push(("delete".into(), format!("<a href=\"{}\" class=\"submitdelete\" aria-label=\"{}\">{}</a>", nonce_url("delete"), esc_attr(&rustpress_core::i18n::sprintf(&tr("Delete &#8220;%s&#8221; permanently"), &[title])), tr("Delete Permanently"))));
        } else {
            actions.push(("trash".into(), format!("<a href=\"{}\" class=\"submitdelete\" aria-label=\"{}\">{}</a>", nonce_url("trash"), esc_attr(&rustpress_core::i18n::sprintf(&tr("Move &#8220;%s&#8221; to the Trash"), &[title])), site.trx("Trash", "verb"))));
        }
    }
    if site.is_post_type_viewable(&info.name) {
        if matches!(post.post_status.as_str(), "pending" | "draft" | "future") {
            if can_edit {
                let preview = if info.name == "page" { site.home_url(&format!("?page_id={}&preview=true", post.id)) } else { site.home_url(&format!("?p={}&preview=true", post.id)) };
                actions.push(("view".into(), format!("<a href=\"{}\" rel=\"bookmark\" aria-label=\"{}\">{}</a>", esc_url(&preview), esc_attr(&rustpress_core::i18n::sprintf(&tr("Preview &#8220;%s&#8221;"), &[title])), site.trx("Preview", "verb"))));
            }
        } else if post.post_status != "trash" {
            let link = site.permalink(post).await.unwrap_or_default();
            actions.push(("view".into(), format!("<a href=\"{}\" rel=\"bookmark\" aria-label=\"{}\">{}</a>", esc_attr(&link), esc_attr(&rustpress_core::i18n::sprintf(&tr("View &#8220;%s&#8221;"), &[title])), tr("View"))));
        }
    }
    let mut html = row_actions(site, &actions);
    // `row_actions()` ends with the toggle button, which the title cell prints itself.
    if let Some(at) = html.find("<button type=\"button\" class=\"toggle-row\">") {
        html.truncate(at);
    }
    html
}

/// `column_date()`.
fn date_cell(ctx: &Ctx<'_>, post: &Post) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let (t_time, time_diff) = if post.post_date == "0000-00-00 00:00:00" {
        (tr("Unpublished"), 0)
    } else {
        let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
        let text = rustpress_core::i18n::sprintf(&tr("%1$s at %2$s"), &[&site.date(&tr("Y/m/d"), ts), &site.date(&tr("g:i a"), ts)]);
        (text, ctx.env.now - ts)
    };
    let status = match post.post_status.as_str() {
        "publish" => tr("Published"),
        "future" => {
            if time_diff > 0 {
                format!("<strong class=\"error-message\">{}</strong>", tr("Missed schedule"))
            } else {
                tr("Scheduled")
            }
        }
        _ => tr("Last Modified"),
    };
    format!("{status}<br />{t_time}")
}

/// `comments_bubble()`.
fn comments_bubble(ctx: &Ctx<'_>, post: &Post, pending: u64) -> String {
    let site = ctx.site();
    let approved = post.comment_count.max(0) as u64;
    let approved_number = site.number_i18n(approved);
    let pending_number = site.number_i18n(pending);
    let approved_only = rustpress_core::i18n::sprintf(&site.trn("%s comment", "%s comments", approved as i64), &[&approved_number]);
    let approved_phrase = rustpress_core::i18n::sprintf(&site.trn("%s approved comment", "%s approved comments", approved as i64), &[&approved_number]);
    let pending_phrase = rustpress_core::i18n::sprintf(&site.trn("%s pending comment", "%s pending comments", pending as i64), &[&pending_number]);
    let mut out = String::new();
    if approved == 0 && pending == 0 {
        out.push_str(&format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{}</span>", ctx.tr("No comments")));
    } else if approved > 0 && post.post_status == "trash" {
        out.push_str(&format!(
            "<span class=\"post-com-count post-com-count-approved\"><span class=\"comment-count-approved\" aria-hidden=\"true\">{approved_number}</span><span class=\"screen-reader-text\">{}</span></span>",
            if pending > 0 { &approved_phrase } else { &approved_only }
        ));
    } else if approved > 0 {
        let url = add_query_arg(&add_query_arg(&ctx.admin_url("edit-comments.php"), "p", &post.id.to_string()), "comment_status", "approved");
        out.push_str(&format!(
            "<a href=\"{}\" class=\"post-com-count post-com-count-approved\"><span class=\"comment-count-approved\" aria-hidden=\"true\">{approved_number}</span><span class=\"screen-reader-text\">{}</span></a>",
            esc_url(&url),
            if pending > 0 { &approved_phrase } else { &approved_only }
        ));
    } else {
        out.push_str(&format!(
            "<span class=\"post-com-count post-com-count-no-comments\"><span class=\"comment-count comment-count-no-comments\" aria-hidden=\"true\">{approved_number}</span><span class=\"screen-reader-text\">{}</span></span>",
            if pending > 0 { ctx.tr("No approved comments") } else { ctx.tr("No comments") }
        ));
    }
    if pending > 0 {
        let url = add_query_arg(&add_query_arg(&ctx.admin_url("edit-comments.php"), "p", &post.id.to_string()), "comment_status", "moderated");
        out.push_str(&format!(
            "<a href=\"{}\" class=\"post-com-count post-com-count-pending\"><span class=\"comment-count-pending\" aria-hidden=\"true\">{pending_number}</span><span class=\"screen-reader-text\">{pending_phrase}</span></a>",
            esc_url(&url)
        ));
    } else {
        out.push_str(&format!(
            "<span class=\"post-com-count post-com-count-pending post-com-count-no-pending\"><span class=\"comment-count comment-count-no-pending\" aria-hidden=\"true\">{pending_number}</span><span class=\"screen-reader-text\">{}</span></span>",
            if approved > 0 { ctx.tr("No pending comments") } else { ctx.tr("No comments") }
        ));
    }
    out
}
