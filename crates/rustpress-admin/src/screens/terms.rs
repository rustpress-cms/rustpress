//! `wp-admin/edit-tags.php` and `term.php`: categories, tags and the terms of other taxonomies.

use rustpress_core::OrLog;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::i18n::sprintf;
use rustpress_core::urls::add_query_arg;
use rustpress_db::Term;

use super::common::{dropdown_terms, forbidden_heading, read_only};
use super::{AdminResponse, Ctx, redirect, render};
use crate::chrome::{Assets, Help, HelpTab, Screen};
use crate::content::{Content, TermData};
use crate::list_table::{Column, Table, row_actions};
use crate::sanitize::esc_textarea;

/// What a taxonomy screen needs to know about the taxonomy.
pub struct TaxInfo {
    pub name: String,
    pub hierarchical: bool,
    pub plural: String,
    pub singular: String,
    pub query_var: Option<String>,
    pub post_type: String,
    pub labels: TaxLabels,
}

/// The labels of `get_taxonomy_labels()` the screens use: a taxonomy that only names itself gets the ones of tags
/// or of categories.
#[derive(Debug, Clone)]
pub struct TaxLabels {
    pub search_items: String,
    pub add_new_item: String,
    pub edit_item: String,
    pub update_item: String,
    pub view_item: String,
    pub parent_item: String,
    pub items_list: String,
    pub items_list_navigation: String,
}

impl TaxLabels {
    fn new(site: &rustpress_core::Site, hierarchical: bool) -> Self {
        let pick = |tag: &str, category: &str| site.tr(if hierarchical { category } else { tag });
        Self {
            search_items: pick("Search Tags", "Search Categories"),
            add_new_item: pick("Add Tag", "Add Category"),
            edit_item: pick("Edit Tag", "Edit Category"),
            update_item: pick("Update Tag", "Update Category"),
            view_item: pick("View Tag", "View Category"),
            parent_item: site.tr("Parent Category"),
            items_list: pick("Tags list", "Categories list"),
            items_list_navigation: pick("Tags list navigation", "Categories list navigation"),
        }
    }
}

pub fn tax_info(ctx: &Ctx<'_>, name: &str, post_type: Option<&str>) -> Option<TaxInfo> {
    let site = ctx.site();
    let (hierarchical, plural, singular, query_var, objects) = match name {
        "category" => (true, site.tr("Categories"), site.tr("Category"), Some("category_name".to_owned()), vec!["post".to_owned()]),
        "post_tag" => (false, site.tr("Tags"), site.tr("Tag"), Some("tag".to_owned()), vec!["post".to_owned()]),
        other => {
            let info = site.types.taxonomy(other).filter(|t| t.public || t.queryable || !t.object_types.is_empty())?;
            (info.hierarchical, info.plural_label(), info.singular_label(), info.query_var.clone(), info.object_types.clone())
        }
    };
    let post_type = post_type.filter(|p| !p.is_empty() && objects.iter().any(|o| o == p)).map(str::to_owned).or_else(|| objects.first().cloned()).unwrap_or_else(|| "post".to_owned());
    let labels = TaxLabels::new(site, hierarchical);
    Some(TaxInfo { name: name.to_owned(), hierarchical, plural, singular, query_var, post_type, labels })
}

pub(super) fn messages(ctx: &Ctx<'_>, info: &TaxInfo) -> Vec<String> {
    let tr = |s: &str| ctx.tr(s);
    match info.name.as_str() {
        "category" => vec![String::new(), tr("Category added."), tr("Category deleted."), tr("Category updated."), tr("Category not added."), tr("Category not updated."), tr("Categories deleted.")],
        "post_tag" => vec![String::new(), tr("Tag added."), tr("Tag deleted."), tr("Tag updated."), tr("Tag not added."), tr("Tag not updated."), tr("Tags deleted.")],
        _ => vec![String::new(), tr("Item added."), tr("Item deleted."), tr("Item updated."), tr("Item not added."), tr("Item not updated."), tr("Items deleted.")],
    }
}

fn term_edit_url(ctx: &Ctx<'_>, info: &TaxInfo, term: &Term, referer: &str) -> String {
    let mut url = ctx.admin_url(&format!("term.php?taxonomy={}&tag_ID={}&post_type={}", info.name, term.term_id, info.post_type));
    if !referer.is_empty() {
        url.push_str(&format!("&wp_http_referer={}", rustpress_core::canonical::rawurlencode(referer)));
    }
    url
}

fn quote(text: &str) -> String {
    format!("&#8220;{}&#8221;", esc_html(text))
}

/// One `<tr>` of the term list.
async fn row(ctx: &Ctx<'_>, info: &TaxInfo, term: &Term, level: usize, pad_level: usize, referer: &str) -> String {
    let site = ctx.site();
    let tr = |s: &str| ctx.tr(s);
    let default = info.name == "category" && site.options.u64("default_category", 1) == term.term_id;
    let can_edit = ctx.can("manage_categories");
    let name = site.term_name(term);
    let pad = "&#8212; ".repeat(pad_level);
    let edit_url = term_edit_url(ctx, info, term, referer);
    let checkbox = if default || !can_edit {
        "&nbsp;".to_owned()
    } else {
        format!("<input type=\"checkbox\" name=\"delete_tags[]\" value=\"{id}\" id=\"cb-select-{id}\" /><label for=\"cb-select-{id}\"><span class=\"screen-reader-text\">{}</span></label>", sprintf(&tr("Select %s"), &[&esc_html(&term.name)]), id = term.term_id)
    };
    let title = if can_edit { format!("<a class=\"row-title\" href=\"{}\">{pad}{}</a>", esc_url(&edit_url), esc_html(&name)) } else { format!("{pad}{}", esc_html(&name)) };
    let mut actions: Vec<(String, String)> = Vec::new();
    if can_edit {
        actions.push(("edit".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&edit_url), esc_attr(&sprintf(&tr("Edit %s"), &[&quote(&name)])), tr("Edit"))));
        actions.push((
            "inline hide-if-no-js".into(),
            format!("<button type=\"button\" class=\"button-link editinline\" aria-label=\"{}\" aria-expanded=\"false\">{}</button>", esc_attr(&sprintf(&tr("Quick edit %s inline"), &[&quote(&name)])), tr("Quick&nbsp;Edit")),
        ));
        if !default {
            let url = format!("edit-tags.php?action=delete&amp;taxonomy={}&amp;tag_ID={}&amp;_wpnonce={}", info.name, term.term_id, ctx.create_nonce(&format!("delete-tag_{}", term.term_id)));
            actions.push(("delete".into(), format!("<a href=\"{url}\" class=\"delete-tag aria-button-if-js\" aria-label=\"{}\">{}</a>", esc_attr(&sprintf(&tr("Delete %s"), &[&quote(&name)])), tr("Delete"))));
        }
    }
    if let Ok(link) = site.term_link(term).await {
        actions.push(("view".into(), format!("<a href=\"{}\" aria-label=\"{}\">{}</a>", esc_url(&link), esc_attr(&sprintf(&tr("View %s archive"), &[&quote(&name)])), tr("View"))));
    }
    let description = if term.description.is_empty() { format!("<span aria-hidden=\"true\">&#8212;</span><span class=\"screen-reader-text\">{}</span>", tr("No description")) } else { term.description.clone() };
    let count_url = match &info.query_var {
        Some(var) => add_query_arg("edit.php", var, &term.slug),
        None => add_query_arg(&add_query_arg("edit.php", "taxonomy", &info.name), "term", &term.slug),
    };
    let count_url = if info.post_type != "post" { add_query_arg(&count_url, "post_type", &info.post_type) } else { count_url };
    format!(
        "<tr id=\"tag-{id}\" class=\"level-{level}\"><td class=\"check-column\">{checkbox}</td><th class='name column-name has-row-actions column-primary' data-colname=\"{n}\" scope=\"row\" aria-label=\"{label}\"><strong>{title}</strong><br /><div class=\"hidden\" id=\"inline_{id}\"><div class=\"name\">{ename}</div><div class=\"slug\">{eslug}</div><div class=\"parent\">{parent}</div></div>{actions}</th><td class='description column-description' data-colname=\"{d}\">{description}</td><td class='slug column-slug' data-colname=\"{s}\">{eslug}</td><td class='posts column-posts' data-colname=\"{c}\"><a href='{}'>{}</a></td></tr>",
        esc_url(&count_url),
        site.number_i18n(term.count.max(0) as u64),
        id = term.term_id,
        n = tr("Name"),
        label = esc_attr(&name),
        ename = esc_textarea(&name),
        eslug = esc_textarea(&term.slug),
        parent = term.parent,
        actions = row_actions(site, &actions),
        d = tr("Description"),
        s = tr("Slug"),
        c = tr("Count"),
    )
}

/// The terms in the order of the list: `(term, class level, dash level)`.
fn ordered(terms: &[Term], tree: bool, orderby: &str, desc: bool, search: &str) -> Vec<(Term, usize, usize)> {
    let search = search.to_lowercase();
    let matches = |t: &Term| search.is_empty() || t.name.to_lowercase().contains(&search) || t.slug.to_lowercase().contains(&search);
    let depth = |t: &Term| {
        let mut level = 0;
        let mut parent = t.parent;
        while parent != 0 && level < 50 {
            match terms.iter().find(|p| p.term_id == parent) {
                Some(p) => {
                    level += 1;
                    parent = p.parent;
                }
                None => break,
            }
        }
        level
    };
    if tree && search.is_empty() {
        let mut out: Vec<(Term, usize, usize)> = super::common::term_tree(terms, &[]).into_iter().map(|(t, l)| (t, l, l)).collect();
        if desc {
            let mut roots: Vec<Vec<(Term, usize, usize)>> = Vec::new();
            for entry in out.drain(..) {
                if entry.1 == 0 {
                    roots.push(Vec::new());
                }
                if let Some(last) = roots.last_mut() {
                    last.push(entry);
                }
            }
            roots.reverse();
            return roots.into_iter().flatten().collect();
        }
        return out;
    }
    let mut flat: Vec<Term> = terms.iter().filter(|t| matches(t)).cloned().collect();
    let key = |t: &Term| -> String {
        match orderby {
            "slug" => t.slug.to_lowercase(),
            "description" => t.description.to_lowercase(),
            _ => t.name.to_lowercase(),
        }
    };
    if orderby == "count" {
        flat.sort_by(|a, b| a.count.cmp(&b.count).then(a.term_id.cmp(&b.term_id)));
        if desc {
            flat.sort_by(|a, b| b.count.cmp(&a.count).then(a.term_id.cmp(&b.term_id)));
        }
    } else {
        flat.sort_by(|a, b| key(a).cmp(&key(b)).then(a.term_id.cmp(&b.term_id)));
        if desc {
            flat.sort_by(|a, b| key(b).cmp(&key(a)).then(a.term_id.cmp(&b.term_id)));
        }
    }
    flat.into_iter().map(|t| if search.is_empty() { let d = depth(&t); (t, d, 0) } else { (t, 0, 0) }).collect()
}

fn help(ctx: &Ctx<'_>, info: &TaxInfo) -> Help {
    let tr = |s: &str| ctx.tr(s);
    let overview = if info.name == "category" {
        format!("<p>{}</p>", tr("You can use categories to define sections of your site and group related posts. The default category is Uncategorized until you change it in your writing settings."))
    } else {
        format!("<p>{}</p>", tr("You can assign keywords to your posts using <strong>tags</strong>. Unlike categories, tags have no hierarchy, meaning there&#8217;s no relationship from one tag to another."))
    };
    Help {
        tabs: vec![HelpTab { id: "overview".into(), title: tr("Overview"), content: overview }],
        sidebar: format!("<p><strong>{}</strong></p><p>{}</p>", tr("For more information:"), tr("<a href=\"https://wordpress.org/support/forums/\">Support forums</a>")),
    }
}

fn screen_for(info: &TaxInfo, title: String, help: Help, file: &str) -> Screen {
    let parent = match info.post_type.as_str() {
        "post" => "edit.php".to_owned(),
        other => format!("edit.php?post_type={other}"),
    };
    let submenu = match info.post_type.as_str() {
        "post" => format!("edit-tags.php?taxonomy={}", info.name),
        other => format!("edit-tags.php?taxonomy={}&amp;post_type={other}", info.name),
    };
    let id = format!("edit-{}", info.name);
    Screen { file: file.into(), parent_file: parent, submenu_file: Some(submenu), post_type: info.post_type.clone(), id, title, help: Some(help), ..Default::default() }
}

/// `edit-tags.php`.
pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let site = ctx.site();
    // Without a taxonomy the screen is the one of the tags.
    let taxonomy = request.request("taxonomy").filter(|t| !t.is_empty()).unwrap_or("post_tag").to_owned();
    // Taxonomies without a screen of their own.
    if matches!(taxonomy.as_str(), "post_format" | "nav_menu" | "link_category" | "wp_theme" | "wp_template_part_area") {
        return super::common::die_plain(ctx, "Sorry, you are not allowed to edit terms in this taxonomy.");
    }
    let Some(info) = tax_info(ctx, &taxonomy, request.request("post_type")) else {
        return super::common::die_plain(ctx, "Invalid taxonomy.");
    };
    if !ctx.can("manage_categories") {
        return forbidden_heading(ctx, "Sorry, you are not allowed to manage terms in this taxonomy.");
    }
    let action = {
        let a = request.request("action").unwrap_or_default();
        if a.is_empty() || a == "-1" { request.request("action2").unwrap_or_default().to_owned() } else { a.to_owned() }
    };
    let referer = super::common::wp_get_referer(ctx).unwrap_or_else(|| request.request_uri.clone());
    let index_url = format!("edit-tags.php?taxonomy={}{}", info.name, if info.post_type == "post" { String::new() } else { format!("&post_type={}", info.post_type) });
    let messages = messages(ctx, &info);

    let back = |message: u32, error: bool| {
        let base = ["message", "error", "action", "action2", "tag_ID", "_wpnonce", "_wp_http_referer", "delete_tags"].iter().fold(referer.clone(), |url, key| rustpress_core::urls::remove_query_arg(&url, key));
        let base = if base.contains("edit-tags.php") || base.contains("term.php") { base } else { ctx.admin_url(&index_url) };
        let mut url = add_query_arg(&base, "message", &message.to_string());
        if error {
            url = add_query_arg(&url, "error", "1");
        }
        redirect(&url)
    };
    let expired = || super::die(ctx.env, 403, &ctx.tr("An error occurred."), &ctx.tr("The link you followed has expired."));
    if request.post && action == "editedtag" {
        return edited_tag(ctx, &info).await;
    }
    if action == "add-tag" && request.post {
        if request.post_field("_wpnonce_add-tag").is_none_or(|n| ctx.env.auth.verify_nonce(n, "add-tag", ctx.user.id, &ctx.token, ctx.env.now).is_none()) {
            return expired();
        }
        let Some(writer) = ctx.env.writer else { return read_only(ctx) };
        let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
        let data = TermData {
            name: request.post_field("tag-name").unwrap_or_default().to_owned(),
            slug: request.post_field("slug").unwrap_or_default().to_owned(),
            parent: request.post_field("parent").and_then(|p| p.parse::<i64>().ok()).filter(|p| *p > 0).unwrap_or(0) as u64,
            description: request.post_field("description").unwrap_or_default().to_owned(),
        };
        return match content.insert_term(&info.name, &data).await {
            Ok(_) => back(1, false),
            Err(_) => back(4, true),
        };
    }
    if action == "delete" {
        let Some(writer) = ctx.env.writer else { return read_only(ctx) };
        let content = Content { site, db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
        let bulk_ids: Vec<u64> = request.form.iter().filter(|(k, _)| k == "delete_tags[]").filter_map(|(_, v)| v.parse().ok()).collect();
        if request.post && !bulk_ids.is_empty() {
            if !ctx.verify_nonce("bulk-tags") {
                return expired();
            }
            for id in bulk_ids {
                content.delete_term(id, &info.name).await.or_log();
            }
            return back(6, false);
        }
        let id: u64 = request.request("tag_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
        if !ctx.verify_nonce(&format!("delete-tag_{id}")) {
            return expired();
        }
        content.delete_term(id, &info.name).await.or_log();
        return back(2, false);
    }

    // The list
    let origin = {
        let siteurl = &site.settings.siteurl;
        match siteurl.split_once("://") {
            Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or_default()),
            None => siteurl.clone(),
        }
    };
    let search = request.get("s").map(|s| s.trim().to_owned()).unwrap_or_default();
    let orderby = request.get("orderby").filter(|o| ["name", "description", "slug", "count"].contains(o)).unwrap_or("name").to_owned();
    let order_inputs = {
        let mut inputs = String::new();
        for name in ["orderby", "order"] {
            if let Some(value) = request.get(name).filter(|v| !v.is_empty()) {
                inputs.push_str(&format!("<input type=\"hidden\" name=\"{name}\" value=\"{}\" />
", esc_attr(value)));
            }
        }
        inputs
    };
    let desc = request.get("order").is_some_and(|o| o.eq_ignore_ascii_case("desc"));
    let terms = ctx.env.db.terms(&info.name).await.unwrap_or_default();
    let hierarchical_view = info.hierarchical && request.get("orderby").is_none();
    let list = ordered(&terms, hierarchical_view, &orderby, desc, &search);
    let per_page = {
        let key = format!("{}edit_{}_per_page", ctx.env.db.table_prefix(), info.name);
        ctx.env.db.user_meta(ctx.user.id, &key).await.ok().flatten().and_then(|v| v.parse::<u64>().ok()).filter(|v| *v >= 1).unwrap_or(20)
    };
    let paged = request.get("paged").and_then(|p| p.parse::<u64>().ok()).filter(|p| *p > 0).unwrap_or(1);
    let total = list.len() as u64;
    if let Some(response) = super::common::past_last_page(ctx, paged, total, per_page) {
        return response;
    }
    let tree_paging = hierarchical_view && search.is_empty();
    let page_rows: Vec<(Term, usize, usize)> = if tree_paging {
        let mut out = Vec::new();
        let mut root = 0u64;
        for entry in list.iter().cloned() {
            if entry.1 == 0 {
                root += 1;
            }
            if root > (paged - 1).saturating_mul(per_page) && root <= paged.saturating_mul(per_page) {
                out.push(entry);
            }
        }
        out
    } else {
        list.iter().skip(((paged - 1).saturating_mul(per_page)) as usize).take(per_page as usize).cloned().collect()
    };
    let current_url = format!("{origin}{}", request.request_uri);
    let mut rows = String::new();
    for (term, level, pad) in page_rows.iter() {
        let r = row(ctx, &info, term, *level, *pad, &request.request_uri).await;
        rows.push_str(&if hierarchical_view { format!("	{r}") } else { r });
    }
    let columns = vec![
        Column::new("cb", ""),
        Column::new("name", &tr("Name")).sortable("name", false, Some(&tr("Name")), Some("asc")),
        Column::new("description", &tr("Description")).sortable("description", false, Some(&tr("Description")), None),
        Column::new("slug", &tr("Slug")).sortable("slug", false, Some(&tr("Slug")), None),
        Column::new("posts", &tr("Count")).sortable("count", false, Some(&tr("Count")), None),
    ];
    let mut table = Table {
        site,
        plural: "tags".into(),
        singular: "tag".into(),
        columns,
        primary: "name".into(),
        current_url,
        orderby: request.get("orderby").unwrap_or_default().to_owned(),
        order: if desc { "desc".into() } else { "asc".into() },
        total_items: total,
        per_page,
        current_page: paged,
        bulk_actions: vec![("delete".into(), tr("Delete"))],
        has_items: !page_rows.is_empty(),
    };
    if !ctx.can("manage_categories") {
        table.bulk_actions.clear();
    }
    let caption = if hierarchical_view {
        if desc { tr("Table ordered hierarchically. Descending.") } else { tr("Table ordered hierarchically. Ascending.") }
    } else {
        let by = match orderby.as_str() {
            "slug" => tr("Slug"),
            "description" => tr("Description"),
            "count" => tr("Posts Count"),
            _ => tr("Name"),
        };
        sprintf(&if desc { tr("Table ordered by %s. Descending.") } else { tr("Table ordered by %s. Ascending.") }, &[&by])
    };
    let nonce_field = format!("<input type=\"hidden\" name=\"taxonomy\" value=\"{}\" />\n<input type=\"hidden\" name=\"post_type\" value=\"{}\" />\n\n{}", info.name, info.post_type, ctx.nonce_field("bulk-tags"));
    let table_html = table.display(&nonce_field, "", "", &caption, &rows, &info.labels.items_list);

    // The page
    let title = info.plural.clone();
    let mut h = format!("<div class=\"wrap nosubsub\">\n<h1 class=\"wp-heading-inline\">{}</h1>\n\n", esc_html(&title));
    if !search.is_empty() {
        h.push_str(&format!("<span class=\"subtitle\">{}</span>\n", sprintf(&tr("Search results for: %s"), &[&format!("<strong>{}</strong>", esc_html(&search))])));
    }
    h.push_str("\n<hr class=\"wp-header-end\">\n\n");
    let code = request.get("message").and_then(|m| m.parse::<usize>().ok()).filter(|m| *m > 0 && *m < messages.len());
    if let Some(code) = code {
        let error = request.get("error").is_some() || code == 4 || code == 5;
        h.push_str(&format!("<div id=\"message\" class=\"{} notice is-dismissible\"><p>{}</p></div>\n", if error { "notice-error" } else { "updated" }, messages[code]));
    }
    h.push_str("<div id=\"ajax-response\"></div>\n\n");
    h.push_str(&format!(
        "<form class=\"search-form wp-clearfix\" method=\"get\">\n<input type=\"hidden\" name=\"taxonomy\" value=\"{tax}\" />\n<input type=\"hidden\" name=\"post_type\" value=\"{pt}\" />\n{order_inputs}\n<p class=\"search-box\">\n\t<label class=\"screen-reader-text\" for=\"tag-search-input\">{}</label>\n\t<input type=\"search\" id=\"tag-search-input\" name=\"s\" value=\"{}\" />\n\t\t<input type=\"submit\" id=\"search-submit\" class=\"button button-compact\" value=\"{}\"  /></p>\n\t\t\n</form>\n\n",
        format!("{}:", info.labels.search_items),
        esc_attr(&search),
        esc_attr(&info.labels.search_items),
        tax = info.name,
        pt = info.post_type,
        order_inputs = order_inputs
    ));
    h.push_str("<div id=\"col-container\" class=\"wp-clearfix\">\n\n<div id=\"col-left\">\n<div class=\"col-wrap\">\n\n\t\n");
    h.push_str(&add_form(ctx, &info).await);
    h.push_str("</div>\n</div><!-- /col-left -->\n\n<div id=\"col-right\">\n<div class=\"col-wrap\">\n\n\n<form id=\"posts-filter\" method=\"post\">\n");
    h.push_str(&table_html.replacen("<input type=\"hidden\" name=\"taxonomy\"", "<input type=\"hidden\" name=\"taxonomy\"", 1));
    h.push_str("\n</form>\n\n");
    let default_name = if info.name == "category" { terms.iter().find(|t| t.term_id == site.options.u64("default_category", 1)).map(|t| t.name.clone()).unwrap_or_default() } else { String::new() };
    // The converters belong to those who may import.
    let can_import = ctx.can("import");
    if info.name == "category" || (info.name == "post_tag" && can_import) {
        h.push_str("<div class=\"form-wrap edit-term-notes\">\n<p>\n\t");
    }
    if info.name == "category" {
        h.push_str(&sprintf(&tr("Deleting a category does not delete the posts in that category. Instead, posts that were only assigned to the deleted category are set to the default category %s. The default category cannot be deleted."), &[&format!("<strong>{}</strong>", esc_html(&default_name))]));
        if can_import {
            h.push_str(&format!("</p>\n\t\t<p>\n\t\t{}\t</p>\n\t</div>\n</div>\n", sprintf(&tr("Categories can be selectively converted to tags using the <a href=\"%s\">category to tag converter</a>."), &[&ctx.admin_url("import.php")])));
        } else {
            h.push_str("</p>\n\t</div>\n</div>\n");
        }
    } else if info.name == "post_tag" && !can_import {
        h.push_str("</div>\n");
    } else if info.name == "post_tag" {
        h.push_str(&format!("{}\t</p>\n</div>\n\t</div>\n", sprintf(&tr("Tags can be selectively converted to categories using the <a href=\"%s\">tag to category converter</a>."), &[&ctx.admin_url("import.php")])));
    } else {
        // The notes are the description of the taxonomy; without one there is no box.
        let description = site.types.taxonomy(&info.name).map(|t| t.description.clone()).unwrap_or_default();
        if description.is_empty() {
            h.push_str("</div>\n");
        } else {
            h.push_str(&format!("<div class=\"form-wrap edit-term-notes\">\n<p>\n\t{}</p>\n</div>\n\t</div>\n", esc_html(&description)));
        }
    }
    h.push_str("</div><!-- /col-right -->\n\n</div><!-- /col-container -->\n\n</div><!-- /wrap -->\n\n<script>\ntry{document.forms.addtag['tag-name'].focus();}catch(e){}\n</script>\n\t\n");
    h.push_str(&inline_edit(ctx, &info));
    let screen = screen_for(&info, title, help(ctx, &info), "edit-tags.php");
    let assets = Assets { scripts: vec!["admin-tags".into(), "inline-edit-tax".into()], ..Default::default() };
    render(ctx, &screen, &h, &assets).await
}

async fn add_form(ctx: &Ctx<'_>, info: &TaxInfo) -> String {
    let tr = |s: &str| ctx.tr(s);
    let singular = &info.labels.add_new_item;
    let mut h = format!(
        "<div class=\"form-wrap\">\n<h2>{}</h2>\n<form id=\"addtag\" method=\"post\" action=\"edit-tags.php\" class=\"validate\"\n\t>\n<input type=\"hidden\" name=\"action\" value=\"add-tag\" />\n<input type=\"hidden\" name=\"screen\" value=\"edit-{}\" />\n<input type=\"hidden\" name=\"taxonomy\" value=\"{}\" />\n<input type=\"hidden\" name=\"post_type\" value=\"{}\" />\n\t",
        singular,
        info.name,
        info.name,
        info.post_type
    );
    h.push_str(&format!(
        "<input type=\"hidden\" id=\"_wpnonce_add-tag\" name=\"_wpnonce_add-tag\" value=\"{}\" /><input type=\"hidden\" name=\"_wp_http_referer\" value=\"{}\" />\n",
        ctx.create_nonce("add-tag"),
        esc_url(&ctx.request.request_uri)
    ));
    h.push_str(&format!(
        "<div class=\"form-field form-required term-name-wrap\">\n\t<label for=\"tag-name\">{}</label>\n\t<input name=\"tag-name\" id=\"tag-name\" type=\"text\" value=\"\" size=\"40\" aria-required=\"true\" aria-describedby=\"name-description\" />\n\t<p id=\"name-description\">{}</p>\n</div>\n",
        tr("Name"),
        tr("The name is how it appears on your site.")
    ));
    h.push_str(&format!(
        "<div class=\"form-field term-slug-wrap\">\n\t<label for=\"tag-slug\">{}</label>\n\t<input name=\"slug\" id=\"tag-slug\" type=\"text\" value=\"\" size=\"40\" aria-describedby=\"slug-description\" />\n\t<p id=\"slug-description\">{}</p>\n</div>\n",
        tr("Slug"),
        tr("The &#8220;slug&#8221; is the URL-friendly version of the name. It is usually all lowercase and contains only letters, numbers, and hyphens.")
    ));
    if info.hierarchical {
        let dropdown = dropdown_terms(ctx, &info.name, "parent", "parent", 0, "postform", Some(("-1", &tr("None"))), &[]).await.replacen("class='postform'>", "class='postform' aria-describedby=\"parent-description\">", 1);
        let help_text = if info.name == "category" { tr("Categories, unlike tags, can have a hierarchy. You might have a Jazz category, and under that have children categories for Bebop and Big Band. Totally optional.") } else { tr("Assign a parent term to create a hierarchy. The term Jazz, for example, would be the parent of Bebop and Big Band.") };
        h.push_str(&format!(
            "\t<div class=\"form-field term-parent-wrap\">\n\t<label for=\"parent\">{}</label>\n\t\t{dropdown}\t\t\t\t<p id=\"parent-description\">{help_text}</p>\n\t</div>\n",
            info.labels.parent_item.clone()
        ));
    }
    h.push_str(&format!(
        "\t<div class=\"form-field term-description-wrap\">\n\t<label for=\"tag-description\">{}</label>\n\t<textarea name=\"description\" id=\"tag-description\" rows=\"5\" cols=\"40\" aria-describedby=\"description-description\"></textarea>\n\t<p id=\"description-description\">{}</p>\n</div>\n\n",
        tr("Description"),
        tr("The description is not prominent by default; however, some themes may show it.")
    ));
    h.push_str(&format!("\t\t<p class=\"submit\">\n\t\t<input type=\"submit\" name=\"submit\" id=\"submit\" class=\"button button-primary\" value=\"{}\"  />\t\t<span class=\"spinner\"></span>\n\t</p>\n\t</form></div>\n", esc_attr(&sprintf(&tr("%s"), &[singular]))));
    h
}

fn inline_edit(ctx: &Ctx<'_>, info: &TaxInfo) -> String {
    let tr = |s: &str| ctx.tr(s);
    format!(
        "\t\t<form method=\"get\">\n\t\t<table style=\"display: none\"><tbody id=\"inlineedit\">\n\n\t\t\t<tr id=\"inline-edit\" class=\"inline-edit-row\" style=\"display: none\">\n\t\t\t<td colspan=\"5\" class=\"colspanchange\">\n\t\t\t<div class=\"inline-edit-wrapper\">\n\n\t\t\t<fieldset>\n\t\t\t\t<legend class=\"inline-edit-legend\">{quick}</legend>\n\t\t\t\t<div class=\"inline-edit-col\">\n\t\t\t\t<label>\n\t\t\t\t\t<span class=\"title\">{name}</span>\n\t\t\t\t\t<span class=\"input-text-wrap\"><input type=\"text\" name=\"name\" class=\"ptitle\" value=\"\" /></span>\n\t\t\t\t</label>\n\n\t\t\t\t<label>\n\t\t\t\t\t<span class=\"title\">{slug}</span>\n\t\t\t\t\t<span class=\"input-text-wrap\"><input type=\"text\" name=\"slug\" class=\"ptitle\" value=\"\" /></span>\n\t\t\t\t</label>\n\t\t\t\t</div>\n\t\t\t</fieldset>\n\n\t\t\t\n\t\t\t<div class=\"inline-edit-save submit\">\n\t\t\t\t<button type=\"button\" class=\"save button button-primary\">{update}</button>\n\t\t\t\t<button type=\"button\" class=\"cancel button\">{cancel}</button>\n\t\t\t\t<span class=\"spinner\"></span>\n\n\t\t\t\t<input type=\"hidden\" id=\"_inline_edit\" name=\"_inline_edit\" value=\"{nonce}\" />\t\t\t\t<input type=\"hidden\" name=\"taxonomy\" value=\"{tax}\" />\n\t\t\t\t<input type=\"hidden\" name=\"post_type\" value=\"{pt}\" />\n\n\t\t\t\t<div class=\"notice notice-error notice-alt inline hidden\"><p class=\"error\"></p></div>\t\t\t</div>\n\t\t\t</div>\n\n\t\t\t</td></tr>\n\n\t\t</tbody></table>\n\t\t</form>\n\t\t",
        quick = tr("Quick Edit"),
        name = tr("Name"),
        slug = tr("Slug"),
        update = info.labels.update_item,
        cancel = tr("Cancel"),
        nonce = ctx.create_nonce("taxinlineeditnonce"),
        tax = info.name,
        pt = info.post_type
    )
}

/// `term.php`: the form that edits one term.
pub async fn edit_page(ctx: &Ctx<'_>) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let taxonomy = request.request("taxonomy").unwrap_or_default().to_owned();
    let id: u64 = request.request("tag_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
    // Without a term the list of the taxonomy is the place to go to: the tags if none is named, none if it is unknown.
    if id == 0 {
        let target = if taxonomy.is_empty() {
            ctx.admin_url("edit-tags.php?taxonomy=post_tag")
        } else if tax_info(ctx, &taxonomy, request.request("post_type")).is_some() {
            ctx.admin_url(&format!("edit-tags.php?taxonomy={taxonomy}"))
        } else {
            ctx.admin_url("edit-tags.php")
        };
        return super::redirect(&target);
    }
    // No taxonomy named means the tags; an unknown one is no taxonomy at all: the term is looked up by its ID and brings its own.
    let taxonomy = if taxonomy.is_empty() {
        "post_tag".to_owned()
    } else if tax_info(ctx, &taxonomy, request.request("post_type")).is_some() {
        taxonomy
    } else {
        ctx.env.db.term_by_id(None, id).await.ok().flatten().map(|t| t.taxonomy).unwrap_or_default()
    };
    let Some(info) = tax_info(ctx, &taxonomy, request.request("post_type")) else {
        return super::common::die_plain(ctx, "You attempted to edit an item that does not exist. Perhaps it was deleted?");
    };
    if !ctx.can("manage_categories") {
        return forbidden_heading(ctx, "Sorry, you are not allowed to edit this item.");
    }
    let Some(term) = ctx.env.db.term_by_id(Some(&info.name), id).await.ok().flatten() else {
        return super::common::die_plain(ctx, "You attempted to edit an item that does not exist. Perhaps it was deleted?");
    };
    let site = ctx.site();

    // The form posts to edit-tags.php with `editedtag`; this screen is reached by GET only.
    let mut h = format!("<div class=\"wrap\">\n<h1>{}</h1>\n\n", info.labels.edit_item);
    if let Some(code) = request.get("message").and_then(|m| m.parse::<usize>().ok()).filter(|m| *m > 0) {
        let list = messages(ctx, &info);
        if let Some(text) = list.get(code) {
            let error = request.get("error").is_some() || code == 5;
            h.push_str(&format!("<div id=\"message\" class=\"{} notice is-dismissible\"><p>{text}</p></div>\n", if error { "notice-error" } else { "updated" }));
        }
    }
    let original = request.get("wp_http_referer").unwrap_or_default();
    h.push_str("\n<div id=\"ajax-response\"></div>\n\n");
    h.push_str(&format!(
        "<form name=\"edittag\" id=\"edittag\" method=\"post\" action=\"edit-tags.php\" class=\"validate\"\n>\n<input type=\"hidden\" name=\"action\" value=\"editedtag\" />\n<input type=\"hidden\" name=\"tag_ID\" value=\"{}\" />\n<input type=\"hidden\" name=\"taxonomy\" value=\"{}\" />\n<input type=\"hidden\" name=\"_wp_original_http_referer\" value=\"{}\" />{}",
        term.term_id,
        esc_attr(&info.name),
        esc_attr(original),
        ctx.nonce_field(&format!("update-tag_{}", term.term_id))
    ));
    h.push_str("\t<table class=\"form-table\" role=\"presentation\">\n");
    h.push_str(&format!(
        "\t\t<tr class=\"form-field form-required term-name-wrap\">\n\t\t\t<th scope=\"row\"><label for=\"name\">{}</label></th>\n\t\t\t<td><input name=\"name\" id=\"name\" type=\"text\" value=\"{}\" size=\"40\" aria-required=\"true\" aria-describedby=\"name-description\" />\n\t\t\t<p class=\"description\" id=\"name-description\">{}</p></td>\n\t\t</tr>\n",
        tr("Name"),
        esc_attr(&term.name),
        tr("The name is how it appears on your site.")
    ));
    h.push_str(&format!(
        "\t\t<tr class=\"form-field term-slug-wrap\">\n\t\t\t<th scope=\"row\"><label for=\"slug\">{}</label></th>\n\t\t\t\t\t\t<td><input name=\"slug\" id=\"slug\" type=\"text\" value=\"{}\" size=\"40\" aria-describedby=\"slug-description\" />\n\t\t\t<p class=\"description\" id=\"slug-description\">{}</p></td>\n\t\t</tr>\n",
        tr("Slug"),
        esc_attr(&term.slug),
        tr("The &#8220;slug&#8221; is the URL-friendly version of the name. It is usually all lowercase and contains only letters, numbers, and hyphens.")
    ));
    if info.hierarchical {
        let dropdown = dropdown_terms(ctx, &info.name, "parent", "parent", term.parent, "postform", Some(("-1", &tr("None"))), &[term.term_id]).await.replacen("class='postform'>", "class='postform' aria-describedby=\"parent-description\">", 1);
        let help_text = if info.name == "category" { tr("Categories, unlike tags, can have a hierarchy. You might have a Jazz category, and under that have children categories for Bebop and Big Band. Totally optional.") } else { tr("Assign a parent term to create a hierarchy. The term Jazz, for example, would be the parent of Bebop and Big Band.") };
        h.push_str(&format!(
            "\t\t<tr class=\"form-field term-parent-wrap\">\n\t\t\t<th scope=\"row\"><label for=\"parent\">{}</label></th>\n\t\t\t<td>\n\t\t\t\t{dropdown}\t\t\t\t\t\t\t\t\t<p class=\"description\" id=\"parent-description\">{help_text}</p>\n\t\t\t\t\t\t\t</td>\n\t\t</tr>\n",
            info.labels.parent_item.clone()
        ));
    }
    h.push_str(&format!(
        "\t\t<tr class=\"form-field term-description-wrap\">\n\t\t\t<th scope=\"row\"><label for=\"description\">{}</label></th>\n\t\t\t<td><textarea name=\"description\" id=\"description\" rows=\"5\" cols=\"50\" class=\"large-text\" aria-describedby=\"description-description\">{}</textarea>\n\t\t\t<p class=\"description\" id=\"description-description\">{}</p></td>\n\t\t</tr>\n\t\t\t</table>\n\n",
        tr("Description"),
        esc_textarea(&term.description),
        tr("The description is not prominent by default; however, some themes may show it.")
    ));
    let default = info.name == "category" && site.options.u64("default_category", 1) == term.term_id;
    h.push_str(&format!("<div class=\"edit-tag-actions\">\n\n\t<input type=\"submit\" class=\"button button-primary\" value=\"{}\"  />\n\t", tr("Update")));
    if !default {
        let url = ctx.admin_url(&format!("edit-tags.php?action=delete&taxonomy={}&tag_ID={}&_wpnonce={}", info.name, term.term_id, ctx.create_nonce(&format!("delete-tag_{}", term.term_id))));
        h.push_str(&format!("\t\t<span id=\"delete-link\">\n\t\t\t<a class=\"delete\" href=\"{}\">{}</a>\n\t\t</span>\n\t", esc_url(&url), tr("Delete")));
    }
    h.push_str("\n</div>\n\n</form>\n</div>\n\n<script>\ntry{document.forms.edittag.name.focus();}catch(e){}\n</script>\n\t\n");
    let title = info.labels.edit_item.clone();
    let mut screen = screen_for(&info, title, Help::default(), "term.php");
    if let Ok(link) = site.term_link(&term).await {
        screen.view_item = Some((info.labels.view_item.clone(), link));
    }
    // The empty help of the screen ends without a line break.
    render(ctx, &screen, &format!("\n{h}"), &Assets::default()).await
}

/// `edit-tags.php?action=editedtag`: saves the edit form of `term.php`.
async fn edited_tag(ctx: &Ctx<'_>, info: &TaxInfo) -> AdminResponse {
    let request = ctx.request;
    let tr = |s: &str| ctx.tr(s);
    let id: u64 = request.request("tag_ID").and_then(|v| v.parse().ok()).unwrap_or(0);
    if !ctx.verify_nonce(&format!("update-tag_{id}")) {
        return super::die(ctx.env, 403, &tr("An error occurred."), &tr("The link you followed has expired."));
    }
    let Some(writer) = ctx.env.writer else { return read_only(ctx) };
    let content = Content { site: ctx.site(), db: ctx.env.db, writer, user: &ctx.user, now: ctx.env.now };
    let data = TermData {
        name: request.post_field("name").unwrap_or_default().to_owned(),
        slug: request.post_field("slug").unwrap_or_default().to_owned(),
        parent: request.post_field("parent").and_then(|p| p.parse::<i64>().ok()).filter(|p| *p > 0).unwrap_or(0) as u64,
        description: request.post_field("description").unwrap_or_default().to_owned(),
    };
    let ok = content.update_term(id, &info.name, &data).await.is_ok();
    let original = request.post_field("_wp_original_http_referer").unwrap_or_default();
    let mut url = ctx.admin_url(&format!("term.php?taxonomy={}&tag_ID={id}&post_type={}", info.name, info.post_type));
    if !original.is_empty() {
        url = add_query_arg(&url, "wp_http_referer", original);
    }
    url = add_query_arg(&url, "message", if ok { "3" } else { "5" });
    if !ok {
        url = add_query_arg(&url, "error", "1");
    }
    redirect(&url)
}

/// The row HTML for the Ajax handlers (`add-tag`, `inline-save-tax`).
pub async fn row_html(ctx: &Ctx<'_>, taxonomy: &str, term: &Term, level: usize) -> Option<String> {
    let info = tax_info(ctx, taxonomy, None)?;
    Some(row(ctx, &info, term, level, level, "").await)
}
