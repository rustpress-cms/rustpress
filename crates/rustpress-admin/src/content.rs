//! The write side of posts: `wp_insert_post()`, trashing, deleting, terms and revisions, ported from
//! `wp-includes/post.php` and `taxonomy.php`.

use rustpress_auth::CurrentUser;
use rustpress_core::Site;
use rustpress_core::slug::sanitize_title;
use rustpress_db::{Db, Post, SlugScope, Val, Writer};
use rustpress_phpser::{Key, Value};

mod comments;
mod terms;
mod users;
pub use comments::CommentUpdate;
pub use terms::TermData;
pub use users::UserData;

/// What goes wrong when writing content.
#[derive(Debug)]
pub enum ContentError {
    /// `WP_Error` with a message for the user.
    Message(String),
    Db(rustpress_db::Error),
}

impl From<rustpress_db::Error> for ContentError {
    fn from(e: rustpress_db::Error) -> Self {
        ContentError::Db(e)
    }
}

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentError::Message(m) => write!(f, "{m}"),
            ContentError::Db(e) => write!(f, "{e}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, ContentError>;

/// A term given by ID or by name (tags are given by name).
#[derive(Debug, Clone)]
pub enum TermRef {
    Id(u64),
    Name(String),
}

/// The fields of `wp_insert_post( $postarr )`; `None` keeps the stored value on update and takes the default on insert.
#[derive(Debug, Clone, Default)]
pub struct PostData {
    pub id: Option<u64>,
    pub post_author: Option<u64>,
    pub post_content: Option<String>,
    pub post_title: Option<String>,
    pub post_excerpt: Option<String>,
    pub post_status: Option<String>,
    pub post_type: Option<String>,
    pub comment_status: Option<String>,
    pub ping_status: Option<String>,
    pub post_password: Option<String>,
    pub post_name: Option<String>,
    pub post_parent: Option<u64>,
    pub menu_order: Option<i32>,
    pub post_date: Option<String>,
    pub post_date_gmt: Option<String>,
    pub post_mime_type: Option<String>,
    pub guid: Option<String>,
    /// Category IDs (`post_category`).
    pub post_category: Option<Vec<u64>>,
    pub tags_input: Option<Vec<String>>,
    /// `tax_input`: taxonomy and its terms.
    pub tax_input: Vec<(String, Vec<TermRef>)>,
    pub meta_input: Vec<(String, String)>,
    pub page_template: Option<String>,
    pub thumbnail_id: Option<i64>,
    /// An edit of the date by the user (`edit_date`).
    pub edit_date: bool,
}

/// The services a write needs.
pub struct Content<'a> {
    pub site: &'a Site,
    pub db: &'a Db,
    pub writer: &'a Writer,
    pub user: &'a CurrentUser,
    pub now: i64,
}

const DATE_FLOATING: [&str; 3] = ["draft", "pending", "auto-draft"];

/// `normalize_whitespace()`.
fn normalize_whitespace(text: &str) -> String {
    text.trim().replace("\r\n", "\n").replace('\r', "\n").split(['\t', ' ']).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ").replace(" \n", "\n").replace("\n ", "\n")
}

/// `_truncate_post_slug()`.
fn truncate_slug(slug: &str, length: usize) -> String {
    if slug.len() <= length {
        return slug.to_owned();
    }
    let mut end = length;
    while !slug.is_char_boundary(end) {
        end -= 1;
    }
    slug[..end].trim_end_matches('-').to_owned()
}

/// Strips what a user without `unfiltered_html` may not post (a coarse `wp_kses_post()`).
pub fn kses_post(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_lowercase();
    let bytes = html.as_bytes();
    let mut i = 0;
    let blocked = ["script", "style", "iframe", "object", "embed", "form", "meta", "link", "base", "applet"];
    while i < bytes.len() {
        if bytes[i] == b'<' && i + 1 < bytes.len() {
            let rest = &lower[i + 1..];
            let name: String = rest.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            if blocked.contains(&name.as_str()) {
                // Drop the element up to the matching close tag or the end of the opening tag.
                if !rest.starts_with('/')
                    && let Some(close) = lower[i..].find(&format!("</{name}")) {
                        let end = lower[i + close..].find('>').map_or(html.len(), |e| i + close + e + 1);
                        i = end;
                        continue;
                    }
                i = lower[i..].find('>').map_or(html.len(), |e| i + e + 1);
                continue;
            }
            // Remove `on...=` attributes and `javascript:` URLs from the tag.
            if let Some(end) = html[i..].find('>') {
                let tag = &html[i..i + end + 1];
                out.push_str(&clean_tag(tag));
                i += end + 1;
                continue;
            }
        }
        let ch = html[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn clean_tag(tag: &str) -> String {
    let lower = tag.to_lowercase();
    if !lower.contains(" on") && !lower.contains("javascript:") && !lower.contains("data:text/html") {
        return tag.to_owned();
    }
    let mut out = String::new();
    let mut chars = tag.char_indices().peekable();
    let mut in_quote: Option<char> = None;
    let mut skipping = false;
    let mut word = String::new();
    let mut token_start = 0;
    while let Some((index, c)) = chars.next() {
        match in_quote {
            Some(q) => {
                if c == q {
                    in_quote = None;
                }
                if !skipping {
                    out.push(c);
                }
                continue;
            }
            None => {}
        }
        if c.is_whitespace() || c == '>' || c == '/' {
            word.clear();
            skipping = false;
            token_start = index + c.len_utf8();
            out.push(c);
            continue;
        }
        word.push(c);
        if c == '=' {
            let name = word.trim_end_matches('=').to_lowercase();
            let value_start = &tag[index + 1..];
            let unsafe_url = value_start.trim_start_matches(['"', '\'']).to_lowercase().starts_with("javascript:");
            if name.starts_with("on") || unsafe_url {
                skipping = true;
                // Remove what was already written for this attribute name.
                let written = out.len() - (index - token_start);
                out.truncate(written);
            }
        }
        if c == '"' || c == '\'' {
            in_quote = Some(c);
        }
        if !skipping {
            out.push(c);
        }
    }
    out
}

impl Content<'_> {
    fn local_now(&self) -> String {
        self.site.date("Y-m-d H:i:s", self.now)
    }

    fn gmt_now(&self) -> String {
        rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", self.now)
    }

    /// `get_gmt_from_date()`.
    fn gmt_from_local(&self, local: &str) -> String {
        match self.site.timezone.local_timestamp(local) {
            Some(ts) => rustpress_core::datetime::SiteTimezone::utc().format("Y-m-d H:i:s", ts),
            None => local.to_owned(),
        }
    }

    /// `get_date_from_gmt()`.
    fn local_from_gmt(&self, gmt: &str) -> String {
        match rustpress_core::datetime::SiteTimezone::gmt_timestamp(gmt) {
            Some(ts) => self.site.date("Y-m-d H:i:s", ts),
            None => gmt.to_owned(),
        }
    }

    pub fn is_hierarchical(&self, post_type: &str) -> bool {
        post_type == "page" || self.site.types.post_type(post_type).is_some_and(|t| t.hierarchical)
    }

    /// Taxonomies of a post type (`get_object_taxonomies()`), `(name, hierarchical)`.
    pub fn object_taxonomies(&self, post_type: &str) -> Vec<(String, bool)> {
        let mut out: Vec<(String, bool)> = Vec::new();
        if post_type == "post" {
            out.push(("category".into(), true));
            out.push(("post_tag".into(), false));
        }
        for tax in &self.site.types.taxonomies {
            if tax.object_types.iter().any(|t| t == post_type) && !out.iter().any(|(n, _)| *n == tax.name) {
                out.push((tax.name.clone(), tax.hierarchical));
            }
        }
        out
    }

    fn can(&self, capability: &str) -> bool {
        self.user.can(capability)
    }

    /// `wp_unique_post_slug()`.
    pub async fn unique_post_slug(&self, slug: &str, post_id: u64, status: &str, post_type: &str, parent: u64) -> Result<String> {
        if DATE_FLOATING.contains(&status) || (status == "inherit" && post_type == "revision") || post_type == "user_request" {
            return Ok(slug.to_owned());
        }
        let feeds = ["feed", "rdf", "rss", "rss2", "atom"];
        let scope = if post_type == "attachment" {
            SlugScope::Attachment
        } else if self.is_hierarchical(post_type) {
            SlugScope::Hierarchical
        } else {
            SlugScope::Flat
        };
        let mut bad = self.db.post_name_taken(scope, slug, post_type, post_id, parent).await? || feeds.contains(&slug) || slug == "embed";
        if scope == SlugScope::Hierarchical {
            let digits = slug.trim_start_matches("page");
            bad = bad || (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()));
        }
        if scope == SlugScope::Flat && post_type == "post" && !bad && !slug.is_empty() && slug.chars().all(|c| c.is_ascii_digit()) {
            let current = self.db.post(post_id).await?.map(|p| p.post_name);
            if current.as_deref() != Some(slug) {
                let structure = self.site.options.string("permalink_structure", "");
                let parts: Vec<&str> = structure.split('/').filter(|p| !p.is_empty()).collect();
                let number: u64 = slug.parse().unwrap_or(0);
                if number > 0
                    && let Some(index) = parts.iter().position(|p| *p == "%postname%")
                {
                    bad = index == 0 || (parts.get(index.wrapping_sub(1)) == Some(&"%year%") && number < 13) || (parts.get(index.wrapping_sub(1)) == Some(&"%monthnum%") && number < 32);
                }
            }
        }
        if !bad {
            return Ok(slug.to_owned());
        }
        let mut suffix = 2u32;
        loop {
            let candidate = format!("{}-{suffix}", truncate_slug(slug, 200 - (suffix.to_string().len() + 1)));
            if !self.db.post_name_taken(scope, &candidate, post_type, post_id, parent).await? {
                return Ok(candidate);
            }
            suffix += 1;
        }
    }

    /// `wp_add_trashed_suffix_to_post_name_for_post()`.
    async fn add_trashed_suffix(&self, post: &Post) -> Result<String> {
        if post.post_name.ends_with("__trashed") {
            return Ok(post.post_name.clone());
        }
        self.writer.add_post_meta(post.id, "_wp_desired_post_slug", &post.post_name).await?;
        let name = format!("{}__trashed", truncate_slug(&post.post_name, 191));
        self.writer.update_rows("posts", "ID", post.id, &[("post_name", Val::Str(name.clone()))]).await?;
        Ok(name)
    }

    /// `wp_insert_post()` / `wp_update_post()`. Returns the post ID.
    pub async fn insert_post(&self, data: &PostData) -> Result<u64> {
        let before = match data.id.filter(|id| *id != 0) {
            Some(id) => Some(self.db.post(id).await?.ok_or_else(|| ContentError::Message(self.site.tr("Invalid post ID.")))?),
            None => None,
        };
        let update = before.is_some();
        let previous_status = before.as_ref().map_or_else(|| "new".to_owned(), |p| p.post_status.clone());
        let post_id = before.as_ref().map_or(0, |p| p.id);
        let pick = |new: &Option<String>, old: Option<&String>| new.clone().or_else(|| old.cloned()).unwrap_or_default();

        let post_type = data.post_type.clone().or_else(|| before.as_ref().map(|p| p.post_type.clone())).filter(|t| !t.is_empty()).unwrap_or_else(|| "post".to_owned());
        let post_title = pick(&data.post_title, before.as_ref().map(|p| &p.post_title));
        let post_content = pick(&data.post_content, before.as_ref().map(|p| &p.post_content));
        let post_excerpt = pick(&data.post_excerpt, before.as_ref().map(|p| &p.post_excerpt));
        let (post_title, post_content, post_excerpt) = if self.can("unfiltered_html") {
            (post_title, post_content, post_excerpt)
        } else {
            (kses_post(&post_title).replace('<', "&lt;").replace('>', "&gt;").replace("&amp;lt;", "&lt;"), kses_post(&post_content), kses_post(&post_excerpt))
        };
        let post_title = if self.can("unfiltered_html") { post_title.trim().to_owned() } else { rustpress_core::formatting::strip_tags(&post_title).trim().to_owned() };
        let supports = |feature: &str| self.site.post_type_supports(&post_type, feature);
        if post_type != "attachment" && post_content.is_empty() && post_title.is_empty() && post_excerpt.is_empty() && supports("editor") && supports("title") && supports("excerpt") {
            return Err(ContentError::Message(self.site.tr("Content, title, and excerpt are empty.")));
        }
        let mut post_status = data.post_status.clone().or_else(|| before.as_ref().map(|p| p.post_status.clone())).filter(|s| !s.is_empty()).unwrap_or_else(|| "draft".to_owned());
        if post_type == "attachment" && !["inherit", "private", "trash", "auto-draft"].contains(&post_status.as_str()) {
            post_status = "inherit".to_owned();
        }

        // Categories
        let mut post_category: Vec<u64> = match &data.post_category {
            Some(cats) => cats.iter().copied().filter(|c| *c != 0).collect(),
            None if update => self.db.terms_for_post(post_id, Some("category")).await?.into_iter().map(|t| t.term_id).collect(),
            None => Vec::new(),
        };
        if post_category.is_empty() && post_type == "post" && post_status != "auto-draft" {
            post_category = vec![self.site.options.u64("default_category", 1)];
        }

        // Slug
        let mut post_name = match &data.post_name {
            Some(name) => name.clone(),
            None => before.as_ref().map(|p| p.post_name.clone()).unwrap_or_default(),
        };
        if post_status == "pending" {
            let publish_cap = if post_type == "page" { "publish_pages" } else { "publish_posts" };
            if !self.can(publish_cap) {
                post_name = String::new();
            }
        }
        if post_name.is_empty() {
            post_name = if !DATE_FLOATING.contains(&post_status.as_str()) { sanitize_title(&post_title) } else { String::new() };
        } else {
            let current = before.as_ref().map(|p| p.post_name.as_str()).unwrap_or_default();
            if !(update && post_name == current) {
                post_name = sanitize_title(&post_name);
            }
        }

        // Dates
        let requested_date = data.post_date.clone().or_else(|| before.as_ref().filter(|_| !data.edit_date || data.post_date.is_none()).map(|p| p.post_date.clone()));
        let requested_gmt = data.post_date_gmt.clone().or_else(|| before.as_ref().map(|p| p.post_date_gmt.clone()));
        let clear_date = update && DATE_FLOATING.contains(&previous_status.as_str()) && !data.edit_date && before.as_ref().is_some_and(|p| p.post_date_gmt == "0000-00-00 00:00:00");
        let (requested_date, requested_gmt) = if clear_date { (Some(self.local_now()), Some(String::new())) } else { (requested_date, requested_gmt) };
        let post_date = match requested_date.filter(|d| !d.is_empty() && d != "0000-00-00 00:00:00") {
            Some(date) => date,
            None => match requested_gmt.as_deref().filter(|g| !g.is_empty() && *g != "0000-00-00 00:00:00") {
                Some(gmt) => self.local_from_gmt(gmt),
                None => self.local_now(),
            },
        };
        if !valid_date(&post_date) {
            return Err(ContentError::Message(self.site.tr("Invalid date.")));
        }
        let post_date_gmt = match requested_gmt.filter(|g| !g.is_empty() && g != "0000-00-00 00:00:00") {
            Some(gmt) if data.post_date.is_none() || data.post_date_gmt.is_some() => gmt,
            _ => {
                if DATE_FLOATING.contains(&post_status.as_str()) {
                    "0000-00-00 00:00:00".to_owned()
                } else {
                    self.gmt_from_local(&post_date)
                }
            }
        };
        let (post_modified, post_modified_gmt) = if update || post_date == "0000-00-00 00:00:00" { (self.local_now(), self.gmt_now()) } else { (post_date.clone(), post_date_gmt.clone()) };
        if post_type != "attachment" {
            let gmt_ts = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post_date_gmt).unwrap_or(0);
            if post_status == "publish" && gmt_ts - self.now >= 60 {
                post_status = "future".to_owned();
            } else if post_status == "future" && gmt_ts - self.now < 60 {
                post_status = "publish".to_owned();
            }
        }

        let comment_status = match data.comment_status.clone().filter(|s| !s.is_empty()) {
            Some(s) => s,
            None if update => before.as_ref().map(|p| p.comment_status.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| "closed".to_owned()),
            None => self.default_status(&post_type, "comment"),
        };
        let ping_status = match data.ping_status.clone().filter(|s| !s.is_empty()) {
            Some(s) => s,
            None if update => before.as_ref().map(|p| p.ping_status.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| self.default_status(&post_type, "ping")),
            None => self.default_status(&post_type, "ping"),
        };
        let post_author = data.post_author.or(before.as_ref().map(|p| p.post_author)).unwrap_or(self.user.id);
        let menu_order = data.menu_order.or(before.as_ref().map(|p| p.menu_order)).unwrap_or(0);
        let mut post_password = pick(&data.post_password, before.as_ref().map(|p| &p.post_password));
        if post_status == "private" {
            post_password.clear();
        }
        let post_parent = data.post_parent.or(before.as_ref().map(|p| p.post_parent)).unwrap_or(0);
        let post_mime_type = pick(&data.post_mime_type, before.as_ref().map(|p| &p.post_mime_type));

        // Trash
        if previous_status == "trash" && post_status != "trash" {
            let desired = self.db.post_meta_value(post_id, "_wp_desired_post_slug").await?;
            if let Some(desired) = desired.filter(|d| !d.is_empty()) {
                self.writer.delete_post_meta(post_id, "_wp_desired_post_slug").await?;
                post_name = desired;
            }
        }
        if post_status != "trash" && !post_name.is_empty() {
            // Other trashed posts with this slug give it up.
            let query = rustpress_db::PostQuery { post_types: Vec::new(), statuses: vec!["trash".into()], name: Some(post_name.clone()), exclude_ids: vec![post_id], ..Default::default() };
            for other in self.db.posts(&query).await? {
                self.add_trashed_suffix(&other).await?;
            }
        }
        if post_status == "trash"
            && previous_status != "trash"
            && previous_status != "new"
            && let Some(current) = &before
        {
            post_name = self.add_trashed_suffix(current).await?;
        }
        let post_name = self.unique_post_slug(&post_name, post_id, &post_status, &post_type, post_parent).await?;

        let fields: Vec<(&str, Val)> = vec![
            ("post_author", Val::Int(post_author as i64)),
            ("post_date", Val::Str(post_date.clone())),
            ("post_date_gmt", Val::Str(post_date_gmt.clone())),
            ("post_content", Val::Str(post_content.clone())),
            ("post_content_filtered", Val::Str(before.as_ref().map(|_| String::new()).unwrap_or_default())),
            ("post_title", Val::Str(post_title.clone())),
            ("post_excerpt", Val::Str(post_excerpt.clone())),
            ("post_status", Val::Str(post_status.clone())),
            ("post_type", Val::Str(post_type.clone())),
            ("comment_status", Val::Str(comment_status)),
            ("ping_status", Val::Str(ping_status)),
            ("post_password", Val::Str(post_password)),
            ("post_name", Val::Str(post_name.clone())),
            ("to_ping", Val::Str(String::new())),
            ("pinged", Val::Str(String::new())),
            ("post_modified", Val::Str(post_modified)),
            ("post_modified_gmt", Val::Str(post_modified_gmt)),
            ("post_parent", Val::Int(post_parent as i64)),
            ("menu_order", Val::Int(i64::from(menu_order))),
            ("post_mime_type", Val::Str(post_mime_type)),
        ];
        let guid = data.guid.clone().or_else(|| before.as_ref().map(|p| p.guid.clone())).unwrap_or_default();
        let mut fields = fields;
        let post_id = if update {
            // `post_content_filtered` is kept as it is stored.
            fields.retain(|(c, _)| *c != "post_content_filtered");
            self.writer.update_rows("posts", "ID", post_id, &fields).await?;
            post_id
        } else {
            fields.push(("guid", Val::Str(guid)));
            self.writer.insert_row("posts", &fields).await?
        };

        if post_name.is_empty() && !DATE_FLOATING.contains(&post_status.as_str()) {
            let unique = self.unique_post_slug(&sanitize_title(&post_title), post_id, &post_status, &post_type, post_parent).await?;
            self.writer.update_rows("posts", "ID", post_id, &[("post_name", Val::Str(unique))]).await?;
        }

        // Terms
        let taxonomies = self.object_taxonomies(&post_type);
        if taxonomies.iter().any(|(n, _)| n == "category") {
            self.set_object_terms(post_id, &post_category.iter().map(|c| TermRef::Id(*c)).collect::<Vec<_>>(), "category", false).await?;
        }
        if let Some(tags) = &data.tags_input
            && taxonomies.iter().any(|(n, _)| n == "post_tag")
        {
            self.set_object_terms(post_id, &tags.iter().map(|t| TermRef::Name(t.clone())).collect::<Vec<_>>(), "post_tag", false).await?;
        }
        for (taxonomy, terms) in &data.tax_input {
            if taxonomies.iter().any(|(n, _)| n == taxonomy) {
                self.set_object_terms(post_id, terms, taxonomy, false).await?;
            }
        }
        for (key, value) in &data.meta_input {
            self.writer.update_post_meta(post_id, key, value).await?;
        }

        // The GUID of a new post is its permalink.
        let stored = self.db.post(post_id).await?;
        if let Some(post) = &stored
            && !update
            && post.guid.is_empty()
            && let Ok(link) = self.site.permalink(post).await
        {
            self.writer.update_rows("posts", "ID", post_id, &[("guid", Val::Str(link))]).await?;
        }
        if let Some(thumbnail) = data.thumbnail_id {
            if thumbnail == -1 {
                self.writer.delete_post_meta(post_id, "_thumbnail_id").await?;
            } else {
                self.writer.update_post_meta(post_id, "_thumbnail_id", &thumbnail.to_string()).await?;
            }
        }
        if let Some(template) = &data.page_template {
            self.writer.update_post_meta(post_id, "_wp_page_template", template).await?;
        }
        if update && post_type != "attachment" {
            self.save_revision(post_id).await?;
        }
        // `_update_term_count_on_transition_post_status()`: the counts follow the publish state.
        let final_status = stored.as_ref().map_or(post_status.as_str(), |p| p.post_status.as_str());
        if (previous_status == "publish") != (final_status == "publish") {
            let ids: Vec<u64> = self.db.terms_for_post(post_id, None).await?.into_iter().map(|t| t.term_taxonomy_id).collect();
            self.update_term_counts(&ids).await?;
        }
        Ok(post_id)
    }

    fn default_status(&self, post_type: &str, kind: &str) -> String {
        let feature = if kind == "ping" { "trackbacks" } else { "comments" };
        if post_type == "page" || !self.site.post_type_supports(post_type, feature) {
            "closed".to_owned()
        } else {
            self.site.options.string(&format!("default_{kind}_status"), "closed")
        }
    }

    /// `wp_save_post_revision()`.
    async fn save_revision(&self, post_id: u64) -> Result<()> {
        let Some(post) = self.db.post(post_id).await? else { return Ok(()) };
        if post.post_status == "auto-draft" || !self.site.post_type_supports(&post.post_type, "revisions") {
            return Ok(());
        }
        let query = rustpress_db::PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(post_id), order_by: rustpress_db::OrderBy::Date, ..Default::default() };
        let revisions = self.db.posts(&query).await?;
        if let Some(latest) = revisions.iter().find(|r| r.post_name.contains(&format!("{post_id}-revision"))) {
            let changed = normalize_whitespace(&post.post_title) != normalize_whitespace(&latest.post_title)
                || normalize_whitespace(&post.post_content) != normalize_whitespace(&latest.post_content)
                || normalize_whitespace(&post.post_excerpt) != normalize_whitespace(&latest.post_excerpt);
            if !changed {
                return Ok(());
            }
        }
        let fields: Vec<(&str, Val)> = vec![
            ("post_author", Val::Int(post.post_author as i64)),
            ("post_date", Val::Str(post.post_modified.clone())),
            ("post_date_gmt", Val::Str(post.post_modified_gmt.clone())),
            ("post_content", Val::Str(post.post_content.clone())),
            ("post_content_filtered", Val::Str(String::new())),
            ("post_title", Val::Str(post.post_title.clone())),
            ("post_excerpt", Val::Str(post.post_excerpt.clone())),
            ("post_status", Val::Str("inherit".into())),
            ("post_type", Val::Str("revision".into())),
            ("comment_status", Val::Str("closed".into())),
            ("ping_status", Val::Str("closed".into())),
            ("post_password", Val::Str(String::new())),
            ("post_name", Val::Str(format!("{post_id}-revision-v1"))),
            ("to_ping", Val::Str(String::new())),
            ("pinged", Val::Str(String::new())),
            ("post_modified", Val::Str(post.post_modified.clone())),
            ("post_modified_gmt", Val::Str(post.post_modified_gmt.clone())),
            ("post_parent", Val::Int(post_id as i64)),
            ("menu_order", Val::Int(0)),
            ("post_mime_type", Val::Str(String::new())),
            ("guid", Val::Str(String::new())),
        ];
        let revision_id = self.writer.insert_row("posts", &fields).await?;
        let guid = format!("{}/?p={revision_id}", self.site.settings.home.trim_end_matches('/'));
        self.writer.update_rows("posts", "ID", revision_id, &[("guid", Val::Str(guid))]).await?;
        Ok(())
    }

    /// `wp_create_post_autosave()`: the autosave of the current user is a revision named `<id>-autosave-v1`.
    pub async fn create_autosave(&self, post_id: u64, data: &PostData) -> Result<()> {
        let Some(post) = self.db.post(post_id).await? else { return Err(ContentError::Message(self.site.tr("Invalid post ID."))) };
        let title = data.post_title.clone().unwrap_or_else(|| post.post_title.clone());
        let content = data.post_content.clone().unwrap_or_else(|| post.post_content.clone());
        let excerpt = data.post_excerpt.clone().unwrap_or_else(|| post.post_excerpt.clone());
        let name = format!("{post_id}-autosave-v1");
        let query = rustpress_db::PostQuery { post_types: vec!["revision".into()], statuses: vec!["inherit".into()], parent: Some(post_id), order_by: rustpress_db::OrderBy::Date, ..Default::default() };
        let existing = self.db.posts(&query).await?.into_iter().find(|r| r.post_name == name && r.post_author == self.user.id);
        let now_local = self.site.date("Y-m-d H:i:s", self.now);
        let now_gmt = self.site.date_utc("Y-m-d H:i:s", self.now);
        // The autosave equals the post: nothing to keep.
        if title == post.post_title && content == post.post_content && excerpt == post.post_excerpt {
            if let Some(autosave) = existing {
                self.writer.execute(format!("DELETE FROM {} WHERE post_id = {}", self.writer.table("postmeta"), autosave.id)).await?;
                self.writer.delete_rows("posts", "ID", autosave.id).await?;
            }
            return Ok(());
        }
        if let Some(autosave) = existing {
            self.writer
                .update_rows(
                    "posts",
                    "ID",
                    autosave.id,
                    &[
                        ("post_title", Val::Str(title)),
                        ("post_content", Val::Str(content)),
                        ("post_excerpt", Val::Str(excerpt)),
                        ("post_modified", Val::Str(now_local)),
                        ("post_modified_gmt", Val::Str(now_gmt)),
                    ],
                )
                .await?;
            return Ok(());
        }
        let fields: Vec<(&str, Val)> = vec![
            ("post_author", Val::Int(self.user.id as i64)),
            ("post_date", Val::Str(now_local.clone())),
            ("post_date_gmt", Val::Str(now_gmt.clone())),
            ("post_content", Val::Str(content)),
            ("post_content_filtered", Val::Str(String::new())),
            ("post_title", Val::Str(title)),
            ("post_excerpt", Val::Str(excerpt)),
            ("post_status", Val::Str("inherit".into())),
            ("post_type", Val::Str("revision".into())),
            ("comment_status", Val::Str("closed".into())),
            ("ping_status", Val::Str("closed".into())),
            ("post_password", Val::Str(String::new())),
            ("post_name", Val::Str(name)),
            ("to_ping", Val::Str(String::new())),
            ("pinged", Val::Str(String::new())),
            ("post_modified", Val::Str(now_local)),
            ("post_modified_gmt", Val::Str(now_gmt)),
            ("post_parent", Val::Int(post_id as i64)),
            ("menu_order", Val::Int(0)),
            ("post_mime_type", Val::Str(String::new())),
            ("guid", Val::Str(String::new())),
        ];
        let id = self.writer.insert_row("posts", &fields).await?;
        let guid = format!("{}/?p={id}", self.site.settings.home.trim_end_matches('/'));
        self.writer.update_rows("posts", "ID", id, &[("guid", Val::Str(guid))]).await?;
        Ok(())
    }

    // ------------------------------------------------------------------ terms

    /// Finds a term of a taxonomy by ID, name or slug, or makes it (`wp_insert_term()`); returns its `term_taxonomy_id`.
    async fn term_taxonomy_id(&self, term: &TermRef, taxonomy: &str, hierarchical: bool) -> Result<Option<u64>> {
        match term {
            TermRef::Id(id) => Ok(self.db.term_by_id(Some(taxonomy), *id).await?.map(|t| t.term_taxonomy_id)),
            TermRef::Name(name) => {
                let name = name.trim();
                if name.is_empty() {
                    return Ok(None);
                }
                if hierarchical && let Ok(id) = name.parse::<u64>() && let Some(t) = self.db.term_by_id(Some(taxonomy), id).await? {
                    return Ok(Some(t.term_taxonomy_id));
                }
                let slug = sanitize_title(name);
                if let Some(t) = self.db.term_by_slug(taxonomy, &slug).await? {
                    return Ok(Some(t.term_taxonomy_id));
                }
                if let Some(t) = self.db.terms(taxonomy).await?.into_iter().find(|t| t.name.eq_ignore_ascii_case(name)) {
                    return Ok(Some(t.term_taxonomy_id));
                }
                // A new term.
                let unique = self.db.unique_term_slug(&slug).await?;
                let term_id = self.writer.insert_row("terms", &[("name", Val::Str(name.to_owned())), ("slug", Val::Str(unique)), ("term_group", Val::Int(0))]).await?;
                let tt = self
                    .writer
                    .insert_row(
                        "term_taxonomy",
                        &[("term_id", Val::Int(term_id as i64)), ("taxonomy", Val::Str(taxonomy.to_owned())), ("description", Val::Str(String::new())), ("parent", Val::Int(0)), ("count", Val::Int(0))],
                    )
                    .await?;
                Ok(Some(tt))
            }
        }
    }

    /// `wp_set_object_terms()`.
    pub async fn set_object_terms(&self, object_id: u64, terms: &[TermRef], taxonomy: &str, append: bool) -> Result<()> {
        let hierarchical = self.object_taxonomies("post").iter().chain(self.site.types.taxonomies.iter().map(|t| (t.name.clone(), t.hierarchical)).collect::<Vec<_>>().iter()).any(|(n, h)| n == taxonomy && *h);
        let mut wanted: Vec<u64> = Vec::new();
        for term in terms {
            if let Some(id) = self.term_taxonomy_id(term, taxonomy, hierarchical).await?
                && !wanted.contains(&id)
            {
                wanted.push(id);
            }
        }
        let current: Vec<u64> = self.db.terms_for_post(object_id, Some(taxonomy)).await?.into_iter().map(|t| t.term_taxonomy_id).collect();
        let mut touched: Vec<u64> = Vec::new();
        if !append {
            for old in current.iter().filter(|c| !wanted.contains(c)) {
                self.writer.execute(format!("DELETE FROM {} WHERE object_id = {object_id} AND term_taxonomy_id = {old}", self.writer.table("term_relationships"))).await?;
                touched.push(*old);
            }
        }
        for (order, id) in wanted.iter().enumerate() {
            if current.contains(id) {
                continue;
            }
            self.writer.insert_row("term_relationships", &[("object_id", Val::Int(object_id as i64)), ("term_taxonomy_id", Val::Int(*id as i64)), ("term_order", Val::Int(order as i64 + 1))]).await?;
            touched.push(*id);
        }
        self.update_term_counts(&touched).await?;
        Ok(())
    }

    /// `wp_update_term_count()` for the given `term_taxonomy_id`s: published posts of the types the taxonomy belongs to.
    pub async fn update_term_counts(&self, tt_ids: &[u64]) -> Result<()> {
        for id in tt_ids {
            let sql = format!(
                "UPDATE {tt} SET count = (SELECT COUNT(*) FROM {tr} tr INNER JOIN {posts} p ON p.ID = tr.object_id WHERE tr.term_taxonomy_id = {id} AND p.post_status = 'publish' AND p.post_type NOT IN ('revision', 'nav_menu_item', 'attachment')) WHERE term_taxonomy_id = {id}",
                tt = self.writer.table("term_taxonomy"),
                tr = self.writer.table("term_relationships"),
                posts = self.writer.table("posts")
            );
            self.writer.execute(sql).await?;
        }
        Ok(())
    }

    /// `wp_delete_object_term_relationships()`.
    async fn delete_object_terms(&self, object_id: u64) -> Result<()> {
        let ids: Vec<u64> = self.db.terms_for_post(object_id, None).await?.into_iter().map(|t| t.term_taxonomy_id).collect();
        self.writer.delete_rows("term_relationships", "object_id", object_id).await?;
        self.update_term_counts(&ids).await
    }

    // ------------------------------------------------------------------ trash and delete

    /// `wp_trash_post()`.
    pub async fn trash_post(&self, id: u64) -> Result<bool> {
        let Some(post) = self.db.post(id).await? else { return Ok(false) };
        if post.post_status == "trash" {
            return Ok(false);
        }
        self.writer.add_post_meta(id, "_wp_trash_meta_status", &post.post_status).await?;
        self.writer.add_post_meta(id, "_wp_trash_meta_time", &self.now.to_string()).await?;
        self.insert_post(&PostData { id: Some(id), post_status: Some("trash".into()), ..Default::default() }).await?;
        // `wp_trash_post_comments()`
        let comments_table = self.writer.table("comments");
        let rows = self.db.comment_statuses(id).await?;
        if !rows.is_empty() {
            let statuses = Value::Array(rows.iter().map(|(cid, status)| (Key::Int(*cid as i64), Value::Str(status.as_bytes().to_vec()))).collect());
            self.writer.add_post_meta(id, "_wp_trash_meta_comments_status", &String::from_utf8_lossy(&rustpress_phpser::serialize(&statuses))).await?;
            self.writer.execute(format!("UPDATE {comments_table} SET comment_approved = 'post-trashed' WHERE comment_post_ID = {id}")).await?;
            self.update_comment_count(id).await?;
        }
        Ok(true)
    }

    /// `wp_untrash_post()`.
    pub async fn untrash_post(&self, id: u64) -> Result<bool> {
        let Some(post) = self.db.post(id).await? else { return Ok(false) };
        if post.post_status != "trash" {
            return Ok(false);
        }
        let new_status = if post.post_type == "attachment" { "inherit" } else { "draft" };
        // `wp_untrash_post_set_previous_status()`: back to what it was.
        let previous = self.db.post_meta_value(id, "_wp_trash_meta_status").await?.filter(|s| !s.is_empty());
        let status = previous.unwrap_or_else(|| new_status.to_owned());
        self.writer.delete_post_meta(id, "_wp_trash_meta_status").await?;
        self.writer.delete_post_meta(id, "_wp_trash_meta_time").await?;
        self.insert_post(&PostData { id: Some(id), post_status: Some(status), ..Default::default() }).await?;
        // `wp_untrash_post_comments()`
        if let Some(raw) = self.db.post_meta_value(id, "_wp_trash_meta_comments_status").await?
            && let Ok(Value::Array(items)) = rustpress_phpser::unserialize(raw.as_bytes())
        {
            for (key, status) in items {
                let Key::Int(cid) = key else { continue };
                let status = String::from_utf8_lossy(status.as_bytes().unwrap_or_default()).into_owned();
                let status = if status == "post-trashed" { "0".to_owned() } else { status };
                self.writer.update_rows("comments", "comment_ID", cid as u64, &[("comment_approved", Val::Str(status))]).await?;
            }
            self.writer.delete_post_meta(id, "_wp_trash_meta_comments_status").await?;
            self.update_comment_count(id).await?;
        }
        Ok(true)
    }

    /// `wp_update_comment_count_now()`: the approved comments of a post.
    pub async fn update_comment_count(&self, post_id: u64) -> Result<()> {
        let sql = format!(
            "UPDATE {posts} SET comment_count = (SELECT COUNT(*) FROM {comments} WHERE comment_post_ID = {post_id} AND comment_approved = '1') WHERE ID = {post_id}",
            posts = self.writer.table("posts"),
            comments = self.writer.table("comments")
        );
        self.writer.execute(sql).await?;
        Ok(())
    }

    /// `wp_delete_post( $id, $force )`.
    pub async fn delete_post(&self, id: u64, force: bool) -> Result<bool> {
        let Some(post) = self.db.post(id).await? else { return Ok(false) };
        if !force && (post.post_type == "post" || post.post_type == "page") && post.post_status != "trash" {
            return self.trash_post(id).await;
        }
        self.writer.delete_post_meta(id, "_wp_trash_meta_status").await?;
        self.writer.delete_post_meta(id, "_wp_trash_meta_time").await?;
        self.delete_object_terms(id).await?;
        let posts = self.writer.table("posts");
        if self.is_hierarchical(&post.post_type) {
            self.writer.execute(format!("UPDATE {posts} SET post_parent = {} WHERE post_parent = {id} AND post_type = '{}'", post.post_parent, post.post_type.replace('\'', ""))).await?;
        }
        // Revisions go with the post; attachments stay, with the grandparent as their parent.
        let revisions = self.db.revision_ids(id).await?;
        for revision in revisions {
            self.writer.execute(format!("DELETE FROM {} WHERE post_id = {revision}", self.writer.table("postmeta"))).await?;
            self.writer.delete_rows("posts", "ID", revision).await?;
        }
        self.writer.execute(format!("UPDATE {posts} SET post_parent = {} WHERE post_parent = {id} AND post_type = 'attachment'", post.post_parent)).await?;
        // Comments and their meta.
        self.writer.execute(format!("DELETE FROM {cm} WHERE comment_id IN (SELECT comment_ID FROM {c} WHERE comment_post_ID = {id})", cm = self.writer.table("commentmeta"), c = self.writer.table("comments"))).await?;
        self.writer.delete_rows("comments", "comment_post_ID", id).await?;
        self.writer.delete_rows("postmeta", "post_id", id).await?;
        self.writer.delete_rows("posts", "ID", id).await?;
        Ok(true)
    }
}

/// `wp_checkdate()` on the date part of `Y-m-d H:i:s`.
pub fn valid_date(date: &str) -> bool {
    let b = date.as_bytes();
    if b.len() < 10 {
        return false;
    }
    let parse = |range: std::ops::Range<usize>| date.get(range).and_then(|s| s.parse::<u32>().ok());
    let (Some(y), Some(m), Some(d)) = (parse(0..4), parse(5..7), parse(8..10)) else { return false };
    if y == 0 && m == 0 && d == 0 {
        return true;
    }
    chrono::NaiveDate::from_ymd_opt(y as i32, m, d).is_some()
}
