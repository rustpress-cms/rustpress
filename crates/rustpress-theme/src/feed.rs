//! RSS 2.0 and Atom feeds (`feed-rss2.php`, `feed-atom.php` and their comment
//! variants), including the conditional-GET headers `WP::send_headers()` adds.
//!
//! The templates are reproduced literally: PHP prints the text outside
//! `<?php ... ?>` unchanged except for one newline directly after `?>`, which
//! is why the whitespace below looks odd. `tests/golden_feeds.rs` compares the
//! result byte for byte with WordPress.

use rustpress_core::i18n::sprintf;
use std::sync::Arc;

use chrono::NaiveDateTime;
use rustpress_blocks::content::{comment_text, get_the_content, post_password_required, the_content, the_title_for};
use rustpress_blocks::Env;
use rustpress_core::formatting::{convert_chars, ent2ncr, esc_attr, esc_html, esc_url, html_entity_decode, strip_tags};
use rustpress_core::urls::trailingslashit;
use rustpress_core::{Queried, QueryResult, Site};
use rustpress_db::{Comment, Post};

use crate::Result;
use crate::document::{document_title, wp_version};
use crate::theme::Theme;

pub struct Feed {
    pub content_type: String,
    pub body: String,
    /// `Last-Modified` (`D, d M Y H:i:s GMT`); absent for password protected posts.
    pub last_modified: Option<String>,
    /// The modification time conditional requests are compared with; set even when the response is not cacheable.
    modified_at: String,
    pub etag: String,
    /// Password protected posts get `wp_get_nocache_headers()`.
    pub nocache: bool,
}

impl Feed {
    /// The conditional-GET decision of `WP::send_headers()`.
    pub fn not_modified(&self, if_none_match: Option<&str>, if_modified_since: Option<&str>) -> bool {
        let timestamp = |s: &str| chrono::DateTime::parse_from_rfc2822(s.trim()).map(|d| d.timestamp()).unwrap_or(0);
        let ours = timestamp(&self.modified_at);
        let client = if_modified_since.map_or(0, timestamp);
        let etag_matches = if_none_match.is_some_and(|e| e == self.etag);
        if if_modified_since.is_some_and(|s| !s.trim().is_empty()) && if_none_match.is_some_and(|e| !e.is_empty()) {
            client >= ours && etag_matches
        } else {
            client >= ours || etag_matches
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Rss2,
    Atom,
}

fn parse_gmt(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
}

/// `mysql2date( 'D, d M Y H:i:s +0000', $gmt, false )` (also used for `r`).
fn rfc2822(gmt: &str) -> String {
    parse_gmt(gmt).map(|d| d.format("%a, %d %b %Y %H:%M:%S +0000").to_string()).unwrap_or_default()
}

/// `Y-m-d\TH:i:s\Z`.
fn atom_date(gmt: &str) -> String {
    parse_gmt(gmt).map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default()
}

/// `get_bloginfo_rss()` + `bloginfo_rss` filters.
fn bloginfo_rss(value: &str) -> String {
    ent2ncr(&convert_chars(&strip_tags(value)))
}

/// `get_comments_number()`-style counts come from the cached `comment_count`.
fn has_comment_section(post: &Post) -> bool {
    post.comment_count != 0 || post.comment_status == "open"
}

async fn comment_feed_link(site: &Site, post: &Post, feed: Kind) -> Result<String> {
    let permalink = site.permalink(post).await?;
    if site.uses_pretty_permalinks() {
        let mut url = format!("{}feed", trailingslashit(&permalink));
        if feed == Kind::Atom {
            url.push_str("/atom");
        }
        Ok(site.user_trailingslashit(&url))
    } else {
        let mut url = rustpress_core::urls::add_query_arg(&site.home_url(""), "feed", if feed == Kind::Atom { "atom" } else { "rss2" });
        url = rustpress_core::urls::add_query_arg(&url, "p", &post.id.to_string());
        Ok(url)
    }
}

struct Ctx<'a> {
    site: &'a Site,
    env: Env<'a>,
    kind: Kind,
    version: String,
    wp_root: std::path::PathBuf,
}

impl Ctx<'_> {
    fn lang(&self) -> String {
        self.site.language_tag()
    }

    fn title_rss(&self, post: &Post) -> String {
        esc_html(&strip_tags(&ent2ncr(&the_title_for(self.site, post))))
    }

    async fn author(&self, post: &Post) -> Result<Option<rustpress_db::User>> {
        Ok(self.site.db.user(post.post_author).await?)
    }

    /// `get_the_category_rss()` for RSS (`<category>` lines) or Atom (`<category scheme term />`).
    async fn categories(&self, post: &Post) -> Result<String> {
        let mut names: Vec<String> = Vec::new();
        for taxonomy in ["category", "post_tag"] {
            let mut terms = self.site.db.terms_for_post(post.id, Some(taxonomy)).await?;
            terms.sort_by_key(|t| (t.name.to_lowercase(), t.term_id));
            for term in terms {
                let name = match self.kind {
                    Kind::Atom => term.name,
                    Kind::Rss2 => convert_chars(&term.name),
                };
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        let mut list = String::new();
        for name in names {
            match self.kind {
                Kind::Atom => list.push_str(&format!("<category scheme=\"{}\" term=\"{}\" />", esc_attr(&bloginfo_rss(self.site.settings.home.trim_end_matches('/'))), esc_attr(&name))),
                Kind::Rss2 => list.push_str(&format!("\t\t<category><![CDATA[{}]]></category>\n", html_entity_decode(&name))),
            }
        }
        Ok(list)
    }

    /// `get_the_excerpt()` and the `the_excerpt_rss` filters.
    async fn excerpt(&self, post: &Arc<Post>) -> Result<String> {
        let text = rustpress_blocks::excerpt::get_the_excerpt(&self.env, post).await?;
        Ok(convert_chars(&ent2ncr(&text)))
    }
    /// `get_the_content_feed()`.
    async fn content(&self, post: &Arc<Post>) -> Result<String> {
        let raw = get_the_content(&self.env, post, 1, true);
        let content = the_content(&self.env, post, &raw).await?.replace("]]>", "]]&gt;");
        // `the_content_feed`: `wp_staticize_emoji()`.
        let formatting = std::fs::read_to_string(self.wp_root.join("wp-includes").join("formatting.php")).unwrap_or_default();
        Ok(rustpress_core::text::staticize_emoji(&content, &formatting))
    }

    fn enclosures(&self, meta: &[rustpress_db::PostMeta], post: &Post) -> String {
        let mut out = String::new();
        if post_password_required(post) {
            return out;
        }
        for m in meta.iter().filter(|m| m.meta_key.as_deref() == Some("enclosure")) {
            let value = m.meta_value.clone().unwrap_or_default();
            let lines: Vec<&str> = value.split('\n').collect();
            if lines.len() < 3 {
                continue;
            }
            let kind = lines[2].trim().split([' ', '\t']).next().unwrap_or("");
            let length: u64 = lines[1].trim().parse().unwrap_or(0);
            match self.kind {
                Kind::Rss2 => out.push_str(&format!("<enclosure url=\"{}\" length=\"{length}\" type=\"{}\" />\n", esc_url(lines[0].trim()), esc_attr(kind))),
                Kind::Atom => out.push_str(&format!("<link href=\"{}\" rel=\"enclosure\" length=\"{length}\" type=\"{}\" />\n", esc_url(lines[0].trim()), esc_attr(kind))),
            }
        }
        out
    }

    fn generator(&self) -> String {
        match self.kind {
            Kind::Rss2 => format!("<generator>https://wordpress.org/?v={}</generator>", self.version),
            Kind::Atom => format!("<generator uri=\"https://wordpress.org/\" version=\"{}\">WordPress</generator>", esc_attr(&self.version)),
        }
    }
}

fn comment_author(site: &Site, c: &Comment) -> String {
    if c.comment_author.is_empty() { site.tr("Anonymous") } else { c.comment_author.clone() }
}

/// `get_feed_build_date()`.
async fn build_date(site: &Site, q: &QueryResult, comments: &[Comment], comment_feed: bool) -> Result<String> {
    // A site-wide comment feed compares the posts its comments belong to.
    let mut times: Vec<String> = Vec::new();
    if comment_feed && !q.flags.is_singular() {
        let mut seen: Vec<u64> = Vec::new();
        for comment in comments {
            if !seen.contains(&comment.comment_post_id) {
                seen.push(comment.comment_post_id);
                if let Some(post) = site.db.post(comment.comment_post_id).await? {
                    times.push(post.post_modified_gmt);
                }
            }
        }
    } else {
        times.extend(q.posts.iter().map(|p| p.post_modified_gmt.clone()));
    }
    if !q.posts.is_empty() || (comment_feed && !comments.is_empty()) {
        if comment_feed {
            times.extend(comments.iter().map(|c| c.comment_date_gmt.clone()));
        }
        if let Some(max) = times.into_iter().max() {
            return Ok(max);
        }
    }
    Ok(site.db.last_post_modified_gmt(&site.types.public_post_types()).await?.unwrap_or_default())
}

/// Renders the feed for a request, or `None` for feed types that are not supported.
pub async fn render_feed(theme: &Theme, site: &Site, q: &QueryResult, registry: &rustpress_blocks::Registry, request_uri: &str) -> Result<Option<Feed>> {
    let feed_var = q.vars.get("feed").unwrap_or("feed");
    let kind = match feed_var.trim_start_matches("comments-") {
        "feed" | "rss2" | "rss" => Kind::Rss2,
        "atom" => Kind::Atom,
        _ => return Ok(None),
    };
    if q.flags.is_404 {
        return Ok(None);
    }
    let singular = q.flags.is_singular();
    let comment_feed = q.vars.get("withcomments").is_some_and(|v| !v.is_empty() && v != "0")
        || feed_var.contains("comments-")
        || (!q.vars.get("withoutcomments").is_some_and(|v| !v.is_empty() && v != "0") && singular);
    // Comment feeds of archives and searches are not supported.
    if comment_feed && !singular && (q.flags.is_archive || q.flags.is_search) {
        return Ok(None);
    }

    let ctx = Ctx { site, env: Env::new(site, q, theme, registry), kind, version: wp_version(&theme.wp_root), wp_root: theme.wp_root.clone() };
    // The feed templates run their `while ( have_posts() )` loop on the main query.
    ctx.env.with_state(|s| s.media.in_main_loop = true);
    let self_url = format!("{}{}", site.settings.home.trim_end_matches('/').split("://").next().map_or(String::new(), |s| format!("{s}://")) + site.settings.home.trim_end_matches('/').split("://").nth(1).unwrap_or("").split('/').next().unwrap_or(""), request_uri);
    let self_url = esc_url(&self_url);
    let home = bloginfo_rss(site.settings.home.trim_end_matches('/'));
    let description = bloginfo_rss(&site.settings.blogdescription);
    let lang = ctx.lang();
    let title = document_title(site, q).await?;

    let comments: Vec<Comment> = if comment_feed {
        let post_id = if singular { q.posts.first().map(|p| p.id) } else { None };
        if singular && post_id.is_none() {
            Vec::new()
        } else {
            site.db.feed_comments(post_id, site.settings.posts_per_rss).await?
        }
    } else {
        Vec::new()
    };
    let built = build_date(site, q, &comments, comment_feed).await?;

    let body = match (kind, comment_feed) {
        (Kind::Rss2, false) => rss2_posts(&ctx, q, &self_url, &home, &description, &lang, &title, &built).await?,
        (Kind::Atom, false) => atom_posts(&ctx, q, &self_url, &home, &description, &lang, &title, &built).await?,
        (Kind::Rss2, true) => rss2_comments(&ctx, q, &comments, &self_url, &home, &description, &title, &built).await?,
        (Kind::Atom, true) => atom_comments(&ctx, q, &comments, &self_url, &home, &description, &lang, &title, &built).await?,
    };

    // WP::send_headers()
    let with_comments = comment_feed
        || (q.vars.get("withoutcomments").is_none_or(|v| v.is_empty() || v == "0")
            && ["p", "name", "page_id", "pagename", "attachment", "attachment_id"].iter().any(|k| q.vars.get(k).is_some_and(|v| !v.is_empty() && v != "0")));
    let post_modified = site.db.last_post_modified_gmt(&site.types.public_post_types()).await?.unwrap_or_default();
    let newest = if with_comments {
        let comment_modified = site.db.last_comment_modified_gmt().await?.unwrap_or_default();
        if post_modified > comment_modified { post_modified } else { comment_modified }
    } else {
        post_modified
    };
    let last_modified = format!("{} GMT", parse_gmt(&newest).map(|d| d.format("%a, %d %b %Y %H:%M:%S").to_string()).unwrap_or_else(|| chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S").to_string()));
    let etag = format!("\"{:x}\"", md5::compute(last_modified.as_bytes()));
    let content_type = match kind {
        Kind::Rss2 => "application/rss+xml; charset=UTF-8",
        Kind::Atom => "application/atom+xml; charset=UTF-8",
    };
    let nocache = singular && q.posts.first().is_some_and(|p| !p.post_password.is_empty());
    Ok(Some(Feed { content_type: content_type.to_owned(), body, modified_at: last_modified.clone(), last_modified: (!nocache).then_some(last_modified), etag, nocache }))
}

#[allow(clippy::too_many_arguments)]
async fn rss2_posts(c: &Ctx<'_>, q: &QueryResult, self_url: &str, home: &str, description: &str, lang: &str, title: &str, built: &str) -> Result<String> {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"\n\txmlns:content=\"http://purl.org/rss/1.0/modules/content/\"\n\txmlns:wfw=\"http://wellformedweb.org/CommentAPI/\"\n\txmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n\txmlns:atom=\"http://www.w3.org/2005/Atom\"\n\txmlns:sy=\"http://purl.org/rss/1.0/modules/syndication/\"\n\txmlns:slash=\"http://purl.org/rss/1.0/modules/slash/\"\n\t>\n\n<channel>\n");
    out.push_str(&format!("\t<title>{title}</title>\n\t<atom:link href=\"{self_url}\" rel=\"self\" type=\"application/rss+xml\" />\n\t<link>{home}</link>\n\t<description>{description}</description>\n\t<lastBuildDate>{}</lastBuildDate>\n\t<language>{lang}</language>\n\t<sy:updatePeriod>\n\thourly\t</sy:updatePeriod>\n\t<sy:updateFrequency>\n\t1\t</sy:updateFrequency>\n\t", rfc2822(built)));
    out.push_str(&c.generator());
    out.push('\n');
    for post in &q.posts {
        let post = Arc::new(post.clone());
        let permalink = c.site.permalink(&post).await?;
        let comments_section = has_comment_section(&post);
        let meta = c.site.db.post_meta(post.id).await?;
        out.push_str(&format!("\t<item>\n\t\t<title>{}</title>\n\t\t<link>{}</link>\n\t\t", c.title_rss(&post), esc_url(&permalink)));
        if comments_section {
            let hash = if post.comment_count != 0 { "#comments" } else { "#respond" };
            out.push_str(&format!("\t\t\t<comments>{}</comments>\n\t\t", esc_url(&format!("{permalink}{hash}"))));
        }
        let author = c.author(&post).await?.map(|u| u.display_name).unwrap_or_default();
        out.push_str(&format!("\n\t\t<dc:creator><![CDATA[{author}]]></dc:creator>\n\t\t<pubDate>{}</pubDate>\n\t\t", rfc2822(&post.post_date_gmt)));
        out.push_str(&c.categories(&post).await?);
        out.push_str(&format!("\t\t<guid isPermaLink=\"false\">{}</guid>\n\n\t\t", esc_url(&post.guid)));
        let excerpt = c.excerpt(&post).await?;
        out.push_str(&format!("\t\t\t<description><![CDATA[{excerpt}]]></description>\n\t\t\t\t\t\t"));
        let content = c.content(&post).await?;
        if content.is_empty() {
            out.push_str(&format!("\t\t\t\t<content:encoded><![CDATA[{excerpt}]]></content:encoded>\n\t\t\t"));
        } else {
            out.push_str(&format!("\t\t\t\t<content:encoded><![CDATA[{content}]]></content:encoded>\n\t\t\t"));
        }
        out.push_str("\t\t\n\t\t");
        if comments_section {
            out.push_str(&format!(
                "\t\t\t<wfw:commentRss>{}</wfw:commentRss>\n\t\t\t<slash:comments>{}</slash:comments>\n\t\t",
                esc_url(&comment_feed_link(c.site, &post, Kind::Rss2).await?),
                post.comment_count
            ));
        }
        out.push_str("\n\t\t");
        out.push_str(&c.enclosures(&meta, &post));
        out.push_str("\n\t\t\t</item>\n\t");
    }
    out.push_str("</channel>\n</rss>\n");
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
async fn atom_posts(c: &Ctx<'_>, q: &QueryResult, self_url: &str, home: &str, description: &str, lang: &str, title: &str, built: &str) -> Result<String> {
    let atom_id = esc_url(&c.site.home_url("feed/atom/"));
    let _ = atom_id;
    let mut out = String::new();
    out.push_str(&format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><feed\n\txmlns=\"http://www.w3.org/2005/Atom\"\n\txmlns:thr=\"http://purl.org/syndication/thread/1.0\"\n\txml:lang=\"{lang}\"\n\t>\n\t<title type=\"text\">{title}</title>\n\t<subtitle type=\"text\">{description}</subtitle>\n\n\t<updated>{}</updated>\n\n\t<link rel=\"alternate\" type=\"text/html\" href=\"{home}\" />\n\t<id>{}</id>\n\t<link rel=\"self\" type=\"application/atom+xml\" href=\"{self_url}\" />\n\n\t", atom_date(built), c.site.home_url("feed/atom/")));
    out.push_str(&c.generator());
    out.push('\n');
    for post in &q.posts {
        let post = Arc::new(post.clone());
        let permalink = c.site.permalink(&post).await?;
        let meta = c.site.db.post_meta(post.id).await?;
        let user = c.author(&post).await?;
        let name = user.as_ref().map(|u| u.display_name.clone()).unwrap_or_default();
        out.push_str(&format!("\t<entry>\n\t\t<author>\n\t\t\t<name>{name}</name>\n\t\t\t"));
        if let Some(url) = user.as_ref().map(|u| u.user_url.clone()).filter(|u| !u.is_empty()) {
            out.push_str(&format!("\t\t\t\t<uri>{url}</uri>\n\t\t\t\t"));
        }
        out.push_str("\t\t</author>\n\n");
        out.push_str(&format!(
            "\t\t<title type=\"html\"><![CDATA[{}]]></title>\n\t\t<link rel=\"alternate\" type=\"text/html\" href=\"{}\" />\n\n\t\t<id>{}</id>\n\t\t<updated>{}</updated>\n\t\t<published>{}</published>\n\t\t",
            c.title_rss(&post),
            esc_url(&permalink),
            esc_url(&post.guid),
            atom_date(&post.post_modified_gmt),
            atom_date(&post.post_date_gmt)
        ));
        out.push_str(&c.categories(&post).await?);
        let excerpt = c.excerpt(&post).await?;
        out.push_str(&format!("\n\t\t<summary type=\"html\"><![CDATA[{excerpt}]]></summary>\n\n\t\t"));
        let content = c.content(&post).await?;
        out.push_str(&format!("\t\t\t<content type=\"html\" xml:base=\"{}\"><![CDATA[{content}]]></content>\n\t\t\n\t\t", esc_url(&permalink)));
        out.push_str(&c.enclosures(&meta, &post));
        if has_comment_section(&post) {
            let count = post.comment_count;
            out.push_str(&format!(
                "\t\t\t<link rel=\"replies\" type=\"text/html\" href=\"{0}#comments\" thr:count=\"{count}\" />\n\t\t\t<link rel=\"replies\" type=\"application/atom+xml\" href=\"{1}\" thr:count=\"{count}\" />\n\t\t\t<thr:total>{count}</thr:total>\n\t\t",
                esc_url(&permalink),
                esc_url(&comment_feed_link(c.site, &post, Kind::Atom).await?)
            ));
        }
        out.push_str("\t</entry>\n\t");
    }
    out.push_str("</feed>\n");
    Ok(out)
}

/// The password form shown in place of protected comments.
fn protected_form(c: &Ctx<'_>, post: &Post) -> String {
    rustpress_blocks::content::password_form(&c.env, post)
}

#[allow(clippy::too_many_arguments)]
async fn rss2_comments(c: &Ctx<'_>, q: &QueryResult, comments: &[Comment], self_url: &str, home: &str, description: &str, title: &str, built: &str) -> Result<String> {
    let singular = q.flags.is_singular();
    let post = if singular { q.posts.first().cloned() } else { None };
    let heading = match &post {
        Some(p) => ent2ncr(&sprintf(&c.site.tr("Comments on: %s"), &[&c.title_rss(p)])),
        None => ent2ncr(&sprintf(&c.site.tr("Comments for %s"), &[title])),
    };
    let link = match &post {
        Some(p) if q.flags.is_single => esc_url(&c.site.permalink(p).await?),
        _ => home.to_owned(),
    };
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"\n\txmlns:content=\"http://purl.org/rss/1.0/modules/content/\"\n\txmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n\txmlns:atom=\"http://www.w3.org/2005/Atom\"\n\txmlns:sy=\"http://purl.org/rss/1.0/modules/syndication/\"\n\t\n\t>\n<channel>\n");
    out.push_str(&format!("\t<title>\n\t{heading}\t</title>\n\t<atom:link href=\"{self_url}\" rel=\"self\" type=\"application/rss+xml\" />\n\t<link>{link}</link>\n\t<description>{description}</description>\n\t<lastBuildDate>{}</lastBuildDate>\n\t<sy:updatePeriod>\n\thourly\t</sy:updatePeriod>\n\t<sy:updateFrequency>\n\t1\t</sy:updateFrequency>\n\t", rfc2822(built)));
    out.push_str(&c.generator());
    out.push('\n');
    for comment in comments {
        let Some(comment_post) = c.site.db.post(comment.comment_post_id).await? else { continue };
        let _permalink = ();
        let comment_link = esc_url(&rustpress_blocks::comment_pages::comment_link(&c.env, comment, None).await?);
        let author = ent2ncr(&comment_author(c.site, comment));
        let item_title = if singular {
            ent2ncr(&sprintf(&c.site.tr("By: %s"), &[&author]))
        } else {
            ent2ncr(&sprintf(&c.site.tr("Comment on %1$s by %2$s"), &[&c.title_rss(&comment_post), &author]))
        };
        out.push_str(&format!(
            "\t<item>\n\t\t<title>\n\t\t{item_title}\t\t</title>\n\t\t<link>{comment_link}</link>\n\n\t\t<dc:creator><![CDATA[{author}]]></dc:creator>\n\t\t<pubDate>{}</pubDate>\n\t\t<guid isPermaLink=\"false\">{}</guid>\n\n\t\t",
            rfc2822(&comment.comment_date_gmt),
            esc_url(&format!("{}#comment-{}", comment_post.guid, comment.comment_id))
        ));
        if post_password_required(&comment_post) {
            out.push_str(&format!(
                "\t\t\t<description>{}</description>\n\t\t\t<content:encoded><![CDATA[{}]]></content:encoded>\n\t\t",
                ent2ncr(&c.site.tr("Protected Comments: Please enter your password to view comments.")),
                protected_form(c, &comment_post)
            ));
        } else {
            out.push_str(&format!(
                "\t\t\t<description><![CDATA[{}]]></description>\n\t\t\t<content:encoded><![CDATA[{}]]></content:encoded>\n\t\t",
                esc_html(&ent2ncr(&feed_comment_text(c, comment).await?)),
                comment_text(c.site, &feed_comment_text(c, comment).await?)
            ));
        }
        out.push_str("\n\t\t\t</item>\n\t");
    }
    out.push_str("</channel>\n</rss>\n");
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
async fn atom_comments(c: &Ctx<'_>, q: &QueryResult, comments: &[Comment], _self_url: &str, home: &str, description: &str, lang: &str, title: &str, built: &str) -> Result<String> {
    let singular = q.flags.is_singular();
    let post = if singular { q.posts.first().cloned() } else { None };
    let heading = match &post {
        Some(p) => ent2ncr(&sprintf(&c.site.tr("Comments on %s"), &[&c.title_rss(p)])),
        None => ent2ncr(&sprintf(&c.site.tr("Comments for %s"), &[title])),
    };
    let mut out = String::new();
    out.push_str(&format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?><feed\n\txmlns=\"http://www.w3.org/2005/Atom\"\n\txml:lang=\"{lang}\"\n\txmlns:thr=\"http://purl.org/syndication/thread/1.0\"\n\t>\n\t<title type=\"text\">\n\t{heading}\t</title>\n\t<subtitle type=\"text\">{description}</subtitle>\n\n\t<updated>{}</updated>\n\n",
        atom_date(built)
    ));
    match &post {
        Some(p) => {
            let permalink = c.site.permalink(p).await?;
            let hash = if p.comment_count != 0 { "#comments" } else { "#respond" };
            let feed_link = esc_url(&comment_feed_link(c.site, p, Kind::Atom).await?);
            out.push_str(&format!(
                "\t<link rel=\"alternate\" type=\"text/html\" href=\"{}\" />\n\t<link rel=\"self\" type=\"application/atom+xml\" href=\"{feed_link}\" />\n\t<id>{feed_link}</id>\n",
                esc_url(&format!("{permalink}{hash}"))
            ));
        }
        None => {
            let atom_url = bloginfo_rss(&c.site.user_trailingslashit(&c.site.home_url("comments/feed/atom")));
            out.push_str(&format!("\t<link rel=\"alternate\" type=\"text/html\" href=\"{home}\" />\n\t<link rel=\"self\" type=\"application/atom+xml\" href=\"{atom_url}\" />\n\t<id>{atom_url}</id>\n"));
        }
    }
    out.push_str(&format!("{}\n", c.generator()));
    for comment in comments {
        let Some(comment_post) = c.site.db.post(comment.comment_post_id).await? else { continue };
        let permalink = c.site.permalink(&comment_post).await?;
        let comment_link = esc_url(&rustpress_blocks::comment_pages::comment_link(&c.env, comment, None).await?);
        let author = ent2ncr(&comment_author(c.site, comment));
        let item_title = if singular {
            ent2ncr(&sprintf(&c.site.tr("By: %s"), &[&author]))
        } else {
            ent2ncr(&sprintf(&c.site.tr("Comment on %1$s by %2$s"), &[&c.title_rss(&comment_post), &author]))
        };
        out.push_str(&format!("\t<entry>\n\t\t<title>\n\t\t{item_title}\t\t</title>\n\t\t<link rel=\"alternate\" href=\"{comment_link}\" type=\"text/html\" />\n\n\t\t<author>\n\t\t\t<name>{author}</name>\n\t\t\t"));
        if !comment.comment_author_url.is_empty() {
            out.push_str(&format!("<uri>{}</uri>", comment.comment_author_url));
        }
        out.push_str(&format!(
            "\n\t\t</author>\n\n\t\t<id>{}</id>\n\t\t<updated>{}</updated>\n\t\t<published>{}</published>\n\n\t\t",
            esc_url(&format!("{}#comment-{}", comment_post.guid, comment.comment_id)),
            atom_date(&comment.comment_date_gmt),
            atom_date(&comment.comment_date_gmt)
        ));
        if post_password_required(&comment_post) {
            out.push_str(&format!("\t\t\t<content type=\"html\" xml:base=\"{comment_link}\"><![CDATA[{}]]></content>\n\t\t", protected_form(c, &comment_post)));
        } else {
            out.push_str(&format!("\t\t\t<content type=\"html\" xml:base=\"{comment_link}\"><![CDATA[{}]]></content>\n\t\t", comment_text(c.site, &feed_comment_text(c, comment).await?)));
        }
        out.push_str("\n\t\t");
        if comment.comment_parent == 0 {
            out.push_str(&format!(
                "\t\t\t<thr:in-reply-to ref=\"{}\" href=\"{}\" type=\"text/html\" />\n\t\t\t",
                esc_url(&comment_post.guid),
                esc_url(&permalink)
            ));
        } else if let Some(parent) = c.site.db.comment(comment.comment_parent).await? {
            out.push_str(&format!(
                "\t\t\t<thr:in-reply-to ref=\"{}\" href=\"{}\" type=\"text/html\" />\n\t\t\t",
                esc_url(&format!("{}#comment-{}", comment_post.guid, parent.comment_id)),
                esc_url(&rustpress_blocks::comment_pages::comment_link(&c.env, &parent, None).await?)
            ));
        }
        out.push_str("\t</entry>\n\t");
    }
    out.push_str("</feed>\n");
    Ok(out)
}

/// Whether the request is for the queried object of a feed (kept for symmetry with `Queried`).
#[allow(dead_code)]
fn queried_post(q: &QueryResult) -> Option<&Post> {
    match &q.queried {
        Queried::Post(p) => Some(p),
        _ => None,
    }
}


/// `get_comment_text()` in a comment feed: a reply starts with a link to its parent.
async fn feed_comment_text(c: &Ctx<'_>, comment: &rustpress_db::Comment) -> Result<String> {
    let mut text = comment.comment_content.clone();
    if comment.comment_parent != 0
        && let Some(parent) = c.site.db.comment(comment.comment_parent).await?
    {
        let link = esc_url(&rustpress_blocks::comment_pages::comment_link(&c.env, &parent, None).await?);
        let name = comment_author(c.site, &parent);
        text = format!("{}\n\n{text}", sprintf(&ent2ncr(&c.site.tr("In reply to %s.")), &[&format!("<a href=\"{link}\">{name}</a>")]));
    }
    Ok(text)
}