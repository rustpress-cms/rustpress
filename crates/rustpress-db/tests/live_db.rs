//! Runs against the reference WordPress database (`dev/up.sh` + `dev/seed.sh`).
//!
//! `RUSTPRESS_TEST_DB` overrides the URL. Without a reachable database the
//! tests print a note and pass, so `cargo test` works everywhere.

use rustpress_db::{Db, Order, OrderBy, PostQuery};

async fn db() -> Option<Db> {
    let url = std::env::var("RUSTPRESS_TEST_DB")
        .unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    match Db::connect(&url, "wp_").await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("no test database ({e}), skipping");
            None
        }
    }
}

macro_rules! db_or_skip {
    () => {
        match db().await {
            Some(db) => db,
            None => return,
        }
    };
}

#[tokio::test]
async fn reads_options() {
    let db = db_or_skip!();
    assert_eq!(db.option("template").await.unwrap().as_deref(), Some("twentytwentyfive"));
    assert_eq!(db.option("does_not_exist").await.unwrap(), None);
    let autoload = db.autoload_options().await.unwrap();
    assert!(autoload.iter().any(|(name, _)| name == "siteurl"));
    assert!(autoload.iter().all(|(name, _)| name != "rustpress_seeded_nonexistent"));
}

#[tokio::test]
async fn lists_only_published_posts_by_default() {
    let db = db_or_skip!();
    let query = PostQuery::default();
    // 12 test posts + hello-world + klassisch + geschuetzt + mehrseitig + gleich-a/b + mehr-bloecke; the draft is hidden.
    assert_eq!(db.count_posts(&query).await.unwrap(), 31);
    let page = db.posts(&PostQuery { limit: Some(5), ..query.clone() }).await.unwrap();
    assert_eq!(page.len(), 5);
    assert!(page.iter().all(|p| p.post_status == "publish" && p.post_type == "post"));
    // Newest first, stable under identical timestamps.
    assert!(page.windows(2).all(|w| (&w[0].post_date, w[0].id) >= (&w[1].post_date, w[1].id)));

    let page2 = db.posts(&PostQuery { limit: Some(5), offset: 5, ..query }).await.unwrap();
    assert!(page.iter().all(|a| page2.iter().all(|b| a.id != b.id)));
}

#[tokio::test]
async fn finds_posts_by_name_and_id() {
    let db = db_or_skip!();
    let query = PostQuery { name: Some("testbeitrag-1".into()), ..PostQuery::default() };
    let post = db.posts(&query).await.unwrap().pop().expect("seeded post");
    assert_eq!(post.post_title, "Testbeitrag 1");
    assert!(post.post_content.contains("<!-- wp:paragraph -->"));
    assert!(post.post_content.contains("äöü"), "umlauts survive: {}", post.post_content);

    assert_eq!(db.post(post.id).await.unwrap().unwrap().post_name, "testbeitrag-1");
    assert!(db.post(u64::MAX).await.unwrap().is_none());

    // `db.post` ignores type/status, so drafts are reachable for previews.
    let draft = db.posts(&PostQuery { name: Some("entwurf".into()), statuses: vec![], ..PostQuery::default() }).await.unwrap();
    assert_eq!(draft.len(), 1);
    assert_eq!(draft[0].post_status, "draft");
}

#[tokio::test]
async fn pages_and_hierarchy() {
    let db = db_or_skip!();
    let about = db
        .posts(&PostQuery { post_types: vec!["page".into()], name: Some("ueber-uns".into()), ..PostQuery::default() })
        .await
        .unwrap()
        .pop()
        .unwrap();
    let children = db
        .posts(&PostQuery { post_types: vec!["page".into()], parent: Some(about.id), ..PostQuery::default() })
        .await
        .unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].post_name, "team");
}

#[tokio::test]
async fn filters_by_term_search_and_date() {
    let db = db_or_skip!();
    let rust = db.term_by_slug("category", "rust").await.unwrap().expect("rust category");
    assert_eq!(rust.name, "Rust");
    let tech = db.term_by_slug("category", "tech").await.unwrap().unwrap();
    assert_eq!(rust.parent, tech.term_id);

    let in_rust = PostQuery { tax_groups: vec![vec![rust.term_taxonomy_id]], ..PostQuery::default() };
    assert_eq!(db.count_posts(&in_rust).await.unwrap(), 12);
    let in_tech = PostQuery { tax_groups: vec![vec![tech.term_taxonomy_id]], ..PostQuery::default() };
    assert_eq!(db.count_posts(&in_tech).await.unwrap(), 0);

    let found = PostQuery { search: vec!["Beitrag 7".into()], ..PostQuery::default() };
    assert_eq!(db.count_posts(&found).await.unwrap(), 1);
    // LIKE wildcards in user input are literal.
    let wildcard = PostQuery { search: vec!["%Beitrag%".into()], ..PostQuery::default() };
    assert_eq!(db.count_posts(&wildcard).await.unwrap(), 0);

    let year_hit = PostQuery { year: Some(2026), ..PostQuery::default() };
    assert_eq!(db.count_posts(&year_hit).await.unwrap(), 31);
    let year_miss = PostQuery { year: Some(1999), ..PostQuery::default() };
    assert_eq!(db.count_posts(&year_miss).await.unwrap(), 0);

    let by_title = PostQuery { order_by: OrderBy::Title, order: Order::Asc, limit: Some(1), ..PostQuery::default() };
    assert_eq!(db.posts(&by_title).await.unwrap()[0].post_title, "Abfrage-Bloecke");
}

#[tokio::test]
async fn terms_meta_comments_users() {
    let db = db_or_skip!();
    let post = db
        .posts(&PostQuery { name: Some("testbeitrag-1".into()), ..PostQuery::default() })
        .await
        .unwrap()
        .remove(0);

    let cats = db.terms_for_post(post.id, Some("category")).await.unwrap();
    let mut names: Vec<_> = cats.iter().map(|t| t.name.as_str()).collect();
    names.sort();
    assert_eq!(names, ["News", "Rust"]);
    let tags = db.terms_for_post(post.id, Some("post_tag")).await.unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(db.terms_for_post(post.id, None).await.unwrap().len(), 4);
    assert_eq!(db.terms("category").await.unwrap().len(), 4); // + Uncategorized

    let meta = db.post_meta(post.id).await.unwrap();
    let demo = meta.iter().find(|m| m.meta_key.as_deref() == Some("demo_array")).expect("demo_array");
    assert!(demo.meta_value.as_deref().unwrap().starts_with("a:3:{s:1:\"a\";i:1;"));

    // Only the approved comment is visible.
    let comments = db.approved_comments(post.id).await.unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0].comment_content, "Guter Beitrag");

    let author = db.user(post.post_author).await.unwrap().expect("author");
    assert_eq!(author.user_login, "admin");
    assert!(db.user_by_nicename("admin").await.unwrap().is_some());
}

#[tokio::test]
async fn connection_is_read_only() {
    let db = db_or_skip!();
    let err = sqlx::query("UPDATE wp_options SET option_value = option_value WHERE option_name = 'siteurl'")
        .execute(db.pool())
        .await
        .expect_err("writes must be rejected");
    assert!(err.to_string().to_lowercase().contains("read"), "unexpected error: {err}");
}
