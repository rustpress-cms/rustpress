//! `wp-admin/export.php`: the form and the WXR file (`wp-admin/includes/export.php`).

use rustpress_core::datetime::SiteTimezone;
use rustpress_core::formatting::{convert_chars, ent2ncr, esc_attr, esc_html, esc_url, strip_tags};
use rustpress_db::{ExportFilter, Post, Term};

use super::{AdminResponse, Ctx, die, render};
use crate::chrome::{Assets, Screen};

/// The taxonomies WordPress knows itself (`_builtin`); their terms are not exported as custom terms.
const BUILTIN_TAXONOMIES: &[&str] = &["category", "post_tag", "nav_menu", "link_category", "post_format", "wp_theme", "wp_template_part_area", "wp_pattern_category"];
/// Post types that cannot be exported.
const NOT_EXPORTABLE: &[&str] = &["revision", "nav_menu_item", "oembed_cache", "user_request", "customize_changeset", "custom_css"];

/// `wxr_cdata()`.
fn cdata(text: &str) -> String {
    format!("<![CDATA[{}]]>", text.replace("]]>", "]]]]><![CDATA[>"))
}

/// The exportable post types that exist in the database, in the order of the type column.
async fn exportable_types(ctx: &Ctx<'_>) -> Vec<String> {
    let mut types = ctx.env.db.used_post_types().await.unwrap_or_default();
    types.retain(|t| !NOT_EXPORTABLE.contains(&t.as_str()));
    types.sort();
    types
}

/// `gmdate( 'Y-m-d', strtotime( 'YYYY-MM' ) )`.
fn month_start(month: &str) -> Option<String> {
    let (year, mon) = month.split_once('-')?;
    let (year, mon): (u32, u32) = (year.parse().ok()?, mon.parse().ok()?);
    ((1..=12).contains(&mon)).then(|| format!("{year:04}-{mon:02}-01"))
}

/// The first day of the month after `YYYY-MM`.
fn next_month_start(month: &str) -> Option<String> {
    let (year, mon) = month.split_once('-')?;
    let (mut year, mut mon): (u32, u32) = (year.parse().ok()?, mon.parse().ok()?);
    if !(1..=12).contains(&mon) {
        return None;
    }
    mon += 1;
    if mon == 13 {
        mon = 1;
        year += 1;
    }
    Some(format!("{year:04}-{mon:02}-01"))
}

/// `D, d M Y H:i:s +0000` of a GMT datetime.
fn rfc2822(gmt: &str) -> String {
    match chrono::NaiveDateTime::parse_from_str(gmt, "%Y-%m-%d %H:%M:%S") {
        Ok(dt) => dt.format("%a, %d %b %Y %H:%M:%S +0000").to_string(),
        // `mysql2date()` has no date for `0000-00-00 00:00:00`.
        Err(_) => String::new(),
    }
}

fn bloginfo_rss(text: &str) -> String {
    ent2ncr(&convert_chars(&strip_tags(text)))
}

/// `sanitize_url()`: `esc_url()` for storing.
fn sanitize_url(url: &str) -> String {
    esc_url(url).replace("&#038;", "&").replace("&#039;", "'")
}

async fn meta_pairs_xml(pairs: &[(Option<String>, Option<String>)], term: bool) -> String {
    let mut out = String::new();
    for (key, value) in pairs {
        let (key, value) = (key.clone().unwrap_or_default(), value.clone().unwrap_or_default());
        if term {
            out.push_str(&format!("\t\t<wp:termmeta>\n\t\t\t<wp:meta_key>{}</wp:meta_key>\n\t\t\t<wp:meta_value>{}</wp:meta_value>\n\t\t</wp:termmeta>\n", cdata(&key), cdata(&value)));
        }
    }
    out
}

/// The arguments of `export_wp()`.
#[derive(Default)]
pub struct ExportArgs {
    pub content: String,
    pub author: Option<u64>,
    pub category: Option<u64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
}

/// The file name and the contents of the export.
pub async fn export_wp(ctx: &Ctx<'_>, args: &ExportArgs) -> (String, String) {
    let site = ctx.site();
    let db = ctx.env.db;
    let now = ctx.env.now;
    let mut sitename: String = rustpress_blocks::style_engine::sanitize_key(&bloginfo_name(site));
    if !sitename.is_empty() {
        sitename.push('.');
    }
    let filename = format!("{sitename}WordPress.{}.xml", SiteTimezone::utc().format("Y-m-d", now));

    let all_types = exportable_types(ctx).await;
    let content = args.content.as_str();
    let single = content != "all" && all_types.iter().any(|t| t == content);
    let post_types: Vec<String> = if single { vec![content.to_owned()] } else { all_types.clone() };

    let mut filter = ExportFilter { post_types, ..Default::default() };
    if args.status.is_some() && (content == "post" || content == "page") {
        filter.status = args.status.clone();
    }
    let mut category_term: Option<Term> = None;
    if let (Some(cat), "post") = (args.category, content) {
        if let Ok(Some(term)) = db.term_by_id(Some("category"), cat).await {
            filter.category = Some(term.term_taxonomy_id);
            category_term = Some(term);
        }
    }
    if matches!(content, "post" | "page" | "attachment") {
        filter.author = args.author;
        filter.start = args.start_date.as_deref().and_then(month_start);
        filter.end = args.end_date.as_deref().and_then(next_month_start);
    }
    let mut post_ids = match db.export_post_ids(&filter).await {
        Ok(ids) => ids,
        Err(e) => {
            eprintln!("export: reading the posts failed: {e}");
            Vec::new()
        }
    };

    if content != "all" && content != "attachment" {
        let mut additional: Vec<u64> = Vec::new();
        for chunk in post_ids.chunks(20) {
            additional.extend(db.export_attachment_ids(chunk).await.unwrap_or_default());
        }
        post_ids.extend(additional);
        let mut seen = std::collections::HashSet::new();
        post_ids.retain(|id| seen.insert(*id));
    }

    // Terms: the category of a category export, or everything for "all".
    let mut cats: Vec<Term> = Vec::new();
    let mut tags: Vec<Term> = Vec::new();
    let mut terms: Vec<Term> = Vec::new();
    if let Some(term) = category_term {
        cats.push(term);
    } else if content == "all" {
        let mut categories = db.export_terms(&["category"], &[]).await.unwrap_or_default();
        tags = db.export_terms(&["post_tag"], &[]).await.unwrap_or_default();
        let mut custom = db.export_terms(&[], BUILTIN_TAXONOMIES).await.unwrap_or_default();
        // Parents come first.
        let mut guard = categories.len() * categories.len() + 1;
        while !categories.is_empty() && guard > 0 {
            guard -= 1;
            let cat = categories.remove(0);
            if cat.parent == 0 || cats.iter().any(|c| c.term_id == cat.parent) {
                cats.push(cat);
            } else {
                categories.push(cat);
            }
        }
        let mut guard = custom.len() * custom.len() + 1;
        while !custom.is_empty() && guard > 0 {
            guard -= 1;
            let t = custom.remove(0);
            if t.parent == 0 || terms.iter().any(|c| c.term_id == t.parent) {
                terms.push(t);
            } else {
                custom.push(t);
            }
        }
    }

    let version = &ctx.env.loader.version;
    let home = site.settings.home.trim_end_matches('/').to_owned();
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
    out.push_str("<!-- This is a WordPress eXtended RSS file generated by WordPress as an export of your site. -->\n<!-- It contains information about your site's posts, pages, comments, categories, and other content. -->\n<!-- You may use this file to transfer that content from one site to another. -->\n<!-- This file is not intended to serve as a complete backup of your site. -->\n\n<!-- To import this information into a WordPress site follow these steps: -->\n<!-- 1. Log in to that site as an administrator. -->\n<!-- 2. Go to Tools: Import in the WordPress admin panel. -->\n<!-- 3. Install the \"WordPress\" importer from the list. -->\n<!-- 4. Activate & Run Importer. -->\n<!-- 5. Upload this file using the form provided on that page. -->\n<!-- 6. You will first be asked to map the authors in this export file to users -->\n<!--    on the site. For each author, you may choose to map to an -->\n<!--    existing user on the site or to create a new user. -->\n<!-- 7. WordPress will then import each of the posts, pages, comments, categories, etc. -->\n<!--    contained in this file into your site. -->\n\n");
    out.push_str(&format!("\t<!-- generator=\"WordPress/{version}\" created=\"{}\" -->\n", SiteTimezone::utc().format("Y-m-d H:i", now)));
    out.push_str("<rss version=\"2.0\"\n\txmlns:excerpt=\"http://wordpress.org/export/1.2/excerpt/\"\n\txmlns:content=\"http://purl.org/rss/1.0/modules/content/\"\n\txmlns:wfw=\"http://wellformedweb.org/CommentAPI/\"\n\txmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n\txmlns:wp=\"http://wordpress.org/export/1.2/\"\n>\n\n<channel>\n");
    let language = site.locale().replace('_', "-");
    out.push_str(&format!(
        "\t<title>{}</title>\n\t<link>{}</link>\n\t<description>{}</description>\n\t<pubDate>{}</pubDate>\n\t<language>{language}</language>\n\t<wp:wxr_version>1.2</wp:wxr_version>\n\t<wp:base_site_url>{}</wp:base_site_url>\n\t<wp:base_blog_url>{}</wp:base_blog_url>\n\n\t",
        bloginfo_rss(&site.options.string("blogname", "")),
        bloginfo_rss(&home),
        bloginfo_rss(&site.options.string("blogdescription", "")),
        SiteTimezone::utc().format("D, d M Y H:i:s +0000", now),
        bloginfo_rss(&home),
        bloginfo_rss(&home),
    ));

    // Authors.
    let author_scope: &[u64] = if post_ids.is_empty() { &[] } else { &post_ids };
    for author_id in db.export_authors(author_scope).await.unwrap_or_default() {
        let Ok(Some(user)) = db.user(author_id).await else { continue };
        let (first, last) = db.user_names(author_id).await.unwrap_or_default();
        out.push_str(&format!(
            "\t<wp:author><wp:author_id>{}</wp:author_id><wp:author_login>{}</wp:author_login><wp:author_email>{}</wp:author_email><wp:author_display_name>{}</wp:author_display_name><wp:author_first_name>{}</wp:author_first_name><wp:author_last_name>{}</wp:author_last_name></wp:author>\n",
            user.id,
            cdata(&user.user_login),
            cdata(&user.user_email),
            cdata(&user.display_name),
            cdata(&first),
            cdata(&last)
        ));
    }
    out.push_str("\n\t");

    for c in &cats {
        let parent = if c.parent != 0 { cats.iter().find(|p| p.term_id == c.parent).map(|p| p.slug.clone()).unwrap_or_default() } else { String::new() };
        out.push_str(&format!("\t<wp:category>\n\t\t<wp:term_id>{}</wp:term_id>\n\t\t<wp:category_nicename>{}</wp:category_nicename>\n\t\t<wp:category_parent>{}</wp:category_parent>\n\t\t", c.term_id, cdata(&c.slug), cdata(&parent)));
        if !c.name.is_empty() {
            out.push_str(&format!("<wp:cat_name>{}</wp:cat_name>\n", cdata(&c.name)));
        }
        if !c.description.is_empty() {
            out.push_str(&format!("<wp:category_description>{}</wp:category_description>\n", cdata(&c.description)));
        }
        out.push_str(&meta_pairs_xml(&db.term_meta_pairs(c.term_id).await.unwrap_or_default(), true).await);
        out.push_str("\t</wp:category>\n\t");
    }
    out.push('\t');
    for t in &tags {
        out.push_str(&format!("\t<wp:tag>\n\t\t<wp:term_id>{}</wp:term_id>\n\t\t<wp:tag_slug>{}</wp:tag_slug>\n\t\t", t.term_id, cdata(&t.slug)));
        if !t.name.is_empty() {
            out.push_str(&format!("<wp:tag_name>{}</wp:tag_name>\n", cdata(&t.name)));
        }
        if !t.description.is_empty() {
            out.push_str(&format!("<wp:tag_description>{}</wp:tag_description>\n", cdata(&t.description)));
        }
        out.push_str(&meta_pairs_xml(&db.term_meta_pairs(t.term_id).await.unwrap_or_default(), true).await);
        out.push_str("\t</wp:tag>\n\t");
    }
    out.push('\t');
    for t in &terms {
        let parent = if t.parent != 0 { terms.iter().find(|p| p.term_id == t.parent).map(|p| p.slug.clone()).unwrap_or_default() } else { String::new() };
        out.push_str(&format!("\t<wp:term>\n\t\t<wp:term_id>{}</wp:term_id>\n\t\t<wp:term_taxonomy>{}</wp:term_taxonomy>\n\t\t<wp:term_slug>{}</wp:term_slug>\n\t\t<wp:term_parent>{}</wp:term_parent>\n\t\t", t.term_id, cdata(&t.taxonomy), cdata(&t.slug), cdata(&parent)));
        if !t.name.is_empty() {
            out.push_str(&format!("<wp:term_name>{}</wp:term_name>\n", cdata(&t.name)));
        }
        if !t.description.is_empty() {
            out.push_str(&format!("\t\t<wp:term_description>{}</wp:term_description>\n", cdata(&t.description)));
        }
        out.push_str(&meta_pairs_xml(&db.term_meta_pairs(t.term_id).await.unwrap_or_default(), true).await);
        out.push_str("\t</wp:term>\n\t");
    }
    out.push('\t');
    if content == "all" {
        for menu in db.export_terms(&["nav_menu"], &[]).await.unwrap_or_default() {
            out.push_str(&format!("\t<wp:term><wp:term_id>{}</wp:term_id><wp:term_taxonomy>nav_menu</wp:term_taxonomy><wp:term_slug>{}</wp:term_slug>", menu.term_id, cdata(&menu.slug)));
            if !menu.name.is_empty() {
                out.push_str(&format!("<wp:term_name>{}</wp:term_name>\n", cdata(&menu.name)));
            }
            out.push_str("</wp:term>\n");
        }
    }
    out.push_str(&format!("\n\t<generator>https://wordpress.org/?v={version}</generator>\n\n\t"));

    if !post_ids.is_empty() {
        for chunk in post_ids.chunks(20) {
            for post in db.posts_by_ids(chunk).await.unwrap_or_default() {
                out.push_str(&item(ctx, &post).await);
            }
        }
    }
    out.push_str("</channel>\n</rss>\n\t");
    (filename, out)
}

fn bloginfo_name(site: &rustpress_core::Site) -> String {
    bloginfo_rss(&site.options.string("blogname", ""))
}

/// One `<item>`.
async fn item(ctx: &Ctx<'_>, post: &Post) -> String {
    let site = ctx.site();
    let db = ctx.env.db;
    let permalink = site.permalink(post).await.unwrap_or_default();
    let creator = db.user(post.post_author).await.ok().flatten().map(|u| u.user_login).unwrap_or_default();
    let sticky = {
        let raw = site.options.string("sticky_posts", "");
        matches!(rustpress_phpser::unserialize(raw.as_bytes()), Ok(rustpress_phpser::Value::Array(items)) if items.iter().any(|(_, v)| v.as_i64() == Some(post.id as i64)))
    };
    let mut out = format!(
        "\t<item>\n\t\t<title>{}</title>\n\t\t<link>{}</link>\n\t\t<pubDate>{}</pubDate>\n\t\t<dc:creator>{}</dc:creator>\n\t\t<guid isPermaLink=\"false\">{}</guid>\n\t\t<description></description>\n\t\t<content:encoded>{}</content:encoded>\n\t\t<excerpt:encoded>{}</excerpt:encoded>\n\t\t<wp:post_id>{}</wp:post_id>\n\t\t<wp:post_date>{}</wp:post_date>\n\t\t<wp:post_date_gmt>{}</wp:post_date_gmt>\n\t\t<wp:post_modified>{}</wp:post_modified>\n\t\t<wp:post_modified_gmt>{}</wp:post_modified_gmt>\n\t\t<wp:comment_status>{}</wp:comment_status>\n\t\t<wp:ping_status>{}</wp:ping_status>\n\t\t<wp:post_name>{}</wp:post_name>\n\t\t<wp:status>{}</wp:status>\n\t\t<wp:post_parent>{}</wp:post_parent>\n\t\t<wp:menu_order>{}</wp:menu_order>\n\t\t<wp:post_type>{}</wp:post_type>\n\t\t<wp:post_password>{}</wp:post_password>\n\t\t<wp:is_sticky>{}</wp:is_sticky>\n",
        cdata(&post.post_title),
        esc_url(&permalink),
        rfc2822(&post.post_date_gmt),
        cdata(&creator),
        esc_url(&post.guid),
        cdata(&post.post_content),
        cdata(&post.post_excerpt),
        post.id,
        cdata(&post.post_date),
        cdata(&post.post_date_gmt),
        cdata(&post.post_modified),
        cdata(&post.post_modified_gmt),
        cdata(&post.comment_status),
        cdata(&post.ping_status),
        cdata(&post.post_name),
        cdata(&post.post_status),
        post.post_parent,
        post.menu_order,
        cdata(&post.post_type),
        cdata(&post.post_password),
        u8::from(sticky)
    );
    let metas = db.post_meta(post.id).await.unwrap_or_default();
    out.push_str("\t\t\t\t");
    if post.post_type == "attachment" {
        let attached = metas.iter().find(|m| m.meta_key.as_deref() == Some("_wp_attached_file")).and_then(|m| m.meta_value.clone()).unwrap_or_default();
        out.push_str(&format!("\t\t<wp:attachment_url>{}</wp:attachment_url>\n\t", cdata(&rustpress_blocks::images::attachment_url(site, &attached))));
    }
    out.push_str("\t\t\t\t");
    let mut terms = db.terms_for_post(post.id, None).await.unwrap_or_default();
    terms.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.term_id.cmp(&b.term_id)));
    for term in &terms {
        out.push_str(&format!("\t\t<category domain=\"{}\" nicename=\"{}\">{}</category>\n", term.taxonomy, term.slug, cdata(&term.name)));
    }
    out.push_str("\t\t\t\t");
    for meta in &metas {
        let key = meta.meta_key.clone().unwrap_or_default();
        if key == "_edit_lock" {
            continue;
        }
        out.push_str(&format!("\t\t<wp:postmeta>\n\t\t<wp:meta_key>{}</wp:meta_key>\n\t\t<wp:meta_value>{}</wp:meta_value>\n\t\t</wp:postmeta>\n\t\t\t\t\t", cdata(&key), cdata(meta.meta_value.as_deref().unwrap_or_default())));
    }
    for c in db.export_comments(post.id).await.unwrap_or_default() {
        out.push_str(&format!(
            "\t\t<wp:comment>\n\t\t\t<wp:comment_id>{}</wp:comment_id>\n\t\t\t<wp:comment_author>{}</wp:comment_author>\n\t\t\t<wp:comment_author_email>{}</wp:comment_author_email>\n\t\t\t<wp:comment_author_url>{}</wp:comment_author_url>\n\t\t\t<wp:comment_author_IP>{}</wp:comment_author_IP>\n\t\t\t<wp:comment_date>{}</wp:comment_date>\n\t\t\t<wp:comment_date_gmt>{}</wp:comment_date_gmt>\n\t\t\t<wp:comment_content>{}</wp:comment_content>\n\t\t\t<wp:comment_approved>{}</wp:comment_approved>\n\t\t\t<wp:comment_type>{}</wp:comment_type>\n\t\t\t<wp:comment_parent>{}</wp:comment_parent>\n\t\t\t<wp:comment_user_id>{}</wp:comment_user_id>\n\t\t\t\t\t",
            c.comment_id,
            cdata(&c.comment_author),
            cdata(&c.comment_author_email),
            sanitize_url(&c.comment_author_url),
            cdata(&c.comment_author_ip),
            cdata(&c.comment_date),
            cdata(&c.comment_date_gmt),
            cdata(&c.comment_content),
            cdata(&c.comment_approved),
            cdata(&c.comment_type),
            c.comment_parent,
            c.user_id
        ));
        for (key, value) in db.comment_meta_pairs(c.comment_id).await.unwrap_or_default() {
            out.push_str(&format!("\t<wp:commentmeta>\n\t<wp:meta_key>{}</wp:meta_key>\n\t\t\t<wp:meta_value>{}</wp:meta_value>\n\t\t\t</wp:commentmeta>\n\t\t\t\t\t", cdata(&key.unwrap_or_default()), cdata(&value.unwrap_or_default())));
        }
        out.push_str("\t\t</wp:comment>\n\t\t\t");
    }
    out.push_str("\t\t</item>\n\t\t\t\t");
    out
}

/// A `<select>` of the months that have posts of the type (`export_date_options()`).
async fn date_options(ctx: &Ctx<'_>, post_type: &str) -> String {
    let months = ctx.env.db.export_months(post_type).await.unwrap_or_default();
    let site = ctx.site();
    let mut out = String::new();
    for (year, month) in months {
        let ts = SiteTimezone::utc().local_timestamp(&format!("{year:04}-{month:02}-01 00:00:00")).unwrap_or(0);
        out.push_str(&format!("<option value=\"{year:04}-{month:02}\">{}</option>", esc_html(&site.date(&ctx.tr("F Y"), ts))));
    }
    out
}

/// `wp_dropdown_users()` for the authors of a type.
async fn author_dropdown(ctx: &Ctx<'_>, post_type: &str, name: &str) -> String {
    let db = ctx.env.db;
    let mut out = format!("<select name='{name}' class=''>\n\t<option value='0'>{}</option>\n", ctx.tr("All"));
    for user in db.export_authors_of_type(post_type).await.unwrap_or_default() {
        out.push_str(&format!("\t<option value='{}'>{} ({})</option>\n", user.id, esc_html(&user.display_name), esc_html(&user.user_login)));
    }
    out.push_str("</select>");
    out
}

/// `wp_dropdown_categories()` with its defaults: flat, by ID, categories without posts left out.
async fn category_dropdown(ctx: &Ctx<'_>) -> String {
    let mut categories = ctx.env.db.terms("category").await.unwrap_or_default();
    categories.retain(|c| c.count > 0);
    categories.sort_by_key(|c| c.term_id);
    let mut out = format!("<select  name='cat' id='cat' class='postform'>\n\t<option value='0' selected='selected'>{}</option>\n", ctx.tr("All"));
    for t in &categories {
        out.push_str(&format!("\t<option class=\"level-0\" value=\"{}\">{}</option>\n", t.term_id, esc_html(&t.name)));
    }
    out.push_str("</select>");
    out
}

pub async fn page(ctx: &Ctx<'_>) -> AdminResponse {
    let tr = |s: &str| ctx.tr(s);
    if !ctx.can("export") {
        return die(ctx.env, 403, "WordPress &rsaquo; Error", &format!("<h1>{}</h1><p>{}</p>", tr("You need a higher level of permission."), tr("Sorry, you are not allowed to export the content of this site.")));
    }
    let request = ctx.request;
    if request.get("download").is_some() {
        let get = |name: &str| request.get(name).filter(|v| !v.is_empty() && *v != "0").map(str::to_owned);
        let number = |name: &str| get(name).and_then(|v| v.parse::<u64>().ok()).filter(|v| *v > 0);
        let dates = |kind: &str| {
            let (start, end) = (get(&format!("{kind}_start_date")), get(&format!("{kind}_end_date")));
            if start.is_some() || end.is_some() { (start, end) } else { (None, None) }
        };
        let args = match request.get("content").unwrap_or("all") {
            "all" | "" => ExportArgs { content: "all".into(), ..Default::default() },
            "posts" => {
                let (start_date, end_date) = dates("post");
                ExportArgs { content: "post".into(), category: number("cat"), author: number("post_author"), start_date, end_date, status: get("post_status") }
            }
            "pages" => {
                let (start_date, end_date) = dates("page");
                ExportArgs { content: "page".into(), category: None, author: number("page_author"), start_date, end_date, status: get("page_status") }
            }
            "attachment" => {
                let (start_date, end_date) = dates("attachment");
                ExportArgs { content: "attachment".into(), start_date, end_date, ..Default::default() }
            }
            other => ExportArgs { content: other.to_owned(), ..Default::default() },
        };
        let (filename, xml) = export_wp(ctx, &args).await;
        let mut response = super::ajax::text(200, &xml);
        response.headers = vec![
            ("Content-Description".into(), "File Transfer".into()),
            ("Content-Disposition".into(), format!("attachment; filename={filename}")),
            ("Content-Type".into(), "text/xml; charset=UTF-8".into()),
        ];
        response.headers.extend(super::page_headers().into_iter().filter(|(k, _)| !k.eq_ignore_ascii_case("content-type")));
        return response;
    }

    let baked = super::pages::load(ctx, "export");
    let mut content = baked.content;
    content = content.replace("{{category_dropdown}}", &category_dropdown(ctx).await);
    content = content.replace("{{authors:post}}", &author_dropdown(ctx, "post", "post_author").await);
    content = content.replace("{{authors:page}}", &author_dropdown(ctx, "page", "page_author").await);
    for kind in ["post", "page", "attachment"] {
        content = content.replace(&format!("{{{{months:{kind}}}}}"), &date_options(ctx, kind).await);
    }
    let mut custom = String::new();
    const CORE_TYPES: &[&str] = &["post", "page", "attachment", "wp_block", "wp_template", "wp_template_part", "wp_navigation", "wp_global_styles", "wp_font_family", "wp_font_face"];
    for name in exportable_types(ctx).await.iter().filter(|n| !CORE_TYPES.contains(&n.as_str())) {
        let label = ctx.site().types.post_type(name).or_else(|| ctx.site().types.rewrite_post_types().iter().find(|t| &t.name == name)).and_then(|t| t.label.clone()).unwrap_or_else(|| name.clone());
        custom.push_str(&format!("<p><label><input type=\"radio\" name=\"content\" value=\"{}\" /> {}</label></p>
", esc_attr(name), esc_html(&label)));
    }
    let content = content.replace("{{custom_types}}\n", &custom);
    let screen = Screen { file: "export.php".into(), parent_file: "tools.php".into(), submenu_file: Some("export.php".into()), title: tr("Export"), help: Some(baked.help), ..Default::default() };
    render(ctx, &screen, &format!("{}\n", content.trim_end_matches('\n')), &Assets::default()).await
}
