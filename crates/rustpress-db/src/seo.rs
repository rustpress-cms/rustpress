//! Reads for the XML sitemaps of SEO plugins (Yoast SEO's `WPSEO_*_Sitemap_Provider`).

use sqlx::{MySql, QueryBuilder};

use crate::models::{POST_COLUMNS, Post};
use crate::{Db, Result};

fn push_list(qb: &mut QueryBuilder<MySql>, values: &[&str]) {
    let mut separated = qb.separated(", ");
    for value in values {
        separated.push_bind(value.to_string());
    }
    separated.push_unseparated(")");
}

impl Db {
    /// The `WHERE` of `WPSEO_Post_Type_Sitemap_Provider::get_sql_where_clause()`: the statuses (for attachments those
    /// of the parent, which must not have a password), no password, a real date.
    fn push_sitemap_where(&self, qb: &mut QueryBuilder<MySql>, post_type: &str, statuses: &[&str]) {
        let posts = self.prefix.table("posts");
        if post_type == "attachment" {
            qb.push(format!(" LEFT JOIN {posts} AS p2 ON ({posts}.post_parent = p2.ID) WHERE p2.post_status IN ("));
            let parents: Vec<&str> = statuses.iter().copied().filter(|s| *s != "inherit").collect();
            push_list(qb, &parents);
            qb.push(" AND p2.post_password = ''");
        } else {
            qb.push(format!(" WHERE {posts}.post_status IN ("));
            push_list(qb, statuses);
        }
        qb.push(format!(" AND {posts}.post_type = ")).push_bind(post_type.to_owned());
        qb.push(format!(" AND {posts}.post_password = '' AND {posts}.post_date != '0000-00-00 00:00:00'"));
    }

    /// `get_post_type_count()`.
    pub async fn sitemap_post_count(&self, post_type: &str, statuses: &[&str]) -> Result<u64> {
        let posts = self.prefix.table("posts");
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT COUNT({posts}.ID) FROM {posts}"));
        self.push_sitemap_where(&mut qb, post_type, statuses);
        let count: i64 = qb.build_query_scalar().fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// `get_posts()`: a page of the posts of a sitemap, least recently modified first.
    pub async fn sitemap_posts(&self, post_type: &str, statuses: &[&str], limit: u64, offset: u64) -> Result<Vec<Post>> {
        let posts = self.prefix.table("posts");
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT {} FROM (SELECT {posts}.ID FROM {posts}", POST_COLUMNS.replacen("ID, ", "l.ID, ", 1)));
        self.push_sitemap_where(&mut qb, post_type, statuses);
        qb.push(format!(" ORDER BY {posts}.post_modified ASC LIMIT ")).push_bind(limit).push(" OFFSET ").push_bind(offset);
        qb.push(format!(") o JOIN {posts} l ON l.ID = o.ID"));
        Ok(qb.build_query_as().fetch_all(&self.pool).await?)
    }

    /// `WPSEO_Sitemaps::get_last_modified_gmt( $types, true )`: the newest `post_modified_gmt` per post type.
    pub async fn last_modified_by_type(&self, post_types: &[&str], statuses: &[&str]) -> Result<Vec<(String, String)>> {
        if post_types.is_empty() {
            return Ok(Vec::new());
        }
        let posts = self.prefix.table("posts");
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT post_type, CAST(MAX(post_modified_gmt) AS CHAR) AS date FROM {posts} WHERE post_status IN ("));
        push_list(&mut qb, statuses);
        qb.push(" AND post_type IN (");
        push_list(&mut qb, post_types);
        qb.push(" GROUP BY post_type ORDER BY date DESC");
        let rows: Vec<(String, Option<String>)> = qb.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().filter_map(|(t, d)| d.map(|d| (t, d))).collect())
    }

    /// The `post_modified_gmt` of every `max`-th post of a type (`get_all_dates_using_with_clause()`).
    pub async fn sitemap_page_dates(&self, post_type: &str, statuses: &[&str], max: u64) -> Result<Vec<String>> {
        let posts = self.prefix.table("posts");
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT CAST(post_modified_gmt AS CHAR) FROM {posts} WHERE post_status IN ("));
        push_list(&mut qb, statuses);
        qb.push(" AND post_type = ").push_bind(post_type.to_owned()).push(" ORDER BY post_modified_gmt");
        let all: Vec<Option<String>> = qb.build_query_scalar().fetch_all(&self.pool).await?;
        Ok(all.into_iter().enumerate().filter(|(i, _)| max > 0 && (*i as u64 + 1).is_multiple_of(max)).map(|(_, d)| d.unwrap_or_default()).collect())
    }

    /// The newest `post_modified_gmt` of the posts in a term (`WPSEO_Taxonomy_Sitemap_Provider::get_sitemap_links()`).
    pub async fn term_last_modified(&self, taxonomy: &str, term_id: u64, statuses: &[&str]) -> Result<Option<String>> {
        let mut qb = QueryBuilder::<MySql>::new(format!(
            "SELECT CAST(MAX(p.post_modified_gmt) AS CHAR) FROM {} AS p INNER JOIN {} AS term_rel ON term_rel.object_id = p.ID \
             INNER JOIN {} AS term_tax ON term_tax.term_taxonomy_id = term_rel.term_taxonomy_id AND term_tax.taxonomy = ",
            self.prefix.table("posts"),
            self.prefix.table("term_relationships"),
            self.prefix.table("term_taxonomy")
        ));
        qb.push_bind(taxonomy.to_owned()).push(" AND term_tax.term_id = ").push_bind(term_id).push(" WHERE p.post_status IN (");
        push_list(&mut qb, statuses);
        qb.push(" AND p.post_password = ''");
        Ok(qb.build_query_scalar::<Option<String>>().fetch_one(&self.pool).await?)
    }

    /// The `post_modified_gmt` of the most recently modified published post of the types in one of the terms
    /// (`WP_Query` with `orderby => modified`, `posts_per_page => 1`).
    pub async fn latest_modified_in_terms(&self, post_types: &[&str], taxonomy: &str, term_ids: &[u64]) -> Result<Option<String>> {
        if post_types.is_empty() || term_ids.is_empty() {
            return Ok(None);
        }
        let posts = self.prefix.table("posts");
        let mut qb = QueryBuilder::<MySql>::new(format!(
            "SELECT CAST(p.post_modified_gmt AS CHAR) FROM {posts} AS p WHERE p.ID IN (SELECT tr.object_id FROM {} tr JOIN {} tt ON tt.term_taxonomy_id = tr.term_taxonomy_id WHERE tt.taxonomy = ",
            self.prefix.table("term_relationships"),
            self.prefix.table("term_taxonomy")
        ));
        qb.push_bind(taxonomy.to_owned()).push(" AND tt.term_id IN (");
        let mut separated = qb.separated(", ");
        for id in term_ids {
            separated.push_bind(*id);
        }
        separated.push_unseparated(")) AND p.post_type IN (");
        push_list(&mut qb, post_types);
        qb.push(" AND p.post_status = 'publish' ORDER BY p.post_modified DESC LIMIT 1");
        Ok(qb.build_query_scalar::<Option<String>>().fetch_optional(&self.pool).await?.flatten())
    }

    /// The users of Yoast's author sitemap: not `wpseo_noindex_author = on`,
    /// with published posts of the types (any user when `with_posts` is empty), newest profile first. Returns the ID
    /// and the timestamp, `None` for a user without one yet.
    pub async fn author_sitemap_users(&self, with_posts: &[&str]) -> Result<Vec<(u64, Option<i64>)>> {
        let users = self.prefix.table("users");
        let usermeta = self.prefix.table("usermeta");
        let mut qb = QueryBuilder::<MySql>::new(format!(
            "SELECT u.ID, CAST(m.meta_value AS SIGNED) FROM {users} u LEFT JOIN {usermeta} m ON m.user_id = u.ID AND m.meta_key = '_yoast_wpseo_profile_updated' \
             WHERE NOT EXISTS (SELECT 1 FROM {usermeta} n WHERE n.user_id = u.ID AND n.meta_key = 'wpseo_noindex_author' AND n.meta_value = 'on')"
        ));
        if !with_posts.is_empty() {
            qb.push(format!(" AND EXISTS (SELECT 1 FROM {} p WHERE p.post_author = u.ID AND p.post_status = 'publish' AND p.post_type IN (", self.prefix.table("posts")));
            push_list(&mut qb, with_posts);
            qb.push(")");
        }
        qb.push(" ORDER BY CAST(m.meta_value AS SIGNED) DESC, u.ID ASC");
        Ok(qb.build_query_as().fetch_all(&self.pool).await?)
    }

    /// Image attachments for a `[gallery]`: those in `ids`, or the children of a post, newest first.
    pub async fn gallery_attachments(&self, include: &[u64], parent: u64, exclude: &[u64]) -> Result<Vec<u64>> {
        let posts = self.prefix.table("posts");
        let mut qb = QueryBuilder::<MySql>::new(format!("SELECT ID FROM {posts} WHERE post_type = 'attachment' AND post_status = 'inherit' AND post_mime_type LIKE 'image/%'"));
        if !include.is_empty() {
            qb.push(" AND ID IN (");
            let mut separated = qb.separated(", ");
            for id in include {
                separated.push_bind(*id);
            }
            separated.push_unseparated(")");
        } else {
            qb.push(" AND post_parent = ").push_bind(parent);
            if !exclude.is_empty() {
                qb.push(" AND ID NOT IN (");
                let mut separated = qb.separated(", ");
                for id in exclude {
                    separated.push_bind(*id);
                }
                separated.push_unseparated(")");
            }
        }
        qb.push(" ORDER BY post_date DESC, ID DESC");
        Ok(qb.build_query_scalar().fetch_all(&self.pool).await?)
    }
}
