//! Navigation menus: `wp_update_nav_menu_object()`, `wp_update_nav_menu_item()`, `wp_delete_nav_menu()` and
//! `set_theme_mod( 'nav_menu_locations', ... )`.

use super::terms::TermData;
use super::{Content, ContentError, PostData, Result, TermRef};
use crate::sanitize;
use rustpress_core::formatting::{esc_html, esc_url_raw, sanitize_html_class};
use rustpress_db::{Order, OrderBy, PostQuery};
use rustpress_phpser::{Key, Value};

/// The fields of `wp_update_nav_menu_item( $menu_id, $id, $data )`, as strings as in a form.
#[derive(Debug, Clone, Default)]
pub struct MenuItemData {
    pub object_id: String,
    pub object: String,
    pub parent_id: String,
    pub position: String,
    pub kind: String,
    pub title: String,
    pub url: String,
    pub description: String,
    pub attr_title: String,
    pub target: String,
    pub classes: String,
    pub xfn: String,
    pub status: String,
}

/// `(int) $value` of a form field.
fn int(value: &str) -> i64 {
    let value = value.trim_start();
    let (negative, digits) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value),
    };
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    let number = digits[..end].parse::<i64>().unwrap_or(0);
    if negative { -number } else { number }
}

impl Content<'_> {
    /// `wp_update_nav_menu_object()`: a new menu for `menu_id` 0 or a menu that is gone. The error is the message.
    pub async fn update_nav_menu_object(&self, menu_id: u64, name: &str) -> std::result::Result<u64, String> {
        let terms = self.db.terms("nav_menu").await.map_err(|e| e.to_string())?;
        let clean = esc_html(name);
        if terms.iter().any(|term| term.name == clean && term.term_id != menu_id) {
            return Err(rustpress_core::i18n::sprintf(&self.site.tr("The menu name %s conflicts with another menu name. Please try another."), &[&format!("<strong>{clean}</strong>")]));
        }
        let data = TermData { name: name.to_owned(), ..Default::default() };
        let message = |error: ContentError| match error {
            ContentError::Message(message) => message,
            other => other.to_string(),
        };
        if terms.iter().any(|term| term.term_id == menu_id) {
            self.update_term(menu_id, "nav_menu", &data).await.map_err(message)?;
            Ok(menu_id)
        } else {
            self.insert_term("nav_menu", &data).await.map_err(message)
        }
    }

    /// `wp_get_nav_menu_items( $menu, [ 'post_status' => 'publish,draft' ] )`: the last `menu_order`, numbered.
    async fn menu_item_count(&self, menu_id: u64) -> Result<usize> {
        let Some(menu) = self.db.term_by_id(Some("nav_menu"), menu_id).await? else { return Ok(0) };
        if menu.count <= 0 {
            return Ok(0);
        }
        let query = PostQuery {
            post_types: vec!["nav_menu_item".into()],
            statuses: vec!["publish".into(), "draft".into()],
            tax_groups: vec![vec![menu.term_taxonomy_id]],
            order_by: OrderBy::MenuOrder,
            order: Order::Asc,
            ..PostQuery::default()
        };
        Ok(self.db.posts(&query).await?.len())
    }

    /// `wp_update_nav_menu_item()`.
    pub async fn update_nav_menu_item(&self, menu_id: u64, item_id: u64, data: &MenuItemData) -> std::result::Result<u64, String> {
        let tr = |s: &str| self.site.tr(s);
        let text = |error: ContentError| match error {
            ContentError::Message(message) => message,
            other => other.to_string(),
        };
        let existing = if item_id != 0 { self.db.post(item_id).await.map_err(|e| e.to_string())? } else { None };
        if item_id != 0 && !existing.as_ref().is_some_and(|post| post.post_type == "nav_menu_item") {
            return Err(tr("The given object ID is not that of a menu item."));
        }
        let menu = if menu_id != 0 { self.db.term_by_id(Some("nav_menu"), menu_id).await.map_err(|e| e.to_string())? } else { None };
        if menu_id != 0 && menu.is_none() {
            return Err(tr("Invalid menu ID."));
        }
        let kind = if data.kind.is_empty() { "custom".to_owned() } else { data.kind.clone() };
        let mut position = int(&data.position);
        if menu_id == 0 {
            position = 1;
        } else if position == 0 {
            let count = self.menu_item_count(menu_id).await.map_err(text)?;
            // `1 + $last_item->menu_order`, or `count()` of the empty list.
            position = if count > 0 { 1 + count as i64 } else { 0 };
        }
        let mut original_parent = existing.as_ref().map_or(0, |post| post.post_parent);
        let mut url = data.url.clone();
        let mut title = data.title.clone();
        let mut description = data.description.clone();
        if kind == "custom" {
            url = rustpress_core::formatting::php_trim(&url).to_owned();
        } else {
            url = String::new();
            let object_id = int(&data.object_id).unsigned_abs();
            let mut original_title = None;
            match kind.as_str() {
                "taxonomy" => {
                    if let Some(term) = self.db.term_by_id(Some(&data.object), object_id).await.map_err(|e| e.to_string())? {
                        original_parent = term.parent;
                        original_title = Some(term.name);
                    }
                }
                "post_type" => {
                    if let Some(post) = self.db.post(object_id).await.map_err(|e| e.to_string())? {
                        original_parent = post.post_parent;
                        original_title = Some(post.post_title);
                    }
                }
                _ => {}
            }
            if original_title.as_deref() == Some(title.as_str()) {
                title.clear();
            }
            // "Hack to get wp to create a post object when too many properties are empty."
            if title.is_empty() && description.is_empty() {
                description = " ".to_owned();
            }
        }
        let update = item_id != 0;
        let mut post = PostData {
            menu_order: Some(position as i32),
            post_content: Some(description),
            post_excerpt: Some(data.attr_title.clone()),
            post_parent: Some(original_parent),
            post_title: Some(title),
            post_type: Some("nav_menu_item".into()),
            ping_status: Some("closed".into()),
            comment_status: Some("closed".into()),
            ..Default::default()
        };
        let mut id = item_id;
        if !update {
            post.post_status = Some(if data.status == "publish" { "publish".into() } else { "draft".into() });
            post.post_author = Some(self.user.id);
            id = self.insert_post(&post).await.map_err(text)?;
        }
        if menu_id != 0 {
            let member = self.db.terms_for_post(id, Some("nav_menu")).await.map_err(|e| e.to_string())?.iter().any(|term| term.term_id == menu_id);
            if !update || !member {
                self.set_object_terms(id, &[TermRef::Id(menu_id)], "nav_menu", false).await.map_err(text)?;
            }
        }
        let mut object_id = int(&data.object_id).to_string();
        let mut object = data.object.clone();
        if kind == "custom" {
            object_id = id.to_string();
            object = "custom".to_owned();
        }
        let mut parent = int(&data.parent_id);
        if parent == id as i64 {
            parent = 0;
        }
        let writer = self.writer;
        let error = |e: rustpress_db::Error| e.to_string();
        writer.update_post_meta(id, "_menu_item_type", &sanitize::key(&kind)).await.map_err(error)?;
        writer.update_post_meta(id, "_menu_item_menu_item_parent", &parent.to_string()).await.map_err(error)?;
        writer.update_post_meta(id, "_menu_item_object_id", &object_id).await.map_err(error)?;
        writer.update_post_meta(id, "_menu_item_object", &sanitize::key(&object)).await.map_err(error)?;
        writer.update_post_meta(id, "_menu_item_target", &sanitize::key(&data.target)).await.map_err(error)?;
        let classes: Vec<(Key, Value)> = data.classes.split(' ').enumerate().map(|(index, class)| (Key::Int(index as i64), Value::Str(sanitize_html_class(class).into_bytes()))).collect();
        let classes = String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(classes))).into_owned();
        writer.update_post_meta(id, "_menu_item_classes", &classes).await.map_err(error)?;
        let xfn = data.xfn.split(' ').map(sanitize_html_class).collect::<Vec<_>>().join(" ");
        writer.update_post_meta(id, "_menu_item_xfn", &xfn).await.map_err(error)?;
        writer.update_post_meta(id, "_menu_item_url", &esc_url_raw(&url)).await.map_err(error)?;
        if menu_id == 0 {
            writer.update_post_meta(id, "_menu_item_orphaned", &self.now.to_string()).await.map_err(error)?;
        } else {
            writer.delete_post_meta(id, "_menu_item_orphaned").await.map_err(error)?;
        }
        if update {
            post.id = Some(id);
            post.post_status = Some(if data.status == "draft" { "draft".into() } else { "publish".into() });
            self.insert_post(&post).await.map_err(text)?;
            // `_update_term_count_on_transition_post_status()`.
            let menus: Vec<u64> = self.db.terms_for_post(id, Some("nav_menu")).await.map_err(|e| e.to_string())?.into_iter().map(|term| term.term_taxonomy_id).collect();
            self.update_term_counts(&menus).await.map_err(text)?;
        }
        Ok(id)
    }

    /// `set_theme_mod( 'nav_menu_locations', $locations )`.
    pub async fn set_nav_menu_locations(&self, locations: &[(String, i64)]) -> Result<()> {
        let name = format!("theme_mods_{}", self.site.settings.stylesheet);
        let stored = self.db.option(&name).await?;
        let mut mods = match stored.as_deref().map(rustpress_core::options::maybe_unserialize) {
            Some(Value::Array(items)) => items,
            _ => Vec::new(),
        };
        let list: Vec<(Key, Value)> = locations.iter().map(|(key, id)| (Key::Str(key.clone().into_bytes()), Value::Int(*id))).collect();
        let key = Key::Str(b"nav_menu_locations".to_vec());
        match mods.iter_mut().find(|(k, _)| *k == key) {
            Some((_, value)) => *value = Value::Array(list),
            None => mods.push((key, Value::Array(list))),
        }
        let serialized = String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(mods))).into_owned();
        self.writer.update_option(&name, &serialized).await?;
        Ok(())
    }

    /// `wp_delete_nav_menu()`, with the locations it leaves.
    pub async fn delete_nav_menu(&self, menu_id: u64, locations: &[(String, i64)]) -> Result<bool> {
        let Some(menu) = self.db.term_by_id(Some("nav_menu"), menu_id).await? else { return Ok(false) };
        let query = PostQuery {
            post_types: vec!["nav_menu_item".into()],
            statuses: ["publish", "draft", "pending", "private", "future", "trash", "auto-draft"].map(str::to_owned).to_vec(),
            tax_groups: vec![vec![menu.term_taxonomy_id]],
            ..PostQuery::default()
        };
        for item in self.db.posts(&query).await? {
            self.delete_post(item.id, false).await?;
        }
        let result = self.delete_term(menu_id, "nav_menu").await?;
        let left: Vec<(String, i64)> = locations.iter().map(|(key, id)| (key.clone(), if *id == menu_id as i64 { 0 } else { *id })).collect();
        self.set_nav_menu_locations(&left).await?;
        Ok(result)
    }
}
