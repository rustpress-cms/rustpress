//! `wp/v2/comments` (`WP_REST_Comments_Controller`): the comments a visitor or the signed-in user may read, in
//! the view, embed and edit contexts. The writes are in `edit/comments.rs`.

use rustpress_blocks::content::comment_text;
use rustpress_db::{Comment, CommentOrder, CommentQuery, Post};
use serde_json::{Map, Value, json};

use crate::error::RestError;
use crate::params::Params;
use crate::util::{LinkParams, Links, link_header, rfc3339};
use crate::{Outcome, Resp, Rest, Result};

const EMBED_FIELDS: &[&str] = &["id", "parent", "author", "author_name", "author_url", "date", "content", "link", "type", "author_avatar_urls", "_links"];

/// The fields of the edit context only.
const EDIT_FIELDS: &[&str] = &["author_email", "author_ip", "author_user_agent"];

/// `sanitize_key()`.
fn sanitize_key(value: &str) -> String {
    value.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')).collect()
}

/// Whether the post type's editor supports notes (`post` and `page`).
fn supports_notes(post_type: &str) -> bool {
    matches!(post_type, "post" | "page")
}

impl Rest<'_> {
    /// `get_items_permissions_check()` and `get_items()`.
    pub(crate) async fn comments_collection(&self, params: &Params) -> Result<Outcome> {
        macro_rules! try_param {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => return Ok(Err(e)),
                }
            };
        }
        let status = params.get("status").map_or_else(|| "approve".to_owned(), sanitize_key);
        let kind = params.get("type").map_or_else(|| "comment".to_owned(), sanitize_key);
        let is_note = kind == "note";
        let is_edit_context = params.get("context") == Some("edit");
        // `0` is a value here: the comments without a post.
        let posts_asked = params.parent_list("post").unwrap_or_default();
        let post_in = posts_asked.clone();
        let moderator = self.can("moderate_comments");

        // Parameters that only those who may edit posts may use.
        let forbidden = || -> Vec<&str> {
            ["author", "author_exclude", "author_email"]
                .into_iter()
                .filter(|p| params.get(p).is_some_and(|v| !v.is_empty()) || params.id_list(p).is_some_and(|l| !l.is_empty()))
                .collect::<Vec<_>>()
                .into_iter()
                .chain((kind != "comment").then_some("type"))
                .chain((status != "approve").then_some("status"))
                .collect()
        };
        for id in &posts_asked {
            let post = if *id == 0 { None } else { self.site.db.post(*id).await? };
            if let Some(post) = &post
                && *id != 0
                && !self.check_read_post_permission(post, params, true, false).await
            {
                return Ok(Err(self.forbidden("rest_cannot_read_post", "Sorry, you are not allowed to read the post for this comment.")));
            } else if *id == 0 && !moderator {
                return Ok(Err(self.forbidden("rest_cannot_read", "Sorry, you are not allowed to read comments without a post.")));
            }
            if let Some(post) = &post
                && is_note
                && !supports_notes(&post.post_type)
            {
                if self.can_edit_post(post) {
                    return Ok(Err(RestError::new("rest_comment_not_supported_post_type", "Sorry, this post type does not support notes.", 403)));
                }
                return Ok(Err(self.forbidden("rest_forbidden_param", &format!("Query parameter not permitted: {}", forbidden().join(", ")))));
            }
        }
        // The edit context: notes ask for `edit_post` on their posts, everything else for moderation.
        if is_edit_context && is_note && !posts_asked.is_empty() {
            for id in &posts_asked {
                let editable = match self.site.db.post(*id).await? {
                    Some(post) => self.can_edit_post(&post),
                    None => false,
                };
                if !editable {
                    return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit comments.")));
                }
            }
        } else if is_edit_context && !moderator {
            return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit comments.")));
        }
        if !self.can("edit_posts") {
            let forbidden = forbidden();
            if !forbidden.is_empty() {
                return Ok(Err(self.forbidden("rest_forbidden_param", &format!("Query parameter not permitted: {}", forbidden.join(", ")))));
            }
        }

        let per_page = try_param!(params.integer("per_page", 10, 1, Some(100)));
        let page = try_param!(params.integer("page", 1, 1, None));
        let offset = try_param!(params.integer("offset", 0, 0, None));
        let order = try_param!(params.one_of("order", &["asc", "desc"], "desc"));
        let orderby = try_param!(params.one_of("orderby", &["date", "date_gmt", "id", "include", "post", "parent", "type"], "date_gmt"));

        // `WP_Comment_Query`: the type (`comment` is also the empty type; notes only when asked for), the status.
        let (types, types_not_in) = match kind.as_str() {
            "all" => (Vec::new(), Vec::new()),
            "comment" | "comments" => (vec![String::new(), "comment".into()], vec!["note".into()]),
            "pings" => (vec!["pingback".into(), "trackback".into()], vec!["note".into()]),
            "note" => (vec!["note".into()], Vec::new()),
            other => (vec![other.to_owned()], vec!["note".into()]),
        };
        let mut query = CommentQuery {
            ids: params.id_list("include").unwrap_or_default(),
            exclude_ids: params.id_list("exclude").unwrap_or_default(),
            post_in,
            parent_in: params.parent_list("parent").unwrap_or_default(),
            parent_not_in: params.parent_list("parent_exclude").unwrap_or_default(),
            types,
            types_not_in,
            statuses: status.split(',').map(|s| s.trim().to_owned()).collect(),
            public_posts: false,
            author_in: params.parent_list("author").unwrap_or_default(),
            author_not_in: params.parent_list("author_exclude").unwrap_or_default(),
            author_email: params.get("author_email").filter(|e| !e.is_empty()).map(str::to_owned),
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

        // The total counts every match; the list leaves out what the user may not read.
        let total = self.site.db.count_comments(&query).await?;
        let max_pages = total.div_ceil(per_page);
        if query.offset == 0 {
            query.offset = (page - 1).saturating_mul(per_page);
        }
        let comments = self.site.db.comments_query(&query).await?;
        let context = params.get("context").unwrap_or("view");
        let mut items = Vec::new();
        for comment in &comments {
            if !self.check_read_comment(comment, params, false).await? {
                continue;
            }
            items.push(self.prepare_comment(comment, context).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        resp.link = link_header(self, "wp/v2/comments", params, &LinkParams { arrays: &["author", "author_exclude", "exclude", "include", "parent", "parent_exclude", "post"], booleans: &[] }, page, max_pages);
        Ok(Ok(resp))
    }

    /// `get_comment()`: a positive ID of a comment whose post (if it has one) exists.
    pub(crate) async fn rest_comment(&self, id: &str) -> Result<std::result::Result<Comment, RestError>> {
        let invalid = || RestError::new("rest_comment_invalid_id", "Invalid comment ID.", 404);
        let id = id.parse::<i64>().unwrap_or(0);
        if id <= 0 {
            return Ok(Err(invalid()));
        }
        let Some(comment) = self.site.db.comment_any(id as u64).await? else { return Ok(Err(invalid())) };
        if comment.comment_post_id != 0 && self.site.db.post(comment.comment_post_id).await?.is_none() {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        }
        Ok(Ok(comment))
    }

    /// `get_item_permissions_check()` and `get_item()`.
    pub(crate) async fn comments_item(&self, id: &str, params: &Params) -> Result<Outcome> {
        if id.parse::<u64>().is_err() {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        }
        let comment = match self.rest_comment(id).await? {
            Ok(comment) => comment,
            Err(e) => return Ok(Err(e)),
        };
        let post = self.site.db.post(comment.comment_post_id).await?;
        // Notes ask for `edit_comment` in the edit context, the others for moderation.
        if params.get("context") == Some("edit") {
            let allowed = if comment.comment_type == "note" { self.user().is_some_and(|u| u.can_edit_comment(post.as_ref())) } else { self.can("moderate_comments") };
            if !allowed {
                return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit comments.")));
            }
        }
        if !self.check_read_comment(&comment, params, true).await? {
            return Ok(Err(self.forbidden("rest_cannot_read", "Sorry, you are not allowed to read this comment.")));
        }
        if let Some(post) = &post
            && !self.check_read_post_permission(post, params, false, true).await
        {
            return Ok(Err(self.forbidden("rest_cannot_read_post", "Sorry, you are not allowed to read the post for this comment.")));
        }
        let context = params.get("context").unwrap_or("view");
        Ok(Ok(Resp::item(self.prepare_comment(&comment, context).await?)))
    }

    /// `check_read_post_permission()`: a protected post asks for `edit_post` unless its password (or the edit context
    /// of an editor) opens it for a request about one post or one comment; otherwise the post has to be readable.
    pub(crate) async fn check_read_post_permission(&self, post: &Post, params: &Params, requested_post: bool, requested_comment: bool) -> bool {
        let single_post = requested_post && params.id_list("post").is_some_and(|l| l.len() == 1);
        let unlocked = (single_post || requested_comment)
            && !post.post_password.is_empty()
            && ((params.get("context") == Some("edit") && self.can_edit_post(post)) || params.get("password").is_some_and(|p| rustpress_core::hash_equals(&post.post_password, p)));
        if !post.post_password.is_empty() && !unlocked { self.can_edit_post(post) } else { self.check_read_permission(post).await }
    }

    /// `check_read_permission()` of a comment: an approved comment (not a note) of a readable post, else the user's
    /// own comment or one the user may edit.
    pub(crate) async fn check_read_comment(&self, comment: &Comment, params: &Params, requested_comment: bool) -> Result<bool> {
        let post = if comment.comment_post_id != 0 { self.site.db.post(comment.comment_post_id).await? } else { None };
        if comment.comment_type != "note"
            && let Some(post) = &post
            && comment.comment_approved == "1"
            && self.check_read_post_permission(post, params, !requested_comment, requested_comment).await
        {
            return Ok(true);
        }
        let Some(user) = self.user().filter(|u| u.id != 0) else { return Ok(false) };
        if comment.comment_post_id == 0 && !user.can("moderate_comments") {
            return Ok(false);
        }
        if comment.user_id != 0 && comment.user_id == user.id {
            return Ok(true);
        }
        Ok(user.can_edit_comment(post.as_ref()))
    }

    /// `rest_get_route_for_post()`: the route of a post type that is in the REST API.
    fn post_route(&self, post: &Post) -> Option<String> {
        let base = match post.post_type.as_str() {
            "post" => "posts".to_owned(),
            "page" => "pages".to_owned(),
            "attachment" => "media".to_owned(),
            "wp_block" => "blocks".to_owned(),
            other => self.site.types.post_type(other).filter(|t| t.show_in_rest).map(|t| t.rest_base().to_owned())?,
        };
        Some(format!("wp/v2/{base}/{}", post.id))
    }

    /// `prepare_item_for_response()`.
    pub(crate) async fn prepare_comment(&self, comment: &Comment, context: &str) -> Result<Value> {
        let post = self.site.db.post(comment.comment_post_id).await?;
        let link = match &post {
            Some(_) => {
                let env = crate::util::env(self);
                rustpress_blocks::comment_pages::comment_link(&env, comment, None).await?
            }
            None => String::new(),
        };
        let mut data = Map::new();
        data.insert("id".into(), Value::from(comment.comment_id));
        data.insert("post".into(), Value::from(comment.comment_post_id));
        data.insert("parent".into(), Value::from(comment.comment_parent));
        data.insert("author".into(), Value::from(comment.user_id));
        data.insert("author_name".into(), Value::from(comment.comment_author.clone()));
        data.insert("author_email".into(), Value::from(comment.comment_author_email.clone()));
        data.insert("author_url".into(), Value::from(comment.comment_author_url.clone()));
        data.insert("author_ip".into(), Value::from(comment.comment_author_ip.clone()));
        data.insert("author_user_agent".into(), Value::from(comment.comment_agent.clone()));
        data.insert("date".into(), Value::from(rfc3339(&comment.comment_date)));
        data.insert("date_gmt".into(), Value::from(rfc3339(&comment.comment_date_gmt)));
        let rendered = comment_text(self.site, &comment.comment_content);
        data.insert("content".into(), if context == "edit" { json!({"rendered": rendered, "raw": comment.comment_content}) } else { json!({"rendered": rendered}) });
        data.insert("link".into(), Value::from(link));
        // `prepare_status_response()`
        let status = match comment.comment_approved.as_str() {
            "hold" | "0" => "hold",
            "approve" | "1" => "approved",
            other => other,
        };
        data.insert("status".into(), Value::from(status));
        let kind = if comment.comment_type.is_empty() { "comment" } else { comment.comment_type.as_str() };
        data.insert("type".into(), Value::from(kind));
        if self.site.show_avatars() {
            // `get_avatar_data()`: only comments and notes have avatars (`get_avatar_comment_types()`); a comment of a
            // registered user shows the user's avatar.
            let value = if matches!(kind, "comment" | "note") {
                let user_email = if comment.user_id != 0 { self.site.db.user(comment.user_id).await?.map(|u| u.user_email) } else { None };
                self.avatar_urls(user_email.as_deref().unwrap_or(&comment.comment_author_email))
            } else {
                json!({"24": false, "48": false, "96": false})
            };
            data.insert("author_avatar_urls".into(), value);
        }
        let note_status = self.site.db.comment_meta_value(comment.comment_id, "_wp_note_status").await?;
        data.insert("meta".into(), json!({"_wp_note_status": note_status}));

        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("wp/v2/comments/{}", comment.comment_id)));
        links.add("collection", vec![], self.rest_url("wp/v2/comments"));
        if comment.user_id != 0 {
            links.embeddable("author", self.rest_url(&format!("wp/v2/users/{}", comment.user_id)));
        }
        if let Some(post) = &post
            && let Some(route) = self.post_route(post)
        {
            links.add("up", vec![("embeddable", Value::Bool(true)), ("post_type", Value::from(post.post_type.clone()))], self.rest_url(&route));
        }
        if comment.comment_parent != 0 {
            links.embeddable("in-reply-to", self.rest_url(&format!("wp/v2/comments/{}", comment.comment_parent)));
        }
        // `get_children()`: any type, approved or waiting.
        let children = self.site.db.count_comments(&CommentQuery { parent_in: vec![comment.comment_id], types: Vec::new(), statuses: vec!["all".into()], public_posts: false, ..Default::default() }).await?;
        if children > 0 {
            let mut href = rustpress_core::urls::add_query_arg(&self.rest_url("wp/v2/comments"), "parent", &comment.comment_id.to_string());
            if comment.comment_type == "note" {
                href = rustpress_core::urls::add_query_arg(&rustpress_core::urls::add_query_arg(&href, "type", "note"), "status", "all");
            }
            links.embeddable("children", href);
        }
        data.insert("_links".into(), links.finish());

        match context {
            "embed" => data.retain(|k, _| EMBED_FIELDS.contains(&k.as_str())),
            "edit" => {}
            _ => data.retain(|k, _| !EDIT_FIELDS.contains(&k.as_str())),
        }
        Ok(Value::Object(data))
    }
}
