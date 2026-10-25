//! Authentication cookies (`wp_generate_auth_cookie()`, `wp_validate_auth_cookie()`), sessions
//! (`WP_Session_Tokens`) and nonces.

use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use rustpress_db::{AuthUser, Db};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::salts::Salts;

/// The three cookie schemes. The frontend uses `LoggedIn`, `wp-admin` the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Auth,
    SecureAuth,
    LoggedIn,
}

impl Scheme {
    /// The name `wp_hash()` knows the scheme by.
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Auth => "auth",
            Scheme::SecureAuth => "secure_auth",
            Scheme::LoggedIn => "logged_in",
        }
    }
}

/// Everything needed to make and check cookies and nonces for one site.
#[derive(Debug, Clone)]
pub struct Auth {
    salts: Salts,
    cookie_hash: String,
}

/// What a valid cookie stands for.
#[derive(Debug, Clone)]
pub struct Session {
    pub user: AuthUser,
    pub token: String,
}

/// The pieces of `username|expiration|token|hmac`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCookie {
    pub username: String,
    pub expiration: i64,
    pub token: String,
    pub hmac: String,
}

impl ParsedCookie {
    /// `wp_parse_auth_cookie()` for the cookie value.
    pub fn parse(cookie: &str) -> Option<Self> {
        let parts: Vec<&str> = cookie.split('|').collect();
        let [username, expiration, token, hmac] = parts.as_slice() else { return None };
        // PHP compares the expiration as an integer; text that is not a number counts as 0.
        let expiration = expiration.trim().parse::<i64>().unwrap_or(0);
        Some(Self { username: (*username).to_owned(), expiration, token: (*token).to_owned(), hmac: (*hmac).to_owned() })
    }
}

fn hmac_sha256_hex(key: &str, data: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("hmac accepts any key length");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn sha256_hex(data: &str) -> String {
    sha2::Sha256::digest(data.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// The part of the stored password hash that goes into the cookie key.
fn pass_fragment(user_pass: &str) -> String {
    let chars: Vec<char> = user_pass.chars().collect();
    if user_pass.starts_with("$P$") || user_pass.starts_with("$2y$") {
        chars.iter().skip(8).take(4).collect()
    } else {
        chars.iter().skip(chars.len().saturating_sub(4)).collect()
    }
}

impl Auth {
    /// `siteurl` is the option as stored (it names the cookies: `COOKIEHASH` is its md5).
    pub fn new(salts: Salts, siteurl: &str) -> Self {
        let cookie_hash = Md5::digest(siteurl.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        Self { salts, cookie_hash }
    }

    pub(crate) fn hash(&self) -> &str {
        &self.cookie_hash
    }

    pub fn salts(&self) -> &Salts {
        &self.salts
    }

    /// The cookie names: `wordpress_<hash>`, `wordpress_sec_<hash>`, `wordpress_logged_in_<hash>`.
    pub fn cookie_name(&self, scheme: Scheme) -> String {
        match scheme {
            Scheme::Auth => format!("wordpress_{}", self.cookie_hash),
            Scheme::SecureAuth => format!("wordpress_sec_{}", self.cookie_hash),
            Scheme::LoggedIn => format!("wordpress_logged_in_{}", self.cookie_hash),
        }
    }

    /// The hash that comes last in an authentication cookie.
    fn cookie_hmac(&self, user: &AuthUser, expiration: i64, token: &str, scheme: Scheme) -> Option<String> {
        let key = self.salts.hash(&format!("{}|{}|{expiration}|{token}", user.user_login, pass_fragment(&user.user_pass)), scheme.as_str())?;
        Some(hmac_sha256_hex(&key, &format!("{}|{expiration}|{token}", user.user_login)))
    }

    /// `wp_generate_auth_cookie()` for an existing session token.
    pub fn generate_cookie(&self, user: &AuthUser, expiration: i64, scheme: Scheme, token: &str) -> Option<String> {
        let hash = self.cookie_hmac(user, expiration, token, scheme)?;
        Some(format!("{}|{expiration}|{token}|{hash}", user.user_login))
    }

    /// `wp_validate_auth_cookie()`: the user and the session token, if the cookie is genuine, unexpired
    /// and its session still exists. A cookie sent with a POST or AJAX request gets an hour of grace.
    pub async fn validate_cookie(&self, db: &Db, cookie: &str, scheme: Scheme, now: i64, grace: bool) -> Result<Option<Session>, rustpress_db::Error> {
        let Some(parsed) = ParsedCookie::parse(cookie) else { return Ok(None) };
        let expired = parsed.expiration + if grace { 3600 } else { 0 };
        if expired < now {
            return Ok(None);
        }
        let Some(user) = db.auth_user_by_login(&parsed.username).await? else { return Ok(None) };
        let Some(expected) = self.cookie_hmac(&user, parsed.expiration, &parsed.token, scheme) else { return Ok(None) };
        if !bool::from(expected.as_bytes().ct_eq(parsed.hmac.as_bytes())) {
            return Ok(None);
        }
        if !self.session_exists(db, user.id, &parsed.token, now).await? {
            return Ok(None);
        }
        Ok(Some(Session { user, token: parsed.token }))
    }

    /// `WP_Session_Tokens::verify()`: the user's `session_tokens` meta holds the sha256 of every live token.
    async fn session_exists(&self, db: &Db, user_id: u64, token: &str, now: i64) -> Result<bool, rustpress_db::Error> {
        let Some(raw) = db.user_meta(user_id, "session_tokens").await? else { return Ok(false) };
        let Ok(sessions) = rustpress_phpser::unserialize(raw.as_bytes()) else { return Ok(false) };
        let verifier = sha256_hex(token);
        Ok(sessions
            .get(&verifier)
            .and_then(|session| session.get("expiration"))
            .and_then(rustpress_phpser::Value::as_i64)
            .is_some_and(|expiration| expiration >= now))
    }

    /// `wp_nonce_tick()` with the default life of a day.
    pub fn nonce_tick(now: i64) -> i64 {
        // ceil( time() / ( 86400 / 2 ) )
        (now + 43_199) / 43_200
    }

    /// `wp_create_nonce()` for a user and the token of their session.
    pub fn create_nonce(&self, action: &str, user_id: u64, token: &str, now: i64) -> Option<String> {
        self.nonce(action, user_id, token, Self::nonce_tick(now))
    }

    fn nonce(&self, action: &str, user_id: u64, token: &str, tick: i64) -> Option<String> {
        let hash = self.salts.hash(&format!("{tick}|{action}|{user_id}|{token}"), "nonce")?;
        Some(hash[hash.len() - 12..hash.len() - 2].to_owned())
    }

    /// `wp_verify_nonce()`: 1 for a nonce from the first half of its life, 2 for the second, `None` if invalid.
    pub fn verify_nonce(&self, nonce: &str, action: &str, user_id: u64, token: &str, now: i64) -> Option<u8> {
        if nonce.is_empty() {
            return None;
        }
        let tick = Self::nonce_tick(now);
        for (age, tick) in [(1, tick), (2, tick - 1)] {
            if self.nonce(action, user_id, token, tick).is_some_and(|expected| bool::from(expected.as_bytes().ct_eq(nonce.as_bytes()))) {
                return Some(age);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn auth() -> Auth {
        let constants: HashMap<String, String> = [
            ("AUTH_KEY", "ak"),
            ("AUTH_SALT", "as"),
            ("SECURE_AUTH_KEY", "sk"),
            ("SECURE_AUTH_SALT", "ss"),
            ("LOGGED_IN_KEY", "lk"),
            ("LOGGED_IN_SALT", "ls"),
            ("NONCE_KEY", "nk"),
            ("NONCE_SALT", "ns"),
        ]
        .iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect();
        Auth::new(Salts::new(constants, HashMap::new()), "http://localhost:8080")
    }

    fn user(pass: &str) -> AuthUser {
        AuthUser { id: 7, user_login: "anna".into(), user_pass: pass.into(), user_nicename: "anna".into(), user_email: "a@example.org".into(), display_name: "Anna".into() }
    }

    #[test]
    fn cookie_names_carry_the_md5_of_the_site_url() {
        let a = auth();
        assert_eq!(a.cookie_name(Scheme::LoggedIn), format!("wordpress_logged_in_{:x}", Md5::digest(b"http://localhost:8080")));
        assert!(a.cookie_name(Scheme::SecureAuth).starts_with("wordpress_sec_"));
    }

    #[test]
    fn generated_cookies_parse_back() {
        let a = auth();
        let cookie = a.generate_cookie(&user("$P$B1234567890abcdefghijklmnopqrs"), 1_900_000_000, Scheme::LoggedIn, "tok").unwrap();
        let parsed = ParsedCookie::parse(&cookie).unwrap();
        assert_eq!((parsed.username.as_str(), parsed.expiration, parsed.token.as_str()), ("anna", 1_900_000_000, "tok"));
        assert_eq!(parsed.hmac.len(), 64);
        assert_eq!(ParsedCookie::parse("a|b|c"), None);
    }

    #[test]
    fn the_password_fragment_depends_on_the_hash_format() {
        assert_eq!(pass_fragment("$P$B1234567890"), "5678");
        assert_eq!(pass_fragment("$2y$10$abcdefgh"), "bcde");
        assert_eq!(pass_fragment("$wp$2y$10$abcdefghijkl"), "ijkl");
    }

    #[test]
    fn nonces_live_two_ticks() {
        let a = auth();
        let now = 1_900_000_000;
        let nonce = a.create_nonce("wp_rest", 7, "tok", now).unwrap();
        assert_eq!(nonce.len(), 10);
        assert_eq!(a.verify_nonce(&nonce, "wp_rest", 7, "tok", now), Some(1));
        assert_eq!(a.verify_nonce(&nonce, "wp_rest", 7, "tok", now + 43_200), Some(2));
        assert_eq!(a.verify_nonce(&nonce, "wp_rest", 7, "tok", now + 86_400 + 43_200), None);
        assert_eq!(a.verify_nonce(&nonce, "other", 7, "tok", now), None);
        assert_eq!(a.verify_nonce(&nonce, "wp_rest", 8, "tok", now), None);
    }

    #[test]
    fn nonce_tick_rounds_up() {
        assert_eq!(Auth::nonce_tick(0), 0);
        assert_eq!(Auth::nonce_tick(1), 1);
        assert_eq!(Auth::nonce_tick(43_200), 1);
        assert_eq!(Auth::nonce_tick(43_201), 2);
    }
}
