//! Quick Edit and Bulk Edit of the posts list: `wp_ajax_inline_save()` and `bulk_edit_posts()`.

use rustpress_core::OrLog;
use rustpress_auth::CurrentUser;
use rustpress_core::formatting::esc_html;
use rustpress_db::Post;

use super::ajax::text;
use super::posts::{RowEnv, list_columns, post_row, type_info};
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use crate::content::{Content, PostData, TermRef};

/// `sanitize_key()`.
fn sanitize_key(value: &str) -> String {
    value.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect()
}

/// The ids of a list field: `name[]` repeated or `name[0]`.
fn list_field(request: &AdminRequest, name: &str) -> Option<Vec<String>> {
    let list: Vec<String> = request.form.iter().chain(request.query.iter()).filter(|(k, _)| k == &format!("{name}[]") || (k.starts_with(&format!("{name}[")) && k.ends_with(']'))).map(|(_, v)| v.clone()).collect();
    (!list.is_empty()).then_some(list)
}

/// The statuses a user may give a post (`_wp_translate_postdata()`).
fn translate_status(user: &CurrentUser, page: bool, post: &Post, requested: Option<String>) -> String {
    let publish_cap = if page { "publish_pages" } else { "publish_posts" };
    let can_publish = user.can(publish_cap);
    let previous = post.post_status.clone();
    let mut status = requested.map(|s| sanitize_key(&s)).filter(|s| !s.is_empty());
    if status.as_deref() == Some("auto-draft") {
        status = Some("draft".into());
    }
    if status.as_deref().is_some_and(|s| !matches!(s, "publish" | "future" | "draft" | "pending" | "private" | "trash")) {
        status = None;
    }
    if status.as_deref() == Some("private") && !can_publish {
        status = Some(if previous.is_empty() { "pending".into() } else { previous.clone() });
    }
    if matches!(status.as_deref(), Some("publish" | "future")) && !can_publish && (!matches!(previous.as_str(), "publish" | "future")) {
        status = Some("pending".into());
    }
    status.unwrap_or_else(|| if previous == "auto-draft" { "draft".into() } else { previous })
}

/// The date of a Quick Edit form when it differs from the stored one: `(post_date, post_date_gmt)`.
fn edited_date(ctx: &Ctx<'_>, request: &AdminRequest, post: &Post) -> Result<Option<(String, String)>, String> {
    let num = |name: &str| request.post_field(name).and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0);
    let now_local = ctx.site().date("Y-m-d H:i:s", ctx.env.now);
    let (cur_year, cur_month, cur_day): (i64, i64, i64) = (now_local[0..4].parse().unwrap_or(1970), now_local[5..7].parse().unwrap_or(1), now_local[8..10].parse().unwrap_or(1));
    let aa = if num("aa") <= 0 { cur_year } else { num("aa") };
    let mm = if num("mm") <= 0 { cur_month } else { num("mm") };
    let jj = if num("jj") > 31 { 31 } else if num("jj") <= 0 { cur_day } else { num("jj") };
    let hh = if num("hh") > 23 { num("hh") - 24 } else { num("hh") };
    let mn = if num("mn") > 59 { num("mn") - 60 } else { num("mn") };
    let ss = if num("ss") > 59 { num("ss") - 60 } else { num("ss") };
    let date = format!("{aa:04}-{mm:02}-{jj:02} {hh:02}:{mn:02}:{ss:02}");
    if !crate::content::valid_date(&date) {
        return Err(ctx.tr("Invalid date."));
    }
    if post.post_date == date {
        return Ok(None);
    }
    let gmt = ctx.site().timezone.local_timestamp(&date).map(|ts| rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", ts)).unwrap_or_else(|| date.clone());
    Ok(Some((date, gmt)))
}

/// `sticky_posts`.
fn sticky_ids(ctx: &Ctx<'_>) -> Vec<u64> {
    match rustpress_phpser::unserialize(ctx.site().options.string("sticky_posts", "").as_bytes()) {
        Ok(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_i64()).map(|i| i as u64).collect(),
        _ => Vec::new(),
    }
}

async fn set_sticky(ctx: &Ctx<'_>, post_id: u64, sticky: bool) {
    let Some(writer) = ctx.env.writer else { return };
    let mut ids = sticky_ids(ctx);
    let before = ids.clone();
    ids.retain(|i| *i != post_id);
    if sticky {
        ids.insert(0, post_id);
    }
    if ids != before {
        let value = rustpress_phpser::Value::Array(ids.iter().enumerate().map(|(i, id)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Int(*id as i64))).collect());
        writer.update_option("sticky_posts", &String::from_utf8_lossy(&rustpress_phpser::serialize(&value))).await.or_log();
    }
}

/// `set_post_format()`.
async fn set_post_format(content: &Content<'_>, post_id: u64, format: &str) {
    let format = sanitize_key(format);
    let terms = if format.is_empty() || format == "0" || format == "standard" { Vec::new() } else { vec![TermRef::Name(format!("post-format-{format}"))] };
    content.set_object_terms(post_id, &terms, "post_format", false).await.or_log();
}

/// The terms of `tax_input[<taxonomy>]` (a comma list or `[]` ids) for every taxonomy but categories and tags.
fn tax_input(request: &AdminRequest, taxonomies: &[(String, bool)]) -> Vec<(String, Vec<TermRef>)> {
    let mut out = Vec::new();
    for (name, hierarchical) in taxonomies {
        if matches!(name.as_str(), "category" | "post_tag" | "post_format") {
            continue;
        }
        let single = format!("tax_input[{name}]");
        if let Some(raw) = request.post_field(&single).or_else(|| request.get(&single)) {
            out.push((name.clone(), raw.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()).map(|t| TermRef::Name(t.to_owned())).collect()));
        } else if let Some(list) = list_field(request, &single) {
            let terms = list.iter().filter_map(|v| v.parse::<u64>().ok()).filter(|id| *id != 0).map(TermRef::Id).collect::<Vec<_>>();
            let _ = hierarchical;
            out.push((name.clone(), terms));
        }
    }
    out
}

/// `wp_ajax_inline_save()`: saves the Quick Edit form and answers with the new row.
pub(super) async fn inline_save(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let tr = |s: &str| ctx.tr(s);
    let nonce = request.request("_inline_edit").unwrap_or_default();
    if env.auth.verify_nonce(nonce, "inlineeditnonce", user.id, token, env.now).is_none() {
        return text(403, "-1");
    }
    let Some(post_id) = request.post_field("post_ID").and_then(|v| v.trim().parse::<u64>().ok()).filter(|id| *id != 0) else { return text(200, "") };
    let Some(post) = env.db.post(post_id).await.ok().flatten() else { return text(200, "") };
    let page = post.post_type == "page";
    let may_edit = user.can_edit_post(&post);
    if !may_edit {
        return text(200, &if page { tr("Sorry, you are not allowed to edit this page.") } else { tr("Sorry, you are not allowed to edit this post.") });
    }
    if let Some(holder) = super::ajax::post_lock_holder(env, user, post_id).await {
        let name = env.db.user(holder).await.ok().flatten().map(|u| u.display_name).unwrap_or_else(|| tr("Someone"));
        let message = if page { tr("Saving is disabled: %s is currently editing this page.") } else { tr("Saving is disabled: %s is currently editing this post.") };
        return text(200, &rustpress_core::i18n::sprintf(&message, &[&esc_html(&name)]));
    }
    let Some(writer) = env.writer else { return text(200, "") };
    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
    let Some(info) = type_info(&ctx, &post.post_type) else { return text(200, "") };
    let field = |name: &str| request.post_field(name);

    // Status: "keep private" wins over the menu.
    let requested = if field("keep_private") == Some("private") { Some("private".to_owned()) } else { field("_status").map(str::to_owned) };
    let status = translate_status(user, page, &post, requested);
    let publish_cap = if page { "publish_pages" } else { "publish_posts" };
    let password = if user.can(publish_cap) { field("post_password").map(str::to_owned) } else { None };
    let (post_date, post_date_gmt, edit_date) = match edited_date(&ctx, request, &post) {
        Ok(Some((date, gmt))) => (Some(date), Some(gmt), true),
        Ok(None) => (None, None, false),
        Err(message) => return text(200, &message),
    };
    // The author: somebody else's posts need `edit_others_posts`.
    let author = field("post_author").and_then(|a| a.parse::<u64>().ok()).filter(|a| *a != 0).unwrap_or(user.id);
    if author != user.id && !user.can("edit_others_posts") && !user.can("edit_others_pages") {
        return text(200, &if page { tr("Sorry, you are not allowed to edit pages as this user.") } else { tr("Sorry, you are not allowed to edit posts as this user.") });
    }
    // The name of a draft is made unique as if the post was published.
    let mut post_name = field("post_name").map(str::to_owned);
    if let Some(name) = post_name.as_ref().filter(|n| !n.is_empty())
        && matches!(post.post_status.as_str(), "draft" | "pending")
        && let Ok(unique) = content.unique_post_slug(&rustpress_core::slug::sanitize_title(name), post.id, "publish", &post.post_type, post.post_parent).await
    {
        post_name = Some(unique);
    }
    let taxonomies = content.object_taxonomies(&post.post_type);
    let categories: Option<Vec<u64>> = list_field(request, "post_category").map(|list| list.iter().filter_map(|v| v.parse::<u64>().ok()).filter(|id| *id != 0).collect()).filter(|_| user.can("edit_posts"));
    let tags: Option<Vec<String>> = field("tax_input[post_tag]").map(|raw| raw.split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect());
    let data = PostData {
        id: Some(post_id),
        post_author: Some(author),
        post_title: field("post_title").map(str::to_owned),
        post_status: Some(status),
        comment_status: Some(if field("comment_status").is_some_and(|v| !v.is_empty()) { "open".into() } else { "closed".into() }),
        ping_status: Some(if field("ping_status").is_some_and(|v| !v.is_empty()) { "open".into() } else { "closed".into() }),
        post_password: password,
        post_name,
        post_parent: field("post_parent").map(|p| p.parse::<u64>().unwrap_or(0)),
        menu_order: field("menu_order").map(|m| m.trim().parse().unwrap_or(0)),
        post_date,
        post_date_gmt,
        edit_date,
        post_category: categories,
        tags_input: tags,
        tax_input: tax_input(request, &taxonomies),
        page_template: field("page_template").filter(|t| *t != "-1").map(str::to_owned),
        ..Default::default()
    };
    if let Err(e) = content.insert_post(&data).await {
        return text(200, &esc_html(&e.to_string()));
    }
    writer.update_post_meta(post_id, "_edit_last", &user.id.to_string()).await.or_log();
    if info.name == "post" && user.can("edit_others_posts") && user.can("publish_posts") && request.post_field("sticky").is_some() {
        set_sticky(&ctx, post_id, field("sticky") == Some("sticky")).await;
    }
    if let Some(format) = field("post_format").filter(|f| *f != "-1") {
        set_post_format(&content, post_id, format).await;
    }

    // The new row.
    let Some(post) = env.db.post(post_id).await.ok().flatten() else { return text(200, "") };
    let mut level = 0;
    if info.hierarchical {
        let mut parent = post.post_parent;
        while parent > 0 {
            let Some(ancestor) = env.db.post(parent).await.ok().flatten() else { break };
            parent = ancestor.post_parent;
            level += 1;
        }
    }
    let status_param = "all".to_owned();
    let taxonomies = content.object_taxonomies(&post.post_type);
    let columns = list_columns(&ctx, &info, &status_param, &taxonomies);
    let sticky = sticky_ids(&ctx);
    let pending = env.db.pending_comment_counts(&[post.id]).await.unwrap_or_default();
    let authors = env.db.users_by_ids(&[post.post_author]).await.unwrap_or_default();
    let mut parents: Vec<Post> = Vec::new();
    if post.post_parent != 0 && let Ok(Some(parent)) = env.db.post(post.post_parent).await {
        parents.push(parent);
    }
    let row_env = RowEnv { info: &info, columns: &columns, status_param: &status_param, hierarchical_display: false, sticky: &sticky, taxonomies: &taxonomies, pending: &pending, authors: &authors, parents: &parents };
    text(200, &post_row(&ctx, &post, level, &row_env).await)
}

/// `bulk_edit_posts()`: `(updated, skipped, locked)`.
pub(super) async fn bulk_edit(ctx: &Ctx<'_>, ids: &[u64]) -> (u64, u64, u64) {
    let env = ctx.env;
    let request = ctx.request;
    let Some(writer) = env.writer else { return (0, ids.len() as u64, 0) };
    let content = Content { site: env.site, db: env.db, writer, user: &ctx.user, now: env.now };
    let field = |name: &str| request.request(name).filter(|v| !v.is_empty());
    let no_change = |v: &str| v == "-1";
    let (mut updated, mut skipped, mut locked) = (0, 0, 0);
    for id in ids {
        let Some(post) = env.db.post(*id).await.ok().flatten() else {
            skipped += 1;
            continue;
        };
        let page = post.post_type == "page";
        let may_edit = ctx.user.can_edit_post(&post);
        if !may_edit {
            skipped += 1;
            continue;
        }
        if super::ajax::post_lock_holder(env, &ctx.user, *id).await.is_some() {
            locked += 1;
            continue;
        }
        let taxonomies = content.object_taxonomies(&post.post_type);
        let status = field("_status").filter(|s| !no_change(s)).map(|s| translate_status(&ctx.user, page, &post, Some(s.to_owned())));
        let author = field("post_author").filter(|a| !no_change(a)).and_then(|a| a.parse::<u64>().ok()).filter(|_| ctx.can("edit_others_posts") || ctx.can("edit_others_pages"));
        let parent = field("post_parent").filter(|p| !no_change(p)).and_then(|p| p.parse::<u64>().ok()).filter(|p| *p != post.id);
        // Terms are added to the ones the post has.
        let mut category_ids: Vec<u64> = Vec::new();
        let mut set_categories = false;
        if let Some(list) = list_field(request, "post_category") {
            let chosen: Vec<u64> = list.iter().filter_map(|v| v.parse::<u64>().ok()).filter(|id| *id != 0).collect();
            if !chosen.is_empty() && taxonomies.iter().any(|(n, _)| n == "category") {
                category_ids = env.db.terms_for_post(post.id, Some("category")).await.unwrap_or_default().into_iter().map(|t| t.term_id).collect();
                for id in chosen {
                    if !category_ids.contains(&id) {
                        category_ids.push(id);
                    }
                }
                set_categories = true;
            }
        }
        let tags: Option<Vec<String>> = request.request("tax_input[post_tag]").filter(|t| !t.trim().is_empty()).map(|raw| raw.split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect());
        let mut data = PostData {
            id: Some(post.id),
            post_author: author,
            post_status: status,
            comment_status: field("comment_status").map(str::to_owned),
            ping_status: field("ping_status").map(str::to_owned),
            post_parent: parent,
            page_template: field("page_template").filter(|t| !no_change(t)).map(str::to_owned),
            post_category: set_categories.then_some(category_ids),
            ..Default::default()
        };
        if let Some(new_tags) = tags && taxonomies.iter().any(|(n, _)| n == "post_tag") {
            let mut merged: Vec<String> = env.db.terms_for_post(post.id, Some("post_tag")).await.unwrap_or_default().into_iter().map(|t| t.name).collect();
            for tag in new_tags {
                if !merged.contains(&tag) {
                    merged.push(tag);
                }
            }
            data.tags_input = Some(merged);
        }
        if content.insert_post(&data).await.is_err() {
            skipped += 1;
            continue;
        }
        if post.post_type == "post"
            && let Some(sticky) = field("sticky").filter(|s| !no_change(s))
            && ctx.can("edit_others_posts")
        {
            set_sticky(ctx, post.id, sticky == "sticky").await;
        }
        if let Some(format) = field("post_format").filter(|f| !no_change(f)) {
            set_post_format(&content, post.id, format).await;
        }
        updated += 1;
    }
    (updated, skipped, locked)
}
