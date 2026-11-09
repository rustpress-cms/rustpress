//! The submissions `dev/wpforms-submit-golden.sh` had the plugin process: the answer of `admin-ajax.php` and the
//! e-mails. The e-mails are compared when the plugin's files are there (its templates are read at runtime):
//! `RUSTPRESS_TEST_WP_ROOT`, else the volume of the reference stack.

use serde_json::Value;

use super::settings::Settings;
use super::submit::{self, Form, Request, User};

fn query_pairs(query: &str) -> Vec<(String, String)> {
    let decode = |s: &str| String::from_utf8_lossy(&rustpress_core::rewrite::urldecode(s.replace('+', " ").as_bytes())).into_owned();
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (decode(k), decode(v)),
            None => (decode(p), String::new()),
        })
        .collect()
}

fn plugin_dir() -> Option<std::path::PathBuf> {
    let root = std::env::var("RUSTPRESS_TEST_WP_ROOT").unwrap_or_else(|_| "/var/lib/docker/volumes/rustpress-wp/_data".into());
    let dir = std::path::Path::new(&root).join("wp-content/plugins/wpforms-lite");
    dir.join("templates/emails/classic-header.php").is_file().then_some(dir)
}

#[test]
fn golden_submissions() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("data").join("wpforms-submit");
    let plugin_dir = plugin_dir();
    if plugin_dir.is_none() {
        eprintln!("note: wpforms-lite not found, the e-mails of the submissions are not compared");
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("golden directory")
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            (name.ends_with(".json") && !name.ends_with(".case.json")).then(|| name.trim_end_matches(".json").to_owned())
        })
        .collect();
    names.sort();
    let mut failures = Vec::new();
    for name in &names {
        let case: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.json"))).expect("case")).expect("json");
        let settings = match &case["settings"] {
            Value::Object(m) => Settings(m.clone()),
            _ => Settings::default(),
        };
        let post = submit::post_tree(&query_pairs(case["query"].as_str().unwrap_or_default()));
        let get = Value::Object(Default::default());
        let tr = |s: &str| s.to_owned();
        let trn = |a: &str, b: &str, n: i64| if n == 1 { a.to_owned() } else { b.to_owned() };
        let nonce = case["nonce"].as_str().unwrap_or_default().to_owned();
        let verify = |n: &str, action: &str| n == nonce && action == "wpforms::form_9300";
        let date = |format: &str, at: i64| rustpress_core::datetime::SiteTimezone::utc().format(format, at);
        let site = &case["site"];
        let s = |key: &str| site[key].as_str().unwrap_or_default().to_owned();
        let user = case["user_data"].as_object().map(|u| User {
            id: u["id"].as_u64().unwrap_or(0),
            display_name: u["display_name"].as_str().unwrap_or_default().into(),
            email: u["email"].as_str().unwrap_or_default().into(),
            first_name: u["first_name"].as_str().unwrap_or_default().into(),
            last_name: u["last_name"].as_str().unwrap_or_default().into(),
        });
        let secret = super::token::secret_key(case["secret"].as_str());
        let permalinks = case["permalinks"].as_object().map(|m| m.iter().map(|(k, v)| (k.parse().unwrap_or(0), v.as_str().unwrap_or_default().to_owned())).collect()).unwrap_or_default();
        let page_author = case["page_author"].as_array().map(|a| (a[0].as_u64().unwrap_or(0), a[1].as_str().unwrap_or_default().to_owned()));
        let manager_emails = case["manager_emails"].as_array().map(|a| a.iter().filter_map(|e| e.as_str().map(str::to_owned)).collect()).unwrap_or_default();
        let mut shortcodes = rustpress_core::shortcodes::CORE_TAGS.to_vec();
        shortcodes.push("wpforms");
        let home = s("home");
        let r = Request {
            settings: &settings,
            tr: &tr,
            trn: &trn,
            post: &post,
            get: &get,
            user,
            can_manage: case["can_manage"].as_bool().unwrap_or(false),
            super_admin: case["super_admin"].as_bool().unwrap_or(false),
            verify_nonce: &verify,
            now: case["now"].as_i64().unwrap_or(0),
            remote_addr: "192.0.2.1",
            origin: home.clone(),
            request_uri: "/wp-admin/admin-ajax.php".into(),
            admin_email: s("admin_email"),
            blogname: s("blogname"),
            home_url: home.clone(),
            home_host: "localhost".into(),
            login_url: s("login"),
            charset: "UTF-8".into(),
            language_attributes: s("language"),
            rtl: false,
            date: &date,
            gmt_offset: site["gmt_offset"].as_f64().unwrap_or(0.0),
            date_format: s("date_format"),
            secret: &secret,
            permalinks,
            shortcodes: &shortcodes,
            page_author,
            manager_emails,
            plugin_dir: plugin_dir.clone(),
        };
        let data = case["form_data"].clone();
        let status = case["status"].as_str().unwrap_or("publish").to_owned();
        let forms = |id: u64| (id == 9300).then(|| Form { status: &status, data: &data, post_date: "2026-10-24 00:00:00" });
        let outcome = submit::ajax_submit(&forms, &r);
        let expected = case["response"].as_str().unwrap_or_default();
        if outcome.body != expected {
            let at = outcome.body.bytes().zip(expected.bytes()).position(|(a, b)| a != b).unwrap_or(outcome.body.len().min(expected.len()));
            let from = at.saturating_sub(80);
            failures.push(format!(
                "{name} answer (byte {at}):\n  expected …{:?}\n  actual   …{:?}",
                expected.get(from..(at + 160).min(expected.len())).unwrap_or(""),
                outcome.body.get(from..(at + 160).min(outcome.body.len())).unwrap_or("")
            ));
        }
        if plugin_dir.is_none() {
            continue;
        }
        let mails = case["mails"].as_array().cloned().unwrap_or_default();
        if mails.len() != outcome.emails.len() {
            failures.push(format!("{name}: {} e-mails, expected {}", outcome.emails.len(), mails.len()));
            continue;
        }
        for (i, (mail, email)) in mails.iter().zip(&outcome.emails).enumerate() {
            let headers: Vec<String> = mail["headers"].as_array().map(|a| a.iter().filter_map(|h| h.as_str().map(str::to_owned)).collect()).unwrap_or_default();
            for (part, expected, actual) in [
                ("to", mail["to"].as_str().unwrap_or_default().to_owned(), email.to.clone()),
                ("subject", mail["subject"].as_str().unwrap_or_default().to_owned(), email.subject.clone()),
                ("headers", headers.join("\n"), email.headers.join("\n")),
                ("body", mail["body"].as_str().unwrap_or_default().to_owned(), email.body.clone()),
            ] {
                if expected != actual {
                    let at = actual.bytes().zip(expected.bytes()).position(|(a, b)| a != b).unwrap_or(actual.len().min(expected.len()));
                    let from = at.saturating_sub(80);
                    failures.push(format!(
                        "{name} mail {i} {part} (byte {at}):\n  expected …{:?}\n  actual   …{:?}",
                        expected.get(from..(at + 160).min(expected.len())).unwrap_or(""),
                        actual.get(from..(at + 160).min(actual.len())).unwrap_or("")
                    ));
                }
            }
        }
    }
    assert!(!names.is_empty(), "no golden cases");
    assert!(failures.is_empty(), "{} differences in {} cases:\n{}", failures.len(), names.len(), failures.join("\n"));
}
