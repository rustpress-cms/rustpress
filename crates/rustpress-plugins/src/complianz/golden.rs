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

/// The legal documents against the real plugin: `tests/data/complianz/documents.json` holds the settings, dates and
/// table rows of each scenario and what `get_document_html()` and `cmplz_used_cookies()` make of them (made by
/// `dev/complianz-document-golden.sh`). `antispambot()` is random, so the addresses are compared decoded.
#[test]
fn documents() {
    use super::documents::{self, CookieData, Inputs, Row};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/complianz/documents.json");
    let scenarios: Json = serde_json::from_str(&std::fs::read_to_string(path).expect("golden file")).expect("valid JSON");
    let rows = |list: &Json| -> Vec<Row> {
        list.as_array()
            .into_iter()
            .flatten()
            .map(|row| row.as_object().into_iter().flatten().map(|(k, v)| (k.clone(), Some(v.as_str().map_or_else(|| v.to_string(), str::to_owned)))).collect())
            .collect()
    };
    let obfuscated = regex::Regex::new(r#"<span class="cmplz-obfuscate">.*?</span>[^<]*</span>"#).expect("valid regex");
    let entity = regex::Regex::new(r"&#(\d+);").expect("valid regex");
    let decode = |html: &str| {
        obfuscated
            .replace_all(html, |c: &regex::Captures<'_>| {
                entity.replace_all(&c[0], |e: &regex::Captures<'_>| e[1].parse().ok().and_then(char::from_u32).map(String::from).unwrap_or_default()).into_owned()
            })
            .into_owned()
    };
    let mut failed = Vec::new();
    let mut count = 0;
    for scenario in scenarios.as_array().expect("scenarios") {
        let name = scenario["name"].as_str().unwrap_or_default();
        let golden = &scenario["golden"];
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
        let (cookies, services) = (rows(&scenario["cookies"]), rows(&scenario["services"]));
        let plugin_url = text("plugin_url");
        let data = CookieData { options: &options, cookies: &cookies, services: &services, locale: "en_US", shredder_list: None, plugin_url: &plugin_url };
        let used = documents::used_cookies(&data);
        if used != text("used_cookies") {
            failed.push(format!("{name} used cookies:\n  expected: {:?}\n  actual:   {used:?}", text("used_cookies")));
        }
        if !options.is("compile_statistics", "google-tag-manager") {
            let only_functional = !documents::uses_non_functional_cookies(&data, golden["statistics_privacy_friendly"].as_bool().unwrap_or(false));
            if Some(only_functional) != golden["callbacks"]["cmplz_uses_only_functional_cookies"].as_bool() {
                failed.push(format!("{name}: uses_only_functional_cookies is {only_functional}"));
            }
        }
        for (key, ts) in [("publish_date", "publish_date"), ("checked_date", "documents_update_date")] {
            let date = documents::localize_date(scenario[ts].as_i64().unwrap_or(0), &text("date_format"), &|t| t.to_owned());
            if date != text(key) {
                failed.push(format!("{name} {key}: expected {:?}, actual {date:?}", text(key)));
            }
        }
        let callback = |func: &str| golden["callbacks"].get(func).and_then(Json::as_bool);
        let page_url = |_: &str, _: &str| "#".to_owned();
        let document_title = |kind: &str, _: &str| kind.replace('-', " ");
        let used_cookies = || documents::used_cookies(&data);
        let inputs = Inputs {
            options: &options,
            callback: &callback,
            page_url: &page_url,
            document_title: &document_title,
            used_cookies: &used_cookies,
            antispambot: &|s: &str| s.to_owned(),
            accept_text: text("accept_text"),
            save_preferences_text: text("save_preferences_text"),
            home_url: text("home_url"),
            site_url: text("site_url"),
            publish_date: text("publish_date"),
            checked_date: text("checked_date"),
            sync_date: text("sync_date"),
        };
        for document in golden["documents"].as_array().expect("documents") {
            let (kind, region, expected) = (document[0].as_str().unwrap_or_default(), document[1].as_str().unwrap_or_default(), decode(document[2].as_str().unwrap_or_default()));
            let actual = documents::document_html(kind, region, &inputs);
            count += 1;
            if actual != expected {
                let at = actual.bytes().zip(expected.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(expected.len()));
                let from = at.saturating_sub(120);
                failed.push(format!(
                    "{name} {kind} {region} differs at {at}:\n  expected: {:?}\n  actual:   {:?}",
                    expected.get(from..(at + 200).min(expected.len())).unwrap_or_default(),
                    actual.get(from..(at + 200).min(actual.len())).unwrap_or_default()
                ));
            }
        }
    }
    assert!(failed.is_empty(), "{} of {count} documents and lists differ\n{}", failed.len(), failed.join("\n"));
}

/// The consent area against the real plugin: `tests/data/complianz/consent.json` holds what `show_consent_area()`
/// gives for attributes, cookies and consent types, what `cmplz_render_consent_area_block()` gives for attributes and
/// the consented content `cmplz_rest_consented_content()` finds (made by `dev/complianz-consent-golden.sh`).
#[test]
fn consent_area() {
    use super::consent::{self, Area};
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/complianz/consent.json");
    let cases: Json = serde_json::from_str(&std::fs::read_to_string(path).expect("golden file")).expect("valid JSON");
    let text = |v: &Json| v.as_str().unwrap_or_default().to_owned();
    let mut failed = Vec::new();
    let cases = cases.as_array().expect("cases");
    for case in cases {
        let expected = text(&case["expected"]);
        let actual = match case["kind"].as_str() {
            Some("shortcode") => {
                let content = text(&case["content"]);
                if content.is_empty() {
                    String::new()
                } else {
                    let atts: Vec<(String, String)> = case["atts"].as_object().expect("atts").iter().map(|(k, v)| (k.to_ascii_lowercase(), text(v))).collect();
                    let get = |name: &str| atts.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str());
                    let area = consent::Atts {
                        id: get("id"),
                        clientside: get("clientside"),
                        cache_redirect: get("cache_redirect"),
                        scroll_into_view: get("scroll_into_view"),
                        service: get("service"),
                        category: get("category"),
                        text: super::sanitize_text_field(&get("text").map_or_else(|| text(&case["default_text"]), str::to_owned)),
                    };
                    let cookies = case["cookies"].as_object().expect("cookies");
                    let cookie = |name: &str| cookies.get(name).map(text);
                    match consent::show_consent_area(&area, case["post_id"].as_u64().unwrap_or(0), &text(&case["consent_type"]), "en_US", &cookie) {
                        Area::Html(html) => html,
                        Area::Consented { prefix } => prefix + &content,
                    }
                }
            }
            Some("block") => consent::render_block(case["attrs"].as_object().expect("attrs"), "en_US"),
            _ => consent::consented_content(&text(&case["content"]), &text(&case["block_id"])),
        };
        if actual != expected {
            failed.push(format!("{case}\n  got: {actual:?}"));
        }
    }
    assert!(failed.is_empty(), "{} of {} cases differ\n{}", failed.len(), cases.len(), failed.iter().take(5).cloned().collect::<Vec<_>>().join("\n"));
}
