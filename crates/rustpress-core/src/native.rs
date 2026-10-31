//! Plugins RustPress implements natively. A module stands in for one WordPress plugin and runs while that plugin is
//! active (`active_plugins`): activating and deactivating stay what they are in WordPress. The PHP of the plugin never
//! runs; its files under `wp-content/plugins` (headers, assets) are used as they are.
//!
//! The server registers the modules once ([register]); [Site::load_at](crate::Site::load_at) then loads the ones whose
//! plugin is active. Every hook of [ActivePlugin] does nothing unless the module overrides it, and is called where
//! WordPress fires the action or filter it stands for.

use std::sync::OnceLock;

use futures::future::BoxFuture;
use rustpress_db::Post;

use crate::{Result, Site};

/// A plugin RustPress can stand in for.
pub trait NativePlugin: Send + Sync {
    /// The plugin file as `active_plugins` names it (`wp-date-remover/wp-date-remover.php`).
    fn file(&self) -> &'static str;
    /// The release of the plugin the module follows.
    fn version(&self) -> &'static str;
    /// `plugins_loaded`/`init`: what the plugin reads from the database, once per snapshot of the site.
    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>>;
    /// The text domain WordPress loads just in time from `wp-content/languages/plugins/<domain>-<locale>.mo`.
    fn text_domain(&self) -> Option<&'static str> {
        None
    }
}

/// A loaded plugin.
pub trait ActivePlugin: Send + Sync {
    /// `wp_enqueue_scripts`: styles and scripts on the website.
    fn enqueue(&self, _site: &Site, _assets: &mut Assets) {}

    /// Filters that make `get_the_date()`, `get_the_time()` and `get_the_modified_date()` return `false` while
    /// `post` is the post of the loop (`the_post`).
    fn hides_date<'a>(&'a self, _site: &'a Site, _post: &'a Post) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async { Ok(false) })
    }

    /// Post meta the plugin registers for the REST API (`register_meta( 'post', $key, array( 'show_in_rest' => true ) )`).
    fn registered_post_meta(&self) -> Vec<RegisteredMeta> {
        Vec::new()
    }

    /// `wp_footer` at the default priority (after the speculation rules, before the script modules). `looped`: the
    /// posts `the_post` ran for, in order.
    fn wp_footer<'a>(&'a self, _site: &'a Site, _looped: &'a [u64]) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `admin_menu`: the pages the plugin adds to the menu of the administration.
    fn admin_pages(&self, _site: &Site) -> Vec<AdminPage> {
        Vec::new()
    }

    /// `admin_enqueue_scripts`: styles and scripts of the administration.
    fn admin_enqueue(&self, _ctx: &AdminContext<'_>, _assets: &mut Assets) {}

    /// `admin_notices`: printed above the content of every screen.
    fn admin_notices<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The callback of one of the plugin's [`AdminPage`]s: what it prints (and does) inside the frame.
    fn admin_page<'a>(&'a self, _slug: &'a str, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `plugin_action_links_{$plugin_file}`: the links below the plugin's name in the list of plugins.
    fn plugin_action_links(&self, _site: &Site, links: Vec<String>) -> Vec<String> {
        links
    }

    /// `plugin_row_meta`: the links after the version, author and details of any plugin in the list.
    fn plugin_row_meta(&self, _site: &Site, _file: &str, links: Vec<String>) -> Vec<String> {
        links
    }

    /// `load-{$hook_suffix}` of one of the plugin's pages: the tabs of its contextual help.
    fn admin_page_help(&self, _slug: &str, _site: &Site) -> AdminHelp {
        AdminHelp::default()
    }

    /// `admin_footer_text`: what the left side of the footer says instead of the thanks for using WordPress.
    fn admin_footer_text(&self, _ctx: &AdminContext<'_>) -> Option<String> {
        None
    }

    /// `register_setting( $group, $option )`: `(group, option)` of the options the plugin's forms save through
    /// `options.php`.
    fn registered_settings(&self) -> Vec<(&'static str, &'static str)> {
        Vec::new()
    }

    /// The `sanitize_callback` of a registered setting (`sanitize_option_{$option}`).
    fn sanitize_setting(&self, _option: &str, value: Option<rustpress_phpser::Value>) -> Option<rustpress_phpser::Value> {
        value
    }

    /// `enqueue_block_editor_assets`.
    fn block_editor_assets(&self, _ctx: &AdminContext<'_>, _assets: &mut Assets) {}

    /// `upload_mimes`: what `get_allowed_mime_types()` gives the uploader, `(extensions, MIME type)`.
    fn upload_mimes(&self, _uploader: &Uploader<'_>, _mimes: &mut Vec<(String, String)>) {}

    /// `wp_handle_upload_prefilter`: may change the file before it is checked and stored, or refuse it with a message.
    fn upload_prefilter(&self, _uploader: &Uploader<'_>, _name: &mut String, _bytes: &mut Vec<u8>) -> std::result::Result<(), String> {
        Ok(())
    }

    /// `wp_check_filetype_and_ext` after the core refused the file: `(name, MIME type)` to accept it with.
    fn accept_filetype(&self, _name: &str, _mimes: &[(String, String)]) -> Option<(String, String)> {
        None
    }

    /// `wp_generate_attachment_metadata` of a new upload (`path` is the stored file, `relative` its path below the
    /// uploads directory).
    fn attachment_metadata(&self, _site: &Site, _mime: &str, _path: &std::path::Path, _relative: &str, metadata: rustpress_phpser::Value) -> rustpress_phpser::Value {
        metadata
    }

    /// `wp_prepare_attachment_for_js` of an attachment (`path` its file, when it exists).
    fn prepare_attachment_for_js(&self, _mime: &str, _path: Option<&std::path::Path>, _response: &mut serde_json::Map<String, serde_json::Value>) {}

    /// The whole page of an administration screen, for plugins that buffer and rewrite it (`ob_start()` on
    /// `admin_init`).
    fn admin_page_output(&self, _ctx: &AdminContext<'_>, html: String) -> String {
        html
    }

    /// `post_thumbnail_html`: the featured image of `post_id` (the post of the loop); `singular` is `is_singular()`.
    fn post_thumbnail_html<'a>(&'a self, _site: &'a Site, _post_id: u64, _singular: bool, html: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(html) })
    }

    /// `wp_ajax_{$action}`: the answer to `admin-ajax.php`, or `None` when the plugin has no such action.
    fn admin_ajax<'a>(&'a self, _action: &'a str, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<AjaxAnswer>>> {
        Box::pin(async { Ok(None) })
    }
}

/// Who uploads a file, and how.
pub struct Uploader<'a> {
    pub site: &'a Site,
    /// The roles of the user.
    pub roles: &'a [String],
    /// `current_user_can( $capability )`.
    pub can: &'a (dyn Fn(&str) -> bool + Send + Sync),
    /// The upload is a request of the REST API (`REST_REQUEST`).
    pub rest: bool,
    /// The request carries a valid `_wpnonce` for `media-form`.
    pub media_form_nonce: bool,
}

/// What a plugin answers to `admin-ajax.php`.
#[derive(Debug, Clone, PartialEq)]
pub struct AjaxAnswer {
    pub status: u16,
    pub body: String,
    /// `wp_send_json()`: `application/json`, else `text/html`.
    pub json: bool,
}

impl AjaxAnswer {
    /// `wp_die( $text )` of `admin-ajax.php` (`0` when the action printed nothing).
    pub fn text(status: u16, body: &str) -> Self {
        Self { status, body: body.to_owned(), json: false }
    }

    /// `wp_send_json( $data, $status )` with the encoded data.
    pub fn json(status: u16, body: String) -> Self {
        Self { status, body, json: true }
    }
}

/// The contextual help of a plugin's page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdminHelp {
    pub tabs: Vec<HelpTab>,
    /// HTML of the sidebar (`set_help_sidebar()`).
    pub sidebar: String,
}

/// One tab of the contextual help (`WP_Screen::add_help_tab()`).
#[derive(Debug, Clone, PartialEq)]
pub struct HelpTab {
    pub id: String,
    pub title: String,
    /// HTML.
    pub content: String,
}

/// `$_POST[ $name ]` as PHP builds it from the fields `name`, `name[key]`, `name[]` and deeper: a string, or an array
/// of what the brackets name; `None` when no field has the name.
pub fn form_value(form: &[(String, String)], name: &str) -> Option<rustpress_phpser::Value> {
    use rustpress_phpser::{Key, Value};
    fn insert(target: &mut Value, keys: &[String], value: &str) {
        let Value::Array(items) = target else { return };
        let Some((first, rest)) = keys.split_first() else { return };
        let key = if first.is_empty() {
            Key::Int(items.iter().filter_map(|(k, _)| if let Key::Int(n) = k { Some(*n + 1) } else { None }).max().unwrap_or(0))
        } else if let Some(n) = first.parse::<i64>().ok().filter(|n| n.to_string() == *first) {
            // PHP keys an array with an integer when the string is one.
            Key::Int(n)
        } else {
            Key::Str(first.as_bytes().to_vec())
        };
        if rest.is_empty() {
            match items.iter_mut().find(|(k, _)| *k == key) {
                Some((_, slot)) => *slot = Value::Str(value.as_bytes().to_vec()),
                None => items.push((key, Value::Str(value.as_bytes().to_vec()))),
            }
            return;
        }
        let index = match items.iter().position(|(k, v)| *k == key && matches!(v, Value::Array(_))) {
            Some(index) => index,
            None => {
                items.retain(|(k, _)| *k != key);
                items.push((key, Value::Array(Vec::new())));
                items.len() - 1
            }
        };
        insert(&mut items[index].1, rest, value);
    }
    let mut result: Option<Value> = None;
    for (field, value) in form {
        let Some(rest) = field.strip_prefix(name) else { continue };
        if rest.is_empty() {
            result = Some(Value::Str(value.as_bytes().to_vec()));
            continue;
        }
        if !rest.starts_with('[') {
            continue;
        }
        let keys: Vec<String> = rest.split('[').skip(1).map_while(|part| part.strip_suffix(']').map(str::to_owned)).collect();
        if keys.is_empty() {
            continue;
        }
        if !matches!(result, Some(Value::Array(_))) {
            result = Some(Value::Array(Vec::new()));
        }
        if let Some(target) = result.as_mut() {
            insert(target, &keys, value);
        }
    }
    result
}

/// A page a plugin adds to the menu of the administration (`add_submenu_page()` and shortcuts such as
/// `add_options_page()`).
#[derive(Debug, Clone, PartialEq)]
pub struct AdminPage {
    /// The file of the parent menu (`options-general.php`).
    pub parent: &'static str,
    /// `$menu_slug`, the `page` argument of the address.
    pub slug: &'static str,
    pub page_title: String,
    pub menu_title: String,
    pub capability: &'static str,
}

impl AdminPage {
    /// `get_plugin_page_hookname()`: `settings_page_<slug>` below Settings.
    pub fn hook_suffix(&self) -> String {
        let parent = match self.parent {
            "index.php" => "dashboard",
            "edit.php" => "posts",
            "upload.php" => "media",
            "edit.php?post_type=page" => "pages",
            "edit-comments.php" => "comments",
            "themes.php" => "appearance",
            "plugins.php" => "plugins",
            "users.php" => "users",
            "tools.php" => "tools",
            "options-general.php" => "settings",
            other => other,
        };
        format!("{}_page_{}", parent, self.slug)
    }
}

/// What the administration hooks of a plugin see of the request and of the signed-in user.
pub struct AdminContext<'a> {
    pub site: &'a Site,
    /// Present when writing is allowed.
    pub writer: Option<&'a rustpress_db::Writer>,
    /// `$hook_suffix` of the screen (`settings_page_svg-support`, `plugins.php`).
    pub hook_suffix: &'a str,
    pub post: bool,
    /// `$_SERVER['REQUEST_URI']`.
    pub request_uri: &'a str,
    pub query: &'a [(String, String)],
    pub form: &'a [(String, String)],
    /// The request came from a page of this site (its `Origin`, else its `Referer`).
    pub same_origin: bool,
    /// `current_user_can( $capability )`.
    pub can: &'a (dyn Fn(&str) -> bool + Send + Sync),
    /// `wp_create_nonce( $action )`.
    pub nonce: &'a (dyn Fn(&str) -> String + Send + Sync),
    /// `wp_verify_nonce( $nonce, $action )`.
    pub verify_nonce: &'a (dyn Fn(&str, &str) -> bool + Send + Sync),
}

impl AdminContext<'_> {
    /// `$_GET[ $name ]`.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.query.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `$_POST[ $name ]`.
    pub fn post_field(&self, name: &str) -> Option<&str> {
        self.form.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `settings_fields( $group )`: the hidden fields of a form that `options.php` saves.
    pub fn settings_fields(&self, group: &str) -> String {
        let referer = crate::formatting::esc_url(&crate::urls::remove_query_arg(self.request_uri, "_wp_http_referer"));
        format!(
            "<input type='hidden' name='option_page' value='{}' /><input type=\"hidden\" name=\"action\" value=\"update\" /><input type=\"hidden\" id=\"_wpnonce\" name=\"_wpnonce\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{referer}\" />",
            crate::formatting::esc_attr(group),
            (self.nonce)(&format!("{group}-options"))
        )
    }
}

/// The type of a registered meta key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaType {
    Boolean,
    String,
}

/// A single meta key of posts shown in the REST API.
#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredMeta {
    pub key: &'static str,
    /// The post type, or `None` for every post type.
    pub post_type: Option<&'static str>,
    pub kind: MetaType,
}

impl RegisteredMeta {
    /// The value the REST API shows for the stored value (`get_metadata( ..., true )` and the schema's type).
    pub fn rest_value(&self, stored: Option<&str>) -> serde_json::Value {
        match self.kind {
            // `rest_sanitize_boolean()` of the stored string; without a row the default `false`.
            MetaType::Boolean => serde_json::Value::Bool(stored.is_some_and(|v| !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"))),
            MetaType::String => serde_json::Value::String(stored.unwrap_or_default().to_owned()),
        }
    }
}

/// A stylesheet or script a plugin enqueues (`wp_enqueue_style()`, `wp_enqueue_script()`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Asset {
    pub handle: String,
    pub src: String,
    pub deps: Vec<String>,
    /// `ver`; `None` prints none.
    pub ver: Option<String>,
    /// Styles: `media`.
    pub media: String,
    /// Scripts: printed in the footer.
    pub in_footer: bool,
    /// The priority of the `wp_enqueue_scripts` callback that enqueues it (10 by default).
    pub priority: i32,
    /// Scripts: the data `wp_localize_script()` adds (`var name = {...};`).
    pub extra: Option<String>,
    /// Scripts: `wp_add_inline_script( $handle, $data, 'before' )`.
    pub before: Vec<String>,
    /// Scripts: `wp_add_inline_script( $handle, $data )` after the script.
    pub after: Vec<String>,
}

/// What the plugins enqueue on a page, in the order they enqueue it.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub styles: Vec<Asset>,
    pub scripts: Vec<Asset>,
}

impl Assets {
    /// `wp_enqueue_style( $handle, $src, $deps, $ver, $media )`.
    pub fn style(&mut self, handle: &str, src: String, deps: &[&str], ver: &str, media: &str) -> &mut Asset {
        self.styles.push(Asset { handle: handle.to_owned(), src, deps: deps.iter().map(|d| (*d).to_owned()).collect(), ver: Some(ver.to_owned()), media: media.to_owned(), priority: 10, ..Asset::default() });
        self.styles.last_mut().expect("just pushed")
    }

    /// `wp_enqueue_script( $handle, $src, $deps, $ver, $in_footer )`.
    pub fn script(&mut self, handle: &str, src: String, deps: &[&str], ver: &str, in_footer: bool) -> &mut Asset {
        self.scripts.push(Asset { handle: handle.to_owned(), src, deps: deps.iter().map(|d| (*d).to_owned()).collect(), ver: Some(ver.to_owned()), in_footer, priority: 10, ..Asset::default() });
        self.scripts.last_mut().expect("just pushed")
    }
}

static REGISTERED: OnceLock<Vec<&'static dyn NativePlugin>> = OnceLock::new();

/// Makes the modules known; the first call wins.
pub fn register(plugins: Vec<&'static dyn NativePlugin>) {
    let _ = REGISTERED.set(plugins);
}

/// The registered modules.
pub fn registered() -> &'static [&'static dyn NativePlugin] {
    REGISTERED.get().map_or(&[], Vec::as_slice)
}

/// The native modules of the active plugins, in the order of `active_plugins` (the order WordPress loads them in).
#[derive(Default)]
pub struct Plugins {
    active: Vec<(&'static dyn NativePlugin, Box<dyn ActivePlugin>)>,
}

impl Plugins {
    /// Loads the modules of `active` (plugin files). A module that fails to load is left out.
    pub async fn load(site: &Site, active: &[String]) -> Self {
        let mut loaded = Vec::new();
        for file in active {
            let Some(plugin) = registered().iter().find(|p| p.file() == file) else { continue };
            match plugin.load(site).await {
                Ok(instance) => loaded.push((*plugin, instance)),
                Err(e) => tracing::error!(plugin = file.as_str(), error = %e, "native plugin failed to load"),
            }
        }
        Self { active: loaded }
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// `is_plugin_active( $file )` for a plugin with a native module.
    pub fn is_active(&self, file: &str) -> bool {
        self.active.iter().any(|(p, _)| p.file() == file)
    }

    fn hooks(&self) -> impl Iterator<Item = &dyn ActivePlugin> {
        self.active.iter().map(|(_, instance)| instance.as_ref())
    }

    /// The assets of `wp_enqueue_scripts`, in the order the callbacks run (by priority, then by plugin).
    pub fn assets(&self, site: &Site) -> Assets {
        let mut assets = Assets::default();
        for plugin in self.hooks() {
            plugin.enqueue(site, &mut assets);
        }
        assets.styles.sort_by_key(|a| a.priority);
        assets.scripts.sort_by_key(|a| a.priority);
        assets
    }

    /// Whether a plugin hides the dates while `post` is the post of the loop.
    pub async fn hides_date(&self, site: &Site, post: &Post) -> Result<bool> {
        for plugin in self.hooks() {
            if plugin.hides_date(site, post).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The post meta the plugins register for the REST API for posts of `post_type`, in the order of registration.
    pub fn registered_post_meta(&self, post_type: &str) -> Vec<RegisteredMeta> {
        self.hooks().flat_map(|plugin| plugin.registered_post_meta()).filter(|m| m.post_type.is_none_or(|t| t == post_type)).collect()
    }

    /// The pages the plugins add to the menu of the administration, in the order they add them.
    pub fn admin_pages(&self, site: &Site) -> Vec<AdminPage> {
        self.hooks().flat_map(|plugin| plugin.admin_pages(site)).collect()
    }

    /// The plugin that added the page `slug` below `parent`, with the page.
    pub fn admin_page_of(&self, site: &Site, parent: &str, slug: &str) -> Option<(&dyn ActivePlugin, AdminPage)> {
        self.hooks().find_map(|plugin| plugin.admin_pages(site).into_iter().find(|p| p.slug == slug && (p.parent == parent || parent == "admin.php")).map(|page| (plugin, page)))
    }

    /// The assets of `admin_enqueue_scripts`, by priority, then by plugin.
    pub fn admin_assets(&self, ctx: &AdminContext<'_>) -> Assets {
        let mut assets = Assets::default();
        for plugin in self.hooks() {
            plugin.admin_enqueue(ctx, &mut assets);
        }
        assets.styles.sort_by_key(|a| a.priority);
        assets.scripts.sort_by_key(|a| a.priority);
        assets
    }

    /// The text domains of the active plugins.
    pub fn text_domains(&self) -> Vec<&'static str> {
        self.active.iter().filter_map(|(p, _)| p.text_domain()).collect()
    }

    /// The help tabs of the plugin page `slug`.
    pub fn admin_page_help(&self, site: &Site, slug: &str) -> AdminHelp {
        self.hooks().find(|plugin| plugin.admin_pages(site).iter().any(|p| p.slug == slug)).map(|plugin| plugin.admin_page_help(slug, site)).unwrap_or_default()
    }

    /// `admin_footer_text` of the plugins.
    pub fn admin_footer_text(&self, ctx: &AdminContext<'_>) -> Option<String> {
        self.hooks().find_map(|plugin| plugin.admin_footer_text(ctx))
    }

    /// The options the plugins registered for the settings group `group`, with the plugin that registered each.
    pub fn registered_settings(&self, group: &str) -> Vec<(&'static str, &dyn ActivePlugin)> {
        self.hooks().flat_map(|plugin| plugin.registered_settings().into_iter().filter(|(g, _)| *g == group).map(move |(_, option)| (option, plugin))).collect()
    }

    /// The assets of `enqueue_block_editor_assets`.
    pub fn block_editor_assets(&self, ctx: &AdminContext<'_>) -> Assets {
        let mut assets = Assets::default();
        for plugin in self.hooks() {
            plugin.block_editor_assets(ctx, &mut assets);
        }
        assets
    }

    /// `upload_mimes` of the plugins on top of the core's types.
    pub fn upload_mimes(&self, uploader: &Uploader<'_>, mut mimes: Vec<(String, String)>) -> Vec<(String, String)> {
        for plugin in self.hooks() {
            plugin.upload_mimes(uploader, &mut mimes);
        }
        mimes
    }

    /// `wp_handle_upload_prefilter` of the plugins; the first refusal ends it.
    pub fn upload_prefilter(&self, uploader: &Uploader<'_>, name: &mut String, bytes: &mut Vec<u8>) -> std::result::Result<(), String> {
        for plugin in self.hooks() {
            plugin.upload_prefilter(uploader, name, bytes)?;
        }
        Ok(())
    }

    /// A plugin's `wp_check_filetype_and_ext` for a file the core refused.
    pub fn accept_filetype(&self, name: &str, mimes: &[(String, String)]) -> Option<(String, String)> {
        self.hooks().find_map(|plugin| plugin.accept_filetype(name, mimes))
    }

    /// `wp_generate_attachment_metadata` of the plugins.
    pub fn attachment_metadata(&self, site: &Site, mime: &str, path: &std::path::Path, relative: &str, metadata: rustpress_phpser::Value) -> rustpress_phpser::Value {
        self.hooks().fold(metadata, |metadata, plugin| plugin.attachment_metadata(site, mime, path, relative, metadata))
    }

    /// `wp_prepare_attachment_for_js` of the plugins.
    pub fn prepare_attachment_for_js(&self, mime: &str, path: Option<&std::path::Path>, response: &mut serde_json::Map<String, serde_json::Value>) {
        for plugin in self.hooks() {
            plugin.prepare_attachment_for_js(mime, path, response);
        }
    }

    /// `post_thumbnail_html` of the plugins.
    pub async fn post_thumbnail_html(&self, site: &Site, post_id: u64, singular: bool, html: String) -> Result<String> {
        let mut html = html;
        for plugin in self.hooks() {
            html = plugin.post_thumbnail_html(site, post_id, singular, html).await?;
        }
        Ok(html)
    }

    /// The page after the plugins' output buffers.
    pub fn admin_page_output(&self, ctx: &AdminContext<'_>, html: String) -> String {
        self.hooks().fold(html, |html, plugin| plugin.admin_page_output(ctx, html))
    }

    /// What the plugins print on `admin_notices`.
    pub async fn admin_notices(&self, ctx: &AdminContext<'_>) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.admin_notices(ctx).await?);
        }
        Ok(out)
    }

    /// `plugin_action_links_{$file}` of the plugin `file`.
    pub fn plugin_action_links(&self, site: &Site, file: &str, links: Vec<String>) -> Vec<String> {
        match self.active.iter().find(|(p, _)| p.file() == file) {
            Some((_, plugin)) => plugin.plugin_action_links(site, links),
            None => links,
        }
    }

    /// `plugin_row_meta` of every plugin for the row of `file`.
    pub fn plugin_row_meta(&self, site: &Site, file: &str, links: Vec<String>) -> Vec<String> {
        self.hooks().fold(links, |links, plugin| plugin.plugin_row_meta(site, file, links))
    }

    /// `wp_ajax_{$action}` of the plugins.
    pub async fn admin_ajax(&self, action: &str, ctx: &AdminContext<'_>) -> Result<Option<AjaxAnswer>> {
        for plugin in self.hooks() {
            if let Some(answer) = plugin.admin_ajax(action, ctx).await? {
                return Ok(Some(answer));
            }
        }
        Ok(None)
    }

    /// What the plugins print in `wp_footer`.
    pub async fn wp_footer(&self, site: &Site, looped: &[u64]) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.wp_footer(site, looped).await?);
        }
        Ok(out)
    }
}
