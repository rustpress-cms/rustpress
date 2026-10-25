//! Deleting attachments: `wp_delete_attachment()`.

use std::path::{Path, PathBuf};

use super::{Content, Result};

impl Content<'_> {
    /// Removes the files of an attachment (all sizes, the scaled-down copy and the original) and its rows.
    pub async fn delete_attachment(&self, id: u64, uploads: PathBuf) -> Result<bool> {
        let Some(post) = self.db.post(id).await? else { return Ok(false) };
        if post.post_type != "attachment" {
            return Ok(false);
        }
        let meta = self.db.post_meta(id).await?;
        let value = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.clone());
        let attached = value("_wp_attached_file");
        if let Some(metadata) = value("_wp_attachment_metadata").and_then(|raw| rustpress_blocks::images::Metadata::parse(&raw)) {
            let dir = attached.as_deref().and_then(|a| Path::new(a).parent()).map(|p| uploads.join(p)).unwrap_or_else(|| uploads.clone());
            if let Some(sizes) = metadata.sizes() {
                for entry in sizes.values() {
                    if let Some(file) = entry.get("file").and_then(|f| f.as_str()) {
                        let _ = std::fs::remove_file(dir.join(file));
                    }
                }
            }
            if let Some(original) = metadata.0.get("original_image").and_then(|f| f.as_str()) {
                let _ = std::fs::remove_file(dir.join(original));
            }
        }
        if let Some(attached) = &attached {
            let _ = std::fs::remove_file(uploads.join(attached));
        }
        // Posts that used it as their featured image.
        self.writer.delete_rows_where("postmeta", &format!("meta_key = '_thumbnail_id' AND meta_value = '{id}'")).await?;
        self.delete_post(id, true).await
    }
}
