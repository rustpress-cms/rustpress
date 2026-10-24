//! Guards the message ids used in the Rust code: every text passed to `tr()`, `trx()` or `trn()`
//! must be one WordPress itself uses, otherwise no language pack can translate it.
//!
//! The ids are checked against the German core language pack of the reference WordPress
//! (`<RUSTPRESS_WP_ROOT>/wp-content/languages/de_DE.mo`); the test is skipped without it.

use std::path::{Path, PathBuf};

use regex::Regex;
use rustpress_core::i18n::I18n;

const DEFAULT_ROOT: &str = r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data";

/// Texts that are not part of WordPress' own catalogs (RustPress additions) and so cannot be checked.
const OWN_MESSAGES: &[&str] = &[];

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
    literal.replace("\\\"", "\"").replace("\\n", "\n").replace("\\\\", "\\")
}

#[test]
fn every_message_id_exists_in_wordpress() {
    let root = PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    if !root.join("wp-content/languages/de_DE.mo").exists() {
        eprintln!("skipped: no de_DE language pack in {}", root.display());
        return;
    }
    let i18n = I18n::load(&root, "de_DE", &[]);
    assert!(i18n.is_translated());

    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("crates directory");
    let mut files = Vec::new();
    rust_files(crates, &mut files);

    let text = r#""((?:[^"\\]|\\.)*)""#;
    let plain = Regex::new(&format!(r"\.tr\(\s*{text}\s*\)")).unwrap();
    let with_context = Regex::new(&format!(r"\.trx\(\s*{text}\s*,\s*{text}\s*\)")).unwrap();
    let plural = Regex::new(&format!(r"\.trn\(\s*{text}\s*,\s*{text}\s*,")).unwrap();
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
        let mut check = |context: Option<String>, id: String| {
            checked += 1;
            if !OWN_MESSAGES.contains(&id.as_str()) && !i18n.has("default", context.as_deref(), &id) {
                missing.push(format!("{name}: {:?} {id:?}", context));
            }
        };
        for c in plain.captures_iter(source) {
            check(None, unescape(&c[1]));
        }
        for c in with_context.captures_iter(source) {
            check(Some(unescape(&c[2])), unescape(&c[1]));
        }
        for c in plural.captures_iter(source) {
            check(None, unescape(&c[1]));
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