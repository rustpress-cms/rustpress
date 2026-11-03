//! ACF on `profile.php`, `user-edit.php` and `user-new.php` (`ACF_Form_User`): the scripts of the inputs
//! (`acf_enqueue_scripts()` on `admin_enqueue_scripts`), the fields at the end of the form (`render_edit()` on
//! `show_user_profile`/`edit_user_profile`, `render_new()` on `user_new_form`), the script of `admin_footer()` on
//! `acf/input/admin_footer` and `acf.data`. Saving is `save_user()` ([`super::save::save_user`]).
//!
//! Differs from the plugin: the registration form of `wp-login.php` (`render_register()`,
//! `filter_registration_errors()`) is not ported, RustPress has no registration.

use rustpress_core::Result;
use rustpress_core::native::{AdminContext, Assets, MetaBoxes};
use rustpress_db::MetaKind;
use rustpress_phpser::Value;
use serde_json::Value as Json;

use super::edit_comment::{AdminServices, uploader_boxes, wants_uploader};
use super::edit_post::{group_title, render_ctx};
use super::fields::{FieldGroup, MatchContext, MatchUser, Screen};
use super::form::{FormData, Uploader, form_data};
use super::render::{RenderField, render_field_wrap};
use super::value::{Object, get_value};

/// `acf_is_screen( [ 'profile', 'user', 'user-edit' ] )`.
pub fn is_user_screen(ctx: &AdminContext<'_>) -> bool {
    matches!(ctx.hook_suffix, "profile.php" | "user-edit.php" | "user-new.php")
}

/// `ACF_Form_User::admin_enqueue_scripts()`.
pub fn admin_enqueue(ctx: &AdminContext<'_>, assets: &mut Assets) {
    if is_user_screen(ctx) {
        super::form::enqueue(ctx.site, ctx.wp_version, assets, true);
    }
}

/// The user of the screen: the signed-in one on `profile.php`, `?user_id=` on `user-edit.php`, none (`new`) on
/// `user-new.php`.
fn screen_user(ctx: &AdminContext<'_>) -> Option<u64> {
    match ctx.hook_suffix {
        "profile.php" => Some(ctx.user),
        "user-edit.php" => ctx.get("user_id").or_else(|| ctx.post_field("user_id")).and_then(|v| v.trim().parse().ok()),
        _ => None,
    }
}

/// `acf_get_field_groups( [ 'user_id' => $id ?: 'new', 'user_form' => $view ] )`.
async fn groups_for<'g>(groups: &'g [FieldGroup], ctx: &AdminContext<'_>, user_id: Option<u64>) -> Result<Vec<&'g FieldGroup>> {
    let view = if user_id.is_some() { "edit" } else { "add" };
    let screen = Screen::User { user_id, form: Some(view) };
    let matcher = MatchContext { site: ctx.site, user: MatchUser::Admin(ctx.can), admin: true };
    let mut out = Vec::new();
    for group in groups {
        if group.visible(&screen, &matcher).await? {
            out.push(group);
        }
    }
    Ok(out)
}

/// `ACF_Form_User::render()` of `render_edit()` (with a user) and `render_new()`.
pub async fn user_form(groups: &[FieldGroup], ctx: &AdminContext<'_>, user_id: Option<u64>) -> Result<MetaBoxes> {
    if !is_user_screen(ctx) {
        return Ok(MetaBoxes::default());
    }
    let visible = groups_for(groups, ctx, user_id).await?;
    if visible.is_empty() {
        return Ok(MetaBoxes::default());
    }
    let id = user_id.unwrap_or(0);
    let services = AdminServices(ctx.wp_root.to_owned());
    let render = render_ctx(ctx, &services);
    let object = Object { kind: MetaKind::User, id };
    // `filter_pre_load_value()`: a form sent again shows the values it sent.
    let posted = match rustpress_core::native::form_value(ctx.form, "acf") {
        Some(Value::Array(items)) => Some(items),
        _ => None,
    };
    let mut out = form_data("user", super::php::string(&format!("user_{id}")), &(ctx.nonce)("user"));
    for group in &visible {
        if group.setting("style", "default") == "default" {
            out.push_str(&format!("<h2>{}</h2>", group_title(group)));
        }
        out.push_str("<table class=\"form-table\"><tbody>");
        let instruction = group.setting("instruction_placement", "label");
        for field in &group.fields {
            let mut f = RenderField::load(field, &render);
            if let Some(value) = posted.as_ref().and_then(|items| items.iter().find(|(k, _)| super::php::key_eq(k, &super::php::key(&field.key))).map(|(_, v)| v.clone())) {
                f.set("value", value);
            } else if matches!(f.get("value"), None | Some(Value::Null)) {
                f.set("value", get_value(&render.site.db, object, field, &field.name).await?);
            }
            out.push_str(&render_field_wrap(&render, f, "tr", &instruction).await?);
        }
        out.push_str("</tbody></table>");
    }
    Ok(uploader_boxes(out, ctx, &visible))
}

/// `ACF_Form_User::admin_footer()` on `acf/input/admin_footer` (`$this->view` is never set).
fn admin_footer() -> &'static str {
    "<script type=\"text/javascript\">\n(function($) {\n\t\n\t// vars\n\tvar view = '';\n\t\n\t// add missing spinners\n\tvar $submit = $('input.button-primary');\n\tif( !$submit.next('.spinner').length ) {\n\t\t$submit.after('<span class=\"spinner\"></span>');\n\t}\n\t\n})(jQuery);\t\n</script>\n\t\t\t"
}

/// `ACF_Assets::print_footer_scripts()` of the user screens: the form data of the fields (`validation` is the integer
/// `1` of `render()`) and the script of `admin_footer()`, if there were any.
pub async fn footer_scripts(groups: &[FieldGroup], ctx: &AdminContext<'_>) -> Result<Option<String>> {
    if !is_user_screen(ctx) {
        return Ok(None);
    }
    let user_id = screen_user(ctx);
    let visible = groups_for(groups, ctx, user_id).await?;
    let (form, uploader, input_footer) = if visible.is_empty() {
        (FormData::default(), false, "")
    } else {
        let form = FormData { screen: Json::String("user".into()), post_id: Json::String(format!("user_{}", user_id.unwrap_or(0))), validation: Json::from(1), ..Default::default() };
        (form, wants_uploader(&visible, (ctx.can)("upload_files")), admin_footer())
    };
    Ok(Some(super::form::footer_with(ctx.site, &form, ctx.wp_version, &(ctx.nonce)("acf_nonce"), ctx.user_agent, ctx.can, if uploader { Uploader::Render } else { Uploader::No }, input_footer)))
}
