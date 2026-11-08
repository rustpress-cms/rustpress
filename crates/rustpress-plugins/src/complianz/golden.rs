//! The cookie blocker against the real plugin: `tests/data/complianz/blocker-*.json` holds the settings of a
//! scenario and what `cmplz_cookie_blocker::replace_tags()` makes of each snippet of `cases/` (made by
//! `dev/complianz-blocker-golden.sh`).

use std::path::Path;

use rustpress_phpser::{unserialize, Key, Value};
use serde_json::Value as Json;

use super::blocker::{Blocker, Placeholders};
use super::options::Options;
use super::services;

fn run(scenario: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/complianz");
    let golden: Json = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("blocker-{scenario}.json"))).expect("golden file")).expect("valid JSON");
    let text = |key: &str| golden[key].as_str().unwrap_or_default().to_owned();
    let stored = match unserialize(text("options").as_bytes()).expect("serialized options") {
        Value::Array(items) => items
            .into_iter()
            .map(|(k, v)| {
                let key = match k {
                    Key::Int(n) => n.to_string(),
                    Key::Str(b) => String::from_utf8_lossy(&b).into_owned(),
                };
                (key, v)
            })
            .collect(),
        _ => Vec::new(),
    };
    let options = Options::from_values(stored);
    let custom = unserialize(text("custom_scripts").as_bytes()).unwrap_or(Value::Bool(false));
    // The plugin's placeholder images, as empty files.
    let images = std::env::temp_dir().join(format!("rustpress-complianz-{scenario}-{}", std::process::id()));
    std::fs::create_dir_all(&images).expect("temp dir");
    for file in golden["placeholder_files"].as_array().into_iter().flatten().filter_map(Json::as_str) {
        std::fs::write(images.join(file), b"").expect("placeholder file");
    }
    let plugin_url = text("plugin_url");
    let placeholders = Placeholders {
        url: format!("{plugin_url}assets/images/placeholders/"),
        dir: images.clone(),
        theme_dir: images.join("no-theme"),
        theme_url: String::new(),
        upload_url: text("upload_url"),
        upload_dir: format!("{}/no-uploads/", images.display()),
        style: options.string("placeholder_style"),
        maps_format: options.string("google-maps-format"),
        dont_use: options.truthy("dont_use_placeholders"),
        disabled: Vec::new(),
        loaded: services::loaded(&options),
        transients: golden["image_transients"].as_object().into_iter().flatten().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned())).collect(),
    };
    let blocker = Blocker::new(&options, &custom, None, golden["cf7"].as_bool().unwrap_or(false), placeholders);
    let blocked_content_text = text("blocked_content_text");
    let mut failed = Vec::new();
    for (case, expected) in golden["cases"].as_object().expect("cases") {
        let input = std::fs::read_to_string(dir.join("cases").join(format!("{case}.html"))).expect("case");
        let actual = blocker.replace_tags(input, &blocked_content_text);
        if actual != expected.as_str().unwrap_or_default() {
            failed.push(format!("{case}:\n  expected: {expected}\n  actual:   {actual:?}"));
        }
    }
    std::fs::remove_dir_all(&images).ok();
    assert!(failed.is_empty(), "{scenario}: {} of {} cases differ\n{}", failed.len(), golden["cases"].as_object().map_or(0, |c| c.len()), failed.join("\n"));
}

#[test]
fn blocker_default() {
    run("default");
}

#[test]
fn blocker_per_service() {
    run("per-service");
}
