//! `WP_Posts_List_Table::inline_edit()`: the hidden Quick Edit and Bulk Edit rows of the posts list.
//! The frame is the PHP template of WordPress turned into `inline_edit.tpl`; the author, term, parent and template
//! lists are made here.

use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_db::{Order, OrderBy, Post, PostQuery, User, UserListQuery};

use super::Ctx;
use super::posts::TypeInfo;

/// `touch_time( 1, 1, 0, 1 )`: the date fields without IDs and hidden copies.
fn touch_time(ctx: &Ctx<'_>, date: &str) -> String {
    let site = ctx.site();
    let part = |range: std::ops::Range<usize>| date.get(range).unwrap_or("00").to_owned();
    let (jj, mm, aa, hh, mn, ss) = (part(8..10), part(5..7), part(0..4), part(11..13), part(14..16), part(17..19));
    let locale = site.locale_data();
    let mut month = format!("<label><span class=\"screen-reader-text\">{}</span><select class=\"form-required\" name=\"mm\">\n", ctx.tr("Month"));
    for index in 1..=12 {
        let number = format!("{index:02}");
        let abbrev = &locale.month_abbrev[index - 1];
        month.push_str(&format!(
            "\t\t\t<option value=\"{number}\" data-text=\"{}\" {}>{}</option>\n",
            esc_attr(abbrev),
            if number == mm { " selected='selected'" } else { "" },
            rustpress_core::i18n::sprintf(&ctx.tr("%1$s-%2$s"), &[&number, abbrev])
        ));
    }
    month.push_str("</select></label>");
    let field = |label: &str, id: &str, value: &str, size: u8| {
        format!("<label><span class=\"screen-reader-text\">{label}</span><input type=\"text\" name=\"{id}\" value=\"{value}\" size=\"{size}\" maxlength=\"{size}\" autocomplete=\"off\" class=\"form-required\" inputmode=\"numeric\" /></label>")
    };
    let mut out = String::from("<div class=\"timestamp-wrap\">");
    out.push_str(&rustpress_core::i18n::sprintf(
        &ctx.tr("%1$s %2$s, %3$s at %4$s:%5$s"),
        &[&month, &field(&ctx.tr("Day"), "jj", &jj, 2), &field(&ctx.tr("Year"), "aa", &aa, 4), &field(&ctx.tr("Hour"), "hh", &hh, 2), &field(&ctx.tr("Minute"), "mn", &mn, 2)],
    ));
    out.push_str(&format!("</div><input type=\"hidden\" id=\"ss\" name=\"ss\" value=\"{ss}\" />"));
    out
}

/// The users `wp_dropdown_users()` lists as authors in Quick Edit and Bulk Edit (read once for both).
async fn author_users(ctx: &Ctx<'_>, info: &TypeInfo) -> Vec<User> {
    let roles: &[&str] = if info.name == "page" { &["administrator", "editor"] } else { &["administrator", "editor", "author", "contributor"] };
    let mut users: Vec<User> = Vec::new();
    for role in roles {
        let query = UserListQuery { role: Some((*role).to_owned()), orderby: "name".into(), ..Default::default() };
        for user in ctx.env.db.list_users(&query).await.map(|(users, _)| users).unwrap_or_default() {
            if !users.iter().any(|u| u.id == user.id) {
                users.push(user);
            }
        }
    }
    users.sort_by_key(|u| u.display_name.to_lowercase());
    users
}

/// `wp_dropdown_users()` of the Quick Edit authors (`show` = `display_name_with_login`).
fn authors(ctx: &Ctx<'_>, users: &[User], bulk: bool) -> String {
    let mut out = String::from("<select name='post_author' class='authors'>\n");
    if bulk {
        out.push_str(&format!("\t<option value='-1'>{}</option>\n", ctx.tr("&mdash; No Change &mdash;")));
    }
    // `wp_dropdown_users()` selects the author of an author archive: the list filtered by `author=`.
    let selected = ctx.request.get("author").and_then(|a| a.parse::<u64>().ok()).unwrap_or(0);
    for user in users {
        out.push_str(&format!("\t<option value='{}'{}>{} ({})</option>\n", user.id, if selected == user.id { " selected='selected'" } else { "" }, esc_html(&user.display_name), esc_html(&user.user_login)));
    }
    out.push_str("</select>");
    out
}

/// `wp_dropdown_pages()` for the parent of a page.
async fn parents(ctx: &Ctx<'_>, info: &TypeInfo, bulk: bool) -> String {
    let query = PostQuery { post_types: vec![info.name.clone()], statuses: vec!["publish".into()], limit: Some(100_000), order_by: OrderBy::MenuOrderTitle, order: Order::Asc, ..Default::default() };
    let mut pages: Vec<Post> = ctx.env.db.posts(&query).await.unwrap_or_default();
    pages.sort_by_key(|p| (p.menu_order, p.post_title.to_lowercase()));
    let (name, id) = if bulk { ("post_parent", "bulk_edit_post_parent") } else { ("post_parent", "post_parent") };
    let mut out = format!("<select name='{name}' id='{id}'>\n");
    if bulk {
        out.push_str(&format!("\t<option value=\"-1\">{}</option>\n", ctx.tr("&mdash; No Change &mdash;")));
    }
    out.push_str(&format!("\t<option value=\"0\">{}</option>\n", ctx.tr("Main Page (no parent)")));
    fn walk(pages: &[Post], parent: u64, depth: usize, out: &mut String, ctx: &Ctx<'_>) {
        for page in pages.iter().filter(|p| p.post_parent == parent) {
            let title = if page.post_title.is_empty() { rustpress_core::i18n::sprintf(&ctx.tr("#%d (no title)"), &[&page.id.to_string()]) } else { page.post_title.clone() };
            out.push_str(&format!("\t<option class=\"level-{depth}\" value=\"{}\">{}{}</option>\n", page.id, "&nbsp;&nbsp;&nbsp;".repeat(depth), esc_html(&title)));
            walk(pages, page.id, depth + 1, out, ctx);
        }
    }
    // Pages whose parent is not published start a tree of their own.
    let ids: Vec<u64> = pages.iter().map(|p| p.id).collect();
    let roots: Vec<Post> = pages.iter().filter(|p| p.post_parent == 0 || !ids.contains(&p.post_parent)).cloned().collect();
    for page in &roots {
        let title = if page.post_title.is_empty() { rustpress_core::i18n::sprintf(&ctx.tr("#%d (no title)"), &[&page.id.to_string()]) } else { page.post_title.clone() };
        out.push_str(&format!("\t<option class=\"level-0\" value=\"{}\">{}</option>\n", page.id, esc_html(&title)));
        walk(&pages, page.id, 1, &mut out, ctx);
    }
    out.push_str("</select>");
    out
}

/// The page templates that apply to the type, sorted by title: `(name, title)`.
pub(super) fn page_templates(ctx: &Ctx<'_>, info: &TypeInfo) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for template in ctx.env.theme.json.custom_templates() {
        let types: Vec<&str> = template.get("postTypes").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_else(|| vec!["page"]);
        if !types.contains(&info.name.as_str()) {
            continue;
        }
        if let (Some(name), Some(title)) = (template.get("name").and_then(|v| v.as_str()), template.get("title").and_then(|v| v.as_str())) {
            out.push((name.to_owned(), ctx.tr(title)));
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// `wp_terms_checklist( 0, array( 'taxonomy' => $taxonomy ) )`.
async fn checklist(ctx: &Ctx<'_>, taxonomy: &str) -> String {
    let terms = ctx.env.db.terms(taxonomy).await.unwrap_or_default();
    let mut popular: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.count > 0).collect();
    popular.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    popular.truncate(10);
    let popular_ids: Vec<u64> = popular.iter().map(|t| t.term_id).collect();
    let input = if taxonomy == "category" { "post_category[]".to_owned() } else { format!("tax_input[{taxonomy}][]") };
    fn tree(terms: &[rustpress_db::Term], parent: u64, popular: &[u64], taxonomy: &str, input: &str, out: &mut String) {
        let mut children: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.parent == parent).collect();
        children.sort_by_key(|t| t.name.to_lowercase());
        for term in children {
            let class = if popular.contains(&term.term_id) { " class=\"popular-category\"" } else { "" };
            out.push_str(&format!(
                "\n<li id='in-{taxonomy}-{id}-1'{class}><label class=\"selectit\"><input value=\"{id}\" type=\"checkbox\" name=\"{input}\" id=\"in-{taxonomy}-{id}-2\" /> {}</label>",
                esc_html(&term.name),
                id = term.term_id
            ));
            if terms.iter().any(|t| t.parent == term.term_id) {
                out.push_str("<ul class='children'>\n");
                tree(terms, term.term_id, popular, taxonomy, input, out);
                out.push_str("</ul>\n");
            }
            out.push_str("</li>\n");
        }
    }
    let mut out = String::new();
    tree(&terms, 0, &popular_ids, taxonomy, &input, &mut out);
    out
}

/// The template of `inline_edit()`, translated from the PHP: `{{NAME}}` is a value, `{{if FLAG}}` / `{{else}}` /
/// `{{end}}` a condition, `{{each LIST}}` a loop; the text between keeps the whitespace of the PHP output.
const FORM: &str = include_str!("inline_edit.tpl");

enum Node {
    Text(String),
    Var(String),
    If(String, Vec<Node>, Vec<Node>),
    Each(String, Vec<Node>),
}

enum Tok {
    Text(String),
    Tag(String),
}

fn tokenize(template: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('\u{E000}') {
        toks.push(Tok::Text(rest[..start].to_owned()));
        let after = &rest[start + '\u{E000}'.len_utf8()..];
        let end = after.find('\u{E001}').unwrap_or(after.len());
        toks.push(Tok::Tag(after[..end].to_owned()));
        rest = after.get(end + '\u{E001}'.len_utf8()..).unwrap_or("");
    }
    toks.push(Tok::Text(rest.to_owned()));
    toks
}

/// Parses up to `{{else}}` / `{{end}}` / the end; returns what stopped it.
fn parse(toks: &[Tok], pos: &mut usize) -> (Vec<Node>, &'static str) {
    let mut nodes = Vec::new();
    while *pos < toks.len() {
        let tok = &toks[*pos];
        *pos += 1;
        match tok {
            Tok::Text(text) => nodes.push(Node::Text(text.clone())),
            Tok::Tag(tag) => {
                if tag == "end" {
                    return (nodes, "end");
                }
                if tag == "else" {
                    return (nodes, "else");
                }
                if let Some(flag) = tag.strip_prefix("if ") {
                    let (then, stop) = parse(toks, pos);
                    let other = if stop == "else" { parse(toks, pos).0 } else { Vec::new() };
                    nodes.push(Node::If(flag.to_owned(), then, other));
                } else if let Some(list) = tag.strip_prefix("each ") {
                    nodes.push(Node::Each(list.to_owned(), parse(toks, pos).0));
                } else {
                    nodes.push(Node::Var(tag.clone()));
                }
            }
        }
    }
    (nodes, "eof")
}

/// The flags, values and lists one part of the template sees.
#[derive(Default)]
struct Scope {
    flags: std::collections::HashSet<&'static str>,
    vars: std::collections::HashMap<&'static str, String>,
    lists: std::collections::HashMap<&'static str, Vec<Scope>>,
}

impl Scope {
    fn flag(&mut self, name: &'static str, on: bool) {
        if on {
            self.flags.insert(name);
        }
    }

    fn var(&mut self, name: &'static str, value: impl Into<String>) {
        self.vars.insert(name, value.into());
    }
}

fn render(nodes: &[Node], scopes: &[&Scope], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Var(name) => {
                if let Some(value) = scopes.iter().rev().find_map(|s| s.vars.get(name.as_str())) {
                    out.push_str(value);
                }
            }
            Node::If(flag, then, other) => {
                let on = scopes.iter().any(|s| s.flags.contains(flag.as_str()));
                render(if on { then } else { other }, scopes, out);
            }
            Node::Each(list, body) => {
                if let Some(items) = scopes.iter().rev().find_map(|s| s.lists.get(list.as_str())) {
                    for item in items {
                        let mut inner: Vec<&Scope> = scopes.to_vec();
                        inner.push(item);
                        render(body, &inner, out);
                    }
                }
            }
        }
    }
}

/// Quick Edit and Bulk Edit rows (`WP_Posts_List_Table::inline_edit()`).
pub async fn inline_edit(ctx: &Ctx<'_>, info: &TypeInfo, first: Option<&Post>, columns: usize, taxonomies: &[(String, bool)], mode: &str) -> String {
    let site = ctx.site();
    let supports = |feature: &str| site.post_type_supports(&info.name, feature);
    let is_page = info.name == "page";
    let can_publish = ctx.can(if is_page { "publish_pages" } else { "publish_posts" });
    let can_edit_others = ctx.can(if is_page { "edit_others_pages" } else { "edit_others_posts" });
    let hier: Vec<&str> = taxonomies.iter().filter(|(name, h)| *h && name != "post_format").map(|(name, _)| name.as_str()).collect();
    // The internal taxonomies stay out, except the pattern categories (`show_in_quick_edit`).
    let flat: Vec<&str> = taxonomies.iter().filter(|(name, h)| !*h && name != "post_format" && (!name.starts_with("wp_") || name == "wp_pattern_category")).map(|(name, _)| name.as_str()).collect();
    let date = match first {
        // `touch_time()`: a draft that has no date of its own shows the current time.
        Some(post) if !post.post_date.is_empty() && post.post_date != "0000-00-00 00:00:00" && !(matches!(post.post_status.as_str(), "draft" | "pending") && (post.post_date_gmt.is_empty() || post.post_date_gmt == "0000-00-00 00:00:00")) => post.post_date.clone(),
        _ => ctx.site().date("Y-m-d H:i:s", ctx.env.now),
    };
    let templates = page_templates(ctx, info);
    let template_options: String = templates.iter().map(|(name, title)| format!("\n\t<option value='{}' >{}</option>", esc_attr(name), esc_html(title))).collect();

    // The template with the words translated; the markers are kept apart from the text for that.
    let marked = FORM.replace("\r\n", "\n").replace("{{", "\u{E000}").replace("}}", "\u{E001}");
    let translated = super::pages::translate_html(ctx, &marked);
    let (loop_start, loop_end) = ("\u{E000}LOOP\u{E001}", "\u{E000}ENDLOOP\u{E001}");
    let (head, rest) = translated.split_once(loop_start).unwrap_or((&translated, ""));
    let (body, foot) = rest.split_once(loop_end).unwrap_or((rest, ""));
    let body_nodes = parse(&tokenize(body), &mut 0).0;

    let hclass = if hier.is_empty() { "page" } else { "post" };
    let mut html = head.trim_start_matches("\n\t\t").to_owned();
    let author_users = if supports("author") && can_edit_others { author_users(ctx, info).await } else { Vec::new() };
    for bulk in [false, true] {
        let mut scope = Scope::default();
        scope.flag("BULK", bulk);
        scope.flag("QUICK", !bulk);
        scope.flag("TITLE", supports("title"));
        scope.flag("VIEWABLE", site.is_post_type_viewable(&info.name));
        scope.flag("QUICK_PUBLISH", !bulk && can_publish);
        scope.flag("HIER_TAX", !bulk && !hier.is_empty());
        scope.flag("PAGE_ATTR", supports("page-attributes"));
        scope.flag("HIERARCHICAL", info.hierarchical);
        scope.flag("HAS_TEMPLATES", !templates.is_empty());
        scope.flag("FLAT_TAX", !bulk && !flat.is_empty());
        scope.flag("COMMENTS", supports("comments"));
        scope.flag("TRACKBACKS", supports("trackbacks"));
        scope.flag("COMMENTS_OR_PINGS", supports("comments") || supports("trackbacks"));
        scope.flag("STICKY", info.name == "post" && can_publish && can_edit_others);
        scope.flag("FORMATS", bulk && supports("post-formats"));
        scope.flag("QUICK_NOAUTHOR", !bulk && !supports("author"));
        let classes = if bulk {
            format!("inline-edit-row inline-edit-row-{hclass} bulk-edit-row bulk-edit-row-{hclass} bulk-edit-{}", info.name)
        } else {
            format!("inline-edit-row inline-edit-row-{hclass} quick-edit-row quick-edit-row-{hclass} inline-edit-{}", info.name)
        };
        scope.var("ID", if bulk { "bulk-edit" } else { "inline-edit" });
        scope.var("CLASSES", classes);
        scope.var("COLSPAN", columns.to_string());
        scope.var("BQ", if bulk { "bulk" } else { "quick" });
        scope.var("LEGEND", ctx.tr(if bulk { "Bulk Edit" } else { "Quick Edit" }));
        scope.var("DATE", touch_time(ctx, &date));
        // `wp_dropdown_users()` for the authors: only a user who may edit the posts of others gets it.
        let authors_label = |dropdown: String| format!("<label class=\"inline-edit-author\"><span class=\"title\">{}</span>{dropdown}</label>", ctx.tr("Author"));
        let authors_html = if supports("author") && can_edit_others { authors_label(authors(ctx, &author_users, bulk)) } else { String::new() };
        scope.var("AUTHOR_QUICK", if bulk { String::new() } else { authors_html.clone() });
        scope.var("AUTHOR_BULK", if bulk { authors_html } else { String::new() });
        // `get_default_post_to_edit()` has no author yet.
        scope.var("POST_AUTHOR", "");
        scope.var("PARENT", if info.hierarchical { format!("{}\n", parents(ctx, info, bulk).await) } else { String::new() });
        scope.var("MENU_ORDER", "0");
        scope.var("DEFAULT_TEMPLATE", esc_html(&ctx.tr("Default template")));
        scope.var("TEMPLATES", template_options.clone());
        scope.var("STANDARD", esc_html(&ctx.tr("Standard")));
        scope.var("NONCE_FIELD", format!("<input type=\"hidden\" id=\"_inline_edit\" name=\"_inline_edit\" value=\"{}\" />", ctx.create_nonce("inlineeditnonce")));
        scope.var("SUBMIT", format!("<input type=\"submit\" name=\"bulk_edit\" id=\"bulk_edit\" class=\"button button-primary\" value=\"{}\"  />", esc_attr(&ctx.tr("Update"))));
        scope.var("M", mode);
        scope.var("SCREEN_ID", format!("edit-{}", info.name));
        scope.var("NOTICE", "<div class=\"notice notice-error notice-alt inline hidden\"><p class=\"error\"></p></div>");
        if !bulk {
            let mut items = Vec::new();
            for name in &hier {
                let mut item = Scope::default();
                item.var("TAX_LABEL", esc_html(&site.taxonomy_label(name)));
                item.var("TAX_INPUT", if *name == "category" { "post_category[]".to_owned() } else { format!("tax_input[{}][]", esc_attr(name)) });
                item.var("TAX_NAME", esc_attr(name));
                item.var("TAX_CHECKLIST", checklist(ctx, name).await);
                items.push(item);
            }
            scope.lists.insert("HIER", items);
            let mut items = Vec::new();
            for name in &flat {
                let mut item = Scope::default();
                // `assign_terms` of a taxonomy is `edit_posts` unless it says otherwise.
                item.flag("ITEM_CAN", ctx.can("edit_posts"));
                item.var("TAX_LABEL", esc_html(&site.taxonomy_label(name)));
                item.var("TAX_NAME", esc_attr(name));
                item.var("TAX_COMMAS", esc_html(&ctx.tr(if *name == "wp_pattern_category" { "Separate pattern categories with commas" } else { "Separate tags with commas" })));
                items.push(item);
            }
            scope.lists.insert("FLAT", items);
        }
        let mut statuses: Vec<(&str, String)> = Vec::new();
        if bulk {
            statuses.push(("-1", ctx.tr("&mdash; No Change &mdash;")));
        }
        if can_publish {
            statuses.push(("publish", ctx.tr("Published")));
            statuses.push(("future", ctx.tr("Scheduled")));
            if bulk {
                statuses.push(("private", ctx.tr("Private")));
            }
        }
        statuses.push(("pending", ctx.tr("Pending Review")));
        statuses.push(("draft", ctx.tr("Draft")));
        scope.lists.insert(
            "STATUS",
            statuses
                .into_iter()
                .map(|(value, text)| {
                    let mut item = Scope::default();
                    item.var("VALUE", esc_attr(value));
                    item.var("TEXT", esc_attr(&text));
                    item
                })
                .collect(),
        );
        // The formats of the theme, by name.
        scope.lists.insert(
            "FORMAT_LIST",
            ["aside", "audio", "chat", "gallery", "image", "link", "quote", "status", "video"]
                .into_iter()
                .map(|format| {
                    let mut item = Scope::default();
                    item.var("VALUE", esc_attr(format));
                    item.var("TEXT", esc_html(&ctx.tr(&format!("{}{}", format[..1].to_uppercase(), &format[1..]))));
                    item
                })
                .collect(),
        );
        render(&body_nodes, &[&scope], &mut html);
    }
    html.push_str(foot);
    // Starts with the empty line after `?>`, ends with the indentation of the closing `<?php`.
    format!("\n\t\t{html}\t\t")
}
