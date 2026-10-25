//! `wp/v2/types` and `wp/v2/taxonomies` in the `edit` context.

use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::meta::{TaxonomyEntry, TypeEntry};
use crate::util::Links;
use crate::{Outcome, Resp, Rest};

fn s(text: &str) -> Value {
    Value::from(text.to_owned())
}

impl Rest<'_> {
    /// `get_post_type_labels()`: the labels of a post type.
    fn type_labels(&self, entry: &TypeEntry, singular: &str, plural: &str) -> Value {
        let site = self.site;
        let lower = plural.to_lowercase();
        let lower_singular = singular.to_lowercase();
        let hierarchical = entry.hierarchical;
        let mut labels = Map::new();
        labels.insert("name".into(), s(plural));
        labels.insert("singular_name".into(), s(singular));
        labels.insert("add_new".into(), s(&site.trx("Add", "post")));
        labels.insert("add_new_item".into(), s(&rustpress_core::i18n::sprintf(&site.tr(if hierarchical { "Add Page" } else { "Add Post" }), &[]).replace("Page", singular).replace("Post", singular)));
        labels.insert("edit_item".into(), s(&format!("{} {singular}", site.tr("Edit"))));
        labels.insert("new_item".into(), s(&format!("{} {singular}", site.tr("New"))));
        labels.insert("view_item".into(), s(&format!("{} {singular}", site.tr("View"))));
        labels.insert("view_items".into(), s(&format!("{} {plural}", site.tr("View"))));
        labels.insert("search_items".into(), s(&format!("{} {plural}", site.tr("Search"))));
        labels.insert("not_found".into(), s(&rustpress_core::i18n::sprintf(&site.tr(if hierarchical { "No pages found." } else { "No posts found." }), &[]).replace("pages", &lower).replace("posts", &lower)));
        labels.insert("not_found_in_trash".into(), s(&rustpress_core::i18n::sprintf(&site.tr(if hierarchical { "No pages found in Trash." } else { "No posts found in Trash." }), &[]).replace("pages", &lower).replace("posts", &lower)));
        labels.insert("parent_item_colon".into(), if hierarchical { s(&format!("{} {singular}:", site.tr("Parent"))) } else { Value::Null });
        labels.insert("all_items".into(), s(&format!("{} {plural}", site.tr("All"))));
        labels.insert("archives".into(), s(&format!("{singular} {}", site.tr("Archives"))));
        labels.insert("attributes".into(), s(&format!("{singular} {}", site.tr("Attributes"))));
        labels.insert("insert_into_item".into(), s(&format!("{} {lower_singular}", site.tr("Insert into"))));
        labels.insert("uploaded_to_this_item".into(), s(&format!("{} {lower_singular}", site.tr("Uploaded to this"))));
        labels.insert("featured_image".into(), s(&site.tr("Featured image")));
        labels.insert("set_featured_image".into(), s(&site.tr("Set featured image")));
        labels.insert("remove_featured_image".into(), s(&site.tr("Remove featured image")));
        labels.insert("use_featured_image".into(), s(&site.tr("Use as featured image")));
        labels.insert("filter_items_list".into(), s(&format!("{} {lower} {}", site.tr("Filter"), site.tr("list"))));
        labels.insert("filter_by_date".into(), s(&site.tr("Filter by date")));
        labels.insert("items_list_navigation".into(), s(&format!("{plural} {}", site.tr("list navigation"))));
        labels.insert("items_list".into(), s(&format!("{plural} {}", site.tr("list"))));
        labels.insert("item_published".into(), s(&format!("{singular} {}", site.tr("published."))));
        labels.insert("item_published_privately".into(), s(&format!("{singular} {}", site.tr("published privately."))));
        labels.insert("item_reverted_to_draft".into(), s(&format!("{singular} {}", site.tr("reverted to draft."))));
        labels.insert("item_trashed".into(), s(&format!("{singular} {}", site.tr("trashed."))));
        labels.insert("item_scheduled".into(), s(&format!("{singular} {}", site.tr("scheduled."))));
        labels.insert("item_updated".into(), s(&format!("{singular} {}", site.tr("updated."))));
        labels.insert("item_link".into(), s(&format!("{singular} {}", site.tr("Link"))));
        labels.insert("item_link_description".into(), s(&format!("{} {lower_singular}.", site.tr("A link to a"))));
        labels.insert("menu_name".into(), s(plural));
        labels.insert("name_admin_bar".into(), s(singular));
        Value::Object(labels)
    }

    /// The `supports` object of a type.
    fn type_supports(&self, name: &str) -> Value {
        let features: &[&str] = match name {
            "post" => &["title", "editor", "author", "thumbnail", "excerpt", "trackbacks", "custom-fields", "comments", "revisions", "post-formats", "autosave"],
            "page" => &["title", "editor", "author", "thumbnail", "page-attributes", "custom-fields", "comments", "revisions", "autosave"],
            "attachment" => &["title", "author", "comments"],
            _ => &[],
        };
        let mut map = Map::new();
        if features.is_empty() {
            let known = ["title", "editor", "author", "thumbnail", "excerpt", "trackbacks", "custom-fields", "comments", "revisions", "page-attributes", "post-formats", "autosave"];
            for feature in known {
                if self.site.post_type_supports(name, feature) {
                    map.insert(feature.to_owned(), Value::Bool(true));
                }
            }
        } else {
            for feature in features {
                map.insert((*feature).to_owned(), Value::Bool(true));
            }
        }
        if let Some(Value::Bool(true)) = map.get("editor") {
            map.insert("editor".into(), json!([{"notes": true}]));
        }
        Value::Object(map)
    }

    fn prepare_type_edit(&self, entry: &TypeEntry) -> Value {
        let (singular, plural) = match entry.name.as_str() {
            "post" => (self.site.trx("Post", "post type singular name"), entry.label.clone()),
            "page" => (self.site.trx("Page", "post type singular name"), entry.label.clone()),
            "attachment" => (self.site.trx("Media", "post type singular name"), entry.label.clone()),
            other => (self.site.types.post_type(other).map_or_else(|| entry.label.clone(), |t| t.singular_label()), entry.label.clone()),
        };
        let plural_caps = if entry.name == "page" { "pages" } else { "posts" };
        let single_caps = if entry.name == "page" { "page" } else { "post" };
        let mut caps = Map::new();
        for (key, value) in [
            ("edit_post", format!("edit_{single_caps}")),
            ("read_post", format!("read_{single_caps}")),
            ("delete_post", format!("delete_{single_caps}")),
            ("edit_posts", format!("edit_{plural_caps}")),
            ("edit_others_posts", format!("edit_others_{plural_caps}")),
            ("delete_posts", format!("delete_{plural_caps}")),
            ("publish_posts", format!("publish_{plural_caps}")),
            ("read_private_posts", format!("read_private_{plural_caps}")),
            ("read", "read".to_owned()),
            ("delete_private_posts", format!("delete_private_{plural_caps}")),
            ("delete_published_posts", format!("delete_published_{plural_caps}")),
            ("delete_others_posts", format!("delete_others_{plural_caps}")),
            ("edit_private_posts", format!("edit_private_{plural_caps}")),
            ("edit_published_posts", format!("edit_published_{plural_caps}")),
            ("create_posts", format!("edit_{plural_caps}")),
        ] {
            caps.insert(key.to_owned(), Value::from(value));
        }
        let mut data = Map::new();
        data.insert("capabilities".into(), Value::Object(caps));
        data.insert("description".into(), s(&entry.description));
        data.insert("hierarchical".into(), Value::Bool(entry.hierarchical));
        data.insert("has_archive".into(), entry.has_archive.clone());
        data.insert("visibility".into(), json!({"show_in_nav_menus": entry.name != "attachment", "show_ui": true}));
        data.insert("viewable".into(), Value::Bool(self.site.is_post_type_viewable(&entry.name)));
        data.insert("labels".into(), self.type_labels(entry, &singular, &entry.label));
        data.insert("name".into(), s(&plural));
        data.insert("slug".into(), s(&entry.name));
        data.insert("icon".into(), entry.icon.clone());
        data.insert("supports".into(), self.type_supports(&entry.name));
        data.insert("taxonomies".into(), json!(entry.taxonomies));
        data.insert("rest_base".into(), s(&entry.rest_base));
        data.insert("rest_namespace".into(), s("wp/v2"));
        data.insert("template".into(), Value::Array(Vec::new()));
        data.insert("template_lock".into(), Value::Bool(false));
        let mut links = Links::new();
        links.add("collection", vec![], self.rest_url("wp/v2/types"));
        links.add("wp:items", vec![], self.rest_url(&format!("wp/v2/{}", entry.rest_base)));
        data.insert("_links".into(), links.finish());
        Value::Object(data)
    }

    pub(crate) fn types_list_edit(&self, args: &Args) -> Outcome {
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        let mut data = Map::new();
        for entry in self.type_entries() {
            if context == "edit" {
                if !self.can_create_or_edit_type(&entry.name) {
                    continue;
                }
                data.insert(entry.name.clone(), self.prepare_type_edit(&entry));
            } else {
                data.insert(entry.name.clone(), self.prepare_type(&entry, &context));
            }
        }
        if context == "edit" && data.is_empty() {
            return Err(self.forbidden("rest_cannot_view", "Sorry, you are not allowed to edit posts in this post type."));
        }
        Ok(Resp::item(Value::Object(data)))
    }

    fn can_create_or_edit_type(&self, name: &str) -> bool {
        self.can_create(name) || self.can(if name == "page" { "edit_pages" } else { "edit_posts" })
    }

    pub(crate) fn type_get_edit(&self, name: &str, args: &Args) -> Outcome {
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        let Some(entry) = self.type_entries().into_iter().find(|e| e.name == name) else {
            return Err(RestError::new("rest_type_invalid", "Invalid post type.", 404).after_permission_check());
        };
        if context == "edit" {
            if !self.can_create_or_edit_type(name) {
                return Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit posts in this post type."));
            }
            return Ok(Resp::item(self.prepare_type_edit(&entry)));
        }
        Ok(Resp::item(self.prepare_type(&entry, &context)))
    }

    fn taxonomy_labels(&self, entry: &TaxonomyEntry) -> Value {
        let site = self.site;
        let (singular, plural) = match entry.name.as_str() {
            "category" => (site.trx("Category", "taxonomy singular name"), entry.label.clone()),
            "post_tag" => (site.trx("Tag", "taxonomy singular name"), entry.label.clone()),
            other => (site.types.taxonomy(other).map_or_else(|| entry.label.clone(), |t| t.singular_label()), entry.label.clone()),
        };
        let lower = plural.to_lowercase();
        let hier = entry.hierarchical;
        let mut labels = Map::new();
        let put = |labels: &mut Map<String, Value>, key: &str, value: Value| {
            labels.insert(key.to_owned(), value);
        };
        put(&mut labels, "name", s(&plural));
        put(&mut labels, "singular_name", s(&singular));
        put(&mut labels, "search_items", s(&format!("{} {plural}", site.tr("Search"))));
        put(&mut labels, "popular_items", if hier { Value::Null } else { s(&format!("{} {plural}", site.tr("Popular"))) });
        put(&mut labels, "all_items", s(&format!("{} {plural}", site.tr("All"))));
        put(&mut labels, "parent_item", if hier { s(&format!("{} {singular}", site.tr("Parent"))) } else { Value::Null });
        put(&mut labels, "parent_item_colon", if hier { s(&format!("{} {singular}:", site.tr("Parent"))) } else { Value::Null });
        put(&mut labels, "name_field_description", s(&site.tr("The name is how it appears on your site.")));
        put(&mut labels, "slug_field_description", s(&site.tr("The &#8220;slug&#8221; is the URL-friendly version of the name. It is usually all lowercase and contains only letters, numbers, and hyphens.")));
        put(&mut labels, "parent_field_description", if hier { s(&site.tr("Assign a parent term to create a hierarchy. The term Jazz, for example, would be the parent of Bebop and Big Band.")) } else { Value::Null });
        put(&mut labels, "desc_field_description", s(&site.tr("The description is not prominent by default; however, some themes may show it.")));
        put(&mut labels, "edit_item", s(&format!("{} {singular}", site.tr("Edit"))));
        put(&mut labels, "view_item", s(&format!("{} {singular}", site.tr("View"))));
        put(&mut labels, "update_item", s(&format!("{} {singular}", site.tr("Update"))));
        put(&mut labels, "add_new_item", s(&format!("{} {singular}", site.tr("Add"))));
        put(&mut labels, "new_item_name", s(&format!("{} {singular} {}", site.tr("New"), site.tr("Name"))));
        put(&mut labels, "separate_items_with_commas", if hier { Value::Null } else { s(&format!("{} {lower} {}", site.tr("Separate"), site.tr("with commas"))) });
        put(&mut labels, "add_or_remove_items", if hier { Value::Null } else { s(&format!("{} {lower}", site.tr("Add or remove"))) });
        put(&mut labels, "choose_from_most_used", if hier { Value::Null } else { s(&format!("{} {lower}", site.tr("Choose from the most used"))) });
        put(&mut labels, "not_found", s(&format!("{} {lower} {}", site.tr("No"), site.tr("found."))));
        put(&mut labels, "no_terms", s(&format!("{} {lower}", site.tr("No"))));
        put(&mut labels, "filter_by_item", if hier { s(&format!("{} {}", site.tr("Filter by"), singular.to_lowercase())) } else { Value::Null });
        put(&mut labels, "items_list_navigation", s(&format!("{plural} {}", site.tr("list navigation"))));
        put(&mut labels, "items_list", s(&format!("{plural} {}", site.tr("list"))));
        put(&mut labels, "most_used", s(&site.tr("Most Used")));
        put(&mut labels, "back_to_items", s(&format!("&larr; {} {plural}", site.tr("Go to"))));
        put(&mut labels, "item_link", s(&format!("{singular} {}", site.tr("Link"))));
        put(&mut labels, "item_link_description", s(&format!("{} {}.", site.tr("A link to a"), singular.to_lowercase())));
        put(&mut labels, "menu_name", s(&plural));
        let core_tax = matches!(entry.name.as_str(), "category" | "post_tag");
        put(&mut labels, "name_admin_bar", s(if core_tax { &entry.name } else { &singular }));
        if core_tax {
            put(&mut labels, "template_name", s(&format!("{singular} {}", site.tr("Archives"))));
        }
        Value::Object(labels)
    }

    fn prepare_taxonomy_edit(&self, entry: &TaxonomyEntry) -> Value {
        let caps = match entry.name.as_str() {
            "category" => json!({"manage_terms": "manage_categories", "edit_terms": "edit_categories", "delete_terms": "delete_categories", "assign_terms": "assign_categories"}),
            "post_tag" => json!({"manage_terms": "manage_post_tags", "edit_terms": "edit_post_tags", "delete_terms": "delete_post_tags", "assign_terms": "assign_post_tags"}),
            "nav_menu" => json!({"manage_terms": "edit_theme_options", "edit_terms": "edit_theme_options", "delete_terms": "edit_theme_options", "assign_terms": "edit_theme_options"}),
            _ => json!({"manage_terms": "manage_categories", "edit_terms": "manage_categories", "delete_terms": "manage_categories", "assign_terms": "edit_posts"}),
        };
        let mut data = Map::new();
        data.insert("name".into(), s(&entry.label));
        data.insert("slug".into(), s(&entry.name));
        data.insert("capabilities".into(), caps);
        data.insert("description".into(), s(&entry.description));
        data.insert("labels".into(), self.taxonomy_labels(entry));
        data.insert("types".into(), json!(entry.types));
        data.insert("show_cloud".into(), Value::Bool(entry.name != "nav_menu" && entry.name != "wp_pattern_category" || entry.name == "wp_pattern_category" && false));
        data.insert("hierarchical".into(), Value::Bool(entry.hierarchical));
        data.insert("rest_base".into(), s(&entry.rest_base));
        data.insert("rest_namespace".into(), s("wp/v2"));
        data.insert("visibility".into(), json!({"public": entry.name != "nav_menu" && entry.name != "wp_pattern_category" || entry.name == "wp_pattern_category" && false, "publicly_queryable": entry.name == "category" || entry.name == "post_tag", "show_admin_column": entry.name == "category" || entry.name == "post_tag", "show_in_nav_menus": entry.name == "category" || entry.name == "post_tag", "show_in_quick_edit": true, "show_ui": true}));
        let mut links = Links::new();
        links.add("collection", vec![], self.rest_url("wp/v2/taxonomies"));
        links.add("wp:items", vec![], self.rest_url(&format!("wp/v2/{}", entry.rest_base)));
        data.insert("_links".into(), links.finish());
        Value::Object(data)
    }

    pub(crate) fn taxonomies_list_edit(&self, args: &Args) -> Outcome {
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        let wanted = args.str("type").filter(|t| !t.is_empty());
        let mut data = Map::new();
        for entry in self.taxonomy_entries() {
            if wanted.as_ref().is_some_and(|t| !entry.types.iter().any(|x| x == t)) {
                continue;
            }
            if context == "edit" {
                if !self.can("manage_categories") && !self.can("edit_posts") {
                    continue;
                }
                data.insert(entry.name.clone(), self.prepare_taxonomy_edit(&entry));
            } else {
                data.insert(entry.name.clone(), self.prepare_taxonomy(&entry, &context));
            }
        }
        Ok(Resp::item(if data.is_empty() { rustpress_blocks::php_json::empty_object() } else { Value::Object(data) }))
    }

    pub(crate) fn taxonomy_get_edit(&self, name: &str, args: &Args) -> Outcome {
        let Some(entry) = self.taxonomy_entries().into_iter().find(|e| e.name == name) else {
            return Err(RestError::new("rest_taxonomy_invalid", "Invalid taxonomy.", 404).after_permission_check());
        };
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if context == "edit" {
            return Ok(Resp::item(self.prepare_taxonomy_edit(&entry)));
        }
        Ok(Resp::item(self.prepare_taxonomy(&entry, &context)))
    }
}
