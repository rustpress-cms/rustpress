//! Field groups and fields as ACF keeps them in the database (`acf-field-group` and `acf-field` posts), and the
//! location rules that decide where a group shows (`acf_get_field_group_visibility()`).

use rustpress_core::Result;
use rustpress_db::{AcfPost, Db, MetaKind, Post};
use rustpress_phpser::Value;

use super::php::{loose_eq, string, strval, truthy};

#[derive(Debug, Clone)]
pub struct Field {
    pub key: String,
    /// `name` (`_name` before a group prefixes it).
    pub name: String,
    pub kind: String,
    /// The validated settings (`acf_validate_field()` with the defaults of the type).
    pub settings: Value,
    /// `sub_fields` of a group.
    pub sub_fields: Vec<Field>,
}

impl Field {
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.settings.get(name)
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub param: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub struct FieldGroup {
    pub id: u64,
    pub key: String,
    pub title: String,
    /// The stored settings (`label_placement`, `instruction_placement`, ...).
    pub settings: Value,
    pub active: bool,
    pub show_in_rest: bool,
    /// Groups of rules: a group matches when all its rules do, the field group when one group does.
    pub location: Vec<Vec<Rule>>,
    pub fields: Vec<Field>,
}

/// The defaults of a field type (`acf_field::$defaults`), which `acf_field::validate_field()` adds for the keys a field
/// does not set. `custom_choice_button_text` of checkboxes is translated, so the renderer adds it.
pub(super) fn type_defaults(kind: &str) -> Vec<(&'static str, Value)> {
    let empty = || Value::Array(Vec::new());
    let s = string;
    let i = Value::Int;
    match kind {
        "accordion" => vec![("open", i(0)), ("multi_expand", i(0)), ("endpoint", i(0))],
        "button_group" => vec![("choices", empty()), ("default_value", s("")), ("allow_null", i(0)), ("return_format", s("value")), ("layout", s("horizontal"))],
        "checkbox" => vec![("layout", s("vertical")), ("choices", empty()), ("default_value", s("")), ("allow_custom", i(0)), ("save_custom", i(0)), ("toggle", i(0)), ("return_format", s("value"))],
        "color_picker" => vec![("default_value", s("")), ("enable_opacity", Value::Bool(false)), ("custom_palette_source", s("")), ("palette_colors", s("")), ("show_color_wheel", Value::Bool(true)), ("return_format", s("string"))],
        "date_picker" => vec![("display_format", s("d/m/Y")), ("return_format", s("d/m/Y")), ("first_day", i(1)), ("default_to_current_date", i(0))],
        "date_time_picker" => vec![("display_format", s("d/m/Y g:i a")), ("return_format", s("d/m/Y g:i a")), ("first_day", i(1)), ("default_to_current_date", i(0))],
        "email" => vec![("default_value", s("")), ("placeholder", s("")), ("prepend", s("")), ("append", s(""))],
        "file" => vec![("return_format", s("array")), ("library", s("all")), ("min_size", i(0)), ("max_size", i(0)), ("mime_types", s(""))],
        "google_map" => vec![("height", s("")), ("center_lat", s("")), ("center_lng", s("")), ("zoom", s(""))],
        "group" => vec![("sub_fields", empty()), ("layout", s("block"))],
        "icon_picker" => vec![
            ("library", s("all")),
            ("tabs", Value::Array(vec![(rustpress_phpser::Key::Int(0), s("dashicons")), (rustpress_phpser::Key::Int(1), s("url")), (rustpress_phpser::Key::Int(2), s("media_library"))])),
            ("return_format", s("string")),
            ("default_value", Value::Array(vec![(super::php::key("type"), Value::Null), (super::php::key("value"), Value::Null)])),
        ],
        "image" => vec![
            ("return_format", s("array")),
            ("preview_size", s("medium")),
            ("library", s("all")),
            ("min_width", i(0)),
            ("min_height", i(0)),
            ("min_size", i(0)),
            ("max_width", i(0)),
            ("max_height", i(0)),
            ("max_size", i(0)),
            ("mime_types", s("")),
        ],
        "link" => vec![("return_format", s("array"))],
        "message" => vec![("message", s("")), ("esc_html", i(0)), ("new_lines", s("wpautop"))],
        "number" => vec![("default_value", s("")), ("min", s("")), ("max", s("")), ("step", s("")), ("placeholder", s("")), ("prepend", s("")), ("append", s(""))],
        "oembed" => vec![("width", s("")), ("height", s(""))],
        "page_link" => vec![("post_type", empty()), ("taxonomy", empty()), ("allow_null", i(0)), ("multiple", i(0)), ("allow_archives", i(1))],
        "password" => vec![("placeholder", s("")), ("prepend", s("")), ("append", s(""))],
        "post_object" => vec![("post_type", empty()), ("taxonomy", empty()), ("allow_null", i(0)), ("multiple", i(0)), ("return_format", s("object")), ("ui", i(1)), ("bidirectional_target", empty())],
        "radio" => vec![("layout", s("vertical")), ("choices", empty()), ("default_value", s("")), ("other_choice", i(0)), ("save_other_choice", i(0)), ("allow_null", i(0)), ("return_format", s("value"))],
        "range" => vec![("default_value", s("")), ("min", s("")), ("max", s("")), ("step", s("")), ("prepend", s("")), ("append", s(""))],
        "relationship" => vec![
            ("post_type", empty()),
            ("taxonomy", empty()),
            ("min", i(0)),
            ("max", i(0)),
            ("filters", Value::Array(vec![(rustpress_phpser::Key::Int(0), s("search")), (rustpress_phpser::Key::Int(1), s("post_type")), (rustpress_phpser::Key::Int(2), s("taxonomy"))])),
            ("elements", empty()),
            ("return_format", s("object")),
            ("bidirectional_target", empty()),
        ],
        "select" => vec![
            ("multiple", i(0)),
            ("allow_null", i(0)),
            ("choices", empty()),
            ("default_value", s("")),
            ("ui", i(0)),
            ("ajax", i(0)),
            ("placeholder", s("")),
            ("return_format", s("value")),
            ("create_options", i(0)),
            ("save_options", i(0)),
        ],
        "tab" => vec![("placement", s("top")), ("endpoint", i(0)), ("selected", i(0))],
        "taxonomy" => vec![
            ("taxonomy", s("category")),
            ("field_type", s("checkbox")),
            ("multiple", i(0)),
            ("allow_null", i(0)),
            ("return_format", s("id")),
            ("add_term", i(1)),
            ("load_terms", i(0)),
            ("save_terms", i(0)),
            ("bidirectional_target", empty()),
        ],
        "text" => vec![("default_value", s("")), ("maxlength", s("")), ("placeholder", s("")), ("prepend", s("")), ("append", s(""))],
        "textarea" => vec![("default_value", s("")), ("new_lines", s("")), ("maxlength", s("")), ("placeholder", s("")), ("rows", s(""))],
        "time_picker" => vec![("display_format", s("g:i a")), ("return_format", s("g:i a"))],
        "true_false" => vec![("default_value", i(0)), ("message", s("")), ("ui", i(0)), ("ui_on_text", s("")), ("ui_off_text", s(""))],
        "url" => vec![("default_value", s("")), ("placeholder", s(""))],
        "user" => vec![("role", s("")), ("multiple", i(0)), ("allow_null", i(0)), ("return_format", s("array")), ("bidirectional_target", empty())],
        "wysiwyg" => vec![("tabs", s("all")), ("toolbar", s("full")), ("media_upload", i(1)), ("default_value", s("")), ("delay", i(0))],
        _ => Vec::new(),
    }
}

/// Whether a field type supports block bindings and the shortcode (`acf_field_type_supports( $type, 'bindings' )`).
pub fn supports_bindings(kind: &str) -> bool {
    !matches!(kind, "accordion" | "group" | "message" | "tab")
}

/// The field types of ACF (free) and whether they show in the REST API (`acf_field::$show_in_rest`).
pub fn show_in_rest(kind: &str) -> bool {
    matches!(
        kind,
        "text" | "textarea" | "number" | "range" | "email" | "url" | "password" | "image" | "file" | "wysiwyg" | "oembed" | "select" | "checkbox" | "radio" | "button_group" | "true_false" | "link"
            | "post_object" | "page_link" | "relationship" | "taxonomy" | "user" | "google_map" | "date_picker" | "date_time_picker" | "time_picker" | "color_picker" | "icon_picker" | "group"
    )
}

/// `acf_get_raw_field()` and `acf_validate_field()`: the stored settings, the post's attributes, the generic
/// defaults and the defaults of the type for the keys that are not set.
fn field_from(post: &AcfPost, all: &[AcfPost]) -> Field {
    let mut items = match rustpress_core::options::maybe_unserialize(&post.content) {
        Value::Array(items) => items,
        _ => Vec::new(),
    };
    let kind = items.iter().find(|(k, _)| super::php::key_eq(k, &super::php::key("type"))).map_or_else(|| "text".to_owned(), |(_, v)| strval(Some(v)));
    super::php::set(&mut items, "ID", Value::Int(post.id as i64));
    super::php::set(&mut items, "key", string(&post.name));
    super::php::set(&mut items, "label", string(&post.title));
    super::php::set(&mut items, "name", string(&post.excerpt));
    super::php::set(&mut items, "menu_order", Value::Int(post.menu_order));
    super::php::set(&mut items, "parent", Value::Int(post.parent as i64));
    super::php::set(&mut items, "type", string(&kind));
    // `isset()`: a stored null takes the default as well.
    let unset = |items: &Vec<(rustpress_phpser::Key, Value)>, name: &str| !items.iter().any(|(k, v)| super::php::key_eq(k, &super::php::key(name)) && !matches!(v, Value::Null));
    // `acf_field::validate_field()`: the defaults of the type for the keys that are not set.
    for (name, default) in type_defaults(&kind) {
        if unset(&items, name) {
            super::php::set(&mut items, name, default);
        }
    }
    // `acf_field_group::load_field()`: the sub fields of a group.
    let sub_fields = if kind == "group" { children(post.id, all) } else { Vec::new() };
    Field { key: post.name.clone(), name: post.excerpt.clone(), kind, settings: Value::Array(items), sub_fields }
}

/// A field from its settings (a local field, the golden tests): the type's defaults for the keys it does not set.
#[cfg(test)]
pub fn from_settings(settings: Value, sub_fields: Vec<Field>) -> Field {
    let mut items = match settings {
        Value::Array(items) => items,
        _ => Vec::new(),
    };
    let get = |items: &Vec<(rustpress_phpser::Key, Value)>, name: &str| items.iter().find(|(k, _)| super::php::key_eq(k, &super::php::key(name))).map(|(_, v)| strval(Some(v)));
    let kind = get(&items, "type").unwrap_or_else(|| "text".to_owned());
    for (name, default) in type_defaults(&kind) {
        if !items.iter().any(|(k, v)| super::php::key_eq(k, &super::php::key(name)) && !matches!(v, Value::Null)) {
            super::php::set(&mut items, name, default);
        }
    }
    Field { key: get(&items, "key").unwrap_or_default(), name: get(&items, "name").unwrap_or_default(), kind, settings: Value::Array(items), sub_fields }
}

/// `acf_get_fields()` of a parent (a field group or a group field).
fn children(parent: u64, all: &[AcfPost]) -> Vec<Field> {
    all.iter().filter(|p| p.parent == parent).map(|p| field_from(p, all)).collect()
}

/// The field groups of the database with their fields (`acf_get_field_groups()` without a filter).
pub async fn load(db: &Db) -> Result<Vec<FieldGroup>> {
    let groups = db.acf_posts("acf-field-group").await?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }
    let fields = db.acf_fields().await?;
    Ok(groups
        .iter()
        .map(|post| {
            let settings = rustpress_core::options::maybe_unserialize(&post.content);
            let location = parse_location(&settings);
            FieldGroup {
                id: post.id,
                key: post.name.clone(),
                title: post.title.clone(),
                active: matches!(post.status.as_str(), "publish" | "auto-draft"),
                show_in_rest: truthy(settings.get("show_in_rest")),
                location,
                fields: children(post.id, &fields),
                settings,
            }
        })
        .collect())
}

/// The `location` of a field group's settings: groups of rules.
pub(super) fn parse_location(settings: &Value) -> Vec<Vec<Rule>> {
    match settings.get("location") {
        Some(Value::Array(groups)) => groups
            .iter()
            .map(|(_, rules)| match rules {
                Value::Array(rules) => rules.iter().map(|(_, rule)| Rule { param: strval(rule.get("param")), operator: strval(rule.get("operator")), value: rule.get("value").cloned().unwrap_or(Value::Null) }).collect(),
                _ => Vec::new(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// What `acf_get_field_groups( $args )` matches against (the "screen").
pub enum Screen<'a> {
    /// `[ 'post_id' => $id ]`, with `post_type` for the revisions and autosaves of a post.
    Post { post: &'a Post, post_type: Option<&'a str> },
    /// `[ 'taxonomy' => $taxonomy ]`.
    Term { taxonomy: &'a str },
    /// `[ 'user_id' => $id, 'user_form' => $view ]` of the user screens (`None` is `'new'`), and
    /// `[ 'user_id' => $id, 'rest' => true ]` without a form.
    User { user_id: Option<u64>, form: Option<&'a str> },
    /// `[ 'comment' => $post_type ]`.
    Comment { post_type: String },
    /// `[ 'attachment_id' => $id, 'attachment' => $id ]`.
    Attachment { post: &'a Post },
    /// `[ 'post_type' => $post_type ]` without a post (the "Field Groups" column of ACF's post types).
    PostType { post_type: &'a str },
}

/// The request around a match: the database, the site and the signed-in user.
pub struct MatchContext<'a> {
    pub site: &'a rustpress_core::Site,
    pub user: MatchUser<'a>,
    /// `is_admin()`.
    pub admin: bool,
}

/// `wp_get_current_user()` of a match.
#[derive(Clone, Copy)]
pub enum MatchUser<'a> {
    Visitor,
    Viewer(&'a rustpress_core::Viewer),
    /// A user of the administration, by `current_user_can()`.
    Admin(&'a (dyn Fn(&str) -> bool + Send + Sync)),
}

impl<'a> MatchUser<'a> {
    pub fn of(viewer: Option<&'a rustpress_core::Viewer>) -> Self {
        viewer.map_or(MatchUser::Visitor, MatchUser::Viewer)
    }

    fn logged_in(&self) -> bool {
        !matches!(self, MatchUser::Visitor)
    }

    /// `user_can()`: roles count as capabilities.
    fn can(&self, capability: &str) -> bool {
        match self {
            MatchUser::Visitor => false,
            MatchUser::Viewer(viewer) => viewer.can(capability),
            MatchUser::Admin(can) => can(capability),
        }
    }
}

impl FieldGroup {
    /// A setting of the group with the default of `acf_validate_field_group()`.
    pub fn setting(&self, name: &str, default: &str) -> String {
        match self.settings.get(name) {
            Some(v) if !matches!(v, Value::Null) => strval(Some(v)),
            _ => default.to_owned(),
        }
    }

    /// `acf_get_field_group_visibility()`.
    pub async fn visible(&self, screen: &Screen<'_>, ctx: &MatchContext<'_>) -> Result<bool> {
        if !self.active {
            return Ok(false);
        }
        for group in &self.location {
            if group.is_empty() {
                continue;
            }
            let mut all = true;
            for rule in group {
                if !match_rule(rule, screen, ctx).await? {
                    all = false;
                    break;
                }
            }
            if all {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// `ACF_Location::compare_to_rule()`.
fn compare(value: &Value, rule: &Rule) -> bool {
    let mut result = loose_eq(value, &rule.value);
    if rule.value == string("all") {
        result = true;
    }
    if rule.operator == "!=" { !result } else { result }
}

fn reverse(result: bool, rule: &Rule) -> bool {
    if rule.operator == "!=" { !result } else { result }
}

/// The post types the theme has templates for (`acf_get_post_templates()`): pages always; block themes list their
/// custom templates for pages only unless `postTypes` says otherwise, which RustPress does not read here.
fn has_templates(post_type: &str) -> bool {
    post_type == "page"
}

/// `acf_match_location_rule()` for the location types of ACF (free). Types it does not know never match.
async fn match_rule(rule: &Rule, screen: &Screen<'_>, ctx: &MatchContext<'_>) -> Result<bool> {
    let db = &ctx.site.db;
    let rule_value = strval(Some(&rule.value));
    Ok(match rule.param.as_str() {
        "post_type" => match screen {
            Screen::Post { post_type: Some(post_type), .. } | Screen::PostType { post_type } => compare(&string(post_type), rule),
            Screen::Post { post, .. } => compare(&string(&post.post_type), rule),
            _ => false,
        },
        "post" | "page" => match screen {
            Screen::Post { post, .. } => compare(&Value::Int(post.id as i64), rule),
            _ => false,
        },
        "post_status" => match screen {
            Screen::Post { post, .. } => {
                let status = if post.post_status == "auto-draft" { "draft" } else { post.post_status.as_str() };
                compare(&string(status), rule)
            }
            _ => false,
        },
        "post_format" => match screen {
            Screen::Post { post, .. } => {
                // `get_post_format()`, `standard` where the type supports formats.
                let format = db.terms_for_post(post.id, Some("post_format")).await?.first().and_then(|t| t.slug.strip_prefix("post-format-").map(str::to_owned));
                let value = match format {
                    Some(format) => string(&format),
                    None if ctx.site.post_type_supports(&post.post_type, "post-formats") => string("standard"),
                    None => Value::Bool(false),
                };
                compare(&value, rule)
            }
            _ => false,
        },
        "post_category" | "post_taxonomy" => match screen {
            Screen::Post { post, .. } => {
                // `acf_get_term()`: `taxonomy:slug`, or a term ID.
                let term = match rule_value.split_once(':') {
                    Some((taxonomy, slug)) if !taxonomy.is_empty() => db.term_by_slug(taxonomy, slug.split(':').next().unwrap_or(slug)).await?,
                    _ => match rule_value.parse::<u64>() {
                        Ok(id) => db.term_by_id(None, id).await?,
                        Err(_) => None,
                    },
                };
                match term {
                    None => false,
                    Some(term) => {
                        let mut ids: Vec<u64> = db.terms_for_post(post.id, Some(&term.taxonomy)).await?.iter().map(|t| t.term_id).collect();
                        if ids.is_empty() && term.taxonomy == "category" {
                            ids.push(1);
                        }
                        reverse(ids.contains(&term.term_id), rule)
                    }
                }
            }
            _ => false,
        },
        "post_template" | "page_template" => match screen {
            Screen::Post { post, post_type } => {
                let post_type = post_type.unwrap_or(&post.post_type);
                if (rule.param == "page_template" && rule_value == "default" && post_type != "page") || !has_templates(post_type) {
                    false
                } else {
                    let template = db.acf_meta(MetaKind::Post, post.id, "_wp_page_template").await?.unwrap_or_default();
                    compare(&string(if template.is_empty() { "default" } else { &template }), rule)
                }
            }
            // No post: its template is "default".
            Screen::PostType { post_type } => {
                if (rule.param == "page_template" && rule_value == "default" && *post_type != "page") || !has_templates(post_type) {
                    false
                } else {
                    compare(&string("default"), rule)
                }
            }
            _ => false,
        },
        "page_type" => match screen {
            Screen::Post { post, .. } => {
                let id = post.id as i64;
                let result = match rule_value.as_str() {
                    "front_page" => ctx.site.settings.page_on_front as i64 == id,
                    "posts_page" => ctx.site.settings.page_for_posts as i64 == id,
                    "top_level" => post.post_parent == 0,
                    "parent" => db.acf_has_child(&post.post_type, post.id).await?,
                    "child" => post.post_parent != 0,
                    _ => return Ok(false),
                };
                reverse(result, rule)
            }
            _ => false,
        },
        "page_parent" => match screen {
            Screen::Post { post, .. } => compare(&Value::Int(post.post_parent as i64), rule),
            _ => false,
        },
        "current_user" => {
            let result = match rule_value.as_str() {
                "logged_in" => ctx.user.logged_in(),
                "viewing_front" => !ctx.admin,
                "viewing_back" => ctx.admin,
                _ => false,
            };
            reverse(result, rule)
        }
        // `is_super_admin()` of a single site is `delete_users`; a visitor has no roles.
        "current_user_role" => reverse(if rule_value == "super_admin" { ctx.user.can("delete_users") } else { !rule_value.is_empty() && ctx.user.can(&rule_value) }, rule),
        // The REST API has no forms; the choice "Add / Edit" (valued `edit`) also matches the add form.
        "user_form" => match screen {
            Screen::User { form: None, .. } => true,
            Screen::User { form: Some(form), .. } => compare(&string(if rule_value == "edit" && *form == "add" { "edit" } else { form }), rule),
            _ => false,
        },
        // A new user has the default role.
        "user_role" => match screen {
            Screen::User { user_id: None, .. } => compare(&string(&ctx.site.options.string("default_role", "")), rule),
            Screen::User { user_id: Some(user_id), .. } => {
                let role = if !rule_value.is_empty() && user_has_role(db, *user_id, &rule_value).await? { rule_value.clone() } else { String::new() };
                compare(&string(&role), rule)
            }
            _ => false,
        },
        "taxonomy" => match screen {
            Screen::Term { taxonomy } => compare(&string(taxonomy), rule),
            _ => false,
        },
        "comment" => match screen {
            Screen::Comment { post_type } => compare(&string(post_type), rule),
            _ => false,
        },
        // The MIME type; a value without a slash matches its first part (`image`).
        "attachment" => match screen {
            Screen::Attachment { post } => {
                let mime = &post.post_mime_type;
                let general = !rule_value.is_empty() && !rule_value.char_indices().any(|(i, c)| c == '/' && i > 0) && mime.split('/').next() == Some(rule_value.as_str());
                compare(&string(if general { &rule_value } else { mime }), rule)
            }
            _ => false,
        },
        _ => false,
    })
}

/// `user_can( $user_id, $role )`: the role is one of the user's capabilities.
async fn user_has_role(db: &Db, user_id: u64, role: &str) -> Result<bool> {
    let key = format!("{}capabilities", db.table_prefix());
    let caps = db.acf_meta(MetaKind::User, user_id, &key).await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
    Ok(caps.as_ref().and_then(|c| c.get(role)).is_some_and(|v| truthy(Some(v))))
}
