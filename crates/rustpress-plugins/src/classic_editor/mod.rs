//! Classic Editor 1.7.0: the classic edit screen with TinyMCE and meta boxes instead of the block editor.
//!
//! Two options on "Settings > Writing" decide what it does (`Classic_Editor::get_settings()`):
//! `classic-editor-replace` (`classic`, the default, or `block`) and `classic-editor-allow-users` (`disallow`, the
//! default, or `allow`).
//!
//! - Users may not switch (the default): with the classic editor, `use_block_editor_for_post_type` is `false` for every
//!   type, so every post opens in the classic form.
//! - Users may switch: `use_block_editor_for_post` chooses per post (`choose_editor()`), from the editor last used for
//!   it (the post meta `classic-editor-remember`), else from whether its content has blocks. The list of posts gets
//!   two edit links and the editor as a post state, both editors get a link to the other, and each user picks the
//!   default on the profile (the user option `classic-editor-settings`).
//!
//! Always: the two settings on "Settings > Writing", a "Settings" link in the list of plugins and two style fixes on
//! the edit screen (Safari 18, the buttons of the Publish box in WordPress 7.0). The plugin's activation hook never
//! runs (it is registered on `plugins_loaded`), so activating it stores nothing. Multisite is not supported by
//! RustPress, so the network settings are not ported.

mod admin;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, AdminContext, Assets, Die, MetaBoxes, NativePlugin, PostMetaBoxes, UserSaved};
use rustpress_core::{Result, Site};
use rustpress_db::Post;

pub struct ClassicEditor;

/// `CLASSIC_EDITOR_VERSION`.
pub const VERSION: &str = "1.7.0";
pub const DOMAIN: &str = "classic-editor";
pub const FILE: &str = "classic-editor/classic-editor.php";

impl NativePlugin for ClassicEditor {
    fn file(&self) -> &'static str {
        FILE
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(DOMAIN)
    }

    fn load<'a>(&'a self, _site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async { Ok(Box::new(Active) as Box<dyn ActivePlugin>) })
    }
}

/// `Classic_Editor::get_settings()` without the user's choice: `(editor is classic, allow users)`.
pub fn site_settings(site: &Site) -> (bool, bool) {
    let allow_users = site.options.get("classic-editor-allow-users").is_some_and(|v| v == "allow");
    // Old values are normalized: `no-replace` was the block editor, `replace` the classic one.
    let classic = !site.options.get("classic-editor-replace").is_some_and(|v| v == "block" || v == "no-replace");
    (classic, allow_users)
}

struct Active;

impl ActivePlugin for Active {
    /// `add_filter( 'use_block_editor_for_post_type', '__return_false', 100 )` while users may not switch and the
    /// editor is the classic one.
    fn use_block_editor_for_post_type(&self, site: &Site, _post_type: &str, use_block_editor: bool) -> bool {
        match site_settings(site) {
            (true, false) => false,
            _ => use_block_editor,
        }
    }

    fn use_block_editor_for_post<'a>(&'a self, ctx: &'a AdminContext<'a>, post: &'a Post, use_block_editor: bool) -> BoxFuture<'a, Result<bool>> {
        Box::pin(admin::choose_editor(ctx, post, use_block_editor))
    }

    fn admin_print_styles(&self, ctx: &AdminContext<'_>) -> String {
        admin::print_styles(ctx)
    }

    fn settings_fields<'a>(&'a self, ctx: &'a AdminContext<'a>, page: &'a str) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async move { Ok(if page == "writing" { admin::settings_fields(ctx) } else { Vec::new() }) })
    }

    /// `register_setting( 'writing', ... )` of both options.
    fn registered_settings(&self) -> Vec<(&'static str, &'static str)> {
        vec![("writing", "classic-editor-replace"), ("writing", "classic-editor-allow-users")]
    }

    /// `validate_option_editor()` and `validate_option_allow_users()`.
    fn sanitize_setting(&self, option: &str, value: Option<rustpress_phpser::Value>) -> Option<rustpress_phpser::Value> {
        let is = |wanted: &str| matches!(&value, Some(rustpress_phpser::Value::Str(text)) if text == wanted.as_bytes());
        let stored = match option {
            "classic-editor-replace" => {
                if is("block") {
                    "block"
                } else {
                    "classic"
                }
            }
            "classic-editor-allow-users" => {
                if is("allow") {
                    "allow"
                } else {
                    "disallow"
                }
            }
            _ => return value,
        };
        Some(rustpress_phpser::Value::Str(stored.as_bytes().to_vec()))
    }

    /// `add_settings_link()`: on single sites only an administrator sees the list of plugins, who may `manage_options`.
    fn plugin_action_links(&self, site: &Site, mut links: Vec<String>) -> Vec<String> {
        links.push(format!("<a href=\"{}\">{}</a>", site.site_url("wp-admin/options-writing.php#classic-editor-options"), site.trd(DOMAIN, "Settings")));
        links
    }

    fn post_meta_boxes<'a>(&'a self, ctx: &'a AdminContext<'a>, post: &'a Post) -> BoxFuture<'a, Result<PostMetaBoxes>> {
        Box::pin(admin::post_meta_boxes(ctx, post))
    }

    fn get_edit_post_link<'a>(&'a self, ctx: &'a AdminContext<'a>, _post_id: u64, link: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(admin::edit_post_link(ctx, link))
    }

    fn redirect_post_location<'a>(&'a self, ctx: &'a AdminContext<'a>, location: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(if site_settings(ctx.site).1 { admin::redirect_location(ctx, location) } else { location }) })
    }

    fn display_post_states<'a>(&'a self, ctx: &'a AdminContext<'a>, post: &'a Post, states: Vec<(String, String)>) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async move { if site_settings(ctx.site).1 { admin::post_states(ctx, post, states).await } else { Ok(states) } })
    }

    fn post_row_actions<'a>(&'a self, ctx: &'a AdminContext<'a>, post: &'a Post, _hierarchical: bool, actions: Vec<(String, String)>) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async move { if site_settings(ctx.site).1 { admin::row_actions(ctx, post, actions).await } else { Ok(actions) } })
    }

    fn edit_form_top<'a>(&'a self, ctx: &'a AdminContext<'a>, post: &'a Post) -> BoxFuture<'a, Result<String>> {
        Box::pin(admin::edit_form_top(ctx, post))
    }

    fn block_editor_settings<'a>(&'a self, ctx: &'a AdminContext<'a>, post: &'a Post) -> BoxFuture<'a, Result<()>> {
        Box::pin(admin::block_editor_settings(ctx, post))
    }

    fn admin_head<'a>(&'a self, ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(admin::admin_head(ctx)) })
    }

    fn profile_personal_options<'a>(&'a self, ctx: &'a AdminContext<'a>, user_id: u64) -> BoxFuture<'a, Result<String>> {
        Box::pin(admin::user_settings(ctx, user_id))
    }

    /// `edit_user_profile` (the profile of someone else).
    fn user_form<'a>(&'a self, ctx: &'a AdminContext<'a>, user_id: Option<u64>) -> BoxFuture<'a, Result<MetaBoxes>> {
        Box::pin(async move {
            let html = match user_id {
                Some(id) if ctx.hook_suffix == "user-edit.php" && id != ctx.user => admin::user_settings(ctx, id).await?,
                _ => String::new(),
            };
            Ok(MetaBoxes { html, ..MetaBoxes::default() })
        })
    }

    fn saved_user<'a>(&'a self, saved: &'a UserSaved<'a>) -> BoxFuture<'a, Result<Option<Die>>> {
        Box::pin(async move {
            admin::save_user_settings(saved).await?;
            Ok(None)
        })
    }

    fn block_editor_assets(&self, ctx: &AdminContext<'_>, assets: &mut Assets) {
        admin::block_editor_assets(ctx, assets);
    }
}
