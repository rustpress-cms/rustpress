//! `wp-admin/upload.php` and `media-new.php`: the media library and the upload screen.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use rustpress_db::{AttachmentQuery, Post};

use super::common::{forbidden, read_only};
use super::{AdminResponse, Ctx, die, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::Content;
use crate::list_table::{Column, Table, row_actions};

/// `human_time_diff()`.
pub fn human_time_diff(ctx: &Ctx<'_>, from: i64, to: i64) -> String {
    let site = ctx.site();
    let diff = (to - from).abs();
    if diff < 3600 {
        let mins = ((diff as f64 / 60.0).round() as i64).max(1);
        sprintf(&site.trn("%s min", "%s mins", mins), &[&mins.to_string()])
    } else if diff < 86_400 {
        let hours = ((diff as f64 / 3600.0).round() as i64).max(1);
        sprintf(&site.trn("%s hour", "%s hours", hours), &[&hours.to_string()])
    } else if diff < 7 * 86_400 {
        let days = ((diff as f64 / 86_400.0).round() as i64).max(1);
        sprintf(&site.trn("%s day", "%s days", days), &[&days.to_string()])
    } else if diff < 30 * 86_400 {
        let weeks = ((diff as f64 / (7.0 * 86_400.0)).round() as i64).max(1);
        sprintf(&site.trn("%s week", "%s weeks", weeks), &[&weeks.to_string()])
    } else if diff < 365 * 86_400 {
        let months = ((diff as f64 / (30.0 * 86_400.0)).round() as i64).max(1);
        sprintf(&site.trn("%s month", "%s months", months), &[&months.to_string()])
    } else {
        let years = ((diff as f64 / (365.0 * 86_400.0)).round() as i64).max(1);
        sprintf(&site.trn("%s year", "%s years", years), &[&years.to_string()])
    }
}

fn help(ctx: &Ctx<'_>) -> Help {
    let tr = |s: &str| ctx.tr(s);
    Help {
        tabs: vec![
            HelpTab { id: "overview".into(), title: tr("Overview"), content: format!("<p>{}</p><p>{}</p>", tr("All the files you&#8217;ve uploaded are listed in the Media Library, with the most recent uploads listed first."), tr("You can view your media in a simple visual grid or a list with columns. Switch between these views using the icons to the left above the media.")) },
            HelpTab { id: "attachment-details".into(), title: tr("Attachment Details"), content: format!("<p>{}</p>", tr("Clicking an item will display an Attachment Details dialog, which allows you to preview media and make quick edits. Any changes you make to the attachment details will be automatically saved.")) },
        ],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/documentation/article/media-library-screen/\">Documentation on Media Library</a>"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

async fn mode(ctx: &Ctx<'_>) -> &'static str {
    match ctx.request.get("mode") {
        Some("list") => return "list",
        Some("grid") => return "grid",
        _ => {}
    }
    match ctx.env.db.user_meta(ctx.user.id, "media_library_mode").await.ok().flatten().as_deref() {
        Some("list") => "list",
        _ => "grid",
    }
}

/// `upload.php`.
pub async fn upload_php(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("upload_files") {
        return forbidden(ctx, "Sorry, you are not allowed to upload files.");
    }
    // Remember the mode.
    let mode_now = mode(ctx).await;
    if request.get("mode").is_some()
        && let Some(writer) = ctx.env.writer
    {
        let _ = writer.update_user_meta(ctx.user.id, "media_library_mode", mode_now).await;
    }
    if let Some(response) = delete_action(ctx).await {
        return response;
    }
    let title = tr("Media Library");
    if mode_now == "grid" {
        let mut assets = Assets::default();
        super::media_ui::enqueue(ctx, &mut assets, 0).await;
        assets.scripts.extend(["media-grid".to_owned(), "media".to_owned()]);
        assets.localize.push(("media-grid".into(), "_wpMediaGridSettings".into(), serde_json::json!({"adminUrl": "/wp-admin/", "queryVars": request.query.iter().filter(|(k, _)| ["s", "m", "attachment-filter", "item", "post__in", "post_mime_type", "orderby", "order"].contains(&k.as_str())).map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect::<serde_json::Map<_, _>>()})));
        let mut h = format!("\t\t<div class=\"wrap\" id=\"wp-media-grid\" data-search=\"{}\">\n\t\t<h1 class=\"wp-heading-inline\">{}</h1>\n\n", esc_attr(request.get("s").unwrap_or_default()), esc_html(&title));
        if ctx.can("upload_files") {
            h.push_str(&format!("\t\t\t\t\t<a href=\"{}\" class=\"page-title-action aria-button-if-js\">{}</a>\n\t\t\t", esc_url(&ctx.admin_url("media-new.php")), tr("Add Media File")));
        }
        h.push_str(&format!("\n\t\t<hr class=\"wp-header-end\">\n\n\t\t<div class=\"notice error hide-if-js\"><p>{}</p></div>\t</div>\n\t", sprintf(&tr("The grid view for the Media Library requires JavaScript. <a href=\"%s\">Switch to the list view</a>."), &["upload.php?mode=list"])));
        let screen = Screen { file: "upload.php".into(), parent_file: "upload.php".into(), submenu_file: Some("upload.php".into()), title, help: Some(help(ctx)), ..Default::default() };
        return render(ctx, &screen, &h, &assets).await;
    }
    list_mode(ctx, title).await
}

fn origin(ctx: &Ctx<'_>) -> String {
    let siteurl = &ctx.site().settings.siteurl;
    match siteurl.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
        None => siteurl.clone(),
    }
}

async fn list_mode(ctx: &Ctx<'_>, title: String) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let db = ctx.env.db;
    // `post_mime_type=image` in the address is the same filter as the drop-down's `post_mime_type:image`.
    let selected_filter = request.get("attachment-filter").unwrap_or_default().to_owned();
    let filter = match (request.get("attachment-filter").filter(|f| !f.is_empty()), request.get("post_mime_type").filter(|m| !m.is_empty())) {
        (Some(filter), _) => filter.to_owned(),
        (None, Some(mime)) => format!("post_mime_type:{mime}"),
        _ => String::new(),
    };
    let search = request.get("s").map(|s| s.trim().to_owned()).unwrap_or_default();
    let month = request.get("m").and_then(|m| m.parse::<u32>().ok()).unwrap_or(0);
    let orderby = request.get("orderby").filter(|o| ["title", "author", "parent", "date"].contains(o)).unwrap_or("date").to_owned();
    let desc = request.get("order").map_or(true, |o| o.eq_ignore_ascii_case("desc"));
    let per_page = db.user_meta(ctx.user.id, "upload_per_page").await.ok().flatten().and_then(|v| v.parse::<u64>().ok()).filter(|v| *v >= 1).unwrap_or(20);
    let paged = request.get("paged").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
    let mut query = AttachmentQuery { orderby: orderby.clone(), desc, limit: per_page, offset: (paged - 1) * per_page, ..Default::default() };
    if let Some(mime) = filter.strip_prefix("post_mime_type:") {
        query.mime = Some(mime.to_owned());
    }
    if filter == "detached" {
        query.parent = Some(0);
    }
    if filter == "mine" {
        query.author = Some(ctx.user.id);
    }
    if !search.is_empty() {
        query.search = Some(search.clone());
    }
    if month >= 190_000 {
        query.year = Some(month / 100);
        query.month = Some(month % 100);
    }
    if !ctx.can("edit_others_posts") {
        query.author = Some(ctx.user.id);
    }
    let (items, total) = db.list_attachments(&query).await.unwrap_or_default();
    let mut rows = String::new();
    for item in &items {
        rows.push_str(&row(ctx, item).await);
    }
    let columns = vec![
        Column::new("cb", ""),
        Column::new("title", &tr("File")).sortable("title", false, Some(&tr("File")), None),
        Column::new("author", &tr("Author")).sortable("author", false, Some(&tr("Author")), None),
        Column::new("parent", &tr("Uploaded to")).sortable("parent", false, Some(&tr("Uploaded to")), None),
        Column::new("date", &tr("Date")).sortable("date", true, Some(&tr("Date")), Some("desc")),
    ];
    let table = Table {
        site,
        plural: "media".into(),
        singular: String::new(),
        columns,
        primary: "title".into(),
        current_url: format!("{}{}", origin(ctx), request.request_uri),
        orderby: request.get("orderby").unwrap_or_default().to_owned(),
        order: if desc { "desc".into() } else { "asc".into() },
        total_items: total,
        per_page,
        current_page: paged,
        bulk_actions: if ctx.can("delete_posts") { vec![("delete".into(), tr("Delete permanently"))] } else { Vec::new() },
        has_items: !items.is_empty(),
    };
    let nonce_field = ctx.nonce_field("bulk-media");
    let ordered_by = match orderby.as_str() {
        "title" => tr("File"),
        "author" => tr("Author"),
        "parent" => tr("Uploaded to"),
        _ => tr("Date"),
    };
    let caption = sprintf(&if desc { tr("Table ordered by %s. Descending.") } else { tr("Table ordered by %s. Ascending.") }, &[&ordered_by]);
    let table_html = table.display(&nonce_field, "", "", &caption, &rows, &tr("Media items list"));

    // Filters above the table.
    let mime_counts = db.attachment_mime_counts().await.unwrap_or_default();
    let count_of = |prefix: &str| mime_counts.iter().filter(|(m, _)| m.starts_with(prefix)).map(|(_, n)| *n).sum::<u64>();
    let mut options = format!("<option value=\"\">{}</option>\n", tr("All media items"));
    for (prefix, key, label) in [("image/", "image", tr("Images")), ("audio/", "audio", tr("Audio")), ("video/", "video", tr("Video"))] {
        if count_of(prefix) > 0 {
            options.push_str(&format!("\t<option value=\"post_mime_type:{key}\"{}>{label}</option>\n", if selected_filter == format!("post_mime_type:{key}") { " selected='selected'" } else { "" }));
        }
    }
    options.push_str(&format!("\t<option value=\"detached\"{}>{}</option>\n", if filter == "detached" { " selected='selected'" } else { "" }, tr("Unattached")));
    options.push_str(&format!("\t<option value=\"mine\"{}>{}</option>\n", if filter == "mine" { " selected='selected'" } else { "" }, tr("Mine")));
    let mut months = format!("\t\t\t<option {} value=\"0\">{}</option>\n", if month == 0 { "selected='selected'" } else { "" }, tr("All dates"));
    for (year, m) in db.attachment_months().await.unwrap_or_default() {
        let value = format!("{year}{m:02}");
        let name = site.locale_data().month[(m as usize).saturating_sub(1).min(11)].clone();
        months.push_str(&format!("\t\t<option {} value='{value}'>{}</option>\n", if month.to_string() == value { "selected='selected'" } else { "" }, esc_html(&sprintf(&tr("%1$s %2$d"), &[&name, &year.to_string()]))));
    }
    let mut h = format!("<div class=\"wrap\">\n<h1 class=\"wp-heading-inline\">{}</h1>\n\n", esc_html(&title));
    h.push_str(&format!("\t<a href=\"{}\" class=\"page-title-action\">{}</a>\n\t\t\t\t\t\t\n", esc_url(&ctx.admin_url("media-new.php")), tr("Add Media File")));
    if !search.is_empty() {
        h.push_str(&format!("<span class=\"subtitle\">{}</span>\n", sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(&search))])));
    }
    h.push_str("<hr class=\"wp-header-end\">\n\n\n");
    h.push_str(&notice(ctx));
    // `view_switcher()`: the addresses of the other modes keep the rest of the query.
    let list_url = esc_url(&add_query_arg(&request.request_uri, "mode", "list"));
    let grid_url = esc_url(&add_query_arg(&request.request_uri, "mode", "grid"));
    h.push_str(&format!(
        "<form id=\"posts-filter\" method=\"get\">\n\n<h2 class='screen-reader-text'>{filter_heading}</h2>\t\t<div class=\"wp-filter\">\n\t\t\t<div class=\"filter-items\">\n\t\t\t\t\t\t<input type=\"hidden\" name=\"mode\" value=\"list\" />\n\t\t<div class=\"view-switch\">\n\t\t<a href='{list_url}' class='view-list current' id='view-switch-list' aria-current=\"page\"><span class='screen-reader-text'>{list}</span></a>\n<a href='{grid_url}' class='view-grid' id='view-switch-grid'><span class='screen-reader-text'>{grid}</span></a>\n\t\t</div>\n\t\t\n\t\t\t\t<label for=\"attachment-filter\" class=\"screen-reader-text\">\n\t\t\t\t\t{by_type}\t\t\t\t</label>\n\t\t\t\t<select class=\"attachment-filters\" name=\"attachment-filter\" id=\"attachment-filter\">\n\t\t\t\t\t\t{options}\t\t\t\t</select>\n\n\t\t\t\t\t\t<div class=\"actions\">\n\t\t\t\t\t<label for=\"filter-by-date\" class=\"screen-reader-text\">{by_date}</label>\n\t\t<select name=\"m\" id=\"filter-by-date\">\n{months}\t\t</select>\n\t\t<input type=\"submit\" name=\"filter_action\" id=\"post-query-submit\" class=\"button button-compact\" value=\"{filter_btn}\"  />\t\t</div>\n\t\t\t\t\t</div>\n\n\t\t\t<div class=\"search-form\">\n\t\t\t\t<p class=\"search-box\">\n\t\t\t\t\t<label class=\"screen-reader-text\" for=\"media-search-input\">\n\t\t\t\t\t{search_label}\t\t\t\t\t</label>\n\t\t\t\t\t<input type=\"search\" id=\"media-search-input\" class=\"search\" name=\"s\" value=\"{search}\">\n\t\t\t\t\t<input id=\"search-submit\" type=\"submit\" class=\"button button-compact\" value=\"{search_label}\">\n\t\t\t\t</p>\n\t\t\t</div>\n\t\t</div>\n\t\t\n",
        filter_heading = tr("Filter media items list"),
        list = tr("List view"),
        grid = tr("Grid view"),
        by_type = tr("Filter by type"),
        by_date = tr("Filter by date"),
        filter_btn = tr("Filter"),
        search_label = tr("Search Media"),
        search = esc_attr(&search),
    ));
    h.push_str(&table_html);
    h.push_str(&format!(
        "\n<div id=\"ajax-response\"></div>\n\t<div id=\"find-posts\" class=\"find-box\" style=\"display: none;\">\n\t\t<div id=\"find-posts-head\" class=\"find-box-head\">\n\t\t\t{attach}\t\t\t<button type=\"button\" id=\"find-posts-close\"><span class=\"screen-reader-text\">\n\t\t\t\t{close}\t\t\t</span></button>\n\t\t</div>\n\t\t<div class=\"find-box-inside\">\n\t\t\t<div class=\"find-box-search\">\n\t\t\t\t\t\t\t\t<input type=\"hidden\" name=\"affected\" id=\"affected\" value=\"\" />\n\t\t\t\t<input type=\"hidden\" id=\"_ajax_nonce\" name=\"_ajax_nonce\" value=\"{nonce}\" />\t\t\t\t<label class=\"screen-reader-text\" for=\"find-posts-input\">\n\t\t\t\t\t{search_word}\t\t\t\t</label>\n\t\t\t\t<input type=\"text\" id=\"find-posts-input\" name=\"ps\" value=\"\" />\n\t\t\t\t<span class=\"spinner\"></span>\n\t\t\t\t<input type=\"button\" id=\"find-posts-search\" value=\"{search_word}\" class=\"button\" />\n\t\t\t\t<div class=\"clear\"></div>\n\t\t\t</div>\n\t\t\t<div id=\"find-posts-response\"></div>\n\t\t</div>\n\t\t<div class=\"find-box-buttons\">\n\t\t\t<input type=\"submit\" name=\"find-posts-submit\" id=\"find-posts-submit\" class=\"button button-primary alignright\" value=\"{select}\"  />\t\t\t<div class=\"clear\"></div>\n\t\t</div>\n\t</div>\n\t</form>\n</div>\n\n",
        attach = tr("Attach to existing content"),
        close = tr("Close media attachment panel"),
        nonce = ctx.create_nonce("find-posts"),
        search_word = tr("Search"),
        select = tr("Select")
    ));
    let screen = Screen { file: "upload.php".into(), parent_file: "upload.php".into(), submenu_file: Some("upload.php".into()), title, help: Some(help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &Assets { scripts: vec!["media".into(), "clipboard".into()], ..Default::default() }).await
}

fn notice(ctx: &Ctx<'_>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let mut message = String::new();
    let count = |key: &str| ctx.request.get(key).and_then(|v| v.parse::<i64>().ok()).filter(|n| *n > 0);
    if let Some(n) = count("attached") {
        message = if n == 1 { tr("Media file attached.") } else { sprintf(&tr("%s media files attached."), &[&site.number_i18n(n as u64)]) };
    }
    if let Some(n) = count("detach") {
        message = if n == 1 { tr("Media file detached.") } else { sprintf(&tr("%s media files detached."), &[&site.number_i18n(n as u64)]) };
    }
    if let Some(n) = count("deleted") {
        message = if n == 1 { tr("Media file permanently deleted.") } else { sprintf(&tr("%s media files permanently deleted."), &[&site.number_i18n(n as u64)]) };
    }
    if message.is_empty() {
        return String::new();
    }
    format!("<div id=\"message\" class=\"updated notice is-dismissible\"><p>{message}</p></div>\n")
}

async fn row(ctx: &Ctx<'_>, post: &Post) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let db = ctx.env.db;
    let meta = db.post_meta(post.id).await.unwrap_or_default();
    let value = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
    let attached = value("_wp_attached_file").unwrap_or_default();
    let url = rustpress_blocks::images::attachment_url(site, &attached);
    let metadata = value("_wp_attachment_metadata").and_then(|raw| rustpress_blocks::images::Metadata::parse(&raw));
    let kind = post.post_mime_type.split('/').next().unwrap_or_default();
    let title = if post.post_title.is_empty() { tr("(no title)") } else { post.post_title.clone() };
    let quote = format!("&#8220;{}&#8221;", esc_html(&title));
    let edit_url = ctx.admin_url(&format!("post.php?post={}&action=edit", post.id));
    // The icon: a 60x60 thumbnail for images.
    let icon = if kind == "image" {
        let thumb = metadata.as_ref().and_then(|m| rustpress_blocks::images::intermediate_size(&url, m, 60, 60)).map(|(u, w, h)| (u, w, h));
        let (src, w, h) = thumb.unwrap_or_else(|| {
            let (w, h) = metadata.as_ref().map_or((60, 60), |m| (m.width(), m.height()));
            (url.clone(), w, h)
        });
        let (w, h) = rustpress_blocks::images::constrain_dimensions(w, h, 60, 60);
        format!("<span class=\"media-icon image-icon\"><img width=\"{w}\" height=\"{h}\" src=\"{}\" class=\"attachment-60x60 size-60x60\" alt=\"\" decoding=\"async\" loading=\"lazy\" /></span>", esc_url(&src))
    } else {
        let icon_url = format!("{}/wp-includes/images/media/{}", site.settings.siteurl.trim_end_matches('/'), super::ajax_media::mime_icon(&post.post_mime_type));
        format!("<span class=\"media-icon {kind}-icon\"><img width=\"48\" height=\"64\" src=\"{}\" class=\"attachment-60x60 size-60x60\" alt=\"\" decoding=\"async\" loading=\"lazy\" /></span>", esc_url(&icon_url))
    };
    let can_edit = ctx.can("upload_files");
    let title_html = if can_edit { format!("<a href=\"{}\">\t\t\t\t{icon}\n\t\t\t\t{}</a>", esc_attr(&edit_url).replace("&#038;", "&amp;"), esc_html(&title)) } else { format!("{icon}\n\t\t\t\t{}", esc_html(&title)) };
    let state = if site.options.u64("site_logo", 0) == post.id { format!(" &mdash; <span class='post-state'>{}</span>", tr("Logo")) } else { String::new() };
    let mut actions: Vec<(String, String)> = Vec::new();
    if can_edit {
        actions.push(("edit".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&edit_url), esc_attr(&sprintf(&tr("Edit %s"), &[&quote])), tr("Edit"))));
        actions.push(("delete".into(), format!("<a href=\"post.php?action=delete&#038;post={}&#038;_wpnonce={}\" class=\"submitdelete aria-button-if-js\" onclick='return showNotice.warn();' aria-label=\"{}\">{}</a>", post.id, ctx.create_nonce(&format!("delete-post_{}", post.id)), esc_attr(&sprintf(&tr("Delete %s permanently"), &[&quote])), tr("Delete Permanently"))));
    }
    let permalink = site.permalink(post).await.unwrap_or_default();
    actions.push(("view".into(), format!("<a href=\"{}\" aria-label=\"{}\" rel=\"bookmark\">{}</a>", esc_url(&permalink), esc_attr(&sprintf(&tr("View %s"), &[&quote])), tr("View"))));
    actions.push(("copy".into(), format!("<span class=\"copy-to-clipboard-container\"><button type=\"button\" class=\"button-link copy-attachment-url media-library\" data-clipboard-text=\"{}\" aria-label=\"{}\">{}</button><span class=\"success hidden\" aria-hidden=\"true\">{}</span></span>", esc_attr(&url), esc_attr(&sprintf(&tr("Copy %s URL to clipboard"), &[&quote])), tr("Copy URL"), tr("Copied!"))));
    actions.push(("download".into(), format!("<a href=\"{}\" aria-label=\"{}\" download>{}</a>", esc_url(&url), esc_attr(&sprintf(&tr("Download %s"), &[&quote])), tr("Download file"))));
    let author = match db.user(post.post_author).await.ok().flatten() {
        Some(user) => format!("<a href=\"{}\">{}</a>", esc_url(&format!("upload.php?author={}", user.id)), esc_html(&user.display_name)),
        None => format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{}</span>", tr("(no author)")),
    };
    let parent = match db.post(post.post_parent).await.ok().flatten().filter(|_| post.post_parent != 0) {
        Some(parent) => {
            let parent_title = if parent.post_title.is_empty() { tr("(no title)") } else { parent.post_title.clone() };
            let detach = if ctx.can("edit_posts") { format!("<br />\n<a href=\"upload.php?parent_post_id={}&media[]={}&_wpnonce={}\" class=\"hide-if-no-js detach-from-parent\" aria-label=\"{}\">{}</a>", parent.id, post.id, ctx.create_nonce("bulk-media"), esc_attr(&sprintf(&tr("Detach from %s"), &[&format!("&#8220;{}&#8221;", esc_html(&parent_title))])), tr("Detach")) } else { String::new() };
            format!("<strong><a href=\"{}\">{}</a></strong>\n{detach}", esc_url(&ctx.admin_url(&format!("post.php?post={}&action=edit", parent.id))).replace("&#038;", "&amp;"), esc_html(&parent_title))
        }
        None => format!("{}\t\t\t<br /><a href=\"#the-list\" onclick=\"findPosts.open( 'media[]', '{}' ); return false;\" class=\"hide-if-no-js aria-button-if-js\" aria-label=\"{}\">{}</a>", tr("(Unattached)"), post.id, esc_attr(&sprintf(&tr("Attach %s to existing content"), &[&quote])), tr("Attach")),
    };
    let timestamp = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt).unwrap_or(0);
    let diff = ctx.env.now - timestamp;
    let date = if diff > 0 && diff < 86_400 { sprintf(&tr("%s ago"), &[&human_time_diff(ctx, timestamp, ctx.env.now)]) } else { site.date(&tr("Y/m/d"), timestamp) };
    format!(
        "\t\t\t<tr id=\"post-{id}\" class=\"author-{owner} status-inherit\">\n\t\t\t\t<td class=\"check-column\">\t\t\t<input type=\"checkbox\" name=\"media[]\" id=\"cb-select-{id}\" value=\"{id}\" />\n\t\t\t<label for=\"cb-select-{id}\">\n\t\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{select}\t\t\t\t</span>\n\t\t\t</label>\n\t\t\t</td><th class='title column-title has-row-actions column-primary' data-colname=\"{file}\" scope=\"row\" aria-label=\"{label}\">\t\t<strong class=\"has-media-icon\">\n\t\t\t{title_html}{state}\t\t</strong>\n\t\t<p class=\"filename\">\n\t\t\t<span class=\"screen-reader-text\">\n\t\t\t\t{filename_label}\t\t\t</span>\n\t\t\t{filename}\t\t</p>\n\t\t{actions}</th><td class='author column-author' data-colname=\"{author_label}\">{author}</td><td class='parent column-parent' data-colname=\"{parent_label}\">{parent}</td><td class='date column-date' data-colname=\"{date_label}\">{date}</td>\t\t\t</tr>\n\t\t\t",
        id = post.id,
        owner = if post.post_author == ctx.user.id { "self" } else { "other" },
        select = sprintf(&tr("Select %s"), &[&esc_html(&title)]),
        file = tr("File"),
        label = esc_attr(&title),
        filename_label = tr("File name:"),
        filename = esc_html(attached.rsplit('/').next().unwrap_or_default()),
        actions = row_actions(site, &actions).trim_start_matches("").to_owned(),
        author_label = tr("Author"),
        parent_label = tr("Uploaded to"),
        date_label = tr("Date"),
    )
}

/// `post.php?action=delete` for one attachment, and the bulk actions of the list: delete, attach and detach.
async fn delete_action(ctx: &Ctx<'_>) -> Option<AdminResponse> {
    let request = ctx.request;
    let has_media = request.form.iter().chain(request.query.iter()).any(|(k, _)| k == "media[]");
    let action = if request.request("found_post_id").is_some() && has_media {
        "attach".to_owned()
    } else if request.request("parent_post_id").is_some() && has_media {
        "detach".to_owned()
    } else {
        let a = request.request("action").unwrap_or_default();
        if a.is_empty() || a == "-1" { request.request("action2").unwrap_or_default().to_owned() } else { a.to_owned() }
    };
    if !matches!(action.as_str(), "delete" | "attach" | "detach") || (action == "delete" && !request.post) {
        return None;
    }
    if !ctx.verify_nonce("bulk-media") {
        return Some(die(ctx.env, 403, &ctx.tr("WordPress Failure Notice"), &ctx.tr("The link you followed has expired.")));
    }
    if ctx.env.writer.is_none() {
        return Some(read_only(ctx));
    }
    let ids: Vec<u64> = request.form.iter().chain(request.query.iter()).filter(|(k, _)| k == "media[]").filter_map(|(_, v)| v.parse().ok()).collect();
    if action != "delete" {
        let parent: u64 = request.request(if action == "attach" { "found_post_id" } else { "parent_post_id" }).and_then(|p| p.parse().ok()).unwrap_or(0);
        if parent == 0 {
            return None;
        }
        if !ctx.can("edit_posts") {
            return Some(forbidden(ctx, "Sorry, you are not allowed to edit this post."));
        }
        let count = super::ajax_misc::attach(ctx, &ids, if action == "attach" { parent } else { 0 }).await;
        let referer = request.request("_wp_http_referer").filter(|r| r.contains("upload.php")).unwrap_or("upload.php").to_owned();
        let base = ctx.admin_url("upload.php");
        let location = match referer.split_once('?') {
            Some((_, query)) if !query.is_empty() => format!("{base}?{query}"),
            _ => base,
        };
        let location = remove_query_arg(&remove_query_arg(&location, "attached"), "detach");
        return Some(redirect(&add_query_arg(&location, if action == "attach" { "attached" } else { "detach" }, &count.to_string())));
    }
    if !ctx.can("delete_posts") {
        return Some(forbidden(ctx, "Sorry, you are not allowed to delete this item."));
    }
    let Some(writer) = ctx.env.writer else { return Some(read_only(ctx)) };
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let mut deleted = 0;
    for id in ids {
        if content.delete_attachment(id, super::ajax_media::uploads_dir(ctx.env)).await.unwrap_or(false) {
            deleted += 1;
        }
    }
    Some(redirect(&add_query_arg(&ctx.admin_url("upload.php?mode=list"), "deleted", &deleted.to_string())))
}

/// `media-new.php`.
pub async fn media_new(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    if !ctx.can("upload_files") {
        return forbidden(ctx, "Sorry, you are not allowed to upload files.");
    }
    // The browser uploader.
    if request.post && request.post_field("html-upload").is_some() {
        if !ctx.verify_nonce("media-form") {
            return die(ctx.env, 403, &tr("WordPress Failure Notice"), &tr("The link you followed has expired."));
        }
        let Some(writer) = ctx.env.writer else { return read_only(ctx) };
        let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
        if let Some(file) = request.files.iter().find(|f| f.field == "async-upload") {
            let uploaded = crate::media::UploadedFile { name: file.filename.clone(), bytes: file.bytes.clone() };
            if crate::media::handle_upload(&content, &super::ajax_media::uploads_dir(ctx.env), uploaded, 0).await.is_ok() {
                return redirect(&ctx.admin_url("upload.php"));
            }
        }
    }
    let title = tr("Upload Media");
    let mut assets = Assets { scripts: vec!["plupload-handlers".into(), "moxiejs".into(), "plupload".into()], ..Default::default() };
    assets.localize.push(("plupload-handlers".into(), "pluploadL10n".into(), {
        let baked: serde_json::Value = serde_json::from_str(include_str!("../data/media_bake.json")).unwrap_or_default();
        let mut strings = serde_json::Map::new();
        if let Some(serde_json::Value::Object(map)) = baked.get("plupload_l10n") {
            for (key, value) in map {
                strings.insert(key.clone(), value.as_str().map_or_else(|| value.clone(), |text| serde_json::Value::String(ctx.tr(text))));
            }
        }
        serde_json::Value::Object(strings)
    }));
    let nonce = ctx.create_nonce("media-form");
    let ajax_url = site.site_url("wp-admin/async-upload.php");
    let init = serde_json::json!({"browse_button": "plupload-browse-button", "container": "plupload-upload-ui", "drop_element": "drag-drop-area", "file_data_name": "async-upload", "url": ajax_url, "filters": {"max_file_size": format!("{}b", super::media_ui::MAX_UPLOAD)}, "multipart_params": {"post_id": 0, "_wpnonce": nonce, "type": "", "tab": "", "short": "1"}});
    let mut h = format!("\t\t<div class=\"wrap\">\n\t<h1>{}</h1>\n\n", esc_html(&title));
    h.push_str(&format!("\t<form enctype=\"multipart/form-data\" method=\"post\" action=\"{}\" class=\"wp-upload-form media-upload-form type-form validate\" id=\"file-form\">\n\n\t\t<div id=\"media-upload-notice\">\n\t\t</div>\n\t<div id=\"media-upload-error\">\n\t\t</div>\n\t\t<script>\n\t\tvar resize_height = 1024, resize_width = 1024,\n\twpUploaderInit = {init};\n\t</script>\n\n", esc_url(&ctx.admin_url("media-new.php"))));
    h.push_str(&format!(
        "\t<div id=\"plupload-upload-ui\" class=\"hide-if-no-js\">\n\t\t<div id=\"drag-drop-area\">\n\t\t<div class=\"drag-drop-inside\">\n\t\t<p class=\"drag-drop-info\">{drop}</p>\n\t\t<p>{or}</p>\n\t\t<p class=\"drag-drop-buttons\"><input id=\"plupload-browse-button\" type=\"button\" value=\"{select}\" class=\"button\" /></p>\n\t\t</div>\n\t</div>\n\t\t<p class=\"upload-flash-bypass\">\n\t{bypass}\t</p>\n\t\t</div>\n\n\t<div id=\"html-upload-ui\" class=\"hide-if-js\">\n\t\t<p id=\"async-upload-wrap\">\n\t\t<label class=\"screen-reader-text\" for=\"async-upload\">\n\t\t\t{upload}\t\t</label>\n\t\t<input type=\"file\" name=\"async-upload\" id=\"async-upload\" />\n\t\t<input type=\"submit\" name=\"html-upload\" id=\"html-upload\" class=\"button button-primary\" value=\"{upload}\"  />\t\t<a href=\"#\" onclick=\"try{{top.tb_remove();}}catch(e){{}}; return false;\">{cancel}</a>\n\t</p>\n\t<div class=\"clear\"></div>\n\t\t<p class=\"upload-html-bypass hide-if-no-js\">\n\t{html_bypass}\t</p>\n\t\t</div>\n\n<p class=\"max-upload-size\">\n\t{max}</p>\n\t\n\t<script>\n\tvar post_id = 0, shortform = 3;\n\t</script>\n\t<input type=\"hidden\" name=\"post_id\" id=\"post_id\" value=\"0\" />\n\t{nonce_field}\t<div id=\"media-items\" class=\"hide-if-no-js\"></div>\n\t</form>\n</div>\n\n",
        drop = tr("Drop files to upload"),
        or = tr("or"),
        select = tr("Select Files"),
        bypass = sprintf(&tr("You are using the multi-file uploader. Problems? Try the <button type=\"button\" class=\"button-link\">browser uploader</button> instead."), &[]),
        upload = tr("Upload"),
        cancel = tr("Cancel"),
        html_bypass = tr("You are using the browser&#8217;s built-in file uploader. The WordPress uploader includes multiple file selection and drag and drop capability. <button type=\"button\" class=\"button-link\">Switch to the multi-file uploader</button>."),
        max = sprintf(&tr("Maximum upload file size: %s."), &[&super::ajax_media::size_format(super::media_ui::MAX_UPLOAD)]),
        nonce_field = format!("<input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{nonce}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />", esc_url(&request.request_uri)),
    ));
    let screen = Screen { file: "media-new.php".into(), parent_file: "upload.php".into(), submenu_file: Some("media-new.php".into()), title, help: Some(help(ctx)), ..Default::default() };
    render(ctx, &screen, &h, &assets).await
}
