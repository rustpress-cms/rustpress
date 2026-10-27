use std::cell::RefCell;
use std::collections::HashMap;

use sqlx::AssertSqlSafe;
use crate::models::{TERM_COLUMNS, Term};
use crate::{Db, Result};

tokio::task_local! {
    /// The terms of the posts of a list, read in one query (`update_object_term_cache()`); filled by
    /// [`Db::prime_terms`] inside [`Db::with_term_cache`], asked first by [`Db::terms_for_post`].
    static PRIMED_TERMS: RefCell<HashMap<u64, Vec<Term>>>;
    /// The number of revisions of posts and the newest one (`wp_get_latest_revision_id_and_total_count()`).
    static PRIMED_REVISIONS: RefCell<HashMap<u64, (u64, u64)>>;
}

#[derive(sqlx::FromRow)]
struct PostTerm {
    object_id: u64,
    #[sqlx(flatten)]
    term: Term,
}

impl Db {
    /// Runs `future` with a cache of the terms of posts for this request (see [`Db::prime_terms`]).
    pub async fn with_term_cache<F: std::future::Future>(future: F) -> F::Output {
        let comments = crate::primed::PRIMED_COMMENTS.scope(RefCell::new(HashMap::new()), crate::primed::PRIMED_APPROVED.scope(RefCell::new(HashMap::new()), future));
        let posts = crate::primed::PRIMED_POSTS.scope(RefCell::new(HashMap::new()), comments);
        let user_meta = crate::primed::PRIMED_USER_META.scope(RefCell::new(HashMap::new()), posts);
        let meta = crate::primed::PRIMED_META.scope(RefCell::new(HashMap::new()), user_meta);
        PRIMED_TERMS.scope(RefCell::new(HashMap::new()), PRIMED_REVISIONS.scope(RefCell::new(HashMap::new()), meta)).await
    }

    /// How many revisions a post has and the ID of the newest (by ID), from the primed cache or one query.
    pub async fn revision_summary(&self, post_id: u64) -> Result<(u64, u64)> {
        if let Ok(Some(summary)) = PRIMED_REVISIONS.try_with(|cache| cache.borrow().get(&post_id).copied()) {
            return Ok(summary);
        }
        Ok(self.revision_summaries(&[post_id]).await?.get(&post_id).copied().unwrap_or((0, 0)))
    }

    async fn revision_summaries(&self, post_ids: &[u64]) -> Result<HashMap<u64, (u64, u64)>> {
        if post_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let marks = vec!["?"; post_ids.len()].join(", ");
        let sql = format!(
            "SELECT post_parent, COUNT(*), MAX(ID) FROM {} WHERE post_type = 'revision' AND post_status = 'inherit' AND post_parent IN ({marks}) GROUP BY post_parent",
            self.prefix.table("posts")
        );
        let mut query = sqlx::query_as::<_, (u64, i64, u64)>(AssertSqlSafe(sql));
        for id in post_ids {
            query = query.bind(*id);
        }
        Ok(query.fetch_all(&self.pool).await?.into_iter().map(|(parent, count, latest)| (parent, (count as u64, latest))).collect())
    }

    /// [`Db::revision_summary`] for all `post_ids` in one query, for the scope of [`Db::with_term_cache`].
    pub async fn prime_revisions(&self, post_ids: &[u64]) -> Result<()> {
        if PRIMED_REVISIONS.try_with(|_| ()).is_err() || post_ids.is_empty() {
            return Ok(());
        }
        let found = self.revision_summaries(post_ids).await?;
        let _ = PRIMED_REVISIONS.try_with(|cache| {
            let mut cache = cache.borrow_mut();
            for id in post_ids {
                cache.insert(*id, found.get(id).copied().unwrap_or((0, 0)));
            }
        });
        Ok(())
    }

    /// Reads the terms of all `post_ids` in one query for the [`Db::with_term_cache`] around the caller. Without that
    /// scope it does nothing. Lists call it before they prepare their posts.
    pub async fn prime_terms(&self, post_ids: &[u64]) -> Result<()> {
        let Ok(missing) = PRIMED_TERMS.try_with(|cache| {
            let cache = cache.borrow();
            post_ids.iter().copied().filter(|id| !cache.contains_key(id)).collect::<Vec<u64>>()
        }) else {
            return Ok(());
        };
        if missing.is_empty() {
            return Ok(());
        }
        let marks = vec!["?"; missing.len()].join(", ");
        let sql = format!(
            "SELECT tr.object_id, {TERM_COLUMNS} FROM {} JOIN {} tr ON tr.term_taxonomy_id = tt.term_taxonomy_id \
             WHERE tr.object_id IN ({marks}) ORDER BY tr.object_id, tr.term_order, t.term_id",
            self.term_from(),
            self.prefix.table("term_relationships"),
        );
        let mut query = sqlx::query_as::<_, PostTerm>(AssertSqlSafe(sql));
        for id in &missing {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await?;
        let _ = PRIMED_TERMS.try_with(|cache| {
            let mut cache = cache.borrow_mut();
            for id in &missing {
                cache.entry(*id).or_default();
            }
            for row in rows {
                cache.entry(row.object_id).or_default().push(row.term);
            }
        });
        Ok(())
    }

    /// Forgets the primed terms of a post (after its terms changed).
    pub fn forget_primed_terms(post_id: u64) {
        let _ = PRIMED_TERMS.try_with(|cache| cache.borrow_mut().remove(&post_id));
    }

    pub(crate) fn term_from(&self) -> String {
        format!(
            "{} t JOIN {} tt ON tt.term_id = t.term_id",
            self.prefix.table("terms"),
            self.prefix.table("term_taxonomy")
        )
    }

    /// Terms attached to a post, optionally restricted to one taxonomy.
    pub async fn terms_for_post(&self, post_id: u64, taxonomy: Option<&str>) -> Result<Vec<Term>> {
        if let Ok(Some(terms)) = PRIMED_TERMS.try_with(|cache| cache.borrow().get(&post_id).cloned()) {
            return Ok(terms.into_iter().filter(|t| taxonomy.is_none_or(|tax| t.taxonomy == tax)).collect());
        }
        let mut sql = format!(
            "SELECT {TERM_COLUMNS} FROM {} JOIN {} tr ON tr.term_taxonomy_id = tt.term_taxonomy_id \
             WHERE tr.object_id = ?",
            self.term_from(),
            self.prefix.table("term_relationships"),
        );
        if taxonomy.is_some() {
            sql.push_str(" AND tt.taxonomy = ?");
        }
        sql.push_str(" ORDER BY tr.term_order, t.term_id");
        let mut query = sqlx::query_as(AssertSqlSafe(sql)).bind(post_id);
        if let Some(taxonomy) = taxonomy {
            query = query.bind(taxonomy);
        }
        Ok(query.fetch_all(&self.pool).await?)
    }

    pub async fn term_by_slug(&self, taxonomy: &str, slug: &str) -> Result<Option<Term>> {
        let sql = format!(
            "SELECT {TERM_COLUMNS} FROM {} WHERE tt.taxonomy = ? AND t.slug = ?",
            self.term_from()
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(taxonomy).bind(slug).fetch_optional(&self.pool).await?)
    }

    /// A term by its `term_id`, optionally restricted to a taxonomy.
    pub async fn term_by_id(&self, taxonomy: Option<&str>, term_id: u64) -> Result<Option<Term>> {
        let mut sql = format!("SELECT {TERM_COLUMNS} FROM {} WHERE t.term_id = ?", self.term_from());
        if taxonomy.is_some() {
            sql.push_str(" AND tt.taxonomy = ?");
        }
        let mut query = sqlx::query_as(AssertSqlSafe(sql)).bind(term_id);
        if let Some(taxonomy) = taxonomy {
            query = query.bind(taxonomy);
        }
        Ok(query.fetch_optional(&self.pool).await?)
    }

    pub async fn terms(&self, taxonomy: &str) -> Result<Vec<Term>> {
        let sql = format!(
            "SELECT {TERM_COLUMNS} FROM {} WHERE tt.taxonomy = ? ORDER BY t.name",
            self.term_from()
        );
        Ok(sqlx::query_as(AssertSqlSafe(sql)).bind(taxonomy).fetch_all(&self.pool).await?)
    }
}
