//! Terms: `wp_insert_term()`, `wp_update_term()` and `wp_delete_term()`.

use super::{Content, ContentError, Result};
use crate::sanitize;
use rustpress_core::formatting::esc_html;
use rustpress_core::slug::sanitize_title;
use rustpress_db::Val;

/// The fields of a term as the forms send them.
#[derive(Debug, Clone, Default)]
pub struct TermData {
    pub name: String,
    pub slug: String,
    pub parent: u64,
    pub description: String,
}

impl Content<'_> {
    pub fn taxonomy_is_hierarchical(&self, taxonomy: &str) -> bool {
        taxonomy == "category" || self.site.types.taxonomy(taxonomy).is_some_and(|t| t.hierarchical)
    }

    /// `pre_term_name`: `sanitize_text_field()`, kses for users without `unfiltered_html`, then `_wp_specialchars()`.
    fn clean_name(&self, name: &str) -> String {
        let mut name = sanitize::text_field(name);
        if !self.user.can("unfiltered_html") {
            name = super::kses_post(&name);
        }
        esc_html(&name)
    }

    fn clean_description(&self, description: &str) -> String {
        if self.user.can("unfiltered_html") { description.to_owned() } else { super::kses_post(description) }
    }

    /// `wp_unique_term_slug()`: the slug, made unique across the terms table when it is taken in this taxonomy.
    async fn unique_slug(&self, slug: &str, taxonomy: &str, parent: u64, term_id: u64) -> Result<String> {
        let taken_here = self.db.term_by_slug(taxonomy, slug).await?.filter(|t| t.term_id != term_id).is_some();
        if !taken_here {
            return Ok(slug.to_owned());
        }
        let mut slug = slug.to_owned();
        if self.taxonomy_is_hierarchical(taxonomy) && parent != 0 {
            let all = self.db.terms(taxonomy).await?;
            let mut suffix = String::new();
            let mut current = parent;
            while current != 0 {
                let Some(parent_term) = all.iter().find(|t| t.term_id == current) else { break };
                suffix.push('-');
                suffix.push_str(&parent_term.slug);
                if self.db.term_by_slug(taxonomy, &format!("{slug}{suffix}")).await?.is_none() {
                    break;
                }
                current = parent_term.parent;
            }
            slug.push_str(&suffix);
        }
        let unique = self.db.unique_term_slug(&slug).await?;
        Ok(unique)
    }

    /// `wp_insert_term()`; returns the new `term_id`.
    pub async fn insert_term(&self, taxonomy: &str, data: &TermData) -> Result<u64> {
        let tr = |s: &str| self.site.tr(s);
        if self.site.types.taxonomy(taxonomy).is_none() && !matches!(taxonomy, "category" | "post_tag") {
            return Err(ContentError::Message(tr("Invalid taxonomy.")));
        }
        let name = self.clean_name(&data.name);
        if name.is_empty() {
            return Err(ContentError::Message(tr("A name is required for this term.")));
        }
        let hierarchical = self.taxonomy_is_hierarchical(taxonomy);
        let parent = if hierarchical { data.parent } else { 0 };
        if parent != 0 && self.db.term_by_id(Some(taxonomy), parent).await?.is_none() {
            return Err(ContentError::Message(tr("Parent term does not exist.")));
        }
        let slug_provided = !data.slug.trim().is_empty();
        let slug = if slug_provided { sanitize_title(&data.slug) } else { sanitize_title(&name) };
        let slug = if slug.is_empty() { sanitize_title(&data.name) } else { slug };

        // A term with the same name (and, in a hierarchy, the same parent) already exists.
        let all = self.db.terms(taxonomy).await?;
        let name_match = all.iter().find(|t| t.name.to_lowercase() == name.to_lowercase() && (!hierarchical || t.parent == parent));
        if let Some(name_match) = name_match {
            let slug_match = all.iter().find(|t| t.slug == slug);
            if !slug_provided || name_match.slug == slug || slug_match.is_some() {
                if hierarchical {
                    let siblings: Vec<_> = all.iter().filter(|t| t.parent == parent).collect();
                    let existing = if (!slug_provided || name_match.slug == slug) && siblings.iter().any(|t| t.name == name) {
                        Some(name_match)
                    } else {
                        slug_match.filter(|m| siblings.iter().any(|t| t.slug == m.slug))
                    };
                    if existing.is_some() {
                        return Err(ContentError::Message(tr("A term with the name provided already exists with this parent.")));
                    }
                } else {
                    return Err(ContentError::Message(tr("A term with the name provided already exists in this taxonomy.")));
                }
            }
        }
        let slug = self.unique_slug(&slug, taxonomy, parent, 0).await?;
        let term_id = self.writer.insert_row("terms", &[("name", Val::Str(name)), ("slug", Val::Str(slug)), ("term_group", Val::Int(0))]).await?;
        self.writer
            .insert_row(
                "term_taxonomy",
                &[
                    ("term_id", Val::Int(term_id as i64)),
                    ("taxonomy", Val::Str(taxonomy.to_owned())),
                    ("description", Val::Str(self.clean_description(&data.description))),
                    ("parent", Val::Int(parent as i64)),
                    ("count", Val::Int(0)),
                ],
            )
            .await?;
        Ok(term_id)
    }

    /// `wp_update_term()`.
    pub async fn update_term(&self, term_id: u64, taxonomy: &str, data: &TermData) -> Result<()> {
        let tr = |s: &str| self.site.tr(s);
        let Some(term) = self.db.term_by_id(Some(taxonomy), term_id).await? else { return Err(ContentError::Message(tr("Empty Term."))) };
        let name = self.clean_name(&data.name);
        if name.is_empty() {
            return Err(ContentError::Message(tr("A name is required for this term.")));
        }
        let hierarchical = self.taxonomy_is_hierarchical(taxonomy);
        let mut parent = if hierarchical { data.parent } else { 0 };
        // `wp_check_term_hierarchy_for_loops()`: a term cannot sit below itself.
        if parent != 0 {
            let all = self.db.terms(taxonomy).await?;
            let mut current = parent;
            let mut guard = 0;
            while current != 0 && guard < 100 {
                if current == term_id {
                    parent = term.parent;
                    break;
                }
                current = all.iter().find(|t| t.term_id == current).map_or(0, |t| t.parent);
                guard += 1;
            }
        }
        let empty_slug = data.slug.trim().is_empty();
        let mut slug = if empty_slug { sanitize_title(&name) } else { sanitize_title(&data.slug) };
        if slug.is_empty() {
            slug = sanitize_title(&data.name);
        }
        if let Some(duplicate) = self.db.term_by_slug(taxonomy, &slug).await?
            && duplicate.term_id != term_id
        {
            if empty_slug || parent != term.parent {
                slug = self.unique_slug(&slug, taxonomy, parent, term_id).await?;
            } else {
                return Err(ContentError::Message(rustpress_core::i18n::sprintf(&tr("The slug &#8220;%s&#8221; is already in use by another term."), &[&slug])));
            }
        }
        self.writer.update_rows("terms", "term_id", term_id, &[("name", Val::Str(name)), ("slug", Val::Str(slug))]).await?;
        self.writer
            .update_rows("term_taxonomy", "term_taxonomy_id", term.term_taxonomy_id, &[("description", Val::Str(self.clean_description(&data.description))), ("parent", Val::Int(parent as i64))])
            .await?;
        Ok(())
    }

    /// `wp_delete_term()`. Returns `false` for the default category, which cannot be deleted.
    pub async fn delete_term(&self, term_id: u64, taxonomy: &str) -> Result<bool> {
        let Some(term) = self.db.term_by_id(Some(taxonomy), term_id).await? else { return Ok(false) };
        let default = if taxonomy == "category" { self.site.options.u64("default_category", 1) } else { 0 };
        if default == term_id {
            return Ok(false);
        }
        let tt = term.term_taxonomy_id;
        let prefix = self.writer.table_prefix().to_owned();
        let (taxo_table, rel_table) = (format!("{prefix}term_taxonomy"), format!("{prefix}term_relationships"));
        // Children move up to the parent of the deleted term.
        if self.taxonomy_is_hierarchical(taxonomy) {
            self.writer.execute(format!("UPDATE {taxo_table} SET parent = {} WHERE parent = {term_id} AND taxonomy = '{}'", term.parent, taxonomy.replace('\'', ""))).await?;
        }
        // Posts only in this category fall back to the default category.
        let objects: Vec<u64> = sqlx_ids(self, &format!("SELECT object_id FROM {rel_table} WHERE term_taxonomy_id = {tt}")).await?;
        for object in &objects {
            if default != 0 {
                let others = self.db.terms_for_post(*object, Some(taxonomy)).await?.into_iter().filter(|t| t.term_taxonomy_id != tt).count();
                if others == 0
                    && let Some(default_term) = self.db.term_by_id(Some("category"), default).await?
                {
                    self.writer.insert_row("term_relationships", &[("object_id", Val::Int(*object as i64)), ("term_taxonomy_id", Val::Int(default_term.term_taxonomy_id as i64)), ("term_order", Val::Int(0))]).await?;
                }
            }
        }
        self.writer.delete_rows("term_relationships", "term_taxonomy_id", tt).await?;
        self.writer.delete_rows("termmeta", "term_id", term_id).await?;
        self.writer.delete_rows("term_taxonomy", "term_taxonomy_id", tt).await?;
        let remaining = sqlx_ids(self, &format!("SELECT term_taxonomy_id FROM {taxo_table} WHERE term_id = {term_id}")).await?;
        if remaining.is_empty() {
            self.writer.delete_rows("terms", "term_id", term_id).await?;
        }
        if default != 0
            && let Some(default_term) = self.db.term_by_id(Some("category"), default).await?
        {
            self.update_term_counts(&[default_term.term_taxonomy_id]).await?;
        }
        Ok(true)
    }
}

/// The first column of a query as IDs (the writer's connection sees its own writes at once).
async fn sqlx_ids(content: &Content<'_>, sql: &str) -> Result<Vec<u64>> {
    Ok(content.writer.column_u64(sql).await?)
}
