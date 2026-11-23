//! The role matrix: the same requests as an administrator, an editor, an author, a contributor, a subscriber and a
//! visitor, sent to the reference WordPress (`dev/up.sh`) and to RustPress. What each role may see and change has to
//! be equal: the status, the error code of a REST answer, the message of `wp_die()`, the target of a redirect and
//! the short answers of `admin-ajax.php`.
//!
//! The test signs in on the reference (RustPress shares its database and salts, so the same cookies and nonces are
//! valid on both sides), creates its users and posts through the REST API of the reference and removes them again at
//! the end, also when a comparison failed. Writes are chosen so that sending them twice gives the same answer.
//! Skips when the stack is not running. `RUSTPRESS_ONLY=<text>` limits the run to the cases whose line contains it.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use rustpress_server::{App, Config};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Where the WordPress files are when `RUSTPRESS_WP_ROOT` is not set: `wordpress/` in the repository (not checked in).
const DEFAULT_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../wordpress");
const HOST: &str = "localhost:8080";
const PASSWORD: &str = "rp-role-matrix-1";
/// The roles below the administrator; the test creates one user of each (`rp-role-<role>`).
const ROLES: [&str; 4] = ["editor", "author", "contributor", "subscriber"];

/// Differences that are known and accepted, by a part of the case line (`<role> <METHOD> <path>`), with the reason.
const KNOWN: &[(&str, &str)] = &[];

/// What is compared of one answer.
#[derive(Debug, PartialEq, Eq)]
struct Answer {
    status: u16,
    /// The `code` of a REST error, the message of `wp_die()`, the `Location` of a redirect, or a short body.
    detail: String,
}

struct Raw {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Raw {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    fn answer(&self, rest: bool) -> Answer {
        let text = String::from_utf8_lossy(&self.body);
        let json = self.json();
        let detail = if let Some(location) = self
            .header("location")
            .filter(|_| (300..400).contains(&self.status))
        {
            location.to_owned()
        } else if let Some(code) = json.get("code").and_then(Value::as_str) {
            format!(
                "{code}: {}",
                json.get("message").and_then(Value::as_str).unwrap_or("")
            )
        } else if rest {
            String::new()
        } else if let Some(object) = json.as_object() {
            // An answer of admin-ajax.php: whether it succeeded and what it names (the values hold times and nonces).
            let keys: Vec<&str> = object.keys().map(String::as_str).collect();
            format!(
                "{} {}",
                object
                    .get("success")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                keys.join(",")
            )
        } else if let Some(start) = text.find("<div class=\"wp-die-message\">") {
            let rest = &text[start + 28..];
            rest[..rest.find("</div>").unwrap_or(rest.len())]
                .trim()
                .to_owned()
        } else if self.status != 200 || text.len() <= 40 {
            text.chars().take(200).collect()
        } else {
            String::new()
        };
        Answer {
            status: self.status,
            detail,
        }
    }
}

/// Sends a request to the reference WordPress.
fn reference(
    method: &str,
    path: &str,
    cookies: &str,
    extra: &[(&str, &str)],
    body: &[u8],
) -> std::io::Result<Raw> {
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    let mut head = format!(
        "{method} {path} HTTP/1.0\r\nHost: {HOST}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !cookies.is_empty() {
        head.push_str(&format!("Cookie: {cookies}\r\n"));
    }
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(raw.len());
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let headers = head
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect();
    Ok(Raw {
        status,
        headers,
        body: raw[(split + 4).min(raw.len())..].to_vec(),
    })
}

/// Sends the same request to RustPress.
async fn ours(
    app: &std::sync::Arc<App>,
    method: &str,
    path: &str,
    cookies: &str,
    extra: &[(&str, &str)],
    body: &[u8],
) -> Raw {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, HOST);
    if !cookies.is_empty() {
        request = request.header(header::COOKIE, cookies);
    }
    for (name, value) in extra {
        request = request.header(*name, *value);
    }
    let response = app
        .router()
        .oneshot(request.body(Body::from(body.to_vec())).unwrap())
        .await
        .unwrap();
    let (parts, body) = response.into_parts();
    let headers = parts
        .headers
        .iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    Raw {
        status: parts.status.as_u16(),
        headers,
        body: body.collect().await.unwrap().to_bytes().to_vec(),
    }
}

/// Somebody who is signed in on the reference: the cookies of `wp-login.php` and the REST nonce.
struct Session {
    role: &'static str,
    cookies: String,
    nonce: String,
}

impl Session {
    fn visitor() -> Self {
        Self {
            role: "visitor",
            cookies: String::new(),
            nonce: String::new(),
        }
    }

    fn sign_in(role: &'static str, login: &str, password: &str) -> Self {
        let form = format!("log={login}&pwd={password}&testcookie=1");
        let answer = reference(
            "POST",
            "/wp-login.php",
            "wordpress_test_cookie=WP%20Cookie%20check",
            &[("Content-Type", "application/x-www-form-urlencoded")],
            form.as_bytes(),
        )
        .unwrap();
        assert_eq!(answer.status, 302, "signing in as {login} on the reference");
        let mut cookies: Vec<&str> = Vec::new();
        for (key, value) in &answer.headers {
            let pair = value.split(';').next().unwrap_or("");
            if key.eq_ignore_ascii_case("set-cookie")
                && pair.starts_with("wordpress_")
                && !cookies.contains(&pair)
            {
                cookies.push(pair);
            }
        }
        let cookies = cookies.join("; ");
        let nonce = reference(
            "GET",
            "/wp-admin/admin-ajax.php?action=rest-nonce",
            &cookies,
            &[],
            b"",
        )
        .unwrap();
        Self {
            role,
            cookies,
            nonce: String::from_utf8_lossy(&nonce.body).trim().to_owned(),
        }
    }

    /// A REST request on the reference with this session.
    fn rest(&self, method: &str, route: &str, body: &Value) -> Raw {
        let body = if body.is_null() {
            Vec::new()
        } else {
            serde_json::to_vec(body).unwrap()
        };
        reference(
            method,
            &format!("/wp-json{route}"),
            &self.cookies,
            &[
                ("X-WP-Nonce", &self.nonce),
                ("Content-Type", "application/json"),
            ],
            &body,
        )
        .unwrap()
    }

    /// Creates something through the REST API of the reference and returns its ID.
    fn create(&self, route: &str, body: Value) -> u64 {
        let answer = self.rest("POST", route, &body);
        answer
            .json()
            .get("id")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| {
                panic!(
                    "creating {route} on the reference: {} {}",
                    answer.status,
                    String::from_utf8_lossy(&answer.body)
                )
            })
    }
}

/// What the test created on the reference, to send requests about and to remove at the end.
struct Fixture {
    users: Vec<(&'static str, u64)>,
    /// A draft and a published post of the administrator, a published post of the author, a draft of the contributor.
    draft: u64,
    published: u64,
    authors: u64,
    contributors: u64,
    page: u64,
    comment: u64,
}

impl Fixture {
    fn user(&self, role: &str) -> u64 {
        self.users
            .iter()
            .find(|(name, _)| *name == role)
            .map_or(1, |(_, id)| *id)
    }
}

/// Removes what an earlier run left behind (it stopped before its end).
fn remove_leftovers(admin: &Session) {
    let users = admin
        .rest(
            "GET",
            "/wp/v2/users?context=edit&search=rp-role-&per_page=100",
            &Value::Null,
        )
        .json();
    for user in users.as_array().into_iter().flatten() {
        if let Some(id) = user.get("id").and_then(Value::as_u64).filter(|_| {
            user.get("username")
                .and_then(Value::as_str)
                .is_some_and(|name| name.starts_with("rp-role-"))
        }) {
            admin.rest(
                "DELETE",
                &format!("/wp/v2/users/{id}?force=true&reassign=1"),
                &Value::Null,
            );
        }
    }
    for kind in ["posts", "pages"] {
        let posts = admin
            .rest(
                "GET",
                &format!(
                    "/wp/v2/{kind}?context=edit&status=any&search=rp-role-matrix&per_page=100"
                ),
                &Value::Null,
            )
            .json();
        for id in posts
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|post| post.get("id").and_then(Value::as_u64))
        {
            admin.rest(
                "DELETE",
                &format!("/wp/v2/{kind}/{id}?force=true"),
                &Value::Null,
            );
        }
    }
}

fn fixture(admin: &Session) -> Fixture {
    let users: Vec<(&'static str, u64)> = ROLES.iter().map(|role| (*role, admin.create("/wp/v2/users", json!({ "username": format!("rp-role-{role}"), "email": format!("rp-role-{role}@example.org"), "password": PASSWORD, "roles": [role] })))).collect();
    let user = |role: &str| users.iter().find(|(name, _)| *name == role).unwrap().1;
    let post = |title: &str, status: &str, author: u64| {
        admin.create("/wp/v2/posts", json!({ "title": format!("rp-role-matrix {title}"), "content": "rp-role-matrix", "status": status, "author": author }))
    };
    let draft = post("draft", "draft", 1);
    let published = post("published", "publish", 1);
    let authors = post("of the author", "publish", user("author"));
    let contributors = post("of the contributor", "draft", user("contributor"));
    let page = admin.create(
        "/wp/v2/pages",
        json!({ "title": "rp-role-matrix page", "content": "rp-role-matrix", "status": "publish" }),
    );
    let comment = admin.create(
        "/wp/v2/comments",
        json!({ "post": published, "content": "rp-role-matrix comment" }),
    );
    Fixture {
        users,
        draft,
        published,
        authors,
        contributors,
        page,
        comment,
    }
}

/// One request of the matrix; `{draft}`, `{published}`, `{authors}`, `{contributors}`, `{page}`, `{comment}`,
/// `{me}` and `{user:<role>}` in the path or body stand for the IDs of the fixture.
struct Case {
    method: &'static str,
    path: &'static str,
    /// JSON for a REST route, a form for everything else.
    body: &'static str,
}

const fn get(path: &'static str) -> Case {
    Case {
        method: "GET",
        path,
        body: "",
    }
}

const fn post(path: &'static str, body: &'static str) -> Case {
    Case {
        method: "POST",
        path,
        body,
    }
}

const CASES: &[Case] = &[
    // The REST API: reading in the context of the editor.
    get("/wp-json/wp/v2/posts?context=edit"),
    get("/wp-json/wp/v2/posts?status=draft"),
    get("/wp-json/wp/v2/posts/{draft}"),
    get("/wp-json/wp/v2/posts/{draft}?context=edit"),
    get("/wp-json/wp/v2/posts/{published}?context=edit"),
    get("/wp-json/wp/v2/posts/{authors}?context=edit"),
    get("/wp-json/wp/v2/posts/{contributors}?context=edit"),
    get("/wp-json/wp/v2/posts/{published}/revisions"),
    get("/wp-json/wp/v2/pages?context=edit"),
    get("/wp-json/wp/v2/pages/{page}?context=edit"),
    get("/wp-json/wp/v2/media?context=edit"),
    get("/wp-json/wp/v2/comments?context=edit"),
    get("/wp-json/wp/v2/comments?status=hold"),
    get("/wp-json/wp/v2/comments/{comment}?context=edit"),
    get("/wp-json/wp/v2/categories?context=edit"),
    get("/wp-json/wp/v2/tags?context=edit"),
    get("/wp-json/wp/v2/users"),
    get("/wp-json/wp/v2/users?context=edit"),
    get("/wp-json/wp/v2/users?roles=subscriber"),
    get("/wp-json/wp/v2/users/me"),
    get("/wp-json/wp/v2/users/me?context=edit"),
    get("/wp-json/wp/v2/users/1?context=edit"),
    get("/wp-json/wp/v2/users/{user:subscriber}"),
    get("/wp-json/wp/v2/users/{user:subscriber}?context=edit"),
    get("/wp-json/wp/v2/users/{me}/application-passwords"),
    get("/wp-json/wp/v2/settings"),
    get("/wp-json/wp/v2/types?context=edit"),
    get("/wp-json/wp/v2/statuses?context=edit"),
    get("/wp-json/wp/v2/taxonomies?context=edit"),
    get("/wp-json/wp/v2/themes"),
    get("/wp-json/wp/v2/plugins"),
    get("/wp-json/wp/v2/block-types"),
    get("/wp-json/wp/v2/blocks"),
    get("/wp-json/wp/v2/templates"),
    get("/wp-json/wp/v2/template-parts"),
    get("/wp-json/wp/v2/navigation?context=edit"),
    get("/wp-json/wp/v2/menus"),
    get("/wp-json/wp/v2/menu-items"),
    get("/wp-json/wp/v2/menu-locations"),
    get("/wp-json/wp/v2/sidebars"),
    get("/wp-json/wp/v2/widgets"),
    get("/wp-json/wp/v2/widget-types"),
    get("/wp-json/wp/v2/global-styles/themes/twentytwentyfive"),
    get("/wp-json/wp/v2/font-families"),
    get("/wp-json/wp/v2/block-directory/search?term=gallery"),
    get("/wp-json/wp/v2/pattern-directory/patterns"),
    get("/wp-json/wp/v2/block-patterns/patterns"),
    get("/wp-json/wp-site-health/v1/tests/background-updates"),
    // The REST API: writing.
    post(
        "/wp-json/wp/v2/posts",
        r#"{"title":"rp-role-matrix new","status":"draft"}"#,
    ),
    post(
        "/wp-json/wp/v2/posts",
        r#"{"title":"rp-role-matrix new","status":"publish"}"#,
    ),
    post(
        "/wp-json/wp/v2/posts",
        r#"{"title":"rp-role-matrix new","status":"draft","author":1}"#,
    ),
    post(
        "/wp-json/wp/v2/posts",
        r#"{"title":"rp-role-matrix new","status":"draft","sticky":true}"#,
    ),
    post(
        "/wp-json/wp/v2/posts/{draft}",
        r#"{"title":"rp-role-matrix draft"}"#,
    ),
    post(
        "/wp-json/wp/v2/posts/{published}",
        r#"{"title":"rp-role-matrix published"}"#,
    ),
    post(
        "/wp-json/wp/v2/posts/{authors}",
        r#"{"title":"rp-role-matrix of the author"}"#,
    ),
    post(
        "/wp-json/wp/v2/posts/{contributors}",
        r#"{"title":"rp-role-matrix of the contributor"}"#,
    ),
    post(
        "/wp-json/wp/v2/posts/{contributors}",
        r#"{"status":"publish","date":"2099-01-01T00:00:00"}"#,
    ),
    post(
        "/wp-json/wp/v2/pages",
        r#"{"title":"rp-role-matrix new","status":"draft"}"#,
    ),
    post(
        "/wp-json/wp/v2/pages/{page}",
        r#"{"title":"rp-role-matrix page"}"#,
    ),
    post(
        "/wp-json/wp/v2/comments/{comment}",
        r#"{"content":"rp-role-matrix comment"}"#,
    ),
    post(
        "/wp-json/wp/v2/comments/{comment}",
        r#"{"status":"approved"}"#,
    ),
    post("/wp-json/wp/v2/categories", r#"{"name":""}"#),
    post("/wp-json/wp/v2/categories/1", r#"{"description":""}"#),
    post("/wp-json/wp/v2/tags", r#"{"name":""}"#),
    post(
        "/wp-json/wp/v2/users",
        r#"{"username":"admin","email":"rp-role-other@example.org","password":"rp-role-matrix-1"}"#,
    ),
    post("/wp-json/wp/v2/users/me", r#"{"description":""}"#),
    post(
        "/wp-json/wp/v2/users/{me}",
        r#"{"roles":["administrator"]}"#,
    ),
    post(
        "/wp-json/wp/v2/users/{user:subscriber}",
        r#"{"description":""}"#,
    ),
    post("/wp-json/wp/v2/users/1", r#"{"description":""}"#),
    post("/wp-json/wp/v2/settings", r#"{"posts_per_page":5}"#),
    post("/wp-json/wp/v2/plugins/hello", r#"{"status":"inactive"}"#),
    Case {
        method: "DELETE",
        path: "/wp-json/wp/v2/posts/999999",
        body: "",
    },
    Case {
        method: "DELETE",
        path: "/wp-json/wp/v2/users/999999?force=true&reassign=1",
        body: "",
    },
    // The screens of the administration.
    get("/wp-admin/"),
    get("/wp-admin/index.php"),
    get("/wp-admin/edit.php"),
    get("/wp-admin/post-new.php"),
    get("/wp-admin/post.php?post={draft}&action=edit"),
    get("/wp-admin/post.php?post={authors}&action=edit"),
    get("/wp-admin/post.php?post={contributors}&action=edit"),
    get("/wp-admin/post.php?post={draft}&action=trash"),
    get("/wp-admin/edit.php?post_type=page"),
    get("/wp-admin/post-new.php?post_type=page"),
    get("/wp-admin/post.php?post={page}&action=edit"),
    get("/wp-admin/upload.php"),
    get("/wp-admin/media-new.php"),
    get("/wp-admin/edit-comments.php"),
    get("/wp-admin/comment.php?action=editcomment&c={comment}"),
    get("/wp-admin/edit-tags.php?taxonomy=category"),
    get("/wp-admin/edit-tags.php?taxonomy=post_tag"),
    get("/wp-admin/term.php?taxonomy=category&tag_ID=1"),
    get("/wp-admin/users.php"),
    get("/wp-admin/user-new.php"),
    get("/wp-admin/profile.php"),
    get("/wp-admin/user-edit.php?user_id=1"),
    get("/wp-admin/user-edit.php?user_id={user:subscriber}"),
    get("/wp-admin/themes.php"),
    get("/wp-admin/site-editor.php"),
    get("/wp-admin/customize.php"),
    get("/wp-admin/nav-menus.php"),
    get("/wp-admin/widgets.php"),
    get("/wp-admin/plugins.php"),
    get("/wp-admin/plugin-install.php"),
    get("/wp-admin/theme-install.php"),
    get("/wp-admin/tools.php"),
    get("/wp-admin/import.php"),
    get("/wp-admin/export.php"),
    get("/wp-admin/site-health.php"),
    get("/wp-admin/export-personal-data.php"),
    get("/wp-admin/erase-personal-data.php"),
    get("/wp-admin/update-core.php"),
    get("/wp-admin/options-general.php"),
    get("/wp-admin/options-writing.php"),
    get("/wp-admin/options-reading.php"),
    get("/wp-admin/options-discussion.php"),
    get("/wp-admin/options-media.php"),
    get("/wp-admin/options-permalink.php"),
    get("/wp-admin/options-privacy.php"),
    get("/wp-admin/options.php"),
    // Writing in the administration without a nonce: who is refused for the permission and who for the nonce.
    post(
        "/wp-admin/options.php",
        "option_page=general&action=update&blogname=x",
    ),
    post(
        "/wp-admin/post.php",
        "action=editpost&post_ID={draft}&post_title=x",
    ),
    post(
        "/wp-admin/post.php",
        "action=editpost&post_ID={contributors}&post_title=x",
    ),
    post(
        "/wp-admin/edit-tags.php",
        "action=add-tag&taxonomy=category&tag-name=x",
    ),
    post("/wp-admin/user-new.php", "action=createuser&user_login=x"),
    post(
        "/wp-admin/users.php",
        "action=delete&users[]={user:subscriber}",
    ),
    post("/wp-admin/profile.php", "action=update&user_id={me}"),
    post("/wp-admin/user-edit.php", "action=update&user_id=1"),
    post(
        "/wp-admin/comment.php",
        "action=editedcomment&comment_ID={comment}",
    ),
    post(
        "/wp-admin/themes.php?action=activate&stylesheet=twentytwentyfour",
        "",
    ),
    post("/wp-admin/plugins.php?action=activate&plugin=hello.php", ""),
    post("/wp-admin/async-upload.php", "action=upload-attachment"),
    // admin-ajax.php.
    get("/wp-admin/admin-ajax.php"),
    get("/wp-admin/admin-ajax.php?action=rest-nonce-of-nobody"),
    get("/wp-admin/admin-ajax.php?action=logged-in"),
    get("/wp-admin/admin-ajax.php?action=ajax-tag-search&tax=post_tag&q=te"),
    get("/wp-admin/admin-ajax.php?action=ajax-tag-search&tax=category&q=te"),
    get("/wp-admin/admin-ajax.php?action=wp-compression-test&test=no"),
    post("/wp-admin/admin-ajax.php", "action=query-attachments"),
    post("/wp-admin/admin-ajax.php", "action=heartbeat"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=add-tag&taxonomy=category&tag-name=x",
    ),
    post("/wp-admin/admin-ajax.php", "action=delete-post&id={draft}"),
    post("/wp-admin/admin-ajax.php", "action=trash-post&id={draft}"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=delete-comment&id={comment}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=dim-comment&id={comment}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=replyto-comment&comment_post_ID={published}&content=x",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=edit-comment&comment_ID={comment}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=inline-save&post_ID={draft}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=inline-save-tax&tax_ID=1&taxonomy=category",
    ),
    post("/wp-admin/admin-ajax.php", "action=add-user&user_login=x"),
    post("/wp-admin/admin-ajax.php", "action=save-attachment&id=1"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=send-attachment-to-editor",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=get-permalink&post_id={draft}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=sample-permalink&post_id={draft}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=wp-remove-post-lock&post_ID={draft}",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=closed-postboxes&page=post",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=hidden-columns&page=edit-post",
    ),
    post("/wp-admin/admin-ajax.php", "action=install-plugin&slug=x"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=delete-plugin&slug=hello&plugin=hello.php",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=update-plugin&slug=hello&plugin=hello.php",
    ),
    post("/wp-admin/admin-ajax.php", "action=install-theme&slug=x"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=delete-theme&slug=twentytwentyfour",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=wp-privacy-export-personal-data&id=1",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=wp-privacy-erase-personal-data&id=1",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=health-check-site-status-result",
    ),
    post("/wp-admin/admin-ajax.php", "action=generate-password"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=destroy-sessions&user_id=1",
    ),
    post("/wp-admin/admin-ajax.php", "action=save-widget"),
    post("/wp-admin/admin-ajax.php", "action=widgets-order"),
    post("/wp-admin/admin-ajax.php", "action=add-menu-item"),
    post("/wp-admin/admin-ajax.php", "action=menu-quick-search&q=te"),
    post("/wp-admin/admin-ajax.php", "action=customize_save"),
    post("/wp-admin/admin-ajax.php", "action=image-editor&postid=1"),
    post(
        "/wp-admin/admin-ajax.php",
        "action=set-post-thumbnail&post_id={draft}&thumbnail_id=1",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=parse-embed&post_ID={draft}&shortcode=x",
    ),
    post(
        "/wp-admin/admin-ajax.php",
        "action=oembed-cache&post={draft}",
    ),
    post("/wp-admin/admin-ajax.php", "action=edit-theme-plugin-file"),
    post("/wp-admin/admin-ajax.php", "action=toggle-auto-updates"),
];

async fn app() -> Option<std::sync::Arc<App>> {
    let root =
        PathBuf::from(std::env::var("RUSTPRESS_WP_ROOT").unwrap_or_else(|_| DEFAULT_ROOT.into()));
    if !root.join("wp-includes").is_dir() || reference("GET", "/", "", &[], b"").is_err() {
        eprintln!("reference stack not available, skipping");
        assert!(
            std::env::var_os("RUSTPRESS_REQUIRE_REFERENCE").is_none(),
            "RUSTPRESS_REQUIRE_REFERENCE is set, so this test must not skip itself"
        );
        return None;
    }
    let config = Config {
        database_url: std::env::var("RUSTPRESS_TEST_DB")
            .unwrap_or_else(|_| "mysql://wordpress:wordpress@127.0.0.1:3306/wordpress".into()),
        wp_root: root,
        allow_writes: true,
        // WordPress' own texts, as the reference shows them.
        branding: false,
        ..Config::default()
    };
    App::new(config).await.ok()
}

/// The matrix itself; returns the differences and the IDs of the posts and pages the cases created.
async fn compare(
    app: &std::sync::Arc<App>,
    sessions: &[Session],
    fixture: &Fixture,
) -> (Vec<String>, usize, Vec<(String, u64)>) {
    let only = std::env::var("RUSTPRESS_ONLY").ok();
    let (mut differences, mut compared, mut created) = (Vec::new(), 0, Vec::new());
    for session in sessions {
        let me = fixture.user(session.role);
        let fill = |text: &str| {
            let mut text = text
                .replace("{draft}", &fixture.draft.to_string())
                .replace("{published}", &fixture.published.to_string())
                .replace("{authors}", &fixture.authors.to_string());
            text = text
                .replace("{contributors}", &fixture.contributors.to_string())
                .replace("{page}", &fixture.page.to_string())
                .replace("{comment}", &fixture.comment.to_string());
            text = text.replace(
                "{me}",
                &if session.role == "visitor" {
                    "0".to_owned()
                } else {
                    me.to_string()
                },
            );
            for (role, id) in &fixture.users {
                text = text.replace(&format!("{{user:{role}}}"), &id.to_string());
            }
            text
        };
        for case in CASES {
            let (path, body) = (fill(case.path), fill(case.body));
            let line = format!("{} {} {path} {body}", session.role, case.method);
            if only
                .as_ref()
                .is_some_and(|only| !line.contains(only.as_str()))
            {
                continue;
            }
            let rest = path.starts_with("/wp-json/");
            let mut extra: Vec<(&str, &str)> = Vec::new();
            if rest && !session.nonce.is_empty() {
                extra.push(("X-WP-Nonce", &session.nonce));
            }
            if !body.is_empty() {
                extra.push((
                    "Content-Type",
                    if rest {
                        "application/json"
                    } else {
                        "application/x-www-form-urlencoded"
                    },
                ));
            }
            let theirs = reference(
                case.method,
                &path,
                &session.cookies,
                &extra,
                body.as_bytes(),
            )
            .unwrap();
            let mine = ours(
                app,
                case.method,
                &path,
                &session.cookies,
                &extra,
                body.as_bytes(),
            )
            .await;
            for answer in [&theirs, &mine] {
                if let (true, 201, Some(id)) = (
                    rest,
                    answer.status,
                    answer.json().get("id").and_then(Value::as_u64),
                ) {
                    created.push((path.clone(), id));
                }
            }
            // Without a connection to WordPress.org the reference cannot answer what the directories hold.
            if theirs
                .json()
                .get("code")
                .and_then(Value::as_str)
                .is_some_and(|code| code.ends_with("_api_failed"))
            {
                continue;
            }
            compared += 1;
            let (theirs, mine) = (theirs.answer(rest), mine.answer(rest));
            if only.is_some() {
                eprintln!(
                    "{line}\n  WordPress {} {}\n  RustPress {} {}",
                    theirs.status, theirs.detail, mine.status, mine.detail
                );
            }
            if theirs != mine {
                match KNOWN.iter().find(|(part, _)| line.contains(part)) {
                    Some((_, reason)) => eprintln!("known difference: {line}: {reason}"),
                    None => differences.push(format!(
                        "{line}\n  WordPress {} {}\n  RustPress {} {}",
                        theirs.status, theirs.detail, mine.status, mine.detail
                    )),
                }
            }
        }
    }
    (differences, compared, created)
}

#[test]
fn every_role_gets_the_answer_of_wordpress() {
    // The screens of the editor recurse deeply; the stack of a test thread is too small for them.
    let run = || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(matrix())
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

async fn matrix() {
    let Some(_) = app().await else { return };
    let admin = Session::sign_in(
        "administrator",
        "admin",
        &std::env::var("RUSTPRESS_TEST_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".into()),
    );
    remove_leftovers(&admin);
    let fixture = fixture(&admin);
    let mut sessions = vec![Session::visitor()];
    sessions.extend(
        ROLES
            .iter()
            .rev()
            .map(|role| Session::sign_in(role, &format!("rp-role-{role}"), PASSWORD)),
    );
    sessions.push(Session::sign_in(
        "administrator",
        "admin",
        &std::env::var("RUSTPRESS_TEST_ADMIN_PASSWORD").unwrap_or_else(|_| "admin".into()),
    ));
    // RustPress reads the users and posts of the fixture when it starts.
    let app = app().await.expect("the reference was there a moment ago");
    let (differences, compared, created) = compare(&app, &sessions, &fixture).await;

    for (path, id) in created {
        admin.rest(
            "DELETE",
            &format!("{}/{id}?force=true", path.trim_start_matches("/wp-json")),
            &Value::Null,
        );
    }
    // `post-new.php` leaves an auto-draft behind on each side.
    for kind in ["posts", "pages"] {
        let drafts = admin
            .rest(
                "GET",
                &format!("/wp/v2/{kind}?context=edit&status=auto-draft&per_page=100"),
                &Value::Null,
            )
            .json();
        for id in drafts
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|post| post.get("id").and_then(Value::as_u64))
            .filter(|id| *id > fixture.draft)
        {
            admin.rest(
                "DELETE",
                &format!("/wp/v2/{kind}/{id}?force=true"),
                &Value::Null,
            );
        }
    }
    admin.rest(
        "DELETE",
        &format!("/wp/v2/comments/{}?force=true", fixture.comment),
        &Value::Null,
    );
    for id in [
        fixture.draft,
        fixture.published,
        fixture.authors,
        fixture.contributors,
    ] {
        admin.rest(
            "DELETE",
            &format!("/wp/v2/posts/{id}?force=true"),
            &Value::Null,
        );
    }
    admin.rest(
        "DELETE",
        &format!("/wp/v2/pages/{}?force=true", fixture.page),
        &Value::Null,
    );
    for (_, id) in &fixture.users {
        admin.rest(
            "DELETE",
            &format!("/wp/v2/users/{id}?force=true&reassign=1"),
            &Value::Null,
        );
    }
    remove_leftovers(&admin);

    eprintln!(
        "{compared} answers compared, {} different",
        differences.len()
    );
    assert!(
        differences.is_empty(),
        "{} of {compared} answers differ from WordPress:\n{}",
        differences.len(),
        differences.join("\n")
    );
}
