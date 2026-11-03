//! Saving a field group from its editor: `ACF_Admin_Field_Group::save_post()` with `acf_update_field()`,
//! `acf_delete_field()` and `acf_update_field_group()` (`ACF_Internal_Post_Type::update_post()`), and the messages of
//! `post_updated_messages()`. The posts are written as `wp_insert_post()`/`wp_update_post()` write them.

use rustpress_core::native::PostSaved;
use rustpress_core::{Result, Site};
use rustpress_db::Post;
use rustpress_phpser::{Key, Value};

use super::php::{key, string, strval, truthy};
use super::render::RenderField;

/// The services of a write (`wp_insert_post()` runs as the signed-in user).
struct Writer<'a> {
    content: rustpress_content::content::Content<'a>,
    user: u64,
}

/// A failed `wp_insert_post()`: a database error stops the save, a `WP_Error` (which ACF ignores) is logged.
fn content_result<T: Default>(result: std::result::Result<T, rustpress_content::content::ContentError>) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(rustpress_content::content::ContentError::Db(e)) => Err(e.into()),
        Err(rustpress_content::content::ContentError::Message(message)) => {
            rustpress_core::OrLog::or_log(Err::<(), _>(format!("ACF could not write a post: {message}")));
            Ok(T::default())
        }
    }
}

/// `acf_parse_type()` of the top-level values: strings are trimmed, integer strings become integers.
fn parse_types(items: Vec<(Key, Value)>) -> Vec<(Key, Value)> {
    items
        .into_iter()
        .map(|(k, v)| {
            let v = match v {
                Value::Str(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let trimmed = text.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b']);
                    match trimmed.parse::<i64>() {
                        Ok(i) if i.to_string() == trimmed => Value::Int(i),
                        _ => string(trimmed),
                    }
                }
                other => other,
            };
            (k, v)
        })
        .collect()
}

/// `array_filter()` then `array_values()` of a list of lists, the inner lists the same (empty values go).
fn clean_groups(value: &Value) -> Value {
    let Value::Array(groups) = value else { return value.clone() };
    let list = |items: Vec<Value>| Value::Array(items.into_iter().enumerate().map(|(i, v)| (Key::Int(i as i64), v)).collect());
    list(
        groups
            .iter()
            .filter(|(_, g)| truthy(Some(g)))
            .map(|(_, g)| match g {
                Value::Array(rules) => list(rules.iter().filter(|(_, r)| truthy(Some(r))).map(|(_, r)| r.clone()).collect()),
                other => other.clone(),
            })
            .collect(),
    )
}

fn get<'v>(items: &'v [(Key, Value)], name: &str) -> Option<&'v Value> {
    let k = key(name);
    items.iter().find(|(existing, _)| super::php::key_eq(existing, &k)).map(|(_, v)| v)
}

fn remove(items: &mut Vec<(Key, Value)>, name: &str) -> Option<Value> {
    let k = key(name);
    let at = items.iter().position(|(existing, _)| super::php::key_eq(existing, &k))?;
    Some(items.remove(at).1)
}

/// `acf_decode_choices( $string, $array_keys )`.
fn decode_choices(value: Option<&Value>, array_keys: bool) -> Value {
    let text = match value {
        Some(Value::Array(_)) => return value.cloned().unwrap_or(Value::Null),
        Some(v @ (Value::Int(_) | Value::Float(_))) => strval(Some(v)),
        Some(Value::Str(bytes)) if !bytes.is_empty() => String::from_utf8_lossy(bytes).into_owned(),
        _ => return Value::Array(Vec::new()),
    };
    let trim = |s: &str| s.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b']).to_owned();
    let mut items: Vec<(Key, Value)> = Vec::new();
    for line in text.split('\n') {
        let (k, v) = match line.split_once(" : ") {
            Some((k, v)) => (trim(k), trim(v.split(" : ").next().unwrap_or(v))),
            None => (trim(line), trim(line)),
        };
        // PHP array keys: integer strings become integers.
        let k = match k.parse::<i64>() {
            Ok(i) if i.to_string() == k => Key::Int(i),
            _ => key(&k),
        };
        super::php::set_key(&mut items, k, string(&v));
    }
    if array_keys {
        return Value::Array(items.into_iter().enumerate().map(|(i, (k, _))| (Key::Int(i as i64), match k { Key::Int(n) => Value::Int(n), Key::Str(s) => Value::Str(s) })).collect());
    }
    Value::Array(items)
}

/// The types' `update_field()`: the choices of select, checkbox, radio and button group back to arrays.
fn type_update_field(items: &mut Vec<(Key, Value)>) {
    let kind = strval(get(items, "type"));
    match kind.as_str() {
        "select" => {
            let choices = decode_choices(get(items, "choices"), false);
            super::php::set(items, "choices", choices);
            let mut defaults = decode_choices(get(items, "default_value"), true);
            if !truthy(get(items, "multiple")) {
                // `acf_unarray()`.
                defaults = match defaults {
                    Value::Array(list) => list.into_iter().next().map(|(_, v)| v).unwrap_or(Value::Bool(false)),
                    other => other,
                };
            }
            super::php::set(items, "default_value", defaults);
        }
        "checkbox" => {
            let choices = decode_choices(get(items, "choices"), false);
            super::php::set(items, "choices", choices);
            let defaults = decode_choices(get(items, "default_value"), true);
            super::php::set(items, "default_value", defaults);
        }
        "radio" | "button_group" => {
            let choices = decode_choices(get(items, "choices"), false);
            super::php::set(items, "choices", choices);
        }
        _ => {}
    }
}

/// `acf_update_field( $field, $specific )`.
async fn update_field(w: &Writer<'_>, posted: Vec<(Key, Value)>, specific: bool) -> Result<u64> {
    // `acf_validate_field()` leaves a field that is already valid alone.
    let mut field = if truthy(get(&posted, "_valid")) { RenderField { items: posted, sub_fields: Vec::new() } } else { RenderField::raw(posted, Vec::new()) };
    // The translated default of checkboxes (`acf_field_checkbox::initialize()`).
    if strval(field.get("type")) == "checkbox" && field.get("custom_choice_button_text").is_none_or(|v| matches!(v, Value::Null)) {
        field.set("custom_choice_button_text", string(&w.content.site.trd("acf", "Add new choice")));
    }
    let mut items = parse_types(field.items);
    if truthy(get(&items, "conditional_logic")) {
        let cleaned = clean_groups(get(&items, "conditional_logic").unwrap_or(&Value::Null));
        super::php::set(&mut items, "conditional_logic", cleaned);
    }
    // A parent given as a field key.
    let parent = get(&items, "parent").cloned().unwrap_or(Value::Null);
    if truthy(Some(&parent)) && !super::php::is_numeric(&parent) {
        let id = w.content.db.acf_fields().await?.into_iter().find(|f| f.name == strval(Some(&parent))).map_or(0, |f| f.id);
        super::php::set(&mut items, "parent", Value::Int(id as i64));
    }
    type_update_field(&mut items);
    let mut stored = items.clone();
    for name in ["ID", "key", "label", "name", "prefix", "value", "menu_order", "id", "class", "parent", "_name", "_prepare", "_valid"] {
        remove(&mut stored, name);
    }
    let id = super::php::intval(get(&items, "ID")).max(0) as u64;
    let parent = super::php::intval(get(&items, "parent")).max(0) as u64;
    let menu_order = super::php::intval(get(&items, "menu_order")) as i32;
    let data = if specific {
        rustpress_content::content::PostData { id: Some(id), post_parent: Some(parent), menu_order: Some(menu_order), ..Default::default() }
    } else {
        rustpress_content::content::PostData {
            id: (id != 0).then_some(id),
            post_author: (id == 0).then_some(w.user),
            post_status: Some("publish".into()),
            post_type: Some("acf-field".into()),
            post_title: Some(strval(get(&items, "label"))),
            post_name: Some(strval(get(&items, "key"))),
            post_excerpt: Some(strval(get(&items, "name"))),
            post_content: Some(String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(stored))).into_owned()),
            post_parent: Some(parent),
            menu_order: Some(menu_order),
            ..Default::default()
        }
    };
    content_result(w.content.insert_post(&data).await)
}

/// `acf_delete_field( $id )`: the field and, for a group, its sub fields.
fn delete_field<'a>(w: &'a Writer<'a>, id: u64) -> futures::future::BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        let fields = w.content.db.acf_fields().await?;
        let Some(field) = fields.iter().find(|f| f.id == id) else { return Ok(()) };
        content_result(w.content.delete_post(field.id, true).await)?;
        let kind = super::php::strval(rustpress_core::options::maybe_unserialize(&field.content).get("type"));
        if kind == "group" {
            for sub in fields.iter().filter(|f| f.parent == id) {
                delete_field(w, sub.id).await?;
            }
        }
        Ok(())
    })
}

/// `ACF_Field_Group::get_settings_array()`.
pub(super) fn group_defaults() -> Vec<(Key, Value)> {
    [
        ("ID", Value::Int(0)),
        ("key", string("")),
        ("title", string("")),
        ("fields", Value::Array(Vec::new())),
        ("location", Value::Array(Vec::new())),
        ("menu_order", Value::Int(0)),
        ("position", string("normal")),
        ("style", string("default")),
        ("label_placement", string("top")),
        ("instruction_placement", string("label")),
        ("hide_on_screen", Value::Array(Vec::new())),
        ("active", Value::Bool(true)),
        ("description", string("")),
        ("show_in_rest", Value::Bool(false)),
        ("display_title", string("")),
        ("allow_ai_access", Value::Bool(false)),
        ("ai_description", string("")),
    ]
    .into_iter()
    .map(|(k, v)| (key(k), v))
    .collect()
}

/// `validate_post()` of a field group, post type or taxonomy that is not `_valid` yet: the defaults of the type, `ID`
/// and `menu_order` as integers, and `active` as a boolean (`ACF_Internal_Post_Type`) or – for post types – every
/// boolean setting posted as `'0'`, `'1'`, `'false'` or `'true'` as a boolean (`ACF_Post_Type::validate_post()`).
fn validate_internal(post_type: &str, posted: Vec<(Key, Value)>) -> Vec<(Key, Value)> {
    let defaults = match post_type {
        "acf-post-type" => match super::internal::post_type_defaults() {
            Value::Array(items) => items,
            _ => Vec::new(),
        },
        "acf-taxonomy" => match super::internal::taxonomy_defaults() {
            Value::Array(items) => items,
            _ => Vec::new(),
        },
        _ => group_defaults(),
    };
    let mut items = defaults.clone();
    for (k, v) in posted {
        super::php::set_key(&mut items, k, v);
    }
    let id = super::php::intval(get(&items, "ID"));
    super::php::set(&mut items, "ID", Value::Int(id));
    let menu_order = super::php::intval(get(&items, "menu_order"));
    super::php::set(&mut items, "menu_order", Value::Int(menu_order));
    if post_type == "acf-post-type" {
        for (k, v) in items.iter_mut() {
            if !matches!(defaults.iter().find(|(d, _)| super::php::key_eq(d, k)), Some((_, Value::Bool(_)))) {
                continue;
            }
            if let Value::Str(s) = v {
                match s.as_slice() {
                    b"0" | b"1" => *v = Value::Bool(s.as_slice() == b"1"),
                    b"false" | b"true" => *v = Value::Bool(s.as_slice() == b"true"),
                    _ => {}
                }
            }
        }
    } else {
        let active = truthy(get(&items, "active"));
        super::php::set(&mut items, "active", Value::Bool(active));
    }
    super::php::set(&mut items, "_valid", Value::Bool(true));
    items
}

/// `ACF_Internal_Post_Type::update_post( $post )` of a field group, post type or taxonomy: `validate_post()`,
/// `acf_parse_types()`, `pre_update_field_group()`, the post and `flush_post_cache()` – a post type with a key or a
/// taxonomy registers again, so the rewrite rules of the site under `wp_root` are flushed.
pub(super) async fn update_internal_post(content: &rustpress_content::content::Content<'_>, wp_root: Option<&std::path::Path>, post_type: &str, posted: Vec<(Key, Value)>) -> Result<()> {
    update_internal_post_as(content, wp_root, post_type, posted, 0).await.map(|_| ())
}

/// [`update_internal_post`] that inserts an item without `ID` as the post of `user` (`wp_insert_post()`); the ID of
/// the post.
pub(super) async fn update_internal_post_as(content: &rustpress_content::content::Content<'_>, wp_root: Option<&std::path::Path>, post_type: &str, posted: Vec<(Key, Value)>, user: u64) -> Result<u64> {
    let items = if truthy(get(&posted, "_valid")) { posted } else { validate_internal(post_type, posted) };
    let mut items = parse_types(items);
    if post_type == "acf-field-group" && truthy(get(&items, "location")) {
        let cleaned = clean_groups(get(&items, "location").unwrap_or(&Value::Null));
        super::php::set(&mut items, "location", cleaned);
    }
    let mut stored = items.clone();
    for name in ["ID", "key", "title", "menu_order", "fields", "active", "_valid", "_parent"] {
        remove(&mut stored, name);
    }
    let title = strval(get(&items, "title"));
    let id = super::php::intval(get(&items, "ID"));
    let data = rustpress_content::content::PostData {
        id: (id > 0).then_some(id as u64),
        post_author: (id <= 0).then_some(user),
        post_status: Some(if truthy(get(&items, "active")) { "publish" } else { "acf-disabled" }.into()),
        post_type: Some(post_type.to_owned()),
        post_title: Some(title.clone()),
        post_name: Some(strval(get(&items, "key"))),
        post_excerpt: Some(rustpress_core::slug::sanitize_title(&title)),
        post_content: Some(String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(stored))).into_owned()),
        menu_order: Some(super::php::intval(get(&items, "menu_order")) as i32),
        comment_status: Some("closed".into()),
        ping_status: Some("closed".into()),
        post_parent: Some(super::php::intval(get(&items, "_parent")).max(0) as u64),
        ..Default::default()
    };
    let saved = content_result(content.insert_post(&data).await)?;
    let registers = match post_type {
        "acf-post-type" => truthy(get(&items, "post_type")),
        "acf-taxonomy" => get(&items, "taxonomy").is_some_and(|v| !matches!(v, Value::Null)),
        _ => false,
    };
    if let Some(wp_root) = wp_root.filter(|_| registers) {
        // The type is unregistered and, when active, registered again.
        let taxonomy = post_type == "acf-taxonomy";
        let name = strval(get(&items, if taxonomy { "taxonomy" } else { "post_type" }));
        let change = rustpress_core::rewrite_gen::Reregistered { taxonomy, name: &name, active: truthy(get(&items, "active")) };
        rustpress_core::rewrite_gen::flush_rewrite_rules(content.site, content.writer, wp_root, change).await?;
    }
    Ok(if id > 0 { id as u64 } else { saved })
}

/// `ACF_Admin_Field_Group::save_post()` (after `verify_save_post()`).
pub async fn save_post(ctx: &PostSaved<'_>) -> Result<()> {
    let post: &Post = ctx.post;
    let actx = ctx.ctx;
    if post.post_type != "acf-field-group" {
        return Ok(());
    }
    // `acf_verify_nonce( 'field_group' )`.
    if !actx.post_field("_acf_nonce").is_some_and(|nonce| !nonce.is_empty() && (actx.verify_nonce)(nonce, "field_group")) {
        return Ok(());
    }
    if !(actx.can)("manage_options") {
        return Ok(());
    }
    let system = rustpress_auth::CurrentUser::system();
    let w = Writer { content: rustpress_content::content::Content { site: actx.site, db: &actx.site.db, writer: ctx.writer, user: &system, now: actx.now }, user: actx.user };
    if let Some(Value::Array(fields)) = rustpress_core::native::form_value(actx.form, "acf_fields") {
        for (_, field) in fields {
            let Value::Array(mut items) = field else { continue };
            if get(&items, "key").is_none() {
                continue;
            }
            let save = remove(&mut items, "save").unwrap_or(Value::Null);
            let specific = strval(Some(&save)) == "meta";
            if !truthy(get(&items, "parent")) {
                super::php::set(&mut items, "parent", Value::Int(post.id as i64));
            }
            update_field(&w, items, specific).await?;
        }
    }
    if let Some(ids) = actx.post_field("_acf_delete_fields").filter(|v| truthy(Some(&string(v)))) {
        for id in ids.split('|').map(|s| super::php::intval(Some(&string(s)))) {
            if id > 0 {
                delete_field(&w, id as u64).await?;
            }
        }
    }
    let mut group = match rustpress_core::native::form_value(actx.form, "acf_field_group") {
        Some(Value::Array(items)) => items,
        _ => Vec::new(),
    };
    super::php::set(&mut group, "ID", Value::Int(post.id as i64));
    super::php::set(&mut group, "title", string(actx.post_field("post_title").unwrap_or_default()));
    update_internal_post(&w.content, None, "acf-field-group", group).await
}

/// `uniqid( $prefix )` after `usleep( 1 )`: keys made one after the other differ.
fn unique_key(prefix: &str) -> String {
    std::thread::sleep(std::time::Duration::from_micros(1));
    super::render::uniqid(prefix)
}

/// `acf_duplicate_fields( $fields, $parent_id )`: new keys first (conditional logic refers to them), then each field.
fn duplicate_fields<'a>(w: &'a Writer<'a>, fields: Vec<RenderField>, parent: u64) -> futures::future::BoxFuture<'a, Result<()>> {
    Box::pin(async move {
        let keys: Vec<(String, String)> = fields.iter().map(|f| (strval(f.get("key")), unique_key("field_"))).collect();
        for field in fields {
            duplicate_field(w, field, parent, &keys).await?;
        }
        Ok(())
    })
}

/// `acf_duplicate_field( $id, $parent_id )`; a group saves itself first and duplicates its sub fields below it
/// (`acf_field_group::duplicate_field()`).
async fn duplicate_field(w: &Writer<'_>, mut field: RenderField, parent: u64, keys: &[(String, String)]) -> Result<u64> {
    field.set("ID", Value::Int(0));
    let old_key = strval(field.get("key"));
    let new_key = keys.iter().find(|(k, _)| *k == old_key).map_or_else(|| unique_key("field_"), |(_, v)| v.clone());
    field.set("key", string(&new_key));
    if parent != 0 {
        field.set("parent", Value::Int(parent as i64));
    }
    if let Some(Value::Array(groups)) = field.get("conditional_logic").cloned() {
        let groups = groups
            .into_iter()
            .map(|(gk, group)| match group {
                Value::Array(rules) => (
                    gk,
                    Value::Array(
                        rules
                            .into_iter()
                            .map(|(rk, rule)| match rule {
                                Value::Array(mut items) => {
                                    let target = strval(get(&items, "field"));
                                    if let Some((_, new)) = keys.iter().find(|(k, _)| *k == target) {
                                        super::php::set(&mut items, "field", string(new));
                                    }
                                    (rk, Value::Array(items))
                                }
                                other => (rk, other),
                            })
                            .collect(),
                    ),
                ),
                other => (gk, other),
            })
            .collect();
        field.set("conditional_logic", Value::Array(groups));
    }
    let sub_fields = std::mem::take(&mut field.sub_fields);
    if strval(field.get("type")) == "group" {
        // `acf_extract_var( $field, 'sub_fields' )`.
        remove(&mut field.items, "sub_fields");
        // `acf_update_field()` unslashes the loaded field; the group is saved with what the first save returned.
        field.items = unslash_items(field.items);
        let id = update_field(w, field.items.clone(), false).await?;
        field.set("ID", Value::Int(id as i64));
        let render = sub_fields_render(w, &sub_fields);
        duplicate_fields(w, render, id).await?;
    }
    update_field(w, unslash_items(field.items), false).await
}

/// [`super::php::wp_unslash`] of a loaded array.
fn unslash_items(items: Vec<(Key, Value)>) -> Vec<(Key, Value)> {
    match super::php::wp_unslash(Value::Array(items)) {
        Value::Array(items) => items,
        _ => Vec::new(),
    }
}

/// The sub fields of a group as `acf_get_fields()` loads them.
fn sub_fields_render(w: &Writer<'_>, fields: &[super::fields::Field]) -> Vec<RenderField> {
    fields.iter().map(|f| loaded(w.content.site, f)).collect()
}

/// `acf_prepare_fields_for_import()`: each field, a group followed by its sub fields with the group's key as their
/// parent and their place as their order (`acf_field_group::prepare_field_for_import()`).
fn flatten_for_import(mut field: Vec<(Key, Value)>, out: &mut Vec<Vec<(Key, Value)>>) {
    let subs = if strval(get(&field, "type")) == "group" && truthy(get(&field, "sub_fields")) { remove(&mut field, "sub_fields") } else { None };
    let field_key = get(&field, "key").cloned().unwrap_or(Value::Null);
    out.push(field);
    if let Some(Value::Array(subs)) = subs {
        for (i, (_, sub)) in subs.into_iter().enumerate() {
            let Value::Array(mut sub) = sub else { continue };
            super::php::set(&mut sub, "parent", field_key.clone());
            super::php::set(&mut sub, "menu_order", Value::Int(i as i64));
            flatten_for_import(sub, out);
        }
    }
}

/// The fields below `parent` in the order `acf_get_fields()` and `acf_prepare_fields_for_import()` give them:
/// `(key, ID)`.
fn stored_fields_below(parent: u64, all: &[rustpress_db::AcfPost], out: &mut Vec<(String, u64)>) {
    for field in all.iter().filter(|f| f.parent == parent) {
        out.push((field.name.clone(), field.id));
        stored_fields_below(field.id, all, out);
    }
}

/// `acf_import_internal_post_type( $post, $post_type )` (`import_post()` of `ACF_Field_Group` and
/// `ACF_Internal_Post_Type`): the item over the defaults; a field group's fields saved first – stored fields the
/// import no longer has are deleted, a new group gets its post before –, then the item. The ID of its post.
/// A post type or taxonomy it registers again is added to `reregistered` (`(taxonomy, key, active)`) for the flush
/// of the rewrite rules after the import.
pub(super) async fn import_item(ctx: &rustpress_core::native::AdminContext<'_>, post_type: &str, item: Vec<(Key, Value)>, existing: Option<u64>, reregistered: &mut Vec<(bool, String, bool)>) -> Result<u64> {
    let Some(writer) = ctx.writer else { return Ok(0) };
    let site = ctx.site;
    let system = rustpress_auth::CurrentUser::system();
    let w = Writer { content: rustpress_content::content::Content { site, db: &site.db, writer, user: &system, now: ctx.now }, user: ctx.user };
    let mut item = item;
    if let Some(id) = existing {
        super::php::set(&mut item, "ID", Value::Int(id as i64));
    }
    // `get_valid_post()`.
    let mut items = if truthy(get(&item, "_valid")) { item } else { validate_internal(post_type, item) };
    if post_type == "acf-field-group" {
        let group_key = strval(get(&items, "key"));
        // `prepare_post_for_import()`.
        let listed: Vec<Vec<(Key, Value)>> = match get(&items, "fields") {
            Some(Value::Array(list)) => list.iter().filter_map(|(_, v)| if let Value::Array(f) = v { Some(f.clone()) } else { None }).collect(),
            _ => Vec::new(),
        };
        let mut flat = Vec::new();
        for (i, mut field) in listed.into_iter().enumerate() {
            super::php::set(&mut field, "parent", string(&group_key));
            super::php::set(&mut field, "menu_order", Value::Int(i as i64));
            flatten_for_import(field, &mut flat);
        }
        // `$ids`: key => ID.
        let mut ids: Vec<(String, u64)> = Vec::new();
        let remember = |ids: &mut Vec<(String, u64)>, key: String, id: u64| match ids.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = id,
            None => ids.push((key, id)),
        };
        let mut group_id = super::php::intval(get(&items, "ID")).max(0) as u64;
        if group_id > 0 {
            let all = site.db.acf_fields().await?;
            let mut stored = Vec::new();
            stored_fields_below(group_id, &all, &mut stored);
            let keys: Vec<String> = flat.iter().map(|f| strval(get(f, "key"))).collect();
            for (key, id) in stored {
                remember(&mut ids, key.clone(), id);
                if !keys.contains(&key) {
                    delete_field(&w, id).await?;
                }
            }
        } else {
            group_id = content_result(w.content.insert_post(&rustpress_content::content::PostData { post_title: Some(group_key.clone()), post_type: Some("acf-field-group".into()), post_author: Some(ctx.user), ..Default::default() }).await)?;
            super::php::set(&mut items, "ID", Value::Int(group_id as i64));
        }
        remember(&mut ids, group_key, group_id);
        for mut field in flat {
            let field_key = strval(get(&field, "key"));
            if let Some((_, id)) = ids.iter().find(|(k, _)| *k == field_key) {
                super::php::set(&mut field, "ID", Value::Int(*id as i64));
            }
            let parent = strval(get(&field, "parent"));
            if let Some((_, id)) = ids.iter().find(|(k, _)| *k == parent) {
                super::php::set(&mut field, "parent", Value::Int(*id as i64));
            }
            let saved = update_field(&w, unslash_items(field), false).await?;
            remember(&mut ids, field_key, saved);
        }
    }
    // `flush_post_cache()` of a post type with a key or a taxonomy.
    let taxonomy = post_type == "acf-taxonomy";
    let registers = match post_type {
        "acf-post-type" => truthy(get(&items, "post_type")),
        "acf-taxonomy" => get(&items, "taxonomy").is_some_and(|v| !matches!(v, Value::Null)),
        _ => false,
    };
    if registers {
        reregistered.push((taxonomy, strval(get(&items, if taxonomy { "taxonomy" } else { "post_type" })), truthy(get(&items, "active"))));
    }
    update_internal_post_as(&w.content, None, post_type, unslash_items(items), ctx.user).await
}

/// The move of `ACF_Admin_Field_Group::ajax_move_field()`: the field (as `acf_get_field()` loads it) gets the new
/// group as its parent and loses its conditional logic. The field's label, or `None` without the field.
pub(super) async fn move_field(ctx: &rustpress_core::native::AdminContext<'_>, field_id: u64, group_id: Option<u64>) -> Result<Option<String>> {
    let Some(writer) = ctx.writer else { return Ok(None) };
    let site = ctx.site;
    let all = site.db.acf_fields().await?;
    let Some((parent, field_key)) = all.iter().find(|f| f.id == field_id).map(|f| (f.parent, f.name.clone())) else { return Ok(None) };
    let Some(field) = super::fields::children_of(parent, &all).into_iter().find(|f| f.key == field_key) else { return Ok(None) };
    let mut field = loaded(site, &field);
    // `$new_field_group['ID']` of a group that does not exist is `null`.
    field.set("parent", group_id.map_or(Value::Null, |id| Value::Int(id as i64)));
    field.set("conditional_logic", Value::Int(0));
    let label = strval(field.get("label"));
    let system = rustpress_auth::CurrentUser::system();
    let w = Writer { content: rustpress_content::content::Content { site, db: &site.db, writer, user: &system, now: ctx.now }, user: ctx.user };
    update_field(&w, unslash_items(field.items), false).await?;
    Ok(Some(label))
}

/// `acf_get_field()`: validated, with the prefix `acf` and the type's `load_field()`.
pub(super) fn loaded(site: &Site, field: &super::fields::Field) -> RenderField {
    let services = super::edit_comment::AdminServices(std::path::PathBuf::new());
    let can = |_: &str| true;
    let nonce = |_: &str| String::new();
    let render = super::render::Ctx { site, nonce: &nonce, services: &services, admin: true, uploader: "wp", wants_uploader: Default::default(), can: &can, rich_edit: true, default_editor: "tinymce", wysiwyg_height: 300 };
    RenderField::load(field, &render)
}

/// `acf_duplicate_field_group( $id )` (`ACF_Field_Group::duplicate_post()`): a temporary post for the new ID, the
/// fields below it with new keys, then the group with a new key and " (copy)" in its title.
pub(super) async fn duplicate_field_group(ctx: &rustpress_core::native::AdminContext<'_>, post: &Post) -> Result<Option<u64>> {
    let Some(writer) = ctx.writer else { return Ok(None) };
    let site = ctx.site;
    let system = rustpress_auth::CurrentUser::system();
    let w = Writer { content: rustpress_content::content::Content { site, db: &site.db, writer, user: &system, now: ctx.now }, user: ctx.user };
    let acf_post = rustpress_db::AcfPost { id: post.id, parent: post.post_parent, title: post.post_title.clone(), name: post.post_name.clone(), excerpt: post.post_excerpt.clone(), menu_order: i64::from(post.menu_order), status: post.post_status.clone(), content: post.post_content.clone() };
    let Value::Array(mut items) = super::internal::validated(&acf_post, Value::Array(group_defaults())) else { return Ok(None) };
    let all = site.db.acf_fields().await?;
    let fields: Vec<RenderField> = super::fields::children_of(post.id, &all).iter().map(|f| loaded(site, f)).collect();
    let key_value = unique_key("group_");
    super::php::set(&mut items, "key", string(&key_value));
    let title = format!("{} ({})", strval(get(&items, "title")), site.trd("acf", "copy"));
    super::php::set(&mut items, "title", string(&title));
    let id = content_result(w.content.insert_post(&rustpress_content::content::PostData { post_title: Some(key_value), post_type: Some("acf-field-group".into()), post_author: Some(ctx.user), ..Default::default() }).await)?;
    if id == 0 {
        return Ok(None);
    }
    super::php::set(&mut items, "ID", Value::Int(id as i64));
    duplicate_fields(&w, fields, id).await?;
    update_internal_post(&w.content, None, "acf-field-group", unslash_items(items)).await?;
    Ok(Some(id))
}

/// `ACF_Internal_Post_Type::untrash_post_status()` and `_acf_untrash_field_post_status()`: ACF's posts and fields
/// come back with their status.
pub fn untrash_post_status(post: &Post, previous: &str) -> Option<String> {
    matches!(post.post_type.as_str(), "acf-field-group" | "acf-post-type" | "acf-taxonomy" | "acf-ui-options-page" | "acf-field").then(|| previous.to_owned())
}

/// `trashed_post`, `untrashed_post` and `deleted_post` of the field group list: the group's fields go to the trash,
/// come back or are deleted with it (`ACF_Field_Group::trash_post()`, `untrash_post()`, `delete_post()`).
pub async fn post_trash_change(change: &rustpress_core::native::PostChange<'_>) -> Result<()> {
    if change.post.post_type != "acf-field-group" {
        return Ok(());
    }
    let system = rustpress_auth::CurrentUser::system();
    let w = Writer { content: rustpress_content::content::Content { site: change.site, db: &change.site.db, writer: change.writer, user: &system, now: change.now }, user: change.user };
    let fields: Vec<u64> = change.site.db.acf_fields().await?.into_iter().filter(|f| f.parent == change.post.id).map(|f| f.id).collect();
    for id in fields {
        match change.kind {
            rustpress_core::native::PostChangeKind::Trashed => {
                content_result(w.content.trash_post(id).await)?;
            }
            rustpress_core::native::PostChangeKind::Untrashed => {
                content_result(w.content.untrash_post(id).await)?;
            }
            rustpress_core::native::PostChangeKind::Deleted => delete_field(&w, id).await?,
        }
    }
    Ok(())
}

/// `ACF_Admin_Field_Group::post_updated_messages()`.
pub fn post_updated_message(site: &Site, post_type: &str, number: u32) -> Option<Option<String>> {
    if post_type != "acf-field-group" {
        return None;
    }
    let tr = |text: &str| Some(site.trd("acf", text));
    Some(match number {
        1 | 2 | 4 => tr("Field group updated."),
        3 => tr("Field group deleted."),
        6 => tr("Field group published."),
        7 => tr("Field group saved."),
        8 => tr("Field group submitted."),
        9 => tr("Field group scheduled for."),
        10 => tr("Field group draft updated."),
        _ => None,
    })
}
