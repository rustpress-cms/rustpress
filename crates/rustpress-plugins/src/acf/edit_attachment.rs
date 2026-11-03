//! ACF on `upload.php` and the attachment's `post.php` (`acf_form_attachment`): the scripts of the inputs with the
//! uploader (`acf_enqueue_scripts( [ 'uploader' => true ] )`), the form data of the media library
//! (`admin_footer()` at priority 0 of `upload.php`), the fields of `edit_attachment()` on `attachment_fields_to_edit`
//! (the edit page and the media modal) and `acf.data`. Saving is `save_attachment()` ([`super::save::save_attachment`]).

use rustpress_core::Result;
use rustpress_core::native::{AdminContext, Assets, AttachmentField, HiddenEditor, MetaBoxes};
use rustpress_db::{MetaKind, Post};
use rustpress_phpser::Value;
use serde_json::{Value as Json, json};

use super::edit_comment::AdminServices;
use super::edit_post::render_ctx;
use super::fields::{FieldGroup, MatchContext, MatchUser, Screen};
use super::form::{FormData, Uploader, form_data};
use super::render::{RenderField, render_field_wrap};
use super::value::{Object, get_value};

/// `acf_is_screen( 'attachment' )`: the edit page of an attachment.
fn edited_attachment<'a>(ctx: &AdminContext<'a>) -> Option<&'a Post> {
    (ctx.hook_suffix == "post.php").then_some(ctx.edited_post).flatten().filter(|p| p.post_type == "attachment")
}

/// `acf_is_screen( 'upload' )`.
fn is_upload(ctx: &AdminContext<'_>) -> bool {
    ctx.hook_suffix == "upload.php"
}

/// `acf_form_attachment::admin_enqueue_scripts()`.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if edited_attachment(ctx).is_none() && !is_upload(ctx) {
        return;
    }
    let mut own = Assets::default();
    super::form::enqueue(ctx.site, ctx.wp_version, &mut own, true);
    // `ACF_Assets::enqueue_uploader()` right after the inputs: `wp_enqueue_media()` for who may upload.
    if (ctx.can)("upload_files") {
        let mut media = Assets::default();
        for handle in ["media-editor", "media-audiovideo", "mce-view", "image-edit", "imgareaselect"] {
            media.core_script(handle, true);
        }
        for handle in ["media-views", "wp-mediaelement", "imgareaselect"] {
            media.core_style(handle);
        }
        for asset in media.scripts.iter_mut().chain(media.styles.iter_mut()) {
            asset.priority = 20;
        }
        let at = own.scripts.iter().position(|a| a.handle == "acf-input").map_or(own.scripts.len(), |i| i + 1);
        own.scripts.splice(at..at, media.scripts);
        let at = own.styles.iter().position(|a| a.handle == "acf-input").map_or(own.styles.len(), |i| i + 1);
        own.styles.splice(at..at, media.styles);
    }
    assets.scripts.extend(own.scripts);
    assets.styles.extend(own.styles);
}

/// `acf_enqueue_uploader()` of the two screens: the media library for who may upload and the hidden editor on
/// `admin_footer`, after the form data `upload.php` prints at priority 0.
pub fn screen_uploader(ctx: &AdminContext<'_>) -> MetaBoxes {
    if edited_attachment(ctx).is_none() && !is_upload(ctx) {
        return MetaBoxes::default();
    }
    let mut before = String::new();
    if is_upload(ctx) {
        before.push_str(&form_data("attachment", Value::Int(0), &(ctx.nonce)("attachment")));
        before.push_str("<script type=\"text/javascript\">\n\t\n// WP saves attachment on any input change, so unload is not needed\nacf.unload.active = 0;\n\n</script>\n\t\t\t");
    }
    before.push_str("\t\t<div id=\"acf-hidden-wp-editor\" style=\"display: none;\">\n\t\t\t");
    MetaBoxes { html: String::new(), media: (ctx.can)("upload_files"), hidden_editor: Some(HiddenEditor { id: "acf_content".into(), before, after: "\t\t</div>\n\t\t\t".into() }) }
}

/// `acf_get_field_groups( [ 'attachment_id' => $id, 'attachment' => $id ] )`.
async fn groups_for<'g>(groups: &'g [FieldGroup], ctx: &AdminContext<'_>, attachment: &Post) -> Result<Vec<&'g FieldGroup>> {
    let screen = Screen::Attachment { post: attachment };
    let matcher = MatchContext { site: ctx.site, user: MatchUser::Admin(ctx.can), admin: true };
    let mut out = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            out.push(group);
        }
    }
    Ok(out)
}

/// `acf_form_attachment::edit_attachment()`: the form data and the rows of the fields as the field `acf-form-data`,
/// which closes the row WordPress opens for it and opens one it closes. In the modal the instructions are at the field.
pub async fn attachment_fields_to_edit(groups: &[FieldGroup], ctx: &AdminContext<'_>, attachment: &Post, in_modal: bool) -> Result<Vec<AttachmentField>> {
    let visible = groups_for(groups, ctx, attachment).await?;
    if visible.is_empty() {
        return Ok(Vec::new());
    }
    let services = AdminServices(ctx.wp_root.to_owned());
    let render = render_ctx(ctx, &services);
    let object = Object { kind: MetaKind::Post, id: attachment.id };
    let mut html = form_data("attachment", Value::Int(attachment.id as i64), &(ctx.nonce)("attachment"));
    html.push_str("</td></tr>");
    for group in &visible {
        let instruction = if in_modal { "field".to_owned() } else { group.setting("instruction_placement", "label") };
        for field in &group.fields {
            let mut f = RenderField::load(field, &render);
            if matches!(f.get("value"), None | Some(Value::Null)) {
                f.set("value", get_value(&render.site.db, object, field, &field.name).await?);
            }
            html.push_str(&render_field_wrap(&render, f, "tr", &instruction).await?);
        }
    }
    html.push_str("<tr class=\"compat-field-acf-blank\"><td>");
    Ok(vec![AttachmentField { id: "acf-form-data".into(), label: String::new(), html }])
}

/// `ACF_Media::get_source_field()`: the field whose uploader opened the media modal (`query[_acfuploader]`).
fn source_field<'g>(groups: &'g [FieldGroup], ctx: &AdminContext<'_>) -> Option<&'g super::fields::Field> {
    let key = ctx.form.iter().rev().find(|(k, _)| k == "query[_acfuploader]").map(|(_, v)| v.as_str())?;
    let key = rustpress_content::sanitize::text_field(key);
    fn find<'a>(fields: &'a [super::fields::Field], key: &str) -> Option<&'a super::fields::Field> {
        fields.iter().find_map(|f| if f.key == key { Some(f) } else { find(&f.sub_fields, key) })
    }
    groups.iter().find_map(|g| find(&g.fields, &key))
}

/// `ACF_Media::image_size_names_choose()`: the field's preview size comes along.
pub fn query_attachments_sizes(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Vec<String> {
    let Some(field) = source_field(groups, ctx) else { return Vec::new() };
    field.get("preview_size").map(|v| super::php::strval(Some(v))).into_iter().collect()
}

/// `ACF_Media::clear_acf_errors_for_core_requests()` (priority 5) and, for the modal of a field,
/// `wp_prepare_attachment_for_js()` with `acf_validate_attachment( $response, $field, 'prepare' )`.
pub fn query_attachment_js(groups: &[FieldGroup], ctx: &AdminContext<'_>, response: &mut serde_json::Map<String, Json>) {
    let Some(field) = source_field(groups, ctx) else {
        response.insert("acf_errors".into(), Json::Bool(false));
        return;
    };
    let errors = validate_attachment(ctx.site, response, field);
    response.insert("acf_errors".into(), if errors.is_empty() { Json::Bool(false) } else { Json::String(errors.join("<br />")) });
}

/// `acf_get_filesize()`: a size in MB, or with its unit (`123 KB`).
fn acf_get_filesize(size: &Value) -> f64 {
    let units = [("TB", 4), ("GB", 3), ("MB", 2), ("KB", 1)];
    let mut unit = 2;
    let mut number = size.clone();
    if let Value::Str(bytes) = size {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let custom = text.get(text.len().saturating_sub(2)..).unwrap_or_default().to_uppercase();
        if let Some((_, v)) = units.iter().find(|(k, _)| *k == custom) {
            unit = *v;
            number = super::php::string(&text[..text.len() - 2]);
        }
    }
    super::save::floatval(&number) * 1024f64.powi(unit)
}

/// `acf_format_filesize()`.
fn acf_format_filesize(size: &Value) -> String {
    let bytes = acf_get_filesize(size);
    for (k, v) in [("TB", 4), ("GB", 3), ("MB", 2), ("KB", 1)] {
        let result = bytes / 1024f64.powi(v);
        if result >= 1.0 {
            return format!("{} {k}", super::php::float_string(result));
        }
    }
    format!("{} B", super::php::float_string(bytes))
}

/// `acf_validate_attachment( $attachment, $field, 'prepare' )`: the size, dimensions and type the field allows.
fn validate_attachment(site: &rustpress_core::Site, response: &serde_json::Map<String, Json>, field: &super::fields::Field) -> Vec<String> {
    let tr = |text: &str| site.trd("acf", text);
    let path = response.get("filename").or_else(|| response.get("url")).and_then(Json::as_str).unwrap_or_default();
    let ext = std::path::Path::new(path).extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let number = |key: &str| response.get(key).and_then(Json::as_f64).unwrap_or(0.0);
    let (size, width, height) = (number("filesizeInBytes"), number("width"), number("height"));
    let setting = |key: &str| field.get(key).cloned().unwrap_or(Value::Int(0));
    let int = |key: &str| super::php::intval(Some(&setting(key)));
    let mut errors = Vec::new();
    if width != 0.0 || height != 0.0 {
        let (min_width, max_width) = (int("min_width"), int("max_width"));
        if width != 0.0 {
            if min_width != 0 && width < min_width as f64 {
                errors.push(rustpress_core::i18n::sprintf(&tr("Image width must be at least %dpx."), &[&min_width.to_string()]));
            } else if max_width != 0 && width > max_width as f64 {
                errors.push(rustpress_core::i18n::sprintf(&tr("Image width must not exceed %dpx."), &[&max_width.to_string()]));
            }
        }
        let (min_height, max_height) = (int("min_height"), int("max_height"));
        if height != 0.0 {
            if min_height != 0 && height < min_height as f64 {
                errors.push(rustpress_core::i18n::sprintf(&tr("Image height must be at least %dpx."), &[&min_height.to_string()]));
            } else if max_height != 0 && height > max_height as f64 {
                errors.push(rustpress_core::i18n::sprintf(&tr("Image height must not exceed %dpx."), &[&max_height.to_string()]));
            }
        }
    }
    if size != 0.0 {
        let (min_size, max_size) = (setting("min_size"), setting("max_size"));
        if super::php::truthy(Some(&min_size)) && size < acf_get_filesize(&min_size) {
            errors.push(rustpress_core::i18n::sprintf(&tr("File size must be at least %s."), &[&acf_format_filesize(&min_size)]));
        } else if super::php::truthy(Some(&max_size)) && size > acf_get_filesize(&max_size) {
            errors.push(rustpress_core::i18n::sprintf(&tr("File size must not exceed %s."), &[&acf_format_filesize(&max_size)]));
        }
    }
    if !ext.is_empty() {
        let types = super::php::strval(field.get("mime_types")).to_lowercase().replace([' ', '.'], "");
        let mut types: Vec<String> = types.split(',').filter(|t| !t.is_empty()).map(str::to_owned).collect();
        if !types.is_empty() && !types.contains(&ext.to_lowercase()) {
            if types.len() > 1 {
                let last1 = types.pop().unwrap_or_default();
                let last2 = types.pop().unwrap_or_default();
                types.push(format!("{last2} {} {last1}", tr("or")));
            }
            errors.push(rustpress_core::i18n::sprintf(&tr("File type must be %s."), &[&types.join(", ")]));
        }
    }
    // `acf_validate_is_image_attachment()` on `acf/validate_attachment/type=image`: `wp_attachment_is_image()`, a file
    // whose MIME type is an image.
    if matches!(field.kind.as_str(), "image" | "gallery") {
        let mime = response.get("mime").and_then(Json::as_str).unwrap_or_default();
        let has_file = response.get("filename").and_then(Json::as_str).is_some_and(|f| !f.is_empty());
        if !(has_file && mime.starts_with("image/")) {
            errors.push(tr("File must be a valid image."));
        }
    }
    errors
}

/// `ACF_Assets::print_footer_scripts()` of the two screens: the form data (the attachment of the edit page, `0` in the
/// media library) and the toolbars of the uploader.
pub async fn footer_scripts(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Result<Option<String>> {
    let form = if let Some(attachment) = edited_attachment(ctx) {
        if groups_for(groups, ctx, attachment).await?.is_empty() {
            FormData::default()
        } else {
            FormData { screen: Json::String("attachment".into()), post_id: json!(attachment.id), validation: Json::Bool(true), ..Default::default() }
        }
    } else if is_upload(ctx) {
        FormData { screen: Json::String("attachment".into()), post_id: json!(0), validation: Json::Bool(true), ..Default::default() }
    } else {
        return Ok(None);
    };
    Ok(Some(super::form::footer(ctx.site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, Uploader::Enqueue)))
}
