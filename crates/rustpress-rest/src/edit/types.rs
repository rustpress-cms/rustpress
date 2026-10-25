//! `wp/v2/types` and `wp/v2/taxonomies` in the `edit` context.

use serde_json::{Map, Value, json};

use super::Args;
use crate::error::RestError;
use crate::meta::{TaxonomyEntry, TypeEntry};
use crate::util::Links;
use crate::{Outcome, Resp, Rest};

/// The labels, capabilities, supports and visibility of the internal types (`wp_template`, `wp_block`, ...).
fn core_types() -> &'static Value {
    static DATA: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/core_types.json")).unwrap_or(Value::Null))
}

fn s(text: &str) -> Value {
    Value::from(text.to_owned())
}

impl Rest<'_> {
    /// `get_post_type_labels()`: the labels of a post type that gave only its name and singular name.
    fn type_labels(&self, entry: &TypeEntry, singular: &str, plural: &str) -> Value {
        let site = self.site;
        let hierarchical = entry.hierarchical;
        let pick = |post: &str, page: &str| s(&site.tr(if hierarchical { page } else { post }));
        let pick_x = |post: &str, page: &str, context: &str| s(&site.trx(if hierarchical { page } else { post }, context));
        let mut labels = Map::new();
        labels.insert("name".into(), s(plural));
        labels.insert("singular_name".into(), s(singular));
        labels.insert("add_new".into(), s(&site.trx("Add", if hierarchical { "page" } else { "post" })));
        labels.insert("add_new_item".into(), pick("Add Post", "Add Page"));
        labels.insert("edit_item".into(), pick("Edit Post", "Edit Page"));
        labels.insert("new_item".into(), pick("New Post", "New Page"));
        labels.insert("view_item".into(), pick("View Post", "View Page"));
        labels.insert("view_items".into(), pick("View Posts", "View Pages"));
        labels.insert("search_items".into(), pick("Search Posts", "Search Pages"));
        labels.insert("not_found".into(), pick("No posts found.", "No pages found."));
        labels.insert("not_found_in_trash".into(), pick("No posts found in Trash.", "No pages found in Trash."));
        labels.insert("parent_item_colon".into(), if hierarchical { s(&site.tr("Parent Page:")) } else { Value::Null });
        labels.insert("all_items".into(), pick("All Posts", "All Pages"));
        labels.insert("archives".into(), pick("Post Archives", "Page Archives"));
        labels.insert("attributes".into(), pick("Post Attributes", "Page Attributes"));
        labels.insert("insert_into_item".into(), pick("Insert into post", "Insert into page"));
        labels.insert("uploaded_to_this_item".into(), pick("Uploaded to this post", "Uploaded to this page"));
        labels.insert("featured_image".into(), s(&site.trx("Featured image", "post")));
        labels.insert("set_featured_image".into(), s(&site.trx("Set featured image", "post")));
        labels.insert("remove_featured_image".into(), s(&site.trx("Remove featured image", "post")));
        labels.insert("use_featured_image".into(), s(&site.trx("Use as featured image", "post")));
        labels.insert("filter_items_list".into(), pick("Filter posts list", "Filter pages list"));
        labels.insert("filter_by_date".into(), s(&site.tr("Filter by date")));
        labels.insert("items_list_navigation".into(), pick("Posts list navigation", "Pages list navigation"));
        labels.insert("items_list".into(), pick("Posts list", "Pages list"));
        labels.insert("item_published".into(), pick("Post published.", "Page published."));
        labels.insert("item_published_privately".into(), pick("Post published privately.", "Page published privately."));
        labels.insert("item_reverted_to_draft".into(), pick("Post reverted to draft.", "Page reverted to draft."));
        labels.insert("item_trashed".into(), pick("Post trashed.", "Page trashed."));
        labels.insert("item_scheduled".into(), pick("Post scheduled.", "Page scheduled."));
        labels.insert("item_updated".into(), pick("Post updated.", "Page updated."));
        labels.insert("item_link".into(), pick_x("Post Link", "Page Link", "navigation link block title"));
        labels.insert("item_link_description".into(), pick_x("A link to a post.", "A link to a page.", "navigation link block description"));
        labels.insert("menu_name".into(), s(plural));
        labels.insert("name_admin_bar".into(), s(singular));
        // `_get_custom_object_labels()`: what a registration without these labels falls back to.
        if !matches!(entry.name.as_str(), "post" | "page") {
            labels.insert("all_items".into(), s(plural));
            labels.insert("archives".into(), s(plural));
            labels.insert("template_name".into(), s(&rustpress_core::i18n::sprintf(&site.tr("Single item: %s"), &[singular])));
        }
        Value::Object(labels)
    }

    /// The English labels of an internal type, translated.
    fn translate_labels(&self, labels: &Value) -> Value {
        match labels {
            Value::Object(map) => Value::Object(map.iter().map(|(k, v)| (k.clone(), v.as_str().map_or_else(|| v.clone(), |text| Value::from(self.site.tr(text))))).collect()),
            other => other.clone(),
        }
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
            // A type of a plugin: what its registration listed, in that order; `autosave` follows `editor`.
            let listed: Vec<String> = self.site.types.post_type(name).and_then(|t| t.supports.clone()).unwrap_or_else(|| vec!["title".into(), "editor".into()]);
            for feature in &listed {
                map.insert(feature.clone(), Value::Bool(true));
            }
            if map.contains_key("editor") && !map.contains_key("autosave") {
                map.insert("autosave".into(), Value::Bool(true));
            }
            return Value::Object(map);
        }
        for feature in features {
            map.insert((*feature).to_owned(), Value::Bool(true));
        }
        if let Some(Value::Bool(true)) = map.get("editor") {
            map.insert("editor".into(), json!([{"notes": true}]));
        }
        Value::Object(map)
    }

    pub(super) fn prepare_type_edit(&self, entry: &TypeEntry) -> Value {
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
        // The internal types register their own labels, capabilities and supports.
        if let Some(Value::Object(own)) = core_types().get(&entry.name) {
            for (key, value) in own {
                let value = if key == "labels" { self.translate_labels(value) } else { value.clone() };
                data.insert(key.clone(), value);
            }
        }
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

    /// `get_taxonomy_labels()`: the defaults of WordPress for a taxonomy that gave only its name.
    fn taxonomy_labels(&self, entry: &TaxonomyEntry) -> Value {
        let site = self.site;
        let (singular, plural) = match entry.name.as_str() {
            "category" => (site.trx("Category", "taxonomy singular name"), entry.label.clone()),
            "post_tag" => (site.trx("Tag", "taxonomy singular name"), entry.label.clone()),
            other => (site.types.taxonomy(other).map_or_else(|| entry.label.clone(), |t| t.singular_label()), entry.label.clone()),
        };
        let hier = entry.hierarchical;
        let core_tax = matches!(entry.name.as_str(), "category" | "post_tag");
        let pick = |tag: &str, category: &str| s(&site.tr(if hier { category } else { tag }));
        let pick_opt = |tag: Option<&str>, category: Option<&str>| match if hier { category } else { tag } {
            Some(text) => s(&site.tr(text)),
            None => Value::Null,
        };
        let pick_x = |tag: &str, category: &str, context: &str| s(&site.trx(if hier { category } else { tag }, context));
        let mut labels = Map::new();
        let mut put = |key: &str, value: Value| {
            labels.insert(key.to_owned(), value);
        };
        put("name", s(&plural));
        put("singular_name", s(&singular));
        put("search_items", pick("Search Tags", "Search Categories"));
        put("popular_items", pick_opt(Some("Popular Tags"), None));
        put("all_items", if core_tax { pick("All Tags", "All Categories") } else { s(&plural) });
        put("parent_item", pick_opt(None, Some("Parent Category")));
        put("parent_item_colon", pick_opt(None, Some("Parent Category:")));
        put("name_field_description", s(&site.tr("The name is how it appears on your site.")));
        put("slug_field_description", s(&site.tr("The &#8220;slug&#8221; is the URL-friendly version of the name. It is usually all lowercase and contains only letters, numbers, and hyphens.")));
        put("parent_field_description", pick_opt(None, Some("Assign a parent term to create a hierarchy. The term Jazz, for example, would be the parent of Bebop and Big Band.")));
        put("desc_field_description", s(&site.tr("The description is not prominent by default; however, some themes may show it.")));
        put("edit_item", pick("Edit Tag", "Edit Category"));
        put("view_item", pick("View Tag", "View Category"));
        put("update_item", pick("Update Tag", "Update Category"));
        put("add_new_item", pick("Add Tag", "Add Category"));
        put("new_item_name", pick("New Tag Name", "New Category Name"));
        put("separate_items_with_commas", pick_opt(Some("Separate tags with commas"), None));
        put("add_or_remove_items", pick_opt(Some("Add or remove tags"), None));
        put("choose_from_most_used", pick_opt(Some("Choose from the most used tags"), None));
        put("not_found", pick("No tags found.", "No categories found."));
        put("no_terms", pick("No tags", "No categories"));
        put("filter_by_item", pick_opt(None, Some("Filter by category")));
        put("items_list_navigation", pick("Tags list navigation", "Categories list navigation"));
        put("items_list", pick("Tags list", "Categories list"));
        put("most_used", pick_x("Most Used", "Most Used", if hier { "categories" } else { "tags" }));
        put("back_to_items", pick("&larr; Go to Tags", "&larr; Go to Categories"));
        put("item_link", pick_x("Tag Link", "Category Link", "navigation link block title"));
        put("item_link_description", pick_x("A link to a tag.", "A link to a category.", "navigation link block description"));
        put("menu_name", s(&plural));
        put("name_admin_bar", s(if core_tax { &entry.name } else { &singular }));
        if !core_tax {
            put("archives", s(&plural));
        }
        put("template_name", s(&rustpress_core::i18n::sprintf(&site.tr("%s Archives"), &[&singular])));
        Value::Object(labels)
    }

    fn prepare_taxonomy_edit(&self, entry: &TaxonomyEntry) -> Value {
        let caps = match entry.name.as_str() {
            "category" => json!({"manage_terms": "manage_categories", "edit_terms": "edit_categories", "delete_terms": "delete_categories", "assign_terms": "assign_categories"}),
            "post_tag" => json!({"manage_terms": "manage_post_tags", "edit_terms": "edit_post_tags", "delete_terms": "delete_post_tags", "assign_terms": "assign_post_tags"}),
            "nav_menu" => json!({"manage_terms": "edit_theme_options", "edit_terms": "edit_theme_options", "delete_terms": "edit_theme_options", "assign_terms": "edit_theme_options"}),
            _ => json!({"manage_terms": "manage_categories", "edit_terms": "manage_categories", "delete_terms": "manage_categories", "assign_terms": "edit_posts"}),
        };
        let custom_tax = self.site.types.taxonomy(&entry.name);
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
        data.insert("visibility".into(), json!({"public": entry.name != "nav_menu" && entry.name != "wp_pattern_category" || entry.name == "wp_pattern_category" && false, "publicly_queryable": entry.name == "category" || entry.name == "post_tag" || custom_tax.is_some_and(|t| t.queryable), "show_admin_column": entry.name == "category" || entry.name == "post_tag", "show_in_nav_menus": entry.name == "category" || entry.name == "post_tag" || custom_tax.is_some_and(|t| t.public), "show_in_quick_edit": true, "show_ui": true}));
        if let Some(Value::Object(own)) = core_types().pointer(&format!("/__taxonomies/{}", entry.name)) {
            for (key, value) in own {
                let value = if key == "labels" { self.translate_labels(value) } else { value.clone() };
                data.insert(key.clone(), value);
            }
        }
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
