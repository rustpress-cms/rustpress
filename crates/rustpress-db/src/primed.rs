//! Post rows and post meta read ahead for the posts of a list (`update_postmeta_cache()`,
//! `update_post_thumbnail_cache()`), kept for one request in task-local maps. [`Db::with_term_cache`] opens the
//! scope; [`Db::post`], [`Db::post_meta`] and [`Db::post_meta_value`] look here first.

use std::cell::RefCell;
use std::collections::HashMap;

use sqlx::AssertSqlSafe;

use crate::models::{Post, PostMeta};
use crate::{Db, PostQuery, Result};

tokio::task_local! {
    pub(crate) static PRIMED_META: RefCell<HashMap<u64, Vec<PostMeta>>>;
    pub(crate) static PRIMED_POSTS: RefCell<HashMap<u64, Option<Post>>>;
}

pub(crate) fn primed_meta(post_id: u64) -> Option<Vec<PostMeta>> {
    PRIMED_META.try_with(|cache| cache.borrow().get(&post_id).cloned()).ok().flatten()
}

pub(crate) fn primed_post(id: u64) -> Option<Option<Post>> {
    PRIMED_POSTS.try_with(|cache| cache.borrow().get(&id).cloned()).ok().flatten()
}

impl Db {
    /// The meta of all `post_ids` in one query, for the request's cache. Without the scope it does nothing.
    pub async fn prime_post_meta(&self, post_ids: &[u64]) -> Result<()> {
        let Ok(missing) = PRIMED_META.try_with(|cache| {
            let cache = cache.borrow();
            post_ids.iter().copied().filter(|id| !cache.contains_key(id)).collect::<Vec<u64>>()
        }) else {
            return Ok(());
        };
        if missing.is_empty() {
            return Ok(());
        }
        let marks = vec!["?"; missing.len()].join(", ");
        let sql = format!("SELECT meta_id, post_id, meta_key, meta_value FROM {} WHERE post_id IN ({marks}) ORDER BY meta_id", self.prefix.table("postmeta"));
        let mut query = sqlx::query_as::<_, PostMeta>(AssertSqlSafe(sql));
        for id in &missing {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await?;
        let _ = PRIMED_META.try_with(|cache| {
            let mut cache = cache.borrow_mut();
            for id in &missing {
                cache.entry(*id).or_default();
            }
            for row in rows {
                cache.entry(row.post_id).or_default().push(row);
            }
        });
        Ok(())
    }

    /// The rows of `ids` (of any type and status) in one query, for the request's cache.
    pub async fn prime_posts(&self, ids: &[u64]) -> Result<()> {
        let Ok(missing) = PRIMED_POSTS.try_with(|cache| {
            let cache = cache.borrow();
            ids.iter().copied().filter(|id| *id != 0 && !cache.contains_key(id)).collect::<Vec<u64>>()
        }) else {
            return Ok(());
        };
        if missing.is_empty() {
            return Ok(());
        }
        let query = PostQuery { ids: missing.clone(), post_types: Vec::new(), statuses: Vec::new(), ..PostQuery::default() };
        let rows = self.posts(&query).await?;
        let _ = PRIMED_POSTS.try_with(|cache| {
            let mut cache = cache.borrow_mut();
            for id in &missing {
                cache.insert(*id, rows.iter().find(|p| p.id == *id).cloned());
            }
        });
        Ok(())
    }

    /// What `WP_Query` primes for the posts of a loop: their terms, their meta, and the rows and meta of their
    /// featured images.
    pub async fn prime_loop(&self, posts: &[Post]) -> Result<()> {
        let ids: Vec<u64> = posts.iter().map(|p| p.id).collect();
        // The rows themselves are at hand: `get_post( $id )` of a post of the loop needs no query.
        let _ = PRIMED_POSTS.try_with(|cache| {
            let mut cache = cache.borrow_mut();
            for post in posts {
                cache.entry(post.id).or_insert_with(|| Some(post.clone()));
            }
        });
        self.prime_terms(&ids).await?;
        self.prime_post_meta(&ids).await?;
        let thumbnails: Vec<u64> = ids
            .iter()
            .filter_map(|id| primed_meta(*id))
            .filter_map(|meta| meta.into_iter().find(|m| m.meta_key.as_deref() == Some("_thumbnail_id")).and_then(|m| m.meta_value).and_then(|v| v.trim().parse::<u64>().ok()))
            .filter(|id| *id != 0)
            .collect();
        self.prime_posts(&thumbnails).await?;
        self.prime_post_meta(&thumbnails).await?;
        Ok(())
    }
}
