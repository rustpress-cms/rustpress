//! Advanced Custom Fields 6.8.10 (`advanced-custom-fields/acf.php`) on the website: the post types and taxonomies
//! defined in ACF, the `acf` field of the REST API (`ACF_Rest_Api`) with the values of the field groups that show
//! in REST, in the `light` format or formatted (`?acf_format=standard`), their links (`ACF_Rest_Embed_Links`), and
//! the `[acf]` shortcode, and the fields of the comment form with their scripts, saved with the comment (`acf_form_comment`), and the
//! Ajax validation of forms (`acf/validate_save_post`).
//!
//! Differs from the plugin: field groups, post types and taxonomies come from the database only (no local JSON
//! from `acf-json` and no `acf_add_local_field_group()` in PHP); the `post_template` rule knows the templates of
//! pages only; the PHP filters of ACF do not run; oEmbed fields are links (no network); a user field that returns
//! objects returns the array; `$content_width` of a classic theme does not limit image sizes; block bindings
//! (`acf/field`) wait for block bindings in the core; the comment form of classic themes has no fields, and the
//! media scripts of `acf_enqueue_uploader()` (a hidden `wp_editor()`, `wp_enqueue_media()`) are not printed.

mod admin;
mod admin_lists;
mod field_group_edit;
mod field_settings;
mod field_types;
mod dashicons;
mod edit_attachment;
mod edit_comment;
mod edit_term;
mod edit_user;
mod edit_post;
mod fields;
mod form;
mod format;
mod internal;
mod locations;
mod php;
mod render;
mod rest_format;
mod rest_schema;
mod save;
mod shortcode;
mod value;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, MetaType, NativePlugin, RegisteredMeta, RestFieldContext, RestLink, RestObject};
use rustpress_core::types::Registration;
use rustpress_core::{Result, Site};
use rustpress_db::MetaKind;
use rustpress_phpser::{Key, Value};

use fields::{Field, FieldGroup, MatchContext, Screen};
use value::Object;

pub const VERSION: &str = "6.8.10";

pub struct Acf;

impl NativePlugin for Acf {
    fn file(&self) -> &'static str {
        "advanced-custom-fields/acf.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some("acf")
    }

    fn registrations<'a>(&'a self, site: &'a Site, _wp_root: &'a std::path::Path) -> BoxFuture<'a, Result<Vec<Registration>>> {
        Box::pin(async move {
            let post_types = site.db.acf_posts("acf-post-type").await?;
            let taxonomies = site.db.acf_posts("acf-taxonomy").await?;
            Ok(internal::registrations(&post_types, &taxonomies))
        })
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move { Ok(Box::new(Loaded { groups: fields::load(&site.db).await?, shortcode: shortcode::enabled(&site.options) }) as Box<dyn ActivePlugin>) })
    }
}

pub struct Loaded {
    groups: Vec<FieldGroup>,
    /// `acf_get_setting( 'enable_shortcode' )`.
    shortcode: bool,
}

/// What `ACF_Rest_Request::parse_request()` learns from the route.
#[derive(Debug, Clone, PartialEq)]
struct Request {
    /// `post`, `term`, `user` or `comment`.
    object_type: &'static str,
    /// The post type, taxonomy, `user` or `comment`.
    sub_type: String,
    /// `{post type}-revision` on the revisions and autosaves of a post.
    child: Option<String>,
}

/// `ACF_Rest_Request::build_supported_routes()`, `set_url_params()` and `set_object_types()`.
fn parse_request(site: &Site, route: &str) -> Option<Request> {
    let route = route.to_ascii_lowercase();
    let rest = route.strip_prefix("/wp/v2/")?;
    let mut parts = rest.split('/');
    let base = parts.next()?;
    let tail: Vec<&str> = parts.collect();
    let numeric = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if base == "users" {
        let supported = match tail.as_slice() {
            [] | ["me"] => true,
            [id] => numeric(id),
            _ => false,
        };
        return supported.then(|| Request { object_type: "user", sub_type: "user".into(), child: None });
    }
    if base == "comments" {
        return (tail.is_empty() || (tail.len() == 1 && numeric(tail[0]))).then(|| Request { object_type: "comment", sub_type: "comment".into(), child: None });
    }
    let shape_ok = |revisions: bool, autosaves: bool| match tail.as_slice() {
        [] => Some(None),
        [id] if numeric(id) => Some(None),
        [id, child] | [id, child, _] if numeric(id) && ((*child == "revisions" && revisions) || (*child == "autosaves" && autosaves)) => {
            (tail.len() == 2 || numeric(tail[2])).then_some(Some(()))
        }
        _ => None,
    };
    for (name, rest_base) in post_types(site) {
        if rest_base.to_ascii_lowercase() != base {
            continue;
        }
        let revisions = site.post_type_supports(&name, "revisions");
        return match shape_ok(revisions, name != "attachment")? {
            None => Some(Request { object_type: "post", sub_type: name, child: None }),
            Some(()) => Some(Request { object_type: "post", child: Some(format!("{name}-revision")), sub_type: name }),
        };
    }
    for (name, rest_base) in taxonomies(site) {
        if rest_base.to_ascii_lowercase() == base && (tail.is_empty() || (tail.len() == 1 && numeric(tail[0]))) {
            return Some(Request { object_type: "term", sub_type: name, child: None });
        }
    }
    None
}

/// `register_rest_field( $base, 'acf' )` happened for the object's type: on the current request or an earlier one of
/// the same process (`ACF_Rest_Api::initialize()` on every `rest_pre_dispatch`). The terms controller names
/// `post_tag` `tag`, so that field never shows; revisions are left out.
fn registered_for(ctx: &RestFieldContext<'_>, object: RestObject<'_>, current: Option<&Request>) -> bool {
    let base = match object {
        RestObject::Post(post) if post.post_type != "revision" => post.post_type.clone(),
        RestObject::Template(post_type) => post_type.to_owned(),
        RestObject::Term(term) if term.taxonomy != "post_tag" => term.taxonomy.clone(),
        RestObject::User(_) => "user".to_owned(),
        RestObject::Comment(_) => "comment".to_owned(),
        _ => return false,
    };
    let base_of = |r: &Request| r.child.clone().unwrap_or_else(|| r.sub_type.clone());
    current.is_some_and(|r| base_of(r) == base) || ctx.earlier_routes.iter().filter_map(|route| parse_request(ctx.site, route)).any(|r| base_of(&r) == base)
}

/// `get_post_types( [ 'show_in_rest' => true ] )` with their REST bases (`acf_get_object_type_rest_base()`).
fn post_types(site: &Site) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = [("post", "posts"), ("page", "pages"), ("attachment", "media"), ("nav_menu_item", "menu-items"), ("wp_block", "blocks"), ("wp_template", "templates"), ("wp_template_part", "template-parts"), ("wp_global_styles", "global-styles"), ("wp_navigation", "navigation"), ("wp_font_family", "font-families"), ("wp_font_face", "font-faces")]
        .iter()
        .map(|(n, b)| ((*n).to_owned(), (*b).to_owned()))
        .collect();
    for info in site.types.rewrite_post_types().iter().chain(site.types.post_types.iter()).filter(|t| t.show_in_rest) {
        if !out.iter().any(|(n, _)| *n == info.name) {
            out.push((info.name.clone(), info.rest_base().to_owned()));
        }
    }
    out
}

/// `get_taxonomies( [ 'show_in_rest' => true ] )` with their REST bases.
fn taxonomies(site: &Site) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = [("category", "categories"), ("post_tag", "tags"), ("nav_menu", "menus"), ("wp_pattern_category", "wp_pattern_category")].iter().map(|(n, b)| ((*n).to_owned(), (*b).to_owned())).collect();
    for info in site.types.rewrite_taxonomies().iter().chain(site.types.taxonomies.iter()).filter(|t| t.show_in_rest) {
        if !out.iter().any(|(n, _)| *n == info.name) {
            out.push((info.name.clone(), info.rest_base().to_owned()));
        }
    }
    out
}

/// A REST object with its ACF identifier and the screen its field groups match against.
struct Target<'a> {
    object: Object,
    screen: Screen<'a>,
}

impl Loaded {
    /// `ACF_Rest_Api::register_field()`: the field is registered for the object type of the route only, so an
    /// object of another type (embedded, or an attachment in a post list) gets none.
    async fn target<'a>(&self, object: RestObject<'a>, request: &'a Request, ctx: &RestFieldContext<'_>) -> Result<Option<Target<'a>>> {
        let base = request.child.as_deref().unwrap_or(&request.sub_type);
        Ok(match object {
            RestObject::Post(post) => {
                let is_revision = post.post_type == "revision";
                let matches = match &request.child {
                    Some(_) => is_revision,
                    None => post.post_type == base,
                };
                matches.then(|| Target {
                    object: Object { kind: MetaKind::Post, id: post.id },
                    screen: Screen::Post { post, post_type: request.child.as_ref().map(|_| request.sub_type.as_str()) },
                })
            }
            // The terms controller names its object type `tag` for `post_tag`, so a field registered for
            // `post_tag` never shows.
            RestObject::Term(term) if term.taxonomy == base && request.object_type == "term" && base != "post_tag" => Some(Target { object: Object { kind: MetaKind::Term, id: term.term_id }, screen: Screen::Term { taxonomy: &term.taxonomy } }),
            RestObject::User(user) if base == "user" => Some(Target { object: Object { kind: MetaKind::User, id: user.id }, screen: Screen::User { user_id: Some(user.id), form: None } }),
            RestObject::Comment(comment) if base == "comment" => {
                let post_type = ctx.site.db.post(comment.comment_post_id).await?.map(|p| p.post_type).unwrap_or_default();
                Some(Target { object: Object { kind: MetaKind::Comment, id: comment.comment_id }, screen: Screen::Comment { post_type } })
            }
            _ => None,
        })
    }

    /// The groups of an object of the route when the caller may read it (`current_user_can_read_request_object()`:
    /// the controller's `get_item_permissions_check()` in the `view` context), else `None`.
    async fn readable_object_groups<'a>(&'a self, ctx: &RestFieldContext<'_>, request: &Request, id: u64) -> Result<Option<Vec<&'a FieldGroup>>> {
        let db = &ctx.site.db;
        let viewer = ctx.query.viewer.as_ref();
        let can = |capability: &str| viewer.is_some_and(|v| v.can(capability));
        let matcher = MatchContext { site: ctx.site, user: fields::MatchUser::of(viewer), admin: false };
        match request.object_type {
            "post" => {
                let Some(post) = db.post(id).await?.filter(|p| p.post_type == request.sub_type) else { return Ok(None) };
                let readable = post.post_status == "publish" || viewer.is_some_and(|v| v.can_edit_post(&post)) || (post.post_status == "private" && viewer.is_some_and(|v| v.can_read_private(&post.post_type)));
                if !readable {
                    return Ok(None);
                }
                Ok(Some(self.visible_rest_groups(&Screen::Post { post: &post, post_type: None }, &matcher).await?))
            }
            "term" => {
                if db.term_by_id(Some(&request.sub_type), id).await?.is_none() {
                    return Ok(None);
                }
                Ok(Some(self.visible_rest_groups(&Screen::Term { taxonomy: &request.sub_type }, &matcher).await?))
            }
            "user" => {
                if db.user(id).await?.is_none() {
                    return Ok(None);
                }
                let own = viewer.is_some_and(|v| v.id == id);
                let mut published = 0;
                if !own && !can("list_users") && !can("edit_users") {
                    for (post_type, _) in post_types(ctx.site) {
                        published += db.count_published_user_posts(&post_type, id).await?;
                    }
                    if published == 0 {
                        return Ok(None);
                    }
                }
                Ok(Some(self.visible_rest_groups(&Screen::User { user_id: Some(id), form: None }, &matcher).await?))
            }
            "comment" => {
                let Some(comment) = db.comment_any(id).await? else { return Ok(None) };
                let post = db.post(comment.comment_post_id).await?;
                let readable = can("moderate_comments") || (comment.comment_approved == "1" && post.as_ref().is_some_and(|p| p.post_status == "publish"));
                if !readable {
                    return Ok(None);
                }
                let post_type = post.map(|p| p.post_type).unwrap_or_default();
                Ok(Some(self.visible_rest_groups(&Screen::Comment { post_type }, &matcher).await?))
            }
            _ => Ok(None),
        }
    }

    /// The groups that show in REST and match `screen`.
    async fn visible_rest_groups<'a>(&'a self, screen: &Screen<'_>, matcher: &MatchContext<'_>) -> Result<Vec<&'a FieldGroup>> {
        let mut out = Vec::new();
        for group in self.groups.iter().filter(|g| g.show_in_rest) {
            if group.visible(screen, matcher).await? {
                out.push(group);
            }
        }
        Ok(out)
    }

    /// `ACF_Rest_Api::get_field_groups_by_id()` and `get_fields()`: the fields of the visible groups that show in REST.
    async fn rest_fields<'a>(&'a self, target: &Target<'_>, ctx: &RestFieldContext<'_>) -> Result<Vec<&'a Field>> {
        let matcher = MatchContext { site: ctx.site, user: fields::MatchUser::of(ctx.query.viewer.as_ref()), admin: false };
        let mut out = Vec::new();
        for group in self.groups.iter().filter(|g| g.show_in_rest) {
            if group.visible(&target.screen, &matcher).await? {
                out.extend(group.fields.iter().filter(|f| fields::show_in_rest(&f.kind)));
            }
        }
        Ok(out)
    }
}

impl ActivePlugin for Loaded {
    /// `ACF::register_post_status()`: the status of disabled field groups, post types and taxonomies.
    fn post_statuses(&self, site: &Site) -> Vec<(&'static str, String)> {
        let label = site.i18n().pgettext("acf", "post status", "Inactive");
        vec![("acf-disabled", label)]
    }

    fn admin_pages(&self, site: &Site) -> Vec<rustpress_core::native::AdminPage> {
        admin::admin_pages(site)
    }

    fn admin_enqueue(&self, ctx: &rustpress_core::native::AdminContext<'_>, assets: &mut rustpress_core::native::Assets) {
        admin::admin_enqueue(ctx.site, assets);
        admin_lists::admin_enqueue(ctx, assets);
        field_group_edit::admin_enqueue(ctx, assets);
        edit_comment::admin_enqueue(ctx, assets);
        edit_post::admin_enqueue(ctx, assets);
        edit_term::admin_enqueue(ctx, assets);
        edit_user::admin_enqueue(ctx, assets);
        edit_attachment::admin_enqueue(ctx, assets);
    }

    fn attachment_fields_to_edit<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, attachment: &'a rustpress_db::Post, in_modal: bool) -> BoxFuture<'a, Result<Vec<rustpress_core::native::AttachmentField>>> {
        Box::pin(edit_attachment::attachment_fields_to_edit(&self.groups, ctx, attachment, in_modal))
    }

    fn attachment_fields_to_save<'a>(&'a self, ctx: &'a rustpress_core::native::AttachmentSaved<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::Die>>> {
        Box::pin(save::save_attachment(&self.groups, ctx))
    }

    /// `register_rest_field( $base, 'acf', [ 'schema' => … ] )` of the route: the type-level union of the field
    /// groups, or – on an object's route, where the caller may read it – the groups of the object
    /// (`maybe_apply_per_object_schema()`, `enrich_options_endpoint_args()`). The terms controller names `post_tag`
    /// `tag`, so that schema has none.
    fn rest_schema_fields<'a>(&'a self, ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<(String, serde_json::Value)>>> {
        Box::pin(async move {
            let Some(request) = parse_request(ctx.site, ctx.route) else { return Ok(Vec::new()) };
            let base = request.child.clone().unwrap_or_else(|| request.sub_type.clone());
            if base == "post_tag" || request.child.is_some() {
                return Ok(Vec::new());
            }
            let viewer = ctx.query.viewer.as_ref();
            let can_upload = viewer.is_some_and(|v| v.can("upload_files"));
            let mut groups: Vec<&FieldGroup> = self.groups.iter().filter(|g| g.show_in_rest && rest_schema::object_type_has_field_group(g, request.object_type, &request.sub_type)).collect();
            let id = ctx.route.trim_end_matches('/').rsplit('/').next().and_then(|s| s.parse::<u64>().ok());
            if let Some(id) = id
                && let Some(visible) = self.readable_object_groups(ctx, &request, id).await?
            {
                groups = visible;
            }
            Ok(vec![("acf".to_owned(), rest_schema::build(ctx.site, can_upload, &groups))])
        })
    }

    fn query_attachments_sizes(&self, ctx: &rustpress_core::native::AdminContext<'_>) -> Vec<String> {
        edit_attachment::query_attachments_sizes(&self.groups, ctx)
    }

    fn query_attachment_js(&self, ctx: &rustpress_core::native::AdminContext<'_>, response: &mut serde_json::Map<String, serde_json::Value>) {
        edit_attachment::query_attachment_js(&self.groups, ctx, response);
    }

    /// `ACF_Media::enqueue_scripts()` localizes `wp_mime_type_icon()` wherever `acf-input` is enqueued.
    fn admin_mime_icon_ext(&self, ctx: &rustpress_core::native::AdminContext<'_>) -> Option<&'static str> {
        let mut assets = rustpress_core::native::Assets::default();
        self.admin_enqueue(ctx, &mut assets);
        assets.scripts.iter().any(|a| a.handle == "acf-input").then_some("png")
    }

    fn screen_uploader<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<rustpress_core::native::MetaBoxes>> {
        Box::pin(async move { Ok(edit_attachment::screen_uploader(ctx)) })
    }

    fn user_form<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, user_id: Option<u64>) -> BoxFuture<'a, Result<rustpress_core::native::MetaBoxes>> {
        Box::pin(edit_user::user_form(&self.groups, ctx, user_id))
    }

    fn saved_user<'a>(&'a self, ctx: &'a rustpress_core::native::UserSaved<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::Die>>> {
        Box::pin(save::save_user(&self.groups, ctx))
    }

    fn term_form<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, taxonomy: &'a str, term: Option<&'a rustpress_db::Term>) -> BoxFuture<'a, Result<rustpress_core::native::MetaBoxes>> {
        Box::pin(edit_term::term_form(&self.groups, ctx, taxonomy, term))
    }

    fn saved_term<'a>(&'a self, ctx: &'a rustpress_core::native::TermSaved<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::Die>>> {
        Box::pin(save::save_term(&self.groups, ctx))
    }

    fn post_meta_boxes<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, post: &'a rustpress_db::Post) -> BoxFuture<'a, Result<rustpress_core::native::PostMetaBoxes>> {
        Box::pin(async move {
            if field_group_edit::edited_group(ctx).is_some() {
                return field_group_edit::meta_boxes(&self.groups, ctx, post).await;
            }
            edit_post::meta_boxes(&self.groups, ctx, post).await
        })
    }

    fn save_post<'a>(&'a self, ctx: &'a rustpress_core::native::PostSaved<'a>) -> BoxFuture<'a, Result<()>> {
        Box::pin(save::save_post(&self.groups, ctx))
    }

    fn revision_post_has_changed<'a>(&'a self, ctx: &'a rustpress_core::native::PostSaved<'a>, latest_revision: u64, changed: bool) -> BoxFuture<'a, Result<bool>> {
        Box::pin(save::revision_post_has_changed(&self.groups, ctx, latest_revision, changed))
    }

    fn put_post_revision<'a>(&'a self, ctx: &'a rustpress_core::native::PostSaved<'a>, revision_id: u64) -> BoxFuture<'a, Result<()>> {
        Box::pin(save::maybe_save_revision(&self.groups, ctx, revision_id))
    }

    /// `ACF_Assets::register_scripts()` runs on `init`.
    fn registers_scripts_on_init(&self) -> bool {
        true
    }

    /// `ACF\Blocks\Bindings::register_binding_sources()`.
    fn block_bindings_sources(&self, site: &Site) -> Vec<(String, String, Option<Vec<String>>)> {
        vec![("acf/field".into(), site.i18n().pgettext("acf", "The core ACF block binding source name for fields on the current page", "Custom Fields"), None)]
    }

    fn comment_meta_boxes<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, comment: &'a rustpress_db::Comment) -> BoxFuture<'a, Result<rustpress_core::native::MetaBoxes>> {
        Box::pin(edit_comment::meta_boxes(&self.groups, ctx, comment))
    }

    fn admin_footer<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(format!("{}{}{}", admin_lists::admin_footer(ctx), edit_comment::admin_footer(ctx), edit_term::admin_footer(ctx))) })
    }

    fn admin_print_footer_scripts<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            if let Some(footer) = admin_lists::footer_scripts(ctx) {
                return Ok(footer);
            }
            if let Some(footer) = field_group_edit::footer_scripts(ctx) {
                return Ok(footer);
            }
            if let Some(footer) = edit_post::footer_scripts(&self.groups, ctx).await? {
                return Ok(footer);
            }
            if let Some(footer) = edit_term::footer_scripts(&self.groups, ctx).await? {
                return Ok(footer);
            }
            if let Some(footer) = edit_user::footer_scripts(&self.groups, ctx).await? {
                return Ok(footer);
            }
            if let Some(footer) = edit_attachment::footer_scripts(&self.groups, ctx).await? {
                return Ok(footer);
            }
            edit_comment::footer_scripts(&self.groups, ctx).await
        })
    }

    fn admin_body_class(&self, ctx: &rustpress_core::native::AdminContext<'_>, classes: String) -> String {
        field_group_edit::admin_body_class(ctx, admin_lists::admin_body_class(ctx, admin::admin_body_class(ctx, classes)))
    }

    fn screen_layout(&self, _ctx: &rustpress_core::native::AdminContext<'_>, screen_id: &str) -> Option<u32> {
        field_group_edit::screen_layout(screen_id)
    }

    fn screen_settings<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, html: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(field_group_edit::screen_settings(ctx, html))
    }

    fn parent_file(&self, _ctx: &rustpress_core::native::AdminContext<'_>, parent_file: String) -> String {
        admin_lists::parent_file(parent_file)
    }

    fn submenu_file(&self, _ctx: &rustpress_core::native::AdminContext<'_>, submenu_file: String) -> String {
        admin_lists::submenu_file(submenu_file)
    }

    fn in_admin_header<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move { Ok(admin_lists::in_admin_header(ctx)) })
    }

    fn admin_footer_text(&self, ctx: &rustpress_core::native::AdminContext<'_>) -> Option<String> {
        admin_lists::footer_text(ctx)
    }

    fn update_footer(&self, ctx: &rustpress_core::native::AdminContext<'_>) -> Option<String> {
        admin_lists::update_footer(ctx)
    }

    fn screen_help(&self, ctx: &rustpress_core::native::AdminContext<'_>) -> Option<rustpress_core::native::AdminHelp> {
        admin_lists::help(ctx)
    }

    fn list_statuses(&self, _site: &Site) -> Vec<rustpress_core::native::ListStatus> {
        admin_lists::list_statuses()
    }

    fn status_label_count(&self, ctx: &rustpress_core::native::AdminContext<'_>, status: &str, n: u64) -> Option<String> {
        admin_lists::status_label_count(ctx, status, n)
    }

    fn posts_columns(&self, ctx: &rustpress_core::native::AdminContext<'_>, post_type: &str, columns: Vec<(String, String)>) -> Vec<(String, String)> {
        admin_lists::posts_columns(ctx, post_type, columns)
    }

    fn posts_custom_column<'a>(&'a self, ctx: &'a rustpress_core::native::AdminContext<'a>, column: &'a str, post: &'a rustpress_db::Post) -> BoxFuture<'a, Result<String>> {
        Box::pin(admin_lists::posts_custom_column(&self.groups, ctx, column, post))
    }

    fn post_row_actions(&self, ctx: &rustpress_core::native::AdminContext<'_>, post: &rustpress_db::Post, hierarchical: bool, actions: Vec<(String, String)>) -> Vec<(String, String)> {
        admin_lists::post_row_actions(ctx, post, hierarchical, actions)
    }

    fn bulk_actions(&self, ctx: &rustpress_core::native::AdminContext<'_>, _screen_id: &str, actions: Vec<(String, String)>) -> Vec<(String, String)> {
        admin_lists::bulk_actions(ctx, actions)
    }

    fn list_views(&self, ctx: &rustpress_core::native::AdminContext<'_>, _screen_id: &str, views: Vec<(String, String)>) -> Vec<(String, String)> {
        admin_lists::list_views(ctx, views)
    }

    fn display_post_states(&self, ctx: &rustpress_core::native::AdminContext<'_>, post: &rustpress_db::Post, states: Vec<(String, String)>) -> Vec<(String, String)> {
        admin_lists::display_post_states(ctx, post, states)
    }

    fn list_not_found(&self, ctx: &rustpress_core::native::AdminContext<'_>, post_type: &str) -> Option<String> {
        admin_lists::list_not_found(ctx, post_type)
    }

    fn admin_ajax<'a>(&'a self, action: &'a str, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::AjaxAnswer>>> {
        Box::pin(async move { Ok((action == "acf/validate_save_post").then(|| save::ajax_validate(&self.groups, ctx))) })
    }

    fn admin_ajax_nopriv<'a>(&'a self, action: &'a str, ctx: &'a rustpress_core::native::AdminContext<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::AjaxAnswer>>> {
        Box::pin(async move { Ok((action == "acf/validate_save_post").then(|| save::ajax_validate(&self.groups, ctx))) })
    }

    fn comment_post<'a>(&'a self, ctx: &'a rustpress_core::native::CommentSaved<'a>) -> BoxFuture<'a, Result<Option<rustpress_core::native::Die>>> {
        Box::pin(save::save_comment(&self.groups, ctx))
    }

    fn comment_form_field_comment<'a>(&'a self, ctx: &'a rustpress_core::native::CommentFormContext<'a>, html: String) -> BoxFuture<'a, Result<String>> {
        Box::pin(form::comment_form_field_comment(&self.groups, ctx, html))
    }

    fn shortcode_tags(&self) -> &'static [&'static str] {
        &["acf"]
    }

    fn shortcode<'a>(&'a self, _tag: &'a str, atts: &'a rustpress_core::shortcodes::Atts, _content: Option<&'a str>, ctx: &'a rustpress_core::native::ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(shortcode::render(&self.groups, self.shortcode, atts, ctx))
    }

    /// `ACF_Revisions::register_meta()`.
    fn registered_post_meta(&self) -> Vec<RegisteredMeta> {
        vec![RegisteredMeta { key: "_acf_changed", post_type: None, kind: MetaType::Boolean }]
    }

    fn rest_fields<'a>(&'a self, object: RestObject<'a>, ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<(String, serde_json::Value)>>> {
        Box::pin(async move {
            let request = parse_request(ctx.site, ctx.route);
            let target = match &request {
                Some(request) => self.target(object, request, ctx).await?,
                None => None,
            };
            let (Some(_), Some(target)) = (&request, target) else {
                // The field is registered for the object's type, but the request names no object of it:
                // `load_fields()` finds no ID or no object type.
                return Ok(if registered_for(ctx, object, request.as_ref()) { vec![("acf".to_owned(), serde_json::Value::Array(Vec::new()))] } else { Vec::new() });
            };
            // `ACF_Rest_Api::load_fields()`: `$request->get_param( 'acf_format' )`, else the `light` default.
            let standard = acf_format(ctx.query_string).as_deref() == Some("standard");
            let global_post = match object {
                RestObject::Post(post) => Some(post),
                _ => None,
            };
            let formatter = format::Formatter { site: ctx.site, services: ctx.services, global_post };
            let sanitizer = rest_format::Sanitizer { site: ctx.site, viewer: ctx.query.viewer.as_ref() };
            let mut values: Vec<(Key, Value)> = Vec::new();
            for field in self.rest_fields(&target, ctx).await? {
                let raw = value::get_value(&ctx.site.db, target.object, field, &field.name).await?;
                let formatted = if standard {
                    let formatted = formatter.format(field, raw.clone(), false).await?;
                    sanitizer.apply(formatted, &raw, field).await?
                } else {
                    value::format_for_rest(raw, field)
                };
                php::set(&mut values, &field.name, formatted);
            }
            Ok(vec![("acf".to_owned(), rustpress_core::options::php_to_json(&Value::Array(values)))])
        })
    }

    fn rest_links<'a>(&'a self, object: RestObject<'a>, ctx: &'a RestFieldContext<'a>) -> BoxFuture<'a, Result<Vec<RestLink>>> {
        Box::pin(async move {
            let Some(request) = parse_request(ctx.site, ctx.route) else { return Ok(Vec::new()) };
            // The links are gathered while `load_fields()` runs, which it does not when `_fields` leaves `acf` out.
            if !(ctx.field_included)("acf") {
                return Ok(Vec::new());
            }
            // `ACF_Rest_Embed_Links::hook_link_handlers()`: posts, terms and users; comments keep theirs.
            if matches!(object, RestObject::Comment(_)) {
                return Ok(Vec::new());
            }
            let Some(target) = self.target(object, &request, ctx).await? else { return Ok(Vec::new()) };
            // `prepare_links()`: one link per `rel:href`, the later one in the place of the first.
            let mut links: Vec<(String, RestLink)> = Vec::new();
            for field in self.rest_fields(&target, ctx).await? {
                let raw = value::get_value(&ctx.site.db, target.object, field, &field.name).await?;
                for link in field_links(ctx, field, &raw).await? {
                    let key = format!("{}:{}", link.rel, link.href);
                    match links.iter_mut().find(|(k, _)| *k == key) {
                        Some(slot) => slot.1 = link,
                        None => links.push((key, link)),
                    }
                }
            }
            // `load_item_links()` pops them: the last one comes first.
            Ok(links.into_iter().rev().map(|(_, link)| link).collect())
        })
    }
}

/// The last `acf_format` of a query string, decoded like PHP does (`$_GET`).
fn acf_format(query_string: &str) -> Option<String> {
    query_string
        .split('&')
        .map(|pair| pair.split_once('=').unwrap_or((pair, "")))
        .filter(|(name, _)| rustpress_core::rewrite::urldecode(name.as_bytes()) == b"acf_format")
        .map(|(_, value)| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(value.as_bytes())).into_owned())
        .next_back()
}

/// The ids of a value: `(array) $value`.
fn ids(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(|(_, v)| php::strval(Some(v))).collect(),
        other => vec![php::strval(Some(other))],
    }
}

/// `get_rest_links()` of the post object, page link, relationship, taxonomy and user fields.
async fn field_links(ctx: &RestFieldContext<'_>, field: &Field, value: &Value) -> Result<Vec<RestLink>> {
    let mut out = Vec::new();
    if !php::truthy(Some(value)) {
        return Ok(out);
    }
    let site = ctx.site;
    let embeddable = || vec![("embeddable".to_owned(), serde_json::Value::Bool(true))];
    match field.kind.as_str() {
        "post_object" | "page_link" | "relationship" => {
            for id in ids(value) {
                // `get_post_type( $object_id )`: a number, or a string PHP casts to one.
                let Ok(post_id) = id.trim().parse::<u64>() else { continue };
                let Some(post) = site.db.post(post_id).await? else { continue };
                let Some((_, rest_base)) = post_types(site).into_iter().find(|(n, _)| *n == post.post_type).or_else(|| (site.types.is_post_type(&post.post_type) || site.types.hidden_post_types.contains(&post.post_type)).then(|| (post.post_type.clone(), post.post_type.clone()))) else {
                    continue;
                };
                let rel = if post.post_type == "attachment" { "acf:attachment" } else { "acf:post" };
                out.push(RestLink { rel: rel.to_owned(), attributes: embeddable(), href: site.rest_url(&format!("/wp/v2/{rest_base}/{id}")) });
            }
        }
        "taxonomy" => {
            for id in ids(value) {
                let Ok(term_id) = id.trim().parse::<u64>() else { continue };
                let Some(term) = site.db.term_by_id(None, term_id).await? else { continue };
                let Some((_, rest_base)) = taxonomies(site).into_iter().find(|(n, _)| *n == term.taxonomy) else { continue };
                let mut attributes = embeddable();
                attributes.push(("taxonomy".to_owned(), serde_json::Value::from(term.taxonomy.clone())));
                out.push(RestLink { rel: "acf:term".to_owned(), attributes, href: site.rest_url(&format!("/wp/v2/{rest_base}/{id}")) });
            }
        }
        "user" => {
            for id in ids(value) {
                out.push(RestLink { rel: "acf:user".to_owned(), attributes: embeddable(), href: site.rest_url(&format!("/wp/v2/users/{id}")) });
            }
        }
        _ => {}
    }
    Ok(out)
}
