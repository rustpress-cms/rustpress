//! `targetHints` of the `self` links in the answers to a signed-in user: the methods they may use on the item.
//! The read-only routes write `["GET"]`; here the items of a list get the hints WordPress works out from the
//! permissions of the controller of each route.

use serde_json::{Value, json};

use crate::Rest;

impl Rest<'_> {
    /// Replaces the `["GET"]` hints of the `self` links in `data` (an item, a list of items, their `_embedded`).
    pub(crate) fn refine_hints<'a>(&'a self, data: &'a mut Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            match data {
                Value::Array(items) => {
                    self.prime_hint_targets(items).await;
                    for item in items {
                        self.refine_hints(item).await;
                    }
                }
                Value::Object(map) => {
                    let href = map
                        .get("_links")
                        .and_then(|l| l.pointer("/self/0"))
                        .filter(|s| s.pointer("/targetHints/allow") == Some(&json!(["GET"])))
                        .and_then(|s| s.get("href"))
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                    if let Some(href) = href
                        && let Some(allow) = self.hint_allow(&href).await
                        && let Some(link) = map.get_mut("_links").and_then(|l| l.pointer_mut("/self/0")).and_then(Value::as_object_mut)
                    {
                        // Nothing allowed leaves no hint at all.
                        if allow.is_empty() {
                            link.remove("targetHints");
                        } else {
                            link.insert("targetHints".into(), json!({"allow": allow}));
                        }
                    }
                    if let Some(embedded) = map.get_mut("_embedded").and_then(Value::as_object_mut) {
                        for (_, value) in embedded.iter_mut() {
                            self.refine_hints(value).await;
                        }
                    }
                }
                _ => {}
            }
        })
    }

    /// The comments of a list and their posts in two queries instead of two per item.
    async fn prime_hint_targets(&self, items: &[Value]) {
        let ids: Vec<u64> = items
            .iter()
            .filter_map(|item| item.pointer("/_links/self/0/href").and_then(Value::as_str))
            .filter_map(|href| href.split("/wp/v2/comments/").nth(1)?.split(['?', '&', '/']).next()?.parse().ok())
            .collect();
        if ids.is_empty() {
            return;
        }
        let db = &self.site.db;
        if db.prime_comments(&ids).await.is_err() {
            return;
        }
        let mut posts = Vec::new();
        for id in &ids {
            if let Ok(Some(comment)) = db.comment_any(*id).await {
                posts.push(comment.comment_post_id);
            }
        }
        let _ = db.prime_posts(&posts).await;
    }

    /// The methods the user may use on the item an `href` names.
    async fn hint_allow(&self, href: &str) -> Option<Vec<&'static str>> {
        let rest = href.split("/wp/v2/").nth(1)?;
        let rest = rest.split(['?', '&']).next()?;
        let (base, id) = rest.split_once('/')?;
        let id: u64 = id.parse().ok()?;
        let db = &self.site.db;
        if base == "users" {
            return Some(self.user_allow(id));
        }
        if base == "comments" {
            // `get_item_permissions_check()` and `check_edit_permission()`; nothing allowed is no hint at all.
            let comment = db.comment_any(id).await.ok().flatten()?;
            let post = db.post(comment.comment_post_id).await.ok().flatten();
            let params = crate::params::Params::parse("");
            let mut allow = Vec::new();
            let readable = self.check_read_comment(&comment, &params, true).await.unwrap_or(false);
            let post_readable = match &post {
                Some(post) => self.check_read_post_permission(post, &params, false, true).await,
                None => true,
            };
            if readable && post_readable {
                allow.push("GET");
            }
            let editable = self.user().is_some_and(|u| u.id != 0 && (u.can("moderate_comments") || u.can_edit_comment(post.as_ref())));
            if editable {
                allow.extend(["POST", "PUT", "PATCH", "DELETE"]);
            }
            return Some(allow);
        }
        if let Some(tax) = self.site.types.taxonomies.iter().filter(|t| t.show_in_rest).find_map(|t| (t.rest_base() == base).then(|| self.tax(&t.name))).flatten().or_else(|| match base {
            "categories" => self.tax("category"),
            "tags" => self.tax("post_tag"),
            _ => None,
        }) {
            // The item was just read: the ID of its link is a term of the taxonomy.
            let mut allow = vec!["GET"];
            if self.can("manage_categories") {
                allow.extend(["POST", "PUT", "PATCH"]);
                // The default category cannot be deleted.
                let default_category = self.site.options.u64("default_category", 1);
                if !(tax.name == "category" && id == default_category) {
                    allow.push("DELETE");
                }
            }
            return Some(allow);
        }
        let known = matches!(base, "posts" | "pages" | "media") || self.site.types.post_types.iter().any(|t| t.show_in_rest && t.rest_base() == base);
        if known {
            let post = db.post(id).await.ok().flatten()?;
            let mut allow = vec!["GET"];
            if self.can_edit_post(&post) {
                allow.extend(["POST", "PUT", "PATCH"]);
            }
            if self.can_delete_post(&post) {
                allow.push("DELETE");
            }
            return Some(allow);
        }
        None
    }
}
