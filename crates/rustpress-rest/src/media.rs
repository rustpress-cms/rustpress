//! `wp/v2/media` (`WP_REST_Attachments_Controller`).

use std::sync::Arc;

use rustpress_blocks::content::{post_class, the_content_prefixed, the_title_for};
use rustpress_blocks::images::{self, Metadata};
use rustpress_core::formatting::{convert_chars, wpautop};
use rustpress_db::Post;
use serde_json::{Map, Value, json};

use crate::params::Params;
use crate::posts::Kind;
use crate::util::{Links, env, rfc3339};
use crate::{Outcome, Rest, Result};

/// The members of an attachment in the `embed` context.
const EMBED_FIELDS: &[&str] = &["id", "date", "slug", "type", "link", "title", "author", "featured_media", "caption", "alt_text", "media_type", "mime_type", "media_details", "source_url", "_links"];

impl Rest<'_> {
    pub(crate) async fn media_collection(&self, params: &Params) -> Result<Outcome> {
        self.posts_collection(Kind::Attachment, params).await
    }

    pub(crate) async fn media_item(&self, id: &str, params: &Params) -> Result<Outcome> {
        self.posts_item(Kind::Attachment, id, params).await
    }

    /// `prepare_item_for_response()` of the attachments controller.
    pub(crate) async fn prepare_media(&self, post: &Post, context: &str) -> Result<Value> {
        let env = env(self);
        let shared = Arc::new(post.clone());
        let (metadata, attached_file) = images::load_metadata(self.site, post).await?;
        let meta_rows = self.site.db.post_meta(post.id).await?;
        let meta_value = |key: &str| meta_rows.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
        let source_url = attached_file.as_deref().map(|f| images::attachment_url(self.site, f)).unwrap_or_default();
        let alt = meta_value("_wp_attachment_image_alt").map(|a| rustpress_core::formatting::strip_tags(&a).trim().to_owned()).unwrap_or_default();

        let mut data = Map::new();
        data.insert("id".into(), Value::from(post.id));
        data.insert("date".into(), Value::from(rfc3339(&post.post_date)));
        data.insert("date_gmt".into(), self.date_gmt(&post.post_date_gmt, &post.post_date));
        data.insert("guid".into(), json!({"rendered": post.guid}));
        data.insert("modified".into(), Value::from(rfc3339(&post.post_modified)));
        data.insert("modified_gmt".into(), self.date_gmt(&post.post_modified_gmt, &post.post_modified));
        data.insert("slug".into(), Value::from(post.post_name.clone()));
        data.insert("status".into(), Value::from(post.post_status.clone()));
        data.insert("type".into(), Value::from("attachment"));
        data.insert("link".into(), Value::from(self.site.permalink(post).await?));
        let plain = Post { post_password: String::new(), post_status: "publish".into(), ..post.clone() };
        data.insert("title".into(), json!({"rendered": the_title_for(self.site, &plain)}));
        data.insert("author".into(), Value::from(post.post_author));
        data.insert("featured_media".into(), Value::from(0));
        data.insert("comment_status".into(), Value::from(post.comment_status.clone()));
        data.insert("ping_status".into(), Value::from(post.ping_status.clone()));
        let template = meta_value("_wp_page_template").filter(|t| !t.is_empty() && t != "default").unwrap_or_default();
        data.insert("template".into(), Value::from(template));
        data.insert("meta".into(), Value::Array(Vec::new()));
        // `get_post_class()` names the post type only outside the administration.
        let mut classes = post_class(&env, post, &[], false, false).await?;
        if self.admin && classes.get(1) == Some(&post.post_type) {
            classes.remove(1);
        }
        data.insert("class_list".into(), Value::from(classes));

        // `description`: the content filters plus `prepend_attachment()`.
        let prefix = rustpress_blocks::content::attachment_prefix(&env, post, metadata.as_ref(), attached_file.as_deref(), &alt).await?;
        let description = the_content_prefixed(&env, &shared, &post.post_content, &prefix).await?;
        data.insert("description".into(), json!({"rendered": description}));
        let raw_caption = rustpress_blocks::excerpt::get_the_excerpt(&env, &shared).await?;
        let caption = wpautop(&convert_chars(&self.site.texturize(&raw_caption)));
        data.insert("caption".into(), json!({"rendered": caption}));
        data.insert("alt_text".into(), Value::from(alt));
        data.insert("media_type".into(), Value::from(if images::is_image(post) { "image" } else { "file" }));
        data.insert("mime_type".into(), Value::from(post.post_mime_type.clone()));
        data.insert("media_details".into(), self.media_details(post, metadata.as_ref(), attached_file.as_deref()));
        data.insert("post".into(), if post.post_parent == 0 { Value::Null } else { Value::from(post.post_parent) });
        data.insert("source_url".into(), Value::from(source_url));
        if let Some(file) = attached_file.as_deref() {
            // `wp_get_original_image_path()`: images that were scaled down report the original file.
            let original = metadata.as_ref().and_then(|m| m.0.get("original_image")).and_then(Value::as_str).filter(|_| images::is_image(post));
            data.insert("filename".into(), Value::from(original.unwrap_or_else(|| file.rsplit('/').next().unwrap_or(file))));
        }
        let filesize = metadata.as_ref().and_then(|m| m.0.get("filesize").cloned());
        if let Some(size) = filesize {
            data.insert("filesize".into(), size);
        }

        data.insert("_links".into(), self.media_links(post).await?);
        if context == "embed" {
            data.retain(|key, _| EMBED_FIELDS.contains(&key.as_str()));
            if let Some(Value::Object(caption)) = data.get_mut("caption") {
                caption.retain(|k, _| k == "rendered");
            }
        }
        Ok(Value::Object(data))
    }

    /// `media_details`: the stored metadata with `source_url`s and the `full` size.
    fn media_details(&self, post: &Post, metadata: Option<&Metadata>, attached_file: Option<&str>) -> Value {
        let Some(metadata) = metadata else { return Value::Array(Vec::new()) };
        let mut details = metadata.0.as_object().cloned().unwrap_or_default();
        if let Some(Value::Object(sizes)) = details.get_mut("sizes") {
            let directory = attached_file.and_then(|f| f.rsplit_once('/')).map(|(d, _)| format!("{d}/")).unwrap_or_default();
            for size in sizes.values_mut() {
                let Value::Object(entry) = size else { continue };
                if let Some(mime) = entry.remove("mime-type") {
                    entry.insert("mime_type".into(), mime);
                }
                let file = entry.get("file").and_then(Value::as_str).unwrap_or_default().to_owned();
                entry.insert("source_url".into(), Value::from(format!("{}/{directory}{file}", images::upload_baseurl(self.site))));
            }
            if let Some(file) = attached_file {
                // `wp_get_attachment_image_src( $id, 'full' )`: a PDF shows its preview image.
                let shown = if images::is_image(post) { Some((file.to_owned(), metadata.width(), metadata.height())) } else { images::preview_metadata(Some(metadata), Some(file)).map(|(m, path)| (path, m.width(), m.height())) };
                if let Some((path, width, height)) = shown {
                    sizes.insert(
                        "full".into(),
                        json!({
                            "file": path.rsplit('/').next().unwrap_or(&path),
                            "width": width,
                            "height": height,
                            "mime_type": post.post_mime_type,
                            "source_url": images::attachment_url(self.site, &path),
                        }),
                    );
                }
            }
        }
        // Details without any sizes still get an (empty) object.
        let sizes_empty = details.get("sizes").is_none_or(rustpress_blocks::style_engine::is_empty);
        if !details.is_empty() && sizes_empty {
            details.insert("sizes".into(), rustpress_blocks::php_json::empty_object());
        }
        Value::Object(details)
    }

    async fn media_links(&self, post: &Post) -> Result<Value> {
        let mut links = Links::new();
        links.self_link(self.rest_url(&format!("wp/v2/media/{}", post.id)));
        links.add("collection", vec![], self.rest_url("wp/v2/media"));
        links.add("about", vec![], self.rest_url("wp/v2/types/attachment"));
        if post.post_author != 0 {
            links.embeddable("author", self.rest_url(&format!("wp/v2/users/{}", post.post_author)));
        }
        links.embeddable("replies", rustpress_core::urls::add_query_arg(&self.rest_url("wp/v2/comments"), "post", &post.id.to_string()));
        if post.post_parent != 0 {
            let parent_type = self.site.db.post(post.post_parent).await?.map_or_else(|| "post".to_owned(), |p| p.post_type);
            let base = match parent_type.as_str() {
                "page" => "pages",
                "post" => "posts",
                other => other,
            };
            links.add("wp:attached-to", vec![("embeddable", Value::Bool(true)), ("post_type", Value::from(parent_type.clone())), ("id", Value::from(post.post_parent))], self.rest_url(&format!("wp/v2/{base}/{}", post.post_parent)));
        }
        Ok(links.finish())
    }
}
