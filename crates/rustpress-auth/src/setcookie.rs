//! `setcookie()` as PHP writes it, and the cookies WordPress sets when signing in and out.

use rustpress_db::AuthUser;

use crate::cookie::{Auth, Scheme};

const DAY: i64 = 86_400;

/// One `Set-Cookie` header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCookie {
    pub name: String,
    pub value: String,
    /// Unix time; `None` makes a session cookie.
    pub expires: Option<i64>,
    pub path: String,
    pub domain: Option<String>,
    pub secure: bool,
    pub http_only: bool,
}

/// PHP's `rawurlencode()`.
fn rawurlencode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

impl SetCookie {
    /// The header value: `name=value; expires=…; Max-Age=…; path=…; domain=…; secure; HttpOnly`.
    pub fn header_value(&self, now: i64) -> String {
        let mut out = format!("{}={}", self.name, rawurlencode(&self.value));
        if let Some(expires) = self.expires {
            let date = chrono::DateTime::from_timestamp(expires.max(1), 0).map(|d| d.format("%a, %d %b %Y %H:%M:%S GMT").to_string()).unwrap_or_default();
            out.push_str(&format!("; expires={date}; Max-Age={}", (expires - now).max(0)));
        }
        out.push_str(&format!("; path={}", self.path));
        if let Some(domain) = &self.domain {
            out.push_str(&format!("; domain={domain}"));
        }
        if self.secure {
            out.push_str("; secure");
        }
        if self.http_only {
            out.push_str("; HttpOnly");
        }
        out
    }
}

/// The paths WordPress sets cookies for.
#[derive(Debug, Clone)]
pub struct CookiePaths {
    /// `COOKIEPATH`: the path of the home address.
    pub cookie: String,
    /// `SITECOOKIEPATH`: the path of the site address.
    pub site: String,
    /// `ADMIN_COOKIE_PATH`.
    pub admin: String,
    /// `PLUGINS_COOKIE_PATH`.
    pub plugins: String,
}

impl CookiePaths {
    /// From the `home` and `siteurl` options (`wp-includes/default-constants.php`).
    pub fn from_urls(home: &str, siteurl: &str) -> Self {
        let path_of = |url: &str| -> String {
            let without_origin = match url.split_once("://") {
                Some((_, rest)) => rest.find('/').map_or(String::new(), |at| rest[at..].to_owned()),
                None => url.to_owned(),
            };
            format!("{without_origin}/")
        };
        let cookie = path_of(home);
        let site = path_of(siteurl);
        let site_trimmed = site.trim_end_matches('/');
        Self { admin: format!("{site}wp-admin"), plugins: format!("{site_trimmed}/wp-content/plugins"), cookie, site }
    }
}

fn cookie(name: String, value: String, expires: Option<i64>, path: &str, secure: bool, http_only: bool) -> SetCookie {
    SetCookie { name, value, expires, path: path.to_owned(), domain: None, secure, http_only }
}

impl Auth {
    /// `wp_set_auth_cookie()`: the cookies of a new sign-in for `token`.
    pub fn login_cookies(&self, user: &AuthUser, remember: bool, secure: bool, home_is_https: bool, token: &str, now: i64, expiration: i64, paths: &CookiePaths) -> Vec<SetCookie> {
        let expire = if remember { Some(expiration + 12 * 3600) } else { None };
        let secure_logged_in = secure && home_is_https;
        let (name, scheme) = if secure { (self.cookie_name(Scheme::SecureAuth), Scheme::SecureAuth) } else { (self.cookie_name(Scheme::Auth), Scheme::Auth) };
        let _ = now;
        let auth_cookie = self.generate_cookie(user, expiration, scheme, token).unwrap_or_default();
        let logged_in = self.generate_cookie(user, expiration, Scheme::LoggedIn, token).unwrap_or_default();
        let mut cookies = vec![
            cookie(name.clone(), auth_cookie.clone(), expire, &paths.plugins, secure, true),
            cookie(name, auth_cookie, expire, &paths.admin, secure, true),
            cookie(self.cookie_name(Scheme::LoggedIn), logged_in.clone(), expire, &paths.cookie, secure_logged_in, true),
        ];
        if paths.cookie != paths.site {
            cookies.push(cookie(self.cookie_name(Scheme::LoggedIn), logged_in, expire, &paths.site, secure_logged_in, true));
        }
        cookies
    }

    /// The default lifetime of a sign-in: 14 days when remembered, else 2 days.
    pub fn login_expiration(remember: bool, now: i64) -> i64 {
        now + if remember { 14 * DAY } else { 2 * DAY }
    }

    /// `wp_clear_auth_cookie()`: every cookie of a sign-in, emptied and expired.
    pub fn clear_cookies(&self, user_id: u64, now: i64, home_hash: &str, paths: &CookiePaths) -> Vec<SetCookie> {
        let past = Some(now - 365 * DAY);
        let blank = |name: String, path: &str| cookie(name, " ".to_owned(), past, path, false, false);
        let auth = self.cookie_name(Scheme::Auth);
        let secure = self.cookie_name(Scheme::SecureAuth);
        let logged_in = self.cookie_name(Scheme::LoggedIn);
        vec![
            blank(auth.clone(), &paths.admin),
            blank(secure.clone(), &paths.admin),
            blank(auth.clone(), &paths.plugins),
            blank(secure.clone(), &paths.plugins),
            blank(logged_in.clone(), &paths.cookie),
            blank(logged_in, &paths.site),
            blank(format!("wp-settings-{user_id}"), &paths.site),
            blank(format!("wp-settings-time-{user_id}"), &paths.site),
            blank(auth.clone(), &paths.cookie),
            blank(auth, &paths.site),
            blank(secure.clone(), &paths.cookie),
            blank(secure, &paths.site),
            blank(format!("wordpressuser_{home_hash}"), &paths.cookie),
            blank(format!("wordpresspass_{home_hash}"), &paths.cookie),
            blank(format!("wordpressuser_{home_hash}"), &paths.site),
            blank(format!("wordpresspass_{home_hash}"), &paths.site),
            blank(format!("wp-postpass_{home_hash}"), &paths.cookie),
        ]
    }

    /// `COOKIEHASH`.
    pub fn cookie_hash(&self) -> &str {
        self.hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_look_like_phps() {
        let c = SetCookie { name: "wordpress_test_cookie".into(), value: "WP Cookie check".into(), expires: None, path: "/".into(), domain: None, secure: false, http_only: true };
        assert_eq!(c.header_value(0), "wordpress_test_cookie=WP%20Cookie%20check; path=/; HttpOnly");
        let c = SetCookie { name: "a".into(), value: "x|y".into(), expires: Some(1_792_915_028 + 31_536_000), path: "/".into(), domain: None, secure: true, http_only: true };
        assert_eq!(c.header_value(1_792_915_028), "a=x%7Cy; expires=Mon, 25 Oct 2027 07:57:08 GMT; Max-Age=31536000; path=/; secure; HttpOnly");
    }

    #[test]
    fn paths_follow_the_site_address() {
        let p = CookiePaths::from_urls("http://localhost:8080", "http://localhost:8080");
        assert_eq!((p.cookie.as_str(), p.admin.as_str(), p.plugins.as_str()), ("/", "/wp-admin", "/wp-content/plugins"));
        let p = CookiePaths::from_urls("https://example.org/blog", "https://example.org/blog/wp");
        assert_eq!((p.cookie.as_str(), p.site.as_str(), p.admin.as_str(), p.plugins.as_str()), ("/blog/", "/blog/wp/", "/blog/wp/wp-admin", "/blog/wp/wp-content/plugins"));
    }
}
