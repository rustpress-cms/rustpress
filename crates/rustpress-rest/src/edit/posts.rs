//! `wp/v2/posts`, `pages` and the routes of custom post types for signed-in users
//! (`WP_REST_Posts_Controller` and `WP_REST_Revisions_Controller` / `Autosaves`).

use rustpress_core::OrLog;
use rustpress_content::content::{PostData, TermRef};
use rustpress_core::slug::sanitize_title_save;
use rustpress_db::{Order, OrderBy, Post, PostQuery};
use serde_json::{Map, Value, json};

use super::Args;
use super::caps::TypeCaps;
use crate::error::{Msg, RestError};
use crate::params::Params;
use crate::posts::Kind;
use crate::util::rfc3339;
use crate::{Outcome, Resp, Rest, Result};

const ALL_STATUSES: [&str; 5] = ["publish", "future", "draft", "pending", "private"];

fn allow_for(can_edit: bool, can_delete: bool) -> String {
    let mut methods = vec!["GET"];
    if can_edit {
        methods.extend(["POST", "PUT", "PATCH"]);
    }
    if can_delete {
        methods.push("DELETE");
    }
    methods.join(", ")
}

impl Rest<'_> {
    fn type_not_allowed(&self, kind: Kind<'_>) -> RestError {
        let message = if kind == Kind::Page { "Sorry, you are not allowed to create pages as this user." } else { "Sorry, you are not allowed to create posts as this user." };
        self.forbidden("rest_cannot_create", message)
    }

    pub(crate) async fn custom_edit_route(&self, base: &str, id: Option<&str>, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let Some(info) = self.site.types.post_types.iter().find(|t| t.show_in_rest && t.rest_base() == base) else {
            return self.terms_custom(base, id, method, args, params).await;
        };
        self.posts_route(Kind::Custom(info), id, method, args, params).await
    }

    pub(crate) async fn posts_route(&self, kind: Kind<'_>, id: Option<&str>, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        match (method, id) {
            ("GET", None) => self.edit_collection(kind, args, params).await,
            ("GET", Some(id)) => self.edit_item(kind, id, args, params).await,
            ("POST", None) => self.create_post(kind, args).await,
            ("POST" | "PUT" | "PATCH", Some(id)) => self.update_post(kind, id, args).await,
            ("DELETE", Some(id)) => self.delete_post_route(kind, id, args).await,
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    // ------------------------------------------------------------------------------------------------ reading

    async fn edit_collection(&self, kind: Kind<'_>, args: &Args, params: &Params) -> Result<Outcome> {
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        let caps = TypeCaps::of(kind.name());
        let statuses: Vec<String> = match args.list("status") {
            Some(list) if list.iter().any(|s| s == "any") => ALL_STATUSES.iter().map(|s| (*s).to_owned()).chain(["trash".to_owned()]).collect(),
            Some(list) => list,
            None => vec![kind.public_status().to_owned()],
        };
        let restricted = statuses.iter().any(|s| s != kind.public_status());
        if restricted && !self.can(&caps.edit_posts) {
            return Ok(Err(RestError::invalid_param_with_data("status", "Status is forbidden.", "rest_forbidden_status", json!({"status": 403}))));
        }
        // The parameters are validated before the permission callback.
        if context == "edit" && !self.can(&caps.edit_posts) {
            return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit posts in this post type.")));
        }
        // The query of the anonymous reader decides the filters; the statuses are the signed-in user's.
        let mut anonymous = params.clone();
        anonymous.set("status", kind.public_status());
        anonymous.set("context", "view");
        let mut query = match self.post_query(kind, &anonymous).await? {
            Ok(query) => query,
            Err(e) => return Ok(Err(e)),
        };
        query.statuses = statuses.clone();
        if kind == Kind::Attachment {
            query.statuses = vec!["inherit".into(), "private".into()];
            query.public_parent = false;
        }
        let mut resp = match self.run_edit_posts(kind, params, query, &context).await? {
            Ok(r) => r,
            Err(e) => return Ok(Err(e)),
        };
        resp.allow = Some(allow_for(self.can_create(kind.name()), false).replace("GET", "GET, POST").replace("GET, POST, POST", "GET, POST"));
        Ok(Ok(resp))
    }

    async fn run_edit_posts(&self, kind: Kind<'_>, params: &Params, mut query: PostQuery, context: &str) -> Result<Outcome> {
        let per_page = query.limit.unwrap_or(10);
        let page = params.integer("page", 1, 1, None).unwrap_or(1);
        let total = self.site.db.count_posts(&query).await?;
        let max_pages = total.div_ceil(per_page);
        if page > max_pages && total > 0 {
            return Ok(Err(RestError::new("rest_post_invalid_page_number", "The page number requested is larger than the number of pages available.", 400)));
        }
        if query.offset == 0 {
            query.offset = (page - 1).saturating_mul(per_page);
        }
        let posts = self.site.db.posts(&query).await?;
        let ids: Vec<u64> = posts.iter().map(|p| p.id).collect();
        self.site.db.prime_loop(&posts).await?;
        self.site.db.prime_revisions(&ids).await?;
        let mut items = Vec::with_capacity(posts.len());
        for post in &posts {
            // Posts the user may not see are left out; the totals stay those of the query, as in WordPress.
            let permitted = if context == "edit" { self.can_edit_post(post) } else { self.check_read_permission(post).await };
            if permitted {
                items.push(self.prepare_post_edit(post, kind, context).await?);
            }
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(max_pages);
        Ok(Ok(resp))
    }

    async fn edit_item(&self, kind: Kind<'_>, id: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == kind.name()) else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        if !self.check_read_permission(&post).await {
            return Ok(Err(self.forbidden("rest_forbidden", "Sorry, you are not allowed to view this post.")));
        }
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if context == "edit" && !self.can_edit_post(&post) {
            return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit this post.")));
        }
        if let Some(given) = params.get("password").filter(|p| !p.is_empty())
            && !rustpress_core::hash_equals(&post.post_password, given)
        {
            return Ok(Err(RestError::new("rest_post_incorrect_password", "Incorrect post password.", 403)));
        }
        let mut resp = Resp::item(self.prepare_post_edit(&post, kind, &context).await?);
        resp.allow = Some(allow_for(self.can_edit_post(&post), self.can_delete_post(&post)));
        resp.alternate = Some(self.site.permalink(&post).await?);
        Ok(Ok(resp))
    }

    /// `prepare_item_for_response()` for the `edit` context, and the signed-in view of the other contexts.
    pub(crate) async fn prepare_post_edit(&self, post: &Post, kind: Kind<'_>, context: &str) -> Result<Value> {
        if kind == Kind::Attachment {
            return self.prepare_media_edit(post, context).await;
        }
        let mut params = Params::default();
        // `check_password_required()`: only in the edit context does who may edit a protected post see its content.
        if !post.post_password.is_empty() && context == "edit" && self.can_edit_post(post) {
            params.set("password", &post.post_password);
        }
        let mut data = self.prepare_post(post, kind, if context == "edit" { "view" } else { context }, &params).await?;
        let Value::Object(map) = &mut data else { return Ok(data) };
        if context == "edit" {
            let custom = matches!(kind, Kind::Custom(_));
            let has = |feature: &str| !custom || self.supports(kind, feature);
            let mut out = Map::new();
            let guid_rendered = map.get("guid").and_then(|g| g.get("rendered")).cloned().unwrap_or(Value::Null);
            for (key, value) in std::mem::take(map) {
                match key.as_str() {
                    "guid" => {
                        out.insert("guid".into(), json!({"rendered": guid_rendered, "raw": post.guid}));
                    }
                    "slug" => {
                        out.insert("password".into(), json!(post.post_password));
                        out.insert("slug".into(), value);
                    }
                    "title" if has("title") => {
                        let rendered = value.get("rendered").cloned().unwrap_or(Value::Null);
                        out.insert("title".into(), json!({"raw": post.post_title, "rendered": rendered}));
                    }
                    "content" if has("editor") => {
                        let rendered = value.get("rendered").cloned().unwrap_or(Value::Null);
                        let protected = value.get("protected").cloned().unwrap_or(Value::Bool(false));
                        out.insert("content".into(), json!({"raw": post.post_content, "rendered": rendered, "protected": protected, "block_version": i32::from(rustpress_blocks::content::has_blocks(&post.post_content))}));
                    }
                    "excerpt" if has("excerpt") => {
                        let rendered = value.get("rendered").cloned().unwrap_or(Value::Null);
                        let protected = value.get("protected").cloned().unwrap_or(Value::Bool(false));
                        out.insert("excerpt".into(), json!({"raw": post.post_excerpt, "rendered": rendered, "protected": protected}));
                    }
                    "class_list" => {
                        let (template, generated) = self.sample_permalink(post).await?;
                        out.insert("permalink_template".into(), json!(template));
                        out.insert("generated_slug".into(), json!(generated));
                        out.insert("class_list".into(), value);
                    }
                    _ => {
                        out.insert(key, value);
                    }
                }
            }
            *map = out;
        }
        // The meta of a post is what the editor stores in it.
        if let Some(meta) = map.get_mut("meta")
            && meta.as_object().is_some()
        {
            // `footnotes` stays; nothing else is registered for the REST API.
        }
        // Links.
        let featured = map.get("featured_media").and_then(Value::as_u64).unwrap_or(0);
        let mut links = self.post_links(post, kind, featured).await?;
        self.add_action_links(&mut links, post, kind, context == "edit");
        map.insert("_links".into(), links);
        Ok(data)
    }

    /// `get_sample_permalink()`: the permalink with `%postname%` and the slug WordPress would give.
    async fn sample_permalink(&self, post: &Post) -> Result<(String, String)> {
        let content = match self.content() {
            Ok(c) => c,
            Err(_) => return Ok((self.site.permalink(post).await?, post.post_name.clone())),
        };
        // `get_sample_permalink( $post->ID, $post->post_title, '' )`: the empty name stands for the title.
        let wanted = sanitize_title_save(&post.post_title, &self.site.locale());
        let wanted = if wanted.is_empty() { post.id.to_string() } else { wanted };
        let generated = content.unique_post_slug(&wanted, post.id, &post.post_status, &post.post_type, post.post_parent).await.unwrap_or(wanted);
        let placeholder = if post.post_type == "page" { "%pagename%" } else { "%postname%" };
        let mut sample = post.clone();
        sample.post_name = placeholder.to_owned();
        if matches!(sample.post_status.as_str(), "draft" | "pending" | "auto-draft" | "future") {
            sample.post_status = "publish".to_owned();
        }
        let template = self.site.permalink(&sample).await?;
        Ok((template, generated))
    }

    /// The `wp:action-*` links and the `targetHints` of the `self` link.
    fn add_action_links(&self, links: &mut Value, post: &Post, kind: Kind<'_>, edit_context: bool) {
        let Value::Object(map) = links else { return };
        let href = self.rest_url(&format!("wp/v2/{}/{}", kind.base(), post.id));
        let caps = TypeCaps::of(kind.name());
        let mut allow = vec!["GET"];
        if self.can_edit_post(post) {
            allow.extend(["POST", "PUT", "PATCH"]);
        }
        if self.can_delete_post(post) {
            allow.push("DELETE");
        }
        if let Some(Value::Array(items)) = map.get_mut("self")
            && let Some(Value::Object(link)) = items.first_mut()
        {
            link.insert("targetHints".into(), json!({"allow": allow}));
        }
        // `get_available_actions()`: only the `edit` context lists them.
        if !edit_context {
            return;
        }
        let curies = map.remove("curies");
        let mut action = |name: &str| {
            map.insert(format!("wp:action-{name}"), json!([{"href": href}]));
        };
        if self.can(&caps.publish) {
            action("publish");
        }
        if self.can("unfiltered_html") {
            action("unfiltered-html");
        }
        if kind == Kind::Post && self.can(&caps.edit_others) && self.can(&caps.publish) {
            action("sticky");
        }
        if self.can(&caps.edit_others) && (!matches!(kind, Kind::Custom(_)) || self.supports(kind, "author")) {
            action("assign-author");
        }
        for tax in self.taxonomies_of(kind.name()) {
            let manage = match tax.name.as_str() {
                "category" | "post_tag" => "manage_categories",
                _ => "manage_categories",
            };
            if self.can(manage) {
                action(&format!("create-{}", tax.base));
            }
            if self.can("edit_posts") {
                action(&format!("assign-{}", tax.base));
            }
        }
        if let Some(curies) = curies {
            map.insert("curies".into(), curies);
        }
    }

    // ------------------------------------------------------------------------------------------------ writing

    /// `prepare_item_for_database()`: the fields of the request as the arguments of `wp_insert_post()`.
    fn post_data(&self, kind: Kind<'_>, args: &Args, existing: Option<&Post>) -> std::result::Result<PostData, RestError> {
        let caps = TypeCaps::of(kind.name());
        let mut data = PostData { id: existing.map(|p| p.id), post_type: Some(kind.name().to_owned()), ..Default::default() };
        data.post_title = args.raw_text("title");
        data.post_content = args.raw_text("content");
        data.post_excerpt = args.raw_text("excerpt");
        if let Some(status) = args.str("status") {
            let status = status.to_lowercase();
            if !matches!(status.as_str(), "publish" | "future" | "draft" | "pending" | "private" | "trash" | "auto-draft") {
                return Err(RestError::invalid_param("status", rustpress_rest_not_in_enum("status", &["publish", "future", "draft", "pending", "private"]), "rest_not_in_enum"));
            }
            let was = existing.map(|p| p.post_status.as_str()).unwrap_or("");
            if matches!(status.as_str(), "publish" | "future" | "private") && !self.can(&caps.publish) && !matches!(was, "publish" | "future" | "private") {
                return Err(self.forbidden("rest_cannot_publish", "Sorry, you are not allowed to publish posts in this post type."));
            }
            data.post_status = Some(status);
        }
        if let Some(slug) = args.str("slug") {
            data.post_name = Some(slug);
        }
        if let Some(password) = args.str("password") {
            if !self.can(&caps.edit_others) && !self.can(&caps.publish) {
                return Err(self.forbidden("rest_cannot_publish", "Sorry, you are not allowed to create password protected posts in this post type."));
            }
            data.post_password = Some(password);
        }
        if let Some(author) = args.u64("author") {
            let own = self.user().map_or(0, |u| u.id);
            if author != own && !self.can(&caps.edit_others) {
                return Err(self.forbidden("rest_cannot_edit_others", "Sorry, you are not allowed to update posts as this user."));
            }
            data.post_author = Some(author);
        }
        if let Some(parent) = args.u64("parent") {
            data.post_parent = Some(parent);
        }
        if let Some(order) = args.i64("menu_order") {
            data.menu_order = Some(order as i32);
        }
        if let Some(status) = args.str("comment_status") {
            data.comment_status = Some(status);
        }
        if let Some(status) = args.str("ping_status") {
            data.ping_status = Some(status);
        }
        if let Some(date) = args.str("date").filter(|d| !d.is_empty()) {
            let local = date.replace('T', " ");
            let local: String = local.chars().take(19).collect();
            data.post_date_gmt = Some(self.gmt_from_local(&local));
            data.post_date = Some(local);
            data.edit_date = true;
        } else if let Some(date) = args.str("date_gmt").filter(|d| !d.is_empty()) {
            let gmt: String = date.replace('T', " ").chars().take(19).collect();
            if let Some(ts) = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&gmt) {
                data.post_date = Some(self.site.date("Y-m-d H:i:s", ts));
                data.post_date_gmt = Some(gmt);
                data.edit_date = true;
            }
        }
        if let Some(template) = args.str("template") {
            data.page_template = Some(if template.is_empty() { "default".to_owned() } else { template });
        }
        if let Some(featured) = args.i64("featured_media") {
            data.thumbnail_id = Some(featured);
        }
        if kind == Kind::Post {
            if let Some(ids) = args.ids("categories") {
                data.post_category = Some(ids);
            }
            if let Some(ids) = args.ids("tags") {
                data.tax_input.push(("post_tag".into(), ids.into_iter().map(TermRef::Id).collect()));
            }
        }
        if let Kind::Custom(info) = kind {
            for tax in self.taxonomies_of(&info.name) {
                if let Some(ids) = args.ids(&tax.base) {
                    data.tax_input.push((tax.name.clone(), ids.into_iter().map(TermRef::Id).collect()));
                }
            }
        }
        if let Some(Value::Object(meta)) = args.get("meta") {
            for (key, value) in meta {
                if key == "footnotes"
                    && let Some(text) = value.as_str()
                {
                    data.meta_input.push((key.clone(), text.to_owned()));
                }
            }
        }
        Ok(data)
    }

    async fn after_write(&self, post_id: u64, kind: Kind<'_>, args: &Args) -> Result<()> {
        let Ok(writer) = self.writer() else { return Ok(()) };
        let caps = TypeCaps::of(kind.name());
        if kind == Kind::Post
            && let Some(sticky) = args.bool("sticky")
            && self.can(&caps.edit_others)
            && self.can(&caps.publish)
        {
            let mut ids = self.site.settings.sticky_posts.clone();
            ids.retain(|i| *i != post_id);
            if sticky {
                ids.insert(0, post_id);
            }
            let value = rustpress_phpser::Value::Array(ids.iter().enumerate().map(|(i, id)| (rustpress_phpser::Key::Int(i as i64), rustpress_phpser::Value::Int(*id as i64))).collect());
            writer.update_option("sticky_posts", &String::from_utf8_lossy(&rustpress_phpser::serialize(&value))).await.or_log();
        }
        if (kind == Kind::Post || matches!(kind, Kind::Custom(info) if info.supports("post-formats")))
            && let Some(format) = args.str("format")
            && let Ok(content) = self.content()
        {
            let terms = if format == "standard" || format.is_empty() { Vec::new() } else { vec![TermRef::Name(format!("post-format-{format}"))] };
            content.set_object_terms(post_id, &terms, "post_format", false).await.or_log();
        }
        writer.update_post_meta(post_id, "_edit_last", &self.user().map_or(0, |u| u.id).to_string()).await.or_log();
        Ok(())
    }

    /// `rest_parse_date()`: the shape of an ISO 8601 date the REST API accepts.
    pub(crate) fn valid_date(date: &str) -> bool {
        static SHAPE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        SHAPE.get_or_init(|| regex::Regex::new(r"^\d{4}-\d{2}-\d{2}[Tt ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}(?::\d{2})?)?$").expect("pattern")).is_match(date)
    }

    /// The arguments' schema check that runs before the permission check (`format: date-time`).
    fn check_dates(args: &Args) -> Option<RestError> {
        ["date", "date_gmt"].into_iter().find_map(|name| args.str(name).filter(|d| !d.is_empty() && !Self::valid_date(d)).map(|_| RestError::invalid_param(name, "Invalid date.", "rest_invalid_date")))
    }

    /// The references of `prepare_item_for_database()` and `handle_featured_media()`: the author, the parent and
    /// the featured image must exist. (WordPress checks the image after saving the post; here nothing is saved.)
    async fn check_references(&self, args: &Args) -> Result<Option<RestError>> {
        let me = self.user().map_or(0, |u| u.id);
        if let Some(author) = args.u64("author").filter(|a| *a != 0 && *a != me)
            && self.site.db.user(author).await?.is_none()
        {
            return Ok(Some(RestError::new("rest_invalid_author", "Invalid author ID.", 400)));
        }
        if let Some(parent) = args.u64("parent").filter(|p| *p != 0)
            && self.site.db.post(parent).await?.is_none()
        {
            return Ok(Some(RestError::new("rest_post_invalid_id", "Invalid post parent ID.", 400)));
        }
        if let Some(media) = args.i64("featured_media").filter(|m| *m > 0) {
            let image = self.site.db.post(media as u64).await?.is_some_and(|p| p.post_type == "attachment" && p.post_mime_type.starts_with("image/"));
            if !image {
                return Ok(Some(RestError::new("rest_invalid_featured_media", "Invalid featured media ID.", 400)));
            }
        }
        Ok(None)
    }

    async fn create_post(&self, kind: Kind<'_>, args: &Args) -> Result<Outcome> {
        if let Some(e) = Self::check_dates(args) {
            return Ok(Err(e));
        }
        if args.has("id") {
            return Ok(Err(RestError::new("rest_post_exists", "Cannot create existing post.", 400)));
        }
        if !self.can_create(kind.name()) {
            return Ok(Err(self.type_not_allowed(kind)));
        }
        let mut data = match self.post_data(kind, args, None) {
            Ok(d) => d,
            Err(e) => return Ok(Err(e)),
        };
        if let Some(e) = self.check_references(args).await? {
            return Ok(Err(e));
        }
        if data.post_status.is_none() {
            data.post_status = Some("draft".to_owned());
        }
        if data.post_author.is_none() {
            data.post_author = self.user().map(|u| u.id);
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let id = match content.insert_post(&data).await {
            Ok(id) => id,
            Err(e) => return Ok(Err(RestError::new("db_insert_error", &e.to_string(), 500))),
        };
        self.after_write(id, kind, args).await?;
        let Some(post) = self.site.db.post(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        let mut resp = Resp::item(self.prepare_post_edit(&post, kind, "edit").await?);
        resp.allow = Some(allow_for(true, self.can_delete_post(&post)));
        let location = self.rest_url(&format!("wp/v2/{}/{id}", kind.base()));
        Ok(Ok(self.created(resp, location)))
    }

    async fn update_post(&self, kind: Kind<'_>, id: &str, args: &Args) -> Result<Outcome> {
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        if let Some(e) = Self::check_dates(args) {
            return Ok(Err(e));
        }
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == kind.name()) else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        if !self.can_edit_post(&post) {
            return Ok(Err(self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this post.")));
        }
        let mut data = match self.post_data(kind, args, Some(&post)) {
            Ok(d) => d,
            Err(e) => return Ok(Err(e)),
        };
        if let Some(e) = self.check_references(args).await? {
            return Ok(Err(e));
        }
        // An auto draft that is saved becomes a draft.
        if data.post_status.is_none() && post.post_status == "auto-draft" {
            data.post_status = Some("draft".to_owned());
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        if let Err(e) = content.insert_post(&data).await {
            return Ok(Err(RestError::new("db_update_error", &e.to_string(), 500)));
        }
        // A saved post makes its autosaves obsolete.
        content.delete_user_autosave(id).await.ok();
        self.after_write(id, kind, args).await?;
        let Some(updated) = self.site.db.post(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        let mut resp = Resp::item(self.prepare_post_edit(&updated, kind, "edit").await?);
        resp.allow = Some(allow_for(true, self.can_delete_post(&updated)));
        Ok(Ok(resp))
    }

    async fn delete_post_route(&self, kind: Kind<'_>, id: &str, args: &Args) -> Result<Outcome> {
        let Ok(id) = id.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == kind.name()) else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        if !self.can_delete_post(&post) {
            return Ok(Err(self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete this post.")));
        }
        let force = args.bool("force").unwrap_or(false);
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        if force || post.post_status == "trash" {
            let previous = self.prepare_post_edit(&post, kind, "edit").await?;
            let ok = if kind == Kind::Attachment { content.delete_attachment(id, self.access.as_ref().map(|a| a.uploads.clone()).unwrap_or_default()).await } else { content.delete_post(id, true).await };
            if !matches!(ok, Ok(true)) {
                return Ok(Err(RestError::new("rest_cannot_delete", "The post cannot be deleted.", 500)));
            }
            return Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))));
        }
        if !matches!(kind, Kind::Post | Kind::Page | Kind::Custom(_)) {
            return Ok(Err(RestError::new("rest_trash_not_supported", Msg::new("The post does not support trashing. Set '%s' to delete.").with(&["force=true"]), 501)));
        }
        if !matches!(content.trash_post(id).await, Ok(true)) {
            return Ok(Err(RestError::new("rest_cannot_delete", "The post cannot be deleted.", 500)));
        }
        let Some(trashed) = self.site.db.post(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        Ok(Ok(Resp::item(self.prepare_post_edit(&trashed, kind, "edit").await?)))
    }

    // ------------------------------------------------------------------------------------------------ revisions

    pub(crate) async fn revisions_route(&self, kind: Kind<'_>, parent: &str, sub: &str, id: Option<&str>, method: &str, args: &Args) -> Result<Outcome> {
        let Ok(parent_id) = parent.parse::<u64>() else {
            return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)));
        };
        let Some(parent_post) = self.site.db.post(parent_id).await?.filter(|p| p.post_type == kind.name()) else {
            return Ok(Err(RestError::new("rest_post_invalid_parent", "Invalid post parent ID.", 404)));
        };
        if self.access.is_none() {
            return self.revisions_denied(kind, parent, sub).await;
        }
        if method == "DELETE" && sub == "revisions" && !self.can_delete_post(&parent_post) {
            return Ok(Err(self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete revisions of this post.")));
        }
        if !self.can_edit_post(&parent_post) {
            return Ok(Err(self.forbidden("rest_cannot_read", &format!("Sorry, you are not allowed to view {sub} of this post."))));
        }
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if sub == "autosaves" {
            return self.autosaves(kind, &parent_post, method, args, &context, id).await;
        }
        let revisions = self.site.db.posts(&PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(parent_id), order_by: OrderBy::Date, order: Order::Desc, ..Default::default() }).await?;
        let revisions: Vec<Post> = revisions.into_iter().filter(|r| !r.post_name.contains("autosave")).collect();
        match (method, id) {
            ("GET", None) => {
                let mut items = Vec::new();
                for revision in &revisions {
                    items.push(self.prepare_revision(revision, &parent_post, kind, &context).await?);
                }
                let mut resp = Resp::item(Value::Array(items));
                resp.total = Some(revisions.len() as u64);
                resp.total_pages = Some(1);
                Ok(Ok(resp))
            }
            ("GET", Some(rid)) => match rid.parse::<u64>().ok().and_then(|id| revisions.iter().find(|r| r.id == id)) {
                Some(revision) => Ok(Ok(Resp::item(self.prepare_revision(revision, &parent_post, kind, &context).await?))),
                None => Ok(Err(RestError::new("rest_post_invalid_id", "Invalid revision ID.", 404))),
            },
            ("DELETE", Some(rid)) => {
                let Some(revision) = rid.parse::<u64>().ok().and_then(|id| revisions.iter().find(|r| r.id == id)) else {
                    return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid revision ID.", 404)));
                };
                // `delete_post` of a revision is `do_not_allow` in `map_meta_cap()`: nobody deletes revisions here.
                if !self.can_delete_post(revision) {
                    return Ok(Err(self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete this revision.")));
                }
                if !args.bool("force").unwrap_or(false) {
                    return Ok(Err(RestError::new("rest_trash_not_supported", Msg::new("Revisions do not support trashing. Set '%s' to delete.").with(&["force=true"]), 501)));
                }
                let previous = self.prepare_revision(revision, &parent_post, kind, "edit").await?;
                let content = match self.content() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(e)),
                };
                if !content.delete_post(revision.id, true).await.unwrap_or(false) {
                    return Ok(Err(RestError::new("rest_cannot_delete", "The post cannot be deleted.", 500)));
                }
                Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))))
            }
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    async fn prepare_revision(&self, revision: &Post, parent: &Post, kind: Kind<'_>, context: &str) -> Result<Value> {
        let mut data = Map::new();
        data.insert("author".into(), json!(revision.post_author));
        data.insert("date".into(), json!(rfc3339(&revision.post_date)));
        data.insert("date_gmt".into(), self.date_gmt(&revision.post_date_gmt, &revision.post_date));
        data.insert("id".into(), json!(revision.id));
        data.insert("modified".into(), json!(rfc3339(&revision.post_modified)));
        data.insert("modified_gmt".into(), self.date_gmt(&revision.post_modified_gmt, &revision.post_modified));
        data.insert("parent".into(), json!(parent.id));
        data.insert("slug".into(), json!(revision.post_name));
        data.insert("guid".into(), if context == "edit" { json!({"rendered": revision.guid, "raw": revision.guid}) } else { json!({"rendered": revision.guid}) });
        let rendered = |text: &str| rustpress_core::formatting::wpautop(&self.site.texturize(text));
        // The content of a revision is shown as the content of its post (`setup_postdata( $parent )`).
        let env = crate::util::env(self);
        let shared = std::sync::Arc::new(parent.clone());
        let content_rendered = rustpress_blocks::content::the_content(&env, &shared, &revision.post_content).await?;
        data.insert("title".into(), if context == "edit" { json!({"raw": revision.post_title, "rendered": self.site.texturize(&revision.post_title)}) } else { json!({"rendered": self.site.texturize(&revision.post_title)}) });
        data.insert("content".into(), if context == "edit" { json!({"raw": revision.post_content, "rendered": content_rendered}) } else { json!({"rendered": content_rendered}) });
        data.insert("excerpt".into(), if context == "edit" { json!({"raw": revision.post_excerpt, "rendered": rendered(&revision.post_excerpt)}) } else { json!({"rendered": rendered(&revision.post_excerpt)}) });
        if !matches!(kind, Kind::Custom(_)) {
            let footnotes = self.site.db.post_meta_value(revision.id, "footnotes").await.ok().flatten().unwrap_or_default();
            data.insert("meta".into(), json!({"footnotes": footnotes}));
        }
        let mut links = crate::util::Links::new();
        links.add("parent", vec![], self.rest_url(&format!("wp/v2/{}/{}", kind.base(), parent.id)));
        if revision.post_name.contains("autosave") {
            links.self_link(self.rest_url(&format!("wp/v2/{}/{}/revisions/{}", kind.base(), parent.id, revision.id)));
        }
        data.insert("_links".into(), links.finish());
        Ok(Value::Object(data))
    }

    async fn autosaves(&self, kind: Kind<'_>, parent: &Post, method: &str, args: &Args, context: &str, id: Option<&str>) -> Result<Outcome> {
        let user = self.user().map_or(0, |u| u.id);
        let existing = self.site.db.posts(&PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(parent.id), author: Some(user), order_by: OrderBy::Date, order: Order::Desc, ..Default::default() }).await?;
        let autosave = existing.into_iter().find(|r| r.post_name == format!("{}-autosave-v1", parent.id));
        match method {
            "GET" if id.is_some() => match autosave.as_ref().filter(|a| id.and_then(|i| i.parse::<u64>().ok()) == Some(a.id)) {
                Some(a) => Ok(Ok(Resp::item(self.prepare_revision(a, parent, kind, context).await?))),
                None => Ok(Err(RestError::new("rest_post_no_autosave", "There is no autosave revision for this post.", 404))),
            },
            "GET" => {
                let items = match &autosave {
                    Some(a) => vec![self.prepare_revision(a, parent, kind, context).await?],
                    None => Vec::new(),
                };
                // Creating an autosave needs the right to edit the post.
                let mut resp = Resp::item(Value::Array(items));
                resp.allow = Some(if self.user().is_some_and(|u| u.can_edit_post(parent)) { "GET, POST" } else { "GET" }.to_owned());
                Ok(Ok(resp))
            }
            "POST" => {
                let content = match self.content() {
                    Ok(c) => c,
                    Err(e) => return Ok(Err(e)),
                };
                let data = PostData { id: Some(parent.id), post_title: args.raw_text("title"), post_content: args.raw_text("content"), post_excerpt: args.raw_text("excerpt"), ..Default::default() };
                // The author of a draft saves it directly; everything else becomes an autosave.
                if matches!(parent.post_status.as_str(), "draft" | "auto-draft") && parent.post_author == user {
                    let mut data = data;
                    data.post_status = Some("draft".to_owned());
                    if let Err(e) = content.insert_post(&data).await {
                        return Ok(Err(RestError::new("rest_autosave_no_changes", &e.to_string(), 500)));
                    }
                    let Some(updated) = self.site.db.post(parent.id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
                    return Ok(Ok(Resp::item(self.prepare_post_edit(&updated, kind, "edit").await?)));
                }
                if let Err(e) = content.create_autosave(parent.id, &data).await {
                    return Ok(Err(RestError::new("rest_autosave_no_changes", &e.to_string(), 500)));
                }
                let existing = self.site.db.posts(&PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(parent.id), author: Some(user), ..Default::default() }).await?;
                match existing.into_iter().find(|r| r.post_name == format!("{}-autosave-v1", parent.id)) {
                    Some(a) => {
                        let mut value = self.prepare_revision(&a, parent, kind, "edit").await?;
                        if let Value::Object(map) = &mut value {
                            map.insert("preview_link".into(), json!(self.site.permalink(parent).await.unwrap_or_default()));
                        }
                        Ok(Ok(self.created(Resp::item(value), self.rest_url(&format!("wp/v2/{}/{}/autosaves/{}", kind.base(), parent.id, a.id)))))
                    }
                    None => Ok(Ok(Resp::item(json!({})))),
                }
            }
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }
}

/// The message of `rest_not_in_enum`.
fn rustpress_rest_not_in_enum(param: &str, allowed: &[&str]) -> crate::error::Msg {
    crate::error::Msg::new("%1$s is not one of %2$l.").with(&[param]).with_list(allowed)
}
