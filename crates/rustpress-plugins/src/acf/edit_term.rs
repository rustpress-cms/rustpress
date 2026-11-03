//! ACF on `edit-tags.php` and `term.php` (`acf_form_taxonomy`): the scripts of the inputs (`acf_enqueue_scripts()` on
//! `admin_enqueue_scripts`), the fields of the add form (`add_term()` on `{$taxonomy}_add_form_fields`) and of the
//! edit form (`edit_term()` on `{$taxonomy}_edit_form`), the script of `admin_footer()` and `acf.data` in the footer.
//! Saving is `save_term()` on `create_term`/`edit_term` ([`super::save::save_term`]).

use rustpress_core::Result;
use rustpress_core::native::{AdminContext, Assets, MetaBoxes};
use rustpress_db::{MetaKind, Term};
use serde_json::Value as Json;

use super::edit_comment::{AdminServices, uploader_boxes, wants_uploader};
use super::edit_post::{group_title, render_ctx, render_fields_as};
use super::fields::{FieldGroup, MatchContext, MatchUser, Screen};
use super::form::{FormData, Uploader, form_data};
use super::value::Object;

/// `acf_form_taxonomy::validate_page()`: `$pagenow` is `edit-tags.php` or `term.php`.
pub fn is_term_screen(ctx: &AdminContext<'_>) -> bool {
    matches!(ctx.hook_suffix, "edit-tags.php" | "term.php")
}

/// `acf_form_taxonomy::admin_enqueue_scripts()`.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if is_term_screen(ctx) {
        super::form::enqueue(ctx.site, ctx.wp_version, assets, true);
    }
}

/// `acf_get_field_groups( [ 'taxonomy' => $taxonomy ] )`.
pub(super) async fn groups_for<'g>(groups: &'g [FieldGroup], ctx: &AdminContext<'_>, taxonomy: &str) -> Result<Vec<&'g FieldGroup>> {
    let screen = Screen::Term { taxonomy };
    let matcher = MatchContext { site: ctx.site, user: MatchUser::Admin(ctx.can), admin: true };
    let mut out = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            out.push(group);
        }
    }
    Ok(out)
}

/// `acf_form_taxonomy::add_term()` (without a term) and `edit_term()`.
pub async fn term_form(groups: &[FieldGroup], ctx: &AdminContext<'_>, taxonomy: &str, term: Option<&Term>) -> Result<MetaBoxes> {
    if !is_term_screen(ctx) {
        return Ok(MetaBoxes::default());
    }
    let visible = groups_for(groups, ctx, taxonomy).await?;
    if visible.is_empty() {
        return Ok(MetaBoxes::default());
    }
    let id = term.map_or(0, |t| t.term_id);
    let services = AdminServices(ctx.wp_root.to_owned());
    let render = render_ctx(ctx, &services);
    let object = Object { kind: MetaKind::Term, id };
    let mut out = form_data("taxonomy", super::php::string(&format!("term_{id}")), &(ctx.nonce)("taxonomy"));
    if term.is_none() {
        out.push_str("<div id=\"acf-term-fields\" class=\"acf-fields -clear\">");
        for group in &visible {
            out.push_str(&render_fields_as(&render, object, group, "div", "field").await?);
        }
        out.push_str("</div>");
    } else {
        for group in &visible {
            if group.setting("style", "default") == "default" {
                out.push_str(&format!("<h2>{}</h2>", group_title(group)));
            }
            out.push_str("<table class=\"form-table\">");
            out.push_str(&render_fields_as(&render, object, group, "tr", "field").await?);
            out.push_str("</table>");
        }
    }
    Ok(uploader_boxes(out, ctx, &visible))
}

/// `acf_form_taxonomy::admin_footer()`: the spinner of the submit button and, on the add form, the validation of the
/// Ajax request and the reset of the fields after it.
pub fn admin_footer(ctx: &AdminContext<'_>) -> String {
    if !is_term_screen(ctx) {
        return String::new();
    }
    // `$this->view` is `add` until `edit_term()` ran.
    let view = if ctx.edited_term.is_some() && ctx.hook_suffix == "term.php" { "edit" } else { "add" };
    let mut out = format!(
        "<script type=\"text/javascript\">\n(function($) {{\n\t\n\t// Define vars.\n\tvar view = '{view}';\n\tvar $form = $('#' + view + 'tag');\n\tvar $submit = $('#' + view + 'tag input[type=\"submit\"]:last');\n\t\n\t// Add missing spinner.\n\tif( !$submit.next('.spinner').length ) {{\n\t\t$submit.after('<span class=\"spinner\"></span>');\n\t}}\n\t\n\t\t\t"
    );
    if view == "add" {
        out.push_str("\t\n\t// vars\n\tvar $fields = $('#acf-term-fields');\n\tvar html = '';\n\t\n\t// Store a copy of the $fields html used later to replace after AJAX request.\n\t// Hook into 'prepare' action to allow ACF core helpers to first modify DOM.\n\t// Fixes issue where hidden #acf-hidden-wp-editor is initialized again.\n\tacf.addAction('prepare', function(){\n\t\thtml = $fields.html();\n\t}, 6);\n\t\t\n\t// WP triggers click as primary action\n\t$submit.on('click', function( e ){\n\t\t\n\t\t// validate\n\t\tvar valid = acf.validateForm({\n\t\t\tform: $form,\n\t\t\tevent: e,\n\t\t\treset: true\n\t\t});\n\t\t\n\t\t// if not valid, stop event and allow validation to continue\n\t\tif( !valid ) {\n\t\t\te.preventDefault();\n\t\t\te.stopImmediatePropagation();\n\t\t}\n\t});\n\t\n\t// listen to AJAX add-tag complete\n\t$(document).ajaxComplete(function(event, xhr, settings) {\n\t\t\n\t\t// bail early if is other ajax call\n\t\tif( settings.data.indexOf('action=add-tag') == -1 ) {\n\t\t\treturn;\n\t\t}\n\t\t\n\t\t// bail early if response contains error\n\t\tif( xhr.responseText.indexOf('wp_error') !== -1 ) {\n\t\t\treturn;\n\t\t}\n\t\t\n\t\t// action for 3rd party customization\n\t\tacf.doAction('remove', $fields);\n\t\t\n\t\t// reset HTML\n\t\t$fields.html( html );\n\t\t\n\t\t// action for 3rd party customization\n\t\tacf.doAction('append', $fields);\n\t\t\n\t\t// reset unload\n\t\tacf.unload.reset();\n\t});\n\t\n\t\t");
    }
    out.push_str("\t\n})(jQuery);\t\n</script>\n\t\t\t");
    out
}

/// `ACF_Assets::print_footer_scripts()` of the term screens: the form data of the fields, if there were any.
pub async fn footer_scripts(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Result<Option<String>> {
    if !is_term_screen(ctx) {
        return Ok(None);
    }
    let mut form = FormData::default();
    let mut uploader = false;
    if let Some(taxonomy) = ctx.taxonomy {
        let visible = groups_for(groups, ctx, taxonomy).await?;
        if !visible.is_empty() {
            let id = ctx.edited_term.map_or(0, |t| t.term_id);
            form = FormData { screen: Json::String("taxonomy".into()), post_id: Json::String(format!("term_{id}")), validation: Json::Bool(true), ..Default::default() };
            uploader = wants_uploader(&visible, (ctx.can)("upload_files"));
        }
    }
    Ok(Some(super::form::footer(ctx.site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, if uploader { Uploader::Render } else { Uploader::No })))
}
