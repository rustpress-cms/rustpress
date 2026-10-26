//! `post.php` for an attachment: the "Edit Media" screen (`edit_form_image_editor()` and
//! `attachment_submitbox_metadata()`) and its save. The inline image editor is not part of RustPress.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_db::{Post, User, UserListQuery};

use super::ajax_media::{mime_icon, size_format, uploads_dir};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Screen};
use crate::content::{Content, PostData};

const TPL: &str = include_str!("../data/attachment_edit.tpl");

fn edit_link(ctx: &Ctx<'_>, id: u64) -> String {
    ctx.admin_url(&format!("post.php?post={id}&action=edit"))
}

/// `esc_textarea()`.
fn esc_textarea(text: &str) -> String {
    super::post_edit::esc_textarea_pub(text)
}

/// `get_sample_permalink_html()`: the link with the editable part of the slug; an unattached file has a plain link.
async fn sample_permalink(ctx: &Ctx<'_>, post: &Post) -> String {
    let permalink = ctx.site().permalink(post).await.unwrap_or_default();
    if post.post_parent == 0 {
        return format!("<a id=\"sample-permalink\" href=\"{}\">{}</a>", esc_url(&permalink), esc_html(&permalink));
    }
    let shown = match permalink.rfind(&format!("/{}/", post.post_name)) {
        Some(at) if !post.post_name.is_empty() => format!("{}/<span id=\"editable-post-name\">{}</span>/{}", &permalink[..at], esc_html(&post.post_name), &permalink[at + post.post_name.len() + 2..]),
        _ => esc_html(&permalink),
    };
    format!(
        "<span id=\"sample-permalink\"><a href=\"{}\">{shown}</a></span>
&lrm;<span id=\"edit-slug-buttons\"><button type=\"button\" class=\"edit-slug button button-small hide-if-no-js\" aria-label=\"{}\">{}</button></span>
<span id=\"editable-post-name-full\">{}</span>",
        esc_url(&permalink),
        esc_attr(&ctx.tr("Edit permalink")),
        ctx.tr("Edit"),
        esc_html(&post.post_name)
    )
}

async fn authors(ctx: &Ctx<'_>, selected: u64) -> String {
    let mut users: Vec<User> = Vec::new();
    for role in ["administrator", "editor", "author", "contributor"] {
        let query = UserListQuery { role: Some(role.to_owned()), orderby: "name".into(), ..Default::default() };
        for user in ctx.env.db.list_users(&query).await.map(|(users, _)| users).unwrap_or_default() {
            if !users.iter().any(|u| u.id == user.id) {
                users.push(user);
            }
        }
    }
    users.sort_by_key(|u| u.display_name.to_lowercase());
    let mut out = String::new();
    for user in &users {
        out.push_str(&format!("\t<option value='{}'{}>{} ({})</option>\n", user.id, if user.id == selected { " selected='selected'" } else { "" }, esc_html(&user.display_name), esc_html(&user.user_login)));
    }
    out
}

pub async fn form(ctx: &Ctx<'_>, post: &Post) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let db = ctx.env.db;
    let id = post.id;
    let meta = db.post_meta(id).await.unwrap_or_default();
    let value = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
    let attached = value("_wp_attached_file").unwrap_or_default();
    let url = rustpress_blocks::images::attachment_url(site, &attached);
    let metadata = value("_wp_attachment_metadata").and_then(|raw| rustpress_blocks::images::Metadata::parse(&raw));
    let kind = post.post_mime_type.split('/').next().unwrap_or_default().to_owned();
    let siteurl = site.settings.siteurl.trim_end_matches('/').to_owned();
    let image = kind == "image";

    // The preview.
    let thumb_url = if image {
        metadata.as_ref().and_then(|m| rustpress_blocks::images::intermediate_size(&url, m, 900, 450)).map_or_else(|| url.clone(), |(u, _, _)| u)
    } else {
        format!("{siteurl}/wp-includes/images/media/{}", mime_icon(&post.post_mime_type))
    };
    // `wp_image_editor_supports()`: the formats the editor can read and write.
    let edit_button = if matches!(post.post_mime_type.as_str(), "image/jpeg" | "image/png" | "image/gif" | "image/webp") {
        format!("<input type='button' id='imgedit-open-btn-{id}' onclick='imageEdit.open( {id}, \"{}\" )' class='button' value='{}' /> <span class='spinner'></span>", ctx.create_nonce(&format!("image_editor-{id}")), esc_attr(&tr("Edit Image")))
    } else {
        String::new()
    };
    let holder = if image {
        format!(
            "\t\t\t<div class=\"imgedit-response\" id=\"imgedit-response-{id}\"></div>\n\n\t\t<div class=\"wp_attachment_image wp-clearfix\" id=\"media-head-{id}\">\n\t\t\t<p id=\"thumbnail-head-{id}\"><img class=\"thumbnail\" src=\"{}\" style=\"max-width:100%\" alt=\"\" /></p>\n\t\t\t<p>{edit_button}</p>\n\t\t</div>\n\t\t<div style=\"display:none\" class=\"image-editor\" id=\"image-editor-{id}\">\n\t\t\t\t</div>\n",
            esc_url(&thumb_url)
        )
    } else if kind == "audio" {
        format!("\t\t<audio class=\"wp-audio-shortcode\" id=\"audio-{id}\" preload=\"none\" style=\"width: 100%;\" controls=\"controls\"><source type=\"{}\" src=\"{}\" /></audio>\n", esc_attr(&post.post_mime_type), esc_url(&url))
    } else if kind == "video" {
        let width = metadata.as_ref().map_or(0, |m| m.width().min(640));
        let height = metadata.as_ref().map_or(0, |m| if m.width() > 640 { (m.height() * 640 + m.width() / 2) / m.width() } else { m.height() });
        let size = if width > 0 && height > 0 { format!(" width=\"{width}\" height=\"{height}\"") } else { String::new() };
        format!("\t\t<video class=\"wp-video-shortcode\" id=\"video-{id}\"{size} preload=\"metadata\" controls=\"controls\"><source type=\"{}\" src=\"{}\" /></video>\n", esc_attr(&post.post_mime_type), esc_url(&url))
    } else {
        format!("\t\t<div class=\"wp_attachment_image wp-clearfix\" id=\"media-head-{id}\">\n\t\t\t<p id=\"thumbnail-head-{id}\">\n\t\t\t\t<img class=\"thumbnail\" src=\"{}\" style=\"max-width:100%\" alt=\"\" />\n\t\t\t</p>\n\t\t</div>\n", esc_url(&thumb_url))
    };
    let alt = value("_wp_attachment_image_alt").unwrap_or_default();
    let alt_block = if image {
        format!(
            "\t\t<p class=\"attachment-alt-text\">\n\t\t\t<label for=\"attachment_alt\"><strong>{}</strong></label><br />\n\t\t\t<textarea class=\"widefat\" name=\"_wp_attachment_image_alt\" id=\"attachment_alt\" aria-describedby=\"alt-text-description\">{}</textarea>\n\t\t</p>\n\t\t<p class=\"attachment-alt-text-description\" id=\"alt-text-description\">\n\t\t{}\t\t</p>\n\t",
            tr("Alternative Text"),
            esc_attr(&alt),
            sprintf(
                &tr("<a href=\"%1$s\" %2$s>Learn how to describe the purpose of the image%3$s</a>. Leave empty if the image is purely decorative."),
                &[&esc_url(&tr("https://www.w3.org/WAI/tutorials/images/decision-tree/")), "target=\"_blank\"", &format!("<span class=\"screen-reader-text\"> {}</span><span aria-hidden=\"true\" class=\"dashicons dashicons-external\"></span>", tr("(opens in a new tab)"))]
            )
        )
    } else {
        String::new()
    };

    // The facts of the save box.
    let filename = attached.rsplit('/').next().unwrap_or_default().to_owned();
    let extension = filename.rsplit_once('.').map(|(_, e)| e.to_uppercase());
    let filetype = match extension {
        Some(ext) => ext,
        None => post.post_mime_type.replace("image/", "").to_uppercase(),
    };
    let file_size = metadata.as_ref().and_then(|m| m.0.get("filesize").and_then(|v| v.as_u64())).filter(|s| *s > 0).or_else(|| std::fs::metadata(uploads_dir(ctx.env).join(&attached)).ok().map(|m| m.len())).unwrap_or(0);
    let filesize_block = if file_size > 0 { format!("			<div class=\"misc-pub-section misc-pub-filesize\">
			{} <strong>{}</strong>
		</div>
", tr("File size:"), size_format(file_size)) } else { String::new() };
    let dims = metadata.as_ref().filter(|m| m.width() > 0 && m.height() > 0).map(|m| format!("				<div class=\"misc-pub-section misc-pub-dimensions\">
			{} <strong><span id='media-dims-{id}'>{}</span></strong>
		</div>
", tr("Dimensions:"), sprintf(&tr("%1$s by %2$s pixels"), &[&m.width().to_string(), &m.height().to_string()]))).unwrap_or_default();
    let original_block = match metadata.as_ref().and_then(|m| m.0.get("original_image")).and_then(|v| v.as_str()) {
        Some(original) => {
            let base = url.rsplit_once('/').map_or("", |(dir, _)| dir);
            format!("				<div class=\"misc-pub-section misc-pub-original-image word-wrap-break-word\">
			{}			<a href=\"{}\">
				<strong>{}</strong>
			</a>
		</div>
", tr("Original image:"), esc_url(&format!("{base}/{original}")), esc_html(original))
        }
        None => String::new(),
    };

    let timestamp = site.timezone.local_timestamp(&post.post_date).unwrap_or(0);
    let uploaded = sprintf(&tr("%1$s at %2$s"), &[&site.date(&tr("M j, Y"), timestamp), &site.date(&tr("H:i"), timestamp)]);
    let uploaded_by = match db.user(post.post_author).await.ok().flatten() {
        Some(user) => {
            let name = if user.display_name.is_empty() { user.user_login.clone() } else { user.display_name.clone() };
            format!("\t\t\t\t\t{} <a href=\"{}\"><strong>{}</strong></a>\n", tr("Uploaded by:"), esc_url(&ctx.admin_url(&if user.id == ctx.user.id { "profile.php".to_owned() } else { format!("user-edit.php?user_id={}", user.id) })), esc_html(&name))
        }
        None => format!("\t\t\t\t\t{} <strong>{}</strong>\n", tr("Uploaded by:"), tr("(no author)")),
    };
    let uploaded_to = match db.post(post.post_parent).await.ok().flatten().filter(|_| post.post_parent != 0) {
        Some(parent) => {
            let title = if parent.post_title.is_empty() { tr("(no title)") } else { parent.post_title.clone() };
            format!("\t\t\t\t<div class=\"misc-pub-section misc-pub-uploadedto\">\n\t\t\t\t\t\t\t\t\t{} <a href=\"{}\"><strong>{}</strong></a>\n\t\t\t\t\t\t\t</div>\n\t\t\t", tr("Uploaded to:"), edit_link(ctx, parent.id), esc_html(&title))
        }
        None => "	".to_owned(),
    };

    // The lock, as `wp_set_post_lock()` leaves it.
    let lock = format!("{}:{}", ctx.env.now, ctx.user.id);
    if let Some(writer) = ctx.env.writer {
        writer.update_post_meta(id, "_edit_lock", &lock).await.or_log();
    }
    let message = match ctx.request.get("message") {
        Some("1" | "4") => format!("<div id=\"message\" class=\"notice notice-success is-dismissible updated\"><p>{}</p></div>", tr("Media file updated.")),
        Some("2") => format!("<div id=\"message\" class=\"notice notice-success is-dismissible updated\"><p>{}</p></div>", tr("Custom field updated.")),
        Some("3") => format!("<div id=\"message\" class=\"notice notice-success is-dismissible updated\"><p>{}</p></div>", tr("Custom field deleted.")),
        _ => String::new(),
    };

    let html = super::pages::translate_html(ctx, TPL)
        .replace("{{HOLDER}}", &holder)
        .replace("{{ALT_BLOCK}}", &alt_block)
        .replace("{{CAPTION_LABEL}}", &if image { tr("Image Caption") } else { tr("Short Description") })
        .replace("{{MESSAGE}}", &message)
        .replace("{{NONCE}}", &ctx.create_nonce(&format!("update-post_{id}")))
        .replace("{{REFERER}}", &esc_url(&ctx.request.request_uri))
        .replace("{{USER}}", &ctx.user.id.to_string())
        .replace("{{POST_AUTHOR}}", &post.post_author.to_string())
        .replace("{{LOCK}}", &lock)
        .replace("{{NONCE_ORDER}}", &ctx.create_nonce("meta-box-order"))
        .replace("{{NONCE_CLOSED}}", &ctx.create_nonce("closedpostboxes"))
        .replace("{{TITLE}}", &esc_attr(&post.post_title))
        .replace("{{SAMPLE_PERMALINK}}", &sample_permalink(ctx, post).await)
        .replace("{{NONCE_SAMPLE}}", &ctx.create_nonce("samplepermalink"))
        .replace("{{NONCE_IMAGE}}", &ctx.create_nonce(&format!("image_editor-{id}")))
        .replace("{{THUMB}}", &esc_url(&thumb_url))
        .replace("{{ALT}}", &esc_attr(&alt))
        .replace("{{CAPTION}}", &esc_textarea(&post.post_excerpt))
        .replace("{{DESCRIPTION}}", &esc_textarea(&post.post_content))
        .replace("{{UPLOADED}}", &uploaded)
        .replace("{{UPLOADED_BY}}", &uploaded_by)
        .replace("{{UPLOADED_TO}}", &uploaded_to)
        .replace("{{URL}}", &esc_url(&url))
        .replace("{{FILENAME}}", &esc_html(&filename))
        .replace("{{FILETYPE}}", &esc_html(&filetype))
        .replace("{{FILESIZE_BLOCK}}", &filesize_block)
        .replace("{{DIMS_BLOCK}}", &dims)
        .replace("{{ORIGINAL_BLOCK}}", &original_block)
        .replace("{{DELETE_URL}}", &esc_url(&ctx.admin_url(&format!("post.php?post={id}&action=delete&_wpnonce={}", ctx.create_nonce(&format!("delete-post_{id}"))))).replace("&#038;", "&amp;"))
        .replace("{{COMMENT_STATUS}}", if post.comment_status == "open" { " checked='checked' " } else { " " })
        .replace("{{PING_STATUS}}", if post.ping_status == "open" { " checked='checked' " } else { " " })
        .replace("{{NONCE_ADD_COMMENT}}", &ctx.create_nonce("add-comment"))
        .replace("{{NONCE_FETCH_LIST}}", &ctx.create_nonce("fetch-list-class-wp-post-comments-list-table"))
        .replace("{{SLUG}}", &esc_attr(&post.post_name))
        .replace("{{AUTHORS}}", &authors(ctx, post.post_author).await)
        .replace("{{NONCE_REPLY}}", &ctx.create_nonce("replyto-comment"))
        .replace("{{NONCE_UNFILTERED}}", &ctx.create_nonce("unfiltered-html-comment"))
        .replace("{{ID}}", &id.to_string())
        .replace("{{SITE}}", &siteurl);

    // The boxes the user hid (`do_meta_boxes()` gives them `hide-if-js`) and the layout of the screen.
    let hidden = super::common::hidden_meta_boxes(ctx, "attachment", "attachment").await;
    let columns = super::common::layout_columns(ctx, "attachment", 2).await;
    let boxes = [("commentstatusdiv", tr("Discussion")), ("commentsdiv", tr("Comments")), ("slugdiv", tr("Slug")), ("authordiv", tr("Author"))];
    let mut html = html;
    for (id, _) in &boxes {
        if !hidden.iter().any(|h| h == id) {
            html = html.replacen(&format!("id=\"{id}\" class=\"postbox  hide-if-js\""), &format!("id=\"{id}\" class=\"postbox \""), 1);
        }
    }
    if columns == 1 {
        html = html.replacen("<div id=\"post-body\" class=\"metabox-holder columns-2\">", "<div id=\"post-body\" class=\"metabox-holder columns-1\">", 1);
    }
    let options = crate::chrome::ScreenOptions {
        meta_boxes: Some(boxes.iter().map(|(id, title)| ((*id).to_owned(), title.clone(), hidden.iter().any(|h| h == id))).collect()),
        layout_columns: Some((2, columns)),
        editor_expand: Some(super::common::user_setting(ctx, "editor_expand").await.is_none_or(|v| v == "on")),
        ..Default::default()
    };
    let baked = super::pages::load(ctx, "attachment-edit");
    let screen = Screen { file: "post.php".into(), parent_file: "upload.php".into(), submenu_file: Some("upload.php".into()), post_type: "attachment".into(), title: tr("Edit Media"), help: Some(baked.help), view_item: Some((tr("View Media File"), site.permalink(post).await.unwrap_or_default())), options: Some(options), ..Default::default() };
    let scripts: Vec<String> = ["admin-comments", "suggest", "post", "image-edit"].into_iter().map(String::from).collect();
    let word_count = serde_json::json!({"type": "words", "shortcodes": ["wp_caption", "caption", "gallery", "playlist", "audio", "video", "embed"]});
    // `admin_footer`: the lock dialog, the backup notice and the arguments of the two comment list tables.
    let pre_footer = format!("{}{}", super::meta_boxes::post_lock_dialog(ctx, "attachment", &tr("Media")), super::meta_boxes::edit_footer_notices(ctx, "attachment", 2));
    let assets = Assets { scripts, styles: vec!["imgareaselect".into()], late_scripts: vec!["wplink".into()], localize: vec![("word-count".into(), "wordCountL10n".into(), word_count)], pre_footer, ..Default::default() };
    render(ctx, &screen, &html, &assets).await
}

/// `edit_post()` for an attachment.
pub async fn save(ctx: &Ctx<'_>, post: &Post) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    if !ctx.user.can_edit_post(post) {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", tr("Sorry, you are not allowed to edit this post.")));
    }
    let Some(writer) = ctx.env.writer else {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>");
    };
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let field = |name: &str| request.post_field(name);
    let author = field("post_author_override").and_then(|a| a.parse::<u64>().ok()).filter(|_| ctx.can("edit_others_posts"));
    let data = PostData {
        id: Some(post.id),
        post_author: author,
        post_content: field("content").map(str::to_owned),
        post_title: field("post_title").map(str::to_owned),
        post_excerpt: field("excerpt").map(str::to_owned),
        comment_status: Some(if field("comment_status").is_some() { "open".into() } else { "closed".into() }),
        ping_status: Some(if field("ping_status").is_some() { "open".into() } else { "closed".into() }),
        post_name: field("post_name").map(str::to_owned),
        ..Default::default()
    };
    if let Err(e) = content.insert_post(&data).await {
        return die(ctx.env, 200, "WordPress &rsaquo; Error", &format!("<p>{}</p>", esc_html(&e.to_string())));
    }
    if let Some(alt) = field("_wp_attachment_image_alt") {
        let current = ctx.env.db.post_meta_value(post.id, "_wp_attachment_image_alt").await.ok().flatten().unwrap_or_default();
        if current != alt {
            writer.update_post_meta(post.id, "_wp_attachment_image_alt", &rustpress_core::formatting::strip_tags(alt)).await.or_log();
        }
    }
    writer.update_post_meta(post.id, "_edit_last", &ctx.user.id.to_string()).await.or_log();
    writer.update_post_meta(post.id, "_edit_lock", &format!("{}:{}", ctx.env.now, ctx.user.id)).await.or_log();
    redirect(&rustpress_core::urls::add_query_arg(&edit_link(ctx, post.id), "message", "1"))
}
