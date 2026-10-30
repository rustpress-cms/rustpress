//! `wp-admin/edit.php`: the list of posts, pages and other post types.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use std::collections::{HashMap, HashSet};

use rustpress_db::{OrderBy, Order, Post, PostQuery};

use super::notices;
use super::{AdminResponse, Ctx, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
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
    pub labels: ListLabels,
}

/// The labels of a post type the list prints (`get_post_type_labels()`).
pub struct ListLabels {
    pub add_new: String,
    pub search_items: String,
    pub not_found: String,
    pub not_found_in_trash: String,
    pub filter_items_list: String,
    pub items_list_navigation: String,
    pub items_list: String,
}

impl ListLabels {
    /// The defaults: those of posts, or of pages for a hierarchical type.
    pub(super) fn defaults(ctx: &Ctx<'_>, hierarchical: bool) -> Self {
        let tr = |s: &str| ctx.tr(s);
        if hierarchical {
            ListLabels {
                add_new: tr("Add Page"),
                search_items: tr("Search Pages"),
                not_found: tr("No pages found."),
                not_found_in_trash: tr("No pages found in Trash."),
                filter_items_list: tr("Filter pages list"),
                items_list_navigation: tr("Pages list navigation"),
                items_list: tr("Pages list"),
            }
        } else {
            ListLabels {
                add_new: tr("Add Post"),
                search_items: tr("Search Posts"),
                not_found: tr("No posts found."),
                not_found_in_trash: tr("No posts found in Trash."),
                filter_items_list: tr("Filter posts list"),
                items_list_navigation: tr("Posts list navigation"),
                items_list: tr("Posts list"),
            }
        }
    }
}

pub fn type_info(ctx: &Ctx<'_>, name: &str) -> Option<TypeInfo> {
    let site = ctx.site();
    match name {
        "post" => Some(TypeInfo { name: "post".into(), hierarchical: false, label: site.post_type_label("post"), singular: site.trx("Post", "post type singular name"), labels: ListLabels::defaults(ctx, false) }),
        "page" => Some(TypeInfo { name: "page".into(), hierarchical: true, label: site.post_type_label("page"), singular: site.trx("Page", "post type singular name"), labels: ListLabels::defaults(ctx, true) }),
        // The patterns (`register_post_type( 'wp_block' )` in `create_initial_post_types()`).
        "wp_block" => {
            let tr = |s: &str| ctx.tr(s);
            Some(TypeInfo {
                name: "wp_block".into(),
                hierarchical: false,
                label: site.trx("Patterns", "post type general name"),
                singular: site.trx("Pattern", "post type singular name"),
                labels: ListLabels {
                    add_new: tr("Add Pattern"),
                    search_items: tr("Search Patterns"),
                    not_found: tr("No patterns found."),
                    not_found_in_trash: tr("No patterns found in Trash."),
                    filter_items_list: tr("Filter patterns list"),
                    items_list_navigation: tr("Patterns list navigation"),
                    items_list: tr("Patterns list"),
                },
            })
        }
        other => site.types.post_type(other).filter(|t| t.public || t.queryable).map(|t| TypeInfo { name: other.to_owned(), hierarchical: t.hierarchical, label: t.plural_label(), singular: t.singular_label(), labels: ListLabels::defaults(ctx, t.hierarchical) }),
    }
}

/// `post_class()` for a list row.
fn post_class(post: &Post, categories: &[rustpress_db::Term], tags: &[rustpress_db::Term], others: &[(String, rustpress_db::Term)], sticky: bool, thumbnail: bool) -> String {
    let mut classes = vec![format!("post-{}", post.id), format!("type-{}", post.post_type), format!("status-{}", post.post_status)];
    if post.post_type == "post" {
        let format = others.iter().find(|(taxonomy, _)| taxonomy == "post_format").and_then(|(_, term)| term.slug.strip_prefix("post-format-"));
        classes.push(format!("format-{}", rustpress_core::formatting::sanitize_html_class(format.unwrap_or("standard"))));
    }
    // `post_password_required()`: in the admin there is no password cookie.
    let protected = !post.post_password.is_empty();
    if protected {
        classes.push("post-password-required".to_owned());
    }
    if thumbnail && !protected {
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
    // The terms of every other public taxonomy: `genre-krimi`.
    // Only public taxonomies give classes; the internal ones (`wp_pattern_category`, …) are not.
    for (taxonomy, term) in others.iter().filter(|(taxonomy, _)| !taxonomy.starts_with("wp_")) {
        classes.push(rustpress_core::formatting::sanitize_html_class(&format!("{taxonomy}-{}", rustpress_core::formatting::sanitize_html_class(&term.slug))));
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
    let mut out = format!("<label class=\"screen-reader-text\" for=\"cat\">{}</label>", ctx.tr("Filter by category"));
    out.push_str("<select  name='cat' id='cat' class='postform'>\n");
    out.push_str(&format!("\t<option value='0'>{}</option>\n", ctx.tr("All Categories")));
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
    let post = if info.name == "page" { ("%s page updated.", "%s pages updated.") } else { ("%s post updated.", "%s posts updated.") };
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
    if let Some(n) = count("locked").filter(|n| *n > 0) {
        let singular = if info.name == "page" { "%s page not updated, somebody is editing it." } else { "%s post not updated, somebody is editing it." };
        let plural = if info.name == "page" { "%s pages not updated, somebody is editing them." } else { "%s posts not updated, somebody is editing them." };
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
        return Some(super::die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired.")));
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
    // Bulk Edit: the form sends `action=edit` with `bulk_edit`.
    if action == "edit" && request.request("bulk_edit").is_some() {
        let (updated, skipped, locked) = super::quick_edit::bulk_edit(ctx, &ids).await;
        sendback = add_query_arg(&sendback, "updated", &updated.to_string());
        sendback = add_query_arg(&sendback, "skipped", &skipped.to_string());
        sendback = add_query_arg(&sendback, "locked", &locked.to_string());
        return Some(redirect(&ctx.admin_url(&sendback)));
    }
    // `delete_all` (Empty Trash): every post of the type with the status of the view is deleted.
    let action = if action == "delete_all" {
        let status: String = request.request("post_status").unwrap_or_default().chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect();
        ids = if status.is_empty() {
            Vec::new()
        } else {
            let query = PostQuery { post_types: vec![info.name.clone()], statuses: vec![status], ..Default::default() };
            ctx.env.db.post_parents(&query).await.unwrap_or_default().into_iter().map(|(id, _)| id).collect()
        };
        "delete".to_owned()
    } else {
        action
    };
    if ids.is_empty() {
        return Some(redirect(&ctx.admin_url(&sendback)));
    }
    // Every post is checked on its own (`current_user_can( 'delete_post', $id )`); one that may not be touched
    // ends the request, as `wp_die()` does in WordPress.
    let may_delete = |post: &Option<Post>| post.as_ref().is_some_and(|p| ctx.user.can_delete_post(p));
    let mut done = 0i64;
    let mut locked = 0i64;
    let key;
    match action.as_str() {
        "trash" => {
            for id in &ids {
                let post = ctx.env.db.post(*id).await.ok().flatten();
                if !may_delete(&post) {
                    return Some(super::common::die_plain(ctx, "Sorry, you are not allowed to move this item to the Trash."));
                }
                if super::ajax::post_lock_holder(ctx.env, &ctx.user, *id).await.is_some() {
                    locked += 1;
                    continue;
                }
                if !content.trash_post(*id).await.unwrap_or(false) {
                    return Some(super::common::die_plain(ctx, "Error in moving the item to Trash."));
                }
                done += 1;
            }
            sendback = add_query_arg(&sendback, "trashed", &done.to_string());
            sendback = add_query_arg(&sendback, "ids", &ids.iter().map(u64::to_string).collect::<Vec<_>>().join(","));
            sendback = add_query_arg(&sendback, "locked", &locked.to_string());
            return Some(redirect(&ctx.admin_url(&sendback)));
        }
        "untrash" => {
            key = "untrashed";
            for id in &ids {
                let post = ctx.env.db.post(*id).await.ok().flatten();
                if !may_delete(&post) {
                    return Some(super::common::die_plain(ctx, "Sorry, you are not allowed to restore this item from the Trash."));
                }
                if !content.untrash_post(*id).await.unwrap_or(false) {
                    return Some(super::common::die_plain(ctx, "Error in restoring the item from Trash."));
                }
                done += 1;
            }
        }
        "delete" => {
            key = "deleted";
            for id in &ids {
                let post = ctx.env.db.post(*id).await.ok().flatten();
                if !may_delete(&post) {
                    return Some(super::common::die_plain(ctx, "Sorry, you are not allowed to delete this item."));
                }
                let Some(post) = post else { continue };
                if post.post_type == "attachment" {
                    if !content.delete_attachment(*id, super::ajax_media::uploads_dir(ctx.env)).await.unwrap_or(false) {
                        return Some(super::common::die_plain(ctx, "Error in deleting the attachment."));
                    }
                } else if !content.delete_post(*id, false).await.unwrap_or(false) {
                    // `wp_delete_post()` without `force`: a post that is not in the trash goes there first.
                    return Some(super::common::die_plain(ctx, "Error in deleting the item."));
                }
                done += 1;
            }
        }
        _ => return None,
    }
    sendback = add_query_arg(&sendback, key, &done.to_string());
    Some(redirect(&ctx.admin_url(&sendback)))
}

/// Flattens pages into the order of a hierarchical list: parents first, children below them.
/// `_page_rows()`: `(ID, level)` of every page, each followed by its descendants, from `(ID, post_parent)` in list order.
fn hierarchical_order(rows: &[(u64, u64)]) -> Vec<(u64, usize)> {
    let ids: HashSet<u64> = rows.iter().map(|(id, _)| *id).collect();
    let mut children: HashMap<u64, Vec<u64>> = HashMap::new();
    for (id, parent) in rows {
        children.entry(*parent).or_default().push(*id);
    }
    let mut out = Vec::with_capacity(rows.len());
    fn walk(children: &HashMap<u64, Vec<u64>>, parent: u64, level: usize, out: &mut Vec<(u64, usize)>) {
        for id in children.get(&parent).into_iter().flatten() {
            out.push((*id, level));
            walk(children, *id, level + 1, out);
        }
    }
    // Roots are the pages whose parent is not in the list.
    for (id, _) in rows.iter().filter(|(_, parent)| *parent == 0 || !ids.contains(parent)) {
        out.push((*id, 0));
        walk(&children, *id, 1, &mut out);
    }
    out
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let site = ctx.site();
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let post_type = request.get("post_type").filter(|t| !t.is_empty()).unwrap_or("post").to_owned();
    let Some(info) = type_info(ctx, &post_type) else {
        return super::common::die_plain(ctx, "Invalid post type.");
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
    let origin = super::request_origin(ctx.env, request);
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
    let per_page_option = format!("edit_{}_per_page", info.name);
    let per_page = super::common::items_per_page(ctx, &per_page_option, 20).await;

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
    // Drafts and pending items are ordered by their modification (`wp_edit_posts_query()`), not by hierarchy.
    let hierarchical_display = info.hierarchical && orderby_param.is_empty() && !matches!(post_status.as_deref(), Some("pending" | "draft"));
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
        // `WP_Query` without an `orderby` orders a search by relevance.
        _ if search.is_some() => OrderBy::Relevance,
        _ => OrderBy::Date,
    };
    let order = if order_param == "asc" || (hierarchical_display) || (order_param.is_empty() && post_status.as_deref() == Some("pending")) { Order::Asc } else { Order::Desc };
    let taxonomies = Content { site, db, writer: match ctx.env.writer { Some(w) => w, None => return super::die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>") }, user: &ctx.user, now: ctx.env.now }.object_taxonomies(&info.name);
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
    // The query variables of the other taxonomies (`genre=krimi`), and the post format (`post_format=video`) of the
    // types with formats.
    let mut filter_taxonomies = taxonomies.clone();
    if site.post_type_supports(&info.name, "post-formats") && !filter_taxonomies.iter().any(|(t, _)| t == "post_format") {
        filter_taxonomies.push(("post_format".to_owned(), false));
    }
    for (taxonomy, _) in &filter_taxonomies {
        let (query_var, hierarchical) = match taxonomy.as_str() {
            "category" | "post_tag" => continue,
            "post_format" => (Some("post_format".to_owned()), false),
            other => (site.types.taxonomy(other).and_then(|t| t.query_var.clone()), site.types.taxonomy(other).is_some_and(|t| t.hierarchical)),
        };
        let Some(slugs) = query_var.and_then(|var| request.get(&var).filter(|v| !v.is_empty()).map(str::to_owned)) else { continue };
        let all_terms = db.terms(taxonomy).await.unwrap_or_default();
        let mut ids: Vec<u64> = Vec::new();
        for slug in slugs.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            // `_post_format_request()`: `video` is the term `post-format-video`.
            let slug = if taxonomy == "post_format" && !slug.starts_with("post-format-") { format!("post-format-{slug}") } else { slug.to_owned() };
            if let Some(term) = all_terms.iter().find(|t| t.slug == slug) {
                ids.push(term.term_id);
                if hierarchical {
                    loop {
                        let before = ids.len();
                        for child in &all_terms {
                            if ids.contains(&child.parent) && !ids.contains(&child.term_id) {
                                ids.push(child.term_id);
                            }
                        }
                        if ids.len() == before {
                            break;
                        }
                    }
                }
            }
        }
        let tt: Vec<u64> = all_terms.iter().filter(|t| ids.contains(&t.term_id)).map(|t| t.term_taxonomy_id).collect();
        // A term that does not exist matches nothing.
        query.tax_groups.push(if tt.is_empty() { vec![0] } else { tt });
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
    let mode = super::common::request_list_mode(ctx).await;
    let total_items = db.count_posts(&query).await.unwrap_or(0);
    if !hierarchical_display
        && let Some(response) = super::common::past_last_page(ctx, paged, total_items, per_page)
    {
        return response;
    }
    let (posts, levels): (Vec<Post>, Vec<usize>) = if hierarchical_display {
        // Like `wp_edit_posts_query()`: every match as `id=>parent`, rows only for the page shown.
        let all = db.post_parents(&query).await.unwrap_or_default();
        let start = ((paged - 1).saturating_mul(per_page)) as usize;
        let slice: Vec<(u64, usize)> = hierarchical_order(&all).into_iter().skip(start).take(per_page as usize).collect();
        let ids: Vec<u64> = slice.iter().map(|(id, _)| *id).collect();
        let mut rows: HashMap<u64, Post> = db.posts_by_ids(&ids).await.unwrap_or_default().into_iter().map(|p| (p.id, p)).collect();
        slice.into_iter().filter_map(|(id, level)| rows.remove(&id).map(|post| (post, level))).unzip()
    } else {
        query.limit = Some(per_page);
        query.offset = (paged - 1).saturating_mul(per_page);
        let posts = db.posts(&query).await.unwrap_or_default();
        let levels = vec![0; posts.len()];
        (posts, levels)
    };

    // Related data of the rows; terms, meta and featured images of all rows in a few queries.
    let _ = db.prime_loop(&posts).await;
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
    let mut columns = list_columns(ctx, &info, &status_param, &taxonomies);
    let screen_options = super::common::list_screen_options(ctx, &format!("edit-{}", info.name), &mut columns, (&per_page_option, per_page), Some(mode.clone())).await;

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
        view_mode: mode.clone(),
    };

    // Rows
    let mut rows = String::new();
    if posts.is_empty() {
        let not_found = if is_trash { info.labels.not_found_in_trash.clone() } else { info.labels.not_found.clone() };
        rows.push_str(&format!("<tr class=\"no-items\"><td class=\"colspanchange\" colspan=\"{}\">{not_found}</td></tr>", columns.iter().filter(|c| !c.hidden).count()));
    }
    for (post, level) in posts.iter().zip(levels.iter()) {
        let row_env = RowEnv { info: &info, columns: &columns, status_param: &status_param, hierarchical_display, sticky: &sticky, taxonomies: &taxonomies, pending: &pending, authors: &authors, parents: &parents , mode: &mode };
        rows.push_str(&post_row(ctx, post, *level, &row_env).await);
    }

    // Extra navigation: months, categories, filter button, "Empty Trash".
    let months = db.post_months(&info.name, is_trash).await.unwrap_or_default();
    let extra_top = {
        let mut filters = String::new();
        if !(months.is_empty() || (months.len() == 1 && months[0].1 == 0)) {
            filters.push_str(&format!("		<label for=\"filter-by-date\" class=\"screen-reader-text\">{}</label>
		<select name=\"m\" id=\"filter-by-date\">
", site.tr("Filter by date")));
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
", if month.to_string() == value { " selected='selected'" } else { "" }, esc_html(&label)));
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
        }
        if is_trash && !posts.is_empty() && ctx.can("edit_others_posts") {
            out.push_str(&submit_button(&tr("Empty Trash"), "apply", "delete_all", "delete_all"));
        }
        out.push_str("		</div>
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
            ("", _) if info.name == "page" => (tr("Table ordered by Hierarchical Menu Order and Title."), false),
            (_, o) => (tr("Table ordered by Date."), o != "asc"),
        };
        format!("{ordered_by} {}", if direction { tr("Descending.") } else { tr("Ascending.") })
    };
    let list_heading = info.labels.items_list.clone();
    let table_html = table.display(&nonce_field, &extra_top, &extra_bottom, &caption, &rows, &list_heading);
    // `heading_pagination` is the post type's `items_list_navigation`.
    let table_html = table_html.replace(&format!("<h2 class='screen-reader-text'>{}</h2>", table.heading_pagination()), &format!("<h2 class='screen-reader-text'>{}</h2>", info.labels.items_list_navigation));

    // The page
    let add_new = if info.name == "post" { ctx.admin_url("post-new.php") } else { ctx.admin_url(&format!("post-new.php?post_type={}", info.name)) };
    let subtitle = match &search {
        Some(term) => format!("<span class=\"subtitle\">{}</span>", rustpress_core::i18n::sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(term))])),
        None => String::new(),
    };
    let heading = format!("<h1 class=\"wp-heading-inline\">\n{}</h1>\n\n <a href=\"{}\" class=\"page-title-action\">{}</a>{subtitle}\n<hr class=\"wp-header-end\">\n\n", esc_html(&info.label), esc_url(&add_new), info.labels.add_new);
    let search_box = if search.is_some() || !posts.is_empty() {
        let mut hidden = String::new();
        if !orderby_param.is_empty() {
            hidden.push_str(&format!("<input type=\"hidden\" name=\"orderby\" value=\"{}\" />", esc_attr(&orderby_param)));
        }
        if !order_param.is_empty() {
            hidden.push_str(&format!("<input type=\"hidden\" name=\"order\" value=\"{}\" />", esc_attr(&order_param)));
        }
        let text = info.labels.search_items.clone();
        format!(
            "{hidden}<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"post-search-input\">{text}:</label>\n\t<input type=\"search\" id=\"post-search-input\" name=\"s\" value=\"{}\" />\n\t\t{}</p>\n\t\t",
            esc_attr(&search.clone().unwrap_or_default()),
            submit_button(&text, "compact", "", "search-submit")
        )
    } else {
        String::new()
    };
    let filter_heading = info.labels.filter_items_list.clone();
    let mut hidden_fields = format!(
        "<input type=\"hidden\" name=\"post_status\" class=\"post_status_page\" value=\"{}\" />\n<input type=\"hidden\" name=\"post_type\" class=\"post_type_page\" value=\"{}\" />\n\n",
        esc_attr(&status_param),
        esc_attr(&info.name)
    );
    if let Some(a) = request.request("author").filter(|a| !a.is_empty() && *a != "0") {
        hidden_fields.push_str(&format!("<input type=\"hidden\" name=\"author\" value=\"{}\" />\n", esc_attr(a)));
    }
    hidden_fields.push('\n');
    if request.request("show_sticky").is_some_and(|v| !v.is_empty() && v != "0") {
        hidden_fields.push_str("<input type=\"hidden\" name=\"show_sticky\" value=\"1\" />\n");
    }
    hidden_fields.push('\n');
    let notice = result_notice(ctx, &info);
    // `if ( $wp_list_table->has_items() )`: an empty list has no Quick Edit.
    let inline = if posts.is_empty() { String::new() } else { super::inline_edit::inline_edit(ctx, &info, posts.first(), columns.iter().filter(|c| !c.hidden).count(), &taxonomies, &mode).await };
    let content = format!(
        "<div class=\"wrap\">\n{heading}{notice}\n{}\n<form id=\"posts-filter\" method=\"get\">\n\n{search_box}\n{hidden_fields}{table_html}\n</form>\n\n{inline}\n<div id=\"ajax-response\"></div>\n<div class=\"clear\"></div>\n</div>\n\n",
        views(&filter_heading, &links)
    );
    // `wp_admin_bar_edit_menu()`: a post type with an archive gets "View Posts" on its list.
    let archive_item = if ctx.can("read") { site.post_type_archive_link(&info.name).map(|url| (if info.hierarchical { tr("View Pages") } else { tr("View Posts") }, url)) } else { None };
    let screen = Screen {
        file: "edit.php".into(),
        parent_file: edit_base.clone(),
        submenu_file: Some(edit_base),
        post_type: info.name.clone(),
        title: info.label.clone(),
        help: list_help(ctx, &info),
        options: Some(screen_options),
        content_inline: true,
        view_item: archive_item.clone(),
        view_archive: archive_item.is_some(),
        ..Default::default()
    };
    let _ = remove_query_arg("", "");
    render(ctx, &screen, &content, &Assets { scripts: vec!["inline-edit-post".into(), "heartbeat".into()], ..Default::default() }).await
}

/// The columns of the list: `WP_Posts_List_Table::get_columns()`.
pub(super) fn list_columns(ctx: &Ctx<'_>, info: &TypeInfo, status_param: &str, taxonomies: &[(String, bool)]) -> Vec<Column> {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let supports_author = site.post_type_supports(&info.name, "author");
    let mut columns: Vec<Column> = vec![Column::new("cb", "<input type=\"checkbox\" />"), Column::new("title", &site.trx("Title", "column name")).sortable("title", false, Some(&tr("Title")), if info.name == "page" { Some("asc") } else { None })];
    if supports_author {
        columns.push(Column::new("author", &tr("Author")));
    }
    for (taxonomy, _) in taxonomies {
        if taxonomy == "category" {
            columns.push(Column::new("categories", &site.taxonomy_label("category")));
        } else if taxonomy == "post_tag" {
            columns.push(Column::new("tags", &site.taxonomy_label("post_tag")));
        }
    }
    // Taxonomies with `show_admin_column` besides categories and tags.
    if info.name == "wp_block" {
        columns.push(Column::new("taxonomy-wp_pattern_category", &tr("Pattern Categories")));
    }
    if site.post_type_supports(&info.name, "comments") && !matches!(status_param, "pending" | "draft" | "future") {
        columns.push(
            Column::new("comments", &format!("<span class=\"vers comment-grey-bubble\" title=\"{}\" aria-hidden=\"true\"></span><span class=\"screen-reader-text\">{}</span>", esc_attr(&tr("Comments")), tr("Comments")))
                .sortable("comment_count", false, Some(&tr("Comments")), None),
        );
    }
    columns.push(Column::new("date", &tr("Date")).sortable("date", true, Some(&tr("Date")), if info.name == "page" { None } else { Some("desc") }));
    columns
}

/// What a row of the posts list needs besides the post.
pub(super) struct RowEnv<'a> {
    pub info: &'a TypeInfo,
    pub columns: &'a [Column],
    pub status_param: &'a str,
    pub hierarchical_display: bool,
    pub sticky: &'a [u64],
    pub taxonomies: &'a [(String, bool)],
    pub pending: &'a [(u64, u64)],
    pub authors: &'a [rustpress_db::User],
    pub parents: &'a [Post],
    /// `list` or `excerpt` (`$mode`).
    pub mode: &'a str,
}

/// `WP_Posts_List_Table::single_row()`.
pub(super) async fn post_row(ctx: &Ctx<'_>, post: &Post, level: usize, row: &RowEnv<'_>) -> String {
    let RowEnv { info, columns, status_param, hierarchical_display, sticky, taxonomies, pending, authors, parents, mode } = *row;
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let db = ctx.env.db;
    let mut out = String::new();
    let categories = db.terms_for_post(post.id, Some("category")).await.unwrap_or_default();
    let tags = db.terms_for_post(post.id, Some("post_tag")).await.unwrap_or_default();
    let mut others: Vec<(String, rustpress_db::Term)> = Vec::new();
    for (taxonomy, _) in taxonomies {
        if matches!(taxonomy.as_str(), "category" | "post_tag") {
            continue;
        }
        for term in db.terms_for_post(post.id, Some(taxonomy)).await.unwrap_or_default() {
            others.push((taxonomy.clone(), term));
        }
    }
    let is_sticky = sticky.contains(&post.id);
    let thumbnail = db.post_meta(post.id).await.unwrap_or_default().iter().any(|m| m.meta_key.as_deref() == Some("_thumbnail_id") && m.meta_value.as_deref().and_then(|v| v.parse::<u64>().ok()).is_some_and(|id| id > 0));
    let author_row = authors.iter().find(|u| u.id == post.post_author);
    // `wp_check_post_lock()`: another user edited the post within the last 150 seconds.
    let lock_holder = {
        let lock = db.post_meta(post.id).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_edit_lock")).and_then(|m| m.meta_value);
        let parsed = lock.as_deref().and_then(|v| v.split_once(':')).and_then(|(time, user)| Some((time.parse::<i64>().ok()?, user.parse::<u64>().ok()?)));
        match parsed {
            Some((time, user)) if time > ctx.env.now - 150 && user != ctx.user.id => db.user(user).await.ok().flatten(),
            _ => None,
        }
    };
    // `get_post_ancestors()`: the class counts the ancestors whatever the display; the title of a child shown at the
    // top (search results) finds its level the same way and names its parent (`column_title()`).
    let mut ancestors: Vec<Post> = Vec::new();
    let mut next = post.post_parent;
    while next != 0 && next != post.id && !ancestors.iter().any(|a| a.id == next) {
        let Some(parent) = db.post(next).await.ok().flatten() else { break };
        next = parent.post_parent;
        ancestors.push(parent);
    }
    let (level, parent_name) = if hierarchical_display && level == 0 && post.post_parent > 0 && !ancestors.is_empty() {
        (ancestors.len(), ancestors.first().map(|p| site.texturize(&p.post_title)))
    } else {
        (level, None)
    };
    let class = format!(
        "iedit author-{} {}level-{} {}",
        if post.post_author == ctx.user.id { "self" } else { "other" },
        if lock_holder.is_some() { "wp-locked " } else { "" },
        ancestors.len(),
        post_class(post, &categories, &tags, &others, is_sticky, thumbnail)
    );
    out.push_str(&format!("\t\t{}<tr id=\"post-{}\" class=\"{class}\">\n\t\t\t", if hierarchical_display { "\t" } else { "" }, post.id));
    let title = draft_or_title(ctx, post);
    let can_edit = ctx.user.can_edit_post(post);
    for column in columns {
        match column.key.as_str() {
            "cb" => {
                if can_edit {
                    out.push_str(&format!(
                        "<td class=\"check-column\">\t\t\t<input id=\"cb-select-{id}\" type=\"checkbox\" name=\"post[]\" value=\"{id}\" />\n\t\t\t<label for=\"cb-select-{id}\">\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</label>\n\t\t\t<div class=\"locked-indicator\">\n\t\t\t\t<span class=\"locked-indicator-icon\" aria-hidden=\"true\"></span>\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t\t</span>\n\t\t\t</div>\n\t\t\t</td>",
                        rustpress_core::i18n::sprintf(&tr("Select %s"), &[&title]),
                        rustpress_core::i18n::sprintf(&tr("&#8220;%s&#8221; is locked"), &[&title]),
                        id = post.id
                    ));
                } else {
                    out.push_str("<td class=\"check-column\"></td>");
                }
            }
            "title" => {
                let aria = if post.post_title.is_empty() { tr("no title") } else { post.post_title.clone() };
                let pad = "<span aria-hidden=\"true\">&#8212;</span> ".repeat(level);
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
                let mut states = post_states(ctx, post, status_param, is_sticky);
                if let Some(name) = &parent_name {
                    states.push_str(&format!(" | {} {}", tr("Parent Page:"), esc_html(name)));
                }
                let name_html = if can_edit && post.post_status != "trash" {
                    format!("{pad}<a class=\"row-title\" href=\"{edit_link}\"{described}>{title}</a>{hierarchy_linked}")
                } else {
                    format!("{pad}<span>{title}{hierarchy_nolink}</span>")
                };
                let (locked_avatar, locked_text) = match &lock_holder {
                    Some(holder) => {
                        let url = |size: u64| esc_url(&rustpress_blocks::content::avatar_url(&ctx.site().options, &holder.user_email, size));
                        (
                            format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-18 photo' height='18' width='18' loading='lazy' decoding='async'/>", url(18), url(36)),
                            esc_html(&rustpress_core::i18n::sprintf(&ctx.tr("%s is currently editing"), &[&holder.display_name])),
                        )
                    }
                    None => (String::new(), String::new()),
                };
                let inline = if can_edit { inline_data(ctx, post, info, taxonomies, is_sticky).await } else { String::new() };
                let excerpt = if mode == "excerpt" && !info.hierarchical && ctx.user.can_read_post(post) { list_excerpt(ctx, post).await } else { String::new() };
                out.push_str(&format!(
                    "<th scope=\"row\" class=\"title column-title has-row-actions column-primary page-title\" data-colname=\"{}\" aria-label=\"{}\"><div class=\"locked-info\"><span class=\"locked-avatar\">{locked_avatar}</span> <span class=\"locked-text\">{locked_text}</span></div>\n<strong>{name_html}{states}</strong>\n{excerpt}{inline}{}<button type=\"button\" class=\"toggle-row\"><span class=\"screen-reader-text\">{}</span></button></th>",
                    esc_attr(&column.label),
                    esc_attr(&aria),
                    row_actions_for(ctx, post, &title, can_edit, info).await,
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
                out.push_str(&format!("<td class='author column-author' data-colname=\"{}\">{cell}</td>", tr("Author")));
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
                    links.join(&site.tr(", "))
                };
                out.push_str(&format!("<td class='{} column-{}' data-colname=\"{}\">{cell}</td>", column.key, column.key, esc_attr(&label)));
            }
            key if key.starts_with("taxonomy-") => {
                // `column_default()`: the terms as links to the list filtered by them.
                let taxonomy = &key["taxonomy-".len()..];
                let terms = db.terms_for_post(post.id, Some(taxonomy)).await.unwrap_or_default();
                // `labels->no_terms`, and the term in the address: by its query variable, or as `taxonomy` and `term`.
                let (no_terms, query_var) = if taxonomy == "wp_pattern_category" { (tr("No pattern categories"), None) } else { (tr("No tags"), Some(taxonomy)) };
                let cell = if terms.is_empty() {
                    format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{no_terms}</span>")
                } else {
                    let links: Vec<String> = terms
                        .iter()
                        .map(|t| {
                            let mut url = "edit.php".to_owned();
                            if post.post_type != "post" {
                                url = add_query_arg(&url, "post_type", &post.post_type);
                            }
                            url = match query_var {
                                Some(var) => add_query_arg(&url, var, &t.slug),
                                None => add_query_arg(&add_query_arg(&url, "taxonomy", taxonomy), "term", &t.slug),
                            };
                            format!("<a href=\"{}\">{}</a>", esc_url(&url), esc_html(&t.name))
                        })
                        .collect();
                    links.join(&site.tr(", "))
                };
                out.push_str(&format!("<td class='{key} column-{key}' data-colname=\"{}\">{cell}</td>", esc_attr(&column.label)));
            }
            "comments" => {
                let pending_count = pending.iter().find(|(id, _)| *id == post.id).map_or(0, |(_, n)| *n);
                out.push_str(&format!("<td class='comments column-comments' data-colname=\"{}\">\t\t<div class=\"post-com-count-wrapper\">\n\t\t{}\t\t</div>\n\t\t</td>", esc_attr(&strip_tags_label(&column.label, ctx)), comments_bubble(ctx, post, pending_count)));
            }
            "date" => {
                out.push_str(&format!("<td class='date column-date' data-colname=\"{}\">{}</td>", tr("Date"), date_cell(ctx, post)));
            }
            _ => {}
        }
    }
    out.push_str("\t\t</tr>\n\t\t");
    out
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
    let separator = site.tr(", ");
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
        // Patterns have no quick edit.
        if info.name != "wp_block" {
            actions.push(("inline hide-if-no-js".into(), format!("<button type=\"button\" class=\"button-link editinline\" aria-label=\"{}\" aria-expanded=\"false\">{}</button>", esc_attr(&rustpress_core::i18n::sprintf(&tr("Quick edit &#8220;%s&#8221; inline"), &[title])), tr("Quick&nbsp;Edit"))));
        }
    }
    let can_delete = ctx.user.can_delete_post(post);
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
    if info.name == "wp_block" {
        actions.push((
            "export".into(),
            format!(
                "<button type=\"button\" class=\"wp-list-reusable-blocks__export button-link\" data-id=\"{}\" aria-label=\"{}\">{}</button>",
                post.id,
                esc_attr(&rustpress_core::i18n::sprintf(&tr("Export &#8220;%s&#8221; as JSON"), &[title])),
                tr("Export as JSON")
            ),
        ));
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

/// The excerpt of a row in the excerpt view: `esc_html( get_the_excerpt() )`, in a span for a protected post.
async fn list_excerpt(ctx: &Ctx<'_>, post: &Post) -> String {
    let query = rustpress_core::QueryResult::blank();
    let env = rustpress_blocks::Env::new(ctx.site(), &query, ctx.env.theme, ctx.env.registry);
    let shared = std::sync::Arc::new(post.clone());
    let excerpt = rustpress_blocks::excerpt::get_the_excerpt(&env, &shared).await.unwrap_or_default();
    if rustpress_blocks::content::post_password_required(post) {
        format!("<span class=\"protected-post-excerpt\">{}</span>", esc_html(&excerpt))
    } else {
        esc_html(&excerpt)
    }
}

/// The help of the posts and pages lists (`edit.php`); other post types have none.
fn list_help(ctx: &Ctx<'_>, info: &TypeInfo) -> Option<Help> {
    let tr = |s: &str| ctx.tr(s);
    let li = |items: &[&str]| format!("<ul>{}</ul>", items.iter().map(|s| format!("<li>{}</li>", tr(s))).collect::<String>());
    let tab = |id: &str, title: &str, content: String| HelpTab { id: id.into(), title: tr(title), content };
    let sidebar = |documentation: &str| format!("<p><strong>{}</strong></p><p>{}</p><p>{}</p>", tr("For more information:"), tr(documentation), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>"));
    match info.name.as_str() {
        "post" => Some(Help {
            tabs: vec![
                tab("overview", "Overview", format!("<p>{}</p>", tr("This screen provides access to all of your posts. You can customize the display of this screen to suit your workflow."))),
                tab("screen-content", "Screen Content", format!("<p>{}</p>{}", tr("You can customize the display of this screen&#8217;s contents in a number of ways:"), li(&[
                    "You can hide/display columns based on your needs and decide how many posts to list per screen using the Screen Options tab.",
                    "You can filter the list of posts by post status using the text links above the posts list to only show posts with that status. The default view is to show all posts.",
                    "You can view posts in a simple title list or with an excerpt using the Screen Options tab.",
                    "You can refine the list to show only posts in a specific category or from a specific month by using the dropdown menus above the posts list. Click the Filter button after making your selection. You also can refine the list by clicking on the post author, category or tag in the posts list.",
                ]))),
                tab("action-links", "Available Actions", format!("<p>{}</p>{}", tr("Hovering over a row in the posts list will display action links that allow you to manage your post. You can perform the following actions:"), li(&[
                    "<strong>Edit</strong> takes you to the editing screen for that post. You can also reach that screen by clicking on the post title.",
                    "<strong>Quick Edit</strong> provides inline access to the metadata of your post, allowing you to update post details without leaving this screen.",
                    "<strong>Trash</strong> removes your post from this list and places it in the Trash, from which you can permanently delete it.",
                    "<strong>Preview</strong> will show you what your draft post will look like if you publish it. View will take you to your live site to view the post. Which link is available depends on your post&#8217;s status.",
                ]))),
                tab("bulk-actions", "Bulk actions", format!(
                    "<p>{}</p><p>{}</p>",
                    tr("You can also edit or move multiple posts to the Trash at once. Select the posts you want to act on using the checkboxes, then select the action you want to take from the Bulk actions menu and click Apply."),
                    rustpress_core::i18n::sprintf(&tr("When using Bulk Edit, you can change the metadata (categories, author, etc.) for all selected posts at once. To remove a post from the grouping, just click the %s button next to its name in the Bulk Edit area that appears."), &[&format!("<span class=\"dashicons dashicons-dismiss\" aria-hidden=\"true\" style=\"font-size: 16px; width: 16px; vertical-align: middle;\"></span><span class=\"screen-reader-text\">{}</span>", tr("remove"))])
                )),
            ],
            sidebar: sidebar("<a href=\"https://wordpress.org/documentation/article/posts-screen/\">Documentation on Managing Posts</a>"),
        }),
        "page" => Some(Help {
            tabs: vec![
                tab("overview", "Overview", format!("<p>{}</p>", tr("Pages are similar to posts in that they have a title, body text, and associated metadata, but they are different in that they are not part of the chronological blog stream, kind of like permanent posts. Pages are not categorized or tagged, but can have a hierarchy. You can nest pages under other pages by making one the &#8220;Parent&#8221; of the other, creating a group of pages."))),
                tab("managing-pages", "Managing Pages", format!(
                    "<p>{}</p><p>{}</p>",
                    tr("Managing pages is very similar to managing posts, and the screens can be customized in the same way."),
                    tr("You can also perform the same types of actions, including narrowing the list by using the filters, acting on a page using the action links that appear when you hover over a row, or using the Bulk actions menu to edit the metadata for multiple pages at once.")
                )),
            ],
            sidebar: sidebar("<a href=\"https://wordpress.org/documentation/article/pages-screen/\">Documentation on Managing Pages</a>"),
        }),
        _ => None,
    }
}
