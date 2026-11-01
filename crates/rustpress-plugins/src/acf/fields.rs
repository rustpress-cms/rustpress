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
    pub active: bool,
    pub show_in_rest: bool,
    /// Groups of rules: a group matches when all its rules do, the field group when one group does.
    pub location: Vec<Vec<Rule>>,
    pub fields: Vec<Field>,
}

/// The defaults of a field type that loading a value depends on (`acf_field::$defaults`): whether it has a
/// `default_value`, and that value.
fn type_default_value(kind: &str) -> Option<Value> {
    match kind {
        "text" | "textarea" | "number" | "range" | "email" | "url" | "wysiwyg" | "select" | "checkbox" | "radio" | "button_group" | "color_picker" => Some(string("")),
        "true_false" => Some(Value::Int(0)),
        "icon_picker" => Some(Value::Array(vec![(super::php::key("type"), Value::Null), (super::php::key("value"), Value::Null)])),
        _ => None,
    }
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
    if let Some(default) = type_default_value(&kind)
        && unset(&items, "default_value")
    {
        super::php::set(&mut items, "default_value", default);
    }
    if matches!(kind.as_str(), "select" | "post_object" | "page_link" | "taxonomy" | "user") && unset(&items, "multiple") {
        super::php::set(&mut items, "multiple", Value::Int(0));
    }
    // `acf_field_group::load_field()`: the sub fields of a group.
    let sub_fields = if kind == "group" { children(post.id, all) } else { Vec::new() };
    Field { key: post.name.clone(), name: post.excerpt.clone(), kind, settings: Value::Array(items), sub_fields }
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
            let location = match settings.get("location") {
                Some(Value::Array(groups)) => groups
                    .iter()
                    .map(|(_, rules)| match rules {
                        Value::Array(rules) => rules
                            .iter()
                            .map(|(_, rule)| Rule { param: strval(rule.get("param")), operator: strval(rule.get("operator")), value: rule.get("value").cloned().unwrap_or(Value::Null) })
                            .collect(),
                        _ => Vec::new(),
                    })
                    .collect(),
                _ => Vec::new(),
            };
            FieldGroup {
                active: matches!(post.status.as_str(), "publish" | "auto-draft"),
                show_in_rest: truthy(settings.get("show_in_rest")),
                location,
                fields: children(post.id, &fields),
            }
        })
        .collect())
}

/// What `acf_get_field_groups( $args )` matches against (the "screen").
pub enum Screen<'a> {
    /// `[ 'post_id' => $id ]`, with `post_type` for the revisions and autosaves of a post.
    Post { post: &'a Post, post_type: Option<&'a str> },
    /// `[ 'taxonomy' => $taxonomy ]`.
    Term { taxonomy: &'a str },
    /// `[ 'user_id' => $id, 'rest' => true ]`.
    User { user_id: u64 },
    /// `[ 'comment' => $post_type ]`.
    Comment { post_type: String },
}

/// The request around a match: the database, the site and the signed-in user.
pub struct MatchContext<'a> {
    pub site: &'a rustpress_core::Site,
    pub viewer: Option<&'a rustpress_core::Viewer>,
}

impl FieldGroup {
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
            Screen::Post { post_type: Some(post_type), .. } => compare(&string(post_type), rule),
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
                "logged_in" => ctx.viewer.is_some(),
                "viewing_front" => true,
                _ => false,
            };
            reverse(result, rule)
        }
        "current_user_role" => match ctx.viewer {
            None => false,
            Some(viewer) => reverse(if rule_value == "super_admin" { false } else { !rule_value.is_empty() && viewer.can(&rule_value) }, rule),
        },
        "user_form" => matches!(screen, Screen::User { .. }),
        "user_role" => match screen {
            Screen::User { user_id } => {
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
        _ => false,
    })
}

/// `user_can( $user_id, $role )`: the role is one of the user's capabilities.
async fn user_has_role(db: &Db, user_id: u64, role: &str) -> Result<bool> {
    let key = format!("{}capabilities", db.table_prefix());
    let caps = db.acf_meta(MetaKind::User, user_id, &key).await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
    Ok(caps.as_ref().and_then(|c| c.get(role)).is_some_and(|v| truthy(Some(v))))
}
