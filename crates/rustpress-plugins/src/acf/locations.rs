//! The location rules on the field group editor: `acf_get_location_rule_types()`, the operators
//! (`ACF_Location::get_operators()`) and the values (`get_values()`) of ACF's location types, and the views
//! `locations.php`, `location-group.php` and `location-rule.php`.
//!
//! Differs from the plugin: the widgets are those WordPress registers itself, no plugin's.

use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_core::{Result, Site};
use rustpress_phpser::{Key, Value};

use super::fields::Rule;
use super::php::{key, string, strval};
use super::render::{Ctx, RenderField, render_field};

fn tr(site: &Site, text: &str) -> String {
    site.trd("acf", text)
}

/// `(name, label, category)` of the location types (free), in the order `acf.php` includes them.
const TYPES: [(&str, &str, &str); 21] = [
    ("post_type", "Post Type", "post"),
    ("post_template", "Post Template", "post"),
    ("post_status", "Post Status", "post"),
    ("post_format", "Post Format", "post"),
    ("post_category", "Post Category", "post"),
    ("post_taxonomy", "Post Taxonomy", "post"),
    ("post", "Post", "post"),
    ("page_template", "Page Template", "page"),
    ("page_type", "Page Type", "page"),
    ("page_parent", "Page Parent", "page"),
    ("page", "Page", "page"),
    ("current_user", "Current User", "user"),
    ("current_user_role", "Current User Role", "user"),
    ("user_form", "User Form", "user"),
    ("user_role", "User Role", "user"),
    ("taxonomy", "Taxonomy", "forms"),
    ("attachment", "Attachment", "forms"),
    ("comment", "Comment", "forms"),
    ("widget", "Widget", "forms"),
    ("nav_menu", "Menu", "forms"),
    ("nav_menu_item", "Menu Item", "forms"),
];

fn pairs(items: Vec<(String, String)>) -> Vec<(Key, Value)> {
    items.into_iter().map(|(k, v)| (key(&k), string(&v))).collect()
}

/// `acf_get_location_rule_types()`: the types by their category's label.
pub fn rule_types(site: &Site) -> Value {
    let categories = [("post", "Post"), ("page", "Page"), ("user", "User"), ("forms", "Forms")];
    let mut out: Vec<(Key, Value)> = Vec::new();
    for (name, label, category) in TYPES {
        let category = categories.iter().find(|(k, _)| *k == category).map_or_else(|| category.to_owned(), |(_, v)| tr(site, v));
        let entry = (key(name), string(&esc_html(&tr(site, label))));
        match out.iter_mut().find(|(k, _)| *k == key(&category)) {
            Some((_, Value::Array(items))) => items.push(entry),
            _ => out.push((key(&category), Value::Array(vec![entry]))),
        }
    }
    Value::Array(out)
}

/// `ACF_Location::get_operators()`.
pub fn operators(site: &Site) -> Value {
    Value::Array(vec![(key("=="), string(&tr(site, "is equal to"))), (key("!="), string(&tr(site, "is not equal to")))])
}

/// `acf_get_post_types( [ 'show_ui' => 1, 'exclude' => $exclude ] )`.
fn ui_post_types(site: &Site, exclude: &[&str]) -> Vec<String> {
    let admin = site.types.admin_post_types();
    super::format::all_post_types(site).into_iter().filter(|t| !exclude.contains(&t.as_str())).filter(|t| matches!(t.as_str(), "post" | "page" | "attachment") || admin.iter().any(|a| a.name == *t && a.show_ui)).collect()
}

/// `acf_get_pretty_post_types( $post_types )`.
fn pretty_post_types(site: &Site, types: &[String]) -> Vec<(String, String)> {
    super::render::disambiguate(types.iter().map(|t| (t.clone(), super::render::post_type_singular(site, t))).collect())
}

/// The custom templates of the theme for a post type (`theme.json` `customTemplates`; pages without `postTypes`).
fn custom_templates(site: &Site, wp_root: &std::path::Path, post_type: &str) -> Vec<(String, String)> {
    let themes = wp_root.join("wp-content/themes");
    for theme in [&site.settings.stylesheet, &site.settings.template] {
        let Some(raw) = rustpress_core::files::read(&themes.join(theme).join("theme.json")) else { continue };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else { continue };
        let Some(templates) = json.get("customTemplates").and_then(serde_json::Value::as_array) else { continue };
        return templates
            .iter()
            .filter(|t| t.get("postTypes").and_then(serde_json::Value::as_array).map_or(post_type == "page", |types| types.iter().any(|p| p.as_str() == Some(post_type))))
            .filter_map(|t| Some((t.get("name")?.as_str()?.to_owned(), t.get("title")?.as_str()?.to_owned())))
            .collect();
    }
    Vec::new()
}

/// `acf_get_post_templates()`: pages always, then the post types with templates.
fn post_templates(site: &Site, wp_root: &std::path::Path) -> Vec<(String, Vec<(String, String)>)> {
    let mut out = vec![("page".to_owned(), custom_templates(site, wp_root, "page"))];
    for post_type in super::format::all_post_types(site) {
        if post_type == "page" {
            continue;
        }
        let templates = custom_templates(site, wp_root, &post_type);
        if !templates.is_empty() {
            out.push((post_type, templates));
        }
    }
    out
}

/// `get_post_stati()` with the labels (a status without one shows its name).
fn post_statuses(site: &Site) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = [("publish", "Published", "post status"), ("future", "Scheduled", "post status"), ("draft", "Draft", "post status"), ("pending", "Pending", "post status"), ("private", "Private", "post status"), ("trash", "Trash", "post status")]
        .iter()
        .map(|(k, label, context)| ((*k).to_owned(), site.trx(label, context)))
        .collect();
    out.push(("auto-draft".into(), "auto-draft".into()));
    out.push(("inherit".into(), "inherit".into()));
    for (k, label) in [("request-pending", "Pending"), ("request-confirmed", "Confirmed"), ("request-failed", "Failed"), ("request-completed", "Completed")] {
        out.push((k.to_owned(), site.trx(label, "request status")));
    }
    out.extend(site.plugins.post_statuses(site).into_iter().map(|(k, label)| (k.to_owned(), label)));
    out
}

/// `acf_get_grouped_posts( [ 'post_type' => $types ] )`: the posts of every status but the trash, by the type's
/// label, ordered by `menu_order` and title, hierarchical types as their tree; titles as `acf_get_post_title()`.
async fn grouped_posts(render: &Ctx<'_>, types: &[String]) -> Result<Vec<(String, Vec<(String, String)>)>> {
    let site = render.site;
    let labels = pretty_post_types(site, types);
    let mut out = Vec::new();
    for post_type in types {
        let query = rustpress_db::PostQuery {
            post_types: vec![post_type.clone()],
            statuses: ["publish", "future", "draft", "pending", "private"].iter().map(|s| (*s).to_owned()).collect(),
            order_by: rustpress_db::OrderBy::MenuOrderTitle,
            order: rustpress_db::Order::Asc,
            ..Default::default()
        };
        let mut posts = site.db.posts(&query).await?;
        if posts.is_empty() {
            continue;
        }
        if is_hierarchical(site, post_type) {
            posts = tree(&posts);
        }
        let mut items = Vec::new();
        for post in &posts {
            items.push((post.id.to_string(), super::render::post_title(render, post).await?));
        }
        let label = labels.iter().find(|(t, _)| t == post_type).map(|(_, l)| l.clone()).unwrap_or_default();
        out.push((label, items));
    }
    Ok(out)
}

/// `is_post_type_hierarchical()`.
fn is_hierarchical(site: &Site, post_type: &str) -> bool {
    match post_type {
        "page" => true,
        "post" | "attachment" => false,
        other => site.types.any_post_type(other).is_some_and(|t| t.hierarchical) || site.types.admin_post_types().iter().any(|t| t.name == other && t.hierarchical),
    }
}

/// `get_page_children()` from the top: each post followed by its children, depth first.
fn tree(posts: &[rustpress_db::Post]) -> Vec<rustpress_db::Post> {
    fn walk(parent: u64, posts: &[rustpress_db::Post], seen: &mut Vec<u64>, out: &mut Vec<rustpress_db::Post>) {
        for post in posts.iter().filter(|p| p.post_parent == parent) {
            if seen.contains(&post.id) {
                continue;
            }
            seen.push(post.id);
            out.push(post.clone());
            walk(post.id, posts, seen, out);
        }
    }
    let mut out = Vec::new();
    let mut seen = Vec::new();
    let ids: Vec<u64> = posts.iter().map(|p| p.id).collect();
    // Posts whose parent is not in the list start a tree of their own, in their order.
    for post in posts {
        if post.post_parent == 0 || !ids.contains(&post.post_parent) {
            if seen.contains(&post.id) {
                continue;
            }
            seen.push(post.id);
            out.push(post.clone());
            walk(post.id, posts, &mut seen, &mut out);
        }
    }
    out
}

/// The widgets WordPress registers (`wp_widgets_init()`): `(id_base, name)`.
fn widgets(site: &Site) -> Vec<(String, String)> {
    [
        ("pages", "Pages"),
        ("calendar", "Calendar"),
        ("archives", "Archives"),
        ("media_audio", "Audio"),
        ("media_image", "Image"),
        ("media_gallery", "Gallery"),
        ("media_video", "Video"),
        ("meta", "Meta"),
        ("search", "Search"),
        ("text", "Text"),
        ("categories", "Categories"),
        ("recent-posts", "Recent Posts"),
        ("recent-comments", "Recent Comments"),
        ("rss", "RSS"),
        ("tag_cloud", "Tag Cloud"),
        ("nav_menu", "Navigation Menu"),
        ("custom_html", "Custom HTML"),
        ("block", "Block"),
    ]
    .iter()
    .map(|(k, name)| ((*k).to_owned(), site.tr(name)))
    .collect()
}

/// `get_values( $rule )` of the rule's location type.
pub async fn values(render: &Ctx<'_>, wp_root: &std::path::Path, rule: &Rule) -> Result<Value> {
    let site = render.site;
    let all = || (key("all"), string(&tr(site, "All")));
    let default_template = || (key("default"), string(&tr(site, "Default Template")));
    Ok(Value::Array(match rule.param.as_str() {
        "post_type" => pairs(pretty_post_types(site, &ui_post_types(site, &["attachment"]))),
        "post_template" => {
            let mut out = vec![default_template()];
            for (post_type, templates) in post_templates(site, wp_root) {
                out.push((key(&post_type), Value::Array(pairs(templates))));
            }
            out
        }
        "post_status" => pairs(post_statuses(site)),
        "post_format" => pairs(["standard", "aside", "chat", "gallery", "link", "image", "quote", "status", "video", "audio"].iter().map(|f| ((*f).to_owned(), site.trx(&capitalize(f), "Post format"))).collect()),
        "post_category" => match super::render::category_terms(render).await? {
            Value::Array(items) => items,
            _ => Vec::new(),
        },
        "post_taxonomy" => match super::render::taxonomy_terms(render).await? {
            Value::Array(items) => items,
            _ => Vec::new(),
        },
        "post" => grouped_posts(render, &ui_post_types(site, &["page", "attachment"])).await?.into_iter().map(|(label, posts)| (key(&label), Value::Array(pairs(posts)))).collect(),
        "page_template" => {
            let mut out = vec![default_template()];
            out.extend(pairs(post_templates(site, wp_root).into_iter().next().map(|(_, t)| t).unwrap_or_default()));
            out
        }
        "page_type" => [("front_page", "Front Page"), ("posts_page", "Posts Page"), ("top_level", "Top Level Page (no parent)"), ("parent", "Parent Page (has children)"), ("child", "Child Page (has parent)")].iter().map(|(k, v)| (key(k), string(&tr(site, v)))).collect(),
        "page_parent" | "page" => grouped_posts(render, &["page".to_owned()]).await?.into_iter().next().map(|(_, posts)| pairs(posts)).unwrap_or_default(),
        "current_user" => [("logged_in", "Logged in"), ("viewing_front", "Viewing front end"), ("viewing_back", "Viewing back end")].iter().map(|(k, v)| (key(k), string(&tr(site, v)))).collect(),
        // `wp_roles()->get_names()`: not translated.
        "current_user_role" => pairs(role_names(site)),
        "user_form" => [("all", "All"), ("add", "Add"), ("edit", "Add / Edit"), ("register", "Register")].iter().map(|(k, v)| (key(k), string(&tr(site, v)))).collect(),
        "user_role" => {
            let mut out = vec![all()];
            out.extend(pairs(role_names(site)));
            out
        }
        "taxonomy" => {
            let mut out = vec![all()];
            let taxonomies = super::render::acf_taxonomies(site, None);
            out.extend(pairs(super::render::taxonomy_singulars(site, &taxonomies)));
            out
        }
        "attachment" => {
            let mut out = vec![all()];
            let mimes = rustpress_content::media::allowed_mime_types_for(site, &[], render.can, false, false);
            for (regex, mime) in mimes {
                let kind = mime.split('/').next().unwrap_or_default().to_owned();
                let group = match out.iter().position(|(k, _)| *k == key(&kind)) {
                    Some(i) => i,
                    None => {
                        out.push((key(&kind), Value::Array(Vec::new())));
                        out.len() - 1
                    }
                };
                if let Value::Array(items) = &mut out[group].1 {
                    super::php::set(items, &kind, string(&rustpress_core::i18n::sprintf(&tr(site, "All %s formats"), &[&kind])));
                    super::php::set(items, &mime, string(&format!("{regex} ({mime})")));
                }
            }
            out
        }
        "comment" => {
            let mut out = vec![all()];
            out.extend(pairs(pretty_post_types(site, &super::format::all_post_types(site))));
            out
        }
        "widget" => {
            let mut out = vec![all()];
            out.extend(pairs(widgets(site)));
            out
        }
        "nav_menu" | "nav_menu_item" => {
            let mut out = vec![all()];
            let menus = site.db.terms("nav_menu").await?;
            if !menus.is_empty() {
                out.push((key(&tr(site, "Menus")), Value::Array(menus.iter().map(|m| (Key::Int(m.term_id as i64), string(&m.name))).collect())));
            }
            out
        }
        _ => Vec::new(),
    }))
}

fn capitalize(text: &str) -> String {
    let mut c = text.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// `wp_roles()->get_names()`.
fn role_names(site: &Site) -> Vec<(String, String)> {
    let Some(Value::Array(items)) = site.options.value(&format!("{}user_roles", site.db.table_prefix())) else { return Vec::new() };
    items.iter().filter(|(k, _)| matches!(k, Key::Str(_))).map(|(k, role)| (super::render::key_text(k), strval(role.get("name")))).collect()
}

/// `views/acf-field-group/location-rule.php`.
async fn rule_row(render: &Ctx<'_>, wp_root: &std::path::Path, rule: &Rule, group_id: &str, rule_id: &str) -> Result<String> {
    let site = render.site;
    let prefix = format!("acf_field_group[location][{group_id}][{rule_id}]");
    let select = |name: &str, value: Value, choices: Value, class: Option<&str>| {
        let mut items = vec![(key("type"), string("select")), (key("name"), string(name)), (key("prefix"), string(&prefix)), (key("value"), value), (key("choices"), choices)];
        if let Some(class) = class {
            items.push((key("class"), string(class)));
        }
        RenderField::raw(items, Vec::new())
    };
    let param = render_field(render, select("param", string(&rule.param), rule_types(site), Some("refresh-location-rule"))).await?;
    let operator = render_field(render, select("operator", string(&rule.operator), operators(site), None)).await?;
    let value = render_field(render, select("value", rule.value.clone(), values(render, wp_root, rule).await?, Some("location-rule-value"))).await?;
    Ok(format!(
        "<tr data-id=\"{}\">\n\t<td class=\"param\">\n\t\t{param}\t</td>\n\t<td class=\"operator\">\n\t\t{operator}\t</td>\n\t<td class=\"value\">\n\t\t{value}\t</td>\n\t<td class=\"add\">\n\t\t<a href=\"#\" class=\"button add-location-rule\">{}</a>\n\t</td>\n\t<td class=\"remove\">\n\t\t<a href=\"#\" class=\"acf-icon -minus remove-location-rule\"></a>\n\t</td>\n</tr>\n",
        esc_attr(rule_id),
        esc_html(&tr(site, "and")),
    ))
}

/// `views/acf-field-group/locations.php`: the rule groups (a group without rules ends the list, as `return` does).
pub async fn locations(render: &Ctx<'_>, wp_root: &std::path::Path, location: &[Vec<Rule>]) -> Result<String> {
    let site = render.site;
    let mut out = format!(
        "<div class=\"acf-field\">\n\t<div class=\"acf-label\">\n\t\t<label>{}</label>\n\t\t<i tabindex=\"0\" class=\"acf-icon acf-icon-help acf-js-tooltip\" title=\"{}\">?</i>\n\t</div>\n\t<div class=\"acf-input\">\n\t\t<div class=\"rule-groups\">\n\n\t\t\t",
        esc_html(&tr(site, "Rules")),
        esc_attr(&tr(site, "Create a set of rules to determine which edit screens will use these advanced custom fields"))
    );
    for (i, group) in location.iter().enumerate() {
        if group.is_empty() {
            return Ok(out);
        }
        let group_id = format!("group_{i}");
        out.push_str(&format!(
            "<div class=\"rule-group\" data-id=\"{}\">\n\n\t<h4>{}</h4>\n\t\n\t<table class=\"acf-table -clear\">\n\t\t<tbody>\n\t\t\t",
            esc_attr(&group_id),
            if group_id == "group_0" { esc_html(&tr(site, "Show this field group if")) } else { esc_html(&tr(site, "or")) }
        ));
        for (j, rule) in group.iter().enumerate() {
            out.push_str(&rule_row(render, wp_root, rule, &group_id, &format!("rule_{j}")).await?);
        }
        out.push_str("\t\t</tbody>\n\t</table>\n\t\n</div>\n");
    }
    out.push_str(&format!(
        "\n\t\t\t<h4>{}</h4>\n\n\t\t\t<a href=\"#\" class=\"button add-location-group\">{}</a>\n\n\t\t</div>\n\t</div>\n</div>\n<script type=\"text/javascript\">\nif( typeof acf !== 'undefined' ) {{\n\n\tacf.newPostbox({{\n\t\t'id': 'acf-field-group-locations',\n\t\t'label': 'left'\n\t}});\n\n}}\n</script>\n",
        esc_html(&tr(site, "or")),
        esc_html(&tr(site, "Add rule group"))
    ));
    Ok(out)
}
