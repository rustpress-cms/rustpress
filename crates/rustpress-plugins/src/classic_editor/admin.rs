//! What Classic Editor does in the administration.

use rustpress_blocks::php_json;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::native::{AdminContext, Assets, PluginMetaBox, PostMetaBoxes, UserSaved};
use rustpress_core::urls::{add_query_arg, remove_query_arg};
use rustpress_core::{Result, Site};
use rustpress_db::Post;

use super::{DOMAIN, VERSION, site_settings};

/// The plugin's file has CRLF line ends, and so has what it prints outside of PHP.
pub fn crlf(html: &str) -> String {
    html.replace('\n', "\r\n")
}

fn tr(site: &Site, text: &str) -> String {
    site.trd(DOMAIN, text)
}

fn trx(site: &Site, text: &str, context: &str) -> String {
    site.i18n().pgettext(DOMAIN, context, text)
}

/// `get_user_option( $option, $user_id )`: the meta with the table prefix first, then the plain one.
async fn user_option(site: &Site, user: u64, name: &str) -> Result<Option<String>> {
    if let Some(value) = site.db.user_meta(user, &format!("{}{name}", site.db.table_prefix())).await? {
        return Ok(Some(value));
    }
    Ok(site.db.user_meta(user, name).await?)
}

/// `Classic_Editor::get_settings()` for `user`: `(editor is classic, allow users)`. While users may switch, their
/// own choice wins, except on "Settings > Writing".
pub async fn settings(ctx: &AdminContext<'_>, user: u64) -> (bool, bool) {
    let (mut classic, allow_users) = site_settings(ctx.site);
    if allow_users && ctx.hook_suffix != "options-writing.php" {
        match user_option(ctx.site, user, "classic-editor-settings").await.ok().flatten().as_deref() {
            Some("block") => classic = false,
            Some("classic") => classic = true,
            _ => {}
        }
    }
    (classic, allow_users)
}

/// `post_type_supports( $post_type, 'editor' )`.
fn supports_editor(site: &Site, post_type: &str) -> bool {
    matches!(post_type, "post" | "page") || site.types.post_type(post_type).is_some_and(|t| t.supports("editor"))
}

/// `use_block_editor_for_post_type( $post_type )` (`can_edit_post_type()`). `rp-classic` stands for the must-use
/// plugin of the reference that turns the block editor off for a comparison (as in the administration).
fn block_editor_for_post_type(ctx: &AdminContext<'_>, post_type: &str) -> bool {
    let site = ctx.site;
    if query_has(ctx, "rp-classic") {
        return false;
    }
    let supported = match post_type {
        "post" | "page" => true,
        other => site.types.post_type(other).is_some_and(|t| t.show_in_rest && t.supports("editor")),
    };
    supported && site.plugins.use_block_editor_for_post_type(site, post_type, true)
}

/// `get_enabled_editors_for_post()`: `(classic editor, block editor)`.
pub fn enabled_editors(ctx: &AdminContext<'_>, post: &Post) -> (bool, bool) {
    if post.post_type.is_empty() {
        return (false, false);
    }
    (supports_editor(ctx.site, &post.post_type), block_editor_for_post_type(ctx, &post.post_type))
}

/// `has_blocks()` of the plugin: the content has `<!-- wp:`.
pub fn has_blocks(content: &str) -> bool {
    content.contains("<!-- wp:")
}

fn query_has(ctx: &AdminContext<'_>, key: &str) -> bool {
    ctx.query.iter().any(|(k, _)| k == key)
}

/// `get_post_meta( $post_id, 'classic-editor-remember', true )`.
pub async fn remembered(site: &Site, post_id: u64) -> Result<String> {
    Ok(site.db.post_meta(post_id).await?.into_iter().find(|m| m.meta_key.as_deref() == Some("classic-editor-remember")).and_then(|m| m.meta_value).unwrap_or_default())
}

/// `is_classic( $post_id )` while users may switch.
async fn is_classic(ctx: &AdminContext<'_>, post: &Post) -> Result<bool> {
    if !query_has(ctx, "classic-editor__forget") {
        match remembered(ctx.site, post.id).await?.as_str() {
            "classic-editor" => return Ok(true),
            "block-editor" => return Ok(false),
            _ => {}
        }
        return Ok(!has_blocks(&post.post_content));
    }
    Ok(query_has(ctx, "classic-editor"))
}

/// `choose_editor()` on `use_block_editor_for_post` (priority 100), while users may switch.
pub async fn choose_editor(ctx: &AdminContext<'_>, post: &Post, mut use_block_editor: bool) -> Result<bool> {
    let (classic, allow_users) = settings(ctx, ctx.user).await;
    if !allow_users {
        return Ok(use_block_editor);
    }
    let (classic_editor, block_editor) = enabled_editors(ctx, post);
    if !classic_editor && !block_editor {
        return Ok(use_block_editor);
    }
    if post.id == 0 || post.post_status == "auto-draft" {
        let forget = query_has(ctx, "classic-editor__forget");
        if (classic && !forget) || (query_has(ctx, "classic-editor") && forget) {
            use_block_editor = false;
        }
    } else if is_classic(ctx, post).await? {
        use_block_editor = false;
    }
    if use_block_editor && !block_editor {
        use_block_editor = false;
    } else if !use_block_editor && !classic_editor && block_editor {
        use_block_editor = true;
    }
    Ok(use_block_editor)
}

/// `safari_18_temp_fix()` and `print_70_publishing_actions_hotfix()` on `admin_print_styles`: both on the edit
/// screen of a post (`$current_screen->base` is `post`), in the block editor too.
pub fn print_styles(ctx: &AdminContext<'_>) -> String {
    let mut out = String::new();
    if matches!(ctx.hook_suffix, "post.php" | "post-new.php") {
        let clear = if ctx.site.is_rtl() { "right" } else { "left" };
        out.push_str(&format!("\t\t\t<style id=\"classic-editor-safari-18-temp-fix\">\n\t\t\t_::-webkit-full-page-media, _:future, :root #post-body #postbox-container-2 {{\n\t\t\t\tclear: {clear};\n\t\t\t}}\n\t\t\t</style>\n\t\t\t"));
        out.push_str("        <style>\n            #major-publishing-actions {\n                flex-wrap: wrap;\n            }\n        </style>\n        ");
    }
    crlf(&out)
}

/// `settings_1()`: the radio buttons of the default editor.
pub fn editor_choice(site: &Site, classic: bool) -> String {
    crlf(&format!(
        "\t\t<div class=\"classic-editor-options\">\n\t\t\t<p>\n\t\t\t\t<input type=\"radio\" name=\"classic-editor-replace\" id=\"classic-editor-classic\" value=\"classic\"{} />\n\t\t\t\t<label for=\"classic-editor-classic\">{}</label>\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<input type=\"radio\" name=\"classic-editor-replace\" id=\"classic-editor-block\" value=\"block\"{} />\n\t\t\t\t<label for=\"classic-editor-block\">{}</label>\n\t\t\t</p>\n\t\t</div>\n\t\t<script>\n\t\tjQuery( 'document' ).ready( function( $ ) {{\n\t\t\tif ( window.location.hash === '#classic-editor-options' ) {{\n\t\t\t\t$( '.classic-editor-options' ).closest( 'td' ).addClass( 'highlight' );\n\t\t\t}}\n\t\t}} );\n\t\t</script>\n\t\t",
        if classic { " checked" } else { "" },
        trx(site, "Classic editor", "Editor Name"),
        if classic { "" } else { " checked" },
        trx(site, "Block editor", "Editor Name"),
    ))
}

/// `register_settings()`: `add_settings_field()` of both settings on "Settings > Writing".
pub fn settings_fields(ctx: &AdminContext<'_>) -> Vec<(String, String)> {
    let site = ctx.site;
    let (classic, allow_users) = site_settings(site);
    let allow = format!(
        "\t\t<div class=\"classic-editor-options\">\n\t\t\t<p>\n\t\t\t\t<input type=\"radio\" name=\"classic-editor-allow-users\" id=\"classic-editor-allow\" value=\"allow\"{} />\n\t\t\t\t<label for=\"classic-editor-allow\">{}</label>\n\t\t\t</p>\n\t\t\t<p>\n\t\t\t\t<input type=\"radio\" name=\"classic-editor-allow-users\" id=\"classic-editor-disallow\" value=\"disallow\"{} />\n\t\t\t\t<label for=\"classic-editor-disallow\">{}</label>\n\t\t\t</p>\n\t\t</div>\n\t\t",
        if allow_users { " checked" } else { "" },
        tr(site, "Yes"),
        if allow_users { "" } else { " checked" },
        tr(site, "No"),
    );
    vec![(tr(site, "Default editor for all users"), editor_choice(site, classic)), (tr(site, "Allow users to switch editors"), crlf(&allow))]
}

/// `enqueue_block_editor_scripts()`: the "Switch to classic editor" item of the block editor's menu, while users may
/// switch and the post's type has the classic editor.
pub fn block_editor_assets(ctx: &AdminContext<'_>, assets: &mut Assets) {
    let Some(post) = ctx.edited_post else { return };
    if !site_settings(ctx.site).1 || !enabled_editors(ctx, post).0 {
        return;
    }
    let src = ctx.site.site_url("wp-content/plugins/classic-editor/js/block-editor-plugin.js");
    let script = assets.script("classic-editor-plugin", src, &["wp-element", "wp-components", "lodash"], VERSION, true);
    let l10n = serde_json::json!({"linkText": tr(ctx.site, "Switch to classic editor")});
    let json = php_json::encode_with(&l10n, php_json::Flags { unescaped_slashes: true, unescaped_unicode: false }).replace('<', "\\u003C").replace('>', "\\u003E");
    script.extra = Some(format!("var classicEditorPluginL10n = {json};"));
}

/// `get_edited_post_id()`: the post of `post.php?post=...&action=edit`.
fn edited_post_id(ctx: &AdminContext<'_>) -> u64 {
    let get = |key: &str| ctx.query.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str()).unwrap_or_default();
    if ctx.hook_suffix != "post.php" || get("action") != "edit" || get("post").is_empty() || get("post") == "0" {
        return 0;
    }
    // `(int) $_GET['post']`: the leading digits.
    get("post").trim_start().chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)
}

/// What the plugin adds to the classic edit form.
pub async fn post_meta_boxes(ctx: &AdminContext<'_>, post: &Post) -> Result<PostMetaBoxes> {
    let mut boxes = PostMetaBoxes::default();
    let (classic, allow_users) = settings(ctx, ctx.user).await;
    // With the block editor for everybody the plugin stops after its settings.
    if !classic && !allow_users {
        return Ok(boxes);
    }
    // `on_admin_init()`: the notice of the privacy policy page moves under the title.
    let post_id = edited_post_id(ctx);
    if post_id != 0 && (classic || (allow_users && post.id == post_id && is_classic(ctx, post).await?)) {
        boxes.privacy_notice_after_title = true;
    }
    if allow_users && let Some(meta_box) = switch_box(ctx, post).await? {
        boxes.boxes.push(meta_box);
    }
    Ok(boxes)
}

/// `isset( $_REQUEST['classic-editor'] )`.
fn request_has(ctx: &AdminContext<'_>, key: &str) -> bool {
    query_has(ctx, key) || ctx.form.iter().any(|(k, _)| k == key)
}

/// `get_edit_post_link()` (on `get_edit_post_link`, while users may switch): the request's or the user's classic
/// editor stays in the link.
pub async fn edit_post_link(ctx: &AdminContext<'_>, link: String) -> Result<String> {
    let (classic, allow_users) = settings(ctx, ctx.user).await;
    if allow_users && (request_has(ctx, "classic-editor") || classic) {
        return Ok(add_query_arg(&link, "classic-editor", ""));
    }
    Ok(link)
}

/// The raw edit link of `post` as the plugin's own filter makes it.
async fn raw_edit_link(ctx: &AdminContext<'_>, post_id: u64) -> Result<String> {
    edit_post_link(ctx, ctx.site.site_url(&format!("wp-admin/post.php?post={post_id}&action=edit"))).await
}

/// `redirect_location()` on `redirect_post_location`: the classic editor comes back after saving.
pub fn redirect_location(ctx: &AdminContext<'_>, location: String) -> String {
    let referer = ctx.form.iter().rev().find(|(k, _)| k == "_wp_http_referer").map(|(_, v)| v.as_str());
    if request_has(ctx, "classic-editor") || referer.is_some_and(|r| r.contains("&classic-editor")) {
        return add_query_arg(&location, "classic-editor", "");
    }
    location
}

/// `add_post_state()` on `display_post_states`: the editor a post opens in.
pub async fn post_states(ctx: &AdminContext<'_>, post: &Post, mut states: Vec<(String, String)>) -> Result<Vec<(String, String)>> {
    if post.post_status == "trash" {
        return Ok(states);
    }
    let site = ctx.site;
    let state = match enabled_editors(ctx, post) {
        (false, false) => return Ok(states),
        (true, false) => format!("<span class=\"classic-editor-forced-state\">{}</span>", trx(site, "classic editor", "Editor Name")),
        (false, true) => format!("<span class=\"classic-editor-forced-state\">{}</span>", trx(site, "block editor", "Editor Name")),
        (true, true) => {
            let last = remembered(site, post.id).await?;
            let is_classic = if !last.is_empty() {
                last == "classic-editor"
            } else if !post.post_content.is_empty() {
                !has_blocks(&post.post_content)
            } else {
                settings(ctx, ctx.user).await.0
            };
            if is_classic { trx(site, "Classic editor", "Editor Name") } else { trx(site, "Block editor", "Editor Name") }
        }
    };
    match states.iter_mut().find(|(key, _)| key == "classic-editor-plugin") {
        Some(entry) => entry.1 = state,
        None => states.push(("classic-editor-plugin".into(), state)),
    }
    Ok(states)
}

/// `add_edit_links()` on `post_row_actions`/`page_row_actions` (priority 15): "Edit" becomes a link to each editor.
pub async fn row_actions(ctx: &AdminContext<'_>, post: &Post, mut actions: Vec<(String, String)>) -> Result<Vec<(String, String)>> {
    actions.retain(|(key, _)| key != "classic");
    let Some(edit_at) = actions.iter().position(|(key, _)| key == "edit") else { return Ok(actions) };
    let site = ctx.site;
    if enabled_editors(ctx, post) != (true, true) {
        return Ok(actions);
    }
    let edit_url = add_query_arg(&raw_edit_link(ctx, post.id).await?, "classic-editor__forget", "");
    // `_draft_or_post_title()`.
    let title = rustpress_blocks::content::the_title_in_admin(site, post);
    let title = esc_html(&if title.is_empty() { site.tr("(no title)") } else { title });
    let link = |url: &str, label: String, text: String| format!("<a href=\"{}\" aria-label=\"{}\">{text}</a>", esc_url(url), esc_attr(&label));
    let block = link(&remove_query_arg(&edit_url, "classic-editor"), sprintf(&tr(site, "Edit &#8220;%s&#8221; in the block editor"), &[&title]), trx(site, "Edit (block editor)", "Editor Name"));
    let classic = link(&add_query_arg(&edit_url, "classic-editor", ""), sprintf(&tr(site, "Edit &#8220;%s&#8221; in the classic editor"), &[&title]), trx(site, "Edit (classic editor)", "Editor Name"));
    // `array_splice()` numbers what it inserts, and renumbers the other numeric keys.
    actions.splice(edit_at..=edit_at, [("block".to_owned(), block), ("classic".to_owned(), classic)]);
    let mut number = 0;
    for (key, _) in &mut actions {
        if key == "block" || key == "classic" || key.parse::<u64>().is_ok() {
            *key = number.to_string();
            number += 1;
        }
    }
    Ok(actions)
}

/// `remember()`: the editor last used for a post.
async fn remember(ctx: &AdminContext<'_>, post_id: u64, editor: &str) -> Result<()> {
    if let Some(writer) = ctx.writer
        && remembered(ctx.site, post_id).await? != editor
    {
        writer.update_post_meta(post_id, "classic-editor-remember", editor).await?;
    }
    Ok(())
}

/// `add_redirect_helper()` and `remember_classic_editor()` on `edit_form_top`.
pub async fn edit_form_top(ctx: &AdminContext<'_>, post: &Post) -> Result<String> {
    if !site_settings(ctx.site).1 {
        return Ok(String::new());
    }
    if supports_editor(ctx.site, &post.post_type) {
        remember(ctx, post.id, "classic-editor").await?;
    }
    Ok(crlf("\t\t<input type=\"hidden\" name=\"classic-editor\" value=\"\" />\n\t\t"))
}

/// `remember_block_editor()` on `block_editor_settings_all`.
pub async fn block_editor_settings(ctx: &AdminContext<'_>, post: &Post) -> Result<()> {
    if site_settings(ctx.site).1 && block_editor_for_post_type(ctx, &post.post_type) {
        remember(ctx, post.id, "block-editor").await?;
    }
    Ok(())
}

/// `add_edit_php_inline_style()` on `admin_head-edit.php`.
pub fn admin_head(ctx: &AdminContext<'_>) -> String {
    if ctx.hook_suffix != "edit.php" || !site_settings(ctx.site).1 {
        return String::new();
    }
    crlf("\t\t<style>\n\t\t.classic-editor-forced-state {\n\t\t\tfont-style: italic;\n\t\t\tfont-weight: 400;\n\t\t\tcolor: #72777c;\n\t\t\tfont-size: small;\n\t\t}\n\t\t</style>\n\t\t")
}

/// `user_settings()` on `profile_personal_options` and `edit_user_profile`: the user's default editor.
pub async fn user_settings(ctx: &AdminContext<'_>, user_id: u64) -> Result<String> {
    // `$user_can_edit` of `user-edit.php`.
    let can_edit = (ctx.can)("edit_posts") || (ctx.can)("edit_pages");
    if !can_edit || !site_settings(ctx.site).1 {
        return Ok(String::new());
    }
    let (classic, _) = settings(ctx, user_id).await;
    let nonce = format!(
        "<input type=\"hidden\" id=\"classic-editor-user-settings\" name=\"classic-editor-user-settings\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />",
        (ctx.nonce)("allow-user-settings"),
        esc_attr(ctx.request_uri)
    );
    Ok(format!(
        "{}\t\t\t\t{}{}",
        crlf(&format!("\t\t<table class=\"form-table\">\n\t\t\t<tr class=\"classic-editor-user-options\">\n\t\t\t\t<th scope=\"row\">{}</th>\n\t\t\t\t<td>\n\t\t\t\t{nonce}", tr(ctx.site, "Default Editor"))),
        editor_choice(ctx.site, classic),
        crlf("\t\t\t\t</td>\n\t\t\t</tr>\n\t\t</table>\n\t\t<script>jQuery( 'tr.user-rich-editing-wrap' ).before( jQuery( 'tr.classic-editor-user-options' ) );</script>\n\t\t")
    ))
}

/// `save_user_settings()` on `personal_options_update` and `edit_user_profile_update`.
pub async fn save_user_settings(saved: &UserSaved<'_>) -> Result<()> {
    let ctx = saved.ctx;
    if saved.created || !site_settings(ctx.site).1 {
        return Ok(());
    }
    let field = |key: &str| ctx.form.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    let (Some(nonce), Some(editor)) = (field("classic-editor-user-settings"), field("classic-editor-replace")) else { return Ok(()) };
    if !(ctx.verify_nonce)(nonce, "allow-user-settings") || (saved.user_id != ctx.user && !(ctx.can)("edit_users")) {
        return Ok(());
    }
    // `update_user_option()`: the meta of the site's table prefix.
    let key = format!("{}classic-editor-settings", ctx.site.db.table_prefix());
    saved.writer.update_user_meta(saved.user_id, &key, if editor == "block" { "block" } else { "classic" }).await?;
    Ok(())
}

/// `add_meta_box()` on `add_meta_boxes`: "Editor" with a link to the block editor, in the classic form.
pub async fn switch_box(ctx: &AdminContext<'_>, post: &Post) -> Result<Option<PluginMetaBox>> {
    if ctx.block_editor || enabled_editors(ctx, post) != (true, true) {
        return Ok(None);
    }
    let url = add_query_arg(&remove_query_arg(&raw_edit_link(ctx, post.id).await?, "classic-editor"), "classic-editor__forget", "");
    let html = crlf(&format!("\t\t<p style=\"margin: 1em 0;\">\n\t\t\t<a href=\"{}\">{}</a>\n\t\t</p>\n\t\t", esc_url(&url), tr(ctx.site, "Switch to block editor")));
    Ok(Some(PluginMetaBox { id: "classic-editor-switch-editor".into(), title: tr(ctx.site, "Editor"), context: "side".into(), priority: "default".into(), html }))
}
