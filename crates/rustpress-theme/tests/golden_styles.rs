//! Compares the generated global styles with the `global-styles-inline-css`
//! of the real WordPress, rule by rule (order-insensitive).
//!
//! Same prerequisites as `golden_pages.rs`; skips without the reference stack.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use rustpress_blocks::Registry;
use rustpress_core::Site;
use rustpress_db::Db;
use rustpress_theme::{Theme, render_body};
use rustpress_theme::global_styles::GlobalStyles;

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";

fn fetch(path: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    write!(stream, "GET {path} HTTP/1.0\r\nHost: localhost:8080\r\nConnection: close\r\n\r\n")?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

fn style_block(html: &str, id: &str) -> Option<String> {
    let open = format!("<style id=\"{id}\">");
    let start = html.find(&open)? + open.len();
    let end = html[start..].find("</style>")? + start;
    let css = &html[start..end];
    Some(css.split("/*# sourceURL").next().unwrap_or(css).to_owned())
}

/// Splits CSS into top-level rules (handles `@media { ... }`) with normalised whitespace.
fn rules(css: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for c in css.chars() {
        current.push(c);
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    out.push(normalise(&current));
                    current.clear();
                }
            }
            _ => {}
        }
    }
    out
}

fn normalise(rule: &str) -> String {
    let mut collapsed = String::new();
    let mut last_space = false;
    for c in rule.trim().chars() {
        if c.is_whitespace() {
            if !last_space {
                collapsed.push(' ');
            }
            last_space = true;
        } else {
            collapsed.push(c);
            last_space = false;
        }
    }
    collapsed.replace("{ ", "{").replace(" }", "}").replace("; ", ";")
}

#[tokio::test]
async fn global_styles_match_wordpress() {
    let root = PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let (Ok(db), true) = (Db::connect(&url, "wp_").await, root.join("wp-includes").is_dir()) else {
        eprintln!("reference stack not available, skipping");
        return;
    };
    let Ok(page) = fetch("/ueber-uns/") else {
        eprintln!("WordPress not reachable, skipping");
        return;
    };
    let reference = style_block(&page, "global-styles-inline-css").expect("global styles in reference page");

    let site = Site::load_at(db, &root).await.unwrap();
    let theme = Theme::load(&site, &root).await.unwrap();
    let registry = Registry::new(root.join("wp-includes").join("blocks"));
    let result = site.query("/ueber-uns/", "").await.unwrap();
    let rendered = render_body(&theme, &site, &result, &registry).await.unwrap();
    let ours = GlobalStyles::new(&theme.json, &registry).stylesheet(Some(&rendered.state.used_blocks));

    let theirs: Vec<String> = rules(&reference);
    let mine: Vec<String> = rules(&ours);
    let theirs_set: BTreeSet<&String> = theirs.iter().collect();
    let mine_set: BTreeSet<&String> = mine.iter().collect();
    let missing: Vec<&&String> = theirs_set.difference(&mine_set).collect();
    let extra: Vec<&&String> = mine_set.difference(&theirs_set).collect();

    let show = |list: &[&&String]| list.iter().take(12).map(|r| format!("  {}", r.chars().take(240).collect::<String>())).collect::<Vec<_>>().join("\n");
    assert!(
        missing.is_empty() && extra.is_empty(),
        "{} rules in WordPress ({} unique) vs {} in ours ({} unique)\nmissing in ours ({}):\n{}\nextra in ours ({}):\n{}",
        theirs.len(),
        theirs_set.len(),
        mine.len(),
        mine_set.len(),
        missing.len(),
        show(&missing),
        extra.len(),
        show(&extra)
    );
}
