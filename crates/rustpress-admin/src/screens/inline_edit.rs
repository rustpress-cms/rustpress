//! `WP_Posts_List_Table::inline_edit()`: the hidden Quick Edit and Bulk Edit rows of the posts list.
//! The frame comes from the markup of WordPress (`inline_edit_post.tpl`, `inline_edit_page.tpl`); the author,
//! category, parent and template lists are made here.

use rustpress_core::formatting::{esc_attr, esc_html};
use rustpress_db::{Order, OrderBy, Post, PostQuery, User, UserListQuery};

use super::Ctx;
use super::posts::TypeInfo;

const POST_FORM: &str = include_str!("inline_edit_post.tpl");
const PAGE_FORM: &str = include_str!("inline_edit_page.tpl");

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

/// `wp_dropdown_users()` of the Quick Edit authors (`show` = `display_name_with_login`).
async fn authors(ctx: &Ctx<'_>, info: &TypeInfo, bulk: bool) -> String {
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
    let mut out = String::from("<select name='post_author' class='authors'>\n");
    if bulk {
        out.push_str(&format!("\t<option value='-1'>{}</option>\n", ctx.tr("&mdash; No Change &mdash;")));
    }
    for user in &users {
        out.push_str(&format!("\t<option value='{}'>{} ({})</option>\n", user.id, esc_html(&user.display_name), esc_html(&user.user_login)));
    }
    out.push_str("</select>");
    out
}

/// `wp_terms_checklist( 0, array( 'taxonomy' => 'category' ) )`.
async fn categories(ctx: &Ctx<'_>) -> String {
    let terms = ctx.env.db.terms("category").await.unwrap_or_default();
    let mut popular: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.count > 0).collect();
    popular.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    popular.truncate(10);
    let popular_ids: Vec<u64> = popular.iter().map(|t| t.term_id).collect();
    fn tree(terms: &[rustpress_db::Term], parent: u64, popular: &[u64], out: &mut String) {
        let mut children: Vec<&rustpress_db::Term> = terms.iter().filter(|t| t.parent == parent).collect();
        children.sort_by_key(|t| t.name.to_lowercase());
        for term in children {
            let class = if popular.contains(&term.term_id) { " class=\"popular-category\"" } else { "" };
            out.push_str(&format!(
                "\n<li id='in-category-{id}-1'{class}><label class=\"selectit\"><input value=\"{id}\" type=\"checkbox\" name=\"post_category[]\" id=\"in-category-{id}-2\" /> {}</label>",
                esc_html(&term.name),
                id = term.term_id
            ));
            if terms.iter().any(|t| t.parent == term.term_id) {
                out.push_str("<ul class='children'>\n");
                tree(terms, term.term_id, popular, out);
                out.push_str("</ul>\n");
            }
            out.push_str("</li>\n");
        }
    }
    let mut out = String::from("\t\t\t\t\t\t\t");
    tree(&terms, 0, &popular_ids, &mut out);
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
fn page_templates(ctx: &Ctx<'_>, info: &TypeInfo) -> Vec<(String, String)> {
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

/// The two rows, for the `post` form (categories) or the `page` form (parent, order, template).
pub async fn inline_edit(ctx: &Ctx<'_>, info: &TypeInfo, first: Option<&Post>, columns: usize, hierarchical_taxonomies: bool) -> String {
    let form = if hierarchical_taxonomies { POST_FORM } else { PAGE_FORM };
    let date = match first {
        // `touch_time()`: a draft that has no date of its own shows the current time.
        Some(post) if !post.post_date.is_empty() && post.post_date != "0000-00-00 00:00:00" && !(matches!(post.post_status.as_str(), "draft" | "pending") && (post.post_date_gmt.is_empty() || post.post_date_gmt == "0000-00-00 00:00:00")) => post.post_date.clone(),
        _ => ctx.site().date("Y-m-d H:i:s", ctx.env.now),
    };
    let can_edit_others = ctx.can(if info.name == "page" { "edit_others_pages" } else { "edit_others_posts" });
    let (quick, bulk) = if can_edit_others { (authors(ctx, info, false).await, authors(ctx, info, true).await) } else { (String::new(), String::new()) };
    let mut html = super::pages::translate_html(ctx, &form.replace("\r\n", "\n"));
    if !can_edit_others {
        html = html.replace("<label class=\"inline-edit-author\"><span class=\"title\">Author</span>{{AUTHORS}}</label>", "").replace("<label class=\"inline-edit-author\"><span class=\"title\">Author</span>{{AUTHORS_BULK}}</label>", "");
    }
    let templates: String = page_templates(ctx, info).iter().map(|(name, title)| format!("\n\t<option value='{}' >{}</option>", esc_attr(name), esc_html(title))).collect();
    let templates = format!("\t\t\t\t\t\t\t{templates}");
    if hierarchical_taxonomies {
        html = html.replace("{{CATEGORIES}}", &categories(ctx).await);
    } else {
        html = html.replace("{{PARENT}}", &parents(ctx, info, false).await).replace("{{PARENT_BULK}}", &parents(ctx, info, true).await);
    }
    // Without templates the select is left out altogether.
    if page_templates(ctx, info).is_empty() {
        let mut cleaned = String::new();
        let mut rest = html.as_str();
        while let Some(start) = rest.find("<label>\n\t\t\t\t\t\t<span class=\"title\">Template</span>") {
            cleaned.push_str(&rest[..start]);
            let end = rest[start..].find("</label>\n").map_or(rest.len(), |i| start + i + "</label>\n".len());
            rest = &rest[end..];
        }
        cleaned.push_str(rest);
        html = cleaned;
    }
    html = html
        .replace("{{COLSPAN}}", &columns.to_string())
        .replace("{{DATE}}", &touch_time(ctx, &date))
        .replace("{{AUTHORS}}", &quick)
        .replace("{{AUTHORS_BULK}}", &bulk)
        .replace("{{TEMPLATES}}", &templates)
        .replace("{{MENU_ORDER}}", &first.map_or(0, |p| p.menu_order).to_string())
        .replace("{{NONCE}}", &ctx.create_nonce("inlineeditnonce"));
    format!("\t\t{html}\n\t\t\n")
}
