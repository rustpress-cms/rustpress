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
    let url = std::env::var("RUSTPRESS_TEST_DB").unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into());
    let writer = rustpress_db::Writer::connect(&url, "wp_").await.unwrap();
    let real_now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    for entry in f["users"].as_array().unwrap() {
        let user = db.auth_user_by_login(text(entry, "login")).await.unwrap().expect("the user exists in the reference");
        let (expiration, token) = (entry["expiration"].as_i64().unwrap(), text(entry, "token"));
        // The session WordPress created for the fixture goes with the cleanup of the shared database: it is stored
        // again for the test and removed afterwards, whatever the checks find.
        session::update_session(&db, &writer, user.id, token, expiration, now).await.unwrap();
        let problems = check_user(&db, &auth, entry, &user, now, expiration, token).await;
        session::destroy_session(&db, &writer, user.id, token, real_now).await.unwrap();
        // The cookies hold a piece of the password hash: after a password change `dev/auth_oracle.sh` makes a new fixture.
        assert!(problems.is_empty(), "{} differs from WordPress (password changed? run dev/auth_oracle.sh):\n{}", user.user_login, problems.join("\n"));
    }
}

/// The cookies and nonces of one user of the fixture: what differs from WordPress.
async fn check_user(db: &Db, auth: &Auth, entry: &Value, user: &rustpress_db::AuthUser, now: i64, expiration: i64, token: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let mut check = |ok: bool, what: String| {
        if !ok {
            problems.push(what);
        }
    };
    // The comparison scripts reset the passwords of the test users; the cookies of the fixture then belong to the old one.
    let same_password = user.user_pass.get(8..12) == Some(text(entry, "pass_frag"));
    if !same_password {
        eprintln!("the password of {} changed since dev/auth_oracle.sh ran: only the nonces are compared", user.user_login);
    }
    for (name, cookie) in entry["cookies"].as_object().unwrap().iter().filter(|_| same_password) {
        let cookie = cookie.as_str().unwrap();
        let scheme = scheme(name);
        check(auth.generate_cookie(user, expiration, scheme, token).as_deref() == Some(cookie), format!("the {name} cookie"));
        let session = auth.validate_cookie(db, cookie, scheme, now, false).await.unwrap();
        check(session.as_ref().is_some_and(|s| (s.user.id, s.token.as_str()) == (user.id, token)), format!("the {name} cookie validates to the session"));

        // A changed cookie, a wrong scheme and an expired cookie do not.
        let mut tampered = cookie.to_owned();
        tampered.pop();
        tampered.push(if cookie.ends_with('0') { '1' } else { '0' });
        check(auth.validate_cookie(db, &tampered, scheme, now, false).await.unwrap().is_none(), format!("a changed {name} cookie is refused"));
        let other = if name == "logged_in" { Scheme::Auth } else { Scheme::LoggedIn };
        check(auth.validate_cookie(db, cookie, other, now, false).await.unwrap().is_none(), format!("the {name} cookie is refused for another scheme"));
        check(auth.validate_cookie(db, cookie, scheme, expiration + 1, false).await.unwrap().is_none(), format!("an expired {name} cookie is refused"));
        // A cookie that has just run out stays good for the session store only while the session lives:
        // the grace hour of POST requests cannot revive an expired session.
        check(auth.validate_cookie(db, cookie, scheme, expiration + 3000, true).await.unwrap().is_none(), format!("the grace hour does not revive the {name} session"));
        check(auth.validate_cookie(db, cookie, scheme, expiration - 10, true).await.unwrap().is_some(), format!("the {name} cookie is good until it expires"));
    }
    for (action, nonce) in entry["nonces"].as_object().unwrap() {
        let action = if action == "other" { "log-out" } else { action.as_str() };
        let nonce = nonce.as_str().unwrap();
        check(auth.create_nonce(action, user.id, token, now).as_deref() == Some(nonce), format!("the {action} nonce"));
        check(auth.verify_nonce(nonce, action, user.id, token, now) == Some(1), format!("the {action} nonce verifies"));
        check(auth.verify_nonce(nonce, "wp_rest_x", user.id, token, now).is_none(), format!("the {action} nonce is refused for another action"));
    }
    problems
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
