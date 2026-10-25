//! `wp-admin/options-privacy.php`: the privacy policy page and the policy guide.

use rustpress_core::formatting::{esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_db::{Order, OrderBy, PostQuery};

use super::common::{forbidden_heading, read_only};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};
use crate::content::{Content, PostData};

const ROW: &str = include_str!("../data/privacy_row.tpl");
const DEFAULT_CONTENT: &str = include_str!("../data/privacy_default_content.html");

/// A notice of `settings_errors()`.
fn settings_error(code: &str, message: &str, kind: &str) -> String {
    format!("<div id='setting-error-{code}' class='notice notice-{kind} settings-error is-dismissible'> \n<p><strong>{message}</strong></p></div> \n")
}

/// `wp_dropdown_pages()` with the pages in the states `draft` and `publish`.
async fn pages_dropdown(ctx: &Ctx<'_>, selected_id: u64) -> String {
    let query = PostQuery { post_types: vec!["page".into()], statuses: vec!["draft".into(), "publish".into()], order_by: OrderBy::Title, order: Order::Asc, ..Default::default() };
    let pages = ctx.env.db.posts(&query).await.unwrap_or_default();
    let ids: Vec<u64> = pages.iter().map(|p| p.id).collect();
    fn walk(pages: &[rustpress_db::Post], ids: &[u64], parent: u64, level: usize, selected: u64, out: &mut String) {
        for page in pages.iter().filter(|p| p.post_parent == parent || (parent == 0 && !ids.contains(&p.post_parent))) {
            if parent != 0 && page.post_parent != parent {
                continue;
            }
            let title = if page.post_title.is_empty() { format!("#{} (no title)", page.id) } else { page.post_title.clone() };
            let sel = if page.id == selected { " selected=\"selected\"" } else { "" };
            out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\"{sel}>{}{}</option>\n", page.id, "&nbsp;".repeat(level * 3), esc_html(&title)));
            walk(pages, ids, page.id, level + 1, selected, out);
        }
    }
    let mut out = format!("<select name='page_for_privacy_policy' id='page_for_privacy_policy'>\n\t<option value=\"0\">{}</option>\n", ctx.tr("&mdash; Select &mdash;"));
    walk(&pages, &ids, 0, 0, selected_id, &mut out);
    out.push_str("</select>");
    out
}

/// `wp-admin/privacy-policy-guide.php`.
pub async fn guide(ctx: &Ctx<'_>) -> AdminResponse {
    if !ctx.can("manage_privacy_options") {
        return super::common::die_plain(ctx, "Sorry, you are not allowed to manage privacy options on this site.");
    }
    let baked = super::pages::load(ctx, "privacy-policy-guide");
    let screen = Screen {
        file: "privacy-policy-guide.php".into(),
        parent_file: "privacy-policy-guide.php".into(),
        submenu_file: None,
        title: ctx.tr("Privacy Policy Guide"),
        help: Some(baked.help),
        early_classes: vec!["privacy-settings ".into()],
        ..Default::default()
    };
    render(ctx, &screen, &baked.content, &Assets { scripts: vec!["privacy-tools".into()], ..Default::default() }).await
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    if !ctx.can("manage_privacy_options") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to manage privacy options on this site.")));
    }
    let assets = Assets { scripts: vec!["privacy-tools".into()], ..Default::default() };
    let screen = |title: &str, help: crate::chrome::Help| Screen { file: "options-privacy.php".into(), parent_file: "options-general.php".into(), submenu_file: Some("options-privacy.php".into()), title: tr(title), help: Some(help), early_classes: vec!["privacy-settings ".into()], ..Default::default() };
    if ctx.request.get("tab") == Some("policyguide") {
        return redirect(&ctx.admin_url("privacy-policy-guide.php"));
    }

    let request = ctx.request;
    let mut notices = String::new();
    let action = request.post_field("action").unwrap_or_default().to_owned();
    if !action.is_empty() {
        if !ctx.verify_nonce(&action) {
            return die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
        }
        let Some(writer) = ctx.env.writer else { return read_only(ctx) };
        let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
        if action == "set-privacy-page" {
            let previous = site.options.u64("wp_page_for_privacy_policy", 0);
            let chosen: u64 = request.post_field("page_for_privacy_policy").and_then(|v| v.parse().ok()).unwrap_or(0);
            let _ = writer.update_option("wp_page_for_privacy_policy", &chosen.to_string()).await;
            let (message, kind) = if chosen != 0 {
                (tr("Privacy Policy page updated successfully."), "success")
            } else if previous != 0 {
                (tr("Privacy Policy page removed."), "success")
            } else {
                (tr("No Privacy Policy page is currently set."), "info")
            };
            notices.push_str(&settings_error("page_for_privacy_policy", &message, kind));
        } else if action == "create-privacy-page" {
            let home = site.settings.home.trim_end_matches('/').to_owned();
            let data = PostData { post_title: Some(tr("Privacy Policy")), post_status: Some("draft".into()), post_type: Some("page".into()), post_content: Some(DEFAULT_CONTENT.replace("{{home}}", &home)), ..Default::default() };
            match content.insert_post(&data).await {
                Ok(id) => {
                    let _ = writer.update_option("wp_page_for_privacy_policy", &id.to_string()).await;
                    return redirect(&ctx.admin_url(&format!("post.php?post={id}&action=edit")));
                }
                Err(_) => notices.push_str(&settings_error("page_for_privacy_policy", &tr("Unable to create a Privacy Policy page."), "error")),
            }
        }
    }

    // The option is read again: the form may just have changed it.
    let page_id: u64 = match ctx.env.db.option("wp_page_for_privacy_policy").await {
        Ok(Some(v)) => v.trim().parse().unwrap_or(0),
        _ => 0,
    };
    let mut exists = false;
    let mut page_status = String::new();
    if page_id != 0 {
        match ctx.env.db.post(page_id).await.ok().flatten() {
            None => notices.push_str(&settings_error("page_for_privacy_policy", &tr("The currently selected Privacy Policy page does not exist. Please create or select a new page."), "error")),
            Some(page) if page.post_status == "trash" => notices.push_str(&settings_error("page_for_privacy_policy", &sprintf(&tr("The currently selected Privacy Policy page is in the Trash. Please create or select a new Privacy Policy page or <a href=\"%s\">restore the current page</a>."), &["edit.php?post_status=trash&post_type=page"]), "error")),
            Some(page) => {
                exists = true;
                page_status = page.post_status.clone();
            }
        }
    }
    let has_pages = !ctx.env.db.posts(&PostQuery { post_types: vec!["page".into()], statuses: vec!["publish".into(), "draft".into()], limit: Some(1), ..Default::default() }).await.unwrap_or_default().is_empty();

    let baked = super::pages::load(ctx, "options-privacy");
    let edit_part = if exists {
        let edit = esc_url(&ctx.admin_url(&format!("post.php?post={page_id}&action=edit")));
        let view = esc_url(&site.permalink(&ctx.env.db.post(page_id).await.ok().flatten().unwrap_or_default()).await.unwrap_or_default());
        let line = if page_status == "publish" {
            sprintf(&tr("<a href=\"%1$s\">Edit</a> or <a href=\"%2$s\">view</a> your Privacy Policy page content."), &[&edit, &view])
        } else {
            sprintf(&tr("<a href=\"%1$s\">Edit</a> or <a href=\"%2$s\">preview</a> your Privacy Policy page content."), &[&edit, &view])
        };
        format!("\t\t\t\t<strong>\n\t\t\t\t{line}\t\t\t\t</strong>\n\t\t\t")
    } else {
        String::new()
    };
    let referer = esc_url(&request.request_uri);
    let row = if has_pages {
        let label = if exists { tr("Change your Privacy Policy page") } else { tr("Select a Privacy Policy page") };
        ROW.replace("{{change_label}}", &label).replace("{{dropdown}}", &pages_dropdown(ctx, page_id).await).replace("{{nonce:set-privacy-page}}", &ctx.create_nonce("set-privacy-page")).replace("{{referer}}", &referer)
    } else {
        String::new()
    };
    let mut body = baked.content;
    body = body.replace("{{edit_part}}", &edit_part).replace("{{create_label}}", &if has_pages { tr("Create a new Privacy Policy page") } else { tr("There are no pages.") }).replace("{{page_row}}", &row).replace("{{referer}}", &referer);
    // The nonce of the create form is part of the baked content (translated by `load`); the row's own are above.
    while let Some(start) = body.find("{{nonce:") {
        let Some(end) = body[start..].find("}}") else { break };
        let action = body[start + 8..start + end].to_owned();
        body.replace_range(start..start + end + 2, &ctx.create_nonce(&action));
    }
    let _ = forbidden_heading;
    render(ctx, &screen("Privacy", baked.help), &format!("{notices}{body}"), &assets).await
}
