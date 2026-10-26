//! `wp/v2/users` (`WP_REST_Users_Controller`) as an anonymous visitor sees it:
//! only authors with published content are listed.

use rustpress_db::User;
use serde_json::{Map, Value};

use crate::error::RestError;
use crate::params::Params;
use crate::util::{LinkParams, Links, link_header};
use crate::{Outcome, Resp, Rest, Result};

/// The post types with `show_in_rest` (`WP_REST_Users_Controller` lists their authors).
pub(crate) const AUTHOR_TYPES: &[&str] = &["post", "page", "attachment", "wp_block", "wp_template", "wp_template_part", "wp_navigation", "wp_font_family", "wp_font_face", "wp_global_styles"];

impl Rest<'_> {
    pub(crate) async fn users_collection(&self, params: &Params) -> Result<Outcome> {
        macro_rules! try_param {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                }
            };
        }
        // The arguments are validated before the permission check runs.
        let per_page = try_param!(params.integer("per_page", 10, 1, Some(100)));
        let page = try_param!(params.integer("page", 1, 1, None));
        let offset = try_param!(params.integer("offset", 0, 0, None));
        let order = try_param!(params.one_of("order", &["asc", "desc"], "asc"));
        let orderby = try_param!(params.one_of("orderby", &["id", "include", "name", "registered_date", "slug", "include_slugs", "email", "url"], "name"));
        let who = if params.has("who") { Some(try_param!(params.one_of("who", &["authors"], "authors"))) } else { None };
        for (param, noun) in [("roles", "role"), ("capabilities", "capability")] {
            if params.has(param) {
                return Ok(Err(RestError::new("rest_user_cannot_view", &format!("Sorry, you are not allowed to filter users by {noun}."), 401)));
            }
        }
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit users.", 401)));
        }
        if matches!(orderby, "email" | "registered_date") {
            return Ok(Err(RestError::new("rest_forbidden_orderby", "Sorry, you are not allowed to order users by this parameter.", 401)));
        }
        if who.is_some() {
            return Ok(Err(RestError::new("rest_forbidden_who", "Sorry, you are not allowed to query users by this parameter.", 401)));
        }
        let include = params.id_list("include").unwrap_or_default();
        let exclude = params.id_list("exclude").unwrap_or_default();
        let slugs = params.list("slug").unwrap_or_default();
        let search = params.get("search").filter(|s| !s.is_empty()).map(str::to_lowercase);

        let count = self.site.db.count_users_with_published_posts(AUTHOR_TYPES).await?;
        let mut users: Vec<User> = self.site.db.users_with_published_posts(AUTHOR_TYPES, count.max(1), 0).await?;
        users.retain(|u| (include.is_empty() || include.contains(&u.id)) && !exclude.contains(&u.id) && (slugs.is_empty() || slugs.contains(&u.user_nicename)));
        if let Some(search) = &search {
            users.retain(|u| [&u.user_login, &u.user_nicename, &u.display_name, &u.user_url].iter().any(|f| f.to_lowercase().contains(search.as_str())));
        }
        let descending = order == "desc";
        let direction = |o: std::cmp::Ordering| if descending { o.reverse() } else { o };
        match orderby {
            "id" => users.sort_by(|a, b| direction(a.id.cmp(&b.id))),
            "include" => users.sort_by_key(|u| include.iter().position(|i| *i == u.id)),
            "slug" => users.sort_by(|a, b| direction(a.user_nicename.to_lowercase().cmp(&b.user_nicename.to_lowercase())).then(a.id.cmp(&b.id))),
            "registered_date" => users.sort_by(|a, b| direction(a.user_registered.cmp(&b.user_registered)).then(a.id.cmp(&b.id))),
            "url" => users.sort_by(|a, b| direction(a.user_url.to_lowercase().cmp(&b.user_url.to_lowercase())).then(a.id.cmp(&b.id))),
            "email" => users.sort_by(|a, b| direction(a.user_email.to_lowercase().cmp(&b.user_email.to_lowercase())).then(a.id.cmp(&b.id))),
            _ => users.sort_by(|a, b| direction(a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase())).then(a.id.cmp(&b.id))),
        }

        let total = users.len() as u64;
        let max_pages = total.div_ceil(per_page);
        let start = if offset > 0 { offset } else { (page - 1).saturating_mul(per_page) } as usize;
        let context = params.get("context").unwrap_or("view");
        let mut items = Vec::new();
        for user in users.iter().skip(start).take(per_page as usize) {
            items.push(self.prepare_user(user, context).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.link = link_header(self, "wp/v2/users", params, &LinkParams { arrays: &["exclude", "include", "slug", "roles"], booleans: &[] }, page, max_pages);
        Ok(Ok(resp))
    }

    pub(crate) async fn users_item(&self, id: &str, params: &Params) -> Result<Outcome> {
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to list users.", 401)));
        }
        let Some(user) = self.site.db.user(id).await? else {
            return Ok(Err(RestError::new("rest_user_invalid_id", "Invalid user ID.", 404)));
        };
        let published = self.site.db.users_with_published_posts(AUTHOR_TYPES, 1_000_000, 0).await?;
        if !published.iter().any(|u| u.id == user.id) {
            return Ok(Err(RestError::new("rest_user_cannot_view", "Sorry, you are not allowed to list users.", 401)));
        }
        let context = params.get("context").unwrap_or("view");
        Ok(Ok(Resp::item(self.prepare_user(&user, context).await?)))
    }

    async fn prepare_user(&self, user: &User, context: &str) -> Result<Value> {
        let mut data = Map::new();
        data.insert("id".into(), Value::from(user.id));
        data.insert("name".into(), Value::from(user.display_name.clone()));
        data.insert("url".into(), Value::from(user.user_url.clone()));
        data.insert("description".into(), Value::from(self.site.db.user_meta(user.id, "description").await?.unwrap_or_default()));
        data.insert("link".into(), Value::from(self.site.author_url(user)));
        data.insert("slug".into(), Value::from(user.user_nicename.clone()));
        if self.site.show_avatars() {
            data.insert("avatar_urls".into(), self.avatar_urls(&user.user_email));
        }
        if context != "embed" {
            data.insert("meta".into(), Value::Array(Vec::new()));
        }
        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("wp/v2/users/{}", user.id)));
        links.add("collection", vec![], self.rest_url("wp/v2/users"));
        data.insert("_links".into(), links.finish());
        Ok(Value::Object(data))
    }
}
