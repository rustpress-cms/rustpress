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

mod dashicons;
mod fields;
mod form;
mod format;
mod internal;
mod php;
mod render;
mod rest_format;
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
#[derive(Debug, PartialEq)]
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
            RestObject::User(user) if base == "user" => Some(Target { object: Object { kind: MetaKind::User, id: user.id }, screen: Screen::User { user_id: user.id } }),
            RestObject::Comment(comment) if base == "comment" => {
                let post_type = ctx.site.db.post(comment.comment_post_id).await?.map(|p| p.post_type).unwrap_or_default();
                Some(Target { object: Object { kind: MetaKind::Comment, id: comment.comment_id }, screen: Screen::Comment { post_type } })
            }
            _ => None,
        })
    }

    /// `ACF_Rest_Api::get_field_groups_by_id()` and `get_fields()`: the fields of the visible groups that show in REST.
    async fn rest_fields<'a>(&'a self, target: &Target<'_>, ctx: &RestFieldContext<'_>) -> Result<Vec<&'a Field>> {
        let matcher = MatchContext { site: ctx.site, viewer: ctx.query.viewer.as_ref() };
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
            let Some(request) = parse_request(ctx.site, ctx.route) else { return Ok(Vec::new()) };
            let Some(target) = self.target(object, &request, ctx).await? else { return Ok(Vec::new()) };
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
