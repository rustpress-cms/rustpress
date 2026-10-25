//! Compares canonical redirects with what the real WordPress decides
//! (`redirect_canonical()` and `wp_old_slug_redirect()`), recorded by
//! `dev/canonical_oracle.ps1` into `tests/fixtures/canonical_expected.tsv`.
//! Needs only the database; skips without it.

use rustpress_core::Site;
use rustpress_db::Db;

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";
const EXPECTED: &str = include_str!("fixtures/canonical_expected.tsv");

#[tokio::test]
async fn canonical_redirects_match_wordpress() {
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let Ok(db) = Db::connect(&url, "wp_").await else {
        eprintln!("reference DB not available, skipping");
        return;
    };
    let site = Site::load_at(db, std::path::Path::new(&std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()))).await.unwrap();

    let mut problems = Vec::new();
    let mut total = 0;
    for line in EXPECTED.lines().filter(|l| !l.is_empty()) {
        let (url, expected) = line.split_once('\t').unwrap_or((line, ""));
        total += 1;
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        let result = site.query(path, query).await.unwrap();
        let ours = site.canonical_redirect(path, query, &result).await.unwrap().unwrap_or_default();
        if ours != expected {
            problems.push(format!("{url}\n    wp:   {expected:?}\n    ours: {ours:?}"));
        }
    }
    assert!(total > 100, "fixture looks truncated");
    assert!(problems.is_empty(), "{} of {total} URLs differ:\n{}", problems.len(), problems.join("\n"));
}
