//! The post types and taxonomies defined in ACF (`ACF_Post_Type`, `ACF_Taxonomy`): the `acf-post-type` and
//! `acf-taxonomy` posts become `register_post_type()` and `register_taxonomy()` calls.

use rustpress_core::php_lit::Lit;
use rustpress_core::types::Registration;
use rustpress_db::AcfPost;
use rustpress_phpser::{Key, Value};

use super::php::{intval, key_eq, parse_args, strval, to_lit, truthy};

/// `ACF_Internal_Post_Type::get_raw_post()` and `validate_post()`: the stored settings over the defaults, with the
/// attributes of the post; `"0"`/`"1"` and `"false"`/`"true"` become booleans where the default is one.
pub fn validated(post: &AcfPost, defaults: Value) -> Value {
    let mut raw = match rustpress_core::options::maybe_unserialize(&post.content) {
        Value::Array(items) => items,
        _ => Vec::new(),
    };
    let set = |raw: &mut Vec<(Key, Value)>, name: &str, value: Value| super::php::set(raw, name, value);
    set(&mut raw, "ID", Value::Int(post.id as i64));
    set(&mut raw, "title", super::php::string(&post.title));
    set(&mut raw, "key", super::php::string(&post.name));
    set(&mut raw, "menu_order", Value::Int(post.menu_order));
    set(&mut raw, "active", Value::Bool(matches!(post.status.as_str(), "publish" | "auto-draft")));
    let Value::Array(default_items) = &defaults else { return Value::Array(raw) };
    let boolean: Vec<Key> = default_items.iter().filter(|(_, v)| matches!(v, Value::Bool(_))).map(|(k, _)| k.clone()).collect();
    let Value::Array(mut items) = parse_args(&Value::Array(raw), defaults) else { unreachable!() };
    for (k, v) in &mut items {
        if !boolean.iter().any(|b| key_eq(b, k)) {
            continue;
        }
        match v {
            Value::Str(s) if s == b"0" || s == b"1" => *v = Value::Bool(s == b"1"),
            Value::Str(s) if s == b"false" || s == b"true" => *v = Value::Bool(s == b"true"),
            _ => {}
        }
    }
    Value::Array(items)
}

fn s(text: &str) -> Value {
    super::php::string(text)
}

fn arr(items: &[(&str, Value)]) -> Value {
    Value::Array(items.iter().map(|(k, v)| (super::php::key(k), v.clone())).collect())
}

/// `ACF_Post_Type::get_settings_array()`.
pub fn post_type_defaults() -> Value {
    let labels = [
        "name", "singular_name", "menu_name", "all_items", "add_new", "add_new_item", "edit_item", "new_item", "view_item", "view_items", "search_items", "not_found", "not_found_in_trash",
        "parent_item_colon", "archives", "attributes", "featured_image", "set_featured_image", "remove_featured_image", "use_featured_image", "insert_into_item", "uploaded_to_this_item",
        "filter_items_list", "filter_by_date", "items_list_navigation", "items_list", "item_published", "item_published_privately", "item_reverted_to_draft", "item_scheduled", "item_updated",
        "item_link", "item_link_description",
    ];
    let t = Value::Bool(true);
    let f = Value::Bool(false);
    arr(&[
        ("ID", Value::Int(0)),
        ("key", s("")),
        ("title", s("")),
        ("menu_order", Value::Int(0)),
        ("active", t.clone()),
        ("post_type", s("")),
        ("advanced_configuration", f.clone()),
        ("import_source", s("")),
        ("import_date", s("")),
        ("allow_ai_access", f.clone()),
        ("ai_description", s("")),
        ("labels", Value::Array(labels.iter().map(|l| (super::php::key(l), s(""))).collect())),
        ("description", s("")),
        ("public", t.clone()),
        ("hierarchical", f.clone()),
        ("exclude_from_search", f.clone()),
        ("publicly_queryable", t.clone()),
        ("show_ui", t.clone()),
        ("show_in_menu", t.clone()),
        ("admin_menu_parent", s("")),
        ("show_in_admin_bar", t.clone()),
        ("show_in_nav_menus", t.clone()),
        ("show_in_rest", t.clone()),
        ("rest_base", s("")),
        ("rest_namespace", s("wp/v2")),
        ("rest_controller_class", s("WP_REST_Posts_Controller")),
        ("menu_position", Value::Null),
        ("menu_icon", s("")),
        ("rename_capabilities", f.clone()),
        ("singular_capability_name", s("post")),
        ("plural_capability_name", s("posts")),
        ("supports", Value::Array(["title", "editor", "thumbnail", "custom-fields"].iter().enumerate().map(|(i, v)| (Key::Int(i as i64), s(v))).collect())),
        ("taxonomies", Value::Array(Vec::new())),
        ("has_archive", f.clone()),
        ("has_archive_slug", s("")),
        ("rewrite", arr(&[("permalink_rewrite", s("post_type_key")), ("slug", s("")), ("feeds", f.clone()), ("pages", t.clone()), ("with_front", t.clone())])),
        ("query_var", s("post_type_key")),
        ("query_var_name", s("")),
        ("can_export", t),
        ("delete_with_user", f),
        ("register_meta_box_cb", s("")),
        ("enter_title_here", s("")),
    ])
}

/// `ACF_Taxonomy::get_settings_array()`.
pub fn taxonomy_defaults() -> Value {
    let labels = [
        "singular_name", "name", "menu_name", "search_items", "popular_items", "all_items", "parent_item", "parent_item_colon", "name_field_description", "slug_field_description",
        "parent_field_description", "desc_field_description", "edit_item", "view_item", "update_item", "add_new_item", "new_item_name", "separate_items_with_commas", "add_or_remove_items",
        "choose_from_most_used", "not_found", "no_terms", "filter_by_item", "items_list_navigation", "items_list", "most_used", "back_to_items", "item_link", "item_link_description",
    ];
    let t = Value::Bool(true);
    let f = Value::Bool(false);
    arr(&[
        ("ID", Value::Int(0)),
        ("key", s("")),
        ("title", s("")),
        ("menu_order", Value::Int(0)),
        ("active", t.clone()),
        ("taxonomy", s("")),
        ("object_type", Value::Array(Vec::new())),
        ("advanced_configuration", Value::Int(0)),
        ("import_source", s("")),
        ("import_date", s("")),
        ("labels", Value::Array(labels.iter().map(|l| (super::php::key(l), s(""))).collect())),
        ("description", s("")),
        (
            "capabilities",
            arr(&[("manage_terms", s("manage_categories")), ("edit_terms", s("manage_categories")), ("delete_terms", s("manage_categories")), ("assign_terms", s("edit_posts"))]),
        ),
        ("public", t.clone()),
        ("publicly_queryable", t.clone()),
        ("hierarchical", f.clone()),
        ("show_ui", t.clone()),
        ("show_in_menu", t.clone()),
        ("show_in_nav_menus", t.clone()),
        ("show_in_rest", t.clone()),
        ("rest_base", s("")),
        ("rest_namespace", s("wp/v2")),
        ("rest_controller_class", s("WP_REST_Terms_Controller")),
        ("show_tagcloud", t.clone()),
        ("show_in_quick_edit", t.clone()),
        ("show_admin_column", f.clone()),
        ("rewrite", arr(&[("permalink_rewrite", s("taxonomy_key")), ("slug", s("")), ("with_front", t.clone()), ("rewrite_hierarchical", f.clone())])),
        ("query_var", s("taxonomy_key")),
        ("query_var_name", s("")),
        (
            "default_term",
            arr(&[("default_term_enabled", f.clone()), ("default_term_name", s("")), ("default_term_slug", s("")), ("default_term_description", s(""))]),
        ),
        ("sort", Value::Null),
        ("meta_box", s("default")),
        ("meta_box_cb", s("")),
        ("meta_box_sanitize_cb", s("")),
        ("allow_ai_access", f.clone()),
        ("ai_description", s("")),
    ])
}

/// The arguments being built, in the order PHP assigns them.
#[derive(Default)]
struct Args(Vec<(Option<Lit>, Lit)>);

impl Args {
    fn set(&mut self, name: &str, value: Lit) {
        match self.0.iter_mut().find(|(k, _)| k.as_ref().and_then(Lit::as_str) == Some(name)) {
            Some(slot) => slot.1 = value,
            None => self.0.push((Some(Lit::Str(name.to_owned())), value)),
        }
    }
}

fn get<'a>(post: &'a Value, name: &str) -> Option<&'a Value> {
    post.get(name)
}

fn boolval(value: Option<&Value>) -> bool {
    truthy(value)
}

/// `array_map( 'esc_html', array_map( 'strval', array_filter( $post['labels'] ) ) )`.
fn labels(post: &Value) -> Option<Lit> {
    let items = get(post, "labels").and_then(Value::as_array)?;
    let labels: Vec<(Option<Lit>, Lit)> = items
        .iter()
        .filter(|(_, v)| truthy(Some(v)))
        .map(|(k, v)| {
            let key = match k {
                Key::Int(i) => Lit::Int(*i),
                Key::Str(s) => Lit::Str(String::from_utf8_lossy(s).into_owned()),
            };
            (Some(key), Lit::Str(rustpress_core::formatting::esc_html(&strval(Some(v)))))
        })
        .collect();
    (!labels.is_empty()).then_some(Lit::Array(labels))
}

/// `(array) $value`.
fn to_array(value: Option<&Value>) -> Vec<(Key, Value)> {
    match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items.clone(),
        Some(other) => vec![(Key::Int(0), other.clone())],
    }
}

fn key_name(name: &str) -> Lit {
    Lit::Str(name.to_owned())
}

/// The rewrite settings both builders share.
struct Rewrite {
    enabled: bool,
    args: Args,
    settings: Vec<(Key, Value)>,
}

fn rewrite(post: &Value, key_default: &str, type_key: &str) -> Rewrite {
    let settings = to_array(get(post, "rewrite"));
    // `isset()`: a null counts as missing.
    let find = |name: &str| settings.iter().find(|(k, v)| key_eq(k, &super::php::key(name)) && !matches!(v, Value::Null)).map(|(_, v)| v);
    let permalink_rewrite = find("permalink_rewrite").map_or_else(|| key_default.to_owned(), |v| strval(Some(v)));
    let enabled = permalink_rewrite != "no_permalink";
    let mut args = Args::default();
    if let Some(slug) = find("slug")
        && truthy(Some(slug))
        && strval(Some(slug)) != strval(get(post, type_key))
        && permalink_rewrite == "custom_permalink"
    {
        args.set("slug", Lit::Str(strval(Some(slug))));
    }
    if let Some(with_front) = find("with_front")
        && !truthy(Some(with_front))
        && enabled
    {
        args.set("with_front", Lit::Bool(false));
    }
    Rewrite { enabled, args, settings }
}

/// `query_var`: `custom_query_var` with its name, `none` for none, the key otherwise.
fn query_var(post: &Value, type_key: &str, args: &mut Args) {
    match strval(get(post, "query_var")).as_str() {
        "custom_query_var" => {
            let name = strval(get(post, "query_var_name"));
            if !name.is_empty() && name != "0" && name != strval(get(post, type_key)) {
                args.set("query_var", Lit::Str(name));
            }
        }
        "none" => args.set("query_var", Lit::Bool(false)),
        _ => {}
    }
}

/// `ACF_Post_Type::get_post_type_args()`. A menu icon from the media library is left unknown.
pub fn post_type_args(post: &Value) -> Lit {
    let mut args = Args::default();
    if let Some(labels) = labels(post) {
        args.set("labels", labels);
    }
    if truthy(get(post, "description")) {
        args.set("description", Lit::Str(strval(get(post, "description"))));
    }
    let public = truthy(get(post, "public"));
    args.set("public", Lit::Bool(public));
    if truthy(get(post, "hierarchical")) {
        args.set("hierarchical", Lit::Bool(true));
    }
    let exclude_from_search = boolval(get(post, "exclude_from_search"));
    if exclude_from_search == public {
        args.set("exclude_from_search", Lit::Bool(exclude_from_search));
    }
    let publicly_queryable = boolval(get(post, "publicly_queryable"));
    if publicly_queryable != public {
        args.set("publicly_queryable", Lit::Bool(publicly_queryable));
    }
    let show_ui = boolval(get(post, "show_ui"));
    if show_ui != public {
        args.set("show_ui", Lit::Bool(show_ui));
    }
    let show_in_menu = boolval(get(post, "show_in_menu"));
    if show_in_menu != show_ui {
        args.set("show_in_menu", Lit::Bool(show_in_menu));
    }
    if show_in_menu && truthy(get(post, "admin_menu_parent")) {
        args.set("show_in_menu", Lit::Str(strval(get(post, "admin_menu_parent"))));
    }
    let show_in_nav_menus = boolval(get(post, "show_in_nav_menus"));
    if show_in_nav_menus != public {
        args.set("show_in_nav_menus", Lit::Bool(show_in_nav_menus));
    }
    let show_in_admin_bar = boolval(get(post, "show_in_admin_bar"));
    if show_in_admin_bar != show_in_menu {
        args.set("show_in_admin_bar", Lit::Bool(show_in_admin_bar));
    }
    args.set("show_in_rest", Lit::Bool(boolval(get(post, "show_in_rest"))));
    let rest_base = strval(get(post, "rest_base"));
    if !rest_base.is_empty() && rest_base != "0" && rest_base != strval(get(post, "post_type")) {
        args.set("rest_base", Lit::Str(rest_base));
    }
    let rest_namespace = strval(get(post, "rest_namespace"));
    if !rest_namespace.is_empty() && rest_namespace != "0" && rest_namespace != "wp/v2" {
        args.set("rest_namespace", get(post, "rest_namespace").map_or(Lit::Null, to_lit));
    }
    let controller = strval(get(post, "rest_controller_class"));
    if !controller.is_empty() && controller != "0" && controller != "WP_REST_Posts_Controller" {
        args.set("rest_controller_class", Lit::Str(controller));
    }
    let menu_position = intval(get(post, "menu_position"));
    if menu_position != 0 {
        args.set("menu_position", Lit::Int(menu_position));
    }
    args.set("menu_icon", Lit::Str("dashicons-admin-post".to_owned()));
    if truthy(get(post, "menu_icon")) {
        match get(post, "menu_icon") {
            Some(Value::Str(icon)) => args.set("menu_icon", Lit::Str(String::from_utf8_lossy(icon).into_owned())),
            Some(icon @ Value::Array(_)) => {
                if strval(icon.get("type")) == "media_library" {
                    args.set("menu_icon", Lit::Unknown);
                } else {
                    args.set("menu_icon", icon.get("value").map_or(Lit::Null, to_lit));
                }
            }
            _ => {}
        }
    }
    if boolval(get(post, "rename_capabilities")) {
        let singular = strval(get(post, "singular_capability_name"));
        let plural = strval(get(post, "plural_capability_name"));
        let empty = |s: &str| s.is_empty() || s == "0";
        let capability_type = if !empty(&singular) && !empty(&plural) {
            Some(Lit::Array(vec![(Some(Lit::Int(0)), Lit::Str(singular.clone())), (Some(Lit::Int(1)), Lit::Str(plural.clone()))]))
        } else if !empty(&singular) {
            Some(Lit::Str(singular.clone()))
        } else {
            None
        };
        let is_default = match &capability_type {
            None => true,
            Some(Lit::Str(s)) => s == "post",
            Some(_) => singular == "post" && plural == "posts",
        };
        if let Some(capability_type) = capability_type.filter(|_| !is_default) {
            args.set("capability_type", capability_type);
            args.set("map_meta_cap", Lit::Bool(true));
        }
    }
    // `array_unique( array_filter( array_map( 'strval', $supports ) ) )`, with the notes of the editor (WP 6.9).
    let mut supports: Vec<String> = Vec::new();
    if let Some(Value::Array(items)) = get(post, "supports") {
        for (_, v) in items {
            let feature = strval(Some(v));
            if !feature.is_empty() && feature != "0" && !supports.contains(&feature) {
                supports.push(feature);
            }
        }
    }
    let mut supports_lit: Vec<(Option<Lit>, Lit)> = Vec::new();
    if supports.iter().any(|f| f == "notes") && supports.iter().any(|f| f == "editor") {
        supports.retain(|f| f != "notes" && f != "editor");
        supports_lit.extend(supports.iter().enumerate().map(|(i, f)| (Some(Lit::Int(i as i64)), Lit::Str(f.clone()))));
        supports_lit.push((Some(key_name("editor")), Lit::Array(vec![(Some(key_name("notes")), Lit::Bool(true))])));
    } else {
        supports_lit.extend(supports.iter().enumerate().map(|(i, f)| (Some(Lit::Int(i as i64)), Lit::Str(f.clone()))));
    }
    args.set("supports", if supports_lit.is_empty() { Lit::Bool(false) } else { Lit::Array(supports_lit) });
    if truthy(get(post, "register_meta_box_cb")) {
        args.set("register_meta_box_cb", Lit::Unknown);
    }
    let taxonomies: Vec<(Key, Value)> = to_array(get(post, "taxonomies")).into_iter().filter(|(_, v)| truthy(Some(v))).collect();
    if !taxonomies.is_empty() {
        args.set("taxonomies", to_lit(&Value::Array(taxonomies)));
    }
    let has_archive = boolval(get(post, "has_archive"));
    if has_archive {
        let slug = strval(get(post, "has_archive_slug"));
        args.set("has_archive", if !slug.is_empty() && slug != "0" { Lit::Str(slug) } else { Lit::Bool(true) });
    }
    let mut rw = rewrite(post, "post_type_key", "post_type");
    let find = |name: &str| rw.settings.iter().find(|(k, v)| key_eq(k, &super::php::key(name)) && !matches!(v, Value::Null)).map(|(_, v)| v.clone());
    if let Some(feeds) = find("feeds")
        && truthy(Some(&feeds)) != has_archive
        && rw.enabled
    {
        rw.args.set("feeds", Lit::Bool(truthy(Some(&feeds))));
    }
    if let Some(pages) = find("pages")
        && !truthy(Some(&pages))
        && rw.enabled
    {
        rw.args.set("pages", Lit::Bool(false));
    }
    if !rw.args.0.is_empty() {
        args.set("rewrite", Lit::Array(rw.args.0));
    } else if !rw.enabled {
        args.set("rewrite", Lit::Bool(false));
    }
    query_var(post, "post_type", &mut args);
    if !boolval(get(post, "can_export")) {
        args.set("can_export", Lit::Bool(false));
    }
    args.set("delete_with_user", Lit::Bool(boolval(get(post, "delete_with_user"))));
    Lit::Array(args.0)
}

/// `ACF_Taxonomy::get_taxonomy_args()`.
pub fn taxonomy_args(post: &Value) -> Lit {
    let mut args = Args::default();
    if let Some(labels) = labels(post) {
        args.set("labels", labels);
    }
    if truthy(get(post, "description")) {
        args.set("description", Lit::Str(strval(get(post, "description"))));
    }
    let public = truthy(get(post, "public"));
    args.set("public", Lit::Bool(public));
    if truthy(get(post, "hierarchical")) {
        args.set("hierarchical", Lit::Bool(true));
    }
    let publicly_queryable = boolval(get(post, "publicly_queryable"));
    if publicly_queryable != public {
        args.set("publicly_queryable", Lit::Bool(publicly_queryable));
    }
    let show_ui = boolval(get(post, "show_ui"));
    if show_ui != public {
        args.set("show_ui", Lit::Bool(show_ui));
    }
    // A strict comparison of the stored value with a boolean.
    let show_in_menu = get(post, "show_in_menu");
    if show_in_menu != Some(&Value::Bool(show_ui)) {
        args.set("show_in_menu", Lit::Bool(boolval(show_in_menu)));
    }
    let show_in_nav_menus = boolval(get(post, "show_in_nav_menus"));
    if show_in_nav_menus != public {
        args.set("show_in_nav_menus", Lit::Bool(show_in_nav_menus));
    }
    args.set("show_in_rest", Lit::Bool(boolval(get(post, "show_in_rest"))));
    let rest_base = strval(get(post, "rest_base"));
    if !rest_base.is_empty() && rest_base != "0" && rest_base != strval(get(post, "taxonomy")) {
        args.set("rest_base", Lit::Str(rest_base));
    }
    let rest_namespace = strval(get(post, "rest_namespace"));
    if !rest_namespace.is_empty() && rest_namespace != "0" && rest_namespace != "wp/v2" {
        args.set("rest_namespace", get(post, "rest_namespace").map_or(Lit::Null, to_lit));
    }
    let controller = strval(get(post, "rest_controller_class"));
    if !controller.is_empty() && controller != "0" && controller != "WP_REST_Terms_Controller" {
        args.set("rest_controller_class", Lit::Str(controller));
    }
    let show_tagcloud = boolval(get(post, "show_tagcloud"));
    if show_tagcloud != show_ui {
        args.set("show_tagcloud", Lit::Bool(show_tagcloud));
    }
    // ACF passes `show_tagcloud` here.
    if boolval(get(post, "show_in_quick_edit")) != show_ui {
        args.set("show_in_quick_edit", Lit::Bool(show_tagcloud));
    }
    if boolval(get(post, "show_admin_column")) {
        args.set("show_admin_column", Lit::Bool(true));
    }
    let mut capabilities = Args::default();
    let caps = get(post, "capabilities");
    for (cap, default) in [("manage_terms", "manage_categories"), ("edit_terms", "manage_categories"), ("delete_terms", "manage_categories"), ("assign_terms", "edit_posts")] {
        let value = caps.and_then(|c| c.get(cap));
        if truthy(value) && strval(value) != default {
            capabilities.set(cap, Lit::Str(strval(value)));
        }
    }
    if !capabilities.0.is_empty() {
        args.set("capabilities", Lit::Array(capabilities.0));
    }
    let meta_box = get(post, "meta_box").map_or_else(|| "default".to_owned(), |v| strval(Some(v)));
    if meta_box == "custom" && truthy(get(post, "meta_box_cb")) {
        args.set("meta_box_cb", Lit::Unknown);
        if truthy(get(post, "meta_box_sanitize_cb")) {
            args.set("meta_box_sanitize_cb", Lit::Str(strval(get(post, "meta_box_sanitize_cb"))));
        }
    } else if meta_box == "disabled" {
        args.set("meta_box_cb", Lit::Bool(false));
    }
    let mut rw = rewrite(post, "taxonomy_key", "taxonomy");
    if let Some((_, hierarchical)) = rw.settings.iter().find(|(k, _)| key_eq(k, &super::php::key("rewrite_hierarchical")))
        && truthy(Some(hierarchical))
        && rw.enabled
    {
        rw.args.set("hierarchical", Lit::Bool(true));
    }
    if rw.enabled && !rw.args.0.is_empty() {
        args.set("rewrite", Lit::Array(rw.args.0));
    } else if !rw.enabled {
        args.set("rewrite", Lit::Bool(false));
    }
    query_var(post, "taxonomy", &mut args);
    let default_term = to_array(get(post, "default_term"));
    let find = |name: &str| default_term.iter().find(|(k, v)| key_eq(k, &super::php::key(name)) && !matches!(v, Value::Null)).map(|(_, v)| v);
    if truthy(find("default_term_enabled")) {
        let mut term = Args::default();
        for (from, to) in [("default_term_name", "name"), ("default_term_slug", "slug"), ("default_term_description", "description")] {
            if truthy(find(from)) {
                term.set(to, Lit::Str(strval(find(from))));
            }
        }
        args.set("default_term", Lit::Array(term.0));
    }
    if boolval(get(post, "sort")) {
        args.set("sort", Lit::Bool(true));
    }
    Lit::Array(args.0)
}

/// `ACF_Post_Type::register_post_types()` and `ACF_Taxonomy::register_taxonomies()` on `acf/init`: the active
/// post types, then the active taxonomies; a type that already exists is skipped.
pub fn registrations(post_types: &[AcfPost], taxonomies: &[AcfPost]) -> Vec<Registration> {
    let mut out: Vec<Registration> = Vec::new();
    for post in post_types {
        let post = validated(post, post_type_defaults());
        let name = strval(post.get("post_type"));
        if !truthy(post.get("active")) || name.is_empty() || is_builtin_post_type(&name) || out.iter().any(|r| !r.taxonomy && r.name == name) {
            continue;
        }
        out.push(Registration { name, taxonomy: false, args: post_type_args(&post), object_types: Vec::new() });
    }
    for post in taxonomies {
        let post = validated(post, taxonomy_defaults());
        let name = strval(post.get("taxonomy"));
        if !truthy(post.get("active")) || name.is_empty() || is_builtin_taxonomy(&name) || out.iter().any(|r| r.taxonomy && r.name == name) {
            continue;
        }
        let object_types = to_array(post.get("object_type")).iter().map(|(_, v)| strval(Some(v))).collect();
        out.push(Registration { name, taxonomy: true, args: taxonomy_args(&post), object_types });
    }
    out
}

fn is_builtin_post_type(name: &str) -> bool {
    matches!(name, "post" | "page" | "attachment" | "revision" | "nav_menu_item" | "custom_css" | "customize_changeset" | "oembed_cache" | "user_request" | "acf-field-group" | "acf-field" | "acf-post-type" | "acf-taxonomy" | "acf-ui-options-page")
        || name.starts_with("wp_")
}

fn is_builtin_taxonomy(name: &str) -> bool {
    matches!(name, "category" | "post_tag" | "nav_menu" | "link_category" | "post_format" | "wp_theme" | "wp_template_part_area" | "wp_pattern_category")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(content: &str) -> AcfPost {
        AcfPost { id: 7, title: "Books".into(), name: "post_type_x".into(), status: "publish".into(), content: content.into(), ..Default::default() }
    }

    #[test]
    fn post_type_with_custom_slug_and_archive() {
        let stored = r#"a:6:{s:9:"post_type";s:7:"rp_book";s:6:"labels";a:2:{s:4:"name";s:5:"Books";s:9:"menu_name";s:12:"Books & More";}s:11:"has_archive";i:1;s:7:"rewrite";a:3:{s:17:"permalink_rewrite";s:16:"custom_permalink";s:4:"slug";s:7:"buecher";s:5:"feeds";i:1;}s:9:"rest_base";s:5:"books";s:12:"hierarchical";s:1:"1";}"#;
        let regs = registrations(&[post(stored)], &[]);
        let args = &regs[0].args;
        assert_eq!(regs[0].name, "rp_book");
        assert_eq!(args.get("labels").and_then(|l| l.get("menu_name")).and_then(Lit::as_str), Some("Books &amp; More"));
        assert_eq!(args.get("has_archive"), Some(&Lit::Bool(true)));
        assert_eq!(args.get("rewrite").and_then(|r| r.get("slug")).and_then(Lit::as_str), Some("buecher"));
        assert_eq!(args.get("rewrite").and_then(|r| r.get("feeds")), None);
        assert_eq!(args.get("rest_base").and_then(Lit::as_str), Some("books"));
        assert_eq!(args.get("hierarchical"), Some(&Lit::Bool(true)));
        assert_eq!(args.get("supports").map(Lit::strings), Some(vec!["title".to_owned(), "editor".to_owned(), "thumbnail".to_owned(), "custom-fields".to_owned()]));
    }

    #[test]
    fn disabled_types_are_not_registered() {
        let mut p = post(r#"a:1:{s:9:"post_type";s:3:"off";}"#);
        p.status = "acf-disabled".into();
        assert!(registrations(&[p], &[]).is_empty());
    }
}
