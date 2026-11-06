//! `media-upload.php`: the upload window from before the media modal (`wp-admin/includes/media.php`), which the
//! featured image link without JavaScript, old plugins and the `TB_iframe` links still open.
//!
//! Differs from WordPress:
//! - the fields of taxonomies on attachments are not listed (WordPress registers none);
//! - the library takes `post_mime_type`, `s`, `m`, `paged`, `attachment-filter`, `detached` and `mine` of the address,
//!   not every argument of `WP_Query`;
//! - the filters of the legacy window (`media_upload_tabs`, `upload_post_params`, `plupload_init`, ...) have no native
//!   counterpart: no module uses them.

use std::collections::BTreeMap;

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_db::{AttachmentQuery, Post};
use serde_json::json;

use super::{AdminResponse, Ctx, die, page_headers, render};
use super::meta_boxes::submit_button;
use crate::chrome::{Assets, FrameKind, Screen};
use crate::content::{Content, PostData};

const URL_SCRIPT: &str = include_str!("../data/media_upload_url.tpl");
const URL_FORM: &str = include_str!("../data/media_upload_url_form.tpl");
const GALLERY: &str = include_str!("../data/media_upload_gallery.tpl");

/// The form of the upload tab in `media_upload_form()`: only what depends on the request is filled in.
const PRELOAD_SCRIPT: &str = "\t<script>\n\tjQuery(function($){\n\t\tvar preloaded = $(\".media-item.preloaded\");\n\t\tif ( preloaded.length > 0 ) {\n\t\t\tpreloaded.each(function(){prepareMediaItem({id:this.id.replace(/[^0-9]/g, '')},'');});\n\t\t}\n\t\tupdateMediaForm();\n\t});\n\t</script>\n";

/// `(int) $value`.
fn php_int(value: &str) -> i64 {
    let value = value.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    let number = digits[..end].parse::<i64>().unwrap_or(0);
    if negative { -number } else { number }
}

/// `get_post( $value )` of an argument: only what `is_numeric()` accepts is an ID.
fn numeric_id(value: &str) -> i64 {
    static NUMERIC: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"^[ \t\n\r\x0b\x0c]*[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?[ \t\n\r\x0b\x0c]*$").expect("valid pattern"));
    if !NUMERIC.is_match(value) {
        return 0;
    }
    value.trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']).parse::<f64>().map_or(0, |id| id as i64)
}

/// `add_magic_quotes()` of a value of `$_GET`.
fn slashed(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\'' | '"' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            '\0' => out.push_str("\\0"),
            _ => out.push(c),
        }
    }
    out
}

/// `empty( $value )` of a string.
fn php_empty(value: &str) -> bool {
    value.is_empty() || value == "0"
}

/// The globals of `media-upload.php`.
struct Request {
    /// `$type`.
    kind: String,
    /// `$tab`.
    tab: String,
    /// `(int) $_REQUEST['post_id']`.
    post_id: i64,
    /// `isset( $_REQUEST['post_id'] )`.
    has_post_id: bool,
    /// `array_key_exists()` in `media_upload_tabs()`, with their texts.
    tabs: Vec<(&'static str, String)>,
}


/// `media_upload_tabs()` with `update_gallery_tab()`.
async fn upload_tabs(ctx: &Ctx<'_>) -> Vec<(&'static str, String)> {
    let mut tabs = vec![("type", ctx.tr("From Computer")), ("type_url", ctx.tr("From URL"))];
    if let Some(post_id) = ctx.request.request("post_id").map(php_int).filter(|id| *id > 0) {
        let attachments = ctx.env.db.count_attached(post_id as u64).await.unwrap_or(0);
        if attachments > 0 {
            tabs.push(("gallery", sprintf(&ctx.tr("Gallery (%s)"), &[&format!("<span id='attachments-count'>{attachments}</span>")])));
        }
    }
    tabs.push(("library", ctx.tr("Media Library")));
    tabs
}

/// `the_media_upload_tabs()`.
fn tab_list(ctx: &Ctx<'_>, page: &Request, redirected_tab: Option<&str>) -> String {
    let current = redirected_tab.or(ctx.request.get("tab")).filter(|tab| page.tabs.iter().any(|(key, _)| key == tab)).unwrap_or("type");
    let mut out = "<ul id='sidemenu'>\n".to_owned();
    for (key, text) in &page.tabs {
        let class = if current == *key { " class='current'" } else { "" };
        let mut href = rustpress_core::urls::add_query_arg(&ctx.request.request_uri, "tab", key);
        for arg in ["s", "paged", "post_mime_type", "m"] {
            href = rustpress_core::urls::remove_query_arg(&href, arg);
        }
        out.push_str(&format!("\t<li id='{}'><a href='{}'{class}>{text}</a></li>\n", esc_attr(&format!("tab-{key}")), esc_url(&href)));
    }
    out.push_str("</ul>\n");
    out
}

/// `media_upload_header()`.
fn header(ctx: &Ctx<'_>, page: &Request, redirected_tab: Option<&str>) -> String {
    let mut out = format!("<script>post_id = {};</script>", page.post_id);
    if ctx.request.get("chromeless").is_none_or(php_empty) {
        out.push_str(&format!("<div id=\"media-upload-header\">{}</div>", tab_list(ctx, page, redirected_tab)));
    }
    out
}

fn form_class(base: &str, html_uploader: bool) -> String {
    if html_uploader { format!("{base} html-uploader") } else { base.to_owned() }
}

/// `wp_required_field_indicator()`.
fn required_indicator(ctx: &Ctx<'_>) -> String {
    format!("<span class=\"required\">{}</span>", esc_html(&ctx.tr("*")))
}

/// `wp_required_field_message()`.
fn required_message(ctx: &Ctx<'_>) -> String {
    format!("<span class=\"required-field-message\">{}</span>", sprintf(&ctx.tr("Required fields are marked %s"), &[&required_indicator(ctx)]))
}

/// `wp_json_encode( $value, JSON_HEX_TAG | JSON_UNESCAPED_SLASHES )`.
fn json_hex_tag(value: &serde_json::Value) -> String {
    let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: true, unescaped_unicode: false };
    rustpress_blocks::php_json::encode_with(value, flags).replace('<', "\\u003C").replace('>', "\\u003E")
}

/// `media_upload_form()`.
fn upload_form(ctx: &Ctx<'_>, page: &Request, upload_error: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let max = super::media_ui::max_upload();
    let init = json!({
        "browse_button": "plupload-browse-button",
        "container": "plupload-upload-ui",
        "drop_element": "drag-drop-area",
        "file_data_name": "async-upload",
        "url": ctx.admin_url("async-upload.php"),
        "filters": {"max_file_size": format!("{max}b")},
        "multipart_params": {"post_id": page.post_id, "_wpnonce": ctx.create_nonce("media-form"), "type": page.kind, "tab": page.tab, "short": "1"},
    });
    // `absint()` of the options; `0` falls back to 1024.
    let size = |name: &str| match php_int(&site.options.string(name, "")).unsigned_abs() {
        0 => 1024,
        size => size,
    };
    let mut h = format!("\t\t<div id=\"media-upload-notice\">\n\t\t</div>\n\t<div id=\"media-upload-error\">\n\t{upload_error}\t</div>\n\t\t");
    h.push_str(&format!(
        "<script>\n\t\tvar resize_height = {}, resize_width = {},\n\twpUploaderInit = {};\n\t</script>\n\n",
        size("large_size_h"),
        size("large_size_w"),
        json_hex_tag(&init)
    ));
    h.push_str(&format!(
        "\t<div id=\"plupload-upload-ui\" class=\"hide-if-no-js\">\n\t\t<div id=\"drag-drop-area\">\n\t\t<div class=\"drag-drop-inside\">\n\t\t<p class=\"drag-drop-info\">{}</p>\n\t\t<p>{}</p>\n\t\t<p class=\"drag-drop-buttons\"><input id=\"plupload-browse-button\" type=\"button\" value=\"{}\" class=\"button\" /></p>\n\t\t</div>\n\t</div>\n\t\t<p class=\"upload-flash-bypass\">\n\t{}\t</p>\n\t\t</div>\n\n",
        tr("Drop files to upload"),
        ctx.env.site.trx("or", "Uploader: Drop files here - or - Select Files"),
        esc_attr(&tr("Select Files")),
        sprintf(&tr("You are using the multi-file uploader. Problems? Try the <button %s>browser uploader</button> instead."), &["type=\"button\" class=\"button-link\""])
    ));
    let upload = ctx.env.site.trx("Upload", "verb");
    h.push_str(&format!(
        "\t<div id=\"html-upload-ui\" class=\"hide-if-js\">\n\t\t<p id=\"async-upload-wrap\">\n\t\t<label class=\"screen-reader-text\" for=\"async-upload\">\n\t\t\t{upload}\t\t</label>\n\t\t<input type=\"file\" name=\"async-upload\" id=\"async-upload\" />\n\t\t{button}\t\t<a href=\"#\" onclick=\"try{{top.tb_remove();}}catch(e){{}}; return false;\">{cancel}</a>\n\t</p>\n\t<div class=\"clear\"></div>\n\t\t<p class=\"upload-html-bypass hide-if-no-js\">\n\t{bypass}\t</p>\n\t\t</div>\n\n",
        button = submit_button(&upload, "primary", "html-upload", false, None, &[]),
        cancel = tr("Cancel"),
        bypass = sprintf(&tr("You are using the browser&#8217;s built-in file uploader. The WordPress uploader includes multiple file selection and drag and drop capability. <button %s>Switch to the multi-file uploader</button>."), &["type=\"button\" class=\"button-link\""]),
    ));
    h.push_str(&format!("<p class=\"max-upload-size\">\n\t{}</p>\n\t", sprintf(&tr("Maximum upload file size: %s."), &[&esc_html(&super::ajax_media::size_format(max))])));
    h
}

/// `media_upload_type_form()`.
async fn type_form(ctx: &Ctx<'_>, page: &Request, upload_error: &str, id: u64) -> String {
    let tr = |s: &str| ctx.tr(s);
    let kind = "image";
    let action = ctx.admin_url(&format!("media-upload.php?type={kind}&tab=type&post_id={}", page.post_id));
    let class = form_class("media-upload-form type-form validate", super::common::user_setting(ctx, "uploader").await.is_some_and(|v| !php_empty(&v)));
    let mut h = header(ctx, page, None);
    h.push_str(&format!("\t<form enctype=\"multipart/form-data\" method=\"post\" action=\"{}\" class=\"{class}\" id=\"{kind}-form\">\n\t\t", esc_url(&action)));
    h.push_str(&submit_button(&tr("Save Changes"), "hidden", "save", false, None, &[]));
    h.push_str(&format!("\t<input type=\"hidden\" name=\"post_id\" id=\"post_id\" value=\"{}\" />\n\t\t{}\n\t<h3 class=\"media-title\">{}</h3>\n\n", page.post_id, ctx.nonce_field("media-form"), tr("Add media files from your computer")));
    h.push_str(&upload_form(ctx, page, upload_error));
    h.push('\n');
    h.push_str(PRELOAD_SCRIPT);
    h.push_str("\t<div id=\"media-items\">\n\t");
    if id != 0 {
        h.push_str(&media_items(ctx, page, id as i64, None).await);
    }
    h.push_str(&format!("\t</div>\n\n\t<p class=\"savebutton ml-submit\">\n\t\t{}\t</p>\n\t</form>\n\t", submit_button(&tr("Save all changes"), "", "save", false, None, &[])));
    h
}

/// `wp_media_insert_url_form()`.
fn insert_url_form(ctx: &Ctx<'_>, kind: &str) -> String {
    let tr = |s: &str| ctx.tr(s);
    let default_align = ctx.site().options.string("image_default_align", "");
    let default_align = if default_align.is_empty() { "none".to_owned() } else { default_align };
    let image = kind == "image";
    let check = |value: &str| if default_align == value { " checked=\"checked\"" } else { "" };
    let radio = |on: bool| if on { " checked='checked'" } else { "" };
    URL_FORM
        .trim_end()
        .replace("{blank}", "\t\t")
        .replace("{image_checked}", radio(image))
        .replace("{generic_checked}", radio(!image))
        .replace("{table_class}", if image { "" } else { "not-image" })
        .replace("{required_message}", &required_message(ctx))
        .replace("{indicator}", &required_indicator(ctx))
        .replace("{image}", &tr("Image"))
        .replace("{generic}", &tr("Audio, Video, or Other File"))
        .replace("{url}", &tr("URL"))
        .replace("{title}", &tr("Title"))
        .replace("{link_text}", &tr("Link text, e.g. &#8220;Ransom Demands (PDF)&#8221;"))
        .replace("{alt}", &tr("Alternative Text"))
        .replace("{alt_help}", &tr("Alt text for the image, e.g. &#8220;The Mona Lisa&#8221;"))
        .replace("{caption}", &tr("Image Caption"))
        .replace("{alignment}", &tr("Alignment"))
        .replace("{checked_none}", check("none"))
        .replace("{checked_left}", check("left"))
        .replace("{checked_center}", check("center"))
        .replace("{checked_right}", check("right"))
        .replace("{none}", &tr("None"))
        .replace("{left}", &tr("Left"))
        .replace("{center}", &tr("Center"))
        .replace("{right}", &tr("Right"))
        .replace("{link_to}", &tr("Link Image To:"))
        .replace("{link_image}", &tr("Link to image"))
        .replace("{link_help}", &tr("Enter a link URL or click above for presets."))
        .replace("{insert_attr}", &esc_attr(&tr("Insert into Post")))
        .replace("{insert_button}", &submit_button(&tr("Insert into Post"), "", "insertonlybutton", false, None, &[]))
}

/// `media_upload_type_url_form()`.
async fn type_url_form(ctx: &Ctx<'_>, page: &Request, kind: &str) -> String {
    let action = ctx.admin_url(&format!("media-upload.php?type={kind}&tab=type&post_id={}", page.post_id));
    let class = form_class("media-upload-form type-form validate", super::common::user_setting(ctx, "uploader").await.is_some_and(|v| !php_empty(&v)));
    let mut h = header(ctx, page, None);
    h.push_str(&format!(
        "\t<form enctype=\"multipart/form-data\" method=\"post\" action=\"{}\" class=\"{class}\" id=\"{kind}-form\">\n\t<input type=\"hidden\" name=\"post_id\" id=\"post_id\" value=\"{}\" />\n\t\t{}\n\t<h3 class=\"media-title\">{}</h3>\n\n",
        esc_url(&action),
        page.post_id,
        ctx.nonce_field("media-form"),
        ctx.tr("Insert media from another website")
    ));
    h.push_str(
        &URL_SCRIPT
            .replace("{no_png}", &esc_url(&ctx.admin_url("images/no.png")))
            .replace("{yes_png}", &esc_url(&ctx.admin_url("images/yes.png")))
            .replace("{spinner}", &esc_url(&ctx.admin_url("images/spinner-2x.gif"))),
    );
    h.push_str("\n\t<div id=\"media-items\">\n\t<div class=\"media-item media-blank\">\n\t");
    h.push_str(&insert_url_form(ctx, kind));
    h.push_str("\t</div>\n\t</div>\n\t</form>\n\t");
    h
}

/// `media_upload_gallery_form()`.
async fn gallery_form(ctx: &Ctx<'_>, page: &Request) -> String {
    let tr = |s: &str| ctx.tr(s);
    let post_id = page.post_id;
    let action = ctx.admin_url(&format!("media-upload.php?type={}&tab=gallery&post_id={post_id}", page.kind));
    let class = form_class("media-upload-form validate", super::common::user_setting(ctx, "uploader").await.is_some_and(|v| !php_empty(&v)));
    let items = media_items(ctx, page, post_id, Some("gallery")).await;
    let save = submit_button(&tr("Save all changes"), "savebutton", "save", false, Some("save-all"), &[("style", "display: none;".to_owned())]);
    let mut h = header(ctx, page, Some("gallery"));
    h.push_str(
        &GALLERY
            .replace("{action}", &esc_url(&action))
            .replace("{form_class}", &class)
            .replace("{nonce_field}", &ctx.nonce_field("media-form"))
            .replace("{all_tabs}", &tr("All Tabs:"))
            .replace("{show}", &tr("Show"))
            .replace("{hide}", &tr("Hide"))
            .replace("{sort_order}", &tr("Sort Order:"))
            .replace("{ascending}", &tr("Ascending"))
            .replace("{descending}", &tr("Descending"))
            .replace("{clear}", &ctx.env.site.trx("Clear", "verb"))
            .replace("{media}", &tr("Media"))
            .replace("{order}", &tr("Order"))
            .replace("{actions}", &tr("Actions"))
            .replace("{save_button}", &save)
            .replace("{post_id}", &post_id.to_string())
            .replace("{type}", &esc_attr(&page.kind))
            .replace("{tab}", &esc_attr(&page.tab))
            .replace("{gallery_settings}", &tr("Gallery Settings"))
            .replace("{link_thumbnails}", &tr("Link thumbnails to:"))
            .replace("{image_file}", &tr("Image File"))
            .replace("{attachment_page}", &tr("Attachment Page"))
            .replace("{order_images}", &tr("Order images by:"))
            .replace("{menu_order}", &tr("Menu order"))
            .replace("{title}", &tr("Title"))
            .replace("{date_time}", &tr("Date/Time"))
            .replace("{random}", &tr("Random"))
            .replace("{order_colon}", &tr("Order:"))
            .replace("{columns}", &tr("Gallery columns:"))
            .replace("{insert_gallery}", &esc_attr(&tr("Insert gallery")))
            .replace("{update_gallery}", &esc_attr(&tr("Update gallery settings")))
            .replace("{items}", &items),
    );
    h.push('\t');
    h
}

/// `get_post_mime_types()`: `(key, singular, plural)`. Three groups are named by the list of their types.
pub(super) fn mime_groups() -> [(&'static str, &'static str, &'static str); 6] {
    [
        ("image", "Image <span class=\"count\">(%s)</span>", "Images <span class=\"count\">(%s)</span>"),
        ("audio", "Audio <span class=\"count\">(%s)</span>", "Audio <span class=\"count\">(%s)</span>"),
        ("video", "Video <span class=\"count\">(%s)</span>", "Video <span class=\"count\">(%s)</span>"),
        (DOCUMENT_TYPES, "Document <span class=\"count\">(%s)</span>", "Documents <span class=\"count\">(%s)</span>"),
        (SPREADSHEET_TYPES, "Spreadsheet <span class=\"count\">(%s)</span>", "Spreadsheets <span class=\"count\">(%s)</span>"),
        (ARCHIVE_TYPES, "Archive <span class=\"count\">(%s)</span>", "Archives <span class=\"count\">(%s)</span>"),
    ]
}

const DOCUMENT_TYPES: &str = "application/msword,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/vnd.ms-word.document.macroEnabled.12,application/vnd.ms-word.template.macroEnabled.12,application/vnd.oasis.opendocument.text,application/vnd.apple.pages,application/pdf,application/vnd.ms-xpsdocument,application/oxps,application/rtf,application/wordperfect,application/octet-stream";
const SPREADSHEET_TYPES: &str = "application/vnd.apple.numbers,application/vnd.oasis.opendocument.spreadsheet,application/vnd.ms-excel,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.ms-excel.sheet.macroEnabled.12,application/vnd.ms-excel.sheet.binary.macroEnabled.12";
const ARCHIVE_TYPES: &str = "application/x-gzip,application/rar,application/x-tar,application/zip,application/x-7z-compressed";

/// `wp_match_mime_types()` for one group: a type matches a part as a whole, a part without `/` anywhere in the type.
pub(super) fn group_matches(group: &str, mime: &str) -> bool {
    group.split(',').map(str::trim).any(|part| part == mime || (!part.contains('/') && mime.contains(part)))
}

/// The first label of a group of `get_post_mime_types()`.
pub(super) fn group_label(ctx: &Ctx<'_>, key: &str) -> String {
    match key {
        "image" => ctx.tr("Images"),
        "audio" => ctx.env.site.trx("Audio", "file type group"),
        "video" => ctx.env.site.trx("Video", "file type group"),
        DOCUMENT_TYPES => ctx.tr("Documents"),
        SPREADSHEET_TYPES => ctx.tr("Spreadsheets"),
        _ => ctx.env.site.trx("Archives", "file type group"),
    }
}

/// `wp_edit_attachments_query()` as `media_upload_library_form()` calls it, the first time.
struct LibraryQuery {
    query: AttachmentQuery,
    paged: u64,
}

/// `wp_edit_attachments_query_vars()`: what the address asks for, without its limits.
fn query_vars(ctx: &Ctx<'_>, mime: Option<&str>) -> AttachmentQuery {
    let request = ctx.request;
    let filter = request.get("attachment-filter").unwrap_or_default();
    let known = |value: &str| mime_groups().iter().any(|(key, ..)| *key == value);
    // `post_mime_type` must be a key of the groups.
    let mut query = AttachmentQuery { orderby: "date".to_owned(), desc: true, ..Default::default() };
    query.mime = mime.filter(|m| known(m)).map(str::to_owned);
    if let Some(key) = filter.strip_prefix("post_mime_type:").filter(|key| known(key)) {
        query.mime = Some(key.to_owned());
    }
    if request.get("detached").is_some() || filter == "detached" {
        query.parent = Some(0);
    }
    if request.get("mine").is_some() || filter == "mine" {
        query.author = Some(ctx.user.id);
    }
    query.search = request.get("s").map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    if let Some(search) = &query.search {
        query.search_terms = rustpress_core::query::parse_search_terms(search, false);
        query.relevance = request.get("orderby").is_none_or(str::is_empty);
    }
    let month = request.get("m").map_or(0, php_int);
    if month >= 190_000 {
        query.year = Some((month / 100) as u32);
        query.month = Some((month % 100) as u32);
    }
    query
}

/// `media_upload_library_form()`.
async fn library_form(ctx: &Ctx<'_>, page: &Request) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let request = ctx.request;
    let db = ctx.env.db;
    let post_id = page.post_id;
    let action = ctx.admin_url(&format!("media-upload.php?type={}&tab=library&post_id={post_id}", page.kind));
    let class = form_class("media-upload-form validate", super::common::user_setting(ctx, "uploader").await.is_some_and(|v| !php_empty(&v)));
    let per_page = super::common::items_per_page(ctx, "upload_per_page", 20).await;
    let paged = request.get("paged").map_or(0, php_int).max(1) as u64;

    let counts = db.attachment_mime_counts().await.unwrap_or_default();
    let count_of = |group: &str| counts.iter().filter(|(mime, _)| group_matches(group, mime)).map(|(_, n)| *n).sum::<u64>();
    let available = db.attachment_mime_types().await.unwrap_or_default();

    // The first query keeps `offset` and the `upload_per_page` of `wp_edit_attachments_query_vars()`.
    let wanted = request.get("post_mime_type").unwrap_or_default();
    let mut library = LibraryQuery { query: AttachmentQuery { limit: per_page, offset: (paged - 1) * 10, ..query_vars(ctx, Some(wanted).filter(|m| !m.is_empty())) }, paged };
    // "If available type specified by media button clicked, filter by that type."
    let mut mime_selected = request.get("post_mime_type").map(str::to_owned);
    if php_empty(wanted) && mime_groups().iter().any(|(key, ..)| *key == page.kind) && count_of(&page.kind) > 0 {
        mime_selected = Some(page.kind.clone());
        library.query = AttachmentQuery { limit: per_page, offset: (paged - 1) * per_page, ..query_vars(ctx, Some(&page.kind)) };
    }
    let (items, found) = db.list_attachments(&library.query).await.unwrap_or_default();
    let found = if items.is_empty() { 0 } else { found };

    let mut h = header(ctx, page, None);
    h.push_str(&format!(
        "\t<form id=\"filter\" method=\"get\">\n\t<input type=\"hidden\" name=\"type\" value=\"{}\" />\n\t<input type=\"hidden\" name=\"tab\" value=\"{}\" />\n\t<input type=\"hidden\" name=\"post_id\" value=\"{post_id}\" />\n\t<input type=\"hidden\" name=\"post_mime_type\" value=\"{}\" />\n\t<input type=\"hidden\" name=\"context\" value=\"{}\" />\n\n",
        esc_attr(&page.kind),
        esc_attr(&page.tab),
        esc_attr(&slashed(wanted)),
        esc_attr(&slashed(request.get("context").unwrap_or_default())),
    ));
    h.push_str(&format!(
        "\t<p id=\"media-search\" class=\"search-box\">\n\t\t<label class=\"screen-reader-text\" for=\"media-search-input\">\n\t\t\t{}\t\t</label>\n\t\t<input type=\"search\" id=\"media-search-input\" name=\"s\" value=\"{}\" />\n\t\t{}\t</p>\n\n",
        tr("Search Media:"),
        esc_attr(request.get("s").unwrap_or_default()),
        submit_button(&tr("Search Media"), "", "", false, None, &[]),
    ));

    // The links of the types.
    let mut links = Vec::new();
    let all_class = if mime_selected.as_deref().is_none_or(|m| php_empty(m) || m == "all") { " class=\"current\"" } else { "" };
    let mut all_href = rustpress_core::urls::add_query_arg(&request.request_uri, "post_mime_type", "all");
    for arg in ["paged", "m"] {
        all_href = rustpress_core::urls::remove_query_arg(&all_href, arg);
    }
    links.push(format!("<li><a href=\"{}\"{all_class}>{}</a>", esc_url(&all_href), tr("All Types")));
    for (key, singular, plural) in mime_groups() {
        if !available.iter().any(|mime| group_matches(key, mime)) {
            continue;
        }
        let current = mime_selected.as_deref().is_some_and(|m| m.split(',').any(|m| group_matches(key, m.trim())));
        let class = if current { " class=\"current\"" } else { "" };
        // A value that is added stays as it is; every further call would encode it.
        let href = rustpress_core::urls::add_query_arg(&rustpress_core::urls::remove_query_arg(&request.request_uri, "paged"), "post_mime_type", key);
        let count = count_of(key);
        let number = format!("<span id=\"{key}-counter\">{}</span>", site.number_i18n(count));
        links.push(format!("<li><a href=\"{}\"{class}>{}</a>", esc_url(&href), sprintf(&site.trn(singular, plural, count as i64), &[&number])));
    }
    h.push_str(&format!("\t<ul class=\"subsubsub\">\n\t\t{}</li>\t</ul>\n\n\t<div class=\"tablenav\">\n\n\t\t", links.join(" | </li>")));

    if let Some(pages) = page_links(ctx, found.div_ceil(10), library.paged) {
        h.push_str(&format!("<div class='tablenav-pages'>{pages}</div>"));
    }
    h.push_str("\n\t<div class=\"alignleft actions\">\n\t\t");
    let months = db.attachment_months().await.unwrap_or_default();
    let selected_month = request.get("m").map_or(0, php_int);
    if !months.is_empty() && (months.len() != 1 || months[0].1 != 0) {
        h.push_str(&format!("\t\t\t<select name='m'>\n\t\t\t\t<option{} value='0'>{}</option>\n\t\t\t", if selected_month == 0 { " selected='selected'" } else { "" }, tr("All dates")));
        for (year, month) in &months {
            if *year == 0 {
                continue;
            }
            let value = format!("{year}{month:02}");
            let name = site.locale_data().month[(*month as usize).saturating_sub(1).min(11)].clone();
            h.push_str(&format!(
                "<option {} value='{value}'>{}</option>\n",
                if selected_month.to_string() == value { " selected='selected'" } else { "" },
                esc_html(&sprintf(&tr("%1$s %2$d"), &[&name, &year.to_string()]))
            ));
        }
        h.push_str("\t\t\t</select>\n\t\t");
    }
    h.push_str(&format!(
        "\n\t\t{}\n\t</div>\n\n\t<br class=\"clear\" />\n\t</div>\n\t</form>\n\n\t<form enctype=\"multipart/form-data\" method=\"post\" action=\"{}\" class=\"{class}\" id=\"library-form\">\n\t{}\n\t",
        submit_button(&tr("Filter &#187;"), "compact", "post-query-submit", false, None, &[]),
        esc_url(&action),
        ctx.nonce_field("media-form"),
    ));
    h.push_str("<script>\n\tjQuery(function($){\n\t\tvar preloaded = $(\".media-item.preloaded\");\n\t\tif ( preloaded.length > 0 ) {\n\t\t\tpreloaded.each(function(){prepareMediaItem({id:this.id.replace(/[^0-9]/g, '')},'');});\n\t\t\tupdateMediaForm();\n\t\t}\n\t});\n\t</script>\n\n\t<div id=\"media-items\">\n\t\t\t\t");
    h.push_str(&items_html(ctx, page, &items, None).await);
    h.push_str(&format!(
        "\t</div>\n\t<p class=\"ml-submit\">\n\t\t{}\t<input type=\"hidden\" name=\"post_id\" id=\"post_id\" value=\"{post_id}\" />\n\t</p>\n\t</form>\n\t",
        submit_button(&tr("Save all changes"), "savebutton", "save", false, None, &[])
    ));
    h
}

/// `paginate_links()` with the arguments of `media_upload_library_form()`.
fn page_links(ctx: &Ctx<'_>, total: u64, current: u64) -> Option<String> {
    if total < 2 {
        return None;
    }
    let uri = &ctx.request.request_uri;
    let link = |n: u64| esc_url(&rustpress_core::urls::add_query_arg(uri, "paged", &n.to_string()));
    let number = |n: u64| ctx.site().number_i18n(n);
    let mut out = Vec::new();
    if current > 1 {
        out.push(format!("<a class=\"prev page-numbers\" href=\"{}\">{}</a>", link(current - 1), ctx.tr("&laquo;")));
    }
    let mut dots = false;
    for n in 1..=total {
        if n == current {
            out.push(format!("<span aria-current=\"page\" class=\"page-numbers current\">{}</span>", number(n)));
            dots = true;
        } else if n <= 1 || (n + 2 >= current && n <= current + 2) || n > total - 1 {
            out.push(format!("<a class=\"page-numbers\" href=\"{}\">{}</a>", link(n), number(n)));
            dots = true;
        } else if dots {
            out.push("<span class=\"page-numbers dots\">&hellip;</span>".to_owned());
            dots = false;
        }
    }
    if current < total {
        out.push(format!("<a class=\"next page-numbers\" href=\"{}\">{}</a>", link(current + 1), ctx.tr("&raquo;")));
    }
    Some(out.join("\n"))
}

/// `get_media_items( $post_id, $errors )` for a post (`$post_id` being an attachment or its parent).
async fn media_items(ctx: &Ctx<'_>, page: &Request, post_id: i64, tab: Option<&str>) -> String {
    if post_id <= 0 {
        return String::new();
    }
    let Ok(Some(post)) = ctx.env.db.post(post_id as u64).await else { return String::new() };
    let attachments = if post.post_type == "attachment" { vec![post] } else { ctx.env.db.attached_posts(post_id as u64).await.unwrap_or_default() };
    items_html(ctx, page, &attachments, tab).await
}

/// The loop of `get_media_items()`.
async fn items_html(ctx: &Ctx<'_>, page: &Request, attachments: &[Post], tab: Option<&str>) -> String {
    let mut out = String::new();
    for attachment in attachments.iter().filter(|a| a.post_status != "trash") {
        let id = attachment.id;
        let item = media_item(ctx, page, attachment, tab).await;
        out.push_str(&format!(
            "\n<div id='media-item-{id}' class='media-item child-of-{} preloaded'><div class='progress hidden'><div class='bar'></div></div><div id='media-upload-error-{id}' class='hidden'></div><div class='filename hidden'></div>{item}\n</div>",
            attachment.post_parent
        ));
    }
    out
}

/// `wp_html_excerpt( $str, 60, '&hellip;' )`.
fn html_excerpt(text: &str) -> String {
    static ENTITY_END: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"&[^;\s]{0,6}$").expect("valid pattern"));
    let text = rustpress_core::formatting::wp_strip_all_tags(text);
    let excerpt: String = text.chars().take(60).collect();
    let cut = ENTITY_END.replace(&excerpt, "").into_owned();
    if cut != text { format!("{cut}&hellip;") } else { cut }
}

/// `mysql2date( __( 'F j, Y' ), $post->post_date )`.
fn upload_date(ctx: &Ctx<'_>, post: &Post) -> String {
    let format = ctx.tr("F j, Y");
    rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date).map_or_else(String::new, |ts| ctx.site().date_utc(&format, ts))
}

/// `image_align_input_fields()`.
fn align_fields(ctx: &Ctx<'_>, id: u64) -> String {
    let default = ctx.site().options.string("image_default_align", "");
    let checked = if ["none", "left", "center", "right"].contains(&default.as_str()) { default } else { "none".to_owned() };
    [("none", "None"), ("left", "Left"), ("center", "Center"), ("right", "Right")]
        .iter()
        .map(|(name, label)| {
            format!(
                "<input type='radio' name='attachments[{id}][align]' id='image-align-{name}-{id}' value='{name}'{} /><label for='image-align-{name}-{id}' class='align image-align-{name}-label'>{}</label>",
                if checked == *name { " checked='checked'" } else { "" },
                ctx.tr(label)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `image_size_input_fields()`.
fn size_fields(ctx: &Ctx<'_>, id: u64, full: &str, meta: Option<&rustpress_blocks::images::Metadata>, is_image: bool) -> String {
    let default = ctx.site().options.string("image_default_size", "medium");
    let mut check = if default.is_empty() { "medium".to_owned() } else { default };
    let mut output = Vec::new();
    for (size, label) in [("thumbnail", "Thumbnail"), ("medium", "Medium"), ("large", "Large"), ("full", "Full Size")] {
        // `image_downsize()`: `false` for what is no image, `is_intermediate` when the size was generated.
        let downsized = meta.filter(|_| is_image).map(|m| rustpress_blocks::images::image_downsize(full, m, size));
        let intermediate = meta.and_then(|m| m.sizes()).is_some_and(|sizes| sizes.get(size).is_some()) && is_image;
        let enabled = intermediate || size == "full";
        let (width, height) = downsized.as_ref().map_or((0, 0), |d| (d.width, d.height));
        let css_id = format!("image-size-{size}-{id}");
        let mut checked = "";
        if size == check {
            if enabled {
                checked = " checked='checked'";
            } else {
                check.clear();
            }
        } else if check.is_empty() && enabled && size != "thumbnail" {
            check = size.to_owned();
            checked = " checked='checked'";
        }
        let disabled = if enabled { "" } else { " disabled='disabled'" };
        let mut html = format!("<div class='image-size-item'><input type='radio' {disabled}name='attachments[{id}][image-size]' id='{css_id}' value='{size}'{checked} /><label for='{css_id}'>{}</label>", ctx.tr(label));
        if enabled {
            html.push_str(&format!(" <label for='{css_id}' class='help'>({width}&nbsp;&times;&nbsp;{height})</label>"));
        }
        html.push_str("</div>");
        output.push(html);
    }
    output.join("\n")
}

/// `image_link_input_fields()`.
fn link_fields(ctx: &Ctx<'_>, id: u64, file: &str, link: &str, url_type: &str) -> String {
    let url = match url_type {
        "file" => file,
        "post" => link,
        _ => "",
    };
    format!(
        "\n\t<input type='text' class='text urlfield' name='attachments[{id}][url]' value='{}' /><br />\n\t<button type='button' class='button urlnone' data-link-url=''>{}</button>\n\t<button type='button' class='button urlfile' data-link-url='{}'>{}</button>\n\t<button type='button' class='button urlpost' data-link-url='{}'>{}</button>\n",
        esc_attr(url),
        ctx.tr("None"),
        esc_url(file),
        ctx.tr("File URL"),
        esc_url(link),
        ctx.tr("Attachment Post URL")
    )
}

/// A row of `get_media_item()`.
struct Field {
    id: String,
    label: String,
    /// `$field['html']`, or the input of `$field['value']`.
    html: Option<String>,
    value: String,
    textarea: bool,
    required: bool,
    help: Option<String>,
}

impl Field {
    fn text(id: &str, label: String, value: String) -> Self {
        Self { id: id.to_owned(), label, html: None, value, textarea: false, required: false, help: None }
    }

    fn html(id: &str, label: String, html: String) -> Self {
        Self { id: id.to_owned(), label, html: Some(html), value: String::new(), textarea: false, required: false, help: None }
    }

    fn help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }

    fn row(&self, ctx: &Ctx<'_>, attachment: u64) -> String {
        let name = format!("attachments[{attachment}][{}]", self.id);
        let required = if self.required { format!(" {}", required_indicator(ctx)) } else { String::new() };
        let required_attr = if self.required { " required" } else { "" };
        let class = if self.required { format!("{} form-required", self.id) } else { self.id.clone() };
        let mut row = format!("\t\t<tr class='{class}'>\n\t\t\t<th scope='row' class='label'><label for='{name}'><span class='alignleft'>{}{required}</span><br class='clear' /></label></th>\n\t\t\t<td class='field'>", self.label);
        match &self.html {
            Some(html) if !html.is_empty() => row.push_str(html),
            _ if self.textarea => row.push_str(&format!("<textarea id='{name}' name='{name}'{required_attr}>{}</textarea>", self.value)),
            _ => row.push_str(&format!("<input type='text' class='text' id='{name}' name='{name}' value='{}'{required_attr} />", esc_attr(&self.value))),
        }
        if let Some(help) = self.help.as_ref().filter(|h| !h.is_empty()) {
            row.push_str(&format!("<p class='help'>{help}</p>"));
        }
        row.push_str("</td>\n\t\t</tr>\n");
        row
    }
}

/// `get_media_item()` with `get_attachment_fields_to_edit()`.
async fn media_item(ctx: &Ctx<'_>, page: &Request, post: &Post, tab: Option<&str>) -> String {
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    let db = ctx.env.db;
    let id = post.id;
    let (meta, attached) = rustpress_blocks::images::load_metadata(site, post).await.unwrap_or((None, None));
    let attached = attached.unwrap_or_default();
    // `wp_get_attachment_url()`: the GUID of a row without a file; `wp_attachment_is_image()` wants the file.
    let file_url = if attached.is_empty() { post.guid.clone() } else { rustpress_blocks::images::attachment_url(site, &attached) };
    let is_image = !attached.is_empty() && rustpress_blocks::images::is_image(post);
    // `wp_mime_type_icon()`: documents, spreadsheets and archives are told by the extension of the file.
    let by_type = !attached.is_empty() || ["audio/", "video/", "text/"].iter().any(|prefix| post.post_mime_type.starts_with(prefix));
    let icon = if by_type { super::ajax_media::mime_icon(&post.post_mime_type) } else { "default.svg" };
    let icon = format!("{}/wp-includes/images/media/{icon}", site.settings.siteurl.trim_end_matches('/'));
    let thumb_url = match &meta {
        Some(meta) if is_image => rustpress_blocks::images::image_downsize(&file_url, meta, "thumbnail").url,
        None if is_image => file_url.clone(),
        _ => icon,
    };
    let current_post = ctx.request.get("post_id").map_or(0, php_int);
    let current_post = if current_post != 0 { db.post(current_post.unsigned_abs()).await.ok().flatten() } else { None };
    let send = match &current_post {
        Some(post) => site.post_type_supports(&post.post_type, "editor"),
        None => ctx.request.get("post_id").is_none_or(php_empty),
    };
    let filename = esc_html(attached.rsplit('/').next().unwrap_or_default());
    let title = esc_attr(&post.post_title);
    let mime_group = mime_groups().iter().find(|(key, ..)| group_matches(key, &post.post_mime_type)).map_or("", |(key, ..)| *key);
    let display = if title.is_empty() { filename.clone() } else { title.clone() };
    let gallery = tab == Some("gallery") || ctx.request.request("tab") == Some("gallery");
    let order = if gallery {
        format!("<div class='menu_order'> <input class='menu_order_input' type='text' id='attachments[{id}][menu_order]' name='attachments[{id}][menu_order]' value='{}' /></div>", post.menu_order)
    } else {
        format!("<input type='hidden' name='attachments[{id}][menu_order]' value='{}' />", post.menu_order)
    };
    let dims = meta.as_ref().filter(|m| m.0.get("width").is_some() && m.0.get("height").is_some()).map(|m| {
        format!("<span id='media-dims-{id}'>{}</span>", sprintf(&tr("%1$s by %2$s pixels"), &[&m.width().to_string(), &m.height().to_string()]))
    });
    // `wp_image_editor_supports()`: the formats the editor can read and write.
    let image_edit = if is_image && matches!(post.post_mime_type.as_str(), "image/jpeg" | "image/png" | "image/gif" | "image/webp") {
        format!(
            "<input type='button' id='imgedit-open-btn-{id}' onclick='imageEdit.open( {id}, \"{}\" )' class='button' value='{}' /> <span class='spinner'></span>",
            ctx.create_nonce(&format!("image_editor-{id}")),
            esc_attr(&tr("Edit Image"))
        )
    } else {
        String::new()
    };
    let link = site.permalink(post).await.unwrap_or_default();
    let mut item = format!(
        "\n\t\t<input type='hidden' id='type-of-{id}' value='{}' />\n\t\t\n\t\t<a class='toggle describe-toggle-on' href='#'>{}</a>\n\t\t<a class='toggle describe-toggle-off' href='#'>{}</a>\n\t\t{order}\n\t\t<div class='filename new'><span class='title'>{}</span></div>\n\t\t<table class='slidetoggle describe startclosed'>\n\t\t\t<thead class='media-item-info' id='media-head-{id}'>\n\t\t\t<tr>\n\t\t\t<td class='A1B1' id='thumbnail-head-{id}'>\n\t\t\t<p><a href='{link}' target='_blank'><img class='thumbnail' src='{thumb_url}' alt='' /></a></p>\n\t\t\t<p>{image_edit}</p>\n\t\t\t</td>\n\t\t\t<td>\n\t\t\t<p><strong>{}</strong> {filename}</p>\n\t\t\t<p><strong>{}</strong> {}</p>\n\t\t\t<p><strong>{}</strong> {}</p>",
        esc_attr(mime_group),
        tr("Show"),
        tr("Hide"),
        html_excerpt(&display),
        tr("File name:"),
        tr("File type:"),
        post.post_mime_type,
        tr("Upload date:"),
        upload_date(ctx, post),
    );
    if let Some(dims) = &dims {
        item.push_str(&format!("<p><strong>{}</strong> {dims}</p>\n", tr("Dimensions:")));
    }
    item.push_str("</td></tr>\n");
    item.push_str(&format!(
        "\n\t\t</thead>\n\t\t<tbody>\n\t\t<tr><td colspan='2' class='imgedit-response' id='imgedit-response-{id}'></td></tr>\n\n\t\t<tr><td style='display:none' colspan='2' class='image-editor' id='image-editor-{id}'></td></tr>\n\n\t\t<tr><td colspan='2'><p class='media-types media-types-required-info'>{}</p></td></tr>\n",
        required_message(ctx)
    ));

    // `get_attachment_fields_to_edit()`; every form of this page adds `media_post_single_attachment_fields_to_edit()`,
    // which takes `image_url` out again.
    let textarea_escape = |text: &str| super::post_edit::esc_textarea_pub(text);
    let alt = db.post_meta(id).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value).unwrap_or_default();
    let mut title_field = Field::text("post_title", tr("Title"), textarea_escape(&post.post_title));
    let image_prefix = post.post_mime_type.starts_with("image");
    title_field.required = image_prefix;
    let mut fields = vec![title_field];
    if image_prefix {
        fields.push(Field::text("image_alt", tr("Alternative Text"), alt).help(tr("Alt text for the image, e.g. &#8220;The Mona Lisa&#8221;")));
    }
    let excerpt_name = format!("attachments[{id}][post_excerpt]");
    fields.push(Field::html("post_excerpt", tr("Caption"), format!("<textarea name=\"{excerpt_name}\" id=\"{excerpt_name}\">{}</textarea>", textarea_escape(&post.post_excerpt))));
    let mut description = Field::text("post_content", tr("Description"), textarea_escape(&post.post_content));
    description.textarea = true;
    fields.push(description);
    let link_type = site.options.string("image_default_link_type", "");
    fields.push(Field::html("url", tr("Link URL"), link_fields(ctx, id, &file_url, &link, if link_type.is_empty() { "post" } else { &link_type })).help(tr("Enter a link URL or click above for presets.")));
    if image_prefix {
        fields.push(Field::html("align", tr("Alignment"), align_fields(ctx, id)));
        fields.push(Field::html("image-size", tr("Size"), size_fields(ctx, id, &file_url, meta.as_ref(), is_image)));
    }
    // `attachment_fields_to_edit` of the plugins.
    if !site.plugins.is_empty() {
        let parts = ctx.plugin_parts();
        let plugin_ctx = ctx.plugin_context(&parts, "media-upload.php");
        let added = site.plugins.attachment_fields_to_edit(&plugin_ctx, post, false).await.unwrap_or_else(|e| {
            tracing::error!(error = %e, "plugin attachment fields failed");
            Vec::new()
        });
        fields.extend(added.into_iter().filter(|f| !f.id.starts_with('_')).map(|f| Field::html(&f.id, f.label, f.html)));
    }
    for field in &fields {
        item.push_str(&field.row(ctx, id));
    }

    // The buttons.
    let send_button = if send { submit_button(&tr("Insert into Post"), "", &format!("send[{id}]"), false, None, &[]) } else { String::new() };
    let delete = if ctx.user.can_delete_post(post) {
        let url = format!("post.php?action=delete&amp;post={id}&amp;_wpnonce={}", ctx.create_nonce(&format!("delete-post_{id}")));
        format!(
            "<a href='#' class='del-link' onclick=\"document.getElementById('del_attachment_{id}').style.display='block';return false;\">{}</a>\n\t\t\t\t<div id='del_attachment_{id}' class='del-attachment' style='display:none;'><p>{}</p>\n\t\t\t\t<a href='{url}' id='del[{id}]' class='button'>{}</a>\n\t\t\t\t<a href='#' class='button' onclick=\"this.parentNode.style.display='none';return false;\">{}</a>\n\t\t\t\t</div>",
            tr("Delete"),
            sprintf(&tr("You are about to delete %s."), &[&format!("<strong>{filename}</strong>")]),
            tr("Continue"),
            tr("Cancel"),
        )
    } else {
        String::new()
    };
    let calling = match ctx.request.get("post_id") {
        Some(value) => php_int(value).unsigned_abs(),
        None if !ctx.request.form.is_empty() => post.post_parent,
        None => 0,
    };
    let mut thumbnail = String::new();
    if mime_group == "image" && calling != 0
        && let Ok(Some(calling_post)) = db.post(calling).await
        && (ctx.env.block_theme || ctx.env.theme.adds_theme_support("post-thumbnails"))
        && site.post_type_supports(&calling_post.post_type, "thumbnail")
    {
        let featured = db.post_meta(calling).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_thumbnail_id")).and_then(|m| m.meta_value).map_or(0, |v| php_int(&v));
        if featured != id as i64 {
            let label = site.types.admin_post_types().into_iter().find(|t| t.name == calling_post.post_type).and_then(|t| t.label("use_featured_image")).unwrap_or_else(|| tr("Use as featured image"));
            thumbnail = format!("<a class='wp-post-thumbnail' id='wp-post-thumbnail-{id}' href='#' onclick='WPSetAsThumbnail(\"{id}\", \"{}\");return false;'>{}</a>", ctx.create_nonce(&format!("set_post_thumbnail-{calling}")), esc_html(&label));
        }
    }
    if !send_button.is_empty() || !thumbnail.is_empty() || !delete.is_empty() {
        item.push_str(&format!("\t\t<tr class='submit'><td></td><td class='savesend'>{send_button} {thumbnail} {delete}</td></tr>\n"));
    }
    item.push_str("\t</tbody>\n\t</table>\n");
    if post.post_parent < 1 && page.has_post_id {
        let parent_name = format!("attachments[{id}][post_parent]");
        item.push_str(&format!("\t<input type='hidden' name='{parent_name}' id='{parent_name}' value='{}' />\n", page.post_id));
    }
    item
}

/// `wp_iframe()`.
async fn frame(ctx: &Ctx<'_>, content: &str, media_styles: bool, more_scripts: &[&str]) -> AdminResponse {
    let inline = ctx.request.get("inline").is_some();
    let mut scripts = vec!["plupload-handlers".to_owned(), "image-edit".to_owned(), "set-post-thumbnail".to_owned(), "media-gallery".to_owned()];
    scripts.extend(more_scripts.iter().map(|s| (*s).to_owned()));
    let assets = Assets { scripts, styles: vec!["imgareaselect".to_owned()], no_auth_check: !inline, ..Default::default() };
    let kind = if inline { FrameKind::Inline } else { FrameKind::Request };
    let screen = Screen { file: "media-upload.php".into(), title: ctx.tr("Uploads"), frame: Some(kind), frame_media_styles: media_styles, ..Default::default() };
    render(ctx, &screen, content, &assets).await
}

/// `media_send_to_editor()`.
fn send_to_editor(html: &str) -> AdminResponse {
    let body = format!("\t<script>\n\tvar win = window.dialogArguments || opener || parent || top;\n\twin.send_to_editor( {} );\n\t</script>\n\t", json_hex_tag(&serde_json::Value::String(html.to_owned())));
    AdminResponse { status: 200, headers: page_headers(), cookies: Vec::new(), body, bytes: Vec::new() }
}

/// `check_admin_referer( 'media-form' )`.
fn check_referer(ctx: &Ctx<'_>) -> Option<AdminResponse> {
    if ctx.verify_nonce("media-form") {
        return None;
    }
    Some(die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired.")))
}

/// The arguments of a form field `attachments[<id>][<name>]`, per attachment in the order of the form.
fn posted_attachments(ctx: &Ctx<'_>) -> Vec<(u64, BTreeMap<String, String>)> {
    let mut out: Vec<(u64, BTreeMap<String, String>)> = Vec::new();
    for (key, value) in &ctx.request.form {
        let Some(rest) = key.strip_prefix("attachments[") else { continue };
        let Some((id, field)) = rest.split_once("][") else { continue };
        let Some(field) = field.strip_suffix(']') else { continue };
        let id = php_int(id).unsigned_abs();
        match out.iter_mut().find(|(known, _)| *known == id) {
            Some((_, fields)) => {
                fields.insert(field.to_owned(), value.clone());
            }
            None => out.push((id, BTreeMap::from([(field.to_owned(), value.clone())]))),
        }
    }
    out
}

/// `media_upload_form_handler()`: a response when the request ends here.
async fn form_handler(ctx: &Ctx<'_>) -> Option<AdminResponse> {
    if let Some(response) = check_referer(ctx) {
        return Some(response);
    }
    let Some(writer) = ctx.env.writer else { return Some(super::common::read_only(ctx)) };
    let db = ctx.env.db;
    let content = Content { site: ctx.site(), db, writer, user: &ctx.user, now: ctx.env.now };
    // The first key of `$_POST['send']`.
    let send_id = ctx.request.form.iter().find_map(|(key, _)| key.strip_prefix("send[").and_then(|k| k.strip_suffix(']'))).map(|k| php_int(k).unsigned_abs());
    let attachments = posted_attachments(ctx);
    for (id, fields) in &attachments {
        let Ok(Some(post)) = db.post(*id).await else { continue };
        if !ctx.user.can_edit_post(&post) {
            continue;
        }
        let mut data = PostData { id: Some(*id), ..Default::default() };
        let mut changed = false;
        if let Some(value) = fields.get("post_content").filter(|v| **v != post.post_content) {
            data.post_content = Some(value.clone());
            changed = true;
        }
        if let Some(value) = fields.get("post_title").filter(|v| **v != post.post_title) {
            data.post_title = Some(value.clone());
            changed = true;
        }
        if let Some(value) = fields.get("post_excerpt").filter(|v| **v != post.post_excerpt) {
            data.post_excerpt = Some(value.clone());
            changed = true;
        }
        if let Some(value) = fields.get("menu_order").map(|v| php_int(v) as i32).filter(|v| *v != post.menu_order) {
            data.menu_order = Some(value);
            changed = true;
        }
        if send_id == Some(*id)
            && let Some(value) = fields.get("post_parent").map(|v| php_int(v).unsigned_abs()).filter(|v| *v != post.post_parent)
        {
            data.post_parent = Some(value);
            changed = true;
        }
        if let Some(alt) = fields.get("image_alt") {
            let stored = db.post_meta(*id).await.unwrap_or_default().into_iter().find(|m| m.meta_key.as_deref() == Some("_wp_attachment_image_alt")).and_then(|m| m.meta_value);
            if stored.as_deref().unwrap_or_default() != alt {
                // `wp_strip_all_tags( $image_alt, true )`.
                let alt = rustpress_core::formatting::wp_strip_all_tags(alt).split(['\r', '\n', '\t', ' ', '\x0b', '\0']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
                writer.update_post_meta(*id, "_wp_attachment_image_alt", &alt).await.or_log();
            }
        }
        if changed && let Err(error) = content.insert_post(&data).await {
            tracing::error!(error = %error, "saving an attachment failed");
        }
    }
    if ctx.request.post_field("insert-gallery").is_some() || ctx.request.post_field("update-gallery").is_some() {
        let body = "\t\t<script>\n\t\tvar win = window.dialogArguments || opener || parent || top;\n\t\twin.tb_remove();\n\t\t</script>\n\t\t".to_owned();
        return Some(AdminResponse { status: 200, headers: page_headers(), cookies: Vec::new(), body, bytes: Vec::new() });
    }
    let send_id = send_id?;
    let empty = BTreeMap::new();
    let fields = attachments.iter().find(|(id, _)| *id == send_id).map_or(&empty, |(_, fields)| fields);
    let field = |name: &str| fields.get(name).cloned().unwrap_or_default();
    let post = db.post(send_id).await.ok().flatten();
    let permalink = match &post {
        Some(post) => ctx.site().permalink(post).await.ok(),
        None => None,
    };
    let url = field("url");
    let linked = url.contains("attachment_id") || permalink.as_deref() == Some(url.as_str());
    let mut html = field("post_title");
    if !url.is_empty() {
        let rel = if linked { format!(" rel='attachment wp-att-{}'", esc_attr(&send_id.to_string())) } else { String::new() };
        html = format!("<a href='{url}'{rel}>{html}</a>");
    }
    // `image_media_send_to_editor()`.
    if let Some(post) = post.filter(|p| p.post_mime_type.starts_with("image")) {
        let or = |name: &str, default: &str| fields.get(name).filter(|v| !php_empty(v)).cloned().unwrap_or_else(|| default.to_owned());
        html = super::ajax_legacy::image_send_to_editor(ctx.env, &post, &field("post_excerpt"), &or("align", "none"), &url, linked, &or("image-size", "medium"), &or("image_alt", "")).await;
    }
    Some(send_to_editor(&html))
}

/// `wp_basename()`.
fn wp_basename(path: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or_default().to_owned()
}

/// The part of `wp_media_upload_handler()` for `insertonlybutton`.
fn insert_from_url(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let mut src = request.post_field("src").unwrap_or_default().to_owned();
    if !src.is_empty() && src.find("://").is_none_or(|at| at == 0) {
        src = format!("http://{src}");
    }
    let mut html = String::new();
    if request.post_field("media_type").is_some_and(|t| t != "image") {
        let mut title = esc_html(request.post_field("title").unwrap_or_default());
        if title.is_empty() {
            title = esc_html(&wp_basename(&src));
        }
        if !title.is_empty() && !src.is_empty() {
            html = format!("<a href='{}'>{title}</a>", esc_url(&src));
        }
    } else {
        let alt = esc_attr(request.post_field("alt").unwrap_or_default());
        let class = request.post_field("align").map_or_else(String::new, |align| format!(" class='align{}'", esc_attr(align)));
        if !src.is_empty() {
            html = format!("<img src='{}' alt='{alt}'{class} />", esc_url(&src));
        }
    }
    send_to_editor(&html)
}

/// `wp_media_upload_handler()`.
async fn upload_handler(ctx: &Ctx<'_>, page: &Request) -> AdminResponse {
    let request = ctx.request;
    let mut upload_error = String::new();
    let mut id = 0;
    if request.post_field("html-upload").is_some() && !request.files.is_empty() {
        if let Some(response) = check_referer(ctx) {
            return response;
        }
        let Some(writer) = ctx.env.writer else { return super::common::read_only(ctx) };
        let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
        if let Some(file) = request.files.iter().find(|f| f.field == "async-upload") {
            let uploaded = crate::media::UploadedFile { name: file.filename.clone(), bytes: file.bytes.clone(), rest: false, media_form_nonce: true };
            match crate::media::handle_upload(&content, &super::ajax_media::uploads_dir(ctx.env), uploaded, page.post_id.max(0) as u64).await {
                Ok(uploaded) => id = uploaded,
                Err(crate::media::UploadError(message)) => upload_error = message,
            }
        }
    }
    if request.post_field("insertonlybutton").is_some_and(|v| !php_empty(v)) {
        return insert_from_url(ctx);
    }
    // `upload_notice` is set here, but the gallery form never prints it.
    if request.post_field("save").is_some() {
        let content = gallery_form(ctx, page).await;
        return frame(ctx, &content, true, &["admin-gallery"]).await;
    }
    if !request.form.is_empty()
        && let Some(response) = form_handler(ctx).await
    {
        return response;
    }
    if request.get("tab") == Some("type_url") {
        let kind = request.get("type").filter(|t| matches!(*t, "video" | "audio" | "file")).unwrap_or("image");
        let content = type_url_form(ctx, page, kind).await;
        return frame(ctx, &content, true, &[]).await;
    }
    let content = type_form(ctx, page, &upload_error, id).await;
    frame(ctx, &content, true, &[]).await
}

/// `media-upload.php`.
pub async fn media_upload(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    if !ctx.can("upload_files") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &ctx.tr("Sorry, you are not allowed to upload files."));
    }
    if request.request("action") == Some("edit") {
        let html = format!("<h1>{}</h1><p>{}</p>", ctx.tr("An error occurred during the upload process."), ctx.tr("Invalid item ID. You can view all media items in the <a href=\"upload.php\">Media Library</a>."));
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &html);
    }
    if let Some(value) = request.request("post_id").filter(|v| !php_empty(v)) {
        let id = numeric_id(value);
        let post = if id > 0 { ctx.env.db.post(id as u64).await.ok().flatten() } else { None };
        if !post.is_some_and(|post| ctx.user.can_edit_post(&post)) {
            return super::common::forbidden_heading(ctx, "Sorry, you are not allowed to edit this item.");
        }
    }
    let page = Request {
        kind: slashed(request.get("type").unwrap_or("file")),
        tab: slashed(request.get("tab").unwrap_or("type")),
        post_id: request.request("post_id").map_or(0, php_int),
        has_post_id: request.request("post_id").is_some(),
        tabs: upload_tabs(ctx).await,
    };
    let known = page.tabs.iter().any(|(key, _)| *key == page.tab);
    if page.tab == "type" || page.tab == "type_url" || !known {
        // `do_action( "media_upload_{$type}" )`: other types have no handler.
        if !matches!(page.kind.as_str(), "image" | "audio" | "video" | "file") {
            return super::bare_pages::empty();
        }
        return upload_handler(ctx, &page).await;
    }
    if !request.form.is_empty()
        && let Some(response) = form_handler(ctx).await
    {
        return response;
    }
    if page.tab == "gallery" {
        let content = gallery_form(ctx, &page).await;
        return frame(ctx, &content, true, &["admin-gallery"]).await;
    }
    let content = library_form(ctx, &page).await;
    frame(ctx, &content, true, &[]).await
}
