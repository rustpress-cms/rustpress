//! The Ajax actions of the term screens (`add-tag`, `inline-save-tax`, `delete-tag`) and `WP_Ajax_Response`.

use rustpress_auth::CurrentUser;

use super::ajax::{headers, text};
use super::terms;
use super::{AdminEnv, AdminRequest, AdminResponse, Ctx};
use crate::content::{Content, TermData};

/// `WP_Ajax_Response`: the XML document the `wp-lists` script reads.
pub struct AjaxResponse {
    action: String,
    entries: Vec<String>,
}

impl AjaxResponse {
    pub fn new(action: &str) -> Self {
        Self { action: action.to_owned(), entries: Vec::new() }
    }

    /// `WP_Ajax_Response::add()`: `data` is the text, or an error `(code, message)`.
    pub fn add(&mut self, what: &str, id: u64, position: Option<i64>, data: Result<&str, (&str, &str)>, supplemental: &[(&str, String)]) {
        self.add_with_old(what, id, None, position, data, supplemental);
    }

    /// [`AjaxResponse::add`] with the `old_id` attribute of an entry that replaces another.
    pub fn add_with_old(&mut self, what: &str, id: u64, old_id: Option<u64>, position: Option<i64>, data: Result<&str, (&str, &str)>, supplemental: &[(&str, String)]) {
        let cdata = |text: &str| format!("<![CDATA[{}]]>", text.replace("]]>", "]]]]><![CDATA[>"));
        let old = old_id.map(|o| format!(" old_id='{o}'")).unwrap_or_default();
        let position = position.map(|p| format!(" position='{p}'")).unwrap_or_default();
        let mut x = format!("<response action='{}_{id}'><{what} id='{id}'{old}{position}>", self.action);
        match data {
            Ok(text) => x.push_str(&format!("<response_data>{}</response_data>", cdata(text))),
            Err((code, message)) => x.push_str(&format!("<wp_error code='{code}'>{}</wp_error>", cdata(message))),
        }
        x.push_str("<supplemental>");
        for (key, value) in supplemental {
            x.push_str(&format!("<{key}>{}</{key}>", cdata(value)));
        }
        x.push_str("</supplemental>");
        x.push_str(&format!("</{what}></response>"));
        self.entries.push(x);
    }

    pub fn send(self) -> AdminResponse {
        let body = format!("<?xml version='1.0' encoding='UTF-8' standalone='yes'?><wp_ajax>{}</wp_ajax>", self.entries.concat());
        AdminResponse { status: 200, headers: headers("text/xml; charset=UTF-8"), cookies: Vec::new(), body, bytes: Vec::new() }
    }
}

/// The depth of a term in its taxonomy.
fn depth(all: &[rustpress_db::Term], term: &rustpress_db::Term) -> usize {
    let mut level = 0;
    let mut parent = term.parent;
    while parent != 0 && level < 50 {
        match all.iter().find(|t| t.term_id == parent) {
            Some(p) => {
                level += 1;
                parent = p.parent;
            }
            None => break,
        }
    }
    level
}

/// `add-tag`, `inline-save-tax` and `delete-tag`.
pub async fn handle(env: &AdminEnv<'_>, request: &AdminRequest, user: &CurrentUser, token: &str, action: &str) -> AdminResponse {
    let ctx = Ctx { env, request, user: user.clone(), token: token.to_owned() };
    // `check_ajax_referer()` comes before everything else.
    let first_nonce = match action {
        "add-tag" => Some((request.request("_wpnonce_add-tag").or(request.request("_ajax_nonce-add-tag")).or(request.request("_ajax_nonce")).or(request.request("_wpnonce")).unwrap_or_default().to_owned(), "add-tag".to_owned())),
        "inline-save-tax" => Some((request.request("_inline_edit").or(request.request("_ajax_nonce")).or(request.request("_wpnonce")).unwrap_or_default().to_owned(), "taxinlineeditnonce".to_owned())),
        _ => {
            let id: i64 = request.request("tag_ID").and_then(|v| v.trim().parse().ok()).unwrap_or(0);
            Some((request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default().to_owned(), format!("delete-tag_{id}")))
        }
    };
    if let Some((value, nonce_action)) = first_nonce
        && env.auth.verify_nonce(&value, &nonce_action, user.id, token, env.now).is_none()
    {
        return text(403, "-1");
    }
    // `add-tag` and `delete-tag` take the tags if no taxonomy is named.
    let taxonomy = request.request("taxonomy").filter(|t| !t.is_empty()).map(str::to_owned).unwrap_or_else(|| if action == "inline-save-tax" { String::new() } else { "post_tag".to_owned() });
    let Some(info) = terms::tax_info(&ctx, &taxonomy, request.request("post_type")) else { return text(200, if action == "delete-tag" { "-1" } else { "0" }) };
    if !ctx.can("manage_categories") {
        return text(403, "-1");
    }
    let Some(writer) = env.writer else { return text(500, "0") };
    let content = Content { site: env.site, db: env.db, writer, user, now: env.now };
    let nonce_ok = |nonce: &str, nonce_action: &str| env.auth.verify_nonce(nonce, nonce_action, user.id, token, env.now).is_some();
    match action {
        "add-tag" => {
            if !nonce_ok(request.request("_wpnonce_add-tag").or(request.request("_ajax_nonce-add-tag")).or(request.request("_ajax_nonce")).unwrap_or_default(), "add-tag") {
                return text(403, "-1");
            }
            let data = TermData {
                name: request.post_field("tag-name").unwrap_or_default().to_owned(),
                slug: request.post_field("slug").unwrap_or_default().to_owned(),
                parent: request.post_field("parent").and_then(|p| p.parse::<i64>().ok()).filter(|p| *p > 0).unwrap_or(0) as u64,
                description: request.post_field("description").unwrap_or_default().to_owned(),
            };
            let mut x = AjaxResponse::new("add-tag");
            let created = match content.insert_term(&info.name, &data).await {
                Ok(id) => env.db.term_by_id(Some(&info.name), id).await.ok().flatten(),
                Err(e) => {
                    x.add("taxonomy", 0, None, Err(("error", &e.to_string())), &[]);
                    return x.send();
                }
            };
            let Some(term) = created else {
                x.add("taxonomy", 0, None, Err(("error", &env.site.tr("An error has occurred. Please reload the page and try again."))), &[]);
                return x.send();
            };
            let all = env.db.terms(&info.name).await.unwrap_or_default();
            let level = depth(&all, &term);
            let noparents = if info.hierarchical { terms::row_html(&ctx, &info.name, &term, level).await.unwrap_or_default() } else { String::new() };
            let parents = terms::row_html(&ctx, &info.name, &term, 0).await.unwrap_or_default();
            let message = terms::messages(&ctx, &info).swap_remove(1);
            x.add("taxonomy", 0, None, Ok(&message), &[("parents", parents), ("noparents", noparents), ("notice", message.clone())]);
            let fields = vec![
                ("term_id", term.term_id.to_string()),
                ("name", term.name.clone()),
                ("slug", term.slug.clone()),
                ("term_group", "0".to_owned()),
                ("term_taxonomy_id", term.term_taxonomy_id.to_string()),
                ("taxonomy", term.taxonomy.clone()),
                ("description", term.description.clone()),
                ("parent", term.parent.to_string()),
                ("count", term.count.to_string()),
                ("filter", "raw".to_owned()),
            ];
            x.add("term", 0, Some(level as i64), Ok(""), &fields);
            x.send()
        }
        "inline-save-tax" => {
            if !nonce_ok(request.request("_inline_edit").unwrap_or_default(), "taxinlineeditnonce") {
                return text(403, "-1");
            }
            let id: u64 = request.request("tax_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
            let Some(existing) = env.db.term_by_id(Some(&info.name), id).await.ok().flatten() else { return text(500, &env.site.tr("Invalid term ID.")) };
            let data = TermData {
                name: request.post_field("name").unwrap_or_default().to_owned(),
                slug: request.post_field("slug").unwrap_or_default().to_owned(),
                parent: existing.parent,
                description: existing.description.clone(),
            };
            if let Err(e) = content.update_term(id, &info.name, &data).await {
                return text(500, &e.to_string());
            }
            let Some(updated) = env.db.term_by_id(Some(&info.name), id).await.ok().flatten() else { return text(500, "0") };
            let all = env.db.terms(&info.name).await.unwrap_or_default();
            let level = depth(&all, &updated);
            text(200, &terms::row_html(&ctx, &info.name, &updated, level).await.unwrap_or_default())
        }
        _ => {
            let id: u64 = request.request("ID").or(request.request("tag_ID")).and_then(|v| v.parse().ok()).unwrap_or(0);
            if !nonce_ok(request.request("_ajax_nonce").or(request.request("_wpnonce")).unwrap_or_default(), &format!("delete-tag_{id}")) {
                return text(403, "-1");
            }
            match content.delete_term(id, &info.name).await {
                Ok(true) => text(200, "1"),
                _ => text(200, "0"),
            }
        }
    }
}
