//! `wp-admin/post.php` and `post-new.php`: the classic edit form of posts and pages, and the actions that
//! come with it (save, trash, restore, delete).

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use rustpress_db::{Post, PostQuery};

use super::notices;
use super::posts::{TypeInfo, type_info};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::{Content, PostData, TermRef};

/// `esc_textarea()`: `htmlspecialchars( $text, ENT_QUOTES )`.
fn esc_textarea(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#039;")
}

pub fn esc_textarea_pub(text: &str) -> String {
    esc_textarea(text)
}

fn content_for<'a>(ctx: &'a Ctx<'a>) -> Option<Content<'a>> {
    Some(Content { site: ctx.site(), db: ctx.env.db, writer: ctx.env.writer?, user: &ctx.user, now: ctx.env.now })
}

fn sticky_ids(ctx: &Ctx<'_>) -> Vec<u64> {
    let raw = ctx.site().options.string("sticky_posts", "");
    match rustpress_phpser::unserialize(raw.as_bytes()) {
        Ok(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_i64()).map(|i| i as u64).collect(),
        _ => Vec::new(),
    }
}

/// `get_edit_post_link( $id, 'url' )`: raw `&`.
fn edit_link(ctx: &Ctx<'_>, id: u64) -> String {
    ctx.admin_url(&format!("post.php?post={id}&action=edit"))
}

fn labels(ctx: &Ctx<'_>, info: &TypeInfo) -> (String, String) {
    let tr = |s: &str| ctx.tr(s);
    match info.name.as_str() {
        "post" => (tr("Edit Post"), tr("Add Post")),
        "page" => (tr("Edit Page"), tr("Add Page")),
        _ => (tr("Edit Post"), tr("Add Post")),
    }
}

/// The wrapper of one meta box, as `do_meta_boxes()` prints it.
fn postbox(id: &str, title: &str, extra_class: &str, inner: &str) -> String {
    let handle = |kind: &str, label: &str, class: &str, aria: &str| {
        let suffix = match kind {
            "higher" => "handle-order-higher-description",
            "lower" => "handle-order-lower-description",
            _ => "handlediv",
        };
        let indicator = match kind {
            "higher" => "order-higher-indicator",
            "lower" => "order-lower-indicator",
            _ => "toggle-indicator",
        };
        format!(
            "<span class=\"wp-tooltip wp-is-tooltip\">\n\t\t\t\t<button aria-label=\"{label}\" type=\"button\" class=\"{class} wp-tooltip__toggle\"{aria} aria-describedby=\"{id}-title\">\n\t\t\t\t\t\t\t<span class=\"screen-reader-text\">{label}</span>\n\t\t\t\t\t\t\t<span class=\"{indicator}\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t</button>\n\t\t\t\t<span popover=\"hint\" id=\"{id}-{suffix}\" class=\"wp-tooltip__bubble\" role=\"tooltip\"><span id=\"{id}-{suffix}-text\" class=\"wp-tooltip__text\">{label}</span></span></span>"
        )
    };
    format!(
        "<div id=\"{id}\" class=\"postbox {extra_class}\"  role=\"region\" aria-label=\"{}\">\n<div class=\"postbox-header\"><h2 class=\"hndle\" id=\"{id}-title\">{title}</h2>\n<div class=\"handle-actions hide-if-no-js\">{}{}{}</div></div><div class=\"inside\">\n{inner}</div>\n</div>\n",
        esc_attr(title),
        handle("higher", "Move up", "handle-order-higher", ""),
        handle("lower", "Move down", "handle-order-lower", ""),
        handle("toggle", "Show or hide panel", "handlediv", " aria-expanded=\"true\"")
    )
}

/// `touch_time()` for the publish box.
fn touch_time(ctx: &Ctx<'_>, post: &Post, edit: bool) -> String {
    let now_local = ctx.site().date("Y-m-d H:i:s", ctx.env.now);
    let date = if edit && post.post_date != "0000-00-00 00:00:00" && !post.post_date.is_empty() { post.post_date.clone() } else { now_local };
    timestamp_fields(ctx, &date)
}

/// The date fields of `touch_time()` for the local date `date` (`Y-m-d H:i:s`).
pub fn timestamp_fields(ctx: &Ctx<'_>, date: &str) -> String {
    let site = ctx.site();
    let (date, cur) = (date.to_owned(), site.date("Y-m-d H:i:s", ctx.env.now));
    let part = |s: &str, range: std::ops::Range<usize>| s.get(range).unwrap_or("00").to_owned();
    let (jj, mm, aa, hh, mn, ss) = (part(&date, 8..10), part(&date, 5..7), part(&date, 0..4), part(&date, 11..13), part(&date, 14..16), part(&date, 17..19));
    let (cur_jj, cur_mm, cur_aa, cur_hh, cur_mn) = (part(&cur, 8..10), part(&cur, 5..7), part(&cur, 0..4), part(&cur, 11..13), part(&cur, 14..16));
    let locale = site.locale_data();
    let mut month = String::from("<label><span class=\"screen-reader-text\">");
    month.push_str(&ctx.tr("Month"));
    month.push_str("</span><select class=\"form-required\" id=\"mm\" name=\"mm\">\n");
    for index in 1..=12 {
        let number = format!("{index:02}");
        let abbrev = &locale.month_abbrev[index - 1];
        month.push_str(&format!(
            "\t\t\t<option value=\"{number}\" data-text=\"{}\" {}>{}</option>\n",
            esc_attr(abbrev),
            if number == mm { " selected='selected'" } else { "" },
            rustpress_core::i18n::sprintf(&ctx.tr("%1$s-%2$s"), &[&number, abbrev])
        ));
    }
    month.push_str("</select></label>");
    let field = |label: &str, id: &str, value: &str, size: u8| {
        format!("<label><span class=\"screen-reader-text\">{label}</span><input type=\"text\" id=\"{id}\" name=\"{id}\" value=\"{value}\" size=\"{size}\" maxlength=\"{size}\" autocomplete=\"off\" class=\"form-required\" inputmode=\"numeric\" /></label>")
    };
    let mut out = String::from("<div class=\"timestamp-wrap\">");
    out.push_str(&rustpress_core::i18n::sprintf(
        &ctx.tr("%1$s %2$s, %3$s at %4$s:%5$s"),
        &[&month, &field(&ctx.tr("Day"), "jj", &jj, 2), &field(&ctx.tr("Year"), "aa", &aa, 4), &field(&ctx.tr("Hour"), "hh", &hh, 2), &field(&ctx.tr("Minute"), "mn", &mn, 2)],
    ));
    out.push_str(&format!("</div><input type=\"hidden\" id=\"ss\" name=\"ss\" value=\"{ss}\" />\n"));
    for (name, value, current) in [("mm", &mm, &cur_mm), ("jj", &jj, &cur_jj), ("aa", &aa, &cur_aa), ("hh", &hh, &cur_hh), ("mn", &mn, &cur_mn)] {
        out.push_str(&format!("<input type=\"hidden\" id=\"hidden_{name}\" name=\"hidden_{name}\" value=\"{value}\" />\n<input type=\"hidden\" id=\"cur_{name}\" name=\"cur_{name}\" value=\"{current}\" />\n"));
    }
    out.push_str(&format!(
        "<p>\n<a href=\"#edit_timestamp\" class=\"save-timestamp hide-if-no-js button\">{}</a>\n<a href=\"#edit_timestamp\" class=\"cancel-timestamp hide-if-no-js button-cancel\">{}</a>\n</p>\n",
        ctx.tr("OK"),
        ctx.tr("Cancel")
    ));
    out
}

/// `post_submit_meta_box()`.
async fn submit_box(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, is_new: bool, sticky: bool) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let can_publish = ctx.can(if info.name == "page" { "publish_pages" } else { "publish_posts" });
    let status = post.post_status.as_str();
    let mut h = String::from("<div class=\"submitbox\" id=\"submitpost\">\n<div id=\"minor-publishing\">\n\t\t<div style=\"display:none;\">\n\t\t<p class=\"submit\"><input type=\"submit\" name=\"save\" id=\"save\" class=\"button\" value=\"");
    h.push_str(&esc_attr(&tr("Save")));
    h.push_str("\"  /></p>\t</div>\n\t<div id=\"minor-publishing-actions\">\n\t\t<div id=\"save-action\">\n\t\t\t\t\t");
    if !["publish", "future", "pending"].contains(&status) {
        let style = if status == "private" { "style=\"display:none\"" } else { "" };
        h.push_str(&format!("\n\t\t\t\t<input {style} type=\"submit\" name=\"save\" id=\"save-post\" value=\"{}\" class=\"button\" />\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t", esc_attr(&tr("Save Draft"))));
    } else if status == "pending" && can_publish {
        h.push_str(&format!("\n\t\t\t\t<input type=\"submit\" name=\"save\" id=\"save-post\" value=\"{}\" class=\"button\" />\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t", esc_attr(&tr("Save as Pending"))));
    }
    h.push_str("\t\t</div>\n");
    if site.is_post_type_viewable(&info.name) {
        let preview = if status == "publish" || status == "private" {
            add_query_arg(&site.permalink(post).await.unwrap_or_default(), "preview", "true")
        } else if info.name == "page" {
            site.home_url(&format!("?page_id={}&preview=true", post.id))
        } else {
            site.home_url(&format!("?p={}&preview=true", post.id))
        };
        let text = if status == "publish" { tr("Preview Changes") } else { site.trx("Preview", "verb") };
        h.push_str(&format!(
            "\t\t\t<div id=\"preview-action\">\n\t\t\t\t\t\t\t\t<a class=\"preview button\" href=\"{}\" target=\"wp-preview-{}\" id=\"post-preview\">{text}<span class=\"screen-reader-text\"> {}</span></a>\n\t\t\t\t<input type=\"hidden\" name=\"wp-preview\" id=\"wp-preview\" value=\"\" />\n\t\t\t</div>\n",
            esc_url(&preview),
            post.id,
            tr("(opens in a new tab)")
        ));
    }
    h.push_str("\t\t<div class=\"clear\"></div>\n\t</div>\n\t<div id=\"misc-publishing-actions\">\n\t\t<div class=\"misc-pub-section misc-pub-post-status\">\n\t\t\t");
    h.push_str(&tr("Status:"));
    let status_text = match status {
        "private" => tr("Privately Published"),
        "publish" => tr("Published"),
        "future" => tr("Scheduled"),
        "pending" => tr("Pending Review"),
        _ => tr("Draft"),
    };
    h.push_str(&format!("\t\t\t<span id=\"post-status-display\">\n\t\t\t\t{status_text}\t\t\t</span>\n\t\t\t"));
    if status == "publish" || status == "private" || can_publish {
        let style = if status == "private" { "style=\"display:none\"" } else { "" };
        h.push_str(&format!(
            "\t\t\t\t<a href=\"#post_status\" {style} class=\"edit-post-status hide-if-no-js\" role=\"button\"><span aria-hidden=\"true\">{}</span> <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></a>\n\t\t\t\t<div id=\"post-status-select\" class=\"hide-if-js\">\n\t\t\t\t\t<input type=\"hidden\" name=\"hidden_post_status\" id=\"hidden_post_status\" value=\"{}\" />\n\t\t\t\t\t<label for=\"post_status\" class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<select name=\"post_status\" id=\"post_status\">\n",
            tr("Edit"),
            tr("Edit status"),
            esc_attr(if status == "auto-draft" { "draft" } else { status }),
            tr("Set status")
        ));
        let sel = |s: &str| if status == s { " selected='selected'" } else { "" };
        match status {
            "publish" => h.push_str(&format!("\t\t\t\t\t\t\t\t\t\t\t\t\t<option{} value='publish'>{}</option>\n", sel("publish"), tr("Published"))),
            "private" => h.push_str(&format!("\t\t\t\t\t\t\t\t\t\t\t\t\t<option{} value='publish'>{}</option>\n", sel("private"), tr("Privately Published"))),
            "future" => h.push_str(&format!("\t\t\t\t\t\t\t\t\t\t\t\t\t<option{} value='future'>{}</option>\n", sel("future"), tr("Scheduled"))),
            _ => {}
        }
        h.push_str(&format!("\t\t\t\t\t\t\t\t\t\t\t<option{} value='pending'>{}</option>\n", sel("pending"), tr("Pending Review")));
        h.push_str(&format!("\t\t\t\t\t\t\t\t\t\t\t<option{} value='draft'>{}</option>\n", if status == "draft" || status == "auto-draft" { " selected='selected'" } else { "" }, tr("Draft")));
        h.push_str(&format!(
            "\t\t\t\t\t\t\t\t\t\t\t</select>\n\t\t\t\t\t<a href=\"#post_status\" class=\"save-post-status hide-if-no-js button\">{}</a>\n\t\t\t\t\t<a href=\"#post_status\" class=\"cancel-post-status hide-if-no-js button-cancel\">{}</a>\n\t\t\t\t</div>\n\t\t\t\t",
            tr("OK"),
            tr("Cancel")
        ));
    }
    h.push_str("\t\t</div>\n");

    // Visibility
    let (visibility, visibility_text, password) = if status == "private" {
        ("private", tr("Private"), String::new())
    } else if !post.post_password.is_empty() {
        ("password", tr("Password protected"), post.post_password.clone())
    } else if info.name == "post" && sticky {
        ("public", tr("Public, Sticky"), String::new())
    } else {
        ("public", tr("Public"), String::new())
    };
    h.push_str(&format!("\t\t<div class=\"misc-pub-section misc-pub-visibility\" id=\"visibility\">\n\t\t\t{}\t\t\t<span id=\"post-visibility-display\">\n\t\t\t\t{}\t\t\t</span>\n\t\t\t", tr("Visibility:"), esc_html(&visibility_text)));
    if can_publish {
        let checked = |v: &str| if visibility == v { " checked='checked'" } else { "" };
        h.push_str(&format!(
            "\t\t\t\t<a href=\"#visibility\" class=\"edit-visibility hide-if-no-js\" role=\"button\"><span aria-hidden=\"true\">{}</span> <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></a>\n\t\t\t\t<div id=\"post-visibility-select\" class=\"hide-if-js\">\n\t\t\t\t\t<input type=\"hidden\" name=\"hidden_post_password\" id=\"hidden-post-password\" value=\"{}\" />\n",
            tr("Edit"),
            tr("Edit visibility"),
            esc_attr(&password)
        ));
        if info.name == "post" {
            h.push_str(&format!("\t\t\t\t\t\t\t<input type=\"checkbox\" style=\"display:none\" name=\"hidden_post_sticky\" id=\"hidden-post-sticky\" value=\"sticky\" {} />\n", if sticky { "checked='checked'" } else { "" }));
        }
        h.push_str(&format!(
            "\t\t\t\t\t<input type=\"hidden\" name=\"hidden_post_visibility\" id=\"hidden-post-visibility\" value=\"{visibility}\" />\n\t\t\t\t\t<input type=\"radio\" name=\"visibility\" id=\"visibility-radio-public\" value=\"public\" {} /> <label for=\"visibility-radio-public\" class=\"selectit\">{}</label><br />\n",
            checked("public").trim_start(),
            tr("Public")
        ));
        if info.name == "post" && ctx.can("edit_others_posts") {
            h.push_str(&format!("\t\t\t\t\t\t<span id=\"sticky-span\"><input id=\"sticky\" name=\"sticky\" type=\"checkbox\" value=\"sticky\" {} /> <label for=\"sticky\" class=\"selectit\">{}</label><br /></span>\n", if sticky { "checked='checked'" } else { "" }, tr("Stick this post to the front page")));
        }
        h.push_str(&format!(
            "\t\t\t\t\t<input type=\"radio\" name=\"visibility\" id=\"visibility-radio-password\" value=\"password\" {} /> <label for=\"visibility-radio-password\" class=\"selectit\">{}</label><br />\n\t\t\t\t\t<span id=\"password-span\"><label for=\"post_password\">{}</label> <input type=\"text\" name=\"post_password\" id=\"post_password\" class=\"ltr\" value=\"{}\"  maxlength=\"255\" /><br /></span>\n\t\t\t\t\t<input type=\"radio\" name=\"visibility\" id=\"visibility-radio-private\" value=\"private\" {} /> <label for=\"visibility-radio-private\" class=\"selectit\">{}</label><br />\n\t\t\t\t\t<p>\n\t\t\t\t\t\t<a href=\"#visibility\" class=\"save-post-visibility hide-if-no-js button\">{}</a>\n\t\t\t\t\t\t<a href=\"#visibility\" class=\"cancel-post-visibility hide-if-no-js button-cancel\">{}</a>\n\t\t\t\t\t</p>\n\t\t\t\t</div>\n\t\t\t",
            checked("password").trim_start(),
            tr("Password protected"),
            tr("Password:"),
            esc_attr(&password),
            checked("private").trim_start(),
            tr("Private"),
            tr("OK"),
            tr("Cancel")
        ));
    }
    h.push_str("\t\t</div>\n");

    // Date
    let stamp_time = |local: &str| {
        let ts = site.timezone.local_timestamp(local).unwrap_or(ctx.env.now);
        rustpress_core::i18n::sprintf(&tr("%1$s at %2$s"), &[&site.date(&site.trx("M j, Y", "publish box date format"), ts), &site.date(&site.trx("H:i", "publish box time format"), ts)])
    };
    let now_local = site.date("Y-m-d H:i:s", ctx.env.now);
    let gmt_ts = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt).unwrap_or(0);
    let (stamp, date) = if !is_new && post.id != 0 {
        let stamp = if status == "future" {
            tr("Scheduled for: %s")
        } else if status == "publish" || status == "private" {
            tr("Published on: %s")
        } else if post.post_date_gmt == "0000-00-00 00:00:00" {
            tr("Publish <b>immediately</b>")
        } else if ctx.env.now < gmt_ts {
            tr("Schedule for: %s")
        } else {
            tr("Publish on: %s")
        };
        (stamp, stamp_time(&post.post_date))
    } else {
        (tr("Publish <b>immediately</b>"), stamp_time(&now_local))
    };
    if can_publish {
        h.push_str(&format!(
            "\t\t\t<div class=\"misc-pub-section curtime misc-pub-curtime\">\n\t\t\t\t<span id=\"timestamp\">\n\t\t\t\t\t{}\t\t\t\t</span>\n\t\t\t\t<a href=\"#edit_timestamp\" class=\"edit-timestamp hide-if-no-js\" role=\"button\">\n\t\t\t\t\t<span aria-hidden=\"true\">{}</span>\n\t\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</span>\n\t\t\t\t</a>\n\t\t\t\t<fieldset id=\"timestampdiv\" class=\"hide-if-js\">\n\t\t\t\t\t<legend class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</legend>\n\t\t\t\t\t{}\t\t\t\t</fieldset>\n\t\t\t</div>\n\t\t\t",
            rustpress_core::i18n::sprintf(&stamp, &[&format!("<b>{date}</b>")]),
            tr("Edit"),
            tr("Edit date and time"),
            tr("Date and time"),
            touch_time(ctx, post, !is_new)
        ));
    }
    h.push_str("\t</div>\n\t<div class=\"clear\"></div>\n</div>\n<div id=\"major-publishing-actions\">\n\t\t<div id=\"delete-action\">\n\t\t");
    if ctx.can(if info.name == "page" { "delete_pages" } else { "delete_posts" }) {
        let url = format!("{}&amp;_wpnonce={}", ctx.admin_url(&format!("post.php?post={}&action=trash", post.id)).replace('&', "&amp;"), ctx.create_nonce(&format!("trash-post_{}", post.id)));
        h.push_str(&format!("\t\t\t<a class=\"submitdelete deletion\" href=\"{url}\">{}</a>\n\t\t\t", tr("Move to Trash")));
    }
    h.push_str("\t\t</div>\n\t<div id=\"publishing-action\">\n\t\t<span class=\"spinner\"></span>\n\t\t");
    if !["publish", "future", "private"].contains(&status) || is_new {
        let scheduled = !post.post_date_gmt.is_empty() && post.post_date_gmt != "0000-00-00 00:00:00" && ctx.env.now < gmt_ts;
        let label = if can_publish { if scheduled { site.trx("Schedule", "post action/button label") } else { tr("Publish") } } else { tr("Submit for Review") };
        h.push_str(&format!(
            "\t\t\t\t\t<input name=\"original_publish\" type=\"hidden\" id=\"original_publish\" value=\"{}\" />\n\t\t\t\t\t<input type=\"submit\" name=\"publish\" id=\"publish\" class=\"button button-primary button-large\" value=\"{}\"  />\t\t\t\t",
            esc_attr(&label),
            esc_attr(&label)
        ));
    } else {
        h.push_str(&format!(
            "\t\t\t<input name=\"original_publish\" type=\"hidden\" id=\"original_publish\" value=\"{}\" />\n\t\t\t<input type=\"submit\" name=\"save\" id=\"publish\" class=\"button button-primary button-large\" value=\"{}\"  />\t\t\t\t",
            esc_attr(&tr("Update")),
            esc_attr(&tr("Update"))
        ));
    }
    h.push_str("</div>\n\t<div class=\"clear\"></div>\n</div>\n</div>\n\t");
    postbox("submitdiv", &tr("Publish"), "", &h)
}

/// `wp_terms_checklist()`: the category boxes.
async fn category_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let terms = db.terms("category").await.unwrap_or_default();
    let mut selected: Vec<u64> = if post.id != 0 { db.terms_for_post(post.id, Some("category")).await.unwrap_or_default().into_iter().map(|t| t.term_id).collect() } else { Vec::new() };
    if selected.is_empty() && post.post_status == "auto-draft" || selected.is_empty() && post.id == 0 {
        selected.push(ctx.site().options.u64("default_category", 1));
    }
    let mut popular: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.count > 0).collect();
    popular.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    popular.truncate(10);
    let popular_ids: Vec<u64> = popular.iter().map(|t| t.term_id).collect();
    let item = |t: &rustpress_db::Term, checked: bool| {
        let class = if popular_ids.contains(&t.term_id) { " class=\"popular-category\"" } else { "" };
        format!(
            "<li id='in-category-{id}-1'{class}><label class=\"selectit\"><input value=\"{id}\" type=\"checkbox\" name=\"post_category[]\" id=\"in-category-{id}-2\"{} /> {}</label>",
            if checked { " checked='checked'" } else { "" },
            esc_html(&t.name),
            id = t.term_id
        )
    };
    fn tree(terms: &[rustpress_db::Term], parent: u64, selected: &[u64], item: &dyn Fn(&rustpress_db::Term, bool) -> String, skip: &[u64], out: &mut String) {
        let mut children: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.parent == parent && !skip.contains(&t.term_id)).collect();
        children.sort_by_key(|t| t.name.to_lowercase());
        for term in children {
            out.push('\n');
            out.push_str(&item(term, selected.contains(&term.term_id)));
            let has_children = terms.iter().any(|t| t.parent == term.term_id && !skip.contains(&t.term_id));
            if has_children {
                out.push_str("<ul class='children'>\n");
                tree(terms, term.term_id, selected, item, skip, out);
                out.push_str("</ul>\n");
            }
            out.push_str("</li>\n");
        }
    }
    let mut list = String::new();
    // Checked terms first, flat.
    let checked_terms: Vec<&rustpress_db::Term> = terms.iter().filter(|t| selected.contains(&t.term_id)).collect();
    for term in &checked_terms {
        list.push('\n');
        list.push_str(&item(term, true));
        list.push_str("</li>\n");
    }
    let skip: Vec<u64> = checked_terms.iter().map(|t| t.term_id).collect();
    tree(&terms, 0, &selected, &item, &skip, &mut list);
    let mut pop = String::new();
    for term in &popular {
        pop.push_str(&format!(
            "\n\t\t<li id=\"popular-category-{id}\" class=\"popular-category\">\n\t\t\t<label class=\"selectit\">\n\t\t\t\t<input id=\"in-popular-category-{id}\" type=\"checkbox\"{} value=\"{id}\"  />\n\t\t\t\t{}\t\t\t</label>\n\t\t</li>",
            if selected.contains(&term.term_id) { " checked=\"checked\"" } else { " " },
            esc_html(&term.name),
            id = term.term_id
        ));
    }
    let mut parent_options = String::from("\t<option value='-1'>&mdash; Parent Category &mdash;</option>\n");
    fn parent_tree(terms: &[rustpress_db::Term], parent: u64, level: usize, out: &mut String) {
        let mut children: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.parent == parent).collect();
        children.sort_by_key(|t| t.name.to_lowercase());
        for term in children {
            out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\">{}{}</option>\n", term.term_id, "&nbsp;".repeat(level * 3), esc_html(&term.name)));
            parent_tree(terms, term.term_id, level + 1, out);
        }
    }
    parent_tree(&terms, 0, 0, &mut parent_options);
    let adder = if ctx.can("edit_posts") {
        format!(
            "\t\t\t\t<div id=\"category-adder\" class=\"wp-hidden-children\">\n\t\t\t\t<a id=\"category-add-toggle\" href=\"#category-add\" class=\"hide-if-no-js taxonomy-add-new\">\n\t\t\t\t\t+ {}\t\t\t\t</a>\n\t\t\t\t<p id=\"category-add\" class=\"category-add wp-hidden-child\">\n\t\t\t\t\t<label class=\"screen-reader-text\" for=\"newcategory\">{}</label>\n\t\t\t\t\t<input type=\"text\" name=\"newcategory\" id=\"newcategory\" class=\"form-required form-input-tip\" value=\"{}\" aria-required=\"true\" />\n\t\t\t\t\t<label class=\"screen-reader-text\" for=\"newcategory_parent\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<select  name='newcategory_parent' id='newcategory_parent' class='postform'>\n{parent_options}</select>\n\t\t\t\t\t<input type=\"button\" id=\"category-add-submit\" data-wp-lists=\"add:categorychecklist:category-add\" class=\"button category-add-submit\" value=\"{}\" />\n\t\t\t\t\t<input type=\"hidden\" id=\"_ajax_nonce-add-category\" name=\"_ajax_nonce-add-category\" value=\"{}\" />\t\t\t\t\t<span id=\"category-ajax-response\"></span>\n\t\t\t\t</p>\n\t\t\t</div>\n\t\t\t",
            tr("Add Category"),
            tr("Add Category"),
            tr("New Category Name"),
            tr("Parent Category:"),
            tr("Add Category"),
            ctx.create_nonce("add-category")
        )
    } else {
        String::new()
    };
    let inner = format!(
        "\t<div id=\"taxonomy-category\" class=\"categorydiv\">\n\t\t<ul id=\"category-tabs\" class=\"category-tabs\" role=\"tablist\">\n\t\t\t<li class=\"tabs\"><a href=\"#category-all\" role=\"tab\" aria-selected=\"true\" aria-controls=\"category-all\">{}</a></li>\n\t\t\t<li class=\"hide-if-no-js\"><a href=\"#category-pop\" role=\"tab\" aria-controls=\"category-pop\">{}</a></li>\n\t\t</ul>\n\t\t<div id=\"category-pop\" class=\"tabs-panel\" style=\"display: none;\" role=\"tabpanel\">\n\t\t\t<ul id=\"categorychecklist-pop\" class=\"categorychecklist form-no-clear\" >{pop}\n\t\t\t\t\t</ul>\n\t\t</div>\n\t\t<div id=\"category-all\" class=\"tabs-panel\" role=\"tabpanel\">\n\t\t\t<input type='hidden' name='post_category[]' value='0' />\t\t\t<ul id=\"categorychecklist\" data-wp-lists=\"list:category\" class=\"categorychecklist form-no-clear\">{list}\t\t\t</ul>\n\t\t</div>\n\t\t{adder}</div>\n\t",
        tr("All Categories"),
        tr("Most Used")
    );
    postbox("categorydiv", &tr("Categories"), "", &inner)
}

/// `post_tags_meta_box()`.
async fn tags_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let tags = if post.id != 0 { ctx.env.db.terms_for_post(post.id, Some("post_tag")).await.unwrap_or_default() } else { Vec::new() };
    let names = tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(&ctx.site().trx(",", "tag delimiter"));
    let inner = format!(
        "<div class=\"tagsdiv\" id=\"post_tag\">\n\t<div class=\"jaxtag\">\n\t<div class=\"nojs-tags hide-if-js\">\n\t\t<label for=\"tax-input-post_tag\">{}</label>\n\t\t<p><textarea name=\"tax_input[post_tag]\" rows=\"3\" cols=\"20\" class=\"the-tags\" id=\"tax-input-post_tag\"  aria-describedby=\"new-tag-post_tag-desc\">{}</textarea></p>\n\t</div>\n\t\t<div class=\"ajaxtag hide-if-no-js\">\n\t\t<label class=\"screen-reader-text\" for=\"new-tag-post_tag\">{}</label>\n\t\t<input data-wp-taxonomy=\"post_tag\" type=\"text\" id=\"new-tag-post_tag\" name=\"newtag[post_tag]\" class=\"newtag form-input-tip\" size=\"16\" autocomplete=\"off\" aria-describedby=\"new-tag-post_tag-desc\" value=\"\" />\n\t\t<input type=\"button\" class=\"button tagadd\" value=\"{}\" />\n\t</div>\n\t<p class=\"howto\" id=\"new-tag-post_tag-desc\">{}</p>\n\t\t</div>\n\t<ul class=\"tagchecklist\" role=\"list\"></ul>\n</div>\n\t<p class=\"hide-if-no-js\"><button type=\"button\" class=\"button-link tagcloud-link\" id=\"link-post_tag\" aria-expanded=\"false\">{}</button></p>\n\t",
        tr("Add or remove tags"),
        esc_textarea(&names),
        tr("Add Tag"),
        tr("Add"),
        tr("Separate tags with commas"),
        tr("Choose from the most used tags")
    );
    postbox("tagsdiv-post_tag", &tr("Tags"), "", &inner)
}

/// Pages: `page_attributes_meta_box()`.
async fn page_attributes_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let pages = ctx.env.db.posts(&PostQuery { post_types: vec!["page".into()], statuses: vec!["publish".into(), "draft".into(), "pending".into(), "private".into(), "future".into()], order_by: rustpress_db::OrderBy::MenuOrderTitle, order: rustpress_db::Order::Asc, ..Default::default() }).await.unwrap_or_default();
    fn tree(pages: &[Post], parent: u64, level: usize, current: u64, selected: u64, out: &mut String) {
        for page in pages.iter().filter(|p| p.post_parent == parent && p.id != current) {
            let pad = "&nbsp;".repeat(level * 3);
            out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\"{}>{pad}{}</option>\n", page.id, if page.id == selected { " selected=\"selected\"" } else { "" }, esc_html(&page.post_title)));
            tree(pages, page.id, level + 1, current, selected, out);
        }
    }
    let mut options = String::new();
    tree(&pages, 0, 0, post.id, post.post_parent, &mut options);
    let inner = format!(
        "<p class=\"post-attributes-label-wrapper parent-id-label-wrapper\"><label class=\"post-attributes-label\" for=\"parent_id\">{}</label></p>\n<select name='parent_id' id='parent_id'>\n\t<option value=''>{}</option>\n{options}</select>\n<p class=\"post-attributes-label-wrapper menu-order-label-wrapper\"><label class=\"post-attributes-label\" for=\"menu_order\">{}</label></p>\n<input name=\"menu_order\" type=\"text\" size=\"4\" id=\"menu_order\" value=\"{}\" />\n",
        tr("Parent"),
        tr("(no parent)"),
        tr("Order"),
        post.menu_order
    );
    postbox("pageparentdiv", &tr("Page Attributes"), "", &inner)
}

fn excerpt_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let inner = format!(
        "<label class=\"screen-reader-text\" for=\"excerpt\">\n\t{}</label><textarea rows=\"1\" cols=\"40\" name=\"excerpt\" id=\"excerpt\">{}</textarea>\n<p>\n\t{}</p>\n\t",
        tr("Excerpt"),
        esc_textarea(&post.post_excerpt),
        rustpress_core::i18n::sprintf(&tr("Excerpts are optional hand-crafted summaries of your content that can be used in your theme. <a href=\"%s\">Learn more about manual excerpts</a>."), &[&tr("https://wordpress.org/documentation/article/what-is-an-excerpt-classic-editor/")])
    );
    postbox("postexcerpt", &tr("Excerpt"), " hide-if-js", &inner)
}

fn discussion_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let open = |s: &str| if s == "open" { " checked='checked'" } else { "" };
    let inner = format!(
        "<input name=\"advanced_view\" type=\"hidden\" value=\"1\" />\n<p class=\"meta-options\">\n\t<label for=\"comment_status\" class=\"selectit\"><input name=\"comment_status\" type=\"checkbox\" id=\"comment_status\" value=\"open\" {} /> {}</label><br />\n\t<label for=\"ping_status\" class=\"selectit\"><input name=\"ping_status\" type=\"checkbox\" id=\"ping_status\" value=\"open\" {} />\n\t\t{}\t</label>\n\t</p>\n\t",
        open(&post.comment_status),
        tr("Allow comments"),
        open(&post.ping_status),
        rustpress_core::i18n::sprintf(&tr("Allow <a href=\"%s\">trackbacks and pingbacks</a>"), &["https://wordpress.org/documentation/article/introduction-to-blogging/#managing-comments"])
    );
    postbox("commentstatusdiv", &tr("Discussion"), " hide-if-js", &inner)
}

fn slug_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let inner = format!("<label class=\"screen-reader-text\" for=\"post_name\">\n\t{}</label><input name=\"post_name\" type=\"text\" class=\"large-text\" id=\"post_name\" value=\"{}\" />\n\t", ctx.tr("Slug"), esc_attr(&post.post_name));
    postbox("slugdiv", &ctx.tr("Slug"), " hide-if-js", &inner)
}

async fn author_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let users = ctx.env.db.all_users().await.unwrap_or_default();
    let mut options = String::new();
    for user in &users {
        options.push_str(&format!(
            "\t<option value='{}'{}>{}</option>\n",
            user.id,
            if user.id == post.post_author { " selected='selected'" } else { "" },
            esc_html(&format!("{} ({})", user.display_name, user.user_login))
        ));
    }
    let inner = format!("<label class=\"screen-reader-text\" for=\"post_author_override\">\n\t{}</label>\n\t<select name='post_author_override' id='post_author_override' class=''>\n{options}</select>", ctx.tr("Author"));
    postbox("authordiv", &ctx.tr("Author"), " hide-if-js", &inner)
}

/// The messages after saving (`$messages` of `edit-form-advanced.php`).
fn message_text(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, number: u32) -> Option<String> {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let page = info.name == "page";
    let permalink = site.home_url(&if page { format!("?page_id={}", post.id) } else { format!("?p={}", post.id) });
    let preview = format!("{permalink}&preview=true");
    let view = if site.is_post_type_viewable(&info.name) { format!(" <a href=\"{}\">{}</a>", esc_url(&permalink), if page { tr("View page") } else { tr("View post") }) } else { String::new() };
    let preview_link = |url: &str| if site.is_post_type_viewable(&info.name) { format!(" <a target=\"_blank\" href=\"{}\">{}</a>", esc_url(url), if page { tr("Preview page") } else { tr("Preview post") }) } else { String::new() };
    let scheduled = {
        let ts = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
        format!("<strong>{}</strong>", rustpress_core::i18n::sprintf(&tr("%1$s at %2$s"), &[&site.date(&site.trx("M j, Y", "publish box date format"), ts), &site.date(&site.trx("H:i", "publish box time format"), ts)]))
    };
    let (updated, published, saved, submitted, scheduled_text, draft_updated) = if page {
        (tr("Page updated."), tr("Page published."), tr("Page saved."), tr("Page submitted."), tr("Page scheduled for: %s."), tr("Page draft updated."))
    } else {
        (tr("Post updated."), tr("Post published."), tr("Post saved."), tr("Post submitted."), tr("Post scheduled for: %s."), tr("Post draft updated."))
    };
    Some(match number {
        1 => format!("{updated}{view}"),
        2 => tr("Custom field updated."),
        3 => tr("Custom field deleted."),
        4 => updated,
        6 => format!("{published}{view}"),
        7 => saved,
        8 => format!("{submitted}{}", preview_link(&preview)),
        9 => format!("{}{}", rustpress_core::i18n::sprintf(&scheduled_text, &[&scheduled]), preview_link(&permalink)),
        10 => format!("{draft_updated}{}", preview_link(&preview)),
        _ => return None,
    })
}

/// The edit form for a post (new posts are auto-drafts).
async fn form(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, is_new: bool) -> AdminResponse {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let (edit_title, add_title) = labels(ctx, info);
    let title = if is_new { add_title.clone() } else { edit_title };
    let sticky = sticky_ids(ctx).contains(&post.id);
    let post_title = if post.post_status == "auto-draft" { String::new() } else { post.post_title.clone() };
    let supports = |feature: &str| site.post_type_supports(&info.name, feature);

    // Lock
    let lock = format!("{}:{}", ctx.env.now, ctx.user.id);
    if let Some(writer) = ctx.env.writer {
        let _ = writer.update_post_meta(post.id, "_edit_lock", &lock).await;
    }

    let mut h = String::from("<div class=\"wrap\">\n<h1 class=\"wp-heading-inline\">\n");
    h.push_str(&esc_html(&title));
    h.push_str("</h1>\n");
    let new_file = if info.name == "post" { "post-new.php".to_owned() } else { format!("post-new.php?post_type={}", info.name) };
    if ctx.can(if info.name == "page" { "edit_pages" } else { "edit_posts" }) {
        h.push_str(&format!(" <a href=\"{}\" class=\"page-title-action\">{}</a>", esc_url(&ctx.admin_url(&new_file)), esc_html(&add_title)));
    }
    h.push_str("\n<hr class=\"wp-header-end\">\n");
    if let Some(number) = ctx.request.get("message").and_then(|m| m.parse::<u32>().ok())
        && let Some(text) = message_text(ctx, post, info, number)
    {
        h.push_str(&notices::notice(&text, "success", true, "message", &["updated"], true));
    }
    let lost = format!("<span class=\"spinner\"></span> {} <span class=\"hide-if-no-sessionstorage\">{}</span>", tr("<strong>Connection lost.</strong> Saving has been disabled until you are reconnected."), tr("This post is being backed up in your browser, just in case."));
    h.push_str(&notices::notice(&lost, "", false, "lost-connection-notice", &["error", "hidden"], true).replace("<div id=\"lost-connection-notice\" class=\"notice error hidden\">", "<div id=\"lost-connection-notice\" class=\"notice error hidden\">").trim_end_matches('\n').to_owned());
    h.push_str("<form name=\"post\" action=\"post.php\" method=\"post\" id=\"post\"\n>\n");
    h.push_str(&ctx.nonce_field(&format!("update-post_{}", post.id)));
    let referer = ctx.request.get("referredby").unwrap_or_default();
    h.push_str(&format!(
        "<input type=\"hidden\" id=\"user-id\" name=\"user_ID\" value=\"{}\" />\n<input type=\"hidden\" id=\"hiddenaction\" name=\"action\" value=\"editpost\" />\n<input type=\"hidden\" id=\"originalaction\" name=\"originalaction\" value=\"editpost\" />\n<input type=\"hidden\" id=\"post_author\" name=\"post_author\" value=\"{}\" />\n<input type=\"hidden\" id=\"post_type\" name=\"post_type\" value=\"{}\" />\n<input type=\"hidden\" id=\"original_post_status\" name=\"original_post_status\" value=\"{}\" />\n<input type=\"hidden\" id=\"referredby\" name=\"referredby\" value=\"{}\" />\n<input type=\"hidden\" id=\"active_post_lock\" value=\"{lock}\" />\n",
        ctx.user.id,
        post.post_author,
        esc_attr(&info.name),
        esc_attr(&post.post_status),
        esc_url(referer)
    ));
    if post.post_status != "draft" {
        h.push_str("\t<input type=\"hidden\" name=\"_wp_original_http_referer\" value=\"\" />");
    }
    if post.post_status == "auto-draft" {
        h.push_str("<input type='hidden' id='auto_draft' name='auto_draft' value='1' />");
    }
    h.push_str(&format!("<input type='hidden' id='post_ID' name='post_ID' value='{}' />", post.id));
    h.push_str(&format!(
        "<input type=\"hidden\" id=\"meta-box-order-nonce\" name=\"meta-box-order-nonce\" value=\"{}\" /><input type=\"hidden\" id=\"closedpostboxesnonce\" name=\"closedpostboxesnonce\" value=\"{}\" />\n",
        ctx.create_nonce("meta-box-order"),
        ctx.create_nonce("closedpostboxes")
    ));
    h.push_str("<div id=\"poststuff\">\n<div id=\"post-body\" class=\"metabox-holder columns-2\">\n<div id=\"post-body-content\">\n");
    if supports("title") {
        h.push_str(&format!(
            "<div id=\"titlediv\">\n<div id=\"titlewrap\">\n\t\t<label class=\"screen-reader-text\" id=\"title-prompt-text\" for=\"title\">{}</label>\n\t<input type=\"text\" name=\"post_title\" size=\"30\" value=\"{}\" id=\"title\" spellcheck=\"true\" autocomplete=\"off\" />\n",
            tr("Add title"),
            esc_attr(&post_title)
        ));
        if supports("editor") {
            h.push_str(&format!("\t\t\t<a href=\"#content\" class=\"button-secondary screen-reader-text skiplink\" onclick=\"if (tinymce) {{ tinymce.execCommand( 'mceFocus', false, 'content' ); }}\">{}</a>\n", tr("Skip to Editor")));
        }
        h.push_str("\t\t</div>\n\t<div class=\"inside\">\n");
        if site.is_post_type_viewable(&info.name) && post.post_status != "auto-draft" && !post.post_name.is_empty() && !site.options.string("permalink_structure", "").is_empty() {
            let link = site.permalink(post).await.unwrap_or_default();
            let name = &post.post_name;
            let shown = link.replacen(&format!("/{name}/"), &format!("/<span id=\"editable-post-name\">{}</span>/", esc_html(name)), 1);
            h.push_str(&format!(
                "\t\t<div id=\"edit-slug-box\" class=\"hide-if-no-js\">\n\t\t\t<strong>{}</strong>\n<span id=\"sample-permalink\"><a href=\"{}\">{shown}</a></span>\n&lrm;<span id=\"edit-slug-buttons\"><button type=\"button\" class=\"edit-slug button button-small hide-if-no-js\" aria-label=\"{}\">{}</button></span>\n<span id=\"editable-post-name-full\">{}</span>\n\t</div>\n",
                tr("Permalink:"),
                esc_url(&link),
                esc_attr(&tr("Edit permalink")),
                tr("Edit"),
                esc_html(name)
            ));
        }
        h.push_str(&format!("\t\t\t</div>\n\t<input type=\"hidden\" id=\"samplepermalinknonce\" name=\"samplepermalinknonce\" value=\"{}\" /></div><!-- /titlediv -->\n", ctx.create_nonce("samplepermalink")));
    }
    let mut editor_preinit = String::new();
    let mut editor_scripts: Vec<String> = Vec::new();
    let mut editor_footer = String::new();
    if supports("editor") {
        let rich_editing = db.user_meta(ctx.user.id, "rich_editing").await.ok().flatten();
        let editor = super::editor::classic_editor(ctx, &post.post_content, &info.name, &post.post_status, rich_editing.as_deref());
        h.push_str(&editor.html);
        editor_scripts = editor.scripts.clone();
        editor_footer = editor.init.clone();
        editor_preinit = editor.preinit.clone();
        let last_edit = if post.post_status == "auto-draft" {
            String::new()
        } else {
            let ts = site.timezone.local_timestamp(&post.post_modified).unwrap_or(0);
            let (date, time) = (site.date(&tr("F j, Y"), ts), site.date(&tr("g:i a"), ts));
            let last_user = match db.post_meta_value(post.id, "_edit_last").await.ok().flatten().and_then(|v| v.parse::<u64>().ok()) {
                Some(id) => db.users_by_ids(&[id]).await.unwrap_or_default().into_iter().next(),
                None => None,
            };
            let text = match last_user {
                Some(user) => rustpress_core::i18n::sprintf(&tr("Last edited by %1$s on %2$s at %3$s"), &[&esc_html(&user.display_name), &date, &time]),
                None => rustpress_core::i18n::sprintf(&tr("Last edited on %1$s at %2$s"), &[&date, &time]),
            };
            format!("<span id=\"last-edit\">{text}</span>")
        };
        h.push_str(&format!(
            "<table id=\"post-status-info\" role=\"presentation\"><tbody><tr>\n\t<td id=\"wp-word-count\" class=\"hide-if-no-js\">\n\t{}\t</td>\n\t<td class=\"autosave-info\">\n\t<span class=\"autosave-message\">&nbsp;</span>\n\t{last_edit}\t</td>\n\t<td id=\"content-resize-handle\" class=\"hide-if-no-js\"><br /></td>\n</tr></tbody></table>\n</div>\n",
            rustpress_core::i18n::sprintf(&tr("Word count: %s"), &["<span class=\"word-count\">0</span>"])
        ));
    }
    h.push_str("\t</div><!-- /post-body-content -->\n<div id=\"postbox-container-1\" class=\"postbox-container\">\n<div id=\"side-sortables\" class=\"meta-box-sortables\">");
    h.push_str(&submit_box(ctx, post, info, is_new, sticky).await);
    if info.name == "post" {
        h.push_str(&category_box(ctx, post).await);
        h.push_str(&tags_box(ctx, post).await);
    }
    if info.hierarchical {
        h.push_str(&page_attributes_box(ctx, post).await);
    }
    h.push_str("</div></div>\n<div id=\"postbox-container-2\" class=\"postbox-container\">\n<div id=\"normal-sortables\" class=\"meta-box-sortables\">");
    if supports("excerpt") {
        h.push_str(&excerpt_box(ctx, post));
    }
    if supports("comments") {
        h.push_str(&discussion_box(ctx, post));
    }
    if supports("slug") || true {
        h.push_str(&slug_box(ctx, post));
    }
    if supports("author") && ctx.can("edit_others_posts") {
        h.push_str(&author_box(ctx, post).await);
    }
    h.push_str("</div><div id=\"advanced-sortables\" class=\"meta-box-sortables\"></div></div>\n</div><!-- /post-body -->\n<br class=\"clear\" />\n</div><!-- /poststuff -->\n</form>\n</div>\n\n");
    if post_title.is_empty() {
        h.push_str("<script>\ntry{document.post.title.focus();}catch(e){}\n</script>\n");
    }
    h.push_str(&editor_preinit);

    let (parent_file, submenu_file) = if info.name == "post" { ("edit.php".to_owned(), "edit.php".to_owned()) } else { (format!("edit.php?post_type={}", info.name), format!("edit.php?post_type={}", info.name)) };
    let file = if is_new { "post-new.php" } else { "post.php" };
    let help = Help {
        tabs: vec![HelpTab { id: "title-post-editor".into(), title: tr("Title and Post Editor"), content: format!("<p>{}</p>", tr("<strong>Title</strong> &mdash; Enter a title for your post. After you enter a title, you&#8217;ll see the permalink below, which you can edit.")) }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    };
    let view_item = if !is_new && site.is_post_type_viewable(&info.name) && post.post_status != "auto-draft" {
        Some((if info.name == "page" { tr("View Page") } else { tr("View Post") }, site.permalink(post).await.unwrap_or_default()))
    } else {
        None
    };
    let screen = Screen { file: file.into(), parent_file, submenu_file: Some(submenu_file), post_type: info.name.clone(), title: title.clone(), help: Some(help), view_item, ..Default::default() };
    let mut scripts: Vec<String> = vec!["post".into(), "word-count".into(), "autosave".into(), "wp-lists".into(), "suggest".into(), "postbox".into(), "tags-box".into()];
    scripts.extend(editor_scripts);
    let assets = Assets { scripts, footer: editor_footer, ..Default::default() };
    render(ctx, &screen, &h, &assets).await
}

/// `post-new.php`.
pub async fn post_new(ctx: &Ctx<'_>) -> AdminResponse {
    let post_type = ctx.request.get("post_type").filter(|t| !t.is_empty()).unwrap_or("post").to_owned();
    let Some(info) = type_info(ctx, &post_type) else {
        return die(ctx.env, 400, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr("Invalid post type.")));
    };
    if !ctx.can(if info.name == "page" { "edit_pages" } else { "edit_posts" }) {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", ctx.tr("You need a higher level of permission."), ctx.tr("Sorry, you are not allowed to create posts as this user.")));
    }
    let Some(content) = content_for(ctx) else {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>");
    };
    let data = PostData { post_title: Some(if ctx.site().post_type_supports(&info.name, "title") { ctx.tr("Auto Draft") } else { String::new() }), post_type: Some(info.name.clone()), post_status: Some("auto-draft".into()), post_author: Some(ctx.user.id), ..Default::default() };
    let id = match content.insert_post(&PostData { post_content: Some(String::new()), ..data }).await {
        Ok(id) => id,
        Err(e) => return die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", esc_html(&e.to_string()))),
    };
    let Ok(Some(post)) = ctx.env.db.post(id).await else {
        return die(ctx.env, 500, "WordPress &rsaquo; Error", "<p>The new post could not be read.</p>");
    };
    if super::block_editor::uses_block_editor(ctx, &post) {
        return super::block_editor::render_editor(ctx, &post, &info).await;
    }
    form(ctx, &post, &info, true).await
}

/// `post.php`.
pub async fn post_php(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let action = request.request("action").unwrap_or_default().to_owned();
    let post_id: u64 = request.get("post").or(request.post_field("post_ID")).and_then(|p| p.parse().ok()).unwrap_or(0);
    let post = if post_id != 0 { ctx.env.db.post(post_id).await.ok().flatten() } else { None };
    let sendback = {
        let referer = request.request("_wp_http_referer").filter(|r| !r.is_empty());
        match referer {
            Some(r) if !r.contains("post.php") && !r.contains("post-new.php") => {
                let mut url = r.to_owned();
                for key in ["trashed", "untrashed", "deleted", "ids"] {
                    url = remove_query_arg(&url, key);
                }
                url
            }
            _ => {
                let mut url = ctx.admin_url("edit.php");
                if let Some(post) = &post {
                    url = if post.post_type == "attachment" { ctx.admin_url("upload.php") } else { add_query_arg(&url, "post_type", &post.post_type) };
                }
                url
            }
        }
    };
    let can_delete = |post: &Post| ctx.can(if post.post_type == "page" { "delete_pages" } else { "delete_posts" }) && (post.post_author == ctx.user.id || ctx.can("delete_others_posts") || ctx.can("delete_others_pages"));
    let ays = || die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
    match action.as_str() {
        "post-quickdraft-save" => super::dashboard::quickdraft_save(ctx).await,
        "edit" => {
            let Some(post) = post else {
                return die(ctx.env, 404, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("You attempted to edit an item that does not exist. Perhaps it was deleted?")));
            };
            if post.post_type == "attachment" {
                if !(ctx.can("edit_posts") && (post.post_author == ctx.user.id || ctx.can("edit_others_posts"))) {
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit this item.")));
                }
                return super::attachment_edit::form(ctx, &post).await;
            }
            let Some(info) = type_info(ctx, &post.post_type) else {
                return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit posts in this post type.")));
            };
            let may_edit = ctx.can(if post.post_type == "page" { "edit_pages" } else { "edit_posts" }) && (post.post_author == ctx.user.id || ctx.can("edit_others_posts") || ctx.can("edit_others_pages"));
            if !may_edit {
                return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit this item.")));
            }
            if post.post_status == "trash" {
                return die(ctx.env, 409, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("You cannot edit this item because it is in the Trash. Please restore it and try again.")));
            }
            if super::block_editor::uses_block_editor(ctx, &post) {
                return super::block_editor::render_editor(ctx, &post, &info).await;
            }
            form(ctx, &post, &info, false).await
        }
        "editpost" => {
            if !request.post || !ctx.verify_nonce(&format!("update-post_{post_id}")) {
                return ays();
            }
            match post.as_ref().filter(|p| p.post_type == "attachment") {
                Some(attachment) => super::attachment_edit::save(ctx, attachment).await,
                None => edit_post(ctx, post_id).await,
            }
        }
        "trash" | "untrash" | "delete" => {
            if !ctx.verify_nonce(&format!("{action}-post_{post_id}")) {
                return ays();
            }
            let Some(post) = post else {
                let message = match action.as_str() {
                    "trash" => tr("The item you are trying to move to the Trash no longer exists."),
                    "untrash" => tr("The item you are trying to restore from the Trash no longer exists."),
                    _ => tr("This item has already been deleted."),
                };
                return die(ctx.env, 410, "WordPress &rsaquo; Error", &format!("<p>{message}</p>"));
            };
            if !can_delete(&post) {
                let message = match action.as_str() {
                    "trash" => tr("Sorry, you are not allowed to move this item to the Trash."),
                    "untrash" => tr("Sorry, you are not allowed to restore this item from the Trash."),
                    _ => tr("Sorry, you are not allowed to delete this item."),
                };
                return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{message}</p>"));
            }
            let Some(content) = content_for(ctx) else {
                return die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>");
            };
            match action.as_str() {
                "trash" => {
                    if !content.trash_post(post_id).await.unwrap_or(false) {
                        return die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Error in moving the item to Trash.")));
                    }
                    redirect(&add_query_arg(&add_query_arg(&sendback, "trashed", "1"), "ids", &post_id.to_string()))
                }
                "untrash" => {
                    if !content.untrash_post(post_id).await.unwrap_or(false) {
                        return die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Error in restoring the item from Trash.")));
                    }
                    redirect(&add_query_arg(&add_query_arg(&sendback, "untrashed", "1"), "ids", &post_id.to_string()))
                }
                _ if post.post_type == "attachment" => {
                    if !content.delete_attachment(post_id, super::ajax_media::uploads_dir(ctx.env)).await.unwrap_or(false) {
                        return die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Error in deleting the item.")));
                    }
                    redirect(&add_query_arg(&sendback, "deleted", "1"))
                }
                _ => {
                    if !content.delete_post(post_id, true).await.unwrap_or(false) {
                        return die(ctx.env, 500, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Error in deleting the item.")));
                    }
                    redirect(&add_query_arg(&sendback, "deleted", "1"))
                }
            }
        }
        _ => redirect(&ctx.admin_url("edit.php")),
    }
}

/// `edit_post()` and `redirect_post()`.
async fn edit_post(ctx: &Ctx<'_>, post_id: u64) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let Some(post) = ctx.env.db.post(post_id).await.ok().flatten() else {
        return die(ctx.env, 404, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Invalid post ID.")));
    };
    let Some(info) = type_info(ctx, &post.post_type) else {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>Invalid post type.</p>");
    };
    let page = info.name == "page";
    let may_edit = ctx.can(if page { "edit_pages" } else { "edit_posts" }) && (post.post_author == ctx.user.id || ctx.can("edit_others_posts") || ctx.can("edit_others_pages"));
    if !may_edit {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", if page { tr("Sorry, you are not allowed to edit this page.") } else { tr("Sorry, you are not allowed to edit this post.") }));
    }
    let Some(content) = content_for(ctx) else {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>");
    };
    let field = |name: &str| request.post_field(name);
    let has = |name: &str| field(name).is_some_and(|v| !v.is_empty());
    let can_publish = ctx.can(if page { "publish_pages" } else { "publish_posts" });

    // Visibility
    let mut status = field("post_status").filter(|s| !s.is_empty()).map(|s| s.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect::<String>());
    let mut password = field("post_password").map(str::to_owned);
    let mut sticky = has("sticky");
    match field("visibility") {
        Some("public") => password = Some(String::new()),
        Some("password") => sticky = false,
        Some("private") => {
            status = Some("private".into());
            password = Some(String::new());
            sticky = false;
        }
        _ => {}
    }
    if status.as_deref() == Some("auto-draft") {
        status = Some("draft".into());
    }
    if status.as_deref() == Some("inherit") {
        status = None;
    }
    if has("saveasdraft") {
        status = Some("draft".into());
    }
    if has("saveasprivate") {
        status = Some("private".into());
    }
    if has("publish") && status.as_deref() != Some("private") {
        status = Some("publish".into());
    }
    if has("advanced") {
        status = Some("draft".into());
    }
    if has("pending") {
        status = Some("pending".into());
    }
    let previous = post.post_status.clone();
    if status.as_deref() == Some("private") && !can_publish {
        status = Some(previous.clone());
    }
    if matches!(status.as_deref(), Some("publish" | "future")) && !can_publish && (!matches!(previous.as_str(), "publish" | "future")) {
        status = Some("pending".into());
    }
    let status = status.unwrap_or_else(|| if previous == "auto-draft" { "draft".into() } else { previous.clone() });
    if !can_publish {
        password = None;
    }

    // Date
    let mut edit_date = false;
    let mut post_date = None;
    let mut post_date_gmt = None;
    for unit in ["aa", "mm", "jj", "hh", "mn"] {
        let hidden = field(&format!("hidden_{unit}")).unwrap_or_default();
        if !hidden.is_empty() && Some(hidden) != field(unit) {
            edit_date = true;
        }
    }
    if edit_date || has("edit_date") {
        let num = |name: &str| field(name).and_then(|v| v.trim().parse::<i64>().ok()).unwrap_or(0);
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
            return die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Invalid date.")));
        }
        if post.post_date != date {
            post_date_gmt = ctx.site().timezone.local_timestamp(&date).map(|ts| rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", ts));
            post_date = Some(date);
        } else {
            edit_date = false;
        }
    }

    let categories: Option<Vec<u64>> = {
        let list: Vec<u64> = request.form.iter().filter(|(k, _)| k == "post_category[]").filter_map(|(_, v)| v.parse().ok()).filter(|id| *id != 0).collect();
        if request.form.iter().any(|(k, _)| k == "post_category[]") && ctx.can("manage_categories") || ctx.can("edit_posts") && request.form.iter().any(|(k, _)| k == "post_category[]") { Some(list) } else { None }
    };
    let tags: Option<Vec<String>> = field("tax_input[post_tag]").map(|raw| raw.split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect());
    let parent = field("parent_id").map(|p| p.parse::<u64>().unwrap_or(0));
    let author = field("post_author_override").and_then(|a| a.parse::<u64>().ok()).filter(|_| ctx.can("edit_others_posts"));
    let data = PostData {
        id: Some(post_id),
        post_author: author,
        post_content: field("content").map(str::to_owned),
        post_title: field("post_title").map(str::to_owned),
        post_excerpt: field("excerpt").map(str::to_owned),
        post_status: Some(status),
        comment_status: Some(if field("comment_status").is_some() { "open".into() } else { "closed".into() }),
        ping_status: Some(if field("ping_status").is_some() { "open".into() } else { "closed".into() }),
        post_password: password,
        post_name: field("post_name").map(str::to_owned),
        post_parent: parent,
        menu_order: field("menu_order").map(|m| m.parse().unwrap_or(0)),
        post_date,
        post_date_gmt,
        post_category: categories,
        tags_input: tags,
        edit_date,
        ..Default::default()
    };
    if let Err(e) = content.insert_post(&data).await {
        return die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", esc_html(&e.to_string())));
    }
    if let Some(writer) = ctx.env.writer {
        let _ = writer.update_post_meta(post_id, "_edit_last", &ctx.user.id.to_string()).await;
        let _ = writer.update_post_meta(post_id, "_edit_lock", &format!("{}:{}", ctx.env.now, ctx.user.id)).await;
        // Sticky posts
        if info.name == "post" && ctx.can("edit_others_posts") && can_publish {
            let mut ids = sticky_ids(ctx);
            let before = ids.clone();
            ids.retain(|i| *i != post_id);
            if sticky {
                ids.insert(0, post_id);
            }
            if ids != before {
                let value = rustpress_phpser::Value::Array(ids.iter().enumerate().map(|(i, id)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Int(*id as i64))).collect());
                let _ = writer.update_option("sticky_posts", &String::from_utf8_lossy(&rustpress_phpser::serialize(&value))).await;
            }
        }
    }
    let _ = (TermRef::Id(0), &parent);

    // `redirect_post()`
    let new_status = ctx.env.db.post(post_id).await.ok().flatten().map(|p| p.post_status).unwrap_or_default();
    let message = if has("save") || has("publish") {
        match new_status.as_str() {
            "pending" => 8,
            "future" => 9,
            "draft" => 10,
            _ => {
                if has("publish") {
                    6
                } else {
                    1
                }
            }
        }
    } else {
        4
    };
    redirect(&add_query_arg(&edit_link(ctx, post_id), "message", &message.to_string()))
}
