//! WP Mail SMTP 4.10.4: sends the site's e-mail through the SMTP server and with the sender of its settings.
//!
//! The settings are the option `wp_mail_smtp` (`WPMailSMTP\Options`), or the `WPMS_*` constants of `wp-config.php`
//! when `WPMS_ON` is `true`; the SMTP password is stored encrypted with `sodium_crypto_secretbox()` and the key in
//! `wp_mail_smtp_mail_key` (or `WPMS_CRYPTO_KEY`), see `WPMailSMTP\Helpers\Crypto`. What the plugin does to a
//! message (`WPMailSMTP\Processor`, `MailCatcherTrait::send()`):
//!
//! - `wp_mail_from` / `wp_mail_from_name`: the address and name of the settings, for messages that do not name their
//!   own, or for all of them when forced.
//! - `phpmailer_init`: the mailer (`smtp` with host, port, encryption, auto TLS and authentication; `mail` for PHP's
//!   `mail()`), and the original sender as `Reply-To` when the sender is forced.
//! - `X-Mailer: WPMailSMTP/Mailer/<mailer> 4.10.4`, and no message at all with "Do not send".
//!
//! Not rebuilt: the mailers that send through a provider's web API (SendGrid, Mailgun, Gmail, ...; RustPress sends
//! those messages its own way, with the plugin's sender), Pepipost, the return path for `mail()` (`-f` is always
//! the sender), the email log, the backup connection and the queue (Pro), the administration.

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, MailSender, MailSettings, MailTransport, NativePlugin, SmtpSecurity};
use rustpress_core::{Result, Site};
use rustpress_phpser::Value;

pub struct WpMailSmtp;

/// `WPMS_PLUGIN_VER`.
const VERSION: &str = "4.10.4";

impl NativePlugin for WpMailSmtp {
    fn file(&self) -> &'static str {
        "wp-mail-smtp/wp_mail_smtp.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        self.load_at(site, std::path::Path::new("."))
    }

    fn load_at<'a>(&'a self, site: &'a Site, wp_root: &'a std::path::Path) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let option = site.options.fetch(&site.db, "wp_mail_smtp").await?.unwrap_or_default();
            let key = site.options.fetch(&site.db, "wp_mail_smtp_mail_key").await?;
            let config = [wp_root.join("wp-config.php"), wp_root.parent().map(|p| p.join("wp-config.php")).unwrap_or_default()].into_iter().find_map(|path| std::fs::read_to_string(path).ok()).unwrap_or_default();
            let options = Options::new(&option, key.as_deref(), config_constants(&config));
            Ok(Box::new(Active { settings: settings(&options) }) as Box<dyn ActivePlugin>)
        })
    }
}

struct Active {
    settings: MailSettings,
}

impl ActivePlugin for Active {
    fn mail_settings(&self) -> Option<MailSettings> {
        Some(self.settings.clone())
    }
}

/// A value of a `define()` in `wp-config.php`: strings, integers and booleans.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Str(String),
    Int(i64),
    Bool(bool),
}

impl Constant {
    /// PHP's truthiness.
    fn truthy(&self) -> bool {
        match self {
            Constant::Str(s) => !s.is_empty() && s != "0",
            Constant::Int(i) => *i != 0,
            Constant::Bool(b) => *b,
        }
    }

    fn value(&self) -> Value {
        match self {
            Constant::Str(s) => Value::Str(s.as_bytes().to_vec()),
            Constant::Int(i) => Value::Int(*i),
            Constant::Bool(b) => Value::Bool(*b),
        }
    }
}

/// The `define( 'WPMS_…', value );` lines of `wp-config.php` (a line commented out with `//` or `#` is not read).
pub fn config_constants(config_php: &str) -> HashMap<String, Constant> {
    let pattern = regex::Regex::new(r#"(?m)^[ \t]*define\s*\(\s*['"]((?:WPMS)_[A-Z0-9_]+)['"]\s*,\s*('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"|-?[0-9]+|[A-Za-z]+)\s*\)"#).expect("valid pattern");
    let mut out = HashMap::new();
    for caps in pattern.captures_iter(config_php) {
        let raw = &caps[2];
        let value = if let Some(inner) = raw.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
            Constant::Str(inner.replace("\\'", "'").replace("\\\\", "\\"))
        } else if let Some(inner) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
            Constant::Str(inner.replace("\\\"", "\"").replace("\\\\", "\\"))
        } else if let Ok(i) = raw.parse() {
            Constant::Int(i)
        } else {
            match raw.to_ascii_lowercase().as_str() {
                "true" => Constant::Bool(true),
                "false" => Constant::Bool(false),
                _ => continue,
            }
        };
        out.entry(caps[1].to_owned()).or_insert(value);
    }
    out
}

/// `WPMailSMTP\Options` for the groups and keys of the mailers RustPress runs.
pub struct Options {
    stored: Value,
    key: Option<Vec<u8>>,
    constants: HashMap<String, Constant>,
}

impl Options {
    /// `option` is the stored `wp_mail_smtp`, `key` the stored `wp_mail_smtp_mail_key`.
    pub fn new(option: &str, key: Option<&str>, constants: HashMap<String, Constant>) -> Self {
        let stored = rustpress_phpser::unserialize(option.as_bytes()).unwrap_or(Value::Array(Vec::new()));
        // `Crypto::get_secret_key()`: `WPMS_CRYPTO_KEY` as it is, else the option base64-decoded.
        let key = match constants.get("WPMS_CRYPTO_KEY") {
            Some(constant) => Some(match constant {
                Constant::Str(s) => s.as_bytes().to_vec(),
                Constant::Int(i) => i.to_string().into_bytes(),
                Constant::Bool(b) => if *b { b"1".to_vec() } else { Vec::new() },
            }),
            None => key.map(php_base64_decode),
        };
        Self { stored, key, constants }
    }

    /// `is_const_enabled()`: `WPMS_ON === true`.
    fn const_enabled(&self) -> bool {
        self.constants.get("WPMS_ON") == Some(&Constant::Bool(true))
    }

    /// `get_const_value()` with `is_const_defined()`: the constant that stands for the setting, if it counts.
    fn constant(&self, group: &str, key: &str) -> Option<Value> {
        if !self.const_enabled() {
            return None;
        }
        let name = match (group, key) {
            ("mail", "from_name") => "WPMS_MAIL_FROM_NAME",
            ("mail", "from_email") => "WPMS_MAIL_FROM",
            ("mail", "mailer") => "WPMS_MAILER",
            ("mail", "return_path") => "WPMS_SET_RETURN_PATH",
            ("mail", "from_name_force") => "WPMS_MAIL_FROM_NAME_FORCE",
            ("mail", "from_email_force") => "WPMS_MAIL_FROM_FORCE",
            ("smtp", "host") => "WPMS_SMTP_HOST",
            ("smtp", "port") => "WPMS_SMTP_PORT",
            ("smtp", "encryption") => "WPMS_SSL",
            ("smtp", "auth") => "WPMS_SMTP_AUTH",
            ("smtp", "autotls") => "WPMS_SMTP_AUTOTLS",
            ("smtp", "user") => "WPMS_SMTP_USER",
            ("smtp", "pass") => "WPMS_SMTP_PASS",
            ("general", "do_not_send") => "WPMS_DO_NOT_SEND",
            _ => return None,
        };
        let constant = self.constants.get(name)?;
        let defined = match name {
            // `defined( … ) && ( … === 'true' || … === true )`.
            "WPMS_SET_RETURN_PATH" | "WPMS_MAIL_FROM_NAME_FORCE" | "WPMS_MAIL_FROM_FORCE" => matches!(constant, Constant::Bool(true)) || *constant == Constant::Str("true".into()),
            // `defined( … )` alone.
            "WPMS_SSL" | "WPMS_SMTP_AUTH" | "WPMS_SMTP_AUTOTLS" => true,
            _ => constant.truthy(),
        };
        if !defined {
            return None;
        }
        Some(match name {
            "WPMS_SSL" if *constant == Constant::Str(String::new()) => Value::Str(b"none".to_vec()),
            "WPMS_SMTP_AUTH" | "WPMS_SMTP_AUTOTLS" => Value::Bool(constant.truthy()),
            _ => constant.value(),
        })
    }

    /// `Options::get( $group, $key )`.
    pub fn get(&self, group: &str, key: &str) -> Value {
        if let Some(value) = self.constant(group, key) {
            return value;
        }
        let stored = self.stored.get(group).and_then(|g| g.get(key));
        let value = match stored {
            Some(value) if group == "smtp" && key == "pass" => match value.as_bytes() {
                Some(encrypted) => Value::Str(self.decrypt(encrypted)),
                None => value.clone(),
            },
            Some(value) => value.clone(),
            None => defaults(group, key),
        };
        // Slashes are stripped from the values of the database.
        match value {
            Value::Str(bytes) => Value::Str(stripslashes(&bytes)),
            other => other,
        }
    }

    fn truthy(&self, group: &str, key: &str) -> bool {
        php_truthy(&self.get(group, key))
    }

    fn text(&self, group: &str, key: &str) -> String {
        match self.get(group, key) {
            Value::Str(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(true) => "1".into(),
            _ => String::new(),
        }
    }

    /// `Crypto::decrypt()`: the encrypted text itself when it cannot be decrypted.
    fn decrypt(&self, encrypted: &[u8]) -> Vec<u8> {
        use crypto_secretbox::aead::{Aead, KeyInit};
        let decoded = php_base64_decode(&String::from_utf8_lossy(encrypted));
        let Some(key) = self.key.as_ref().filter(|k| !k.is_empty()) else { return encrypted.to_vec() };
        let Ok(key) = <[u8; 32]>::try_from(key.as_slice()) else { return encrypted.to_vec() };
        if decoded.len() < 24 + 16 {
            return encrypted.to_vec();
        }
        let (nonce, ciphertext) = decoded.split_at(24);
        let Ok(nonce) = <[u8; 24]>::try_from(nonce) else { return encrypted.to_vec() };
        let cipher = crypto_secretbox::XSalsa20Poly1305::new(&crypto_secretbox::Key::from(key));
        cipher.decrypt(&crypto_secretbox::Nonce::from(nonce), ciphertext).unwrap_or_else(|_| encrypted.to_vec())
    }
}

/// `postprocess_key_defaults()`.
fn defaults(group: &str, key: &str) -> Value {
    let smtp = group == "smtp" || group == "pepipost";
    match key {
        "from_email_force" | "from_name_force" | "return_path" => Value::Bool(group != "mail"),
        "mailer" => Value::Str(b"mail".to_vec()),
        "encryption" if smtp => Value::Str(b"none".to_vec()),
        "auth" | "autotls" => Value::Bool(!smtp),
        _ => Value::Str(Vec::new()),
    }
}

/// What the plugin makes of the settings.
pub fn settings(options: &Options) -> MailSettings {
    let mailer = options.text("mail", "mailer");
    let host = options.text("smtp", "host");
    // `phpmailer_init()` leaves PHPMailer alone without a mailer, or for `smtp` without a host.
    let configures = !mailer.is_empty() && mailer != "0" && !(mailer == "smtp" && host.is_empty());
    // PHP's `mail()` (also when the plugin leaves PHPMailer alone) is RustPress's own way: `RUSTPRESS_SMTP_URL`,
    // else `sendmail`.
    let transport = match mailer.as_str() {
        _ if !configures => None,
        "mail" => None,
        "smtp" => {
            let encryption = options.text("smtp", "encryption");
            let security = match encryption.as_str() {
                "ssl" => SmtpSecurity::Implicit,
                "tls" => SmtpSecurity::StartTls,
                // `SMTPAutoTLS` stays on unless the setting turns it off.
                _ if options.truthy("smtp", "autotls") => SmtpSecurity::Opportunistic,
                _ => SmtpSecurity::Plain,
            };
            let port = options.text("smtp", "port").trim().parse().unwrap_or(25);
            let credentials = options.truthy("smtp", "auth").then(|| (options.text("smtp", "user"), options.text("smtp", "pass")));
            Some(MailTransport::Smtp { host, port, security, credentials })
        }
        // The mailers of web APIs.
        _ => None,
    };
    let from_email = options.text("mail", "from_email");
    let from_name = options.text("mail", "from_name");
    let email_forced = options.truthy("mail", "from_email_force");
    let name_forced = options.truthy("mail", "from_name_force");
    let x_mailer = format!("WPMailSMTP/Mailer/{mailer} {VERSION}");
    let sender = Arc::new(move |mut sender: MailSender| {
        // `filter_mail_from_email()`: the original address, when it is not the default one, is kept for `Reply-To`.
        let original = (sender.email != sender.default_email).then(|| sender.email.clone()).filter(|e| is_email(e));
        sender.email = if email_forced && !from_email.is_empty() {
            from_email.clone()
        } else if !sender.default_email.is_empty() && sender.email != sender.default_email {
            sender.email
        } else if !from_email.is_empty() {
            from_email.clone()
        } else {
            sender.email
        };
        // `filter_mail_from_name()`.
        if name_forced || sender.name == "WordPress" {
            sender.name = from_name.clone();
        }
        // `allow_setting_original_from_email_to_reply_to()`.
        if configures
            && sender.reply_to.is_empty()
            && let Some(original) = original
            && email_forced
            && from_email != original
        {
            sender.reply_to.push(original);
        }
        sender.headers.push(("X-Mailer".into(), x_mailer.clone()));
        sender
    });
    MailSettings { transport, blocked: options.truthy("general", "do_not_send"), sender }
}

/// `filter_var( $email, FILTER_VALIDATE_EMAIL )`, roughly: one `@` with something on both sides and a dot after it.
fn is_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else { return false };
    !local.is_empty() && !domain.contains('@') && domain.contains('.') && !email.chars().any(char::is_whitespace)
}

/// PHP's truthiness of a value.
fn php_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(s) => !s.is_empty() && s != b"0",
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

/// `stripslashes()`.
fn stripslashes(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut iter = bytes.iter();
    while let Some(&b) = iter.next() {
        if b == b'\\' {
            match iter.next() {
                Some(b'0') => out.push(0),
                Some(&next) => out.push(next),
                None => {}
            }
        } else {
            out.push(b);
        }
    }
    out
}

/// `base64_decode()` without `$strict`: characters outside the alphabet are skipped.
fn php_base64_decode(text: &str) -> Vec<u8> {
    let clean: String = text.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/').collect();
    let engine = base64::engine::GeneralPurpose::new(&base64::alphabet::STANDARD, base64::engine::GeneralPurposeConfig::new().with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent).with_decode_allow_trailing_bits(true));
    let usable = if clean.len() % 4 == 1 { &clean[..clean.len() - 1] } else { &clean[..] };
    engine.decode(usable).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_of_wp_config() {
        let config = "<?php\ndefine( 'WPMS_ON', true );\ndefine('WPMS_SMTP_PORT', 2525);\ndefine( \"WPMS_SMTP_PASS\", 'it\\'s' );\n// define( 'WPMS_SMTP_HOST', 'x' );\ndefine( 'DB_NAME', 'wp' );\n";
        let constants = config_constants(config);
        assert_eq!(constants.get("WPMS_ON"), Some(&Constant::Bool(true)));
        assert_eq!(constants.get("WPMS_SMTP_PORT"), Some(&Constant::Int(2525)));
        assert_eq!(constants.get("WPMS_SMTP_PASS"), Some(&Constant::Str("it's".into())));
        assert_eq!(constants.get("WPMS_SMTP_HOST"), None);
        assert_eq!(constants.get("DB_NAME"), None);
    }

    /// The cases of `tests/data/wp-mail-smtp` against what the plugin set up in PHPMailer (`dev/wp-mail-smtp-golden.sh`).
    #[test]
    fn matches_the_plugin() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/wp-mail-smtp/cases.json");
        let cases: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut checked = 0;
        for case in cases.as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let constants = case["constants"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| {
                    let value = match v {
                        serde_json::Value::Bool(b) => Constant::Bool(*b),
                        serde_json::Value::Number(n) => Constant::Int(n.as_i64().unwrap()),
                        other => Constant::Str(other.as_str().unwrap().to_owned()),
                    };
                    (k.clone(), value)
                })
                .collect();
            let options = Options::new(case["option"].as_str().unwrap(), case["mail_key"].as_str(), constants);
            let settings = settings(&options);
            let expected = &case["expected"];
            if expected.is_null() {
                // Blocked, a web API, or an address PHPMailer refuses (`wordpress@localhost` has no dot).
                let error = case["error"].as_str().unwrap_or_default();
                if name.starts_with("blocked/") {
                    assert!(settings.blocked, "{name}");
                } else if name.starts_with("sendgrid/") {
                    assert_eq!(settings.transport, None, "{name}");
                } else {
                    assert!(error.starts_with("Invalid address"), "{name}: {error}");
                }
                continue;
            }
            assert!(!settings.blocked, "{name}");
            let text = |key: &str| match &expected[key] {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            let want = match text("Mailer").as_str() {
                "smtp" => Some(MailTransport::Smtp {
                    host: text("Host"),
                    port: text("Port").parse().unwrap(),
                    security: match (text("SMTPSecure").as_str(), expected["SMTPAutoTLS"].as_bool().unwrap()) {
                        ("ssl", _) => SmtpSecurity::Implicit,
                        ("tls", _) => SmtpSecurity::StartTls,
                        (_, true) => SmtpSecurity::Opportunistic,
                        (_, false) => SmtpSecurity::Plain,
                    },
                    credentials: expected["SMTPAuth"].as_bool().unwrap().then(|| (text("Username"), text("Password"))),
                }),
                _ => None,
            };
            assert_eq!(settings.transport, want, "{name}");
            // `wp_mail()` with the case's headers.
            let mut sender = MailSender { email: "wordpress@localhost".into(), name: "WordPress".into(), default_email: "wordpress@localhost".into(), reply_to: Vec::new(), headers: Vec::new() };
            for line in case["headers"].as_array().unwrap() {
                let (key, value) = line.as_str().unwrap().split_once(':').unwrap();
                let value = value.trim();
                match key {
                    "From" => {
                        let (n, e) = value.split_once('<').unwrap();
                        sender.name = n.trim().to_owned();
                        sender.email = e.trim_end_matches('>').trim().to_owned();
                    }
                    "Reply-To" => sender.reply_to.push(value.to_owned()),
                    _ => {}
                }
            }
            let out = (settings.sender)(sender);
            assert_eq!(out.email, text("From"), "{name}");
            assert_eq!(out.name, text("FromName"), "{name}");
            let reply: Vec<String> = expected["ReplyTo"].as_array().unwrap().iter().map(|r| r.as_str().unwrap().to_owned()).collect();
            assert_eq!(out.reply_to, reply, "{name}");
            assert_eq!(out.headers, vec![("X-Mailer".to_owned(), text("XMailer"))], "{name}");
            checked += 1;
        }
        assert!(checked >= 35, "{checked}");
    }

    #[test]
    fn lenient_base64() {
        assert_eq!(php_base64_decode("aGVsbG8="), b"hello");
        assert_eq!(php_base64_decode("aGVs bG8"), b"hello");
    }
}
