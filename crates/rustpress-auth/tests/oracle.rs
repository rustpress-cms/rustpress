//! Compares RustPress with what the reference WordPress computes (`dev/auth_oracle.sh` writes
//! `tests/fixtures/oracle.json`): password hashes, cookies of every scheme, nonces and the
//! capabilities of two users. The live part needs the reference database and its `wp-config.php`
//! (`RUSTPRESS_TEST_DB`, `RUSTPRESS_TEST_WP_CONFIG`); without them it prints a note and passes.

use rustpress_auth::{Auth, CurrentUser, Roles, Salts, Scheme, password, session};
use rustpress_db::Db;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/oracle.json")).expect("the fixture is JSON")
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[test]
fn every_password_format_of_wordpress_is_accepted() {
    let f = fixture();
    let password = text(&f, "password");
    for key in ["hash_wp", "hash_phpass", "hash_md5", "hash_bcrypt"] {
        assert!(password::check_password(password, text(&f, key)), "{key} must accept the password");
        assert!(!password::check_password("falsches Passwort", text(&f, key)), "{key} must reject another password");
    }
    assert!(!password::needs_rehash(text(&f, "hash_wp")));
    assert!(password::needs_rehash(text(&f, "hash_phpass")));
    assert!(password::needs_rehash(text(&f, "hash_bcrypt")));
}

fn wp_config_path() -> std::path::PathBuf {
    std::env::var("RUSTPRESS_TEST_WP_CONFIG").map(Into::into).unwrap_or_else(|_| {
        r"\\wsl.localhost\Debian\home\yoshi\.local\share\containers\storage\volumes\rustpress-wp\_data\wp-config.php".into()
    })
}

async fn live() -> Option<(Db, Auth)> {
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let db = match Db::connect(&url, "wp_").await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("no test database ({e}), skipping");
            return None;
        }
    };
    let Ok(config) = std::fs::read_to_string(wp_config_path()) else {
        eprintln!("no wp-config.php of the reference, skipping");
        return None;
    };
    let siteurl = text(&fixture(), "siteurl").to_owned();
    if db.option("siteurl").await.ok().flatten().as_deref() != Some(siteurl.as_str()) {
        eprintln!("the fixture is from another site, skipping");
        return None;
    }
    let salts = Salts::new(rustpress_auth::salts::parse_config_constants(&config), Default::default());
    Some((db, Auth::new(salts, &siteurl)))
}

fn scheme(name: &str) -> Scheme {
    match name {
        "auth" => Scheme::Auth,
        "secure_auth" => Scheme::SecureAuth,
        _ => Scheme::LoggedIn,
    }
}

#[tokio::test]
async fn cookies_and_nonces_equal_those_of_wordpress() {
    let Some((db, auth)) = live().await else { return };
    let f = fixture();
    let now = f["now"].as_i64().unwrap();
    assert_eq!(auth.cookie_name(Scheme::LoggedIn), format!("wordpress_logged_in_{}", text(&f, "cookiehash")));
    assert_eq!(auth.cookie_name(Scheme::Auth), format!("wordpress_{}", text(&f, "cookiehash")));
    for entry in f["users"].as_array().unwrap() {
        let user = db.auth_user_by_login(text(entry, "login")).await.unwrap().expect("the user exists in the reference");
        let (expiration, token) = (entry["expiration"].as_i64().unwrap(), text(entry, "token"));
        for (name, cookie) in entry["cookies"].as_object().unwrap() {
            let cookie = cookie.as_str().unwrap();
            let scheme = scheme(name);
            assert_eq!(auth.generate_cookie(&user, expiration, scheme, token).as_deref(), Some(cookie), "the {name} cookie of {}", user.user_login);
            let session = auth.validate_cookie(&db, cookie, scheme, now, false).await.unwrap().expect("a genuine cookie validates");
            assert_eq!((session.user.id, session.token.as_str()), (user.id, token));

            // A changed cookie, a wrong scheme and an expired cookie do not.
            let mut tampered = cookie.to_owned();
            tampered.pop();
            tampered.push(if cookie.ends_with('0') { '1' } else { '0' });
            assert!(auth.validate_cookie(&db, &tampered, scheme, now, false).await.unwrap().is_none());
            let other = if name == "logged_in" { Scheme::Auth } else { Scheme::LoggedIn };
            assert!(auth.validate_cookie(&db, cookie, other, now, false).await.unwrap().is_none());
            assert!(auth.validate_cookie(&db, cookie, scheme, expiration + 1, false).await.unwrap().is_none());
            // A cookie that has just run out stays good for the session store only while the session lives:
            // the grace hour of POST requests cannot revive an expired session.
            assert!(auth.validate_cookie(&db, cookie, scheme, expiration + 3000, true).await.unwrap().is_none());
            assert!(auth.validate_cookie(&db, cookie, scheme, expiration - 10, true).await.unwrap().is_some());
        }
        for (action, nonce) in entry["nonces"].as_object().unwrap() {
            let action = if action == "other" { "log-out" } else { action.as_str() };
            let nonce = nonce.as_str().unwrap();
            assert_eq!(auth.create_nonce(action, user.id, token, now).as_deref(), Some(nonce), "the {action} nonce of {}", user.user_login);
            assert_eq!(auth.verify_nonce(nonce, action, user.id, token, now), Some(1));
            assert_eq!(auth.verify_nonce(nonce, "wp_rest_x", user.id, token, now), None);
        }
    }
}

#[tokio::test]
async fn roles_give_the_capabilities_of_wordpress() {
    let Some((db, _)) = live().await else { return };
    let roles = Roles::from_option(&db.option("wp_user_roles").await.unwrap().unwrap_or_default());
    let admin = CurrentUser::load(&db, &roles, &db.auth_user_by_login("rpadmin").await.unwrap().unwrap()).await.unwrap();
    assert_eq!(admin.roles, ["administrator"]);
    assert!(admin.can("manage_options") && admin.can("edit_posts") && admin.can("administrator"));
    let editor = CurrentUser::load(&db, &roles, &db.auth_user_by_login("rpeditor").await.unwrap().unwrap()).await.unwrap();
    assert_eq!(editor.roles, ["editor"]);
    assert!(editor.can("edit_others_posts") && editor.can("publish_pages"));
    assert!(!editor.can("manage_options") && !editor.can("install_plugins") && !editor.can("nonsense"));
}

#[tokio::test]
async fn a_new_session_is_valid_and_can_be_ended() {
    let Some((db, auth)) = live().await else { return };
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let writer = rustpress_db::Writer::connect(&url, "wp_").await.unwrap();
    let user = db.auth_user_by_login("rpeditor").await.unwrap().unwrap();
    let now = 1_900_000_000;
    let expiration = rustpress_auth::Auth::login_expiration(false, now);
    let token = session::create_session(&db, &writer, user.id, expiration, Some("127.0.0.1"), Some("test agent"), now).await.unwrap();
    let cookie = auth.generate_cookie(&user, expiration, Scheme::LoggedIn, &token).unwrap();
    assert!(auth.validate_cookie(&db, &cookie, Scheme::LoggedIn, now, false).await.unwrap().is_some());
    session::destroy_session(&db, &writer, user.id, &token, now).await.unwrap();
    assert!(auth.validate_cookie(&db, &cookie, Scheme::LoggedIn, now, false).await.unwrap().is_none());
}
