//! Guards the message ids used in the Rust code: every text passed to `tr()`, `trx()` or `trn()`
//! must be one WordPress itself uses, otherwise no language pack can translate it.
//!
//! The ids are checked against the German language packs of the reference WordPress (`de_DE.mo` and
//! `admin-de_DE.mo` in `<RUSTPRESS_WP_ROOT>/wp-content/languages`); the test is skipped without them.

use std::path::{Path, PathBuf};

use regex::Regex;
use rustpress_core::i18n::I18n;

/// Where the WordPress files are when `RUSTPRESS_WP_ROOT` is not set: `wordpress/` in the repository (not checked in).
const DEFAULT_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../wordpress");

/// Texts that are not part of WordPress' own catalogs (RustPress additions) and so cannot be checked.
const OWN_MESSAGES: &[&str] = &[
    // get_status_header_desc( 404 ) is not translated
    "Not Found",
    // A site started without write access, failures of the disk or the database, files that are not there.
    "<strong>Error:</strong> This site was started without write access.",
    "<strong>Error:</strong> Signing in is not possible: this site was started without write access.",
    "<strong>Error:</strong> Could not start a session.",
    "RustPress was started without write access.",
    "The site is read only.",
    "The requested page does not exist.",
    "The uploaded file could not be moved to the fonts directory.",
    "The plugin status could not be changed.",
    // Site Health: what RustPress does instead of updating itself and talking to WordPress.org.
    "RustPress does not communicate with WordPress.org",
    "RustPress does not contact the WordPress servers: updates are installed by replacing its program, and themes and plugins are installed from ZIP files.",
    "RustPress does not use background updates",
    "RustPress is a single program and does not update itself; install a new version to update it.",
    "Your site does not need loopback requests",
    // Only in `admin-network-<locale>.mo`, which WordPress does not load for network.php either.
    "You must define the %1$s constant as true in your %2$s file to allow creation of a Network.",
    // A plugin route's message, already translated in the plugin's text domain, passed through as is.
    "%s",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target" || n == "tests") {
                continue;
            }
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn unescape(literal: &str) -> String {
    let text = literal.replace("\\\"", "\"").replace("\\n", "\n").replace("\\\\", "\\");
    // `\u{2318}` and the like.
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("\\u{") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 3..];
        match after.find('}').and_then(|end| u32::from_str_radix(&after[..end], 16).ok().and_then(char::from_u32).map(|c| (c, end))) {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push_str("\\u{");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn every_message_id_exists_in_wordpress() {
    let root = PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    if !root.join("wp-content/languages/de_DE.mo").exists() {
        eprintln!("skipped: no de_DE language pack in {}", root.display());
        assert!(std::env::var_os("RUSTPRESS_REQUIRE_REFERENCE").is_none(), "RUSTPRESS_REQUIRE_REFERENCE is set, so this test must not skip itself");
        return;
    }
    let i18n = I18n::load(&root, "de_DE", &[]);
    assert!(i18n.is_translated());

    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("crates directory");
    let mut files = Vec::new();
    rust_files(crates, &mut files);

    let text = r#""((?:[^"\\]|\\.)*)""#;
    // Group 1 is the receiver: in the native plugin modules only `site` translates in WordPress' own text domain;
    // the modules' own `tr()` methods use the plugin's domain, whose language packs the reference does not have.
    let plain = Regex::new(&format!(r"(\w+)\.tr\(\s*{text}\s*\)")).unwrap();
    let with_context = Regex::new(&format!(r"(\w+)\.trx\(\s*{text}\s*,\s*{text}\s*\)")).unwrap();
    let plural = Regex::new(&format!(r"(\w+)\.trn\(\s*{text}\s*,\s*{text}\s*,")).unwrap();
    // REST errors carry their message id as a literal and are translated when serialized.
    let rest_messages = [
        Regex::new(&format!(r"RestError::new\(\s*{text}\s*,\s*{text}")).unwrap(),
        Regex::new(&format!(r"invalid_param(?:_with_data)?\(\s*[^,]+,\s*{text}")).unwrap(),
    ];
    let msg = Regex::new(&format!(r"Msg::new\(\s*{text}\s*\)")).unwrap();

    let mut missing = Vec::new();
    let mut checked = 0;
    for file in files {
        let source = std::fs::read_to_string(&file).unwrap();
        let source = source.split("#[cfg(test)]").next().unwrap_or_default();
        let name = file.strip_prefix(crates).unwrap_or(&file).display().to_string();
        let plugin_module = name.starts_with("rustpress-plugins");
        let core_domain = |c: &regex::Captures| !plugin_module || &c[1] == "site";
        let mut check = |context: Option<String>, id: String| {
            checked += 1;
            if !OWN_MESSAGES.contains(&id.as_str()) && !i18n.has("default", context.as_deref(), &id) {
                missing.push(format!("{name}: {:?} {id:?}", context));
            }
        };
        for c in plain.captures_iter(source).filter(core_domain) {
            check(None, unescape(&c[2]));
        }
        for c in with_context.captures_iter(source).filter(core_domain) {
            check(Some(unescape(&c[3])), unescape(&c[2]));
        }
        for c in plural.captures_iter(source).filter(core_domain) {
            check(None, unescape(&c[2]));
        }
        for re in &rest_messages {
            for c in re.captures_iter(source) {
                // `RestError::new( code, message` has the message in group 2; `invalid_param( param, message` in group 1.
                let message = c.get(2).or_else(|| c.get(1)).map_or("", |m| m.as_str());
                check(None, unescape(message));
            }
        }
        for c in msg.captures_iter(source) {
            check(None, unescape(&c[1]));
        }
    }
    assert!(checked > 20, "only {checked} messages found: the patterns no longer match the code");
    assert!(missing.is_empty(), "message ids WordPress does not have (typo, or a changed English text):\n{}", missing.join("\n"));
}