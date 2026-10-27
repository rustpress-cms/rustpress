//! Meta boxes of the classic edit screen: `add_meta_box()`/`do_meta_boxes()` with the user's order, hidden and closed
//! boxes, and the boxes of `wp-admin/includes/meta-boxes.php` that `post_edit.rs` does not keep itself. The markup
//! follows the PHP templates byte for byte, including the newline PHP drops after `?>`.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;

use super::Ctx;

/// One registered meta box: `add_meta_box( $id, $title, …, $context, $priority )` and what its callback prints.
#[derive(Debug, Clone)]
pub struct MetaBox {
    pub id: String,
    pub title: String,
    pub context: &'static str,
    pub priority: &'static str,
    pub html: String,
}

impl MetaBox {
    pub fn new(id: &str, title: &str, context: &'static str, priority: &'static str, html: String) -> Self {
        Self { id: id.to_owned(), title: title.to_owned(), context, priority, html }
    }
}

/// What the user changed about the boxes of a screen.
#[derive(Debug, Clone, Default)]
pub struct BoxState {
    /// `get_hidden_meta_boxes()`.
    pub hidden: Vec<String>,
    /// `closedpostboxes_{$page}`.
    pub closed: Vec<String>,
    /// `meta-box-order_{$page}`: context -> box ids, in the saved order.
    pub order: Vec<(String, Vec<String>)>,
}

/// The strings of a stored PHP list.
fn stored_list(raw: Option<String>) -> Vec<String> {
    raw.and_then(|r| rustpress_phpser::unserialize(r.as_bytes()).ok())
        .and_then(|v| v.as_array().map(|items| items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect()))
        .unwrap_or_default()
}

impl BoxState {
    pub async fn load(ctx: &Ctx<'_>, screen_id: &str, post_type: &str) -> Self {
        let hidden = super::common::hidden_meta_boxes(ctx, screen_id, post_type).await;
        let closed = stored_list(super::common::user_option(ctx, &format!("closedpostboxes_{screen_id}")).await);
        let order = super::common::user_option(ctx, &format!("meta-box-order_{screen_id}"))
            .await
            .and_then(|r| rustpress_phpser::unserialize(r.as_bytes()).ok())
            .and_then(|v| {
                v.as_array().map(|items| {
                    items
                        .iter()
                        .filter_map(|(k, v)| {
                            let context = match k {
                                rustpress_phpser::Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
                                rustpress_phpser::Key::Int(n) => n.to_string(),
                            };
                            Some((context, v.as_str()?.split(',').filter(|s| !s.is_empty()).map(str::to_owned).collect()))
                        })
                        .collect()
                })
            })
            .unwrap_or_default();
        Self { hidden, closed, order }
    }
}

/// The wrapper of one box (`do_meta_boxes()`): header with the order and toggle buttons, then the content.
pub fn postbox(ctx: &Ctx<'_>, id: &str, title: &str, closed: bool, hidden: bool, body: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let classes = format!("postbox {}{}", if closed { "closed" } else { "" }, if hidden { " hide-if-js" } else { "" });
    let tooltip = |class: &str, label: &str, extra: &str, indicator: &str, bubble: &str| {
        format!(
            "<span class=\"wp-tooltip wp-is-tooltip\">\n\t\t\t\t<button aria-label=\"{label}\" type=\"button\" class=\"{class} wp-tooltip__toggle\"{extra} aria-describedby=\"{id}-title\">\n\t\t\t\t\t\t\t<span class=\"screen-reader-text\">{label}</span>\n\t\t\t\t\t\t\t<span class=\"{indicator}\" aria-hidden=\"true\"></span>\n\t\t\t\t\t\t</button>\n\t\t\t\t<span popover=\"hint\" id=\"{id}-{bubble}\" class=\"wp-tooltip__bubble\" role=\"tooltip\"><span id=\"{id}-{bubble}-text\" class=\"wp-tooltip__text\">{label}</span></span></span>"
        )
    };
    let expanded = if closed { "false" } else { "true" };
    let higher = tooltip("handle-order-higher", &tr("Move up"), "", "order-higher-indicator", "handle-order-higher-description");
    let lower = tooltip("handle-order-lower", &tr("Move down"), "", "order-lower-indicator", "handle-order-lower-description");
    let toggle = tooltip("handlediv", &tr("Show or hide panel"), &format!(" aria-expanded=\"{expanded}\""), "toggle-indicator", "handlediv");
    format!(
        "<div id=\"{id}\" class=\"{classes}\"  role=\"region\" aria-label=\"{}\">\n<div class=\"postbox-header\"><h2 class=\"hndle\" id=\"{id}-title\">{title}</h2>\n<div class=\"handle-actions hide-if-no-js\">{higher}{lower}{toggle}</div></div><div class=\"inside\">\n{body}</div>\n</div>\n",
        esc_attr(&rustpress_core::formatting::wp_strip_all_tags(title))
    )
}

/// Where the boxes are after the user's order moved them (`add_meta_box( …, 'sorted' )`): `(context, priority)`.
fn placement<'a>(boxes: &'a [MetaBox], state: &BoxState) -> Vec<(&'a MetaBox, String, &'static str)> {
    let mut placed: Vec<(&MetaBox, String, &'static str)> = boxes.iter().map(|b| (b, b.context.to_owned(), b.priority)).collect();
    let mut sorted: Vec<(String, String)> = Vec::new();
    for (context, ids) in &state.order {
        for id in ids {
            if !boxes.iter().any(|b| &b.id == id) {
                continue;
            }
            // An ID can be in only one priority and one context; a later position replaces an earlier one.
            sorted.retain(|(_, s)| s != id);
            sorted.push((context.clone(), id.clone()));
        }
    }
    let mut out: Vec<(&MetaBox, String, &'static str)> = Vec::new();
    // The boxes that keep their place, then the sorted ones in their order.
    placed.retain(|(b, ..)| !sorted.iter().any(|(_, id)| *id == b.id));
    out.extend(placed);
    for (context, id) in sorted {
        if let Some(b) = boxes.iter().find(|b| b.id == id) {
            out.push((b, context, "sorted"));
        }
    }
    out
}

/// `do_meta_boxes( $screen, $context, … )`.
pub fn do_meta_boxes(ctx: &Ctx<'_>, boxes: &[MetaBox], context: &str, state: &BoxState) -> String {
    let mut out = format!("<div id=\"{}-sortables\" class=\"meta-box-sortables\">", esc_attr(context));
    let placed = placement(boxes, state);
    for priority in ["high", "sorted", "core", "default", "low"] {
        for (b, c, p) in &placed {
            if c != context || *p != priority || b.title.is_empty() {
                continue;
            }
            out.push_str(&postbox(ctx, &b.id, &b.title, state.closed.contains(&b.id), state.hidden.contains(&b.id), &b.html));
        }
    }
    out.push_str("</div>");
    out
}

/// `meta_box_prefs()`: the boxes in the order they were added (by context, then priority), without the publish box.
pub fn prefs(boxes: &[MetaBox], state: &BoxState) -> Vec<(String, String, bool)> {
    let mut contexts: Vec<&str> = Vec::new();
    for b in boxes {
        if !contexts.contains(&b.context) {
            contexts.push(b.context);
        }
    }
    let mut out = Vec::new();
    for context in contexts {
        for priority in ["high", "core", "default", "low"] {
            for b in boxes.iter().filter(|b| b.context == context && b.priority == priority) {
                if b.title.is_empty() || b.id == "submitdiv" || b.id == "linksubmitdiv" {
                    continue;
                }
                out.push((b.id.clone(), b.title.clone(), state.hidden.contains(&b.id)));
            }
        }
    }
    out
}

/// `get_submit_button()`.
pub fn submit_button(text: &str, kind: &str, name: &str, wrap: bool, id: Option<&str>, attributes: &[(&str, String)]) -> String {
    let mut classes = vec!["button".to_owned()];
    for t in kind.split(' ').filter(|t| !t.is_empty()) {
        if t == "secondary" || t == "button-secondary" {
            continue;
        }
        let class = if matches!(t, "primary" | "small" | "large") { format!("button-{t}") } else { t.to_owned() };
        if !classes.contains(&class) {
            classes.push(class);
        }
    }
    let id = id.unwrap_or(name);
    let extra: String = attributes.iter().map(|(k, v)| format!("{k}=\"{}\" ", esc_attr(v))).collect();
    let name_attr = if name.is_empty() { String::new() } else { format!(" name=\"{}\"", esc_attr(name)) };
    let id_attr = if id.is_empty() { String::new() } else { format!(" id=\"{}\"", esc_attr(id)) };
    let button = format!("<input type=\"submit\"{name_attr}{id_attr} class=\"{}\" value=\"{}\" {extra} />", esc_attr(&classes.join(" ")), esc_attr(text));
    if wrap { format!("<p class=\"submit\">{button}</p>") } else { button }
}

/// `checked( $a, $b )`.
fn checked(on: bool) -> &'static str {
    if on { " checked='checked'" } else { "" }
}

/// `post_format_meta_box()`: `formats` are those of `add_theme_support( 'post-formats' )`, `current` the format of the
/// post (`0` for standard).
pub fn post_format_box(ctx: &Ctx<'_>, formats: &[String], current: &str) -> String {
    let name = |slug: &str| ctx.site().trx(&format_name(slug), "Post format");
    let mut list: Vec<String> = formats.to_vec();
    if current != "0" && !list.iter().any(|f| f == current) {
        list.push(current.to_owned());
    }
    let mut out = format!(
        "\t\t<div id=\"post-formats-select\">\n\t\t<fieldset>\n\t\t\t<legend class=\"screen-reader-text\">\n\t\t\t\t{}\t\t\t</legend>\n\t\t\t<input type=\"radio\" name=\"post_format\" class=\"post-format\" id=\"post-format-0\" value=\"0\" {} /> <label for=\"post-format-0\" class=\"post-format-icon post-format-standard\">{}</label>\n\t\t\t",
        ctx.tr("Post Formats"),
        checked(current == "0"),
        name("standard")
    );
    for format in &list {
        let f = esc_attr(format);
        out.push_str(&format!(
            "\t\t\t<br /><input type=\"radio\" name=\"post_format\" class=\"post-format\" id=\"post-format-{f}\" value=\"{f}\" {} /> <label for=\"post-format-{f}\" class=\"post-format-icon post-format-{f}\">{}</label>\n\t\t\t",
            checked(current == format),
            esc_html(&name(format))
        ));
    }
    out.push_str("\t\t</fieldset>\n\t</div>\n\t\t\t");
    out
}

/// `get_post_format_strings()` (untranslated).
fn format_name(slug: &str) -> String {
    match slug {
        "aside" => "Aside",
        "chat" => "Chat",
        "gallery" => "Gallery",
        "link" => "Link",
        "image" => "Image",
        "quote" => "Quote",
        "status" => "Status",
        "video" => "Video",
        "audio" => "Audio",
        _ => "Standard",
    }
    .to_owned()
}

/// `post_trackback_meta_box()`.
pub fn trackback_box(ctx: &Ctx<'_>, to_ping: &str, pinged: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let form = format!("<input type=\"text\" name=\"trackback_url\" id=\"trackback_url\" class=\"code\" value=\"{}\" aria-describedby=\"trackback-url-desc\" />", esc_attr(&to_ping.replace('\n', " ")));
    let mut out = format!(
        "<p>\n\t<label for=\"trackback_url\">{}</label>\n\t{form}</p>\n<p id=\"trackback-url-desc\" class=\"howto\">{}</p>\n<p>\n\t{}</p>\n\t",
        tr("Send trackbacks to:"),
        tr("Separate multiple URLs with spaces"),
        sprintf(
            &tr("Trackbacks are a way to notify legacy blog systems that you&#8217;ve linked to them. If you link other WordPress sites, they&#8217;ll be notified automatically using <a href=\"%s\">pingbacks</a>, no other action necessary."),
            &[&tr("https://wordpress.org/documentation/article/introduction-to-blogging/#comments")]
        )
    );
    if !pinged.is_empty() {
        out.push_str(&format!("<p>{}</p><ul>", tr("Already pinged:")));
        for url in pinged.trim().split('\n') {
            out.push_str(&format!("\n\t<li>{}</li>", esc_html(url)));
        }
        out.push_str("</ul>");
    }
    out
}

/// `wp_get_attachment_image( $id, array( 266, 266 ) )`, the size of the featured image box when the theme has no
/// `post-thumbnail` size; empty when the attachment has no image. `None` when there is no such post.
pub async fn post_thumbnail_image(ctx: &Ctx<'_>, id: u64) -> Option<String> {
    use rustpress_blocks::images::{attachment_url, calculate_sizes, constrain_dimensions, intermediate_size, is_image, load_metadata, preview_metadata, srcset};
    const SIZE: u64 = 266;
    let site = ctx.site();
    let post = ctx.env.db.post(id).await.ok().flatten()?;
    if post.post_type != "attachment" {
        return Some(String::new());
    }
    let (meta, attached) = load_metadata(site, &post).await.ok()?;
    let (meta, attached) = if is_image(&post) {
        (meta, attached)
    } else {
        match preview_metadata(meta.as_ref(), attached.as_deref()) {
            Some((meta, path)) => (Some(meta), Some(path)),
            None => return Some(String::new()),
        }
    };
    let Some(attached) = attached else { return Some(String::new()) };
    let full_url = attachment_url(site, &attached);
    // `image_downsize()`: the best generated size, else the original; then `image_constrain_size_for_editor()`.
    let (src, width, height) = match &meta {
        Some(meta) => {
            let (url, w, h) = intermediate_size(&full_url, meta, SIZE, SIZE).unwrap_or_else(|| (full_url.clone(), meta.width(), meta.height()));
            let (w, h) = constrain_dimensions(w, h, SIZE, SIZE);
            (url, w, h)
        }
        None => (full_url, 0, 0),
    };
    let alt = ctx.env.db.post_meta_value(id, "_wp_attachment_image_alt").await.ok().flatten().unwrap_or_default();
    let alt = rustpress_core::formatting::strip_tags(&alt).trim().to_owned();
    let mut attrs: Vec<(&str, String)> = vec![("src", src.clone()), ("class", format!("attachment-{SIZE}x{SIZE} size-{SIZE}x{SIZE}")), ("alt", alt), ("decoding", "async".into()), ("loading", "lazy".into())];
    if let Some(meta) = &meta
        && let (Some(set), Some(sizes)) = (srcset(site, width, height, &src, meta), calculate_sizes(width))
    {
        // Lazy images get `auto` in front of their sizes.
        attrs.extend([("srcset", set), ("sizes", format!("auto, {sizes}"))]);
    }
    let mut html = String::from("<img");
    if width != 0 {
        html.push_str(&format!(" width=\"{width}\""));
    }
    if height != 0 {
        html.push_str(&format!(" height=\"{height}\""));
    }
    for (name, value) in attrs {
        html.push_str(&format!(" {name}=\"{}\"", esc_attr(&value)));
    }
    html.push_str(" />");
    Some(html)
}

/// `_wp_post_thumbnail_html()` without a featured image or with one (`image` is `wp_get_attachment_image()`).
pub fn post_thumbnail_html(ctx: &Ctx<'_>, post_id: u64, thumbnail_id: u64, image: Option<&str>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let src = esc_url(&ctx.admin_url(&format!("media-upload.php?post_id={post_id}&type=image&TB_iframe=1")));
    let link = |describedby: &str, text: &str| format!("<p class=\"hide-if-no-js\"><a href=\"{src}\" id=\"set-post-thumbnail\"{describedby} class=\"thickbox\" role=\"button\" aria-haspopup=\"dialog\" aria-controls=\"wp-media-modal\">{text}</a></p>");
    let mut content = match image.filter(|i| !i.is_empty()) {
        Some(image) => format!(
            "{}<p class=\"hide-if-no-js howto\" id=\"set-post-thumbnail-desc\">{}</p><p class=\"hide-if-no-js\"><a href=\"#\" id=\"remove-post-thumbnail\" role=\"button\">{}</a></p>",
            link(" aria-describedby=\"set-post-thumbnail-desc\"", image),
            tr("Click the image to edit or update"),
            esc_html(&tr("Remove featured image"))
        ),
        None => link("", &esc_html(&tr("Set featured image"))),
    };
    content.push_str(&format!("<input type=\"hidden\" id=\"_thumbnail_id\" name=\"_thumbnail_id\" value=\"{}\" />", if thumbnail_id != 0 { thumbnail_id.to_string() } else { "-1".to_owned() }));
    content
}

/// `wp_list_post_revisions()`: `rows` are `wp_post_revision_title_expanded()` of each revision.
pub fn revisions_box(ctx: &Ctx<'_>, rows: &[String]) -> String {
    let mut out = format!("<div class='hide-if-js'><p>{}</p></div>\n<ul class='post-revisions hide-if-no-js'>\n", ctx.tr("JavaScript must be enabled to use this feature."));
    for row in rows {
        out.push_str(&format!("\t<li>{row}</li>\n"));
    }
    out.push_str("</ul>");
    out
}

/// `_admin_notice_post_locked()` for a post nobody else is editing: the hidden dialog shown when the lock is lost.
/// `all_items` is the label of the list the dialog leads back to.
pub fn post_lock_dialog(ctx: &Ctx<'_>, post_type: &str, all_items: &str) -> String {
    let back = if post_type == "post" { ctx.admin_url("edit.php") } else { ctx.admin_url(&format!("edit.php?post_type={post_type}")) };
    format!(
        "\t<div id=\"post-lock-dialog\" class=\"notification-dialog-wrap hidden\">\n\t<div class=\"notification-dialog-background\"></div>\n\t<div class=\"notification-dialog\">\n\t\t\t<div class=\"post-taken-over\">\n\t\t\t<div class=\"post-locked-avatar\"></div>\n\t\t\t<p class=\"wp-tab-first\" tabindex=\"0\">\n\t\t\t<span class=\"currently-editing\"></span><br />\n\t\t\t<span class=\"locked-saving hidden\"><img src=\"{}\" width=\"16\" height=\"16\" alt=\"\" /> {}</span>\n\t\t\t<span class=\"locked-saved hidden\">{}</span>\n\t\t\t</p>\n\t\t\t\t\t\t<p><a class=\"button button-primary wp-tab-last\" href=\"{}\">{all_items}</a></p>\n\t\t</div>\n\t\t\t</div>\n\t</div>\n\t",
        esc_url(&ctx.admin_url("images/spinner-2x.gif")),
        ctx.tr("Saving revision&hellip;"),
        ctx.tr("Your latest changes were saved as a revision."),
        esc_url(&back)
    )
}

/// `_admin_notice_post_locked()` while `holder` edits the post: the open dialog that offers to take it over.
pub async fn post_locked_dialog(ctx: &Ctx<'_>, post: &rustpress_db::Post, holder: &rustpress_db::User, all_items: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let public = ctx.site().is_post_type_viewable(&post.post_type);
    let (sendback, sendback_text) = match super::common::wp_get_referer(ctx) {
        Some(referer) if !referer.contains("post.php") && !referer.contains("post-new.php") => (referer, tr("Go back")),
        _ => {
            let list = if post.post_type == "post" { ctx.admin_url("edit.php") } else { ctx.admin_url(&format!("edit.php?post_type={}", post.post_type)) };
            (list, all_items.to_owned())
        }
    };
    let avatar = if ctx.site().show_avatars() {
        let url = |size: u64| esc_url(&rustpress_blocks::content::avatar_url(&ctx.site().options, &holder.user_email, size));
        format!("<img alt='' src='{}' srcset='{} 2x' class='avatar avatar-64 photo' height='64' width='64' loading='lazy' decoding='async'/>", url(64), url(128))
    } else {
        String::new()
    };
    // `get_preview_post_link()`: the latest content of a published post or of someone else's is in the autosave.
    let preview = if public && let Ok(permalink) = ctx.site().permalink(post).await {
        let mut link = permalink;
        if post.post_status == "publish" || holder.id != post.post_author {
            link = rustpress_core::urls::add_query_arg(&link, "preview_id", &post.id.to_string());
            link = rustpress_core::urls::add_query_arg(&link, "preview_nonce", &ctx.create_nonce(&format!("post_preview_{}", post.id)));
        }
        let link = rustpress_core::urls::add_query_arg(&link, "preview", "true");
        format!("\t\t<a class=\"button\" href=\"{}\">{}</a>\n\t\t\t", esc_url(&link), esc_html(&ctx.site().trx("Preview", "verb")))
    } else {
        String::new()
    };
    let take_over = format!("{}&get-post-lock=1", ctx.admin_url(&format!("post.php?post={}&action=edit&_wpnonce={}", post.id, ctx.create_nonce(&format!("lock-post_{}", post.id)))));
    format!(
        "\t<div id=\"post-lock-dialog\" class=\"notification-dialog-wrap\">\n\t<div class=\"notification-dialog-background\"></div>\n\t<div class=\"notification-dialog\">\n\t\t\t<div class=\"post-locked-message\">\n\t\t<div class=\"post-locked-avatar\">{avatar}</div>\n\t\t<p class=\"currently-editing wp-tab-first\" tabindex=\"0\">\n\t\t{}\t\t</p>\n\t\t\t\t<p>\n\t\t<a class=\"button\" href=\"{}\">{sendback_text}</a>\n\t\t{preview}\t<a class=\"button button-primary wp-tab-last\" href=\"{}\">{}</a>\n\t\t\t\t\t</p>\n\t\t</div>\n\t\t\t</div>\n\t</div>\n\t",
        sprintf(&tr("%s is currently editing this post. Do you want to take over?"), &[&esc_html(&holder.display_name)]),
        esc_url(&sendback),
        esc_url(&take_over),
        tr("Take over")
    )
}

/// `_local_storage_notice()` and the `list_args` of the comment list tables of the screen (`$count` tables).
pub fn edit_footer_notices(ctx: &Ctx<'_>, screen_id: &str, comment_tables: usize) -> String {
    let tr = |s: &str| ctx.tr(s);
    let mut out = format!(
        "<div id=\"local-storage-notice\" class=\"notice is-dismissible hidden\"><p class=\"local-restore\">{}<button type=\"button\" class=\"button restore-backup\">{}</button></p><p class=\"help\">{}</p></div>",
        tr("The backup of this post in your browser is different from the version below."),
        tr("Restore the backup"),
        tr("This will replace the current editor content with the last backup version. You can use undo and redo in the editor to get the old content back or to return to the restored version.")
    );
    for _ in 0..comment_tables {
        out.push_str(&format!("<script>list_args = {{\"class\":\"WP_Post_Comments_List_Table\",\"screen\":{{\"id\":\"{screen_id}\",\"base\":\"post\"}}}};</script>\n"));
    }
    out
}

/// `natcasesort()`: natural order, case-insensitive.
pub fn natcasesort(keys: &mut [String]) {
    fn chunks(s: &str) -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.to_lowercase().chars() {
            let digit = c.is_ascii_digit();
            match out.last_mut() {
                Some((d, text)) if *d == digit => text.push(c),
                _ => out.push((digit, c.to_string())),
            }
        }
        out
    }
    keys.sort_by(|a, b| {
        let (ca, cb) = (chunks(a), chunks(b));
        for (x, y) in ca.iter().zip(cb.iter()) {
            let ord = if x.0 && y.0 {
                let (nx, ny) = (x.1.trim_start_matches('0'), y.1.trim_start_matches('0'));
                nx.len().cmp(&ny.len()).then(nx.cmp(ny))
            } else {
                x.1.cmp(&y.1)
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        ca.len().cmp(&cb.len())
    });
}

/// `is_protected_meta()`.
pub fn is_protected_meta(key: &str) -> bool {
    key.trim().starts_with('_')
}

/// `_list_meta_row()`: one custom field of the box; empty for a protected key and for a serialized array or object.
pub fn list_meta_row(ctx: &Ctx<'_>, id: u64, key: &str, raw: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    if is_protected_meta(key) {
        return String::new();
    }
    // A serialized array or object is not shown; a serialized string is.
    let value = match rustpress_phpser::unserialize(raw.as_bytes()) {
        Ok(rustpress_phpser::Value::Str(s)) => String::from_utf8_lossy(&s).into_owned(),
        Ok(_) if raw.trim() != "N;" && !raw.trim().is_empty() && looks_serialized(raw) => return String::new(),
        _ => raw.to_owned(),
    };
    let key = esc_attr(key);
    let update_nonce = ctx.create_nonce("add-meta");
    let delete_nonce = ctx.create_nonce(&format!("delete-meta_{id}"));
    format!(
        "\n\t<tr id='meta-{id}'>\n\t\t<td class='left'><label class='screen-reader-text' for='meta-{id}-key'>{}</label><input name='meta[{id}][key]' id='meta-{id}-key' type='text' size='20' value='{key}' />\n\t\t<div class='submit'>{}\n\t\t{}</div>{}</td>\n\t\t<td><label class='screen-reader-text' for='meta-{id}-value'>{}</label><textarea name='meta[{id}][value]' id='meta-{id}-value' rows='2' cols='30'>{}</textarea></td>\n\t</tr>",
        tr("Key"),
        submit_button(&tr("Delete"), "deletemeta small", &format!("deletemeta[{id}]"), false, None, &[("data-wp-lists", format!("delete:the-list:meta-{id}::_ajax_nonce={delete_nonce}"))]),
        submit_button(&tr("Update"), "updatemeta small", &format!("meta-{id}-submit"), false, None, &[("data-wp-lists", format!("add:the-list:meta-{id}::_ajax_nonce-add-meta={update_nonce}"))]),
        format_args!("<input type=\"hidden\" id=\"_ajax_nonce\" name=\"_ajax_nonce\" value=\"{}\" />", ctx.create_nonce("change-meta")),
        tr("Value"),
        rustpress_content::sanitize::esc_textarea(&value)
    )
}

/// `post_custom_meta_box()` with `list_meta()` and `meta_form()`: `meta` are the post's rows (`has_meta()`), `keys` the
/// keys `meta_form()` offers.
pub fn custom_fields_box(ctx: &Ctx<'_>, meta: &[rustpress_db::PostMeta], keys: &[String]) -> String {
    let tr = |s: &str| ctx.tr(s);
    let name_label = ctx.site().trx("Name", "meta name");
    let mut out = String::from("<div id=\"postcustomstuff\">\n<div id=\"ajax-response\"></div>\n\t");
    // `list_meta()`
    let rows: Vec<&rustpress_db::PostMeta> = meta.iter().filter(|m| !is_protected_meta(m.meta_key.as_deref().unwrap_or_default())).collect();
    if rows.is_empty() {
        out.push_str(&format!(
            "\n<table id=\"list-table\" style=\"display: none;\">\n\t<thead>\n\t<tr>\n\t\t<th class=\"left\">{name_label}</th>\n\t\t<th>{}</th>\n\t</tr>\n\t</thead>\n\t<tbody id=\"the-list\" data-wp-lists=\"list:meta\">\n\t<tr><td></td></tr>\n\t</tbody>\n</table>",
            tr("Value")
        ));
    } else {
        out.push_str(&format!(
            "<table id=\"list-table\">\n\t<thead>\n\t<tr>\n\t\t<th class=\"left\">{name_label}</th>\n\t\t<th>{}</th>\n\t</tr>\n\t</thead>\n\t<tbody id='the-list' data-wp-lists='list:meta'>\n\t",
            tr("Value")
        ));
        for m in rows {
            out.push_str(&list_meta_row(ctx, m.meta_id, m.meta_key.as_deref().unwrap_or_default(), m.meta_value.as_deref().unwrap_or_default()));
        }
        out.push_str("\t</tbody>\n</table>\n\t");
    }
    // `meta_form()`
    out.push_str(&format!(
        "<p><strong>{}</strong></p>\n<table id=\"newmeta\">\n<thead>\n<tr>\n<th class=\"left\"><label for=\"metakeyselect\">{name_label}</label></th>\n<th><label for=\"metavalue\">{}</label></th>\n</tr>\n</thead>\n\n<tbody>\n<tr>\n<td id=\"newmetaleft\" class=\"left\">\n\t",
        tr("Add Custom Field:"),
        tr("Value")
    ));
    if keys.is_empty() {
        out.push_str("<input type=\"text\" id=\"metakeyinput\" name=\"metakeyinput\" value=\"\" />\n");
    } else {
        out.push_str(&format!("<select id=\"metakeyselect\" name=\"metakeyselect\">\n<option value=\"#NONE#\">{}</option>\n\t\t", tr("&mdash; Select &mdash;")));
        for key in keys {
            out.push_str(&format!("\n<option value='{}'>{}</option>", esc_attr(key), esc_html(key)));
        }
        out.push_str(&format!(
            "</select>\n<input class=\"hidden\" type=\"text\" id=\"metakeyinput\" name=\"metakeyinput\" value=\"\" aria-label=\"{}\" />\n<button type=\"button\" id=\"newmeta-button\" class=\"button button-small hide-if-no-js\" onclick=\"jQuery('#metakeyinput, #metakeyselect, #enternew, #cancelnew').toggleClass('hidden');jQuery('#metakeyinput, #metakeyselect').filter(':visible').trigger('focus');\">\n<span id=\"enternew\">{}</span>\n<span id=\"cancelnew\" class=\"hidden\">{}</span></button>\n",
            tr("New custom field name"),
            tr("Enter new"),
            tr("Cancel")
        ));
    }
    out.push_str(&format!(
        "</td>\n<td><textarea id=\"metavalue\" name=\"metavalue\" rows=\"2\" cols=\"25\"></textarea>\n\t<input type=\"hidden\" id=\"_ajax_nonce-add-meta\" name=\"_ajax_nonce-add-meta\" value=\"{}\" /></td>\n</tr>\n</tbody>\n</table>\n<div class=\"submit add-custom-field\">\n\t{}</div>\n\t</div>\n<p>\n\t{}</p>\n\t",
        ctx.create_nonce("add-meta"),
        submit_button(&tr("Add Custom Field"), "", "addmeta", false, Some("newmeta-submit"), &[("data-wp-lists", "add:the-list:newmeta".to_owned())]),
        sprintf(&tr("Custom fields can be used to add extra metadata to a post that you can <a href=\"%s\">use in your theme</a>."), &[&tr("https://wordpress.org/documentation/article/assign-custom-fields/")])
    ));
    out
}

/// `is_serialized()` for a value that unserialized to something other than a string.
fn looks_serialized(raw: &str) -> bool {
    let t = raw.trim();
    t.len() >= 4 && matches!(t.as_bytes()[0], b'a' | b'O' | b's' | b'i' | b'd' | b'b' | b'C' | b'E') && t.as_bytes()[1] == b':'
}
