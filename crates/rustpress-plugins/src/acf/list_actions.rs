//! The actions of ACF's lists on `current_screen` (`ACF_Admin_Internal_Post_Type_List::check_duplicate()`,
//! `check_activate()`, `check_deactivate()`): a link or the bulk action does the work and redirects, the redirect
//! shows a notice (`acf_add_admin_notice()`, rendered on `admin_notices`).
//!
//! Differs from the plugin: no local JSON, so no "Sync".

use rustpress_core::formatting::{esc_html, esc_url};
use rustpress_core::native::{AdminContext, PageLoad};
use rustpress_core::{Result, Site};
use rustpress_db::Post;
use rustpress_phpser::Value;

use super::php::string;

/// The list the screen is.
fn list_of(ctx: &AdminContext<'_>) -> Option<&'static str> {
    let listed = ctx.screen_id.strip_prefix("edit-")?;
    ["acf-field-group", "acf-post-type", "acf-taxonomy"].into_iter().find(|t| *t == listed)
}

/// `$_REQUEST[ $name ]` (`acf_request_arg()`), sanitized.
fn request<'a>(ctx: &'a AdminContext<'a>, name: &str) -> Option<&'a str> {
    ctx.post_field(name).or_else(|| ctx.get(name))
}

/// `post[]` of the request.
fn posted_ids(ctx: &AdminContext<'_>) -> Vec<u64> {
    let mut out: Vec<u64> = ctx.query.iter().chain(ctx.form.iter()).filter(|(k, _)| k == "post[]").map(|(_, v)| super::php::intval(Some(&string(v))).max(0) as u64).collect();
    out.dedup();
    out
}

/// `get_admin_url( $params )` with `paged` of the request.
fn list_url(ctx: &AdminContext<'_>, post_type: &str, params: &str) -> String {
    let mut params = params.to_owned();
    if let Some(paged) = ctx.get("paged") {
        params.push_str(&format!("&paged={}", super::php::intval(Some(&string(paged)))));
    }
    ctx.site.site_url(&format!("wp-admin/edit.php?post_type={post_type}{params}"))
}

/// `check_admin_referer( 'bulk-posts' )`: an invalid nonce ends with `wp_nonce_ays()`.
fn referer_ok(ctx: &AdminContext<'_>) -> bool {
    request(ctx, "_wpnonce").is_some_and(|nonce| (ctx.verify_nonce)(nonce, "bulk-posts"))
}

/// `wp_nonce_ays( 'bulk-posts' )`.
fn expired(ctx: &AdminContext<'_>) -> PageLoad {
    let site = ctx.site;
    PageLoad::Die { status: 403, title: site.tr("An error occurred."), message: site.tr("The link you followed has expired.") }
}

/// The items a request asks to act on: `$_GET[ $action ]` or the bulk action of the lower select (`action2`).
fn targets(ctx: &AdminContext<'_>, action: &str) -> Vec<u64> {
    match request(ctx, action).filter(|v| super::php::truthy(Some(&string(v)))) {
        Some(id) => vec![super::php::intval(Some(&string(id))).max(0) as u64],
        None if request(ctx, "action2") == Some(action) => posted_ids(ctx),
        None => Vec::new(),
    }
}

/// `acf_update_internal_post_type_active_status()`: the item saved again with `active` changed.
async fn set_active(ctx: &AdminContext<'_>, post: &Post, active: bool) -> Result<bool> {
    let Some(writer) = ctx.writer else { return Ok(false) };
    let defaults = match post.post_type.as_str() {
        "acf-field-group" => Value::Array(super::save_group::group_defaults()),
        "acf-post-type" => super::internal::post_type_defaults(),
        "acf-taxonomy" => super::internal::taxonomy_defaults(),
        _ => return Ok(false),
    };
    let acf_post = rustpress_db::AcfPost { id: post.id, parent: post.post_parent, title: post.post_title.clone(), name: post.post_name.clone(), excerpt: post.post_excerpt.clone(), menu_order: i64::from(post.menu_order), status: post.post_status.clone(), content: post.post_content.clone() };
    let Value::Array(mut items) = super::internal::validated(&acf_post, defaults) else { return Ok(false) };
    super::php::set(&mut items, "active", Value::Bool(active));
    let system = rustpress_auth::CurrentUser::system();
    let content = rustpress_content::content::Content { site: ctx.site, db: &ctx.site.db, writer, user: &system, now: ctx.now };
    // `update_post()` unslashes the loaded item.
    let Value::Array(items) = super::php::wp_unslash(Value::Array(items)) else { return Ok(false) };
    super::save_group::update_internal_post(&content, Some(ctx.wp_root), &post.post_type, items).await?;
    Ok(true)
}

/// `current_screen` of the lists: the actions that redirect, or an expired link.
pub async fn load(ctx: &AdminContext<'_>) -> Result<Option<PageLoad>> {
    let Some(post_type) = list_of(ctx) else { return Ok(None) };
    if !(ctx.can)("manage_options") {
        return Ok(None);
    }
    let site = ctx.site;
    for action in ["acfduplicate", "acfactivate", "acfdeactivate"] {
        if request(ctx, &format!("{action}complete")).is_some_and(|v| super::php::truthy(Some(&string(v)))) {
            if !referer_ok(ctx) {
                return Ok(Some(expired(ctx)));
            }
            continue;
        }
        let ids = targets(ctx, action);
        if ids.is_empty() {
            continue;
        }
        if !referer_ok(ctx) {
            return Ok(Some(expired(ctx)));
        }
        let mut done: Vec<u64> = Vec::new();
        for id in ids {
            let Some(post) = site.db.post(id).await? else { continue };
            match action {
                "acfduplicate" => {
                    if post.post_type == "acf-field-group"
                        && let Some(new_id) = super::save_group::duplicate_field_group(ctx, &post).await?
                    {
                        done.push(new_id);
                    }
                }
                _ => {
                    if set_active(ctx, &post, action == "acfactivate").await? {
                        done.push(id);
                    }
                }
            }
        }
        let nonce = (ctx.nonce)("bulk-posts");
        let ids = done.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        return Ok(Some(PageLoad::Redirect(list_url(ctx, post_type, &format!("&_wpnonce={nonce}&{action}complete={ids}")))));
    }
    Ok(None)
}

/// `get_action_notice_text( $action, $count )` of the three lists.
fn notice_text(site: &Site, post_type: &str, action: &str, count: usize) -> String {
    let (one, many) = match (post_type, action) {
        ("acf-field-group", "acfactivatecomplete") => ("Field group activated.", "%s field groups activated."),
        ("acf-field-group", "acfdeactivatecomplete") => ("Field group deactivated.", "%s field groups deactivated."),
        ("acf-field-group", "acfduplicatecomplete") => ("Field group duplicated.", "%s field groups duplicated."),
        ("acf-post-type", "acfactivatecomplete") => ("Post type activated.", "%s post types activated."),
        ("acf-post-type", "acfdeactivatecomplete") => ("Post type deactivated.", "%s post types deactivated."),
        ("acf-post-type", "acfduplicatecomplete") => ("Post type duplicated.", "%s post types duplicated."),
        ("acf-taxonomy", "acfactivatecomplete") => ("Taxonomy activated.", "%s taxonomies activated."),
        ("acf-taxonomy", "acfdeactivatecomplete") => ("Taxonomy deactivated.", "%s taxonomies deactivated."),
        ("acf-taxonomy", "acfduplicatecomplete") => ("Taxonomy duplicated.", "%s taxonomies duplicated."),
        _ => return String::new(),
    };
    rustpress_core::i18n::sprintf(&site.i18n().ngettext("acf", one, many, count as i64), &[&count.to_string()])
}

/// `acf_render_admin_notices()` of the notices `check_*()` add after a redirect.
pub async fn notices(ctx: &AdminContext<'_>) -> Result<String> {
    let Some(post_type) = list_of(ctx) else { return Ok(String::new()) };
    if !(ctx.can)("manage_options") {
        return Ok(String::new());
    }
    let site = ctx.site;
    let mut out = String::new();
    for action in ["acfduplicatecomplete", "acfactivatecomplete", "acfdeactivatecomplete"] {
        let Some(ids) = request(ctx, action).filter(|v| super::php::truthy(Some(&string(v)))) else { continue };
        let ids: Vec<i64> = ids.split(',').map(|s| super::php::intval(Some(&string(s)))).collect();
        let mut links = Vec::new();
        for id in &ids {
            let post = if *id > 0 { site.db.post(*id as u64).await? } else { None };
            let title = post.as_ref().map(|p| rustpress_blocks::content::the_title_in_admin(site, p)).unwrap_or_default();
            let edit = post.as_ref().map(|p| site.site_url(&format!("wp-admin/post.php?post={}&amp;action=edit", p.id))).unwrap_or_default();
            links.push(format!("<a href=\"{}\">{}</a>", esc_url(&edit), esc_html(&title)));
        }
        let text = format!("{} {}", notice_text(site, post_type, action, ids.len()), links.join(", "));
        // `acf_punctify()`.
        let trimmed = text.trim();
        let shown = if rustpress_core::formatting::wp_strip_all_tags(trimmed).trim().ends_with('.') { trimmed.to_owned() } else { format!("{trimmed}.") };
        out.push_str(&format!(
            "<div class=\"acf-admin-notice notice notice-success is-dismissible\" data-persisted=\"false\" data-persist-id=\"{:x}\">{}</div>",
            md5::compute(text.as_bytes()),
            super::render::esc_html_acf(&string(&rustpress_core::formatting::wpautop(&shown)))
        ));
    }
    Ok(out)
}
