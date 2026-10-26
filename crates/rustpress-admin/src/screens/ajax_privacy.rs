//! The Ajax actions of the privacy tools: `wp-privacy-export-personal-data` and `wp-privacy-erase-personal-data`.
//! The exporters and the eraser of WordPress (user, comments, media) run page by page; the last page of the last
//! exporter writes the report (HTML and JSON) into a ZIP in `uploads/wp-personal-data-exports`. RustPress cannot
//! send e-mail, so the export is only offered as a download.

use rustpress_core::OrLog;
use rustpress_auth::CurrentUser;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::slug::sanitize_title;
use rustpress_db::{CommentListQuery, Post, PostQuery};
use rustpress_phpser::{Key, Value};
use serde_json::{Map, Value as Json, json};

use super::ajax::{json, php_json, text};
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use crate::content::PostData;
use crate::sanitize::is_email;

/// One item of an exporter: `group_id`, `group_label`, `group_description`, `item_id` and the name/value pairs.
struct Datum {
    group_id: String,
    group_label: String,
    group_description: String,
    item_id: String,
    data: Vec<(String, String)>,
}

impl Datum {
    fn to_json(&self) -> Json {
        json!({
            "group_id": self.group_id,
            "group_label": self.group_label,
            "group_description": self.group_description,
            "item_id": self.item_id,
            "data": self.data.iter().map(|(name, value)| json!({"name": name, "value": value})).collect::<Vec<_>>(),
        })
    }

    fn to_php(&self) -> Value {
        let s = |v: &str| Value::Str(v.as_bytes().to_vec());
        let key = |k: &str| Key::Str(k.as_bytes().to_vec());
        let pairs = self.data.iter().enumerate().map(|(i, (name, value))| (Key::Int(i as i64), Value::Array(vec![(key("name"), s(name)), (key("value"), s(value))]))).collect();
        Value::Array(vec![
            (key("group_id"), s(&self.group_id)),
            (key("group_label"), s(&self.group_label)),
            (key("group_description"), s(&self.group_description)),
            (key("item_id"), s(&self.item_id)),
            (key("data"), Value::Array(pairs)),
        ])
    }

    fn from_php(value: &Value) -> Option<Self> {
        let Value::Array(entries) = value else { return None };
        let get = |name: &str| entries.iter().find(|(k, _)| matches!(k, Key::Str(s) if s == name.as_bytes())).map(|(_, v)| v);
        let string = |name: &str| match get(name) {
            Some(Value::Str(s)) => String::from_utf8_lossy(s).into_owned(),
            _ => String::new(),
        };
        let mut data = Vec::new();
        if let Some(Value::Array(pairs)) = get("data") {
            for (_, pair) in pairs {
                if let Value::Array(fields) = pair {
                    let field = |name: &str| fields.iter().find(|(k, _)| matches!(k, Key::Str(s) if s == name.as_bytes())).map(|(_, v)| if let Value::Str(s) = v { String::from_utf8_lossy(s).into_owned() } else { String::new() }).unwrap_or_default();
                    data.push((field("name"), field("value")));
                }
            }
        }
        Some(Self { group_id: string("group_id"), group_label: string("group_label"), group_description: string("group_description"), item_id: string("item_id"), data })
    }
}

fn error(message: &str) -> AdminResponse {
    json(200, &json!({"success": false, "data": message}))
}

/// `empty()` of PHP for a string.
fn empty(value: &str) -> bool {
    value.is_empty() || value == "0"
}

/// A request of the `user_request` type with the action it is for (`wp_get_user_request()`).
async fn user_request(env: &AdminEnv<'_>, id: u64) -> Option<Post> {
    env.db.post(id).await.ok().flatten().filter(|p| p.post_type == "user_request")
}

/// `wp_privacy_anonymize_ip()`.
fn anonymize_ip(ip: &str) -> String {
    if ip.is_empty() {
        return "0.0.0.0".into();
    }
    let colons = ip.matches(':').count();
    let dots = ip.matches('.').count();
    let (mut prefix, mut ip, mut v6) = (String::new(), ip.to_owned(), colons > 1);
    let v4 = dots == 3;
    if v6 && v4 {
        prefix = "::ffff:".into();
        ip = ip.rsplit_once(':').map(|(_, rest)| rest.to_owned()).unwrap_or(ip).replace(']', "");
        v6 = false;
    }
    if v6 {
        let (left, right) = (ip.find('['), ip.find(']'));
        match (left, right) {
            (Some(l), Some(r)) if r > l => ip = ip[l + 1..r].to_owned(),
            (None, None) => {}
            _ => return "::".into(),
        }
        if let Some(percent) = ip.find('%') {
            ip.truncate(percent);
        }
        if ip.chars().any(|c| !(c.is_ascii_hexdigit() || c == ':')) {
            return "::".into();
        }
        return match ip.parse::<std::net::Ipv6Addr>() {
            Ok(addr) => {
                let mut segments = addr.segments();
                for s in &mut segments[4..] {
                    *s = 0;
                }
                format!("{prefix}{}", std::net::Ipv6Addr::from(segments))
            }
            Err(_) => "::".into(),
        };
    }
    if v4 || !prefix.is_empty() {
        let cut = ip.rfind('.').unwrap_or(ip.len());
        return format!("{prefix}{}.0", &ip[..cut]);
    }
    "0.0.0.0".into()
}

/// The exporters in the order of `wp_privacy_personal_data_exporters`: user, comments, media.
const EXPORTERS: [&str; 3] = ["wordpress-user", "wordpress-comments", "wordpress-media"];

async fn export_user(ctx: &Ctx<'_>, email: &str) -> (Vec<Datum>, bool) {
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let Some(user) = env.db.user_by_email(email.trim()).await.ok().flatten() else { return (Vec::new(), true) };
    let meta = |key: &'static str| async move { env.db.user_meta(user.id, key).await.ok().flatten().unwrap_or_default() };
    let props = [
        (tr("User ID"), user.id.to_string()),
        (tr("User Login Name"), user.user_login.clone()),
        (tr("User Nice Name"), user.user_nicename.clone()),
        (tr("User Email"), user.user_email.clone()),
        (tr("User URL"), user.user_url.clone()),
        (tr("User Registration Date"), user.user_registered.clone()),
        (tr("User Display Name"), user.display_name.clone()),
        (tr("User Nickname"), meta("nickname").await),
        (tr("User First Name"), meta("first_name").await),
        (tr("User Last Name"), meta("last_name").await),
        (tr("User Description"), meta("description").await),
    ];
    let user_data: Vec<(String, String)> = props.into_iter().filter(|(_, v)| !empty(v)).collect();
    let mut out = vec![Datum {
        group_id: "user".into(),
        group_label: tr("User"),
        group_description: tr("User&#8217;s profile data."),
        item_id: format!("user-{}", user.id),
        data: user_data,
    }];
    if let Some(raw) = env.db.user_meta(user.id, "community-events-location").await.ok().flatten()
        && let Ok(Value::Array(location)) = rustpress_phpser::unserialize(raw.as_bytes())
    {
        let field = |name: &str| location.iter().find(|(k, _)| matches!(k, Key::Str(s) if s == name.as_bytes())).map(|(_, v)| match v {
            Value::Str(s) => String::from_utf8_lossy(s).into_owned(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            _ => String::new(),
        });
        let data = [("description", tr("City")), ("country", tr("Country")), ("latitude", tr("Latitude")), ("longitude", tr("Longitude")), ("ip", tr("IP"))]
            .into_iter()
            .filter_map(|(key, name)| field(key).filter(|v| !empty(v)).map(|v| (name, v)))
            .collect();
        out.push(Datum {
            group_id: "community-events-location".into(),
            group_label: tr("Community Events Location"),
            group_description: tr("User&#8217;s location data used for the Community Events in the WordPress Events and News dashboard widget."),
            item_id: format!("community-events-location-{}", user.id),
            data,
        });
    }
    if let Some(raw) = env.db.user_meta(user.id, "session_tokens").await.ok().flatten()
        && let Ok(Value::Array(tokens)) = rustpress_phpser::unserialize(raw.as_bytes())
    {
        for (token_key, session) in &tokens {
            let Value::Array(session) = session else { continue };
            let field = |name: &str| session.iter().find(|(k, _)| matches!(k, Key::Str(s) if s == name.as_bytes())).map(|(_, v)| v);
            let mut data = Vec::new();
            for (key, name) in [("expiration", tr("Expiration")), ("ip", tr("IP")), ("ua", tr("User Agent")), ("login", tr("Last Login"))] {
                let value = match field(key) {
                    Some(Value::Int(t)) if matches!(key, "expiration" | "login") && *t != 0 => ctx.site().date("F d, Y H:i A", *t),
                    Some(Value::Int(t)) if *t != 0 => t.to_string(),
                    Some(Value::Str(s)) if !s.is_empty() => String::from_utf8_lossy(s).into_owned(),
                    _ => continue,
                };
                data.push((name, value));
            }
            let token_key = match token_key {
                Key::Int(i) => i.to_string(),
                Key::Str(s) => String::from_utf8_lossy(s).into_owned(),
            };
            out.push(Datum {
                group_id: "session-tokens".into(),
                group_label: tr("Session Tokens"),
                group_description: tr("User&#8217;s Session Tokens data."),
                item_id: format!("session-tokens-{}-{token_key}", user.id),
                data,
            });
        }
    }
    (out, true)
}

async fn export_comments(ctx: &Ctx<'_>, email: &str, page: u64) -> (Vec<Datum>, bool) {
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let number = 500;
    let query = CommentListQuery { status: "all".into(), author_email: Some(email.to_owned()), orderby: "comment_ID".into(), desc: false, limit: number, offset: (page.max(1) - 1) * number, ..Default::default() };
    let comments = env.db.list_comments_admin(&query).await.map(|(c, _)| c).unwrap_or_default();
    let mut out = Vec::new();
    for comment in &comments {
        let (ip, agent) = env.db.comment_extra(comment.comment_id).await.ok().flatten().unwrap_or_default();
        let link = super::comments::comment_link(ctx, comment).await;
        let link_html = if link.is_empty() { String::new() } else { format!("<a href=\"{}\" target=\"_blank\">{}</a>", esc_url(&link), esc_html(&link)) };
        let props = [
            (tr("Comment Author"), comment.comment_author.clone()),
            (tr("Comment Author Email"), comment.comment_author_email.clone()),
            (tr("Comment Author URL"), comment.comment_author_url.clone()),
            (tr("Comment Author IP"), ip),
            (tr("Comment Author User Agent"), agent),
            (tr("Comment Date"), comment.comment_date.clone()),
            (tr("Comment Content"), comment.comment_content.clone()),
            (tr("Comment URL"), link_html),
        ];
        out.push(Datum {
            group_id: "comments".into(),
            group_label: tr("Comments"),
            group_description: tr("User&#8217;s comment data."),
            item_id: format!("comment-{}", comment.comment_id),
            data: props.into_iter().filter(|(_, v)| !empty(v)).collect(),
        });
    }
    (out, (comments.len() as u64) < number)
}

async fn export_media(ctx: &Ctx<'_>, email: &str, page: u64) -> (Vec<Datum>, bool) {
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let Some(user) = env.db.user_by_email(email).await.ok().flatten() else { return (Vec::new(), true) };
    let number = 50;
    let statuses = ["publish", "future", "draft", "pending", "private", "inherit"].map(String::from).to_vec();
    let query = PostQuery { post_types: vec!["attachment".into()], statuses, author: Some(user.id), order_by: rustpress_db::OrderBy::Id, order: rustpress_db::Order::Asc, ..Default::default() };
    let all = env.db.posts(&query).await.unwrap_or_default();
    let total = all.len() as u64;
    let posts: Vec<&Post> = all.iter().skip(((page.max(1) - 1) * number) as usize).take(number as usize).collect();
    let mut out = Vec::new();
    for post in posts {
        let Some(file) = env.db.post_meta_value(post.id, "_wp_attached_file").await.ok().flatten().filter(|f| !f.is_empty()) else { continue };
        out.push(Datum {
            group_id: "media".into(),
            group_label: tr("Media"),
            group_description: tr("User&#8217;s media data."),
            item_id: format!("post-{}", post.id),
            data: vec![(tr("URL"), rustpress_blocks::images::attachment_url(ctx.site(), &file))],
        });
    }
    let max_pages = total.div_ceil(number).max(1);
    (out, max_pages <= page)
}

/// `wp_ajax_wp_privacy_export_personal_data()`.
pub async fn export(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let tr = |s: &str| ctx.tr(s);
    let post = |name: &str| request.post_field(name);
    if post("id").is_none_or(str::is_empty) {
        return error(&tr("Missing request ID."));
    }
    let request_id = post("id").unwrap_or_default().parse::<i64>().unwrap_or(0);
    if request_id < 1 {
        return error(&tr("Invalid request ID."));
    }
    if !user.can("export_others_personal_data") {
        return error(&tr("Sorry, you are not allowed to perform this action."));
    }
    let nonce = request.request("security").unwrap_or_default();
    if env.auth.verify_nonce(nonce, &format!("wp-privacy-export-personal-data-{request_id}"), user.id, token, env.now).is_none() {
        return text(403, "-1");
    }
    let request_id = request_id as u64;
    let Some(req) = user_request(env, request_id).await.filter(|r| r.post_name == "export_personal_data") else { return error(&tr("Invalid request type.")) };
    let email = req.post_title.clone();
    if !is_email(&email) {
        return error(&tr("A valid email address must be given."));
    }
    let Some(exporter) = post("exporter") else { return error(&tr("Missing exporter index.")) };
    let exporter: i64 = exporter.parse().unwrap_or(0);
    let Some(page) = post("page") else { return error(&tr("Missing page index.")) };
    let page: i64 = page.parse().unwrap_or(0);
    let send_as_email = post("sendAsEmail") == Some("true");
    if exporter < 1 {
        return error(&tr("Exporter index cannot be negative."));
    }
    if exporter as usize > EXPORTERS.len() {
        return error(&tr("Exporter index is out of range."));
    }
    if page < 1 {
        return error(&tr("Page index cannot be less than one."));
    }
    let (data, done) = match EXPORTERS[exporter as usize - 1] {
        "wordpress-user" => export_user(&ctx, &email).await,
        "wordpress-comments" => export_comments(&ctx, &email, page as u64).await,
        _ => export_media(&ctx, &email, page as u64).await,
    };
    let mut response = Map::new();
    response.insert("data".into(), Json::Array(data.iter().map(Datum::to_json).collect()));
    response.insert("done".into(), Json::Bool(done));

    // `wp_privacy_process_personal_data_export_page()`: collect the pages; the last one writes the file.
    let Some(writer) = env.writer else { return error(&tr("Sorry, you are not allowed to perform this action.")) };
    let mut collected: Vec<Datum> = Vec::new();
    if !(exporter == 1 && page == 1)
        && let Some(raw) = env.db.post_meta_value(request_id, "_export_data_raw").await.ok().flatten()
        && let Ok(Value::Array(items)) = rustpress_phpser::unserialize(raw.as_bytes())
    {
        collected = items.iter().filter_map(|(_, v)| Datum::from_php(v)).collect();
    }
    collected.extend(data);
    let store = |items: &[Datum]| Value::Array(items.iter().enumerate().map(|(i, d)| (Key::Int(i as i64), d.to_php())).collect());
    if let Err(e) = writer.update_post_meta(request_id, "_export_data_raw", &String::from_utf8_lossy(&rustpress_phpser::serialize(&store(&collected)))).await {
        return json(500, &json!({"success": false, "data": e.to_string()}));
    }
    if exporter as usize != EXPORTERS.len() || !done {
        return json(200, &json!({"success": true, "data": Json::Object(response)}));
    }
    writer.delete_post_meta(request_id, "_export_data_raw").await.or_log();

    // The groups and their items, the first data of an item last (`array_merge( $new, $old )`).
    let mut groups: Vec<(String, String, String, Vec<(String, Vec<(String, String)>)>)> = Vec::new();
    for datum in collected {
        let group = match groups.iter().position(|g| g.0 == datum.group_id) {
            Some(i) => i,
            None => {
                groups.push((datum.group_id.clone(), datum.group_label.clone(), datum.group_description.clone(), Vec::new()));
                groups.len() - 1
            }
        };
        let items = &mut groups[group].3;
        match items.iter().position(|i| i.0 == datum.item_id) {
            Some(i) => {
                let mut merged = datum.data;
                merged.extend(std::mem::take(&mut items[i].1));
                items[i].1 = merged;
            }
            None => items.push((datum.item_id, datum.data)),
        }
    }
    match generate_file(&ctx, request_id, &email, &groups).await {
        Ok(name) => {
            let exports_url = format!("{}/wp-personal-data-exports/", rustpress_blocks::images::upload_baseurl(ctx.site()));
            if send_as_email {
                // The link goes by e-mail, and the request is completed once the e-mail is sent.
                if let Err(message) = env.mail_ctx().export_email(request_id, &exports_url).await {
                    return error(&message);
                }
                writer.update_post_meta(request_id, "_wp_user_request_completed_timestamp", &env.now.to_string()).await.or_log();
                let system = rustpress_auth::CurrentUser::system();
                let content = crate::content::Content { site: env.site, db: env.db, writer, user: &system, now: env.now };
                content.insert_post(&PostData { id: Some(request_id), post_status: Some("request-completed".into()), ..Default::default() }).await.or_log();
                return json(200, &json!({"success": true, "data": Json::Object(response)}));
            }
            let url = format!("{exports_url}{name}");
            response.insert("url".into(), Json::String(url));
            json(200, &json!({"success": true, "data": Json::Object(response)}))
        }
        Err(message) => error(&message),
    }
}

type Groups = [(String, String, String, Vec<(String, Vec<(String, String)>)>)];

/// `wp_privacy_generate_personal_data_export_group_html()`.
fn group_html(ctx: &Ctx<'_>, id: &str, label: &str, description: &str, items: &[(String, Vec<(String, String)>)], groups_count: usize) -> String {
    let attr = sanitize_title(&format!("{label}-{id}"));
    let mut html = format!("<h2 id=\"{}\">{}", esc_attr(&attr), esc_html(label));
    if items.len() > 1 {
        html.push_str(&format!(" <span class=\"count\">({})</span>", items.len()));
    }
    html.push_str("</h2>");
    if !description.is_empty() {
        html.push_str(&format!("<p>{}</p>", esc_html(description)));
    }
    html.push_str("<div>");
    for (_, data) in items {
        html.push_str("<table><tbody>");
        for (name, value) in data {
            let value = if !value.contains(' ') && (value.starts_with("http://") || value.starts_with("https://")) { format!("<a href=\"{}\">{}</a>", esc_url(value), esc_html(value)) } else { value.clone() };
            html.push_str(&format!("<tr><th>{}</th><td>{}</td></tr>", esc_html(name), rustpress_core::plugins::kses_data(&value)));
        }
        html.push_str("</tbody></table>");
    }
    if groups_count > 1 {
        html.push_str(&format!("<div class=\"return-to-top\"><a href=\"#top\"><span aria-hidden=\"true\">&uarr; </span> {}</a></div>", esc_html(&ctx.tr("Go to top"))));
    }
    html.push_str("</div>");
    html
}

/// `wp_privacy_generate_personal_data_export_file()`: returns the file name of the archive.
async fn generate_file(ctx: &Ctx<'_>, request_id: u64, email: &str, groups: &Groups) -> Result<String, String> {
    use std::io::Write;
    let env = ctx.env;
    let tr = |s: &str| ctx.tr(s);
    let dir = super::ajax_media::uploads_dir(env).join("wp-personal-data-exports");
    std::fs::create_dir_all(&dir).map_err(|_| tr("Unable to create personal data export folder."))?;
    let index = dir.join("index.php");
    if !index.exists() {
        std::fs::write(&index, "<?php\n// Silence is golden.\n").map_err(|_| tr("Unable to protect personal data export folder from browsing."))?;
    }
    let title = sprintf(&tr("Personal Data Export for %s"), &[email]);
    let site = ctx.site();
    let about = (
        "about".to_owned(),
        site.trx("About", "personal data group label"),
        site.trx("Overview of export report.", "personal data group description"),
        vec![(
            "about-1".to_owned(),
            vec![
                (site.trx("Report generated for", "email address"), email.to_owned()),
                (site.trx("For site", "website name"), site.options.string("blogname", "")),
                (site.trx("At URL", "website URL"), site.options.string("home", "")),
                (site.trx("On", "date/time"), site.date("Y-m-d H:i:s", env.now)),
            ],
        )],
    );
    let mut all = vec![about];
    all.extend(groups.iter().cloned());
    let count = all.len();

    // The JSON report.
    let mut object = Map::new();
    for (id, label, description, items) in &all {
        let mut item_map = Map::new();
        for (item_id, data) in items {
            item_map.insert(item_id.clone(), Json::Array(data.iter().map(|(n, v)| json!({"name": n, "value": v})).collect()));
        }
        object.insert(id.clone(), json!({"group_label": label, "group_description": description, "items": Json::Object(item_map)}));
    }
    let groups_json = php_json(&Json::Object(object));
    let json_report = format!("{{\"{title}\":{groups_json}}}");

    // The HTML report.
    let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n<meta http-equiv='Content-Type' content='text/html; charset=UTF-8' />\n<style>");
    html.push_str("body { color: black; font-family: Arial, sans-serif; font-size: 11pt; margin: 15px auto; width: 860px; }");
    html.push_str("table { background: #f0f0f0; border: 1px solid #ddd; margin-bottom: 20px; width: 100%; }");
    html.push_str("th { padding: 5px; text-align: left; width: 20%; }");
    html.push_str("td { padding: 5px; }");
    html.push_str("tr:nth-child(odd) { background-color: #fafafa; }");
    html.push_str(".return-to-top { text-align: right; }");
    html.push_str(&format!("</style><title>{}</title></head>\n<body>\n<h1 id=\"top\">{}</h1>", esc_html(&title), esc_html(&tr("Personal Data Export"))));
    if count > 1 {
        html.push_str(&format!("<div id=\"table_of_contents\"><h2>{}</h2><ul>", esc_html(&tr("Table of Contents"))));
        for (id, label, _, items) in &all {
            let mut shown = esc_html(label);
            if items.len() > 1 {
                shown.push_str(&format!(" <span class=\"count\">({})</span>", items.len()));
            }
            html.push_str(&format!("<li><a href=\"#{}\">{shown}</a></li>", esc_attr(&sanitize_title(&format!("{label}-{id}")))));
        }
        html.push_str("</ul></div>");
    }
    for (id, label, description, items) in &all {
        html.push_str(&group_html(ctx, id, label, description, items, count));
    }
    html.push_str("</body>\n</html>\n");

    // The archive keeps its name once it exists, so links sent before keep working.
    let writer = env.writer.ok_or_else(|| tr("Unable to open personal data export file (archive) for writing."))?;
    let archive = match env.db.post_meta_value(request_id, "_export_file_name").await.ok().flatten().filter(|n| !n.is_empty()) {
        Some(name) => name,
        None => {
            let name = format!("wp-personal-data-file-{}.zip", crate::sanitize::random_password(32, false));
            writer.update_post_meta(request_id, "_export_file_name", &name).await.or_log();
            name
        }
    };
    let path = dir.join(&archive);
    let _ = std::fs::remove_file(&path);
    let file = std::fs::File::create(&path).map_err(|_| tr("Unable to open personal data export file (archive) for writing."))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file("export.json", options).map_err(|_| tr("Unable to archive the personal data export file (JSON format)."))?;
    zip.write_all(json_report.as_bytes()).map_err(|_| tr("Unable to archive the personal data export file (JSON format)."))?;
    zip.start_file("index.html", options).map_err(|_| tr("Unable to archive the personal data export file (HTML format)."))?;
    zip.write_all(html.as_bytes()).map_err(|_| tr("Unable to archive the personal data export file (HTML format)."))?;
    zip.finish().map_err(|_| tr("Unable to open personal data export file (archive) for writing."))?;
    Ok(archive)
}

/// `wp_ajax_wp_privacy_erase_personal_data()`.
pub async fn erase(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    let tr = |s: &str| ctx.tr(s);
    let post = |name: &str| request.post_field(name);
    if post("id").is_none_or(str::is_empty) {
        return error(&tr("Missing request ID."));
    }
    let request_id = post("id").unwrap_or_default().parse::<i64>().unwrap_or(0);
    if request_id < 1 {
        return error(&tr("Invalid request ID."));
    }
    if !user.can("erase_others_personal_data") || !user.can("delete_users") {
        return error(&tr("Sorry, you are not allowed to perform this action."));
    }
    let nonce = request.request("security").unwrap_or_default();
    if env.auth.verify_nonce(nonce, &format!("wp-privacy-erase-personal-data-{request_id}"), user.id, token, env.now).is_none() {
        return text(403, "-1");
    }
    let request_id = request_id as u64;
    let Some(req) = user_request(env, request_id).await.filter(|r| r.post_name == "remove_personal_data") else { return error(&tr("Invalid request type.")) };
    let email = req.post_title.clone();
    if !is_email(&email) {
        return error(&tr("Invalid email address in request."));
    }
    let Some(eraser) = post("eraser") else { return error(&tr("Missing eraser index.")) };
    let eraser: i64 = eraser.parse().unwrap_or(0);
    let Some(page) = post("page") else { return error(&tr("Missing page index.")) };
    let page: i64 = page.parse().unwrap_or(0);
    if eraser < 1 {
        return error(&tr("Eraser index cannot be less than one."));
    }
    if eraser > 1 {
        return error(&tr("Eraser index is out of range."));
    }
    if page < 1 {
        return error(&tr("Page index cannot be less than one."));
    }
    // `wp_comments_personal_data_eraser()`.
    let number = 500;
    let query = CommentListQuery { status: "all".into(), author_email: Some(email.clone()), orderby: "comment_ID".into(), desc: false, limit: number, offset: (page as u64 - 1) * number, ..Default::default() };
    let comments = env.db.list_comments_admin(&query).await.map(|(c, _)| c).unwrap_or_default();
    let (mut removed, mut retained) = (false, false);
    if let Some(writer) = env.writer {
        for comment in &comments {
            let (ip, _) = env.db.comment_extra(comment.comment_id).await.ok().flatten().unwrap_or_default();
            let fields: Vec<(&str, rustpress_db::Val)> = vec![
                ("comment_agent", rustpress_db::Val::Str(String::new())),
                ("comment_author", rustpress_db::Val::Str(tr("Anonymous"))),
                ("comment_author_email", rustpress_db::Val::Str(String::new())),
                ("comment_author_IP", rustpress_db::Val::Str(anonymize_ip(&ip))),
                ("comment_author_url", rustpress_db::Val::Str(String::new())),
                ("user_id", rustpress_db::Val::Int(0)),
            ];
            match writer.update_rows("comments", "comment_ID", comment.comment_id, &fields).await {
                Ok(n) if n > 0 => removed = true,
                _ => retained = true,
            }
        }
    }
    let done = (comments.len() as u64) < number;
    // `wp_privacy_process_personal_data_erasure_page()`: the last page of the last eraser completes the request.
    if done && let Some(writer) = env.writer {
        let content = crate::content::Content { site: env.site, db: env.db, writer, user, now: env.now };
        writer.update_post_meta(request_id, "_wp_user_request_completed_timestamp", &env.now.to_string()).await.or_log();
        content.insert_post(&PostData { id: Some(request_id), post_status: Some("request-completed".into()), ..Default::default() }).await.or_log();
        // `wp_privacy_personal_data_erased`: the person is told that their data is gone.
        env.mail_ctx().erasure_fulfillment_notification(request_id).await;
    }
    json(200, &json!({"success": true, "data": {"items_removed": removed, "items_retained": retained, "messages": [], "done": done}}))
}
