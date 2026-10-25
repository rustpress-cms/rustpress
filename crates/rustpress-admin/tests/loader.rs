//! Compares the registered scripts and styles with what the reference WordPress registers
//! (`dev/loader_oracle.sh` writes `tests/fixtures/loader.json`). The WordPress files come from
//! `RUSTPRESS_TEST_WP_ROOT` (default: the copy this repository is developed against).

use rustpress_admin::registry::{LoaderEnv, default_scripts, default_styles};
use serde_json::Value;

fn env() -> Option<LoaderEnv> {
    let root = std::env::var("RUSTPRESS_TEST_WP_ROOT").unwrap_or_else(|_| r"C:\work\claude\rustpress\wp".into());
    let includes = std::path::Path::new(&root).join("wp-includes");
    if !includes.join("script-loader.php").is_file() {
        eprintln!("no WordPress files at {root}, skipping");
        return None;
    }
    Some(LoaderEnv { includes, site_url: "http://localhost:8080".into(), content_url: "http://localhost:8080/wp-content".into(), version: "7.1.3".into(), rtl: false, separate_block_assets: true, compressed_tinymce: false })
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/loader.json")).expect("the fixture is JSON")
}

fn src_text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

use rustpress_admin::deps::{Arg, Dependency, Extra, Ver};

/// `$ver` as the reference prints it: `false` (default), `null` or a string.
fn ver_matches(actual: &Ver, expected: &Value) -> bool {
    match (actual, expected) {
        (Ver::Default, Value::Bool(false)) => true,
        (Ver::None, Value::Null) => true,
        (Ver::Value(a), Value::String(e)) => a == e,
        (Ver::Value(a), Value::Number(n)) => *a == n.to_string(),
        _ => false,
    }
}

fn args_matches(actual: &Option<Arg>, expected: &Value) -> bool {
    match (actual, expected) {
        (None, Value::Null) | (None, Value::Bool(false)) => true,
        (Some(Arg::Int(a)), Value::Number(n)) => n.as_i64() == Some(*a),
        (Some(Arg::Text(a)), Value::String(e)) => a == e,
        _ => false,
    }
}

/// The extra data that does not depend on the site: when and how to load.
fn extra_differences(actual: &Dependency, expected: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["strategy", "fetchpriority", "group", "rtl", "suffix", "conditional"] {
        let want = expected["extra"].get(key);
        let have = actual.extra.get(key);
        let same = match (have, want) {
            (None, None) => true,
            (Some(Extra::Text(a)), Some(Value::String(b))) => a == b,
            (Some(Extra::Int(a)), Some(Value::Number(b))) => b.as_i64() == Some(*a),
            (Some(Extra::Bool(a)), Some(Value::Bool(b))) => a == b,
            _ => false,
        };
        if !same {
            out.push(format!("extra.{key} {have:?} / {want:?}"));
        }
    }
    out
}

#[test]
fn scripts_match_the_reference() {
    let Some(env) = env() else { return };
    let scripts = default_scripts(&env);
    let mut missing = Vec::new();
    let mut different = Vec::new();
    for expected in fixture()["scripts"].as_array().unwrap() {
        let handle = expected["handle"].as_str().unwrap();
        let Some(actual) = scripts.deps.get(handle) else {
            missing.push(handle.to_owned());
            continue;
        };
        let deps: Vec<String> = expected["deps"].as_array().unwrap().iter().filter_map(|d| d.as_str().map(str::to_owned)).collect();
        if actual.src != src_text(&expected["src"]) || actual.deps != deps {
            different.push(format!("{handle}: src {:?} / {:?}, deps {:?} / {:?}", actual.src, expected["src"], actual.deps, deps));
        }
        if !ver_matches(&actual.ver, &expected["ver"]) || !args_matches(&actual.args, &expected["args"]) {
            different.push(format!("{handle}: ver {:?} / {:?}, args {:?} / {:?}", actual.ver, expected["ver"], actual.args, expected["args"]));
        }
        for difference in extra_differences(actual, expected) {
            different.push(format!("{handle}: {difference}"));
        }
    }
    assert!(missing.is_empty(), "scripts the reference registers and we do not ({}): {:?}", missing.len(), missing);
    assert!(different.is_empty(), "scripts that differ ({}):\n{}", different.len(), different.join("\n"));
}

#[test]
fn styles_match_the_reference() {
    let Some(env) = env() else { return };
    let styles = default_styles(&env);
    let mut missing = Vec::new();
    let mut different = Vec::new();
    for expected in fixture()["styles"].as_array().unwrap() {
        let handle = expected["handle"].as_str().unwrap();
        // Block styles are registered with the blocks, not by `wp_default_styles()`.
        if handle.starts_with("wp-block-") && !matches!(handle, "wp-block-library-theme" | "wp-block-editor-content" | "wp-block-template-skip-link") {
            continue;
        }
        let Some(actual) = styles.deps.get(handle) else {
            missing.push(handle.to_owned());
            continue;
        };
        let deps: Vec<String> = expected["deps"].as_array().unwrap().iter().filter_map(|d| d.as_str().map(str::to_owned)).collect();
        if actual.src != src_text(&expected["src"]) || actual.deps != deps {
            different.push(format!("{handle}: src {:?} / {:?}, deps {:?} / {:?}", actual.src, expected["src"], actual.deps, deps));
        }
        if !ver_matches(&actual.ver, &expected["ver"]) || !args_matches(&actual.args, &expected["args"]) {
            different.push(format!("{handle}: ver {:?} / {:?}, args {:?} / {:?}", actual.ver, expected["ver"], actual.args, expected["args"]));
        }
        for difference in extra_differences(actual, expected) {
            different.push(format!("{handle}: {difference}"));
        }
    }
    assert!(missing.is_empty(), "styles the reference registers and we do not ({}): {:?}", missing.len(), missing);
    assert!(different.is_empty(), "styles that differ ({}):\n{}", different.len(), different.join("\n"));
}
