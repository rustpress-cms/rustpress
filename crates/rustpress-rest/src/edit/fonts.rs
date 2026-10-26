//! `wp/v2/font-families`, `wp/v2/font-families/<id>/font-faces` and `wp/v2/font-collections`
//! (`WP_REST_Font_Families_Controller`, `WP_REST_Font_Faces_Controller`, `WP_REST_Font_Collections_Controller`).
//!
//! A family is a `wp_font_family` post (title: name, slug: slug, content: JSON with the font family and the
//! preview); a face is a `wp_font_face` post whose parent is the family. Uploaded font files live in
//! `wp-content/uploads/fonts` and are listed in `_wp_font_face_file` meta. Font collections come from
//! WordPress.org in WordPress; RustPress makes no outbound requests, so there are none.

use rustpress_core::OrLog;
use rustpress_blocks::php_json::{self, Flags};
use rustpress_content::content::PostData;
use rustpress_core::formatting::esc_url_raw;
use rustpress_core::slug::sanitize_title_save;
use rustpress_db::{Order, OrderBy, Post, PostQuery};
use serde_json::{Map, Value, json};

use super::{Args, Body};
use crate::error::{Msg, RestError};
use crate::params::Params;
use crate::util::Links;
use crate::{Outcome, Resp, Rest, Result};

const FAMILY: &str = "wp_font_family";
const FACE: &str = "wp_font_face";
const FONT_TYPES: [&str; 4] = ["otf", "ttf", "woff", "woff2"];
const THEME_JSON_VERSION: u64 = 3;

/// `WP_Font_Utils::format_font_family()`: names with spaces are quoted, quotes of the input are dropped.
fn format_font_family(family: &str) -> String {
    family
        .split(',')
        .map(|part| {
            let part = part.trim().trim_matches(['"', '\'']).trim();
            if part.contains(' ') { format!("\"{part}\"") } else { part.to_owned() }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `wp_json_encode()` for a post's content.
fn stored_json(value: &Value) -> String {
    php_json::encode_with(value, Flags::default())
}

fn no_route() -> RestError {
    RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404)
}

fn required(parent: &str, property: &str) -> RestError {
    RestError::invalid_param_with_data(parent, Msg::new("%1$s is a required property of %2$s.").with(&[property, parent]), "rest_property_required", json!({"status": 400}))
}

fn invalid(parent: &str, message: Msg) -> RestError {
    RestError::invalid_param_with_data(parent, message, "rest_invalid_param", json!({"status": 400}))
}

impl Rest<'_> {
    fn fonts_denied(&self, code: &str, message: &str) -> RestError {
        self.forbidden(code, message)
    }

    fn can_fonts(&self) -> bool {
        self.can("edit_theme_options")
    }

    /// Routes `font-families`, its faces and `font-collections`.
    pub(crate) async fn fonts_route(&self, segments: &[&str], method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let tail: Vec<&str> = segments.iter().skip(3).copied().collect();
        match (segments.get(2).copied(), tail.as_slice(), method) {
            (Some("font-collections"), [], "GET") => self.collections_list().await,
            (Some("font-collections"), [slug], "GET") => self.collections_item(slug).await,
            (Some("font-families"), [], "GET") => self.families_list(args, params).await,
            (Some("font-families"), [], "POST") => self.family_create(args).await,
            (Some("font-families"), [id], "GET") => self.family_get(id).await,
            (Some("font-families"), [id], "POST" | "PUT" | "PATCH") => self.family_update(id, args).await,
            (Some("font-families"), [id], "DELETE") => self.family_delete(id, args).await,
            (Some("font-families"), [id, "font-faces"], "GET") => self.faces_list(id, args, params).await,
            (Some("font-families"), [id, "font-faces"], "POST") => self.face_create(id, args).await,
            (Some("font-families"), [id, "font-faces", face], "GET") => self.face_get(id, face).await,
            (Some("font-families"), [id, "font-faces", face], "DELETE") => self.face_delete(id, face, args).await,
            _ => Ok(Err(no_route())),
        }
    }

    // ---------------------------------------------------------------- collections

    async fn collections_list(&self) -> Result<Outcome> {
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_read", "Sorry, you are not allowed to access font collections.")));
        }
        let mut resp = Resp::item(json!([]));
        resp.total = Some(0);
        resp.total_pages = Some(0);
        Ok(Ok(resp))
    }

    async fn collections_item(&self, _slug: &str) -> Result<Outcome> {
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_read", "Sorry, you are not allowed to access font collections.")));
        }
        Ok(Err(RestError::new("rest_font_collection_not_found", "Font collection not found.", 404)))
    }

    // ---------------------------------------------------------------- families

    async fn family(&self, id: &str) -> Result<Option<Post>> {
        let id: u64 = id.parse().unwrap_or(0);
        Ok(self.site.db.post(id).await?.filter(|p| p.post_type == FAMILY))
    }

    async fn face_ids(&self, family: u64) -> Result<Vec<u64>> {
        let query = PostQuery { post_types: vec![FACE.into()], statuses: vec!["publish".into()], parent: Some(family), order_by: OrderBy::Id, order: Order::Asc, ..Default::default() };
        Ok(self.site.db.posts(&query).await?.into_iter().map(|p| p.id).collect())
    }

    async fn prepare_family(&self, post: &Post, with_links: bool) -> Result<Value> {
        let faces = self.face_ids(post.id).await?;
        let mut settings = Map::new();
        settings.insert("name".into(), json!(post.post_title));
        settings.insert("slug".into(), json!(post.post_name));
        if let Ok(Value::Object(content)) = serde_json::from_str::<Value>(&post.post_content) {
            for (key, value) in content {
                settings.insert(key, value);
            }
        }
        let mut data = Map::new();
        data.insert("id".into(), json!(post.id));
        data.insert("theme_json_version".into(), json!(THEME_JSON_VERSION));
        data.insert("font_faces".into(), json!(faces));
        data.insert("font_family_settings".into(), Value::Object(settings));
        if with_links {
            let mut links = Links::new();
            let mut allow = vec!["GET"];
            if self.can_fonts() {
                allow.extend(["POST", "PUT", "PATCH", "DELETE"]);
            }
            links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/font-families/{}", post.id)), "targetHints": {"allow": allow}}));
            links.push_raw("collection", json!({"href": self.rest_url("wp/v2/font-families")}));
            for face in &faces {
                links.push_raw("font_faces", json!({"embeddable": true, "href": self.rest_url(&format!("wp/v2/font-families/{}/font-faces/{face}", post.id))}));
            }
            data.insert("_links".into(), links.finish());
        }
        Ok(Value::Object(data))
    }

    async fn families_list(&self, args: &Args, params: &Params) -> Result<Outcome> {
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_read", "Sorry, you are not allowed to access font families.")));
        }
        let _ = params;
        let per_page = args.u64("per_page").unwrap_or(10).clamp(1, 100);
        let page = args.u64("page").unwrap_or(1).max(1);
        let slugs = args.list("slug").unwrap_or_default();
        let mut query = PostQuery {
            post_types: vec![FAMILY.into()],
            statuses: vec!["publish".into()],
            ids: args.ids("include").unwrap_or_default(),
            exclude_ids: args.ids("exclude").unwrap_or_default(),
            names: slugs,
            order_by: if args.str("orderby").as_deref() == Some("include") { OrderBy::Include } else { OrderBy::Id },
            order: if args.str("order").as_deref() == Some("asc") { Order::Asc } else { Order::Desc },
            ..Default::default()
        };
        let total = self.site.db.count_posts(&query).await?;
        query.limit = Some(per_page);
        query.offset = args.u64("offset").unwrap_or((page - 1).saturating_mul(per_page));
        let posts = self.site.db.posts(&query).await?;
        let mut items = Vec::new();
        for post in &posts {
            items.push(self.prepare_family(post, true).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(total.div_ceil(per_page));
        resp.allow = Some("GET, POST".to_owned());
        Ok(Ok(resp))
    }

    async fn family_get(&self, id: &str) -> Result<Outcome> {
        let Some(post) = self.family(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_read", "Sorry, you are not allowed to access font families.")));
        }
        Ok(Ok(Resp::item(self.prepare_family(&post, true).await?)))
    }

    /// The `font_family_settings` parameter: a JSON string that holds an object.
    fn family_settings(&self, args: &Args, create: bool) -> std::result::Result<Map<String, Value>, RestError> {
        const PARAM: &str = "font_family_settings";
        let raw = match args.get(PARAM) {
            None => return Err(RestError::missing_params(&[PARAM])),
            Some(Value::String(s)) => s.clone(),
            Some(_) => return Err(RestError::invalid_param_with_data(PARAM, Msg::new("%s is not of type %s.").with(&[PARAM, "string"]), "rest_invalid_type", json!({"param": PARAM}))),
        };
        let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(&raw) else {
            return Err(invalid(PARAM, Msg::new("%s parameter must be a valid JSON string.").with(&[PARAM])));
        };
        if create {
            for property in ["name", "slug", "fontFamily"] {
                if !settings.contains_key(property) {
                    return Err(required(PARAM, property));
                }
            }
        } else if settings.contains_key("slug") {
            return Err(invalid(PARAM, Msg::new("%s cannot be updated.").with(&["font_family_settings[slug]"])));
        }
        for property in ["name", "slug", "fontFamily", "preview"] {
            if let Some(value) = settings.get(property)
                && !value.is_string() {
                    return Err(invalid(PARAM, Msg::new("%1$s is not of type %2$s.").with(&[&format!("{PARAM}[{property}]"), "string"])));
                }
        }
        Ok(settings)
    }

    async fn family_create(&self, args: &Args) -> Result<Outcome> {
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_create", "Sorry, you are not allowed to create posts as this user.")));
        }
        let settings = match self.family_settings(args, true) {
            Ok(s) => s,
            Err(e) => return Ok(Err(e)),
        };
        let text = |key: &str| settings.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
        let slug = sanitize_title_save(&text("slug"), &self.site.locale());
        let existing = PostQuery { post_types: vec![FAMILY.into()], statuses: vec!["publish".into()], name: Some(slug.clone()), ..Default::default() };
        if self.site.db.count_posts(&existing).await? > 0 {
            return Ok(Err(RestError::new("rest_duplicate_font_family", Msg::new("A font family with slug \"%s\" already exists.").with(&[&slug]), 400)));
        }
        let content = json!({"fontFamily": format_font_family(&rustpress_content::sanitize::text_field(&text("fontFamily"))), "preview": esc_url_raw(&text("preview"))});
        let data = PostData {
            post_type: Some(FAMILY.into()),
            post_status: Some("publish".into()),
            post_title: Some(rustpress_content::sanitize::text_field(&text("name"))),
            post_name: Some(slug),
            post_content: Some(stored_json(&content)),
            comment_status: Some("closed".into()),
            ping_status: Some("closed".into()),
            ..Default::default()
        };
        let content_api = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let id = match content_api.insert_post(&data).await {
            Ok(id) => id,
            Err(e) => return Ok(Err(RestError::new("db_insert_error", &e.to_string(), 500))),
        };
        let Some(post) = self.site.db.post(id).await? else { return Ok(Err(no_route())) };
        let resp = Resp::item(self.prepare_family(&post, true).await?);
        Ok(Ok(self.created(resp, self.rest_url(&format!("wp/v2/font-families/{id}")))))
    }

    async fn family_update(&self, id: &str, args: &Args) -> Result<Outcome> {
        let Some(post) = self.family(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_edit", "Sorry, you are not allowed to edit this post.")));
        }
        let settings = match self.family_settings(args, false) {
            Ok(s) => s,
            Err(e) => return Ok(Err(e)),
        };
        let mut content = match serde_json::from_str::<Value>(&post.post_content) {
            Ok(Value::Object(map)) => map,
            _ => Map::new(),
        };
        let text = |key: &str| settings.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
        if settings.contains_key("fontFamily") {
            content.insert("fontFamily".into(), json!(format_font_family(&rustpress_content::sanitize::text_field(&text("fontFamily")))));
        }
        if settings.contains_key("preview") {
            content.insert("preview".into(), json!(esc_url_raw(&text("preview"))));
        }
        let data = PostData {
            id: Some(post.id),
            post_title: settings.contains_key("name").then(|| rustpress_content::sanitize::text_field(&text("name"))),
            post_content: Some(stored_json(&Value::Object(content))),
            ..Default::default()
        };
        let content_api = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        if let Err(e) = content_api.insert_post(&data).await {
            return Ok(Err(RestError::new("db_update_error", &e.to_string(), 500)));
        }
        let Some(post) = self.site.db.post(post.id).await? else { return Ok(Err(no_route())) };
        Ok(Ok(Resp::item(self.prepare_family(&post, true).await?)))
    }

    async fn family_delete(&self, id: &str, args: &Args) -> Result<Outcome> {
        let Some(post) = self.family(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_delete", "Sorry, you are not allowed to delete this post.")));
        }
        if !args.bool("force").unwrap_or(false) {
            return Ok(Err(RestError::new("rest_trash_not_supported", Msg::new("Font faces do not support trashing. Set \"%s\" to delete.").with(&["force=true"]), 501)));
        }
        let previous = self.prepare_family(&post, false).await?;
        for face in self.face_ids(post.id).await? {
            self.remove_face(face).await?;
        }
        let content_api = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        if content_api.delete_post(post.id, true).await.is_err() {
            return Ok(Err(RestError::new("rest_cannot_delete", "The post cannot be deleted.", 500)));
        }
        Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))))
    }

    // ---------------------------------------------------------------- faces

    fn fonts_dir(&self) -> std::path::PathBuf {
        self.access.as_ref().map(|a| a.uploads.join("fonts")).unwrap_or_default()
    }

    fn fonts_url(&self) -> String {
        format!("{}/fonts", rustpress_blocks::images::upload_baseurl(self.site))
    }

    /// The font family of a request path; faces of a missing family answer `rest_post_invalid_parent`.
    async fn face_parent(&self, family: &str) -> Result<std::result::Result<Post, RestError>> {
        Ok(self.family(family).await?.ok_or_else(|| RestError::new("rest_post_invalid_parent", "Invalid post parent ID.", 404)))
    }

    fn prepare_face(&self, post: &Post, with_links: bool) -> Value {
        let settings = serde_json::from_str::<Value>(&post.post_content).ok().filter(Value::is_object).unwrap_or_else(|| json!({}));
        let mut data = Map::new();
        data.insert("id".into(), json!(post.id));
        data.insert("theme_json_version".into(), json!(THEME_JSON_VERSION));
        data.insert("parent".into(), json!(post.post_parent));
        data.insert("font_face_settings".into(), settings);
        if with_links {
            let mut links = Links::new();
            links.push_raw("self", json!({"href": self.rest_url(&format!("wp/v2/font-families/{}/font-faces/{}", post.post_parent, post.id)), "targetHints": {"allow": if self.can_fonts() { vec!["GET", "DELETE"] } else { vec!["GET"] }}}));
            links.push_raw("collection", json!({"href": self.rest_url(&format!("wp/v2/font-families/{}/font-faces", post.post_parent))}));
            links.push_raw("parent", json!({"href": self.rest_url(&format!("wp/v2/font-families/{}", post.post_parent))}));
            data.insert("_links".into(), links.finish());
        }
        Value::Object(data)
    }

    async fn faces_list(&self, family: &str, args: &Args, params: &Params) -> Result<Outcome> {
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_read", "Sorry, you are not allowed to access font faces.")));
        }
        let parent = match self.face_parent(family).await? {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let _ = params;
        let per_page = args.u64("per_page").unwrap_or(10).clamp(1, 100);
        let page = args.u64("page").unwrap_or(1).max(1);
        let mut query = PostQuery {
            post_types: vec![FACE.into()],
            statuses: vec!["publish".into()],
            parent: Some(parent.id),
            ids: args.ids("include").unwrap_or_default(),
            exclude_ids: args.ids("exclude").unwrap_or_default(),
            order_by: OrderBy::Id,
            order: if args.str("order").as_deref() == Some("desc") { Order::Desc } else { Order::Asc },
            ..Default::default()
        };
        let total = self.site.db.count_posts(&query).await?;
        query.limit = Some(per_page);
        query.offset = args.u64("offset").unwrap_or((page - 1).saturating_mul(per_page));
        let posts = self.site.db.posts(&query).await?;
        let mut resp = Resp::item(Value::Array(posts.iter().map(|p| self.prepare_face(p, true)).collect()));
        resp.total = Some(total);
        resp.total_pages = Some(total.div_ceil(per_page));
        resp.allow = Some("GET, POST".to_owned());
        Ok(Ok(resp))
    }

    async fn face_of(&self, family: &Post, face: &str) -> Result<std::result::Result<Post, RestError>> {
        let id: u64 = face.parse().unwrap_or(0);
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == FACE) else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        if post.post_parent != family.id {
            return Ok(Err(RestError::new("rest_font_face_parent_id_mismatch", Msg::new("The font face does not belong to the specified font family with id of \"%d\".").with(&[&family.id.to_string()]), 404)));
        }
        Ok(Ok(post))
    }

    async fn face_get(&self, family: &str, face: &str) -> Result<Outcome> {
        // The face itself is looked up before its family.
        let face_id: u64 = face.parse().unwrap_or(0);
        if self.site.db.post(face_id).await?.is_none_or(|p| p.post_type != FACE) {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        }
        let parent = match self.face_parent(family).await? {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let post = match self.face_of(&parent, face).await? {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_read", "Sorry, you are not allowed to access font faces.")));
        }
        Ok(Ok(Resp::item(self.prepare_face(&post, true))))
    }

    /// `font_face_settings`: the object, its `src` checked against the files of the request.
    fn face_settings(&self, args: &Args, files: &[(String, String)]) -> std::result::Result<Map<String, Value>, RestError> {
        const PARAM: &str = "font_face_settings";
        let raw = match args.get(PARAM) {
            None => return Err(RestError::missing_params(&[PARAM])),
            Some(Value::String(s)) => s.clone(),
            Some(_) => return Err(RestError::invalid_param_with_data(PARAM, Msg::new("%s is not of type %s.").with(&[PARAM, "string"]), "rest_invalid_type", json!({"param": PARAM}))),
        };
        let Ok(Value::Object(settings)) = serde_json::from_str::<Value>(&raw) else {
            return Err(invalid(PARAM, Msg::new("%s parameter must be a valid JSON string.").with(&[PARAM])));
        };
        for property in ["fontFamily", "src"] {
            if !settings.contains_key(property) {
                return Err(required(PARAM, property));
            }
        }
        let sources: Vec<&Value> = match settings.get("src") {
            Some(Value::Array(items)) => items.iter().collect(),
            Some(other) => vec![other],
            None => Vec::new(),
        };
        for source in sources {
            let Some(text) = source.as_str() else {
                return Err(invalid(PARAM, Msg::new("%1$s is not of type %2$s.").with(&["font_face_settings[src]", "string"])));
            };
            let is_file = files.iter().any(|(field, _)| field == text);
            let is_url = text.starts_with("http://") || text.starts_with("https://");
            if !is_file && !is_url {
                return Err(invalid(PARAM, Msg::new("%1$s value \"%2$s\" must be a valid URL or file reference.").with(&["font_face_settings[src]", text])));
            }
        }
        Ok(settings)
    }

    async fn face_create(&self, family: &str, args: &Args) -> Result<Outcome> {
        let parent = match self.face_parent(family).await? {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_create", "Sorry, you are not allowed to create posts as this user.")));
        }
        let uploads: Vec<(String, String, Vec<u8>)> = match self.request.as_ref().map(|r| &r.body) {
            Some(Body::Multipart { files, .. }) => files.iter().map(|f| (f.field.clone(), f.filename.clone(), f.bytes.clone())).collect(),
            _ => Vec::new(),
        };
        let names: Vec<(String, String)> = uploads.iter().map(|(field, name, _)| (field.clone(), name.clone())).collect();
        let mut settings = match self.face_settings(args, &names) {
            Ok(s) => s,
            Err(e) => return Ok(Err(e)),
        };
        // Files first: a type that is not a font stops the request before anything is stored.
        for (field, name, _) in &uploads {
            let extension = name.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
            let used = match settings.get("src") {
                Some(Value::Array(items)) => items.iter().any(|v| v.as_str() == Some(field)),
                Some(Value::String(s)) => s == field,
                _ => false,
            };
            if used && !FONT_TYPES.contains(&extension.as_str()) {
                return Ok(Err(RestError::new("rest_font_upload_invalid_file_type", "Sorry, you are not allowed to upload this file type.", 400)));
            }
        }
        let dir = self.fonts_dir();
        let mut stored: Vec<String> = Vec::new();
        let mut urls: Vec<(String, String)> = Vec::new();
        for (field, name, bytes) in &uploads {
            let used = match settings.get("src") {
                Some(Value::Array(items)) => items.iter().any(|v| v.as_str() == Some(field)),
                Some(Value::String(s)) => s == field,
                _ => false,
            };
            if !used {
                continue;
            }
            if std::fs::create_dir_all(&dir).is_err() {
                return Ok(Err(RestError::new("rest_font_upload_unknown_error", "The uploaded file could not be moved to the fonts directory.", 500)));
            }
            let filename = rustpress_content::media::unique_filename(&dir, &rustpress_content::sanitize::file_name(name));
            if std::fs::write(dir.join(&filename), bytes).is_err() {
                return Ok(Err(RestError::new("rest_font_upload_unknown_error", "The uploaded file could not be moved to the fonts directory.", 500)));
            }
            urls.push((field.clone(), format!("{}/{filename}", self.fonts_url())));
            stored.push(filename);
        }
        let replace = |value: &Value| -> Value {
            match value.as_str().and_then(|s| urls.iter().find(|(field, _)| field == s)) {
                Some((_, url)) => json!(url),
                None => value.clone(),
            }
        };
        if let Some(src) = settings.get("src").cloned() {
            let replaced = match &src {
                Value::Array(items) => Value::Array(items.iter().map(replace).collect()),
                other => replace(other),
            };
            settings.insert("src".into(), replaced);
        }
        // The text properties are cleaned, the font family is quoted like the families.
        let mut content = Map::new();
        for (key, value) in &settings {
            let value = match (key.as_str(), value) {
                ("fontFamily", Value::String(s)) => json!(format_font_family(&rustpress_content::sanitize::text_field(s))),
                ("preview", Value::String(s)) => json!(esc_url_raw(s)),
                (_, Value::String(s)) if key != "src" => json!(rustpress_content::sanitize::text_field(s)),
                _ => value.clone(),
            };
            content.insert(key.clone(), value);
        }
        let get = |key: &str, default: &str| settings.get(key).and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or(default).to_owned();
        let family_names: Vec<String> = get("fontFamily", "").split(',').map(|p| p.trim().trim_matches(['"', '\'']).to_lowercase()).collect();
        let title = format!("{};{};{};{};{}", family_names.join(","), get("fontStyle", "normal").to_lowercase(), get("fontWeight", "400"), get("fontStretch", "100%"), get("unicodeRange", "U+0-10FFFF"));
        let data = PostData {
            post_type: Some(FACE.into()),
            post_status: Some("publish".into()),
            post_title: Some(title),
            post_parent: Some(parent.id),
            post_content: Some(stored_json(&Value::Object(content))),
            comment_status: Some("closed".into()),
            ping_status: Some("closed".into()),
            ..Default::default()
        };
        let content_api = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let id = match content_api.insert_post(&data).await {
            Ok(id) => id,
            Err(e) => return Ok(Err(RestError::new("db_insert_error", &e.to_string(), 500))),
        };
        let writer = match self.writer() {
            Ok(w) => w,
            Err(e) => return Ok(Err(e)),
        };
        for filename in &stored {
            if let Err(e) = writer.add_post_meta(id, "_wp_font_face_file", filename).await {
                return Ok(Err(RestError::new("db_insert_error", &e.to_string(), 500)));
            }
        }
        let Some(post) = self.site.db.post(id).await? else { return Ok(Err(no_route())) };
        let resp = Resp::item(self.prepare_face(&post, true));
        Ok(Ok(self.created(resp, self.rest_url(&format!("wp/v2/font-families/{}/font-faces/{id}", parent.id)))))
    }

    /// Deletes a face and its font files.
    async fn remove_face(&self, face: u64) -> Result<()> {
        let dir = self.fonts_dir();
        let meta = self.site.db.post_meta(face).await?;
        for row in meta.iter().filter(|m| m.meta_key.as_deref() == Some("_wp_font_face_file")) {
            if let Some(file) = row.meta_value.as_deref().filter(|f| !f.contains(['/', '\\'])) {
                let _ = std::fs::remove_file(dir.join(file));
            }
        }
        if let Ok(content_api) = self.content() {
            content_api.delete_post(face, true).await.or_log();
        }
        Ok(())
    }

    async fn face_delete(&self, family: &str, face: &str, args: &Args) -> Result<Outcome> {
        let parent = match self.face_parent(family).await? {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        let post = match self.face_of(&parent, face).await? {
            Ok(p) => p,
            Err(e) => return Ok(Err(e)),
        };
        if !self.can_fonts() {
            return Ok(Err(self.fonts_denied("rest_cannot_delete", "Sorry, you are not allowed to delete this post.")));
        }
        if !args.bool("force").unwrap_or(false) {
            return Ok(Err(RestError::new("rest_trash_not_supported", Msg::new("Font faces do not support trashing. Set \"%s\" to delete.").with(&["force=true"]), 501)));
        }
        let previous = self.prepare_face(&post, false);
        self.remove_face(post.id).await?;
        Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))))
    }
}
