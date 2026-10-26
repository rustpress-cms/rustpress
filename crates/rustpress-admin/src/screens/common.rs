//! Pieces the screens share: capability checks, settings notices and the drop-downs of terms and pages.

use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_db::{Order, OrderBy, PostQuery, Term};

use super::notices::{self, SettingsError};
use super::{AdminResponse, Ctx, die};

/// `wp_die()` with the 403 of a missing capability.
pub fn forbidden(ctx: &Ctx<'_>, message: &str) -> AdminResponse {
    die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<p>{}</p>", ctx.tr(message)))
}

/// `wp_die( $message )`: a plain message and the status 500 of a call without one.
pub fn die_plain(ctx: &Ctx<'_>, message: &str) -> AdminResponse {
    die(ctx.env, 500, "WordPress &rsaquo; Error", &ctx.tr(message))
}

/// `wp_die( $html )`: the message as it is, with the status 500 of a call without one.
pub fn die_plain_html(ctx: &Ctx<'_>, html: &str) -> AdminResponse {
    die(ctx.env, 500, "WordPress &rsaquo; Error", html)
}

/// `wp_die()` with the 403 and the heading of `You need a higher level of permission.`
pub fn forbidden_heading(ctx: &Ctx<'_>, message: &str) -> AdminResponse {
    die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", ctx.tr("You need a higher level of permission."), ctx.tr(message)))
}

/// `wp_die()` for a request without write access.
pub fn read_only(ctx: &Ctx<'_>) -> AdminResponse {
    die(ctx.env, 403, "WordPress &rsaquo; Error", "<p>RustPress was started without write access.</p>")
}

/// `checked( $a, $b )`, with the leading space of the markup WordPress prints.
pub fn checked(on: bool) -> &'static str {
    if on { " checked='checked'" } else { "" }
}

pub fn selected(on: bool) -> &'static str {
    if on { " selected='selected'" } else { "" }
}

/// `form_option( $name )`.
pub fn form_option(ctx: &Ctx<'_>, name: &str) -> String {
    esc_attr(&ctx.site().options.string(name, ""))
}

/// The notices `settings_errors()` prints after `options.php` saved something.
pub async fn settings_errors(ctx: &Ctx<'_>) -> String {
    if ctx.request.get("settings-updated").is_none() {
        return String::new();
    }
    let Ok(Some(raw)) = ctx.env.db.option("_transient_settings_errors").await else { return String::new() };
    let Ok(value) = rustpress_phpser::unserialize(raw.as_bytes()) else { return String::new() };
    let Some(items) = value.as_array() else { return String::new() };
    let errors: Vec<SettingsError> = items
        .iter()
        .map(|(_, e)| {
            let field = |n: &str| e.get(n).and_then(|v| v.as_str()).unwrap_or_default().to_owned();
            SettingsError { setting: field("setting"), code: field("code"), message: field("message"), kind: field("type") }
        })
        .collect();
    notices::render_settings_errors(&errors, false)
}

/// The submit button of a settings form (`submit_button()`).
pub fn submit(ctx: &Ctx<'_>) -> String {
    format!("<p class=\"submit\"><input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button button-primary\" value=\"{}\"  /></p>", esc_attr(&ctx.tr("Save Changes")))
}

/// The terms of a taxonomy as a tree: `(term, depth)` in the order of `wp_dropdown_categories()` (by name).
pub fn term_tree(terms: &[Term], exclude: &[u64]) -> Vec<(Term, usize)> {
    fn walk(terms: &[Term], parent: u64, level: usize, exclude: &[u64], out: &mut Vec<(Term, usize)>) {
        let mut children: Vec<&Term> = terms.iter().filter(|t| t.parent == parent && !exclude.contains(&t.term_id)).collect();
        children.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
        for term in children {
            out.push((term.clone(), level));
            walk(terms, term.term_id, level + 1, exclude, out);
        }
    }
    let mut out = Vec::new();
    // Terms whose parent is not in the list are shown at the top.
    let ids: Vec<u64> = terms.iter().map(|t| t.term_id).collect();
    let orphans: Vec<Term> = terms.iter().map(|t| if t.parent != 0 && !ids.contains(&t.parent) { Term { parent: 0, ..t.clone() } } else { t.clone() }).collect();
    walk(&orphans, 0, 0, exclude, &mut out);
    out
}

/// `wp_dropdown_categories( [ 'hide_empty' => 0, 'hierarchical' => 1, ... ] )` for an arbitrary taxonomy.
pub async fn dropdown_terms(ctx: &Ctx<'_>, taxonomy: &str, name: &str, id: &str, selected_id: u64, class: &str, none: Option<(&str, &str)>, exclude: &[u64]) -> String {
    let terms = ctx.env.db.terms(taxonomy).await.unwrap_or_default();
    let mut out = format!("<select  name='{name}' id='{id}' class='{class}'>\n");
    if let Some((value, label)) = none {
        out.push_str(&format!("\t<option value='{}'>{label}</option>\n", esc_attr(value)));
    }
    for (term, level) in term_tree(&terms, exclude) {
        let pad = "&nbsp;".repeat(level * 3);
        let sel = if term.term_id == selected_id { " selected=\"selected\"" } else { "" };
        out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\"{sel}>{pad}{}</option>\n", term.term_id, esc_html(&term.name)));
    }
    out.push_str("</select>\n");
    out
}

/// `wp_dropdown_pages()` for the published pages: `<select name id>` with the pages in a tree sorted by title.
pub async fn dropdown_pages(ctx: &Ctx<'_>, name: &str, id: &str, selected_id: u64, none: Option<&str>) -> String {
    let query = PostQuery { post_types: vec!["page".into()], statuses: vec!["publish".into()], order_by: OrderBy::Title, order: Order::Asc, ..Default::default() };
    let pages = ctx.env.db.posts(&query).await.unwrap_or_default();
    fn walk(pages: &[rustpress_db::Post], ids: &[u64], parent: u64, level: usize, selected: u64, out: &mut String) {
        for page in pages.iter().filter(|p| p.post_parent == parent || (parent == 0 && !ids.contains(&p.post_parent))) {
            if parent != 0 && page.post_parent != parent {
                continue;
            }
            let title = if page.post_title.is_empty() { format!("#{} (no title)", page.id) } else { page.post_title.clone() };
            let pad = "&nbsp;".repeat(level * 3);
            let sel = if page.id == selected { " selected=\"selected\"" } else { "" };
            out.push_str(&format!("\t<option class=\"level-{level}\" value=\"{}\"{sel}>{pad}{}</option>\n", page.id, esc_html(&title)));
            walk(pages, ids, page.id, level + 1, selected, out);
        }
    }
    let ids: Vec<u64> = pages.iter().map(|p| p.id).collect();
    let mut out = format!("<select name='{name}' id='{id}'>\n");
    if let Some(label) = none {
        out.push_str(&format!("\t<option value=\"0\">{label}</option>\n"));
    }
    walk(&pages, &ids, 0, 0, selected_id, &mut out);
    out.push_str("</select>\n");
    out
}

/// `wp_get_referer()`: the `_wp_http_referer` of the form when it is not this request and stays on the site
/// (`wp_validate_redirect()`).
pub fn wp_get_referer(ctx: &Ctx<'_>) -> Option<String> {
    // `wp_get_raw_referer()`: the form field, else the `Referer` header.
    let raw = ctx.request.request("_wp_http_referer").filter(|r| !r.is_empty()).or_else(|| ctx.request.referer.as_deref().filter(|r| !r.is_empty()))?;
    let uri = &ctx.request.request_uri;
    if raw == uri || raw == format!("{}{uri}", ctx.site().settings.home.trim_end_matches('/')) {
        return None;
    }
    Some(crate::login::validate_redirect(ctx.site(), raw, "")).filter(|r| !r.is_empty())
}

/// The list tables: a page past the last one goes to the last one
/// (`if ( $pagenum > $total_pages && $total_pages > 0 ) wp_redirect( add_query_arg( 'paged', $total_pages ) )`).
pub fn past_last_page(ctx: &Ctx<'_>, paged: u64, total_items: u64, per_page: u64) -> Option<AdminResponse> {
    let total_pages = total_items.div_ceil(per_page.max(1));
    (paged > total_pages && total_pages > 0).then(|| super::redirect(&rustpress_core::urls::add_query_arg(&ctx.request.request_uri, "paged", &total_pages.to_string())))
}

/// `get_all_user_settings()`: the `wp-settings-{id}` cookie, or the stored `user-settings` without one.
pub async fn user_settings(ctx: &Ctx<'_>) -> Vec<(String, String)> {
    let raw = match ctx.request.cookie(&format!("wp-settings-{}", ctx.user.id)) {
        Some(cookie) => cookie.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '=' | '&' | '_' | '-')).collect::<String>(),
        None => ctx.env.db.user_meta(ctx.user.id, &format!("{}user-settings", ctx.env.db.table_prefix())).await.ok().flatten().unwrap_or_default(),
    };
    // `parse_str()` only when there is a `=` after the first character.
    if raw.find('=').is_none_or(|i| i == 0) {
        return Vec::new();
    }
    raw.split('&').filter_map(|pair| pair.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned()))).filter(|(k, _)| !k.is_empty()).collect()
}

/// `get_user_setting( $name )`.
pub async fn user_setting(ctx: &Ctx<'_>, name: &str) -> Option<String> {
    user_settings(ctx).await.into_iter().find(|(k, _)| k == name).map(|(_, v)| v)
}

/// `set_user_setting( $name, $value )`: stored as the `user-settings` option with its time; the cookies follow with
/// the page (`wp_user_settings()`).
pub async fn set_user_setting(ctx: &Ctx<'_>, name: &str, value: &str) {
    let Some(writer) = ctx.env.writer else { return };
    let mut all = user_settings(ctx).await;
    match all.iter_mut().find(|(k, _)| k == name) {
        Some(entry) => entry.1 = value.to_owned(),
        None => all.push((name.to_owned(), value.to_owned())),
    }
    let clean = |s: &str| s.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')).collect::<String>();
    let settings = all.iter().map(|(k, v)| (clean(k), clean(v))).filter(|(k, _)| !k.is_empty()).map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&");
    let prefix = ctx.env.db.table_prefix();
    use rustpress_core::OrLog;
    writer.update_user_meta(ctx.user.id, &format!("{prefix}user-settings"), &settings).await.or_log();
    writer.update_user_meta(ctx.user.id, &format!("{prefix}user-settings-time"), &ctx.env.now.to_string()).await.or_log();
}

/// The view mode of the list tables (`get_user_setting( 'posts_list_mode', 'list' )`).
pub async fn list_mode(ctx: &Ctx<'_>) -> String {
    user_setting(ctx, "posts_list_mode").await.filter(|m| !m.is_empty()).unwrap_or_else(|| "list".to_owned())
}

/// `$mode` of the posts and comments lists: `?mode=` is stored for the user (`set_user_setting( 'posts_list_mode' )`),
/// else the stored one.
pub async fn request_list_mode(ctx: &Ctx<'_>) -> String {
    match ctx.request.request("mode").filter(|m| !m.is_empty()) {
        Some(requested) => {
            let mode = if requested == "excerpt" { "excerpt" } else { "list" };
            set_user_setting(ctx, "posts_list_mode", mode).await;
            mode.to_owned()
        }
        None => list_mode(ctx).await,
    }
}

/// `get_user_option( $option )`: the meta with the blog prefix first, then the plain one.
pub async fn user_option(ctx: &Ctx<'_>, name: &str) -> Option<String> {
    let db = ctx.env.db;
    if let Ok(Some(value)) = db.user_meta(ctx.user.id, &format!("{}{name}", db.table_prefix())).await {
        return Some(value);
    }
    db.user_meta(ctx.user.id, name).await.ok().flatten()
}

/// `get_hidden_meta_boxes( $screen )` of an edit screen (`post` base): the boxes the user hid, or the ones WordPress hides
/// by default.
pub async fn hidden_meta_boxes(ctx: &Ctx<'_>, screen_id: &str, post_type: &str) -> Vec<String> {
    let stored = user_option(ctx, &format!("metaboxhidden_{screen_id}")).await.and_then(|raw| rustpress_phpser::unserialize(raw.as_bytes()).ok()).filter(|v| v.as_array().is_some());
    match stored {
        Some(value) => value.as_array().unwrap_or_default().iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        None if matches!(post_type, "post" | "page" | "attachment") => {
            ["slugdiv", "trackbacksdiv", "postcustom", "postexcerpt", "commentstatusdiv", "commentsdiv", "authordiv", "revisionsdiv"].iter().map(|s| (*s).to_owned()).collect()
        }
        None => vec!["slugdiv".to_owned()],
    }
}

/// `wp_is_mobile()` from the user agent of the request.
pub fn wp_is_mobile(ctx: &Ctx<'_>) -> bool {
    let agent = ctx.request.user_agent.as_deref().unwrap_or_default();
    ["Mobile", "Android", "Silk/", "Kindle", "BlackBerry", "Opera Mini", "Opera Mobi"].iter().any(|m| agent.contains(m))
}

/// `WP_Screen::get_columns()` of a screen with the `layout_columns` option: the user's choice, else the default.
pub async fn layout_columns(ctx: &Ctx<'_>, screen_id: &str, default: u32) -> u32 {
    user_option(ctx, &format!("screen_layout_{screen_id}")).await.and_then(|v| v.trim().parse::<u32>().ok()).filter(|n| *n != 0).unwrap_or(default)
}

/// `get_hidden_columns( $screen )`: the columns the user hid (`manage{screen}columnshidden`).
pub async fn hidden_columns(ctx: &Ctx<'_>, screen_id: &str) -> Vec<String> {
    let Some(raw) = user_option(ctx, &format!("manage{screen_id}columnshidden")).await else { return Vec::new() };
    match rustpress_phpser::unserialize(raw.as_bytes()) {
        Ok(rustpress_phpser::Value::Array(items)) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

/// The items per page of a list screen: the user's `$option` between 1 and 999, else `default`
/// (`render_per_page_options()`, `get_items_per_page()`).
pub async fn items_per_page(ctx: &Ctx<'_>, option: &str, default: u64) -> u64 {
    user_option(ctx, option).await.and_then(|v| v.trim().parse::<i64>().ok()).filter(|v| *v >= 1).map_or(default, |v| v as u64)
}

/// The "Screen Options" of a list table: the hideable columns (`render_list_table_columns_preferences()`) marked
/// hidden in `columns`, the items per page and, for the posts and comments lists, the view mode.
pub async fn list_screen_options(ctx: &Ctx<'_>, screen_id: &str, columns: &mut [crate::list_table::Column], per_page: (&str, u64), view_mode: Option<String>) -> crate::chrome::ScreenOptions {
    let hidden = hidden_columns(ctx, screen_id).await;
    const SPECIAL: [&str; 8] = ["_title", "cb", "comment", "media", "name", "title", "username", "blogname"];
    let mut toggles = Vec::new();
    for column in columns.iter_mut() {
        column.hidden = hidden.contains(&column.key);
        if SPECIAL.contains(&column.key.as_str()) || column.label.is_empty() {
            continue;
        }
        toggles.push((column.key.clone(), column.label.clone(), column.hidden));
    }
    crate::chrome::ScreenOptions { columns: toggles, per_page: Some((per_page.0.to_owned(), per_page.1)), view_mode, ..Default::default() }
}

/// `set_screen_options()`: a POST of the Screen Options stores "Number of items per page" for the user and goes
/// back to the list (without its page number). Options other than the known ones are left to filters, which
/// RustPress does not have, so they are not stored and the page runs as usual.
pub async fn set_screen_options(ctx: &Ctx<'_>) -> Option<AdminResponse> {
    let request = ctx.request;
    if !request.post || !request.form.iter().any(|(k, _)| k.starts_with("wp_screen_options[")) {
        return None;
    }
    let tr = |s: &str| ctx.tr(s);
    let nonce = request.request("screenoptionnonce").unwrap_or_default();
    if ctx.env.auth.verify_nonce(nonce, "screen-options-nonce", ctx.user.id, &ctx.token, ctx.env.now).is_none() {
        return Some(die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired.")));
    }
    let option = request.post_field("wp_screen_options[option]")?;
    let value = request.post_field("wp_screen_options[value]").unwrap_or_default();
    let sanitized: String = option.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-')).collect();
    if sanitized != option {
        return None;
    }
    let kind = option.replace("edit_", "").replace("_per_page", "");
    let types = &ctx.site().types;
    let map_option = if types.is_taxonomy(&kind) {
        "edit_tags_per_page"
    } else if types.is_post_type(&kind) {
        "edit_per_page"
    } else {
        option
    };
    const PER_PAGE: [&str; 8] = ["edit_per_page", "users_per_page", "edit_comments_per_page", "upload_per_page", "edit_tags_per_page", "plugins_per_page", "export_personal_data_requests_per_page", "remove_personal_data_requests_per_page"];
    if !PER_PAGE.contains(&map_option) {
        return None;
    }
    // `(int) $value` between 1 and 999.
    let digits: String = value.trim_start().chars().enumerate().take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && matches!(c, '-' | '+'))).map(|(_, c)| c).collect();
    let number = digits.parse::<i64>().unwrap_or(0);
    if !(1..=999).contains(&number) {
        return None;
    }
    if let Some(writer) = ctx.env.writer {
        use rustpress_core::OrLog;
        writer.update_user_meta(ctx.user.id, option, &number.to_string()).await.or_log();
    }
    let mut url = wp_get_referer(ctx).unwrap_or_else(|| request.request_uri.clone());
    for arg in ["pagenum", "apage", "paged"] {
        url = rustpress_core::urls::remove_query_arg(&url, arg);
    }
    if let Some(mode) = request.post_field("mode") {
        url = rustpress_core::urls::add_query_arg(&url, "mode", mode);
    }
    Some(super::redirect(&url))
}
