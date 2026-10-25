//! `wp/v2/media` for signed-in users (`WP_REST_Attachments_Controller`): listing, uploading, editing, deleting.

use rustpress_content::content::PostData;
use rustpress_content::media::{UploadedFile, handle_upload};
use rustpress_db::{AttachmentQuery, Post};
use serde_json::{Map, Value, json};

use super::{Args, Body};
use crate::error::RestError;
use crate::params::Params;
use crate::posts::Kind;
use crate::{Outcome, Resp, Rest, Result};

impl Rest<'_> {
    pub(crate) async fn prepare_media_edit(&self, post: &Post, context: &str) -> Result<Value> {
        let mut data = self.prepare_media(post, if context == "edit" { "view" } else { context }).await?;
        let Value::Object(map) = &mut data else { return Ok(data) };
        if context == "edit" {
            let mut out = Map::new();
            for (key, value) in std::mem::take(map) {
                match key.as_str() {
                    "guid" => {
                        out.insert("guid".into(), json!({"rendered": post.guid, "raw": post.guid}));
                    }
                    "slug" => {
                        out.insert("password".into(), json!(post.post_password));
                        out.insert("slug".into(), value);
                    }
                    "title" => {
                        out.insert("title".into(), json!({"raw": post.post_title, "rendered": value.get("rendered").cloned().unwrap_or(Value::Null)}));
                    }
                    "description" => {
                        out.insert("description".into(), json!({"raw": post.post_content, "rendered": value.get("rendered").cloned().unwrap_or(Value::Null)}));
                    }
                    "caption" => {
                        out.insert("caption".into(), json!({"raw": post.post_excerpt, "rendered": value.get("rendered").cloned().unwrap_or(Value::Null)}));
                    }
                    "class_list" => {
                        let (template, generated) = (self.site.permalink(post).await.unwrap_or_default(), post.post_name.clone());
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
        if let Some(Value::Object(links)) = map.get_mut("_links")
            && let Some(Value::Array(items)) = links.get_mut("self")
            && let Some(Value::Object(link)) = items.first_mut()
        {
            let mut allow = vec!["GET"];
            if self.can_edit_post(post) {
                allow.extend(["POST", "PUT", "PATCH"]);
            }
            if self.can_delete_post(post) {
                allow.push("DELETE");
            }
            link.insert("targetHints".into(), json!({"allow": allow}));
        }
        Ok(data)
    }

    pub(crate) async fn media_route(&self, id: Option<&str>, method: &str, args: &Args, params: &Params) -> Result<Outcome> {
        match (method, id) {
            ("GET", None) => self.media_list(args, params).await,
            ("GET", Some(id)) => self.media_get(id, args, params).await,
            ("POST", None) => self.media_create(args).await,
            ("POST" | "PUT" | "PATCH", Some(id)) => self.media_update(id, args).await,
            ("DELETE", Some(id)) => self.media_delete(id, args).await,
            _ => Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        }
    }

    async fn media_list(&self, args: &Args, params: &Params) -> Result<Outcome> {
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if context == "edit" && !self.can("upload_files") {
            return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit posts in this post type.")));
        }
        let per_page = args.u64("per_page").unwrap_or(10).clamp(1, 100);
        let page = args.u64("page").unwrap_or(1).max(1);
        let mime = args.str("media_type").filter(|t| matches!(t.as_str(), "image" | "video" | "audio" | "application")).or_else(|| args.str("mime_type"));
        let query = AttachmentQuery {
            mime,
            search: args.str("search").filter(|s| !s.is_empty()),
            parent: args.ids("parent").and_then(|p| p.first().copied()),
            ids: args.ids("include").unwrap_or_default(),
            exclude_ids: args.ids("exclude").unwrap_or_default(),
            author: args.ids("author").and_then(|a| a.first().copied()),
            orderby: args.str("orderby").unwrap_or_else(|| "date".to_owned()),
            desc: args.str("order").as_deref() != Some("asc"),
            limit: per_page,
            offset: (page - 1) * per_page,
            ..Default::default()
        };
        let _ = params;
        let (posts, total) = self.site.db.list_attachments(&query).await?;
        let mut items = Vec::new();
        for post in &posts {
            items.push(self.prepare_media_edit(post, &context).await?);
        }
        let mut resp = Resp::item(Value::Array(items));
        resp.total = Some(total);
        resp.total_pages = Some(total.div_ceil(per_page));
        resp.allow = Some(if self.can("upload_files") { "GET, POST" } else { "GET" }.to_owned());
        Ok(Ok(resp))
    }

    async fn media_get(&self, id: &str, args: &Args, params: &Params) -> Result<Outcome> {
        let _ = params;
        let id: u64 = id.parse().unwrap_or(0);
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == "attachment") else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        let context = args.str("context").unwrap_or_else(|| "view".to_owned());
        if context == "edit" && !self.can_edit_post(&post) {
            return Ok(Err(self.forbidden("rest_forbidden_context", "Sorry, you are not allowed to edit this post.")));
        }
        Ok(Ok(Resp::item(self.prepare_media_edit(&post, &context).await?)))
    }

    async fn media_create(&self, args: &Args) -> Result<Outcome> {
        if !self.can("upload_files") {
            return Ok(Err(self.forbidden("rest_cannot_create", "Sorry, you are not allowed to upload media on this site.")));
        }
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let Some(request) = &self.request else { return Ok(Err(RestError::new("rest_upload_no_data", "No data supplied.", 400))) };
        let (file, extra_fields): (UploadedFile, Vec<(String, String)>) = match &request.body {
            Body::Multipart { fields, files } => {
                let Some(part) = files.iter().find(|f| f.field == "file").or(files.first()) else {
                    return Ok(Err(RestError::new("rest_upload_no_data", "No data supplied.", 400)));
                };
                (UploadedFile { name: part.filename.clone(), bytes: part.bytes.clone() }, fields.clone())
            }
            Body::Raw { bytes, disposition, .. } => {
                let filename = disposition.split(';').find_map(|p| p.trim().strip_prefix("filename=")).map(|f| f.trim_matches('"').to_owned());
                let Some(filename) = filename else {
                    return Ok(Err(RestError::new("rest_upload_no_content_disposition", "No Content-Disposition supplied.", 400)));
                };
                (UploadedFile { name: filename, bytes: bytes.clone() }, Vec::new())
            }
            _ => return Ok(Err(RestError::new("rest_upload_no_data", "No data supplied.", 400))),
        };
        let uploads = self.access.as_ref().map(|a| a.uploads.clone()).unwrap_or_default();
        let parent = args.u64("post").or_else(|| extra_fields.iter().find(|(k, _)| k == "post").and_then(|(_, v)| v.parse().ok())).unwrap_or(0);
        let id = match handle_upload(&content, &uploads, file, parent).await {
            Ok(id) => id,
            Err(rustpress_content::media::UploadError(message)) => return Ok(Err(RestError::new("rest_upload_unknown_error", &message, 500))),
        };
        // Fields sent along with the file.
        let mut update = Args::default();
        for (key, value) in &extra_fields {
            update.map.insert(key.clone(), Value::from(value.clone()));
        }
        for (key, value) in &args.map {
            update.map.insert(key.clone(), value.clone());
        }
        let _ = self.apply_media_fields(id, &update).await;
        let Some(post) = self.site.db.post(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        let resp = Resp::item(self.prepare_media_edit(&post, "edit").await?);
        let location = self.rest_url(&format!("wp/v2/media/{id}"));
        Ok(Ok(self.created(resp, location)))
    }

    async fn apply_media_fields(&self, id: u64, args: &Args) -> std::result::Result<(), RestError> {
        let content = self.content()?;
        let data = PostData {
            id: Some(id),
            post_title: args.raw_text("title"),
            post_excerpt: args.raw_text("caption"),
            post_content: args.raw_text("description"),
            post_name: args.str("slug"),
            post_author: args.u64("author"),
            comment_status: args.str("comment_status"),
            ping_status: args.str("ping_status"),
            post_parent: args.u64("post"),
            ..Default::default()
        };
        content.insert_post(&data).await.map_err(|e| RestError::new("rest_cannot_edit", &e.to_string(), 500))?;
        if let Some(alt) = args.str("alt_text") {
            self.writer()?.update_post_meta(id, "_wp_attachment_image_alt", &rustpress_content::sanitize::text_field(&alt)).await.map_err(|e| RestError::new("rest_cannot_edit", &e.to_string(), 500))?;
        }
        Ok(())
    }

    async fn media_update(&self, id: &str, args: &Args) -> Result<Outcome> {
        let id: u64 = id.parse().unwrap_or(0);
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == "attachment") else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        if !self.can_edit_post(&post) {
            return Ok(Err(self.forbidden("rest_cannot_edit", "Sorry, you are not allowed to edit this post.")));
        }
        if let Err(e) = self.apply_media_fields(id, args).await {
            return Ok(Err(e));
        }
        let Some(post) = self.site.db.post(id).await? else { return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404))) };
        Ok(Ok(Resp::item(self.prepare_media_edit(&post, "edit").await?)))
    }

    async fn media_delete(&self, id: &str, args: &Args) -> Result<Outcome> {
        let id: u64 = id.parse().unwrap_or(0);
        let Some(post) = self.site.db.post(id).await?.filter(|p| p.post_type == "attachment") else {
            return Ok(Err(RestError::new("rest_post_invalid_id", "Invalid post ID.", 404)));
        };
        if !self.can_delete_post(&post) {
            return Ok(Err(self.forbidden("rest_cannot_delete", "Sorry, you are not allowed to delete this post.")));
        }
        if !args.bool("force").unwrap_or(false) {
            return Ok(Err(RestError::new("rest_trash_not_supported", "Media does not support trashing. Set force=true to delete.", 501)));
        }
        let previous = self.prepare_media_edit(&post, "edit").await?;
        let content = match self.content() {
            Ok(c) => c,
            Err(e) => return Ok(Err(e)),
        };
        let uploads = self.access.as_ref().map(|a| a.uploads.clone()).unwrap_or_default();
        let _ = content.delete_attachment(id, uploads).await;
        let _ = Kind::Attachment;
        Ok(Ok(Resp::item(json!({"deleted": true, "previous": previous}))))
    }
}
