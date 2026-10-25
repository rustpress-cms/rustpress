//! `wp_salt()`: the keys and salts that sign cookies and nonces. They come from the constants in
//! `wp-config.php`, or from the options WordPress stores when the constants are missing.

use std::collections::HashMap;

use hmac::{Hmac, Mac};
use md5::Md5;

/// The sample value of a fresh `wp-config.php`; WordPress treats it (and repeated values) as unset.
const PLACEHOLDER: &str = "put your unique phrase here";

/// The constants of a `wp-config.php` that matter: `define( 'AUTH_KEY', '…' );` and the like. The Docker
/// image of WordPress writes `getenv_docker( 'WORDPRESS_AUTH_KEY', '…' )`: the environment variable
/// wins when it is set, else the second argument counts.
pub fn parse_config_constants(config_php: &str) -> HashMap<String, String> {
    let pattern = regex::Regex::new(
        r#"(?m)^\s*define\s*\(\s*'([A-Z_]+)'\s*,\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)"|getenv_docker\(\s*'([^']*)'\s*,\s*'((?:[^'\\]|\\.)*)'\s*\))\s*\)"#,
    )
    .expect("valid pattern");
    let mut out = HashMap::new();
    for caps in pattern.captures_iter(config_php) {
        let name = caps[1].to_owned();
        if let Some(single) = caps.get(2) {
            out.insert(name, unescape_single(single.as_str()));
        } else if let Some(double) = caps.get(3) {
            out.insert(name, unescape_double(double.as_str()));
        } else if let (Some(variable), Some(default)) = (caps.get(4), caps.get(5)) {
            let value = std::env::var(variable.as_str()).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| unescape_single(default.as_str()));
            out.insert(name, value);
        }
    }
    out
}

fn unescape_single(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.peek(), Some('\\' | '\'')) {
            out.push(chars.next().unwrap_or('\\'));
        } else {
            out.push(c);
        }
    }
    out
}

fn unescape_double(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('\\' | '"' | '$') => out.push(chars.next().unwrap_or('\\')),
                Some('n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('t') => {
                    chars.next();
                    out.push('\t');
                }
                _ => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// What `wp_salt()` can use: the constants and the stored options.
#[derive(Debug, Clone, Default)]
pub struct Salts {
    constants: HashMap<String, String>,
    options: HashMap<String, String>,
}

impl Salts {
    /// `constants` as `parse_config_constants()` found them; `options` holds `auth_key`, `auth_salt`, …,
    /// `secret_key` as far as the site has them.
    pub fn new(constants: HashMap<String, String>, options: HashMap<String, String>) -> Self {
        Self { constants, options }
    }

    /// The names of the options that may hold a key or a salt.
    pub fn option_names() -> Vec<String> {
        let mut names: Vec<String> = ["auth", "secure_auth", "logged_in", "nonce"].iter().flat_map(|s| [format!("{s}_key"), format!("{s}_salt")]).collect();
        names.push("secret_key".to_owned());
        names
    }

    /// A constant that is set, not empty and not the sample text nor a copy of another one
    /// (`$duplicated_keys` of `wp_salt()`).
    fn constant(&self, name: &str) -> Option<&str> {
        let value = self.constants.get(name).filter(|v| !v.is_empty())?;
        if value == PLACEHOLDER {
            return None;
        }
        let copies = ["AUTH", "SECURE_AUTH", "LOGGED_IN", "NONCE", "SECRET"]
            .iter()
            .flat_map(|first| ["KEY", "SALT"].map(|second| format!("{first}_{second}")))
            .filter(|n| self.constants.get(n).is_some_and(|v| v == value))
            .count();
        (copies == 1).then_some(value.as_str())
    }

    fn option(&self, name: &str) -> Option<&str> {
        self.options.get(name).map(String::as_str).filter(|v| !v.is_empty())
    }

    /// `wp_salt( $scheme )`: key and salt of the scheme joined; `None` when WordPress would have to make
    /// one up (and store it).
    pub fn salt(&self, scheme: &str) -> Option<String> {
        let mut key = self.constant("SECRET_KEY").unwrap_or_default().to_owned();
        let mut salt = if scheme == "auth" { self.constant("SECRET_SALT").unwrap_or_default().to_owned() } else { String::new() };
        if matches!(scheme, "auth" | "secure_auth" | "logged_in" | "nonce") {
            for kind in ["key", "salt"] {
                let constant = format!("{}_{}", scheme.to_ascii_uppercase(), kind.to_ascii_uppercase());
                let slot = if kind == "key" { &mut key } else { &mut salt };
                if let Some(value) = self.constant(&constant) {
                    *slot = value.to_owned();
                } else if slot.is_empty() {
                    *slot = self.option(&format!("{scheme}_{kind}"))?.to_owned();
                }
            }
        } else {
            if key.is_empty() {
                key = self.option("secret_key")?.to_owned();
            }
            salt = hmac_md5_hex(&key, scheme);
        }
        Some(format!("{key}{salt}"))
    }

    /// `wp_hash( $data, $scheme )`.
    pub fn hash(&self, data: &str, scheme: &str) -> Option<String> {
        Some(hmac_md5_hex(&self.salt(scheme)?, data))
    }
}

/// `md5( $data )` in lower-case hex.
pub fn md5_hex(data: &str) -> String {
    use md5::Digest;
    Md5::digest(data.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// `hash_hmac( 'md5', $data, $key )`.
pub fn hmac_md5_hex(key: &str, data: &str) -> String {
    let mut mac = Hmac::<Md5>::new_from_slice(key.as_bytes()).expect("hmac accepts any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_constants_are_read() {
        let config = "<?php\ndefine( 'DB_NAME', 'wordpress' );\ndefine( 'AUTH_KEY',         'a\\'b\\\\c' );\ndefine('NONCE_SALT', \"x\\$y\");\n// define( 'LOGGED_IN_KEY', 'no' );\n";
        let constants = parse_config_constants(config);
        assert_eq!(constants["AUTH_KEY"], "a'b\\c");
        assert_eq!(constants["NONCE_SALT"], "x$y");
        assert!(!constants.contains_key("LOGGED_IN_KEY"));
    }

    #[test]
    fn schemes_join_key_and_salt() {
        let constants: HashMap<String, String> = [("AUTH_KEY", "k1"), ("AUTH_SALT", "s1"), ("LOGGED_IN_KEY", "k3"), ("LOGGED_IN_SALT", "s3")].iter().map(|(a, b)| ((*a).to_owned(), (*b).to_owned())).collect();
        let salts = Salts::new(constants, HashMap::new());
        assert_eq!(salts.salt("auth").as_deref(), Some("k1s1"));
        assert_eq!(salts.salt("logged_in").as_deref(), Some("k3s3"));
        assert_eq!(salts.salt("nonce"), None);
    }

    #[test]
    fn repeated_and_sample_values_fall_back_to_options() {
        let constants: HashMap<String, String> = [("AUTH_KEY", "same"), ("AUTH_SALT", "same"), ("NONCE_KEY", PLACEHOLDER), ("NONCE_SALT", "fine")].iter().map(|(a, b)| ((*a).to_owned(), (*b).to_owned())).collect();
        let options: HashMap<String, String> = [("auth_key", "ok"), ("auth_salt", "os"), ("nonce_key", "nk")].iter().map(|(a, b)| ((*a).to_owned(), (*b).to_owned())).collect();
        let salts = Salts::new(constants, options);
        assert_eq!(salts.salt("auth").as_deref(), Some("okos"));
        assert_eq!(salts.salt("nonce").as_deref(), Some("nkfine"));
    }

    #[test]
    fn hmac_md5_matches_rfc_2202() {
        assert_eq!(hmac_md5_hex("Jefe", "what do ya want for nothing?"), "750c783e6ab0b503eaa86e310a5db738");
    }
}
