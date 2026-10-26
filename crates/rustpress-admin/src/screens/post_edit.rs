//! `wp-admin/post.php` and `post-new.php`: the classic edit form of posts and pages, and the actions that
//! come with it (save, trash, restore, delete).

use rustpress_core::OrLog;
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
    out.push_str(&format!("</div><input type=\"hidden\" id=\"ss\" name=\"ss\" value=\"{ss}\" />\n\n"));
    for (name, value, current) in [("mm", &mm, &cur_mm), ("jj", &jj, &cur_jj), ("aa", &aa, &cur_aa), ("hh", &hh, &cur_hh), ("mn", &mn, &cur_mn)] {
        out.push_str(&format!("<input type=\"hidden\" id=\"hidden_{name}\" name=\"hidden_{name}\" value=\"{value}\" />\n<input type=\"hidden\" id=\"cur_{name}\" name=\"cur_{name}\" value=\"{current}\" />\n"));
    }
    out.push_str(&format!(
        "\n<p>\n<a href=\"#edit_timestamp\" class=\"save-timestamp hide-if-no-js button\">{}</a>\n<a href=\"#edit_timestamp\" class=\"cancel-timestamp hide-if-no-js button-cancel\">{}</a>\n</p>\n\t",
        ctx.tr("OK"),
        ctx.tr("Cancel")
    ));
    out
}

/// `post_submit_meta_box()`: `revisions` is `(count, latest id)` when the revisions box is shown.
async fn submit_box(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, is_new: bool, sticky: bool, revisions: Option<(u64, u64)>) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let can_publish = ctx.can(if info.name == "page" { "publish_pages" } else { "publish_posts" });
    let status = post.post_status.as_str();
    let mut h = format!(
        "<div class=\"submitbox\" id=\"submitpost\">\n\n<div id=\"minor-publishing\">\n\n\t\t<div style=\"display:none;\">\n\t\t{}\t</div>\n\n\t<div id=\"minor-publishing-actions\">\n\t\t<div id=\"save-action\">\n\t\t\t",
        super::meta_boxes::submit_button(&tr("Save"), "", "save", true, None, &[])
    );
    if !["publish", "future", "pending"].contains(&status) {
        let style = if status == "private" { "style=\"display:none\"" } else { "" };
        h.push_str(&format!("\t\t\t\t<input {style} type=\"submit\" name=\"save\" id=\"save-post\" value=\"{}\" class=\"button\" />\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t", esc_attr(&tr("Save Draft"))));
    } else if status == "pending" && can_publish {
        h.push_str(&format!("\t\t\t\t<input type=\"submit\" name=\"save\" id=\"save-post\" value=\"{}\" class=\"button\" />\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t", esc_attr(&tr("Save as Pending"))));
    }
    h.push_str("\t\t</div>\n\n\t\t");
    if site.is_post_type_viewable(&info.name) {
        // `get_preview_post_link()`
        let preview = add_query_arg(&site.permalink(post).await.unwrap_or_default(), "preview", "true");
        let text = if status == "publish" { tr("Preview Changes") } else { site.trx("Preview", "verb") };
        h.push_str(&format!(
            "\t\t\t<div id=\"preview-action\">\n\t\t\t\t\t\t\t\t<a class=\"preview button\" href=\"{}\" target=\"wp-preview-{}\" id=\"post-preview\">{text}<span class=\"screen-reader-text\"> {}</span></a>\n\t\t\t\t<input type=\"hidden\" name=\"wp-preview\" id=\"wp-preview\" value=\"\" />\n\t\t\t</div>\n\t\t\t",
            esc_url(&preview),
            post.id,
            tr("(opens in a new tab)")
        ));
    }
    let status_text = match status {
        "private" => tr("Privately Published"),
        "publish" => tr("Published"),
        "future" => tr("Scheduled"),
        "pending" => tr("Pending Review"),
        "draft" | "auto-draft" => tr("Draft"),
        _ => String::new(),
    };
    h.push_str(&format!(
        "\t\t<div class=\"clear\"></div>\n\t</div>\n\n\t<div id=\"misc-publishing-actions\">\n\t\t<div class=\"misc-pub-section misc-pub-post-status\">\n\t\t\t{}\t\t\t<span id=\"post-status-display\">\n\t\t\t\t{status_text}\t\t\t</span>\n\n\t\t\t",
        tr("Status:")
    ));
    if status == "publish" || status == "private" || can_publish {
        let style = if status == "private" { "style=\"display:none\"" } else { "" };
        h.push_str(&format!(
            "\t\t\t\t<a href=\"#post_status\" {style} class=\"edit-post-status hide-if-no-js\" role=\"button\"><span aria-hidden=\"true\">{}</span> <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></a>\n\n\t\t\t\t<div id=\"post-status-select\" class=\"hide-if-js\">\n\t\t\t\t\t<input type=\"hidden\" name=\"hidden_post_status\" id=\"hidden_post_status\" value=\"{}\" />\n\t\t\t\t\t<label for=\"post_status\" class=\"screen-reader-text\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<select name=\"post_status\" id=\"post_status\">\n\t\t\t\t\t\t",
            tr("Edit"),
            tr("Edit status"),
            esc_attr(if status == "auto-draft" { "draft" } else { status }),
            tr("Set status")
        ));
        let sel = |s: &str| if status == s { " selected='selected'" } else { "" };
        match status {
            "publish" => h.push_str(&format!("\t\t\t\t\t\t\t<option{} value='publish'>{}</option>\n\t\t\t\t\t\t", sel("publish"), tr("Published"))),
            "private" => h.push_str(&format!("\t\t\t\t\t\t\t<option{} value='publish'>{}</option>\n\t\t\t\t\t\t", sel("private"), tr("Privately Published"))),
            "future" => h.push_str(&format!("\t\t\t\t\t\t\t<option{} value='future'>{}</option>\n\t\t\t\t\t\t", sel("future"), tr("Scheduled"))),
            _ => {}
        }
        h.push_str(&format!("\t\t\t\t\t\t\t<option{} value='pending'>{}</option>\n\t\t\t\t\t\t", sel("pending"), tr("Pending Review")));
        let draft = if status == "auto-draft" { sel("auto-draft") } else { sel("draft") };
        h.push_str(&format!("\t\t\t\t\t\t\t<option{draft} value='draft'>{}</option>\n\t\t\t\t\t\t", tr("Draft")));
        h.push_str(&format!(
            "\t\t\t\t\t</select>\n\t\t\t\t\t<a href=\"#post_status\" class=\"save-post-status hide-if-no-js button\">{}</a>\n\t\t\t\t\t<a href=\"#post_status\" class=\"cancel-post-status hide-if-no-js button-cancel\">{}</a>\n\t\t\t\t</div>\n\t\t\t\t",
            tr("OK"),
            tr("Cancel")
        ));
    }
    h.push_str("\t\t</div>\n\n");

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
    h.push_str(&format!(
        "\t\t<div class=\"misc-pub-section misc-pub-visibility\" id=\"visibility\">\n\t\t\t{}\t\t\t<span id=\"post-visibility-display\">\n\t\t\t\t{}\t\t\t</span>\n\n\t\t\t",
        tr("Visibility:"),
        esc_html(&visibility_text)
    ));
    if can_publish {
        let checked = |v: &str| if visibility == v { " checked='checked'" } else { "" };
        h.push_str(&format!(
            "\t\t\t\t<a href=\"#visibility\" class=\"edit-visibility hide-if-no-js\" role=\"button\"><span aria-hidden=\"true\">{}</span> <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></a>\n\n\t\t\t\t<div id=\"post-visibility-select\" class=\"hide-if-js\">\n\t\t\t\t\t<input type=\"hidden\" name=\"hidden_post_password\" id=\"hidden-post-password\" value=\"{}\" />\n\t\t\t\t\t",
            tr("Edit"),
            tr("Edit visibility"),
            esc_attr(&password)
        ));
        if info.name == "post" {
            h.push_str(&format!("\t\t\t\t\t\t<input type=\"checkbox\" style=\"display:none\" name=\"hidden_post_sticky\" id=\"hidden-post-sticky\" value=\"sticky\" {} />\n\t\t\t\t\t", if sticky { " checked='checked'" } else { "" }));
        }
        h.push_str(&format!(
            "\n\t\t\t\t\t<input type=\"hidden\" name=\"hidden_post_visibility\" id=\"hidden-post-visibility\" value=\"{}\" />\n\t\t\t\t\t<input type=\"radio\" name=\"visibility\" id=\"visibility-radio-public\" value=\"public\" {} /> <label for=\"visibility-radio-public\" class=\"selectit\">{}</label><br />\n\n\t\t\t\t\t",
            esc_attr(visibility),
            checked("public"),
            tr("Public")
        ));
        if info.name == "post" && ctx.can("edit_others_posts") {
            h.push_str(&format!(
                "\t\t\t\t\t\t<span id=\"sticky-span\"><input id=\"sticky\" name=\"sticky\" type=\"checkbox\" value=\"sticky\" {} /> <label for=\"sticky\" class=\"selectit\">{}</label><br /></span>\n\t\t\t\t\t",
                if sticky { " checked='checked'" } else { "" },
                tr("Stick this post to the front page")
            ));
        }
        h.push_str(&format!(
            "\n\t\t\t\t\t<input type=\"radio\" name=\"visibility\" id=\"visibility-radio-password\" value=\"password\" {} /> <label for=\"visibility-radio-password\" class=\"selectit\">{}</label><br />\n\t\t\t\t\t<span id=\"password-span\"><label for=\"post_password\">{}</label> <input type=\"text\" name=\"post_password\" id=\"post_password\" class=\"ltr\" value=\"{}\"  maxlength=\"255\" /><br /></span>\n\n\t\t\t\t\t<input type=\"radio\" name=\"visibility\" id=\"visibility-radio-private\" value=\"private\" {} /> <label for=\"visibility-radio-private\" class=\"selectit\">{}</label><br />\n\n\t\t\t\t\t<p>\n\t\t\t\t\t\t<a href=\"#visibility\" class=\"save-post-visibility hide-if-no-js button\">{}</a>\n\t\t\t\t\t\t<a href=\"#visibility\" class=\"cancel-post-visibility hide-if-no-js button-cancel\">{}</a>\n\t\t\t\t\t</p>\n\t\t\t\t</div>\n\t\t\t",
            checked("password"),
            tr("Password protected"),
            tr("Password:"),
            esc_attr(&password),
            checked("private"),
            tr("Private"),
            tr("OK"),
            tr("Cancel")
        ));
    }
    h.push_str("\t\t</div>\n\n\t\t");

    // Date
    let stamp_time = |local: &str| {
        let ts = site.timezone.local_timestamp(local).unwrap_or(ctx.env.now);
        rustpress_core::i18n::sprintf(&tr("%1$s at %2$s"), &[&site.date(&site.trx("M j, Y", "publish box date format"), ts), &site.date(&site.trx("H:i", "publish box time format"), ts)])
    };
    let now_local = site.date("Y-m-d H:i:s", ctx.env.now);
    let gmt_ts = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt).unwrap_or(0);
    let (stamp, date) = if post.id != 0 {
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
    if let Some((count, latest)) = revisions {
        h.push_str(&format!(
            "\t\t\t<div class=\"misc-pub-section misc-pub-revisions\">\n\t\t\t\t{}\t\t\t\t<a class=\"hide-if-no-js\" href=\"{}\"><span aria-hidden=\"true\">{}</span> <span class=\"screen-reader-text\">\n\t\t\t\t\t{}\t\t\t\t</span></a>\n\t\t\t</div>\n\t\t\t",
            rustpress_core::i18n::sprintf(&tr("Revisions: %s"), &[&format!("<b>{}</b>", site.number_i18n(count))]),
            esc_url(&ctx.admin_url(&format!("revision.php?revision={latest}"))),
            site.trx("Browse", "revisions"),
            tr("Browse revisions")
        ));
    }
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
    h.push_str("\t</div>\n\t<div class=\"clear\"></div>\n</div>\n\n<div id=\"major-publishing-actions\">\n\t\t<div id=\"delete-action\">\n\t\t");
    if ctx.user.can_delete_post(post) {
        let url = format!("{}&amp;_wpnonce={}", ctx.admin_url(&format!("post.php?post={}&action=trash", post.id)).replace('&', "&amp;"), ctx.create_nonce(&format!("trash-post_{}", post.id)));
        h.push_str(&format!("\t\t\t<a class=\"submitdelete deletion\" href=\"{url}\">{}</a>\n\t\t\t", tr("Move to Trash")));
    }
    h.push_str("\t</div>\n\n\t<div id=\"publishing-action\">\n\t\t<span class=\"spinner\"></span>\n\t\t");
    if !["publish", "future", "private"].contains(&status) || post.id == 0 {
        let scheduled = !post.post_date_gmt.is_empty() && post.post_date_gmt != "0000-00-00 00:00:00" && ctx.env.now < gmt_ts;
        let submit = |label: &str| super::meta_boxes::submit_button(label, "primary large", "publish", false, None, &[]);
        if can_publish {
            let label = if scheduled { site.trx("Schedule", "post action/button label") } else { tr("Publish") };
            h.push_str(&format!("\t\t\t\t\t<input name=\"original_publish\" type=\"hidden\" id=\"original_publish\" value=\"{}\" />\n\t\t\t\t\t{}\t\t\t\t\t", esc_attr(&label), submit(&label)));
        } else {
            let label = tr("Submit for Review");
            h.push_str(&format!("\t\t\t\t<input name=\"original_publish\" type=\"hidden\" id=\"original_publish\" value=\"{}\" />\n\t\t\t\t{}\t\t\t\t", esc_attr(&label), submit(&label)));
        }
    } else {
        let label = tr("Update");
        h.push_str(&format!(
            "\t\t\t<input name=\"original_publish\" type=\"hidden\" id=\"original_publish\" value=\"{}\" />\n\t\t\t{}\t\t\t",
            esc_attr(&label),
            super::meta_boxes::submit_button(&label, "primary large", "save", false, Some("publish"), &[])
        ));
    }
    h.push_str("\t</div>\n\t<div class=\"clear\"></div>\n</div>\n\n</div>\n\t");
    h
}

/// The help tabs `edit-form-advanced.php` adds for posts and pages.
fn edit_help(ctx: &Ctx<'_>, info: &TypeInfo) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let p = |s: &str| format!("<p>{}</p>", tr(s));
    let mut tabs = Vec::new();
    let mut sidebar = String::new();
    if info.name == "post" {
        tabs.push(HelpTab { id: "customize-display".into(), title: tr("Customizing This Display"), content: p("The title field and the big Post Editing Area are fixed in place, but you can reposition all the other boxes using drag and drop. You can also minimize or expand them by clicking the title bar of each box. Use the Screen Options tab to unhide more boxes (Excerpt, Send Trackbacks, Custom Fields, Discussion, Slug, Author) or to choose a 1- or 2-column layout for this screen.") });
        let content = [
            p("<strong>Title</strong> &mdash; Enter a title for your post. After you enter a title, you&#8217;ll see the permalink below, which you can edit."),
            p("<strong>Post editor</strong> &mdash; Enter the text for your post. There are two modes of editing: Visual and Code. Choose the mode by clicking on the appropriate tab."),
            p("Visual mode gives you an editor that is similar to a word processor. Click the Extended Toolbar button to get a second row of controls."),
            p("The Code mode allows you to enter HTML along with your post text. Note that &lt;p&gt; and &lt;br&gt; tags are converted to line breaks when switching to the Code editor to make it less cluttered. When you type, a single line break can be used instead of typing &lt;br&gt;, and two line breaks instead of paragraph tags. The line breaks are converted back to tags automatically."),
            p("You can insert media files by clicking the button above the post editor and following the directions. You can align or edit images using the inline formatting toolbar available in Visual mode."),
            p("You can enable distraction-free writing mode using the icon to the right. This feature is not available for old browsers or devices with small screens, and requires that the full-height editor be enabled in Screen Options."),
            format!("<p>{}</p>", rustpress_core::i18n::sprintf(&tr("Keyboard users: When you are working in the visual editor, you can use %s to access the toolbar."), &["<kbd>Alt + F10</kbd>"])),
        ]
        .concat();
        tabs.push(HelpTab { id: "title-post-editor".into(), title: tr("Title and Post Editor"), content });
        sidebar = format!(
            "<p>{}</p><p><strong>{}</strong></p>{}{}",
            rustpress_core::i18n::sprintf(&tr("You can also create posts with the <a href=\"%s\">Press This bookmarklet</a>."), &["tools.php"]),
            tr("For more information:"),
            p("<a href=\"https://wordpress.org/documentation/article/write-posts-classic-editor/\">Documentation on Writing and Editing Posts</a>"),
            p("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
        );
    } else if info.name == "page" {
        let content = [
            p("Pages are similar to posts in that they have a title, body text, and associated metadata, but they are different in that they are not part of the chronological blog stream, kind of like permanent posts. Pages are not categorized or tagged, but can have a hierarchy. You can nest pages under other pages by making one the &#8220;Parent&#8221; of the other, creating a group of pages."),
            p("Creating a Page is very similar to creating a Post, and the screens can be customized in the same way using drag and drop, the Screen Options tab, and expanding/collapsing boxes as you choose. This screen also has the distraction-free writing space, available in both the Visual and Code modes via the Fullscreen buttons. The Page editor mostly works the same as the Post editor, but there are some Page-specific features in the Page Attributes box."),
        ]
        .concat();
        tabs.push(HelpTab { id: "about-pages".into(), title: tr("About Pages"), content });
        sidebar = format!(
            "<p><strong>{}</strong></p>{}{}{}",
            tr("For more information:"),
            p("<a href=\"https://wordpress.org/documentation/article/pages-add-new-screen/\">Documentation on Adding New Pages</a>"),
            p("<a href=\"https://wordpress.org/documentation/article/pages-screen/\">Documentation on Editing Pages</a>"),
            p("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")
        );
    }
    if info.name == "post" || info.name == "page" {
        let content = [
            p("You can upload and insert media (images, audio, documents, etc.) by clicking the Add Media button. You can select from the images and files already uploaded to the Media Library, or upload new media to add to your page or post. To create an image gallery, select the images to add and click the &#8220;Create a new gallery&#8221; button."),
            p("You can also embed media from many popular websites including Twitter, YouTube, Flickr and others by pasting the media URL on its own line into the content of your post/page. <a href=\"https://wordpress.org/documentation/article/embeds/\">Learn more about embeds</a>."),
        ]
        .concat();
        tabs.push(HelpTab { id: "inserting-media".into(), title: tr("Inserting Media"), content });
    }
    if info.name == "post" {
        let mut publish = format!("{}<ul><li>{}</li>", p("Several boxes on this screen contain settings for how your content will be published, including:"), tr("<strong>Publish</strong> &mdash; You can set the terms of publishing your post in the Publish box. For Status, Visibility, and Publish (immediately), click on the Edit link to reveal more options. Visibility includes options for password-protecting a post or making it stay at the top of your blog indefinitely (sticky). The Password protected option allows you to set an arbitrary password for each post. The Private option hides the post from everyone except editors and administrators. Publish (immediately) allows you to set a future or past date and time, so you can schedule a post to be published in the future or backdate a post."));
        if !ctx.env.theme.post_formats().is_empty() {
            publish.push_str(&format!("<li>{}</li>", tr("<strong>Format</strong> &mdash; Post Formats designate how your theme will display a specific post. For example, you could have a <em>standard</em> blog post with a title and paragraphs, or a short <em>aside</em> that omits the title and contains a short text blurb. Your theme could enable all or some of 10 possible formats. <a href=\"https://developer.wordpress.org/advanced-administration/wordpress/post-formats/#supported-formats\">Learn more about each post format</a>.")));
        }
        if ctx.env.block_theme || ctx.env.theme.adds_theme_support("post-thumbnails") {
            publish.push_str(&format!("<li>{}</li>", rustpress_core::i18n::sprintf(&tr("<strong>%s</strong> &mdash; This allows you to associate an image with your post without inserting it. This is usually useful only if your theme makes use of the image as a post thumbnail on the home page, a custom header, etc."), &[&esc_html(&tr("Featured image"))])));
        }
        publish.push_str("</ul>");
        tabs.push(HelpTab { id: "publish-box".into(), title: tr("Publish Settings"), content: publish });
        let content = [
            p("<strong>Send Trackbacks</strong> &mdash; Trackbacks are a way to notify legacy blog systems that you&#8217;ve linked to them. Enter the URL(s) you want to send trackbacks. If you link to other WordPress sites they&#8217;ll be notified automatically using pingbacks, and this field is unnecessary."),
            p("<strong>Discussion</strong> &mdash; You can turn comments and pings on or off, and if there are comments on the post, you can see them here and moderate them."),
        ]
        .concat();
        tabs.push(HelpTab { id: "discussion-settings".into(), title: tr("Discussion Settings"), content });
    } else if info.name == "page" {
        let content = [
            p("<strong>Parent</strong> &mdash; You can arrange your pages in hierarchies. For example, you could have an &#8220;About&#8221; page that has &#8220;Life Story&#8221; and &#8220;My Dog&#8221; pages under it. There are no limits to how many levels you can nest pages."),
            p("<strong>Template</strong> &mdash; Some themes have custom templates you can use for certain pages that might have additional features or custom layouts. If so, you&#8217;ll see them in this dropdown menu."),
            p("<strong>Order</strong> &mdash; Pages are usually ordered alphabetically, but you can choose your own order by entering a number (1 for first, etc.) in this field."),
        ]
        .concat();
        tabs.push(HelpTab { id: "page-attributes".into(), title: tr("Page Attributes"), content });
    }
    Help { tabs, sidebar }
}

/// `register_and_do_post_meta_boxes()`: the boxes of the screen in the order they are added.
async fn register_meta_boxes(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo, is_new: bool, sticky: bool) -> Vec<super::meta_boxes::MetaBox> {
    use super::meta_boxes::{MetaBox, custom_fields_box, post_format_box, post_thumbnail_html, revisions_box, trackback_box};
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let supports = |feature: &str| site.post_type_supports(&info.name, feature);
    let db = ctx.env.db;
    let mut boxes: Vec<MetaBox> = Vec::new();
    // Revisions: only when there is more than one.
    let mut revisions = None;
    if supports("revisions") && post.post_status != "auto-draft" {
        let (count, latest) = db.revision_summary(post.id).await.unwrap_or((0, 0));
        if count > 1 {
            revisions = Some((count, latest));
            let mut list = db
                .posts(&PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(post.id), order_by: rustpress_db::OrderBy::Date, order: rustpress_db::Order::Desc, ..Default::default() })
                .await
                .unwrap_or_default();
            list.sort_by(|a, b| b.post_date.cmp(&a.post_date).then(b.id.cmp(&a.id)));
            let mut rows = Vec::new();
            for revision in &list {
                rows.push(revision_title(ctx, revision, post).await);
            }
            boxes.push(MetaBox::new("revisionsdiv", &tr("Revisions"), "normal", "core", revisions_box(ctx, &rows)));
        }
    }
    boxes.push(MetaBox::new("submitdiv", &tr("Publish"), "side", "core", submit_box(ctx, post, info, is_new, sticky, revisions).await));
    let formats = ctx.env.theme.post_formats();
    if !formats.is_empty() && supports("post-formats") {
        let current = db.terms_for_post(post.id, Some("post_format")).await.unwrap_or_default().first().and_then(|t| t.slug.strip_prefix("post-format-").map(str::to_owned)).unwrap_or_else(|| "0".to_owned());
        boxes.push(MetaBox::new("formatdiv", &site.trx("Format", "post format"), "side", "core", post_format_box(ctx, &formats, &current)));
    }
    if info.name == "post" {
        boxes.push(MetaBox::new("categorydiv", &tr("Categories"), "side", "core", category_box(ctx, post).await));
        boxes.push(MetaBox::new("tagsdiv-post_tag", &tr("Tags"), "side", "core", tags_box(ctx, post).await));
    }
    if supports("page-attributes") || !super::inline_edit::page_templates(ctx, info).is_empty() {
        let label = if info.name == "page" { tr("Page Attributes") } else { tr("Post Attributes") };
        boxes.push(MetaBox::new("pageparentdiv", &label, "side", "core", page_attributes_box(ctx, post, info).await));
    }
    // Block themes and themes with `add_theme_support( 'post-thumbnails' )`.
    if supports("thumbnail") && (ctx.env.block_theme || ctx.env.theme.adds_theme_support("post-thumbnails")) && ctx.can("upload_files") {
        let thumbnail = db.post_meta_value(post.id, "_thumbnail_id").await.ok().flatten().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        boxes.push(MetaBox::new("postimagediv", &esc_html(&tr("Featured image")), "side", "low", post_thumbnail_html(ctx, post.id, thumbnail, None)));
    }
    if supports("excerpt") {
        boxes.push(MetaBox::new("postexcerpt", &tr("Excerpt"), "normal", "core", excerpt_box(ctx, post)));
    }
    if supports("trackbacks") {
        let (to_ping, pinged) = db.post_pings(post.id).await.ok().flatten().unwrap_or_default();
        boxes.push(MetaBox::new("trackbacksdiv", &tr("Send Trackbacks"), "normal", "core", trackback_box(ctx, &to_ping, &pinged)));
    }
    if supports("custom-fields") {
        let meta: Vec<rustpress_db::PostMeta> = {
            let mut rows = db.post_meta(post.id).await.unwrap_or_default();
            rows.sort_by(|a, b| a.meta_key.cmp(&b.meta_key).then(a.meta_id.cmp(&b.meta_id)));
            rows
        };
        let mut keys = db.custom_field_keys(30).await.unwrap_or_default();
        super::meta_boxes::natcasesort(&mut keys);
        keys.retain(|k| !super::meta_boxes::is_protected_meta(k));
        boxes.push(MetaBox::new("postcustom", &tr("Custom Fields"), "normal", "core", custom_fields_box(ctx, &meta, &keys)));
    }
    if post.comment_status == "open" || post.ping_status == "open" || supports("comments") {
        boxes.push(MetaBox::new("commentstatusdiv", &tr("Discussion"), "normal", "core", discussion_box(ctx, post)));
    }
    if matches!(post.post_status.as_str(), "publish" | "private") && (post.comment_status == "open" || post.ping_status == "open" || post.comment_count > 0 || supports("comments")) {
        boxes.push(MetaBox::new("commentsdiv", &tr("Comments"), "normal", "core", comments_box(ctx, post).await));
    }
    if !(post.post_status == "pending" && !ctx.can(if info.name == "page" { "publish_pages" } else { "publish_posts" })) {
        boxes.push(MetaBox::new("slugdiv", &tr("Slug"), "normal", "core", slug_box(ctx, post)));
    }
    if supports("author") && ctx.can(if info.name == "page" { "edit_others_pages" } else { "edit_others_posts" }) {
        boxes.push(MetaBox::new("authordiv", &tr("Author"), "normal", "core", author_box(ctx, post).await));
    }
    boxes
}

/// `wp_post_revision_title_expanded()` of a revision in the list of the revisions box.
async fn revision_title(ctx: &Ctx<'_>, revision: &Post, parent: &Post) -> String {
    let site = ctx.site();
    let author = ctx.env.db.users_by_ids(&[revision.post_author]).await.unwrap_or_default().into_iter().next();
    let name = author.as_ref().map(|u| u.display_name.clone()).unwrap_or_default();
    let email = author.as_ref().map(|u| u.user_email.clone()).unwrap_or_default();
    let url = |s: u64| esc_url(&rustpress_blocks::content::avatar_url(&site.options, &email, s));
    let gravatar = format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-24 photo' height='24' width='24' loading='lazy' decoding='async'/>", url(24), url(48));
    let ts = site.timezone.local_timestamp(&revision.post_modified).unwrap_or(0);
    let mut date = site.date(&site.trx("F j, Y @ H:i:s", "revision date format"), ts);
    if ctx.user.can_edit_post(parent) {
        date = format!("<a href='{}'>{date}</a>", ctx.admin_url(&format!("revision.php?revision={}", revision.id)));
    }
    let modified = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&revision.post_modified_gmt).unwrap_or(0);
    let mut text = rustpress_core::i18n::sprintf(&ctx.tr("%1$s %2$s, %3$s ago (%4$s)"), &[&gravatar, &name, &super::media_lib::human_time_diff(ctx, modified, ctx.env.now), &date]);
    if revision.post_name.contains("autosave") {
        text = rustpress_core::i18n::sprintf(&ctx.tr("%s [Autosave]"), &[&text]);
    }
    text
}

/// `post_comment_meta_box()`: the comments of the post are loaded by Ajax.
async fn comments_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let total = ctx.env.db.comment_count_all_types(post.id).await.unwrap_or(0);
    let mut out = format!(
        "<input type=\"hidden\" id=\"add_comment_nonce\" name=\"add_comment_nonce\" value=\"{}\" />\t<p class=\"hide-if-no-js\" id=\"add-new-comment\"><button type=\"button\" class=\"button\" onclick=\"window.commentReply && commentReply.addcomment({});\">{}</button></p>\n\t<input type=\"hidden\" id=\"_ajax_fetch_list_nonce\" name=\"_ajax_fetch_list_nonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" /><table class=\"widefat fixed striped table-view-list comments wp-list-table comments-box\" style=\"display:none;\">\n\t<tbody id=\"the-comment-list\"\n\t\t data-wp-lists='list:comment'\t\t>\n\t\t\t</tbody>\n</table>\n\t\t",
        ctx.create_nonce("get-comments"),
        post.id,
        tr("Add Comment"),
        ctx.create_nonce("fetch-list-WP_Post_Comments_List_Table"),
        esc_url(&ctx.request.request_uri),
        );
    if total < 1 {
        out.push_str(&format!("<p id=\"no-comments\">{}</p>", tr("No comments yet.")));
    } else {
        let hidden = super::common::hidden_meta_boxes(ctx, &post.post_type, &post.post_type).await;
        if !hidden.iter().any(|h| h == "commentsdiv") {
            out.push_str(&format!("\t\t\t<script>jQuery(function(){{commentsBox.get({total}, 10);}});</script>\n\t\t\t"));
        }
        out.push_str(&format!("\t\t<p class=\"hide-if-no-js\" id=\"show-comments\"><a href=\"#commentstatusdiv\" onclick=\"commentsBox.load({total});return false;\">{}</a> <span class=\"spinner\"></span></p>\n\t\t", tr("Show comments")));
    }
    out.push_str(&format!(
        "<div class=\"hidden\" id=\"trash-undo-holder\">\n\t<div class=\"trash-undo-inside\">\n\t\t{}\t\t<span class=\"undo untrash\"><a href=\"#\">{}</a></span>\n\t</div>\n</div>\n<div class=\"hidden\" id=\"spam-undo-holder\">\n\t<div class=\"spam-undo-inside\">\n\t\t{}\t\t<span class=\"undo unspam\"><a href=\"#\">{}</a></span>\n\t</div>\n</div>\n\t",
        rustpress_core::i18n::sprintf(&tr("Comment by %s moved to the Trash."), &["<strong></strong>"]),
        tr("Undo"),
        rustpress_core::i18n::sprintf(&tr("Comment by %s marked as spam."), &["<strong></strong>"]),
        tr("Undo")
    ));
    out
}

/// `wp_terms_checklist()`: the category boxes.
async fn category_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let terms = db.terms("category").await.unwrap_or_default();
    // A new post has no category yet; the default one is given when it is saved.
    let selected: Vec<u64> = if post.id != 0 { db.terms_for_post(post.id, Some("category")).await.unwrap_or_default().into_iter().map(|t| t.term_id).collect() } else { Vec::new() };
    let assigned = selected.clone();
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
            "\n\t\t<li id=\"popular-category-{id}\" class=\"popular-category\">\n\t\t\t<label class=\"selectit\">\n\t\t\t\t<input id=\"in-popular-category-{id}\" type=\"checkbox\"{} value=\"{id}\"  />\n\t\t\t\t{}\t\t\t</label>\n\t\t</li>\n\n\t\t",
            if assigned.contains(&term.term_id) { " checked=\"checked\"" } else { " " },
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
    let adder = if ctx.can("manage_categories") {
        format!(
            "\t\t\t<div id=\"category-adder\" class=\"wp-hidden-children\">\n\t\t\t\t<a id=\"category-add-toggle\" href=\"#category-add\" class=\"hide-if-no-js taxonomy-add-new\">\n\t\t\t\t\t+ {}\t\t\t\t</a>\n\t\t\t\t<p id=\"category-add\" class=\"category-add wp-hidden-child\">\n\t\t\t\t\t<label class=\"screen-reader-text\" for=\"newcategory\">{}</label>\n\t\t\t\t\t<input type=\"text\" name=\"newcategory\" id=\"newcategory\" class=\"form-required form-input-tip\" value=\"{}\" aria-required=\"true\" />\n\t\t\t\t\t<label class=\"screen-reader-text\" for=\"newcategory_parent\">\n\t\t\t\t\t\t{}\t\t\t\t\t</label>\n\t\t\t\t\t<select  name='newcategory_parent' id='newcategory_parent' class='postform'>\n{parent_options}</select>\n\t\t\t\t\t<input type=\"button\" id=\"category-add-submit\" data-wp-lists=\"add:categorychecklist:category-add\" class=\"button category-add-submit\" value=\"{}\" />\n\t\t\t\t\t<input type=\"hidden\" id=\"_ajax_nonce-add-category\" name=\"_ajax_nonce-add-category\" value=\"{}\" />\t\t\t\t\t<span id=\"category-ajax-response\"></span>\n\t\t\t\t</p>\n\t\t\t</div>\n\t\t",
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
        "\t<div id=\"taxonomy-category\" class=\"categorydiv\">\n\t\t<ul id=\"category-tabs\" class=\"category-tabs\" role=\"tablist\">\n\t\t\t<li class=\"tabs\"><a href=\"#category-all\" role=\"tab\" aria-selected=\"true\" aria-controls=\"category-all\">{}</a></li>\n\t\t\t<li class=\"hide-if-no-js\"><a href=\"#category-pop\" role=\"tab\" aria-controls=\"category-pop\">{}</a></li>\n\t\t</ul>\n\n\t\t<div id=\"category-pop\" class=\"tabs-panel\" style=\"display: none;\" role=\"tabpanel\">\n\t\t\t<ul id=\"categorychecklist-pop\" class=\"categorychecklist form-no-clear\" >\n\t\t\t\t{pop}\t\t\t</ul>\n\t\t</div>\n\n\t\t<div id=\"category-all\" class=\"tabs-panel\" role=\"tabpanel\">\n\t\t\t<input type='hidden' name='post_category[]' value='0' />\t\t\t<ul id=\"categorychecklist\" data-wp-lists=\"list:category\" class=\"categorychecklist form-no-clear\">\n\t\t\t\t{list}\t\t\t</ul>\n\t\t</div>\n\t{adder}\t</div>\n\t",
        tr("All Categories"),
        tr("Most Used")
    );
    inner
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
    inner
}

/// `page_attributes_meta_box()`: the parent, the template and the order.
async fn page_attributes_box(ctx: &Ctx<'_>, post: &Post, info: &TypeInfo) -> String {
    let tr = |s: &str| ctx.tr(s);
    let mut out = String::new();
    if info.hierarchical {
        let pages = ctx.env.db.posts(&PostQuery { post_types: vec![post.post_type.clone()], statuses: vec!["publish".into()], order_by: rustpress_db::OrderBy::MenuOrderTitle, order: rustpress_db::Order::Asc, ..Default::default() }).await.unwrap_or_default();
        fn tree(pages: &[Post], parent: u64, level: usize, current: u64, selected: u64, out: &mut String) {
            for page in pages.iter().filter(|p| p.post_parent == parent && p.id != current) {
                let pad = "&nbsp;".repeat(level * 3);
                out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\"{}>{pad}{}</option>\n", page.id, if page.id == selected { " selected=\"selected\"" } else { "" }, esc_html(&page.post_title)));
                tree(pages, page.id, level + 1, current, selected, out);
            }
        }
        let mut options = String::new();
        tree(&pages, 0, 0, post.id, post.post_parent, &mut options);
        // `wp_dropdown_pages()` prints nothing without pages.
        if !options.is_empty() {
            out.push_str(&format!(
                "<p class=\"post-attributes-label-wrapper parent-id-label-wrapper\"><label class=\"post-attributes-label\" for=\"parent_id\">{}</label></p>\n\t\t\t<select name='parent_id' id='parent_id'>\n\t<option value=\"\">{}</option>\n{options}</select>\n",
                tr("Parent"),
                tr("(no parent)")
            ));
            out.push_str("\t\t\t");
        }
    }
    let templates = super::inline_edit::page_templates(ctx, info);
    if !templates.is_empty() && ctx.site().settings.page_for_posts != post.id {
        let current = ctx.env.db.post_meta_value(post.id, "_wp_page_template").await.ok().flatten().unwrap_or_default();
        let mut sorted = templates.clone();
        sorted.sort_by(|a, b| a.1.cmp(&b.1));
        let options: String = sorted
            .iter()
            .map(|(name, title)| format!("\n\t<option value='{}' {}>{}</option>", esc_attr(name), if !current.is_empty() && &current == name { " selected='selected'" } else { "" }, esc_html(title)))
            .collect();
        out.push_str(&format!(
            "<p class=\"post-attributes-label-wrapper page-template-label-wrapper\"><label class=\"post-attributes-label\" for=\"page_template\">{}</label>\n\t\t</p>\n<select name=\"page_template\" id=\"page_template\">\n\t\t<option value=\"default\">{}</option>\n\t\t{options}</select>\n",
            tr("Template"),
            esc_html(&tr("Default template"))
        ));
    }
    out.push('\t');
    if ctx.site().post_type_supports(&info.name, "page-attributes") {
        out.push_str(&format!(
            "<p class=\"post-attributes-label-wrapper menu-order-label-wrapper\"><label class=\"post-attributes-label\" for=\"menu_order\">{}</label></p>\n<input name=\"menu_order\" type=\"text\" size=\"4\" id=\"menu_order\" value=\"{}\" />\n\t\t\t\t",
            tr("Order"),
            post.menu_order
        ));
        // The page screen has help tabs.
        if info.name == "page" {
            out.push_str(&format!("<p class=\"post-attributes-help-text\">{}</p>\n\t\t\t", tr("Need help? Use the Help tab above the screen title.")));
        }
    }
    out
}

fn excerpt_box(ctx: &Ctx<'_>, post: &Post) -> String {
    let tr = |s: &str| ctx.tr(s);
    let inner = format!(
        "<label class=\"screen-reader-text\" for=\"excerpt\">\n\t{}</label><textarea rows=\"1\" cols=\"40\" name=\"excerpt\" id=\"excerpt\">{}</textarea>\n<p>\n\t{}</p>\n\t",
        tr("Excerpt"),
        esc_textarea(&post.post_excerpt),
        rustpress_core::i18n::sprintf(&tr("Excerpts are optional hand-crafted summaries of your content that can be used in your theme. <a href=\"%s\">Learn more about manual excerpts</a>."), &[&tr("https://wordpress.org/documentation/article/what-is-an-excerpt-classic-editor/")])
    );
    inner
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
    inner
}

fn slug_box(ctx: &Ctx<'_>, post: &Post) -> String {
    format!("<label class=\"screen-reader-text\" for=\"post_name\">\n\t{}</label><input name=\"post_name\" type=\"text\" class=\"large-text\" id=\"post_name\" value=\"{}\" />\n\t", ctx.tr("Slug"), esc_attr(&post.post_name))
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
    format!("<label class=\"screen-reader-text\" for=\"post_author_override\">\n\t{}</label>\n\t<select name='post_author_override' id='post_author_override' class=''>\n{options}</select>", ctx.tr("Author"))
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

    // `wp_set_post_lock()` of post.php; post-new.php sets none.
    let lock = format!("{}:{}", ctx.env.now, ctx.user.id);
    if !is_new && let Some(writer) = ctx.env.writer {
        writer.update_post_meta(post.id, "_edit_lock", &lock).await.or_log();
    }

    let mut h = String::from("<div class=\"wrap\">\n<h1 class=\"wp-heading-inline\">\n");
    h.push_str(&esc_html(&title));
    h.push_str("</h1>\n\n");
    let new_file = if info.name == "post" { "post-new.php".to_owned() } else { format!("post-new.php?post_type={}", info.name) };
    if !is_new && ctx.can(if info.name == "page" { "edit_pages" } else { "edit_posts" }) {
        h.push_str(&format!(" <a href=\"{}\" class=\"page-title-action\">{}</a>", esc_url(&ctx.admin_url(&new_file)), esc_html(&add_title)));
    }
    h.push_str("\n<hr class=\"wp-header-end\">\n\n");
    if let Some(number) = ctx.request.get("message").and_then(|m| m.parse::<u32>().ok())
        && let Some(text) = message_text(ctx, post, info, number)
    {
        h.push_str(&notices::notice(&text, "success", true, "message", &["updated"], true));
    }
    let lost = format!("<span class=\"spinner\"></span> {} <span class=\"hide-if-no-sessionstorage\">{}</span>", tr("<strong>Connection lost.</strong> Saving has been disabled until you are reconnected."), tr("This post is being backed up in your browser, just in case."));
    h.push_str(notices::notice(&lost, "", false, "lost-connection-notice", &["error", "hidden"], true).trim_end_matches('\n'));
    h.push_str("<form name=\"post\" action=\"post.php\" method=\"post\" id=\"post\"\n>\n");
    h.push_str(&ctx.nonce_field(&format!("update-post_{}", post.id)));
    let referer = ctx.request.get("referredby").unwrap_or_default();
    h.push_str(&format!(
        "<input type=\"hidden\" id=\"user-id\" name=\"user_ID\" value=\"{}\" />\n<input type=\"hidden\" id=\"hiddenaction\" name=\"action\" value=\"editpost\" />\n<input type=\"hidden\" id=\"originalaction\" name=\"originalaction\" value=\"editpost\" />\n<input type=\"hidden\" id=\"post_author\" name=\"post_author\" value=\"{}\" />\n<input type=\"hidden\" id=\"post_type\" name=\"post_type\" value=\"{}\" />\n<input type=\"hidden\" id=\"original_post_status\" name=\"original_post_status\" value=\"{}\" />\n<input type=\"hidden\" id=\"referredby\" name=\"referredby\" value=\"{}\" />\n",
        ctx.user.id,
        post.post_author,
        esc_attr(&info.name),
        esc_attr(&post.post_status),
        esc_url(referer)
    ));
    if !is_new {
        h.push_str(&format!("<input type=\"hidden\" id=\"active_post_lock\" value=\"{lock}\" />\n\t"));
    }
    if post.post_status != "draft" {
        h.push_str("<input type=\"hidden\" name=\"_wp_original_http_referer\" value=\"\" />");
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
    let screen_id = info.name.clone();
    let columns = super::common::layout_columns(ctx, &screen_id, 2).await;
    h.push_str(&format!("\n<div id=\"poststuff\">\n<div id=\"post-body\" class=\"metabox-holder columns-{}\">\n<div id=\"post-body-content\">\n\n", if columns == 1 { 1 } else { 2 }));
    if supports("title") {
        h.push_str(&format!(
            "<div id=\"titlediv\">\n<div id=\"titlewrap\">\n\t\t<label class=\"screen-reader-text\" id=\"title-prompt-text\" for=\"title\">{}</label>\n\t<input type=\"text\" name=\"post_title\" size=\"30\" value=\"{}\" id=\"title\" spellcheck=\"true\" autocomplete=\"off\" />\n",
            tr("Add title"),
            esc_attr(&post_title)
        ));
        if supports("editor") {
            h.push_str(&format!("\t\t\t<a href=\"#content\" class=\"button-secondary screen-reader-text skiplink\" onclick=\"if (tinymce) {{ tinymce.execCommand( 'mceFocus', false, 'content' ); }}\">{}</a>\n", tr("Skip to Editor")));
        }
        h.push_str("\t\t</div>\n\t<div class=\"inside\">\n\t");
        let can_publish = ctx.can(if info.name == "page" { "publish_pages" } else { "publish_posts" });
        let slug_box = site.is_post_type_viewable(&info.name) && !(post.post_status == "pending" && !can_publish);
        if slug_box {
            h.push_str("\t<div id=\"edit-slug-box\" class=\"hide-if-no-js\">\n\t\t\t");
        }
        if slug_box && post.post_status != "auto-draft" && !post.post_name.is_empty() && !site.options.string("permalink_structure", "").is_empty() {
            let link = site.permalink(post).await.unwrap_or_default();
            let name = &post.post_name;
            let shown = link.replacen(&format!("/{name}/"), &format!("/<span id=\"editable-post-name\">{}</span>/", esc_html(name)), 1);
            h.push_str(&format!(
                "<strong>{}</strong>\n<span id=\"sample-permalink\"><a href=\"{}\">{shown}</a></span>\n&lrm;<span id=\"edit-slug-buttons\"><button type=\"button\" class=\"edit-slug button button-small hide-if-no-js\" aria-label=\"{}\">{}</button></span>\n<span id=\"editable-post-name-full\">{}</span>\n",
                tr("Permalink:"),
                esc_url(&link),
                esc_attr(&tr("Edit permalink")),
                tr("Edit"),
                esc_html(name)
            ));
        }
        if slug_box {
            h.push_str("\t</div>\n\t\t\t");
        }
        h.push_str(&format!("</div>\n\t<input type=\"hidden\" id=\"samplepermalinknonce\" name=\"samplepermalinknonce\" value=\"{}\" /></div><!-- /titlediv -->\n", ctx.create_nonce("samplepermalink")));
    }
    let expand_setting = super::common::user_setting(ctx, "editor_expand").await.is_none_or(|v| v == "on");
    let mut editor_preinit = String::new();
    let mut editor_scripts: Vec<String> = Vec::new();
    let mut editor_footer = String::new();
    if supports("editor") {
        let rich_editing = db.user_meta(ctx.user.id, "rich_editing").await.ok().flatten();
        let mut editor = super::editor::classic_editor(ctx, &post.post_content, &info.name, &post.post_status, rich_editing.as_deref());
        // `wp_editor_expand` and the distraction-free mode, not on phones; the media button with `upload_files`.
        let mobile = super::common::wp_is_mobile(ctx);
        let expand = !mobile && super::common::user_setting(ctx, "editor_expand").await.is_none_or(|v| v == "on");
        let mut html = editor.html.replacen("\t<div id=\"postdivrich\" class=\"postarea\">\n", &format!("\t<div id=\"postdivrich\" class=\"postarea{}\">\n\n", if expand { " wp-editor-expand" } else { "" }), 1);
        if !mobile {
            html = html.replacen(" wp-editor-wrap html-active\"", " wp-editor-wrap html-active has-dfw\"", 1).replacen(" wp-editor-wrap tmce-active\"", " wp-editor-wrap tmce-active has-dfw\"", 1);
        }
        if ctx.can("upload_files") {
            html = html.replacen(
                "<div id=\"wp-content-editor-tools\" class=\"wp-editor-tools hide-if-no-js\">",
                &format!(
                    "<div id=\"wp-content-editor-tools\" class=\"wp-editor-tools hide-if-no-js\"><div id=\"wp-content-media-buttons\" class=\"wp-media-buttons\"><button type=\"button\" id=\"insert-media-button\" class=\"button insert-media add_media\" data-editor=\"content\" aria-haspopup=\"dialog\" aria-controls=\"wp-media-modal\"><span class=\"wp-media-buttons-icon\" aria-hidden=\"true\"></span> {}</button></div>\n",
                    tr("Add Media")
                ),
                1,
            );
        }
        editor.html = html;
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
            "\n<table id=\"post-status-info\" role=\"presentation\"><tbody><tr>\n\t<td id=\"wp-word-count\" class=\"hide-if-no-js\">\n\t{}\t</td>\n\t<td class=\"autosave-info\">\n\t<span class=\"autosave-message\">&nbsp;</span>\n\t{last_edit}\t</td>\n\t<td id=\"content-resize-handle\" class=\"hide-if-no-js\"><br /></td>\n</tr></tbody></table>\n\n</div>\n",
            rustpress_core::i18n::sprintf(&tr("Word count: %s"), &["<span class=\"word-count\">0</span>"])
        ));
    }
    h.push_str("\t</div><!-- /post-body-content -->\n\n<div id=\"postbox-container-1\" class=\"postbox-container\">\n");
    let boxes = register_meta_boxes(ctx, post, info, is_new, sticky).await;
    let state = super::meta_boxes::BoxState::load(ctx, &screen_id, &info.name).await;
    h.push_str(&super::meta_boxes::do_meta_boxes(ctx, &boxes, "side", &state));
    h.push_str("</div>\n<div id=\"postbox-container-2\" class=\"postbox-container\">\n");
    h.push_str(&super::meta_boxes::do_meta_boxes(ctx, &boxes, "normal", &state));
    h.push_str(&super::meta_boxes::do_meta_boxes(ctx, &boxes, "advanced", &state));
    h.push_str("</div>\n</div><!-- /post-body -->\n<br class=\"clear\" />\n</div><!-- /poststuff -->\n</form>\n</div>\n\n");
    if supports("comments") {
        h.push_str(&super::comments::reply_form_single(ctx));
    }
    h.push_str("\n\n");
    if post_title.is_empty() {
        h.push_str("<script>\ntry{document.post.title.focus();}catch(e){}\n</script>\n");
    }
    h.push_str(&editor_preinit);

    let (parent_file, mut submenu_file) = if info.name == "post" { ("edit.php".to_owned(), "edit.php".to_owned()) } else { (format!("edit.php?post_type={}", info.name), format!("edit.php?post_type={}", info.name)) };
    // post-new.php marks "Add New" in the menu.
    if is_new {
        submenu_file = if info.name == "post" { "post-new.php".to_owned() } else { format!("post-new.php?post_type={}", info.name) };
    }
    let file = if is_new { "post-new.php" } else { "post.php" };
    let help = edit_help(ctx, info);
    let view_item = if !is_new && site.is_post_type_viewable(&info.name) && post.post_status != "auto-draft" {
        Some((if info.name == "page" { tr("View Page") } else { tr("View Post") }, site.permalink(post).await.unwrap_or_default()))
    } else {
        None
    };
    let options = crate::chrome::ScreenOptions {
        meta_boxes: Some(super::meta_boxes::prefs(&boxes, &state)),
        layout_columns: Some((2, columns)),
        editor_expand: supports("editor").then(|| !super::common::wp_is_mobile(ctx)).filter(|shown| *shown).map(|_| expand_setting),
        ..Default::default()
    };
    let screen = Screen { file: file.into(), parent_file, submenu_file: Some(submenu_file), post_type: info.name.clone(), title: title.clone(), help: Some(help), view_item, options: Some(options), ..Default::default() };
    let mut scripts: Vec<String> = vec!["post".into(), "word-count".into(), "autosave".into(), "wp-lists".into(), "suggest".into(), "postbox".into(), "tags-box".into()];
    scripts.extend(editor_scripts);
    let mut assets = Assets { scripts, footer: editor_footer, ..Default::default() };
    if supports("editor") && ctx.can("upload_files") {
        super::media_ui::enqueue(ctx, &mut assets, post.id).await;
    }
    render(ctx, &screen, &h, &assets).await
}

/// `post-new.php`.
pub async fn post_new(ctx: &Ctx<'_>) -> AdminResponse {
    let post_type = ctx.request.get("post_type").filter(|t| !t.is_empty()).unwrap_or("post").to_owned();
    let Some(info) = type_info(ctx, &post_type) else {
        return super::common::die_plain(ctx, "Invalid post type.");
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
        match super::common::wp_get_referer(ctx) {
            Some(r) if !r.contains("post.php") && !r.contains("post-new.php") => {
                let mut url = r;
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
    let can_delete = |post: &Post| ctx.user.can_delete_post(post);
    let ays = || die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
    match action.as_str() {
        "post-quickdraft-save" => super::dashboard::quickdraft_save(ctx).await,
        "edit" => {
            // `post.php?action=edit` without a post goes to `post.php`, which goes to the list.
            if post_id == 0 {
                return redirect(&ctx.admin_url("post.php"));
            }
            let Some(post) = post else {
                return die(ctx.env, 404, "WordPress &rsaquo; Error", &tr("You attempted to edit an item that does not exist. Perhaps it was deleted?"));
            };
            if post.post_type == "attachment" {
                if !(ctx.user.can_edit_post(&post)) {
                    return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit this item.")));
                }
                return super::attachment_edit::form(ctx, &post).await;
            }
            let Some(info) = type_info(ctx, &post.post_type) else {
                return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit posts in this post type.")));
            };
            let may_edit = ctx.user.can_edit_post(&post);
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
    let may_edit = ctx.user.can_edit_post(&post);
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
        writer.update_post_meta(post_id, "_edit_last", &ctx.user.id.to_string()).await.or_log();
        writer.update_post_meta(post_id, "_edit_lock", &format!("{}:{}", ctx.env.now, ctx.user.id)).await.or_log();
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
                writer.update_option("sticky_posts", &String::from_utf8_lossy(&rustpress_phpser::serialize(&value))).await.or_log();
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
