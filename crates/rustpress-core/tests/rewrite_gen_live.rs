//! The rules generated from the stored permalink structure are the ones WordPress stored.
//!
//! Runs against the reference database (`RUSTPRESS_TEST_DB` overrides the URL); without one the test passes.
//! With `RUSTPRESS_BASE_RULES` (a file with the serialized rules of an earlier structure) and
//! `RUSTPRESS_OLD_FRONT` the custom types are read from those rules and the rules for the structure
//! now in the database are generated from them: `dev/rewrite-check.ps1` changes the structure of the
//! reference WordPress and runs this for each case.

use rustpress_core::Site;
use rustpress_core::rewrite::RewriteRules;
use rustpress_core::rewrite_gen::{RewriteInput, rewrite_rules, serialize_rules};
use rustpress_core::types::Types;
use rustpress_db::Db;

#[tokio::test]
async fn generated_rules_equal_the_stored_ones() {
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let Ok(db) = Db::connect(&url, "wp_").await else {
        eprintln!("no test database, skipping");
        return;
    };
    let site = Site::load(db).await.unwrap();
    let stored = site.options.get("rewrite_rules").map(str::to_owned).unwrap_or_default();
    let structure = site.options.string("permalink_structure", "");
    let mut old_front = structure.find('%').map_or("", |at| &structure[..at]).to_owned();
    let mut types = site.types.clone();
    if let Ok(path) = std::env::var("RUSTPRESS_BASE_RULES") {
        let base = rustpress_phpser::unserialize(std::fs::read(path).unwrap().as_slice()).unwrap();
        let base = RewriteRules::from_option(&base);
        let post_types = site.db.used_post_types().await.unwrap();
        let taxonomies = site.db.used_taxonomies().await.unwrap();
        let relations = site.db.taxonomy_post_types().await.unwrap();
        let with_parents = site.db.taxonomies_with_parents().await.unwrap();
        types = Types::infer(&base, &post_types, &taxonomies, &relations, &with_parents);
        old_front = std::env::var("RUSTPRESS_OLD_FRONT").unwrap_or_default();
    }
    let home = site.settings.home.trim_end_matches('/');
    let input = RewriteInput {
        permalink_structure: &structure,
        old_front: &old_front,
        category_base: &site.options.string("category_base", ""),
        tag_base: &site.options.string("tag_base", ""),
        page_on_front: site.options.u64("page_on_front", 0),
        home_at_root: home.split_once("://").is_none_or(|(_, rest)| !rest.contains('/')),
        blog_public: site.options.string("blog_public", "1") != "0",
        types: &types,
    };
    let generated = rewrite_rules(&input);
    let expected = rustpress_phpser::unserialize(stored.as_bytes()).unwrap_or(rustpress_phpser::Value::Array(Vec::new()));
    let expected: Vec<(String, String)> = expected
        .as_array()
        .unwrap_or_default()
        .iter()
        .map(|(k, v)| match k {
            rustpress_phpser::Key::Str(s) => (String::from_utf8_lossy(s).into_owned(), v.as_str().unwrap().to_owned()),
            rustpress_phpser::Key::Int(i) => (i.to_string(), v.as_str().unwrap().to_owned()),
        })
        .collect();
    for (i, (g, e)) in generated.iter().zip(&expected).enumerate() {
        assert_eq!(g, e, "structure {structure:?}: rule {i} differs");
    }
    assert_eq!(generated.len(), expected.len(), "structure {structure:?}");
    if !stored.is_empty() {
        assert_eq!(serialize_rules(&generated), stored);
    }
}
