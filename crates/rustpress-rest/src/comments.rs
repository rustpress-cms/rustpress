//! `wp/v2/comments` (`WP_REST_Comments_Controller`) for anonymous visitors:
//! approved comments of published posts without a password.

use rustpress_blocks::content::comment_text;
use rustpress_db::{Comment, CommentOrder, CommentQuery};
use serde_json::{Map, Value, json};

use crate::error::RestError;
use crate::params::Params;
use crate::util::{LinkParams, Links, link_header, rfc3339};
use crate::{Outcome, Resp, Rest, Result};

const EMBED_FIELDS: &[&str] = &["id", "parent", "author", "author_name", "author_url", "date", "content", "link", "type", "author_avatar_urls", "_links"];

impl Rest<'_> {
    pub(crate) async fn comments_collection(&self, params: &Params) -> Result<Outcome> {
        macro_rules! try_param {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                }
            };
        }
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit comments.", 401)));
        }
        // Parameters that only moderators may use.
        let forbidden: Vec<&str> = ["author", "author_exclude", "author_email"]
            .into_iter()
            .filter(|p| params.has(p))
            .chain(params.get("type").filter(|t| *t != "comment").map(|_| "type"))
            .chain(params.get("status").filter(|s| *s != "approve").map(|_| "status"))
            .collect();
        if !forbidden.is_empty() {
            return Ok(Err(RestError::new("rest_forbidden_param", &format!("Query parameter not permitted: {}", forbidden.join(", ")), 401)));
        }
        let per_page = try_param!(params.integer("per_page", 10, 1, Some(100)));
        let page = try_param!(params.integer("page", 1, 1, None));
        let offset = try_param!(params.integer("offset", 0, 0, None));
        let order = try_param!(params.one_of("order", &["asc", "desc"], "desc"));
        let orderby = try_param!(params.one_of("orderby", &["date", "date_gmt", "id", "include", "post", "parent", "type"], "date_gmt"));
        let post_in = params.id_list("post").unwrap_or_default();
        for id in &post_in {
            if let Some(post) = self.site.db.post(*id).await?
                && (post.post_status != "publish" || !post.post_password.is_empty()) {
                    return Ok(Err(RestError::new("rest_cannot_read_post", "Sorry, you are not allowed to read the post for this comment.", 401)));
                }
        }

        let mut query = CommentQuery {
            ids: params.id_list("include").unwrap_or_default(),
            exclude_ids: params.id_list("exclude").unwrap_or_default(),
            post_in,
            parent_in: params.id_list("parent").unwrap_or_default(),
            parent_not_in: params.id_list("parent_exclude").unwrap_or_default(),
            search: params.get("search").filter(|s| !s.is_empty()).map(str::to_owned),
            descending: order == "desc",
            limit: Some(per_page),
            offset,
            ..Default::default()
        };
        query.order_by = match orderby {
            "date" => CommentOrder::Date,
            "id" | "include" => CommentOrder::Id,
            "post" => CommentOrder::Post,
            "parent" => CommentOrder::Parent,
            _ => CommentOrder::DateGmt,
        };
        for (key, slot) in [("after", &mut query.after), ("before", &mut query.before)] {
            if let Some(value) = params.get(key) {
                match chrono::NaiveDateTime::parse_from_str(value.trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S") {
                    Ok(date) => *slot = Some(date.format("%Y-%m-%d %H:%M:%S").to_string()),
                    Err(_) => return Ok(Err(RestError::invalid_param(key, "Invalid date.", "rest_invalid_date"))),
                }
            }
        }

        let total = self.site.db.count_comments(&query).await?;
        let max_pages = total.div_ceil(per_page);
        if query.offset == 0 {
            query.offset = (page - 1) * per_page;
        }
        let comments = self.site.db.comments_query(&query).await?;
        let context = params.get("context").unwrap_or("view");
        let mut items = Vec::new();
        for comment in &comments {
            items.push(self.prepare_comment(comment, context).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.link = link_header(self, "wp/v2/comments", params, &LinkParams { arrays: &["author", "author_exclude", "exclude", "include", "parent", "parent_exclude", "post"], booleans: &[] }, page, max_pages);
        Ok(Ok(resp))
    }

    pub(crate) async fn comments_item(&self, id: &str, params: &Params) -> Result<Outcome> {
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        if params.get("context") == Some("edit") {
            return Ok(Err(RestError::new("rest_forbidden_context", "Sorry, you are not allowed to edit comments.", 401)));
        }
        let Some(comment) = self.site.db.comment(id).await? else {
            return Ok(Err(RestError::new("rest_comment_invalid_id", "Invalid comment ID.", 404)));
        };
        let readable = comment.comment_approved == "1" && matches!(comment.comment_type.as_str(), "" | "comment") && {
            let post = self.site.db.post(comment.comment_post_id).await?;
            post.is_some_and(|p| p.post_status == "publish" && p.post_password.is_empty())
        };
        if !readable {
            return Ok(Err(RestError::new("rest_cannot_read", "Sorry, you are not allowed to read this comment.", 401)));
        }
        let context = params.get("context").unwrap_or("view");
        Ok(Ok(Resp::item(self.prepare_comment(&comment, context).await?)))
    }

    async fn prepare_comment(&self, comment: &Comment, context: &str) -> Result<Value> {
        let post = self.site.db.post(comment.comment_post_id).await?;
        let link = match &post {
            Some(post) => format!("{}#comment-{}", self.site.permalink(post).await?, comment.comment_id),
            None => String::new(),
        };
        let mut data = Map::new();
        data.insert("id".into(), Value::from(comment.comment_id));
        data.insert("post".into(), Value::from(comment.comment_post_id));
        data.insert("parent".into(), Value::from(comment.comment_parent));
        data.insert("author".into(), Value::from(comment.user_id));
        data.insert("author_name".into(), Value::from(comment.comment_author.clone()));
        data.insert("author_url".into(), Value::from(comment.comment_author_url.clone()));
        data.insert("date".into(), Value::from(rfc3339(&comment.comment_date)));
        data.insert("date_gmt".into(), Value::from(rfc3339(&comment.comment_date_gmt)));
        data.insert("content".into(), json!({"rendered": comment_text(&comment.comment_content)}));
        data.insert("link".into(), Value::from(link));
        data.insert("status".into(), Value::from("approved"));
        let kind = if comment.comment_type.is_empty() { "comment" } else { comment.comment_type.as_str() };
        data.insert("type".into(), Value::from(kind));
        data.insert("author_avatar_urls".into(), self.avatar_urls(&comment.comment_author_email));
        data.insert("meta".into(), json!({"_wp_note_status": null}));

        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("wp/v2/comments/{}", comment.comment_id)));
        links.add("collection", vec![], self.rest_url("wp/v2/comments"));
        if comment.user_id != 0 {
            links.embeddable("author", self.rest_url(&format!("wp/v2/users/{}", comment.user_id)));
        }
        if let Some(post) = &post {
            let route = match post.post_type.as_str() {
                "page" => Some("pages"),
                "post" => Some("posts"),
                _ => None,
            };
            if let Some(route) = route {
                links.add("up", vec![("embeddable", Value::Bool(true)), ("post_type", Value::from(post.post_type.clone()))], self.rest_url(&format!("wp/v2/{route}/{}", post.id)));
            }
        }
        if comment.comment_parent != 0 {
            links.embeddable("in-reply-to", self.rest_url(&format!("wp/v2/comments/{}", comment.comment_parent)));
        }
        let children = self.site.db.count_comments(&CommentQuery { parent_in: vec![comment.comment_id], ..Default::default() }).await?;
        if children > 0 {
            links.embeddable("children", rustpress_core::urls::add_query_arg(&self.rest_url("wp/v2/comments"), "parent", &comment.comment_id.to_string()));
        }
        data.insert("_links".into(), links.finish());

        if context == "embed" {
            data.retain(|k, _| EMBED_FIELDS.contains(&k.as_str()));
        }
        Ok(Value::Object(data))
    }
}
