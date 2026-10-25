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
