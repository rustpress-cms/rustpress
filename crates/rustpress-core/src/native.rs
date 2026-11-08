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

    /// [NativePlugin::load] for a module that also reads files of the installation under `wp_root` (`ABSPATH`).
    fn load_at<'a>(&'a self, site: &'a Site, _wp_root: &'a std::path::Path) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        self.load(site)
    }
    /// The text domain WordPress loads just in time from `wp-content/languages/plugins/<domain>-<locale>.mo`.
    fn text_domain(&self) -> Option<&'static str> {
        None
    }

    /// `activate_{$plugin}`: what the plugin sets up when `user` activates it (its options, first content).
    fn activate<'a>(&'a self, _site: &'a Site, _writer: &'a rustpress_db::Writer, _user: u64, _now: i64) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// The `register_post_type()` and `register_taxonomy()` calls the plugin makes on `init` from data it keeps
    /// itself (types defined in the database), in their order. Read once per snapshot, before the module loads;
    /// they are applied before the ones found in PHP sources.
    fn registrations<'a>(&'a self, _site: &'a Site, _wp_root: &'a std::path::Path) -> BoxFuture<'a, Result<Vec<crate::types::Registration>>> {
        Box::pin(async { Ok(Vec::new()) })
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

    /// `wp_footer` at the default priority, for plugins that look at the main query (`global $post`), after
    /// [ActivePlugin::wp_footer].
    fn wp_footer_query<'a>(&'a self, _site: &'a Site, _query: &'a crate::QueryResult) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `wp_enqueue_scripts` for plugins that look at the main query (`is_singular()`, `is_preview()`), after
    /// [ActivePlugin::enqueue].
    fn enqueue_query<'a>(&'a self, _site: &'a Site, _query: &'a crate::QueryResult, _assets: &'a mut Assets) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// `wp_print_footer_scripts` after `_wp_footer_scripts()` (and after the scripts of
    /// [PluginState::footer_scripts]): the last thing before `</body>`.
    fn print_footer_scripts(&self, _site: &Site, _query: &crate::QueryResult) -> String {
        String::new()
    }

    /// The `body_class` filter of the website: classes the plugin appends (before the theme's own filter).
    fn body_class<'a>(&'a self, _site: &'a Site, _query: &'a crate::QueryResult) -> BoxFuture<'a, Result<Vec<String>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    /// An output buffer the plugin starts on `template_redirect` (`ob_start( $callback )`) and flushes on
    /// `shutdown`: the whole HTML of a page of the website, as the theme printed it.
    fn template_output(&self, _site: &Site, _query: &crate::QueryResult, html: String) -> String {
        html
    }

    /// `login_head` at the default priority: what the plugin prints in the head of `wp-login.php`.
    fn login_head(&self, _site: &Site) -> String {
        String::new()
    }

    /// URLs the `wp_resource_hints` filter adds for `relation` (`dns-prefetch`, `preconnect`, `prefetch`,
    /// `prerender`).
    fn resource_hints(&self, _relation: &str) -> Vec<String> {
        Vec::new()
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

    /// The public post statuses the plugin registers (`register_post_status()`): the slug and the translated label.
    fn post_statuses(&self, _site: &Site) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// The internal post statuses the plugin registers (`register_post_status()` with `internal`): only
    /// `get_post_stati()` without arguments lists them.
    fn internal_post_statuses(&self) -> &'static [&'static str] {
        &[]
    }

    /// Links a plugin adds to an object of the REST API on `rest_prepare_{$type}` (`WP_REST_Response::add_link()`),
    /// after the core's.
    fn rest_links<'a>(&'a self, _object: RestObject<'a>, _ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<RestLink>>> {
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

    /// `init` on the website, before WordPress parses the request: the plugin may answer it, change it, or keep
    /// `redirect_canonical()` from redirecting it.
    fn init<'a>(&'a self, _site: &'a Site, _request: &'a PluginRequest<'a>) -> BoxFuture<'a, Result<Option<Init>>> {
        Box::pin(async { Ok(None) })
    }

    /// `send_headers`: headers of every page (and of the redirects `template_redirect` sends).
    fn send_headers(&self, _site: &Site, _request: &PluginRequest<'_>) -> Vec<(String, String)> {
        Vec::new()
    }

    /// The `wp_redirect` and `x_redirect_by` filters: headers a redirect sends before `X-Redirect-By` and
    /// `Location` (`header()` replaces one of the same name).
    fn wp_redirect<'a>(&'a self, _site: &'a Site, _request: &'a PluginRequest<'a>, _redirect: &'a Redirected<'a>) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    /// `template_redirect` after `redirect_canonical()` on a 404 page.
    fn not_found<'a>(&'a self, _site: &'a Site, _query: &'a crate::QueryResult, _request: &'a PluginRequest<'a>) -> BoxFuture<'a, Result<Option<Redirect>>> {
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

    /// `comment_post` (a visitor's comment was saved) and `edit_comment` (the administration saved one): the plugin
    /// saves what the form sent along. A `wp_die()` stops the request.
    fn comment_post<'a>(&'a self, _ctx: &'a CommentSaved<'a>) -> BoxFuture<'a, Result<Option<Die>>> {
        Box::pin(async { Ok(None) })
    }

    /// The `comment_form_field_comment` filter of `comment_form()`: the HTML of the comment field, after which a
    /// plugin may add its own fields.
    fn comment_form_field_comment<'a>(&'a self, _ctx: &'a CommentFormContext<'a>, html: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(html) })
    }

    /// `add_meta_boxes_comment`: what the plugin prints in the second column of `comment.php?action=editcomment`,
    /// before the meta boxes.
    fn comment_meta_boxes<'a>(&'a self, _ctx: &'a AdminContext<'a>, _comment: &'a rustpress_db::Comment) -> BoxFuture<'a, Result<MetaBoxes>> {
        Box::pin(async { Ok(MetaBoxes::default()) })
    }

    /// `{$taxonomy}_add_form_fields` of `edit-tags.php` (`term` is `None`) and `{$taxonomy}_edit_form` of `term.php`:
    /// what the plugin prints into the form of a term, and what it asks of the screen for it.
    fn term_form<'a>(&'a self, _ctx: &'a AdminContext<'a>, _taxonomy: &'a str, _term: Option<&'a rustpress_db::Term>) -> BoxFuture<'a, Result<MetaBoxes>> {
        Box::pin(async { Ok(MetaBoxes::default()) })
    }

    /// The `attachment_fields_to_edit` filter of `get_compat_media_markup()`: the fields the plugin adds to the edit form of
    /// an attachment (`in_modal` in the media modal, else on `post.php`).
    fn attachment_fields_to_edit<'a>(&'a self, _ctx: &'a AdminContext<'a>, _attachment: &'a Post, _in_modal: bool) -> BoxFuture<'a, Result<Vec<AttachmentField>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    /// The `attachment_fields_to_save` filter of `edit_post()` and `wp_ajax_save_attachment_compat()`; a `wp_die()` of
    /// the plugin ends the request.
    fn attachment_fields_to_save<'a>(&'a self, _ctx: &'a AttachmentSaved<'a>) -> BoxFuture<'a, Result<Option<Die>>> {
        Box::pin(async { Ok(None) })
    }

    /// What a plugin's `acf_enqueue_uploader()`-like call asks of a screen without boxes (`upload.php`, the attachment's
    /// `post.php`): the media library and an editor on `admin_footer`, whose `before` holds what the plugin prints there
    /// first.
    fn screen_uploader<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<MetaBoxes>> {
        Box::pin(async { Ok(MetaBoxes::default()) })
    }

    /// `wp_ajax_query-attachments` at priority -1: the sizes the plugin adds to `image_size_names_choose` for the answer.
    fn query_attachments_sizes(&self, _ctx: &AdminContext<'_>) -> Vec<String> {
        Vec::new()
    }

    /// The `wp_prepare_attachment_for_js` filter the plugin adds on `wp_ajax_query-attachments`, after the others.
    fn query_attachment_js(&self, _ctx: &AdminContext<'_>, _response: &mut serde_json::Map<String, serde_json::Value>) {}

    /// The `schema` of the fields the plugin registers with `register_rest_field()` for the route of `ctx` (its
    /// `OPTIONS`): `(field, schema)` for `schema.properties` and the arguments of the writing endpoints.
    fn rest_schema_fields<'a>(&'a self, _ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<(String, serde_json::Value)>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    /// The `parent_file` filter of `menu-header.php`: the top-level menu entry the screen belongs to.
    fn parent_file(&self, _ctx: &AdminContext<'_>, parent_file: String) -> String {
        parent_file
    }

    /// The `submenu_file` filter of `menu-header.php`: the highlighted submenu entry.
    fn submenu_file(&self, _ctx: &AdminContext<'_>, submenu_file: String) -> String {
        submenu_file
    }

    /// `trashed_post`, `untrashed_post` and `deleted_post`: after a post went to the trash, came back or was deleted.
    fn post_trash_change<'a>(&'a self, _change: &'a PostChange<'a>) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// `wp_untrash_post_status`: the status a post restored from the trash gets (`draft` by default).
    fn untrash_post_status(&self, _post: &Post, _new_status: &str, _previous: &str) -> Option<String> {
        None
    }

    /// `current_screen` of a posts list (`edit.php`, before WordPress handles its bulk actions): an action of the plugin
    /// that redirects, or stops with an error.
    fn list_load<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<PageLoad>>> {
        Box::pin(async { Ok(None) })
    }

    /// `post_updated_messages`: the message `$number` of the edit screen of `post` after a save (`Some(None)` for
    /// `false`, which prints nothing); `nonce` is `wp_create_nonce()` of the signed-in user.
    fn post_updated_message(&self, _site: &Site, _post: &Post, _number: u32, _nonce: &dyn Fn(&str) -> String) -> Option<Option<String>> {
        None
    }

    /// `get_user_option_screen_layout_{$screen_id}`: the number of columns the plugin sets for a screen.
    fn screen_layout(&self, _ctx: &AdminContext<'_>, _screen_id: &str) -> Option<u32> {
        None
    }

    /// The `screen_settings` filter: what the plugin appends to the settings of "Screen Options".
    fn screen_settings<'a>(&'a self, _ctx: &'a AdminContext<'a>, html: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(html) })
    }

    /// `in_admin_header` after the toolbar of the administration: what the plugin prints before `#wpbody`.
    fn in_admin_header<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The `update_footer` filter: the right part of the footer (`Version %s` by default).
    fn update_footer(&self, _ctx: &AdminContext<'_>) -> Option<String> {
        None
    }

    /// The help the plugin adds to the screen on `current_screen` (`WP_Screen::add_help_tab()`,
    /// `set_help_sidebar()`), before the screen's own.
    fn screen_help(&self, _ctx: &AdminContext<'_>) -> Option<AdminHelp> {
        None
    }

    /// The post statuses of `register_post_status()` as the lists of `edit.php` use them.
    fn list_statuses(&self, _site: &Site) -> Vec<ListStatus> {
        Vec::new()
    }

    /// `$wp_post_statuses[ $status ]->label_count` on the list of `ctx`, translated for `n` (with `%s` for the
    /// number): the plugin's own statuses, or a core status it relabels; `None` keeps the core label.
    fn status_label_count(&self, _ctx: &AdminContext<'_>, _status: &str, _n: u64) -> Option<String> {
        None
    }

    /// `manage_{$post_type}_posts_columns` of `edit.php`: the columns as `(key, title)`.
    fn posts_columns(&self, _ctx: &AdminContext<'_>, _post_type: &str, columns: Vec<(String, String)>) -> Vec<(String, String)> {
        columns
    }

    /// `manage_{$post_type}_posts_custom_column` (and `manage_pages_custom_column`/`manage_posts_custom_column`):
    /// what the plugin prints in a column of a row that is not one of WordPress.
    fn posts_custom_column<'a>(&'a self, _ctx: &'a AdminContext<'a>, _column: &'a str, _post: &'a Post) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `post_row_actions` (`page_row_actions` for hierarchical types): the actions of a row as `(class, link)`.
    fn post_row_actions<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post: &'a Post, _hierarchical: bool, actions: Vec<(String, String)>) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async move { Ok(actions) })
    }

    /// `bulk_actions-{$screen_id}`: `(value, label)`.
    fn bulk_actions(&self, _ctx: &AdminContext<'_>, _screen_id: &str, actions: Vec<(String, String)>) -> Vec<(String, String)> {
        actions
    }

    /// `views_{$screen_id}`: the links above a list as `(class, link)`.
    fn list_views(&self, _ctx: &AdminContext<'_>, _screen_id: &str, views: Vec<(String, String)>) -> Vec<(String, String)> {
        views
    }

    /// `display_post_states`: the states after a row's title as `(key, HTML)`.
    fn display_post_states<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post: &'a Post, states: Vec<(String, String)>) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async move { Ok(states) })
    }

    /// `$post_type_object->labels->not_found` as the plugin changes it for the list of `ctx` (HTML).
    fn list_not_found(&self, _ctx: &AdminContext<'_>, _post_type: &str) -> Option<String> {
        None
    }

    /// The extension of the first `wp_mime_type_icon()` the plugin calls on `admin_enqueue_scripts` of the screen: it fixes
    /// the icons of the request (`icon_files` in the object cache), `.png` where WordPress asks for `.svg`.
    fn admin_mime_icon_ext(&self, _ctx: &AdminContext<'_>) -> Option<&'static str> {
        None
    }

    /// `show_user_profile`/`edit_user_profile` of `profile.php`/`user-edit.php` (the user's ID) and `user_new_form` of
    /// `user-new.php` (`None`): what the plugin prints at the end of the form, and what it asks of the screen for it.
    fn user_form<'a>(&'a self, _ctx: &'a AdminContext<'a>, _user_id: Option<u64>) -> BoxFuture<'a, Result<MetaBoxes>> {
        Box::pin(async { Ok(MetaBoxes::default()) })
    }

    /// `profile_update` and `user_register` of a user the administration saved (`edit_user()`); a `wp_die()` of the
    /// plugin ends the request.
    fn saved_user<'a>(&'a self, _ctx: &'a UserSaved<'a>) -> BoxFuture<'a, Result<Option<Die>>> {
        Box::pin(async { Ok(None) })
    }

    /// `create_term` and `edit_term` of a term the administration saved (`wp_insert_term()`, `wp_update_term()`); a
    /// `wp_die()` of the plugin ends the request.
    fn saved_term<'a>(&'a self, _ctx: &'a TermSaved<'a>) -> BoxFuture<'a, Result<Option<Die>>> {
        Box::pin(async { Ok(None) })
    }

    /// `add_meta_boxes` of `post.php`/`post-new.php` ([`AdminContext::edited_post`] is the post): the plugin's boxes,
    /// the boxes of WordPress it removes, and what it prints on the actions of the form.
    fn post_meta_boxes<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post: &'a rustpress_db::Post) -> BoxFuture<'a, Result<PostMetaBoxes>> {
        Box::pin(async { Ok(PostMetaBoxes::default()) })
    }

    /// `save_post` of a post the administration saved (`edit_post()`), before its revision is made.
    fn save_post<'a>(&'a self, _ctx: &'a PostSaved<'a>) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// The `wp_save_post_revision_post_has_changed` filter while `edit_post()` saves: whether the post differs from
    /// its latest revision.
    fn revision_post_has_changed<'a>(&'a self, _ctx: &'a PostSaved<'a>, _latest_revision: u64, changed: bool) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move { Ok(changed) })
    }

    /// `_wp_put_post_revision` while `edit_post()` saves: a revision of the post was made.
    fn put_post_revision<'a>(&'a self, _ctx: &'a PostSaved<'a>, _revision_id: u64) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// The plugin registers scripts on `init`, so `wp_scripts()` is built before `wp-admin/includes/admin.php` adds
    /// its filters: `heartbeatSettings` misses what `wp_heartbeat_set_suspension()` adds.
    fn registers_scripts_on_init(&self) -> bool {
        false
    }

    /// `register_block_bindings_source()`: `(name, label, uses_context)` of the plugin's sources, as the editors list
    /// them before those of WordPress.
    fn block_bindings_sources(&self, _site: &Site) -> Vec<(String, String, Option<Vec<String>>)> {
        Vec::new()
    }

    /// `admin_footer`: printed after `#wpfooter`, before the media templates.
    fn admin_footer<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `admin_print_footer_scripts` after the scripts of the footer and the emoji script (priority 20).
    fn admin_print_footer_scripts<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The `admin_body_class` filter: classes (each with a leading space) the plugin adds to the `<body>` of every
    /// screen.
    fn admin_body_class(&self, _ctx: &AdminContext<'_>, classes: String) -> String {
        classes
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

    /// `admin_print_styles`: what the plugin prints before WordPress prints the styles of the screen (priority 20).
    fn admin_print_styles(&self, _ctx: &AdminContext<'_>) -> String {
        String::new()
    }

    /// `use_block_editor_for_post_type`: whether the block editor edits posts of `post_type` (`true` when WordPress
    /// would use it).
    fn use_block_editor_for_post_type(&self, _site: &Site, _post_type: &str, use_block_editor: bool) -> bool {
        use_block_editor
    }

    /// `use_block_editor_for_post`: whether the block editor edits `post`.
    fn use_block_editor_for_post<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post: &'a Post, use_block_editor: bool) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move { Ok(use_block_editor) })
    }

    /// `add_settings_field()` for the default section of a settings screen of WordPress (`writing`): the rows
    /// `do_settings_fields( $page, 'default' )` prints, as `(title, HTML of the field)`.
    fn settings_fields<'a>(&'a self, _ctx: &'a AdminContext<'a>, _page: &'a str) -> BoxFuture<'a, Result<Vec<(String, String)>>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    /// `get_edit_post_link`: the address of a post's edit screen as WordPress builds it (`&amp;` in the `display`
    /// context).
    fn get_edit_post_link<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post_id: u64, link: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(link) })
    }

    /// `redirect_post_location`: where `post.php` sends the browser after saving a post.
    fn redirect_post_location<'a>(&'a self, _ctx: &'a AdminContext<'a>, location: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(location) })
    }

    /// `edit_form_top`: printed at the top of the classic edit form, after its hidden fields.
    fn edit_form_top<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post: &'a Post) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `block_editor_settings_all` with the edited post as the context, for what a plugin does there besides
    /// changing the settings (which no module does yet).
    fn block_editor_settings<'a>(&'a self, _ctx: &'a AdminContext<'a>, _post: &'a Post) -> BoxFuture<'a, Result<()>> {
        Box::pin(async { Ok(()) })
    }

    /// `admin_head-{$hook_suffix}` and `admin_head` (before the colors of the scheme): printed in the head.
    fn admin_head<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `profile_personal_options`: printed after the personal options of the own profile.
    fn profile_personal_options<'a>(&'a self, _ctx: &'a AdminContext<'a>, _user_id: u64) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// `admin_notices`: printed above the content of every screen.
    fn admin_notices<'a>(&'a self, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The callback of one of the plugin's [`AdminPage`]s: what it prints (and does) inside the frame.
    fn admin_page<'a>(&'a self, _slug: &'a str, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async { Ok(String::new()) })
    }

    /// The callback of a page that prints meta boxes: its output in parts, [`FormPart::MetaBoxes`] where it calls
    /// `do_meta_boxes( $screen, $context, … )`, and the boxes it added (`add_meta_box()`); `None` for
    /// [`Self::admin_page`].
    fn admin_page_parts<'a>(&'a self, _slug: &'a str, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<(Vec<FormPart>, Vec<PluginMetaBox>)>>> {
        Box::pin(async { Ok(None) })
    }

    /// `plugin_action_links_{$plugin_file}` (or `plugin_action_links` for its own file): the links below the plugin's name
    /// in the list of plugins, starting with those of WordPress (Deactivate).
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

    /// `wp_ajax_nopriv_{$action}`: an Ajax request of a visitor who is not signed in (`user` is 0).
    fn admin_ajax_nopriv<'a>(&'a self, _action: &'a str, _ctx: &'a AdminContext<'a>) -> BoxFuture<'a, Result<Option<AjaxAnswer>>> {
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
    /// What the plugins keep for the rest of the request (their `static` variables).
    pub state: &'a std::sync::Mutex<PluginState>,
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
    Comment(&'a rustpress_db::Comment),
    /// A post type of `wp/v2/types`.
    PostType(&'a str),
    /// A block template or template part of `wp/v2/templates` (`WP_Block_Template`, no post ID), by post type.
    Template(&'a str),
}

/// A link of a REST response: `rel`, the attributes in their order, `href`.
#[derive(Debug, Clone)]
pub struct RestLink {
    pub rel: String,
    pub attributes: Vec<(String, serde_json::Value)>,
    pub href: String,
}

/// What a plugin sees while it adds fields to the REST API (no main query ran).
pub struct RestFieldContext<'a> {
    pub site: &'a Site,
    /// The empty query of a REST request.
    pub query: &'a crate::QueryResult,
    pub services: &'a dyn PageServices,
    /// The route of the request (`/wp/v2/posts/1`), its method (after `_method` and `X-HTTP-Method-Override`) and
    /// its query string.
    pub route: &'a str,
    pub method: &'a str,
    pub query_string: &'a str,
    /// The routes dispatched before in the same PHP process (the editors' preloads, embedded requests): what plugins
    /// registered on their `rest_pre_dispatch` stays registered.
    pub earlier_routes: &'a [String],
    /// `rest_is_field_included()` for the request's `_fields`.
    pub field_included: &'a (dyn Fn(&str) -> bool + Send + Sync),
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

/// The request as `$_SERVER` describes it, for the hooks that run besides the main query.
pub struct PluginRequest<'a> {
    pub method: &'a str,
    /// `$_SERVER['REQUEST_URI']` as the client sent it.
    pub uri: &'a str,
    /// The request headers with lower-case names.
    pub headers: &'a [(String, String)],
    /// `$_SERVER['REMOTE_ADDR']`.
    pub remote_addr: &'a str,
    /// `is_ssl()`.
    pub https: bool,
    /// The signed-in user.
    pub viewer: Option<&'a crate::Viewer>,
    pub writer: Option<&'a rustpress_db::Writer>,
    /// What the plugins remember between the hooks of the request.
    pub state: std::sync::Mutex<PluginState>,
}

impl PluginRequest<'_> {
    /// The first header of the name (lower case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }

    /// `$_SERVER['HTTP_HOST']`.
    pub fn host(&self) -> &str {
        self.header("host").unwrap_or_default()
    }
}

/// What a plugin does with a request on `init`.
#[derive(Debug)]
pub enum Init {
    /// `wp_redirect()` and `die()`: the answer as the plugin sends it.
    Response(PluginResponse),
    /// The request goes on with another `REQUEST_URI` (and its `$_GET`).
    Rewrite(String),
    /// The 404 template with this status (`status_header()`, `nocache_headers()` and `X-Redirect-By` on `wp`).
    Error { status: u16, by: &'static str },
    /// A rule took the request without changing it.
    Matched,
}

/// A redirect WordPress or a plugin sends with `wp_redirect()`.
#[derive(Debug)]
pub struct Redirected<'a> {
    pub location: &'a str,
    pub status: u16,
    /// The `$x_redirect_by` argument.
    pub by: &'a str,
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

    /// The `the_content` filters without `do_blocks()` as a plugin hooks them to its own filter (ACF's
    /// `acf_the_content`): `[embed]`, `wptexturize`, `wpautop`, `shortcode_unautop`, `wp_filter_content_tags` (with
    /// the filter's name as context), then `capital_P_dangit`, `do_shortcode` and `convert_smilies`. `post` is
    /// `global $post`.
    fn content_filters<'a>(&'a self, _post: Option<&'a Post>, text: &'a str, _context: &'a str) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(text.to_owned()) })
    }

    /// `get_avatar( $id_or_email, $size )`.
    fn avatar(&self, _email: &str, _size: u64) -> String {
        String::new()
    }
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
    /// Printed on `wp_head` at priority 0 by an action the plugin adds when it loads: before the theme's actions of
    /// priority 0 (in a block theme, like [Head::first], before `_block_template_viewport_meta_tag()`).
    pub early: String,
    /// Printed on `wp_head` at priority 0 by an action the plugin hooks on `init`: before the robots tag, after the
    /// theme's actions of priority 0.
    pub first: String,
    /// Printed on `wp_head` at priority 10, after the actions of core at that priority (`rel_canonical`,
    /// `wp_shortlink_wp_head`) and before the theme's.
    pub late: String,
    /// Printed on `wp_head` at priority 10 by actions the plugin hooks on `init`: after the import map and the
    /// preloads of the script modules.
    pub late_init: String,
    /// Core actions removed from `wp_head`: `rel_canonical`, `_wp_render_title_tag`.
    pub removed: Vec<&'static str>,
}

/// What the shortcode of a plugin sees of the page it is printed on.
pub struct ShortcodeContext<'a> {
    pub site: &'a Site,
    pub query: &'a crate::QueryResult,
    /// `get_the_ID()` while `in_the_loop()`.
    pub loop_post: Option<u64>,
    /// `global $post` (`get_the_ID()`), also outside the loop.
    pub global_post: Option<u64>,
    /// `doing_filter( 'the_content' )`.
    pub in_the_content: bool,
    /// `wp_is_block_theme()`.
    pub block_theme: bool,
    /// What the plugins keep for the rest of the request (their `static` variables).
    pub state: &'a std::sync::Mutex<PluginState>,
    pub services: &'a dyn PageServices,
}

/// Per-request state of the plugins: counters and lists by a name the plugin picks.
#[derive(Debug, Default)]
pub struct PluginState {
    pub counters: std::collections::HashMap<&'static str, u64>,
    pub lists: std::collections::HashMap<&'static str, Vec<String>>,
    pub flags: std::collections::HashSet<&'static str>,
    /// What the plugins enqueue while the page is rendered (on `wp_enqueue_scripts` at the asset's priority, which
    /// a block theme runs after rendering the template).
    pub assets: Assets,
    /// Printed on `wp_print_footer_scripts` after the scripts of the footer.
    pub footer_scripts: Vec<String>,
}

/// A comment that was just saved, with the form that sent it.
pub struct CommentSaved<'a> {
    pub site: &'a Site,
    pub writer: &'a rustpress_db::Writer,
    pub comment_id: u64,
    /// The post of the comment.
    pub post_id: u64,
    /// The fields of the form (`$_POST`), as sent.
    pub form: &'a [(String, String)],
    /// `current_user_can( $capability )` of who sent it.
    pub can: &'a (dyn Fn(&str) -> bool + Send + Sync),
    /// `wp_verify_nonce( $nonce, $action )` for who sent it.
    pub verify_nonce: &'a (dyn Fn(&str, &str) -> bool + Send + Sync),
}

/// What a plugin prints into an edit screen, and what it asks of the screen for it.
#[derive(Debug, Clone, Default)]
pub struct MetaBoxes {
    pub html: String,
    /// `wp_enqueue_media()` while the page is printed: the media library comes with the footer.
    pub media: bool,
    /// An editor with the default settings printed on `admin_footer` (priority 1) between the two parts (ACF's
    /// hidden `wp_editor( '', 'acf_content' )`).
    pub hidden_editor: Option<HiddenEditor>,
}

/// A box of a plugin: `add_meta_box( $id, $title, $callback, $screen, $context, $priority )` with what the callback
/// prints.
#[derive(Debug, Clone)]
pub struct PluginMetaBox {
    pub id: String,
    /// The title, HTML.
    pub title: String,
    /// `side`, `normal`, `advanced` or a context of the plugin (ACF's `acf_after_title`).
    pub context: String,
    /// `high`, `core`, `default` or `low`.
    pub priority: String,
    pub html: String,
}

/// A piece of what a plugin prints on an action of the edit form.
#[derive(Debug, Clone)]
pub enum FormPart {
    Html(String),
    /// `do_meta_boxes( $screen, $context, $post )`.
    MetaBoxes(String),
}

/// What a plugin adds to the edit screen of a post.
#[derive(Debug, Clone, Default)]
pub struct PostMetaBoxes {
    pub boxes: Vec<PluginMetaBox>,
    /// `remove_meta_box()`: the boxes of WordPress the plugin takes away.
    pub removed: Vec<String>,
    /// `edit_form_after_title` (the classic editor).
    pub after_title: Vec<FormPart>,
    /// `block_editor_meta_box_hidden_fields` (the block editor).
    pub hidden_fields: Vec<FormPart>,
    /// An editor printed on `admin_footer` (priority 1), as for [`MetaBoxes::hidden_editor`].
    pub hidden_editor: Option<HiddenEditor>,
    /// `wp_enqueue_media()` while the boxes are printed, as for [`MetaBoxes::media`].
    pub media: bool,
    /// `WP_Privacy_Policy_Content::notice()` moves from `admin_notices` to `edit_form_after_title` (Classic Editor).
    pub privacy_notice_after_title: bool,
}

/// What happened to a post in [`ActivePlugin::post_trash_change`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostChangeKind {
    Trashed,
    Untrashed,
    Deleted,
}

/// A post that went to the trash, came back or was deleted.
pub struct PostChange<'a> {
    pub site: &'a Site,
    pub writer: &'a rustpress_db::Writer,
    /// `time()`.
    pub now: i64,
    /// The signed-in user.
    pub user: u64,
    /// The post before the change.
    pub post: &'a Post,
    pub kind: PostChangeKind,
}

/// A post `edit_post()` saved, with the request that sent it.
pub struct PostSaved<'a> {
    pub ctx: &'a AdminContext<'a>,
    pub writer: &'a rustpress_db::Writer,
    /// The post as it was stored.
    pub post: &'a rustpress_db::Post,
}

/// A field of `attachment_fields_to_edit` with `'input' => 'html'`.
#[derive(Debug, Clone, Default)]
pub struct AttachmentField {
    pub id: String,
    pub label: String,
    pub html: String,
}

/// An attachment the administration saves, as `attachment_fields_to_save` sees it (before the post is updated).
pub struct AttachmentSaved<'a> {
    pub ctx: &'a AdminContext<'a>,
    pub writer: &'a rustpress_db::Writer,
    pub attachment: &'a Post,
    /// `wp_ajax_save_attachment_compat()` rather than `edit_post()`.
    pub ajax: bool,
}

/// A user the administration saved, as `profile_update`/`user_register` see it.
pub struct UserSaved<'a> {
    pub ctx: &'a AdminContext<'a>,
    pub writer: &'a rustpress_db::Writer,
    pub user_id: u64,
    /// `user_register` rather than `profile_update`.
    pub created: bool,
}

/// A term the administration saved, as `create_term`/`edit_term` see it.
pub struct TermSaved<'a> {
    pub ctx: &'a AdminContext<'a>,
    pub writer: &'a rustpress_db::Writer,
    pub term_id: u64,
    pub taxonomy: &'a str,
    /// `create_term` rather than `edit_term`.
    pub created: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HiddenEditor {
    pub id: String,
    pub before: String,
    pub after: String,
}

/// A `wp_die( $message, $title )` page of a plugin (status 500 by default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Die {
    pub title: String,
    /// The message, HTML.
    pub message: String,
    pub status: u16,
}

/// What a plugin sees of the comment form it filters.
pub struct CommentFormContext<'a> {
    pub site: &'a Site,
    pub query: &'a crate::QueryResult,
    /// `global $post`: the post the form is for.
    pub post: &'a Post,
    pub state: &'a std::sync::Mutex<PluginState>,
    pub services: &'a dyn PageServices,
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
    /// The plugin sends its own answer and exits (a download): the headers it sets and what it prints.
    Output { headers: Vec<(String, String)>, body: String },
    /// The page goes on; the plugin prints these notices on `admin_notices` (what it did while loading).
    Notices(String),
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
    /// `$_SERVER['HTTP_USER_AGENT']`.
    pub user_agent: Option<&'a str>,
    /// `$wp_version` of the WordPress files.
    pub wp_version: &'a str,
    /// `ABSPATH`: where the WordPress files are.
    pub wp_root: &'a std::path::Path,
    /// The global `$post` of `post.php`/`post-new.php`.
    pub edited_post: Option<&'a rustpress_db::Post>,
    /// `WP_Screen::is_block_editor()`.
    pub block_editor: bool,
    /// `get_current_screen()->taxonomy` of `edit-tags.php`/`term.php`.
    pub taxonomy: Option<&'a str>,
    /// The term of `term.php`.
    pub edited_term: Option<&'a rustpress_db::Term>,
    /// `get_current_screen()->id` (`edit-acf-field-group`); empty outside a screen.
    pub screen_id: &'a str,
    /// `$typenow`: the post type of the screen.
    pub typenow: &'a str,
    /// `$_FILES`: the uploaded files of a multipart form.
    pub files: &'a [RestFile],
}

/// A post status a plugin registers, as the lists of `edit.php` use it.
#[derive(Debug, Clone)]
pub struct ListStatus {
    pub name: &'static str,
    /// `show_in_admin_all_list`: counted and listed under "All".
    pub show_in_admin_all_list: bool,
    /// `show_in_admin_status_list`: a link of its own above the list.
    pub show_in_admin_status_list: bool,
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

/// A stylesheet or script a plugin enqueues (`wp_enqueue_style()`, `wp_enqueue_script()`). Without a `src` it is
/// one WordPress registers itself.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Asset {
    pub handle: String,
    pub src: String,
    pub deps: Vec<String>,
    /// `ver`; `None` prints none.
    pub ver: Option<String>,
    /// Styles: `media`.
    pub media: String,
    /// Scripts: printed in the footer. Styles: enqueued after the head, printed by `print_late_styles()`.
    pub in_footer: bool,
    /// The priority of the `wp_enqueue_scripts` callback that enqueues it (10 by default).
    pub priority: i32,
    /// Scripts: the data `wp_localize_script()` adds (`var name = {...};`).
    pub extra: Option<String>,
    /// Scripts: `wp_add_inline_script( $handle, $data, 'before' )`.
    pub before: Vec<String>,
    /// Scripts: `wp_add_inline_script( $handle, $data )` after the script. Styles: `wp_add_inline_style()`.
    pub after: Vec<String>,
    /// Registered but not enqueued (`wp_register_script()`): printed only as a dependency of another script.
    pub registered_only: bool,
    /// Scripts: attributes a `script_loader_tag` filter adds to the script element (` async`), before `></script>`.
    pub tag_attributes: String,
    /// Scripts: what `script_loader_tag` filters put in front of the tags of the script (an HTML comment).
    pub tag_prefix: String,
    /// Scripts: the text domain of `wp_set_script_translations()`.
    pub translations: Option<String>,
    /// Scripts: a `script_loader_tag` filter that puts `defer` in front of the tags of the handle
    /// (`preg_replace( '/^<script /', '<script defer ', $tag )`).
    pub defer_attr: bool,
}

/// What the plugins enqueue on a page, in the order they enqueue it.
#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub styles: Vec<Asset>,
    pub scripts: Vec<Asset>,
    /// `wp_dequeue_script( $handle )` of scripts the screen enqueues.
    pub dequeue: Vec<String>,
}

impl Assets {
    /// `wp_enqueue_style( $handle, $src, $deps, $ver, $media )`.
    pub fn style(&mut self, handle: &str, src: String, deps: &[&str], ver: &str, media: &str) -> &mut Asset {
        self.styles.push(Asset { handle: handle.to_owned(), src, deps: deps.iter().map(|d| (*d).to_owned()).collect(), ver: Some(ver.to_owned()), media: media.to_owned(), priority: 10, ..Asset::default() });
        self.styles.last_mut().expect("just pushed")
    }

    /// `wp_enqueue_style( $handle )` of a stylesheet WordPress registers (`wp-color-picker`).
    pub fn core_style(&mut self, handle: &str) {
        self.styles.push(Asset { handle: handle.to_owned(), priority: 10, ..Asset::default() });
    }

    /// `wp_enqueue_script( $handle )` of a script WordPress registers. `in_footer` is where it ends up: its own
    /// group, or the head when a script of the head depends on it.
    pub fn core_script(&mut self, handle: &str, in_footer: bool) {
        self.scripts.push(Asset { handle: handle.to_owned(), in_footer, priority: 10, ..Asset::default() });
    }

    /// `wp_enqueue_script( $handle, $src, $deps, $ver, $in_footer )`.
    pub fn script(&mut self, handle: &str, src: String, deps: &[&str], ver: &str, in_footer: bool) -> &mut Asset {
        self.scripts.push(Asset { handle: handle.to_owned(), src, deps: deps.iter().map(|d| (*d).to_owned()).collect(), ver: Some(ver.to_owned()), in_footer, priority: 10, ..Asset::default() });
        self.scripts.last_mut().expect("just pushed")
    }
}

/// `PHP_URL_HOST` of an address (`wp_parse_url( $url, PHP_URL_HOST )`), if it has one.
fn url_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("//").or_else(|| url.split_once("://").map(|(_, r)| r))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host.split(':').next().unwrap_or_default();
    (!host.is_empty()).then_some(host)
}

/// `wp_resource_hints()` on `wp_head` at priority 2 (`login_head` at 8): `wp_dependencies_unique_hosts()` of the enqueued plugin scripts
/// and stylesheets (core's are on the site's own host) as `dns-prefetch`, with what the plugins add.
pub fn resource_hints(site: &Site, assets: &Assets) -> String {
    let server = url_host(&site.settings.home).unwrap_or_default().to_owned();
    let mut hosts: Vec<String> = Vec::new();
    for asset in assets.scripts.iter().chain(&assets.styles).filter(|a| !a.registered_only) {
        if let Some(host) = url_host(&asset.src)
            && !hosts.iter().any(|h| h == host)
            && host != server
        {
            hosts.push(host.to_owned());
        }
    }
    let mut out = String::new();
    for relation in ["dns-prefetch", "preconnect", "prefetch", "prerender"] {
        let urls = site.plugins.resource_hints(relation, if relation == "dns-prefetch" { hosts.clone() } else { Vec::new() });
        let mut seen: Vec<String> = Vec::new();
        let mut unique: Vec<String> = Vec::new();
        for url in urls {
            // `esc_url( $url, array( 'http', 'https' ) )`.
            let url = crate::formatting::esc_url(&url);
            let scheme_ok = url.split_once(':').is_none_or(|(scheme, _)| scheme.contains(['/', '?', '#']) || matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https"));
            if url.is_empty() || !scheme_ok || seen.contains(&url) {
                continue;
            }
            let href = if matches!(relation, "preconnect" | "dns-prefetch") {
                let Some(host) = url_host(&url) else { continue };
                match url.split_once("://") {
                    Some((scheme, _)) if relation == "preconnect" => format!("{scheme}://{host}"),
                    _ => format!("//{host}"),
                }
            } else {
                url
            };
            if !seen.contains(&href) {
                seen.push(href.clone());
                unique.push(href);
            }
        }
        for href in unique {
            out.push_str(&format!("<link rel='{}' href='{}' />\n", crate::formatting::esc_attr(relation), crate::formatting::esc_url(&href)));
        }
    }
    out
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
#[derive(Default, Clone)]
pub struct Plugins {
    active: std::sync::Arc<Vec<(&'static dyn NativePlugin, Box<dyn ActivePlugin>)>>,
}

impl Plugins {
    /// Loads the modules of `active` (plugin files). A module that fails to load is left out.
    pub async fn load(site: &Site, active: &[String], wp_root: &std::path::Path) -> Self {
        let mut loaded = Vec::new();
        for file in active {
            let Some(plugin) = registered().iter().find(|p| p.file() == file) else { continue };
            match plugin.load_at(site, wp_root).await {
                Ok(instance) => loaded.push((*plugin, instance)),
                Err(e) => tracing::error!(plugin = file.as_str(), error = %e, "native plugin failed to load"),
            }
        }
        Self { active: std::sync::Arc::new(loaded) }
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
        self.assets_with(site, None)
    }

    /// The assets of `wp_enqueue_scripts` with what the plugins enqueued while the page was rendered.
    pub fn assets_with(&self, site: &Site, state: Option<&PluginState>) -> Assets {
        let mut assets = Assets::default();
        for plugin in self.hooks() {
            plugin.enqueue(site, &mut assets);
        }
        Self::finish_assets(assets, state)
    }

    /// What the plugins enqueue on a page of the website with its main query.
    pub async fn assets_for(&self, site: &Site, query: Option<&crate::QueryResult>, state: Option<&PluginState>) -> Assets {
        let mut assets = Assets::default();
        for plugin in self.hooks() {
            plugin.enqueue(site, &mut assets);
            if let Some(query) = query
                && let Err(e) = plugin.enqueue_query(site, query, &mut assets).await {
                    tracing::error!(error = %e, "native plugin enqueue failed");
                }
        }
        Self::finish_assets(assets, state)
    }

    /// Adds what the plugins enqueued while the page was rendered and sorts by the priority of the callbacks.
    fn finish_assets(mut assets: Assets, state: Option<&PluginState>) -> Assets {
        if let Some(state) = state {
            assets.styles.extend(state.assets.styles.iter().cloned());
            assets.scripts.extend(state.assets.scripts.iter().cloned());
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

    /// What the plugins print on `admin_print_styles`.
    pub fn admin_print_styles(&self, ctx: &AdminContext<'_>) -> String {
        self.hooks().map(|plugin| plugin.admin_print_styles(ctx)).collect()
    }

    /// `use_block_editor_for_post_type` of the plugins.
    pub fn use_block_editor_for_post_type(&self, site: &Site, post_type: &str, use_block_editor: bool) -> bool {
        self.hooks().fold(use_block_editor, |value, plugin| plugin.use_block_editor_for_post_type(site, post_type, value))
    }

    /// `use_block_editor_for_post` of the plugins.
    pub async fn use_block_editor_for_post(&self, ctx: &AdminContext<'_>, post: &Post, mut use_block_editor: bool) -> Result<bool> {
        for plugin in self.hooks() {
            use_block_editor = plugin.use_block_editor_for_post(ctx, post, use_block_editor).await?;
        }
        Ok(use_block_editor)
    }

    /// The fields the plugins add to the default section of the settings screen `page`.
    pub async fn settings_fields(&self, ctx: &AdminContext<'_>, page: &str) -> Result<Vec<(String, String)>> {
        let mut fields = Vec::new();
        for plugin in self.hooks() {
            fields.extend(plugin.settings_fields(ctx, page).await?);
        }
        Ok(fields)
    }

    /// `get_edit_post_link` of the plugins.
    pub async fn get_edit_post_link(&self, ctx: &AdminContext<'_>, post_id: u64, mut link: String) -> Result<String> {
        for plugin in self.hooks() {
            link = plugin.get_edit_post_link(ctx, post_id, link).await?;
        }
        Ok(link)
    }

    /// `redirect_post_location` of the plugins.
    pub async fn redirect_post_location(&self, ctx: &AdminContext<'_>, mut location: String) -> Result<String> {
        for plugin in self.hooks() {
            location = plugin.redirect_post_location(ctx, location).await?;
        }
        Ok(location)
    }

    /// What the plugins print on `edit_form_top`.
    pub async fn edit_form_top(&self, ctx: &AdminContext<'_>, post: &Post) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.edit_form_top(ctx, post).await?);
        }
        Ok(out)
    }

    /// `block_editor_settings_all` of the plugins.
    pub async fn block_editor_settings(&self, ctx: &AdminContext<'_>, post: &Post) -> Result<()> {
        for plugin in self.hooks() {
            plugin.block_editor_settings(ctx, post).await?;
        }
        Ok(())
    }

    /// What the plugins print on `admin_head`.
    pub async fn admin_head(&self, ctx: &AdminContext<'_>) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.admin_head(ctx).await?);
        }
        Ok(out)
    }

    /// What the plugins print on `profile_personal_options`.
    pub async fn profile_personal_options(&self, ctx: &AdminContext<'_>, user_id: u64) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.profile_personal_options(ctx, user_id).await?);
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

    /// The first plugin that answers `wp_ajax_nopriv_{$action}`.
    pub async fn admin_ajax_nopriv(&self, action: &str, ctx: &AdminContext<'_>) -> Result<Option<AjaxAnswer>> {
        for plugin in self.hooks() {
            if let Some(answer) = plugin.admin_ajax_nopriv(action, ctx).await? {
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

    /// What the plugins do on `init` (the first one that acts wins).
    pub async fn init(&self, site: &Site, request: &PluginRequest<'_>) -> Result<Option<Init>> {
        for plugin in self.hooks() {
            if let Some(init) = plugin.init(site, request).await? {
                return Ok(Some(init));
            }
        }
        Ok(None)
    }

    /// The headers the plugins send on `send_headers`.
    pub fn send_headers(&self, site: &Site, request: &PluginRequest<'_>) -> Vec<(String, String)> {
        self.hooks().flat_map(|plugin| plugin.send_headers(site, request)).collect()
    }

    /// The headers the plugins add to a redirect.
    pub async fn wp_redirect(&self, site: &Site, request: &PluginRequest<'_>, redirect: &Redirected<'_>) -> Result<Vec<(String, String)>> {
        let mut headers = Vec::new();
        for plugin in self.hooks() {
            headers.extend(plugin.wp_redirect(site, request, redirect).await?);
        }
        Ok(headers)
    }

    /// The first redirect a plugin asks for on a 404 page after `redirect_canonical()`.
    pub async fn not_found(&self, site: &Site, query: &crate::QueryResult, request: &PluginRequest<'_>) -> Result<Option<Redirect>> {
        for plugin in self.hooks() {
            if let Some(redirect) = plugin.not_found(site, query, request).await? {
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

    /// The public post statuses of the plugins, in the order they register them.
    pub fn internal_post_statuses(&self) -> Vec<&'static str> {
        self.hooks().flat_map(|plugin| plugin.internal_post_statuses().iter().copied()).collect()
    }

    pub fn post_statuses(&self, site: &Site) -> Vec<(&'static str, String)> {
        self.hooks().flat_map(|plugin| plugin.post_statuses(site)).collect()
    }

    pub async fn rest_links(&self, object: RestObject<'_>, ctx: &RestFieldContext<'_>) -> Result<Vec<RestLink>> {
        let mut links = Vec::new();
        for plugin in self.hooks() {
            links.extend(plugin.rest_links(object, ctx).await?);
        }
        Ok(links)
    }

    /// Whether a plugin renders the block.
    /// The names of the blocks the active plugins register.
    pub fn block_names(&self) -> Vec<String> {
        self.hooks().flat_map(|p| p.block_names().iter().map(|n| (*n).to_owned())).collect()
    }

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

    /// `comment_post` / `edit_comment` of the plugins; the first `wp_die()` ends it.
    pub async fn comment_post(&self, ctx: &CommentSaved<'_>) -> Result<Option<Die>> {
        for plugin in self.hooks() {
            if let Some(die) = plugin.comment_post(ctx).await? {
                return Ok(Some(die));
            }
        }
        Ok(None)
    }

    /// `add_meta_boxes_comment` of the plugins.
    pub async fn comment_meta_boxes(&self, ctx: &AdminContext<'_>, comment: &rustpress_db::Comment) -> Result<MetaBoxes> {
        let mut out = MetaBoxes::default();
        for plugin in self.hooks() {
            let boxes = plugin.comment_meta_boxes(ctx, comment).await?;
            out.html.push_str(&boxes.html);
            out.media |= boxes.media;
            out.hidden_editor = out.hidden_editor.or(boxes.hidden_editor);
        }
        Ok(out)
    }

    /// `{$taxonomy}_add_form_fields`/`{$taxonomy}_edit_form` of the plugins.
    pub async fn term_form(&self, ctx: &AdminContext<'_>, taxonomy: &str, term: Option<&rustpress_db::Term>) -> Result<MetaBoxes> {
        let mut out = MetaBoxes::default();
        for plugin in self.hooks() {
            let boxes = plugin.term_form(ctx, taxonomy, term).await?;
            out.html.push_str(&boxes.html);
            out.media |= boxes.media;
            out.hidden_editor = out.hidden_editor.or(boxes.hidden_editor);
        }
        Ok(out)
    }

    /// The schemas of the plugins' REST fields for a route.
    pub async fn rest_schema_fields(&self, ctx: &RestFieldContext<'_>) -> Result<Vec<(String, serde_json::Value)>> {
        let mut out = Vec::new();
        for plugin in self.hooks() {
            out.extend(plugin.rest_schema_fields(ctx).await?);
        }
        Ok(out)
    }

    /// The sizes of `image_size_names_choose` the plugins add to `query-attachments`.
    pub fn query_attachments_sizes(&self, ctx: &AdminContext<'_>) -> Vec<String> {
        self.hooks().flat_map(|plugin| plugin.query_attachments_sizes(ctx)).collect()
    }

    /// The filters the plugins add to the attachments of `query-attachments`.
    pub fn query_attachment_js(&self, ctx: &AdminContext<'_>, response: &mut serde_json::Map<String, serde_json::Value>) {
        for plugin in self.hooks() {
            plugin.query_attachment_js(ctx, response);
        }
    }

    /// `parent_file` after the plugins.
    pub fn parent_file(&self, ctx: &AdminContext<'_>, parent_file: String) -> String {
        self.hooks().fold(parent_file, |file, plugin| plugin.parent_file(ctx, file))
    }

    /// `submenu_file` after the plugins.
    pub fn submenu_file(&self, ctx: &AdminContext<'_>, submenu_file: String) -> String {
        self.hooks().fold(submenu_file, |file, plugin| plugin.submenu_file(ctx, file))
    }

    /// `trashed_post`, `untrashed_post` or `deleted_post` of the plugins.
    pub async fn post_trash_change(&self, change: &PostChange<'_>) -> Result<()> {
        for plugin in self.hooks() {
            plugin.post_trash_change(change).await?;
        }
        Ok(())
    }

    /// The status of a post restored from the trash after the plugins.
    pub fn untrash_post_status(&self, post: &Post, new_status: &str, previous: &str) -> String {
        self.hooks().fold(new_status.to_owned(), |status, plugin| plugin.untrash_post_status(post, &status, previous).unwrap_or(status))
    }

    /// The first action of a plugin on a posts list.
    pub async fn list_load(&self, ctx: &AdminContext<'_>) -> Result<Option<PageLoad>> {
        for plugin in self.hooks() {
            if let Some(load) = plugin.list_load(ctx).await? {
                return Ok(Some(load));
            }
        }
        Ok(None)
    }

    /// The message after a save of the first plugin that gives one.
    pub fn post_updated_message(&self, site: &Site, post: &Post, number: u32, nonce: &dyn Fn(&str) -> String) -> Option<Option<String>> {
        self.hooks().find_map(|plugin| plugin.post_updated_message(site, post, number, nonce))
    }

    /// The layout of the first plugin that sets one.
    pub fn screen_layout(&self, ctx: &AdminContext<'_>, screen_id: &str) -> Option<u32> {
        self.hooks().find_map(|plugin| plugin.screen_layout(ctx, screen_id))
    }

    /// `screen_settings` after the plugins.
    pub async fn screen_settings(&self, ctx: &AdminContext<'_>, html: String) -> Result<String> {
        let mut html = html;
        for plugin in self.hooks() {
            html = plugin.screen_settings(ctx, html).await?;
        }
        Ok(html)
    }

    /// `in_admin_header` of the plugins.
    pub async fn in_admin_header(&self, ctx: &AdminContext<'_>) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.in_admin_header(ctx).await?);
        }
        Ok(out)
    }

    /// `update_footer` of the first plugin that changes it.
    pub fn update_footer(&self, ctx: &AdminContext<'_>) -> Option<String> {
        self.hooks().find_map(|plugin| plugin.update_footer(ctx))
    }

    /// The help the plugins add to a screen, in their order.
    pub fn screen_help(&self, ctx: &AdminContext<'_>) -> Option<AdminHelp> {
        let mut out: Option<AdminHelp> = None;
        for help in self.hooks().filter_map(|plugin| plugin.screen_help(ctx)) {
            let all = out.get_or_insert_with(AdminHelp::default);
            all.tabs.extend(help.tabs);
            if !help.sidebar.is_empty() {
                all.sidebar = help.sidebar;
            }
        }
        out
    }

    /// The post statuses the plugins register, in their order.
    pub fn list_statuses(&self, site: &Site) -> Vec<ListStatus> {
        self.hooks().flat_map(|plugin| plugin.list_statuses(site)).collect()
    }

    /// The label of a status with its count on a list, from the first plugin that gives one.
    pub fn status_label_count(&self, ctx: &AdminContext<'_>, status: &str, n: u64) -> Option<String> {
        self.hooks().find_map(|plugin| plugin.status_label_count(ctx, status, n))
    }

    /// The columns of a posts list after the plugins.
    pub fn posts_columns(&self, ctx: &AdminContext<'_>, post_type: &str, columns: Vec<(String, String)>) -> Vec<(String, String)> {
        self.hooks().fold(columns, |columns, plugin| plugin.posts_columns(ctx, post_type, columns))
    }

    /// What the plugins print in a column of theirs.
    pub async fn posts_custom_column(&self, ctx: &AdminContext<'_>, column: &str, post: &Post) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.posts_custom_column(ctx, column, post).await?);
        }
        Ok(out)
    }

    /// The row actions after the plugins.
    pub async fn post_row_actions(&self, ctx: &AdminContext<'_>, post: &Post, hierarchical: bool, mut actions: Vec<(String, String)>) -> Result<Vec<(String, String)>> {
        for plugin in self.hooks() {
            actions = plugin.post_row_actions(ctx, post, hierarchical, actions).await?;
        }
        Ok(actions)
    }

    /// The bulk actions after the plugins.
    pub fn bulk_actions(&self, ctx: &AdminContext<'_>, screen_id: &str, actions: Vec<(String, String)>) -> Vec<(String, String)> {
        self.hooks().fold(actions, |actions, plugin| plugin.bulk_actions(ctx, screen_id, actions))
    }

    /// The views of a list after the plugins.
    pub fn list_views(&self, ctx: &AdminContext<'_>, screen_id: &str, views: Vec<(String, String)>) -> Vec<(String, String)> {
        self.hooks().fold(views, |views, plugin| plugin.list_views(ctx, screen_id, views))
    }

    /// The post states after the plugins.
    pub async fn display_post_states(&self, ctx: &AdminContext<'_>, post: &Post, mut states: Vec<(String, String)>) -> Result<Vec<(String, String)>> {
        for plugin in self.hooks() {
            states = plugin.display_post_states(ctx, post, states).await?;
        }
        Ok(states)
    }

    /// The `not_found` label of the first plugin that changes it.
    pub fn list_not_found(&self, ctx: &AdminContext<'_>, post_type: &str) -> Option<String> {
        self.hooks().find_map(|plugin| plugin.list_not_found(ctx, post_type))
    }

    /// The extension of the icons of the request: that of the first plugin that asks for icons early, else `svg`.
    pub fn admin_mime_icon_ext(&self, ctx: &AdminContext<'_>) -> &'static str {
        self.hooks().find_map(|plugin| plugin.admin_mime_icon_ext(ctx)).unwrap_or("svg")
    }

    /// `attachment_fields_to_edit` of the plugins, in their order.
    pub async fn attachment_fields_to_edit(&self, ctx: &AdminContext<'_>, attachment: &Post, in_modal: bool) -> Result<Vec<AttachmentField>> {
        let mut out = Vec::new();
        for plugin in self.hooks() {
            out.extend(plugin.attachment_fields_to_edit(ctx, attachment, in_modal).await?);
        }
        Ok(out)
    }

    /// `attachment_fields_to_save` of the plugins; the first `wp_die()` ends the request.
    pub async fn attachment_fields_to_save(&self, ctx: &AttachmentSaved<'_>) -> Result<Option<Die>> {
        for plugin in self.hooks() {
            if let Some(die) = plugin.attachment_fields_to_save(ctx).await? {
                return Ok(Some(die));
            }
        }
        Ok(None)
    }

    /// The uploader the plugins ask for on a screen without boxes.
    pub async fn screen_uploader(&self, ctx: &AdminContext<'_>) -> Result<MetaBoxes> {
        let mut out = MetaBoxes::default();
        for plugin in self.hooks() {
            let boxes = plugin.screen_uploader(ctx).await?;
            out.html.push_str(&boxes.html);
            out.media |= boxes.media;
            out.hidden_editor = out.hidden_editor.or(boxes.hidden_editor);
        }
        Ok(out)
    }

    /// `show_user_profile`/`edit_user_profile`/`user_new_form` of the plugins.
    pub async fn user_form(&self, ctx: &AdminContext<'_>, user_id: Option<u64>) -> Result<MetaBoxes> {
        let mut out = MetaBoxes::default();
        for plugin in self.hooks() {
            let boxes = plugin.user_form(ctx, user_id).await?;
            out.html.push_str(&boxes.html);
            out.media |= boxes.media;
            out.hidden_editor = out.hidden_editor.or(boxes.hidden_editor);
        }
        Ok(out)
    }

    /// `profile_update`/`user_register` of the plugins; the first `wp_die()` ends the request.
    pub async fn saved_user(&self, ctx: &UserSaved<'_>) -> Result<Option<Die>> {
        for plugin in self.hooks() {
            if let Some(die) = plugin.saved_user(ctx).await? {
                return Ok(Some(die));
            }
        }
        Ok(None)
    }

    /// `create_term`/`edit_term` of the plugins; the first `wp_die()` ends the request.
    pub async fn saved_term(&self, ctx: &TermSaved<'_>) -> Result<Option<Die>> {
        for plugin in self.hooks() {
            if let Some(die) = plugin.saved_term(ctx).await? {
                return Ok(Some(die));
            }
        }
        Ok(None)
    }

    /// `add_meta_boxes` of the plugins on the edit screen of `post`.
    pub async fn post_meta_boxes(&self, ctx: &AdminContext<'_>, post: &rustpress_db::Post) -> Result<PostMetaBoxes> {
        let mut out = PostMetaBoxes::default();
        for plugin in self.hooks() {
            let boxes = plugin.post_meta_boxes(ctx, post).await?;
            out.boxes.extend(boxes.boxes);
            out.removed.extend(boxes.removed);
            out.after_title.extend(boxes.after_title);
            out.hidden_fields.extend(boxes.hidden_fields);
            out.hidden_editor = out.hidden_editor.or(boxes.hidden_editor);
            out.media |= boxes.media;
            out.privacy_notice_after_title |= boxes.privacy_notice_after_title;
        }
        Ok(out)
    }

    /// `save_post` of the plugins.
    pub async fn save_post(&self, ctx: &PostSaved<'_>) -> Result<()> {
        for plugin in self.hooks() {
            plugin.save_post(ctx).await?;
        }
        Ok(())
    }

    /// `wp_save_post_revision_post_has_changed` of the plugins.
    pub async fn revision_post_has_changed(&self, ctx: &PostSaved<'_>, latest_revision: u64, mut changed: bool) -> Result<bool> {
        for plugin in self.hooks() {
            changed = plugin.revision_post_has_changed(ctx, latest_revision, changed).await?;
        }
        Ok(changed)
    }

    /// `_wp_put_post_revision` of the plugins.
    pub async fn put_post_revision(&self, ctx: &PostSaved<'_>, revision_id: u64) -> Result<()> {
        for plugin in self.hooks() {
            plugin.put_post_revision(ctx, revision_id).await?;
        }
        Ok(())
    }

    /// Whether a plugin registers scripts on `init`.
    pub fn registers_scripts_on_init(&self) -> bool {
        self.hooks().any(|p| p.registers_scripts_on_init())
    }

    /// The block bindings sources of the plugins.
    pub fn block_bindings_sources(&self, site: &Site) -> Vec<(String, String, Option<Vec<String>>)> {
        self.hooks().flat_map(|p| p.block_bindings_sources(site)).collect()
    }

    /// `admin_footer` of the plugins.
    pub async fn admin_footer(&self, ctx: &AdminContext<'_>) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.admin_footer(ctx).await?);
        }
        Ok(out)
    }

    /// `admin_print_footer_scripts` of the plugins.
    pub async fn admin_print_footer_scripts(&self, ctx: &AdminContext<'_>) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.admin_print_footer_scripts(ctx).await?);
        }
        Ok(out)
    }

    /// `apply_filters( 'admin_body_class', '' )` of the plugins.
    pub fn admin_body_class(&self, ctx: &AdminContext<'_>) -> String {
        self.hooks().fold(String::new(), |classes, plugin| plugin.admin_body_class(ctx, classes))
    }

    /// The comment field of `comment_form()` after the plugins' `comment_form_field_comment` filters.
    pub async fn comment_form_field_comment(&self, ctx: &CommentFormContext<'_>, mut html: String) -> Result<String> {
        for plugin in self.hooks() {
            html = plugin.comment_form_field_comment(ctx, html).await?;
        }
        Ok(html)
    }

    /// What the plugins print in `wp_footer`.
    pub async fn wp_footer(&self, site: &Site, query: &crate::QueryResult, looped: &[u64]) -> Result<String> {
        let mut out = String::new();
        for plugin in self.hooks() {
            out.push_str(&plugin.wp_footer(site, looped).await?);
            out.push_str(&plugin.wp_footer_query(site, query).await?);
        }
        Ok(out)
    }

    /// What the plugins print on `wp_print_footer_scripts` after the scripts of the footer.
    pub fn print_footer_scripts(&self, site: &Site, query: &crate::QueryResult) -> String {
        self.hooks().map(|plugin| plugin.print_footer_scripts(site, query)).collect()
    }

    /// The classes the plugins add on `body_class`.
    pub async fn body_class(&self, site: &Site, query: &crate::QueryResult) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for plugin in self.hooks() {
            out.extend(plugin.body_class(site, query).await?);
        }
        Ok(out)
    }

    /// The page after the plugins' output buffers of the website.
    pub fn template_output(&self, site: &Site, query: &crate::QueryResult, mut html: String) -> String {
        for plugin in self.hooks() {
            html = plugin.template_output(site, query, html);
        }
        html
    }

    /// What the plugins print on `login_head` at the default priority.
    pub fn login_head(&self, site: &Site) -> String {
        self.hooks().map(|plugin| plugin.login_head(site)).collect()
    }

    /// The `wp_resource_hints` filter of the plugins for `relation`, after the URLs of core.
    pub fn resource_hints(&self, relation: &str, urls: Vec<String>) -> Vec<String> {
        let mut urls = urls;
        for plugin in self.hooks() {
            urls.extend(plugin.resource_hints(relation));
        }
        urls
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
            head.first.push_str(&changed.first);
            head.late.push_str(&changed.late);
            head.early.push_str(&changed.early);
            head.late_init.push_str(&changed.late_init);
            head.removed.extend(changed.removed);
        }
        Ok(head)
    }
}
