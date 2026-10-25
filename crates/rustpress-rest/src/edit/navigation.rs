//! `wp-block-editor/v1/navigation-fallback`: the navigation menu the Navigation block uses when none is chosen.

use rustpress_content::content::PostData;
use rustpress_db::{Order, OrderBy, PostQuery};
use serde_json::json;

use super::Args;
use crate::error::RestError;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

impl Rest<'_> {
    /// `WP_Navigation_Fallback::get_fallback()`: the newest published menu, else a new one with the page list.
    pub(super) async fn navigation_fallback(&self, args: &Args) -> Result<Outcome> {
        let message = self.site.tr("Sorry, you are not allowed to create Navigation Menus as this user.");
        if !(self.can("edit_theme_options") && self.can("edit_posts") && self.can("publish_posts")) {
            return Ok(Err(self.forbidden("rest_cannot_create", &message)));
        }
        if args.str("context").as_deref() == Some("edit") && !self.can("edit_posts") {
            return Ok(Err(self.forbidden("rest_forbidden_context", &self.site.tr("Sorry, you are not allowed to edit Navigation Menus as this user."))));
        }
        let query = PostQuery { post_types: vec!["wp_navigation".into()], statuses: vec!["publish".into()], order_by: OrderBy::Date, order: Order::Desc, ..Default::default() };
        let id = match self.site.db.posts(&query).await?.first() {
            Some(post) => post.id,
            None => {
                let content = match self.content() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(e)),
                };
                let data = PostData {
                    post_content: Some("<!-- wp:page-list /-->".into()),
                    post_title: Some(self.site.trx("Navigation", "Title of a Navigation menu")),
                    post_name: Some("navigation".into()),
                    post_status: Some("publish".into()),
                    post_type: Some("wp_navigation".into()),
                    ..Default::default()
                };
                match content.insert_post(&data).await {
                    Ok(id) => id,
                    Err(_) => return Ok(Err(RestError::new("no_fallback_menu", &self.site.tr("No fallback menu found."), 404))),
                }
            }
        };
        let mut links = Links::new();
        links.push_raw("self", json!({"embeddable": true, "href": self.rest_url(&format!("wp/v2/navigation/{id}")), "targetHints": {"allow": ["GET", "POST", "PUT", "PATCH", "DELETE"]}}));
        Ok(Ok(Resp::item(json!({"id": id, "_links": links.finish()}))))
    }
}
