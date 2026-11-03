//! ACF on `post.php` and `post-new.php` (`ACF_Form_Post`, in the block editor with `ACF_Form_Gutenberg`): the scripts
//! of the inputs with the uploader (`acf_enqueue_scripts()` on `load-post.php`), a meta box per field group
//! (`add_meta_boxes()`), the form data, the boxes after the title and the style of `hide_on_screen`
//! (`edit_form_after_title()`, in the block editor on `block_editor_meta_box_hidden_fields`), the hidden editor of
//! the uploader and `acf.data` in the footer. Saving is in [`super::save::save_post`].
//!
//! Differs from the plugin: in the block editor, a user's saved order of the `acf_after_title` boxes is not merged
//! into that of `normal` (`modify_user_option_meta_box_order()`).

use rustpress_core::Result;
use rustpress_core::native::{AdminContext, Assets, FormPart, HiddenEditor, PluginMetaBox, PostMetaBoxes};
use rustpress_db::{MetaKind, Post};
use rustpress_phpser::Value;
use serde_json::{Value as Json, json};

use super::edit_comment::AdminServices;
use super::fields::{FieldGroup, MatchContext, MatchUser, Screen};
use super::form::{FormData, Uploader, form_data};
use super::render::{Ctx, RenderField, render_field_wrap};
use super::value::{Object, get_value};

/// `acf_get_internal_post_types()` with `acf-taxonomy` and attachments: `initialize()` stops for them.
const RESTRICTED: [&str; 5] = ["acf-field-group", "acf-post-type", "acf-taxonomy", "acf-ui-options-page", "attachment"];

/// The post of `post.php`/`post-new.php` when `ACF_Form_Post::initialize()` goes on.
pub fn edited_post<'a>(ctx: &AdminContext<'a>) -> Option<&'a Post> {
    if !matches!(ctx.hook_suffix, "post.php" | "post-new.php") {
        return None;
    }
    ctx.edited_post.filter(|p| !RESTRICTED.contains(&p.post_type.as_str()))
}

/// `acf_enqueue_scripts( [ 'uploader' => true ] )` of the post screen.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if edited_post(ctx).is_some() {
        super::form::enqueue(ctx.site, ctx.wp_version, assets, true);
    }
}

/// `acf_get_field_groups( [ 'post_id' => $post->ID, 'post_type' => $post_type ] )`.
async fn groups_for<'g>(groups: &'g [FieldGroup], ctx: &AdminContext<'_>, post: &Post) -> Result<Vec<&'g FieldGroup>> {
    let screen = Screen::Post { post, post_type: None };
    let matcher = MatchContext { site: ctx.site, user: MatchUser::Admin(ctx.can), admin: true };
    let mut out = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            out.push(group);
        }
    }
    Ok(out)
}

/// `acf_get_field_group_title()` through `acf_esc_html()`.
pub fn group_title(group: &FieldGroup) -> String {
    let display = group.setting("display_title", "");
    let title = if display.is_empty() { group.title.as_str() } else { display.as_str() };
    super::render::esc_html_acf(&super::php::string(&rustpress_core::formatting::esc_html(title)))
}

/// `acf_get_field_group_edit_link()`: for who may administer ACF.
pub fn edit_link(ctx: &AdminContext<'_>, group: &FieldGroup) -> String {
    if group.id != 0 && (ctx.can)("manage_options") { ctx.site.site_url(&format!("wp-admin/post.php?post={}&action=edit", group.id)) } else { String::new() }
}

/// `acf_get_field_group_style()`: hides what `hide_on_screen` lists, and its screen option.
fn group_style(group: &FieldGroup) -> String {
    let elements = [
        ("permalink", "#edit-slug-box"),
        ("the_content", "#postdivrich"),
        ("excerpt", "#postexcerpt"),
        ("custom_fields", "#postcustom"),
        ("discussion", "#commentstatusdiv"),
        ("comments", "#commentsdiv"),
        ("slug", "#slugdiv"),
        ("author", "#authordiv"),
        ("format", "#formatdiv"),
        ("page_attributes", "#pageparentdiv"),
        ("featured_image", "#postimagediv"),
        ("revisions", "#revisionsdiv"),
        ("categories", "#categorydiv"),
        ("tags", "#tagsdiv-post_tag"),
        ("send-trackbacks", "#trackbacksdiv"),
    ];
    let Some(Value::Array(items)) = group.settings.get("hide_on_screen") else { return String::new() };
    let mut hide = Vec::new();
    for (_, k) in items {
        let k = super::php::strval(Some(k));
        if let Some((_, id)) = elements.iter().find(|(name, _)| *name == k) {
            hide.push((*id).to_owned());
            hide.push(format!("#screen-meta label[for={}-hide]", &id[1..]));
        }
    }
    format!("{} {{display: none;}}", hide.join(", "))
}

/// The renderer of the administration (`uploader` is `basic` for who may not upload).
pub(super) fn render_ctx<'a>(ctx: &'a AdminContext<'a>, services: &'a AdminServices) -> Ctx<'a> {
    let uploader = if (ctx.can)("upload_files") { "wp" } else { "basic" };
    let rich_edit = super::form::rich_edit_browser(ctx.user_agent.unwrap_or_default());
    Ctx { site: ctx.site, nonce: ctx.nonce, services, admin: true, uploader, wants_uploader: Default::default(), can: ctx.can, rich_edit, default_editor: if rich_edit { "tinymce" } else { "html" }, wysiwyg_height: 300 }
}

/// `acf_render_fields( $fields, $post_id, 'div', $instruction_placement )`.
pub async fn render_fields(render: &Ctx<'_>, object: Object, group: &FieldGroup) -> Result<String> {
    render_fields_as(render, object, group, "div", &group.setting("instruction_placement", "label")).await
}

/// `acf_render_fields( $fields, $post_id, $el, $instruction )`.
pub async fn render_fields_as(render: &Ctx<'_>, object: Object, group: &FieldGroup, el: &str, instruction: &str) -> Result<String> {
    let mut out = String::new();
    for field in &group.fields {
        let mut f = RenderField::load(field, render);
        if matches!(f.get("value"), None | Some(Value::Null)) {
            f.set("value", get_value(&render.site.db, object, field, &field.name).await?);
        }
        out.push_str(&render_field_wrap(render, f, el, instruction).await?);
    }
    Ok(out)
}

/// `ACF_Form_Post::add_meta_boxes()`, with `edit_form_after_title()` where the editor calls it.
pub async fn meta_boxes(groups: &[FieldGroup], ctx: &AdminContext<'_>, post: &Post) -> Result<PostMetaBoxes> {
    if edited_post(ctx).is_none() {
        return Ok(PostMetaBoxes::default());
    }
    let visible = groups_for(groups, ctx, post).await?;
    let services = AdminServices(ctx.wp_root.to_owned());
    let render = render_ctx(ctx, &services);
    let object = Object { kind: MetaKind::Post, id: post.id };
    let mut boxes = Vec::new();
    let mut after_title = Vec::new();
    for group in &visible {
        let id = rustpress_core::formatting::esc_attr(&format!("acf-{}", group.key));
        let context = rustpress_core::formatting::esc_attr(&group.setting("position", "normal"));
        let priority = if context == "side" { "core" } else { "high" };
        let html = render_fields(&render, object, group).await?;
        let meta_box = PluginMetaBox { id, title: group_title(group), context: context.clone(), priority: priority.into(), html };
        // `ACF_Form_Gutenberg::filter_block_editor_meta_boxes()`: the boxes after the title go first into `normal`.
        if ctx.block_editor && context == "acf_after_title" {
            after_title.push(PluginMetaBox { context: "normal".into(), ..meta_box });
        } else {
            boxes.push(meta_box);
        }
    }
    after_title.extend(boxes);
    let style = visible.first().map(|g| group_style(g)).unwrap_or_default();
    let parts = vec![
        FormPart::Html(form_data("post", Value::Int(post.id as i64), &(ctx.nonce)("post"))),
        FormPart::MetaBoxes("acf_after_title".into()),
        FormPart::Html(format!("<style type=\"text/css\" id=\"acf-style\">{}</style>", rustpress_core::formatting::wp_strip_all_tags(&style))),
    ];
    let (after_title_parts, hidden_fields) = if ctx.block_editor { (Vec::new(), parts) } else { (parts, Vec::new()) };
    Ok(PostMetaBoxes {
        boxes: after_title,
        // `acf_get_setting( 'remove_wp_meta_box' )`.
        removed: vec!["postcustom".into()],
        after_title: after_title_parts,
        hidden_fields,
        // `ACF_Assets::print_uploader_scripts()`.
        hidden_editor: Some(HiddenEditor {
            id: "acf_content".into(),
            before: "\t\t<div id=\"acf-hidden-wp-editor\" style=\"display: none;\">\n\t\t\t".into(),
            after: "\t\t</div>\n\t\t\t".into(),
        }),
    })
}

/// `ACF_Assets::print_footer_scripts()` of the post screen: the form data, the boxes (`postboxes`) and the toolbars
/// of the uploader.
pub async fn footer_scripts(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Result<Option<String>> {
    let Some(post) = edited_post(ctx) else { return Ok(None) };
    let visible = groups_for(groups, ctx, post).await?;
    let postboxes = (!visible.is_empty()).then(|| {
        Json::Array(
            visible
                .iter()
                .map(|g| {
                    let esc = rustpress_core::formatting::esc_attr;
                    json!({"id": esc(&format!("acf-{}", g.key)), "key": esc(&g.key), "style": esc(&g.setting("style", "default")), "label": esc(&g.setting("label_placement", "top")), "edit": rustpress_core::formatting::esc_url(&edit_link(ctx, g))})
                })
                .collect(),
        )
    });
    let form = FormData { screen: Json::String("post".into()), post_id: json!(post.id), validation: Json::Bool(true), postboxes, block_editor: ctx.block_editor, bare: false };
    Ok(Some(super::form::footer(ctx.site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, Uploader::Enqueue)))
}
