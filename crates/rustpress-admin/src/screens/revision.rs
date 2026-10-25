//! `wp-admin/revision.php`: comparing and restoring the revisions of a post, and the `get-revision-diffs` action
//! of `admin-ajax.php` that the screen's JavaScript asks for the differences.

use rustpress_auth::CurrentUser;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::text_diff::{DiffArgs, TableLabels, wp_text_diff};
use rustpress_db::{Order, OrderBy, Post, PostQuery};
use serde_json::{Map, Value, json};

use super::ajax::json as json_response;
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::{Content, PostData};

const TEMPLATES: &str = include_str!("../data/revision_templates.html");

/// `absint()`.
fn absint(value: &str) -> u64 {
    value.trim().parse::<i64>().map(i64::unsigned_abs).unwrap_or(0)
}

/// `wp_is_post_autosave()`.
fn is_autosave(revision: &Post) -> bool {
    revision.post_name.contains(&format!("{}-autosave", revision.post_parent))
}

/// `wp_revisions_enabled()`.
fn revisions_enabled(env: &AdminEnv<'_>, post: &Post) -> bool {
    env.site.post_type_supports(&post.post_type, "revisions")
}

fn may_edit(user: &CurrentUser, post: &Post) -> bool {
    let page = post.post_type == "page";
    user.can(if page { "edit_pages" } else { "edit_posts" }) && (post.post_author == user.id || user.can("edit_others_posts") || user.can("edit_others_pages"))
}

/// The revision and its parent, when the user may see them.
async fn revision_and_parent(env: &AdminEnv<'_>, user: &CurrentUser, revision_id: u64) -> Option<(Post, Post)> {
    let revision = env.db.post(revision_id).await.ok().flatten().filter(|p| p.post_type == "revision")?;
    let post = env.db.post(revision.post_parent).await.ok().flatten()?;
    may_edit(user, &post).then_some((revision, post))
}

/// `_wp_post_revision_fields()`: `(field, name)`.
fn revision_fields(env: &AdminEnv<'_>) -> [(&'static str, String); 3] {
    [("post_title", env.site.tr("Title")), ("post_content", env.site.tr("Content")), ("post_excerpt", env.site.tr("Excerpt"))]
}

fn field_of<'a>(post: &'a Post, field: &str) -> &'a str {
    match field {
        "post_title" => &post.post_title,
        "post_content" => &post.post_content,
        _ => &post.post_excerpt,
    }
}

/// `wp_get_revision_ui_diff()`: `[{id, name, diff}]`, or `None` for revisions that do not belong together.
async fn revision_ui_diff(env: &AdminEnv<'_>, post_id: u64, compare_from: u64, compare_to: u64) -> Option<Vec<Value>> {
    let site = env.site;
    let post = env.db.post(post_id).await.ok().flatten()?;
    let mut from = if compare_from != 0 { Some(env.db.post(compare_from).await.ok().flatten()?) } else { None };
    let mut to = env.db.post(compare_to).await.ok().flatten()?;
    if let Some(f) = &from
        && f.post_parent != post.id
        && f.id != post.id
    {
        return None;
    }
    if to.post_parent != post.id && to.id != post.id {
        return None;
    }
    let gmt = |p: &Post| rustpress_core::datetime::SiteTimezone::gmt_timestamp(&p.post_date_gmt).unwrap_or(0);
    if let Some(f) = from.take() {
        if gmt(&f) > gmt(&to) {
            from = Some(std::mem::replace(&mut to, f));
        } else {
            from = Some(f);
        }
    }
    let no_title = site.tr("(no title)");
    if let Some(f) = from.as_mut()
        && f.post_title.is_empty()
    {
        f.post_title = no_title.clone();
    }
    if to.post_title.is_empty() {
        to.post_title = no_title;
    }
    let mut out = Vec::new();
    for (field, name) in revision_fields(env) {
        let content_from = from.as_ref().map(|p| field_of(p, field).to_owned()).unwrap_or_default();
        let content_to = field_of(&to, field).to_owned();
        let args = DiffArgs {
            show_split_view: true,
            title_left: site.tr("Removed"),
            title_right: site.tr("Added"),
            labels: TableLabels { added: site.tr("Added:"), deleted: site.tr("Deleted:"), unchanged: site.tr("Unchanged:") },
            ..Default::default()
        };
        let mut diff = wp_text_diff(&content_from, &content_to, &args);
        if diff.is_empty() && field == "post_title" {
            // It's a better user experience to still show the Title, even if it didn't change.
            diff = String::from("<table class=\"diff\"><colgroup><col class=\"content diffsplit left\"><col class=\"content diffsplit middle\"><col class=\"content diffsplit right\"></colgroup><tbody><tr>");
            let left = from.as_ref().map(|p| p.post_title.clone()).unwrap_or_default();
            diff.push_str(&format!("<td>{}</td><td></td><td>{}</td>", esc_html(&left), esc_html(&to.post_title)));
            diff.push_str("</tr></tbody></table>");
        }
        if !diff.is_empty() {
            out.push(json!({"id": field, "name": name, "diff": diff}));
        }
    }
    Some(out)
}

/// `wp_prepare_revisions_for_js()`.
async fn revisions_for_js(ctx: &Ctx<'_>, post: &Post, selected: u64, from: Option<u64>) -> Value {
    let env = ctx.env;
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let query = PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(post.id), order_by: OrderBy::Date, order: Order::Asc, ..Default::default() };
    let mut revisions: Vec<Post> = env.db.posts(&query).await.unwrap_or_default();
    revisions.sort_by(|a, b| a.post_date.cmp(&b.post_date).then(a.id.cmp(&b.id)));
    // If revisions are disabled, we only want autosaves and the current post.
    if !revisions_enabled(env, post) {
        revisions.retain(is_autosave);
        revisions.insert(0, post.clone());
    }
    let show_avatars = site.options.string("show_avatars", "1") != "0";
    let can_restore = may_edit(&ctx.user, post);
    let now = env.now;
    let mut authors: Map<String, Value> = Map::new();
    let mut data: Vec<Value> = Vec::new();
    let mut current_index: Option<usize> = None;
    for revision in &revisions {
        let modified_gmt = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&revision.post_modified_gmt).unwrap_or(0);
        let restore_url = if can_restore {
            Value::from(format!("{}&_wpnonce={}", ctx.admin_url(&format!("revision.php?revision={}&action=restore", revision.id)), ctx.create_nonce(&format!("restore-post_{}", revision.id))))
        } else {
            Value::Bool(false)
        };
        let key = revision.post_author.to_string();
        if !authors.contains_key(&key) {
            let user = env.db.user(revision.post_author).await.ok().flatten();
            let (name, email) = user.map(|u| (u.display_name.clone(), u.user_email.clone())).unwrap_or_default();
            let avatar = if show_avatars {
                let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&site.options, &email, s));
                format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-32 photo' height='32' width='32' loading='lazy' decoding='async'/>", url(32), url(64))
            } else {
                String::new()
            };
            authors.insert(key.clone(), json!({"id": revision.post_author, "avatar": avatar, "name": name}));
        }
        let autosave = is_autosave(revision);
        let mut current = !autosave && revision.post_modified_gmt == post.post_modified_gmt;
        if current && let Some(previous) = current_index {
            // If multiple revisions have the same post_modified_gmt, highest ID is current.
            if data[previous]["id"].as_u64().unwrap_or(0) < revision.id {
                data[previous]["current"] = Value::Bool(false);
                current_index = Some(data.len());
            } else {
                current = false;
            }
        } else if current {
            current_index = Some(data.len());
        }
        // `strtotime( post_modified )` in UTC, formatted as it is: the wall-clock time of the post.
        let stamp = modified_gmt;
        data.push(json!({
            "id": revision.id,
            "title": rustpress_blocks::content::the_title_for(site, post),
            "author": authors[&key].clone(),
            "date": site.date(&tr("M j, Y @ H:i"), stamp),
            "dateShort": site.date(&site.trx("j M Y @ H:i", "revision date short format"), stamp),
            "timeAgo": sprintf(&tr("%s ago"), &[&super::media_lib::human_time_diff(ctx, modified_gmt, now)]),
            "autosave": autosave,
            "current": current,
            "restoreUrl": restore_url,
        }));
    }
    // No revision is marked current: take the newest one that is not an autosave.
    if current_index.is_none()
        && let Some(last) = data.iter().rposition(|r| r["autosave"] == Value::Bool(false)).or(data.len().checked_sub(1))
    {
        data[last]["current"] = Value::Bool(true);
    }
    let ids: Vec<u64> = data.iter().filter_map(|r| r["id"].as_u64()).collect();
    let compare_two = from.is_some();
    let from = match from {
        Some(f) => f,
        None => match ids.iter().position(|id| *id == selected) {
            Some(found) if found > 0 => ids[found - 1],
            _ => 0,
        },
    };
    let diffs = revision_ui_diff(env, post.id, from, selected).await;
    json!({
        "postId": post.id,
        "nonce": ctx.create_nonce("revisions-ajax-nonce"),
        "revisionData": data,
        "to": selected,
        "from": from,
        "diffData": [{"id": format!("{from}:{selected}"), "fields": diffs.map_or(Value::Bool(false), Value::from)}],
        "baseUrl": format!("{}/wp-admin/revision.php", ctx.site().site_url("").trim_end_matches('/').split_once("://").map_or("", |(_, rest)| rest.split_once('/').map_or("", |(_, path)| path)).trim_end_matches('/')),
        "compareTwoMode": u64::from(compare_two),
        "revisionIds": ids,
    })
}

fn edit_link(ctx: &Ctx<'_>, post: &Post) -> String {
    ctx.admin_url(&format!("post.php?post={}&action=edit", post.id))
}

fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let overview = format!(
        "<p>{}</p><p>{}</p><p>{}</p><ul><li>{}</li><li>{}</li><li>{}</li></ul>",
        tr("This screen is used for managing your content revisions."),
        tr("Revisions are saved copies of your post or page, which are periodically created as you update your content. The red text on the left shows the content that was removed. The green text on the right shows the content that was added."),
        tr("From this screen you can review, compare, and restore revisions:"),
        tr("To navigate between revisions, <strong>drag the slider handle left or right</strong> or <strong>use the Previous or Next buttons</strong>."),
        tr("Compare two different revisions by <strong>selecting the &#8220;Compare any two revisions&#8221; box</strong> to the side."),
        tr("To restore a revision, <strong>click Restore This Revision</strong>."),
    );
    Help {
        tabs: vec![HelpTab { id: "revisions-overview".into(), title: tr("Overview"), content: overview }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/documentation/article/revisions/\">Revisions Management</a>"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

/// `wp-admin/revision.php`.
pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let env = ctx.env;
    let to = request.request("to").filter(|v| v.parse::<f64>().is_ok()).map(absint).filter(|v| *v != 0);
    let from = request.request("from").filter(|v| v.parse::<f64>().is_ok()).map(absint).filter(|v| *v != 0);
    let mut revision_id = request.request("revision").map(absint).unwrap_or(0);
    if revision_id == 0 {
        revision_id = to.unwrap_or(0);
    }
    let action = rustpress_content::sanitize::text_field(request.request("action").unwrap_or_default());
    let edit_php = ctx.admin_url("edit.php");

    if action == "restore" {
        let Some((revision, post)) = revision_and_parent(env, &ctx.user, revision_id).await else { return redirect(&edit_php) };
        if !revisions_enabled(env, &post) && !is_autosave(&revision) {
            return redirect(&ctx.admin_url(&format!("edit.php?post_type={}", post.post_type)));
        }
        if !ctx.verify_nonce(&format!("restore-post_{}", revision.id)) {
            return die(env, 403, &ctx.tr("WordPress Failure Notice"), &ctx.tr("The link you followed has expired."));
        }
        let Some(writer) = env.writer else { return super::common::read_only(ctx) };
        let content = Content { site: env.site, db: env.db, writer, user: &ctx.user, now: env.now };
        let data = PostData { id: Some(post.id), post_title: Some(revision.post_title.clone()), post_content: Some(revision.post_content.clone()), post_excerpt: Some(revision.post_excerpt.clone()), ..Default::default() };
        let _ = content.insert_post(&data).await;
        return redirect(&format!("{}&message=5&revision={}", edit_link(ctx, &post), revision.id));
    }

    let Some((revision, post)) = revision_and_parent(env, &ctx.user, revision_id).await else { return redirect(&edit_php) };
    if !revisions_enabled(env, &post) && !is_autosave(&revision) {
        return redirect(&ctx.admin_url(&format!("edit.php?post_type={}", post.post_type)));
    }
    let tr = |s: &str| ctx.tr(s);
    let edit = edit_link(ctx, &post);
    let shown_title = {
        let title = rustpress_blocks::content::the_title_for(env.site, &post);
        if post.post_title.is_empty() { tr("(no title)") } else { title }
    };
    let post_title = format!("<a href=\"{}\">{}</a>", esc_url(&edit), esc_html(&shown_title));
    let h1 = sprintf(&tr("Compare Revisions of &#8220;%s&#8221;"), &[&post_title]);
    let return_to_post = format!("<a href=\"{}\">{}</a>", esc_url(&edit), tr("&larr; Go to editor"));
    let templates = super::pages::translate_html(ctx, TEMPLATES);
    let content = format!("<div class=\"wrap\">\n\t<h1 class=\"long-header\">{h1}</h1>\n\t{return_to_post}</div>\n{templates}");

    let (parent_file, submenu) = if post.post_type == "post" { ("edit.php".to_owned(), "edit.php".to_owned()) } else { (format!("edit.php?post_type={}", post.post_type), format!("edit.php?post_type={}", post.post_type)) };
    let screen = Screen { file: "revision.php".into(), parent_file, submenu_file: Some(submenu), title: tr("Revisions"), help: Some(help(ctx)), ..Default::default() };
    let settings = revisions_for_js(ctx, &post, revision_id, from).await;
    let assets = Assets { scripts: vec!["revisions".into()], localize: vec![("revisions".into(), "_wpRevisionsSettings".into(), settings)], ..Default::default() };
    let _ = (esc_attr(""), to);
    render(ctx, &screen, &content, &assets).await
}

/// `wp_ajax_get_revision_diffs()`.
pub async fn ajax_diffs(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, _token: &str) -> AdminResponse {
    let failure = || json_response(200, &json!({"success": false}));
    let post_id = request.request("post_id").map(|v| v.trim().parse::<i64>().unwrap_or(0).unsigned_abs()).unwrap_or(0);
    let Some(post) = env.db.post(post_id).await.ok().flatten() else { return failure() };
    if !may_edit(user, &post) {
        return failure();
    }
    // Really just pre-loading the cache here.
    let query = PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(post.id), ..Default::default() };
    if env.db.posts(&query).await.unwrap_or_default().is_empty() {
        return failure();
    }
    let compare: Vec<String> = request.form.iter().chain(request.query.iter()).filter(|(k, _)| k == "compare[]").map(|(_, v)| v.clone()).collect();
    let mut out = Vec::new();
    for key in compare {
        let (from, to) = key.split_once(':').map_or((absint(&key), 0), |(f, t)| (absint(f), absint(t)));
        let fields = revision_ui_diff(env, post.id, from, to).await.map_or(Value::Bool(false), Value::from);
        out.push(json!({"id": key, "fields": fields}));
    }
    json_response(200, &json!({"success": true, "data": out}))
}
