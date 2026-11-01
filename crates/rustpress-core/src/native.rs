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

    /// `activate_{$plugin}`: what the plugin sets up when `user` activates it (its options, first content).
    fn activate<'a>(&'a self, _site: &'a Site, _writer: &'a rustpress_db::Writer, _user: u64, _now: i64) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// Runs the activation of the native module of `file`, if there is one.
pub async fn activate(file: &str, site: &Site, writer: &rustpress_db::Writer, user: u64, now: i64) -> Result<()> {
    match registered().iter().find(|p| p.file() == file) {
        Some(plugin) => plugin.activate(site, writer, user, now).await,
        None => Ok(()),
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

    /// `wp_head` at priority 1, after `wp_robots()`, with what the plugin changes around it: the `wp_robots` filter
    /// (gets the directives of the core filters), `pre_get_document_title` and core actions it removes from `wp_head`.
    fn wp_head<'a>(&'a self, _ctx: &'a HeadContext<'a>, _robots: &'a [(String, Robot)]) -> BoxFuture<'a, Result<Option<Head>>> {
        Box::pin(async { Ok(None) })
    }

    /// A request the plugin answers itself before the main query (its own rewrite rules at the top, handled on
    /// `pre_get_posts` or `parse_request`): a sitemap, a stylesheet, a redirect, or a 404 page.
    fn serve<'a>(&'a self, _site: &'a Site, _request: &'a ServeRequest<'a>) -> BoxFuture<'a, Result<Option<Serve>>> {
        Box::pin(async { Ok(None) })
    }

    /// `register_rest_field()`: fields the plugin adds to an object of the REST API, in order.
    fn rest_fields<'a>(&'a self, _object: RestObject<'a>, _ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<(String, serde_json::Value)>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    /// What the plugin changes in a feed: the `get_bloginfo_rss( 'url' )` filter and the headers it sends on
    /// `do_feed_*`.
    fn feed<'a>(&'a self, _ctx: &'a FeedContext<'a>) -> BoxFuture<'a, Result<Option<FeedChanges>>> {
        Box::pin(async { Ok(None) })
    }

    /// The `the_content_feed` (`excerpt` false) and `the_excerpt_rss` filters of an item of a feed.
    fn feed_item<'a>(&'a self, _site: &'a Site, _post: &'a Post, _excerpt: bool, text: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(text) })
    }

    /// `pre_handle_404`: the feed is answered with the 404 page instead.
    fn feed_not_found(&self, _query: &crate::QueryResult) -> bool {
        false
    }

    /// Dynamic blocks of the plugin (`register_block_type()` with a `render_callback`).
    fn block_names(&self) -> &'static [&'static str] {
        &[]
    }

    /// The `render_callback` of one of [ActivePlugin::block_names].
    fn render_block<'a>(&'a self, _name: &'a str, _attrs: &'a serde_json::Map<String, serde_json::Value>, _ctx: &'a BlockContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The `robots_txt` filter: the text of `robots.txt`.
    fn robots_txt(&self, _site: &Site, text: String) -> String {
        text
    }

    /// Headers the plugin sends with `robots.txt` (`template_redirect` while `is_robots()`).
    fn robots_txt_headers(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// The `wp_sitemaps_enabled` filter returns `false`: the plugin replaces the sitemaps of WordPress.
    fn disables_core_sitemaps(&self) -> bool {
        false
    }

    /// `template_redirect`: a redirect instead of the page (before `redirect_canonical()`).
    fn template_redirect<'a>(&'a self, _site: &'a Site, _query: &'a crate::QueryResult) -> BoxFuture<'a, Result<Option<Redirect>>> {
        Box::pin(async { Ok(None) })
    }

    /// Whether the plugin calls `wp_reset_query()` on `wp_head`: `wp_reset_postdata()` then fires `the_post` for the
    /// global post (the first post of the main query), before a classic theme prints its header and after a block
    /// theme rendered its template.
    fn resets_query_in_head(&self) -> bool {
        false
    }

    /// The `loginout` and `register` filters: the link of `wp_loginout()` and `wp_register()`.
    fn loginout(&self, link: String) -> String {
        link
    }

    /// The `comment_reply_link` filter: the link of `get_comment_reply_link()`.
    fn comment_reply_link(&self, link: String) -> String {
        link
    }

    /// `admin_menu`: the pages the plugin adds to the menu of the administration.
    fn admin_pages(&self, _site: &Site) -> Vec<AdminPage> {
        Vec::new()
    }

    /// The screen options of a page the plugin added (`add_screen_option()` and the columns of its list table).
    fn admin_screen_options(&self, _slug: &str, _site: &Site, _query: &[(String, String)]) -> Option<PageScreenOptions> {
        None
    }

    /// The per-page options of its screens the plugin lets `set_screen_options()` store
    /// (`set_screen_option_{$option}`).
    fn screen_option_names(&self) -> &'static [&'static str] {
        &[]
    }

    /// `load-{$page_hook}` of a page the plugin added: what happens before the page is printed (an action that
    /// redirects, or stops with an error). `None` prints the page.
    fn admin_page_load<'a>(&'a self, _slug: &'a str, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<PageLoad>>> {
        Box::pin(async { Ok(None) })
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
    fn admin_page_help(&self, _slug: &str, _site: &Site, _query: &[(String, String)]) -> AdminHelp {
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

    /// The shortcodes the plugin registers (`add_shortcode()`).
    fn shortcode_tags(&self) -> &'static [&'static str] {
        &[]
    }

    /// The callback of one of [`shortcode_tags`](Self::shortcode_tags): what `[tag atts]content[/tag]` turns into.
    fn shortcode<'a>(&'a self, _tag: &'a str, _atts: &'a crate::shortcodes::Atts, _content: Option<&'a str>, _ctx: &'a ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The routes of the plugin's REST API as `GET /wp-json/` lists them (`register_rest_route()` on
    /// `rest_api_init`): a JSON object of route and description, `@REST@` standing for the address of the API. A
    /// `schema` of a description is only sent to `OPTIONS`.
    fn rest_index(&self) -> &'static str {
        ""
    }

    /// A request to the plugin's REST API: the answer, or `None` when no endpoint of the plugin takes it.
    fn rest<'a>(&'a self, _request: &'a RestRequest<'a>) -> BoxFuture<'a, Result<Option<RestAnswer>>> {
        Box::pin(async { Ok(None) })
    }
}

/// A request to the REST API of a plugin.
pub struct RestRequest<'a> {
    pub site: &'a Site,
    pub method: &'a str,
    /// The route after `/wp-json` (`/contact-form-7/v1/contact-forms/5/feedback`).
    pub route: &'a str,
    /// The path and the query string of the request as they were sent (`/wp-json/...`, or `/` with
    /// `rest_route=...` for plain permalinks).
    pub path: &'a str,
    pub query: &'a str,
    /// The fields of a form body, as they were sent (`boxes[]` keeps its brackets).
    pub fields: &'a [(String, String)],
    pub files: &'a [RestFile],
    /// The `Content-Type` header.
    pub content_type: &'a str,
    /// `$_SERVER['REMOTE_ADDR']`, `HTTP_USER_AGENT` and `HTTP_REFERER`.
    pub remote_addr: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub referer: Option<&'a str>,
    /// The signed-in user.
    pub user: Option<&'a crate::Viewer>,
    /// `time()`.
    pub now: i64,
    /// `wp_mail()`, when the site can send e-mail.
    pub mailer: Option<&'a dyn Mailer>,
}

/// A file of a multipart body.
#[derive(Debug, Clone)]
pub struct RestFile {
    pub field: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// What a plugin's endpoint answers: `rest_ensure_response( $data )`, or a `WP_Error` as its JSON with the status.
#[derive(Debug, Clone, PartialEq)]
pub struct RestAnswer {
    pub status: u16,
    pub data: serde_json::Value,
    /// The request passed the `permission_callback`: the response names the methods of the route (`Allow`).
    pub permitted: bool,
}

impl RestAnswer {
    pub fn ok(data: serde_json::Value) -> Self {
        Self { status: 200, data, permitted: true }
    }

    /// `new WP_Error( $code, $message, array( 'status' => $status ) )` of the callback.
    pub fn error(code: &str, message: &str, status: u16) -> Self {
        Self { status, data: serde_json::json!({"code": code, "message": message, "data": {"status": status}}), permitted: true }
    }

    /// The error of a `permission_callback`.
    pub fn forbidden(code: &str, message: &str, status: u16) -> Self {
        Self { permitted: false, ..Self::error(code, message, status) }
    }
}

/// The arguments of `wp_mail( $to, $subject, $message, $headers, $attachments )`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Email {
    /// One address or several, separated by commas.
    pub to: String,
    pub subject: String,
    pub body: String,
    /// `Name: value` lines (`From`, `Content-Type`, `Reply-To`, `Cc`, `Bcc`, others are passed on).
    pub headers: Vec<String>,
    pub attachments: Vec<Attachment>,
}

/// A file attached to an [Email].
#[derive(Debug, Clone, PartialEq)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

/// `wp_mail()`: `false` when the message could not be sent.
pub trait Mailer: Send + Sync {
    fn send<'a>(&'a self, email: &'a Email) -> BoxFuture<'a, bool>;
}

/// What a plugin's block sees of the page it is rendered on.
pub struct BlockContext<'a> {
    pub site: &'a Site,
    pub query: &'a crate::QueryResult,
    pub services: &'a dyn PageServices,
    /// `get_the_ID()`: the post whose content is rendered.
    pub post: Option<&'a Post>,
    /// `wp_is_serving_rest_request()`.
    pub rest: bool,
}

/// A feed a plugin may change.
pub struct FeedContext<'a> {
    pub site: &'a Site,
    pub query: &'a crate::QueryResult,
    pub services: &'a dyn PageServices,
    /// `rss2`, `rss`, `atom` or `rdf`.
    pub kind: &'a str,
    /// A feed of comments.
    pub comments: bool,
}

/// What a plugin changes in a feed.
#[derive(Debug, Default)]
pub struct FeedChanges {
    /// `get_bloginfo_rss( 'url' )`.
    pub url: Option<String>,
    /// Headers added to the feed.
    pub headers: Vec<(String, String)>,
}

/// An object of the REST API a plugin may add fields to.
#[derive(Clone, Copy)]
pub enum RestObject<'a> {
    Post(&'a Post),
    Term(&'a rustpress_db::Term),
    User(&'a rustpress_db::User),
    /// A post type of `wp/v2/types`.
    PostType(&'a str),
}

/// What a plugin sees while it adds fields to the REST API (no main query ran).
pub struct RestFieldContext<'a> {
    pub site: &'a Site,
    /// The empty query of a REST request.
    pub query: &'a crate::QueryResult,
    pub services: &'a dyn PageServices,
}

/// A request as a plugin's own rewrite rules see it.
pub struct ServeRequest<'a> {
    /// The path below the site's home, without the leading slash (`sitemap_index.xml`).
    pub path: &'a str,
    /// The raw query string.
    pub query: &'a str,
    /// `ABSPATH`.
    pub wp_root: &'a std::path::Path,
}

/// What a plugin answers to a request it serves.
#[derive(Debug)]
pub enum Serve {
    Response(PluginResponse),
    /// The theme's 404 page (`$wp_query->set_404()`).
    NotFound,
}

/// A complete answer of a plugin.
#[derive(Debug, Default)]
pub struct PluginResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// A redirect of a plugin (`wp_safe_redirect( $location, $status, $x_redirect_by )`).
#[derive(Debug, Clone)]
pub struct Redirect {
    pub location: String,
    pub status: u16,
    /// The `X-Redirect-By` header.
    pub by: &'static str,
}

/// A directive of `wp_robots()`: `Flag(true)` prints its name, `Value` prints `name:value`, `Flag(false)` nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Robot {
    Flag(bool),
    Value(String),
}

/// `wp_robots()`: the meta tag of the directives, or nothing when none is printed.
pub fn robots_tag(robots: &[(String, Robot)]) -> String {
    let strings: Vec<String> = robots
        .iter()
        .filter_map(|(name, value)| match value {
            Robot::Value(v) => Some(format!("{name}:{v}")),
            Robot::Flag(true) => Some(name.clone()),
            Robot::Flag(false) => None,
        })
        .collect();
    if strings.is_empty() {
        return String::new();
    }
    format!("<meta name='robots' content='{}' />\n", crate::formatting::esc_attr(&strings.join(", ")))
}

/// What the theme offers a plugin that prints into the head (functions that need the rendering environment).
pub trait PageServices: Send + Sync {
    /// `get_the_excerpt( $post )` outside the loop.
    fn excerpt<'a>(&'a self, post: &'a Post) -> BoxFuture<'a, Result<String>>;

    /// `ABSPATH`: where the WordPress files are (uploads under `wp-content/uploads`).
    fn wp_root(&self) -> &std::path::Path;
}

/// What a plugin sees of the page whose head is printed.
pub struct HeadContext<'a> {
    pub site: &'a Site,
    pub query: &'a crate::QueryResult,
    pub services: &'a dyn PageServices,
    /// `current_theme_supports( 'title-tag' )`: a classic theme that lets WordPress print the title (block themes
    /// print it from their template, after `wp_head` at priority 1).
    pub title_tag: bool,
    /// `is_admin_bar_showing()`.
    pub admin_bar: bool,
    /// What `render_block` saw before the head (block themes render the template first): a `core/query` that does
    /// not inherit the main query, and the last `core/query-pagination-next`/`-previous`.
    pub custom_query_loop: bool,
    pub pagination_next: Option<String>,
    pub pagination_previous: Option<String>,
}

/// What a plugin changes in the head.
#[derive(Debug, Default)]
pub struct Head {
    /// The directives after the `wp_robots` filter.
    pub robots: Option<Vec<(String, Robot)>>,
    /// `pre_get_document_title`, unescaped.
    pub title: Option<String>,
    /// Printed on `wp_head` at priority 1, right after the robots tag.
    pub html: String,
    /// Core actions removed from `wp_head`: `rel_canonical`, `_wp_render_title_tag`.
    pub removed: Vec<&'static str>,
}

/// What the shortcode of a plugin sees of the page it is printed on.
pub struct ShortcodeContext<'a> {
    pub site: &'a Site,
    pub query: &'a crate::QueryResult,
    /// `get_the_ID()` while `in_the_loop()`.
    pub loop_post: Option<u64>,
    /// What the plugins keep for the rest of the request (their `static` variables).
    pub state: &'a std::sync::Mutex<PluginState>,
    pub services: &'a dyn PageServices,
}

/// Per-request state of the plugins: counters and lists by a name the plugin picks.
#[derive(Debug, Default)]
pub struct PluginState {
    pub counters: std::collections::HashMap<&'static str, u64>,
    pub lists: std::collections::HashMap<&'static str, Vec<String>>,
}

impl PluginState {
    /// Adds one to the counter `name` and returns the new count.
    pub fn next(&mut self, name: &'static str) -> u64 {
        let count = self.counters.entry(name).or_insert(0);
        *count += 1;
        *count
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

/// The screen options of a plugin's page: the columns of its list (key and title, in order), "Number of items per
/// page" (the option and its default) and the boxes of a dashboard.
#[derive(Debug, Clone, PartialEq)]
pub struct PageScreenOptions {
    pub columns: Vec<(String, String)>,
    pub per_page: Option<(&'static str, u64)>,
    /// The boxes of a dashboard (id and title), for "Screen elements".
    pub meta_boxes: Vec<(String, String)>,
}

/// The end of a `load-{$page_hook}` callback that does not print the page.
#[derive(Debug, Clone, PartialEq)]
pub enum PageLoad {
    /// `wp_safe_redirect( $location ); exit();`
    Redirect(String),
    /// `wp_die( $message, $title, $status )`; the message is HTML.
    Die { status: u16, title: String, message: String },
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

/// A page a plugin adds to the menu of the administration: `add_menu_page()` (no parent), `add_submenu_page()` and
/// shortcuts such as `add_options_page()`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdminPage {
    /// The file of the parent menu (`options-general.php`), the slug of a plugin's own menu, or empty for an entry of
    /// the main menu.
    pub parent: &'static str,
    /// `$menu_slug`, the `page` argument of the address.
    pub slug: &'static str,
    pub page_title: String,
    pub menu_title: String,
    pub capability: &'static str,
    /// The icon of an entry of the main menu (`dashicons-email`).
    pub icon: &'static str,
    /// `$position` in the main menu; without one the entry comes last.
    pub position: Option<u32>,
}

impl AdminPage {
    /// An entry of the main menu (`add_menu_page()`).
    pub fn is_top_level(&self) -> bool {
        self.parent.is_empty()
    }

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
    /// The signed-in user (`get_current_user_id()`).
    pub user: u64,
    /// `time()`.
    pub now: i64,
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
    /// Registered but not enqueued (`wp_register_script()`): printed only as a dependency of another script.
    pub registered_only: bool,
    /// Scripts: the text domain of `wp_set_script_translations()`.
    pub translations: Option<String>,
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

    /// `get_plugin_page_hookname()`: `toplevel_page_<slug>` for an entry of the main menu, `<menu title>_page_<slug>`
    /// below a plugin's own menu, `settings_page_<slug>` and the like below the core's menus.
    pub fn page_hook(&self, site: &Site, page: &AdminPage) -> String {
        if page.is_top_level() {
            return format!("toplevel_page_{}", page.slug);
        }
        match self.admin_pages(site).into_iter().find(|p| p.is_top_level() && p.slug == page.parent) {
            Some(top) => format!("{}_page_{}", crate::slug::sanitize_title(&top.menu_title), page.slug),
            None => page.hook_suffix(),
        }
    }

    /// Whether a plugin lets `set_screen_options()` store `option`.
    pub fn stores_screen_option(&self, option: &str) -> bool {
        self.hooks().any(|plugin| plugin.screen_option_names().contains(&option))
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
    pub fn admin_page_help(&self, site: &Site, slug: &str, query: &[(String, String)]) -> AdminHelp {
        self.hooks().find(|plugin| plugin.admin_pages(site).iter().any(|p| p.slug == slug)).map(|plugin| plugin.admin_page_help(slug, site, query)).unwrap_or_default()
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

    /// The shortcodes of the plugins, in the order they register them.
    pub fn shortcode_tags(&self) -> Vec<&'static str> {
        self.hooks().flat_map(|plugin| plugin.shortcode_tags().iter().copied()).collect()
    }

    /// The output of the plugin shortcode `tag`, `None` when no plugin registers it.
    pub async fn shortcode(&self, tag: &str, atts: &crate::shortcodes::Atts, content: Option<&str>, ctx: &ShortcodeContext<'_>) -> Result<Option<String>> {
        for plugin in self.hooks() {
            if plugin.shortcode_tags().contains(&tag) {
                return plugin.shortcode(tag, atts, content, ctx).await.map(Some);
            }
        }
        Ok(None)
    }

    /// The routes of the plugins' REST APIs (see [`ActivePlugin::rest_index`]), in the order they register them.
    pub fn rest_index(&self) -> Vec<&'static str> {
        self.hooks().map(|plugin| plugin.rest_index()).filter(|index| !index.is_empty()).collect()
    }

    /// The answer of the plugin whose endpoint takes `request`.
    pub async fn rest(&self, request: &RestRequest<'_>) -> Result<Option<RestAnswer>> {
        for plugin in self.hooks() {
            if let Some(answer) = plugin.rest(request).await? {
                return Ok(Some(answer));
            }
        }
        Ok(None)
    }

    /// The first redirect a plugin asks for on `template_redirect`.
    pub async fn template_redirect(&self, site: &Site, query: &crate::QueryResult) -> Result<Option<Redirect>> {
        for plugin in self.hooks() {
            if let Some(redirect) = plugin.template_redirect(site, query).await? {
                return Ok(Some(redirect));
            }
        }
        Ok(None)
    }

    /// The first plugin that answers the request itself.
    pub async fn serve(&self, site: &Site, request: &ServeRequest<'_>) -> Result<Option<Serve>> {
        for plugin in self.hooks() {
            if let Some(answer) = plugin.serve(site, request).await? {
                return Ok(Some(answer));
            }
        }
        Ok(None)
    }

    /// The fields the plugins add to an object of the REST API.
    pub async fn rest_fields(&self, object: RestObject<'_>, ctx: &RestFieldContext<'_>) -> Result<Vec<(String, serde_json::Value)>> {
        let mut fields = Vec::new();
        for plugin in self.hooks() {
            fields.extend(plugin.rest_fields(object, ctx).await?);
        }
        Ok(fields)
    }

    /// Whether a plugin renders the block.
    pub fn renders_block(&self, name: &str) -> bool {
        self.hooks().any(|p| p.block_names().contains(&name))
    }

    /// The output of a plugin's block.
    pub async fn render_block(&self, name: &str, attrs: &serde_json::Map<String, serde_json::Value>, ctx: &BlockContext<'_>) -> Result<String> {
        match self.hooks().find(|p| p.block_names().contains(&name)) {
            Some(plugin) => plugin.render_block(name, attrs, ctx).await,
            None => Ok(String::new()),
        }
    }

    /// What the plugins change in a feed (the last URL wins, headers add up).
    pub async fn feed(&self, ctx: &FeedContext<'_>) -> Result<FeedChanges> {
        let mut all = FeedChanges::default();
        for plugin in self.hooks() {
            if let Some(changes) = plugin.feed(ctx).await? {
                all.url = changes.url.or(all.url);
                all.headers.extend(changes.headers);
            }
        }
        Ok(all)
    }

    /// An item of a feed after the plugins' filters.
    pub async fn feed_item(&self, site: &Site, post: &Post, excerpt: bool, mut text: String) -> Result<String> {
        for plugin in self.hooks() {
            text = plugin.feed_item(site, post, excerpt, text).await?;
        }
        Ok(text)
    }

    /// Whether a plugin turns the feed into a 404 page.
    pub fn feed_not_found(&self, query: &crate::QueryResult) -> bool {
        self.hooks().any(|p| p.feed_not_found(query))
    }

    /// `robots.txt` after the plugins' `robots_txt` filters.
    pub fn robots_txt(&self, site: &Site, text: String) -> String {
        self.hooks().fold(text, |text, plugin| plugin.robots_txt(site, text))
    }

    /// The headers the plugins add to `robots.txt`.
    pub fn robots_txt_headers(&self) -> Vec<(&'static str, &'static str)> {
        self.hooks().flat_map(|p| p.robots_txt_headers().iter().copied()).collect()
    }

    /// Whether a plugin turns the sitemaps of WordPress off.
    pub fn disables_core_sitemaps(&self) -> bool {
        self.hooks().any(|p| p.disables_core_sitemaps())
    }

    /// Whether a plugin fires `the_post` for the global post on `wp_head` ([ActivePlugin::resets_query_in_head]).
    pub fn resets_query_in_head(&self) -> bool {
        self.hooks().any(|p| p.resets_query_in_head())
    }

    /// The link of `wp_loginout()` after the plugins' `loginout` filters.
    pub fn loginout(&self, link: String) -> String {
        self.hooks().fold(link, |link, plugin| plugin.loginout(link))
    }

    /// The link of `get_comment_reply_link()` after the plugins' `comment_reply_link` filters.
    pub fn comment_reply_link(&self, link: String) -> String {
        self.hooks().fold(link, |link, plugin| plugin.comment_reply_link(link))
    }

    /// What the plugins print in `wp_footer`.
    pub async fn wp_footer(&self, site: &Site, looped: &[u64]) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.wp_footer(site, looped).await?);
        }
        Ok(out)
    }

    /// What the plugins change in the head; `robots` are the directives of core's `wp_robots` filters.
    pub async fn wp_head(&self, ctx: &HeadContext<'_>, robots: Vec<(String, Robot)>) -> Result<Head> {
        let mut head = Head { robots: Some(robots), ..Head::default() };
        for plugin in self.hooks() {
            let current = head.robots.take().unwrap_or_default();
            let Some(changed) = plugin.wp_head(ctx, &current).await? else {
                head.robots = Some(current);
                continue;
            };
            head.robots = Some(changed.robots.unwrap_or(current));
            head.title = changed.title.or(head.title);
            head.html.push_str(&changed.html);
            head.removed.extend(changed.removed);
        }
        Ok(head)
    }
}
