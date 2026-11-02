//! ACF on `comment.php` (`acf_form_comment`): the scripts of the inputs (`acf_enqueue_scripts()` on
//! `admin_enqueue_scripts`), a box per field group in the second column (`edit_comment()` on
//! `add_meta_boxes_comment`), the spinner of `admin_footer()` and `acf.data` in the footer. Saving goes through
//! `edit_comment`, like the website's `comment_post` ([`super::save`]).

use rustpress_core::native::{AdminContext, Assets, HiddenEditor, MetaBoxes};
use rustpress_core::Result;
use rustpress_db::{Comment, MetaKind};
use rustpress_phpser::Value;
use serde_json::Value as Json;

use super::fields::{FieldGroup, MatchContext, Screen};
use super::form::{FormData, form_data};
use super::render::{Ctx, RenderField, render_field_wrap};
use super::value::{Object, get_value};

/// `acf_form_comment::validate_page()`: `$pagenow` is `comment.php`.
pub fn is_comment_screen(ctx: &AdminContext<'_>) -> bool {
    ctx.hook_suffix == "comment.php"
}

/// `acf_enqueue_scripts()` of the comment screen.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if is_comment_screen(ctx) {
        super::form::enqueue(ctx.site, ctx.wp_version, assets, true);
    }
}

/// The comment of the request (`?c=`).
async fn requested_comment(ctx: &AdminContext<'_>) -> Result<Option<Comment>> {
    let Some(id) = ctx.get("c").or_else(|| ctx.post_field("comment_ID")).and_then(|v| v.trim().parse::<u64>().ok()) else { return Ok(None) };
    Ok(ctx.site.db.comment_any(id).await?)
}

/// `acf_get_field_groups( [ 'comment' => get_post_type( $comment->comment_post_ID ) ] )`.
async fn groups_for<'g>(groups: &'g [FieldGroup], ctx: &AdminContext<'_>, comment: &Comment) -> Result<Vec<&'g FieldGroup>> {
    let post_type = ctx.site.db.post(comment.comment_post_id).await?.map(|p| p.post_type).unwrap_or_default();
    let screen = Screen::Comment { post_type };
    let matcher = MatchContext { site: ctx.site, viewer: None };
    let mut out = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            out.push(group);
        }
    }
    Ok(out)
}

/// `acf_form_comment::edit_comment()`.
pub async fn meta_boxes(groups: &[FieldGroup], ctx: &AdminContext<'_>, comment: &Comment) -> Result<MetaBoxes> {
    let visible = groups_for(groups, ctx, comment).await?;
    if visible.is_empty() {
        return Ok(MetaBoxes::default());
    }
    let post_id = format!("comment_{}", comment.comment_id);
    let uploader = if (ctx.can)("upload_files") { "wp" } else { "basic" };
    let rich_edit = super::form::rich_edit_browser(ctx.user_agent.unwrap_or_default());
    let services = AdminServices(ctx.wp_root.to_owned());
    let render = Ctx { site: ctx.site, nonce: ctx.nonce, services: &services, admin: true, uploader, wants_uploader: Default::default(), can: ctx.can, rich_edit, default_editor: if rich_edit { "tinymce" } else { "html" }, wysiwyg_height: 300 };
    let mut out = form_data("comment", super::php::string(&post_id), &(ctx.nonce)("comment"));
    let object = Object { kind: MetaKind::Comment, id: comment.comment_id };
    for group in visible.iter().copied() {
        let mut fields = String::new();
        for field in &group.fields {
            let mut f = RenderField::load(field, &render);
            if matches!(f.get("value"), None | Some(Value::Null)) {
                f.set("value", get_value(&ctx.site.db, object, field, &field.name).await?);
            }
            fields.push_str(&render_field_wrap(&render, f, "div", &group.setting("instruction_placement", "label")).await?);
        }
        let edit_url = if group.id != 0 && (ctx.can)("manage_options") { ctx.site.site_url(&format!("wp-admin/post.php?post={}&action=edit", group.id)) } else { String::new() };
        let mut o = serde_json::Map::new();
        o.insert("id".into(), Json::String(format!("acf-{}", group.id)));
        o.insert("key".into(), Json::String(group.key.clone()));
        o.insert("label".into(), Json::String(group.setting("label_placement", "top")));
        o.insert("edit_url".into(), Json::String(edit_url));
        o.insert("edit_title".into(), Json::String(ctx.site.trd("acf", "Edit field group")));
        let json = rustpress_blocks::php_json::encode_with(&Json::Object(o), rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: false });
        out.push_str(&format!(
            "\t\t\t\t\t<div id=\"acf-{}\" class=\"stuffbox\">\n\t\t\t\t\t\t<h3 class=\"hndle\">{}</h3>\n\t\t\t\t\t\t<div class=\"inside\">\n\t\t\t\t\t\t\t{fields}\t\t\t\t\t\t\t<script type=\"text/javascript\">\n\t\t\t\t\t\t\tif( typeof acf !== 'undefined' ) {{\n\t\t\t\t\t\t\t\tacf.newPostbox({json});\n\t\t\t\t\t\t\t}}\n\t\t\t\t\t\t\t</script>\n\t\t\t\t\t\t</div>\n\t\t\t\t\t</div>\n\t\t\t\t\t",
            group.id,
            super::render::esc_html_acf(&super::php::string(&rustpress_core::formatting::esc_html(&group.title)))
        ));
    }
    Ok(uploader_boxes(out, ctx, &visible))
}

/// The fields that call `acf_enqueue_uploader()` while they are rendered: images, files and icons with the media
/// library, links and WYSIWYG editors always (sub fields of groups included).
fn wants_uploader(groups: &[&FieldGroup], media_library: bool) -> bool {
    fn walk(fields: &[super::fields::Field], media_library: bool) -> bool {
        fields.iter().any(|f| matches!(f.kind.as_str(), "link" | "wysiwyg") || (media_library && matches!(f.kind.as_str(), "image" | "file" | "icon_picker")) || walk(&f.sub_fields, media_library))
    }
    groups.iter().any(|g| walk(&g.fields, media_library))
}

/// `ACF_Assets::enqueue_uploader()`: `wp_enqueue_media()` for who may upload, and the hidden editor of
/// `print_uploader_scripts()` on `admin_footer`.
fn uploader_boxes(html: String, ctx: &AdminContext<'_>, groups: &[&FieldGroup]) -> MetaBoxes {
    let can_upload = (ctx.can)("upload_files");
    if !wants_uploader(groups, can_upload) {
        return MetaBoxes { html, ..MetaBoxes::default() };
    }
    let hidden = HiddenEditor { id: "acf_content".into(), before: "\t\t<div id=\"acf-hidden-wp-editor\" style=\"display: none;\">\n\t\t\t".into(), after: "\t\t</div>\n\t\t\t".into() };
    MetaBoxes { html, media: can_upload, hidden_editor: Some(hidden) }
}

/// `acf_form_comment::admin_footer()`.
pub fn admin_footer(ctx: &AdminContext<'_>) -> String {
    if !is_comment_screen(ctx) {
        return String::new();
    }
    "<script type=\"text/javascript\">\n(function($) {\n\t\n\t// vars\n\tvar $spinner = $('#publishing-action .spinner');\n\t\n\t\n\t// create spinner if not exists (may exist in future WP versions)\n\tif( !$spinner.exists() ) {\n\t\t\n\t\t// create spinner\n\t\t$spinner = $('<span class=\"spinner\"></span>');\n\t\t\n\t\t\n\t\t// append\n\t\t$('#publishing-action').prepend( $spinner );\n\t\t\n\t}\n\t\n})(jQuery);\t\n</script>\n\t\t\t".to_owned()
}

/// `ACF_Assets::print_footer_scripts()` of the comment screen: the form data of the boxes, if there were any.
pub async fn footer_scripts(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Result<String> {
    if !is_comment_screen(ctx) {
        return Ok(String::new());
    }
    let mut form = FormData { screen: Json::Null, post_id: Json::Null, validation: Json::Null };
    let mut uploader = false;
    if ctx.get("action") == Some("editcomment")
        && let Some(comment) = requested_comment(ctx).await?
        && !groups_for(groups, ctx, &comment).await?.is_empty()
    {
        form = FormData { screen: Json::String("comment".into()), post_id: Json::String(format!("comment_{}", comment.comment_id)), validation: Json::Bool(true) };
        uploader = wants_uploader(&groups_for(groups, ctx, &comment).await?, (ctx.can)("upload_files"));
    }
    Ok(super::form::footer(ctx.site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, uploader))
}

/// What the renderer asks of the page: nothing in the administration but the WordPress files.
struct AdminServices(std::path::PathBuf);

impl rustpress_core::native::PageServices for AdminServices {
    fn excerpt<'a>(&'a self, _post: &'a rustpress_db::Post) -> futures::future::BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    fn wp_root(&self) -> &std::path::Path {
        &self.0
    }
}
