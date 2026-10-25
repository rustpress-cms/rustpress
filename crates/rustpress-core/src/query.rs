//! The main query: turns a request into WordPress' conditional flags
//! (`is_home`, `is_single`, ...) and the list of posts to show. This follows
//! `WP::parse_request`, `WP_Query::parse_query` / `get_posts` and
//! `WP::handle_404`.

use rustpress_db::{OrderBy, Post, PostQuery, Term, User};

use crate::Result;
use crate::rewrite::{QueryVars, request_path};
use crate::settings::ShowOnFront;
use crate::site::Site;
use crate::slug::sanitize_title;
use crate::types::is_internal_post_type;

/// Query variables WordPress reads from the query string (`$wp->public_query_vars`
/// plus the ones core registers through the `query_vars` filter).
const PUBLIC_QUERY_VARS: &[&str] = &[
    "m", "p", "posts", "w", "cat", "withcomments", "withoutcomments", "s", "search", "exact", "sentence",
    "calendar", "page", "paged", "more", "tb", "pb", "author", "order", "orderby", "year", "monthnum", "day",
    "hour", "minute", "second", "name", "category_name", "tag", "feed", "author_name", "pagename", "page_id",
    "attachment", "attachment_id", "subpost", "subpost_id", "preview", "robots", "favicon", "taxonomy", "term",
    "cpage", "post_type", "embed", "tag_id", "post_format", "rest_route", "sitemap", "sitemap-stylesheet", "sitemap-subtype",
];

const SEARCH_STOPWORDS: &[&str] = &[
    "about", "an", "are", "as", "at", "be", "by", "com", "for", "from", "how", "in", "is", "it", "of", "on", "or",
    "that", "the", "this", "to", "was", "what", "when", "where", "who", "will", "with", "www",
];

/// The `is_*()` conditionals of `WP_Query`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flags {
    pub is_home: bool,
    pub is_front_page: bool,
    pub is_posts_page: bool,
    pub is_single: bool,
    pub is_page: bool,
    pub is_attachment: bool,
    pub is_archive: bool,
    pub is_category: bool,
    pub is_tag: bool,
    pub is_tax: bool,
    pub is_post_type_archive: bool,
    pub is_author: bool,
    pub is_date: bool,
    pub is_year: bool,
    pub is_month: bool,
    pub is_day: bool,
    pub is_search: bool,
    pub is_feed: bool,
    /// The request asks for the embeddable view of a post (`/post/embed/`, `?embed=true`).
    pub is_embed: bool,
    pub is_paged: bool,
    pub is_robots: bool,
    pub is_sitemap: bool,
    pub is_favicon: bool,
    pub is_404: bool,
}

impl Flags {
    pub fn is_singular(&self) -> bool {
        self.is_single || self.is_page || self.is_attachment
    }
}

/// `get_queried_object()`.
#[derive(Debug, Clone, Default)]
pub enum Queried {
    #[default]
    None,
    Post(Box<Post>),
    Term(Term),
    User(User),
    /// The archive of a post type (get_queried_object() is its WP_Post_Type).
    PostType(String),
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The effective query variables (permalink rule merged with the query string).
    pub vars: QueryVars,
    pub flags: Flags,
    pub posts: Vec<Post>,
    pub queried: Queried,
    pub found_posts: u64,
    pub max_pages: u64,
    /// Current page of an archive (1-based).
    pub paged: u64,
    pub per_page: u64,
    /// Pattern of the rewrite rule that matched, if any.
    pub matched_rule: Option<String>,
    /// The request path as received (percent-encoded) and its raw query string.
    pub url_path: String,
    pub raw_query: String,
    /// The 404 was decided while parsing the query (an impossible date), not by `handle_404()`;
    /// only the latter asks for no caching.
    pub parse_error_404: bool,
}

impl QueryResult {
    pub(crate) fn new(vars: QueryVars, matched_rule: Option<String>) -> Self {
        Self {
            vars,
            flags: Flags::default(),
            posts: Vec::new(),
            queried: Queried::None,
            found_posts: 0,
            max_pages: 0,
            paged: 1,
            per_page: 10,
            matched_rule,
            url_path: String::new(),
            raw_query: String::new(),
            parse_error_404: false,
        }
    }

    /// A query that matched nothing, for rendering outside of a page request (REST).
    pub fn blank() -> Self {
        Self::new(QueryVars::new(), None)
    }

    /// `WP_Query::set_404()`: all conditionals off except `is_404`.
    pub fn into_404(mut self) -> Self {
        self.flags = Flags { is_404: true, ..Flags::default() };
        self.posts.clear();
        self.queried = Queried::None;
        self.found_posts = 0;
        self.max_pages = 0;
        self
    }

    pub fn status(&self) -> u16 {
        if self.flags.is_404 { 404 } else { 200 }
    }

    /// `get_queried_object_id()`: the post, term or user ID of the queried object.
    pub fn queried_object_id(&self) -> u64 {
        match &self.queried {
            Queried::Post(post) => post.id,
            Queried::Term(term) => term.term_id,
            Queried::User(user) => user.id,
            Queried::None | Queried::PostType(_) => 0,
        }
    }

    pub fn queried_post(&self) -> Option<&Post> {
        match &self.queried {
            Queried::Post(post) => Some(post.as_ref()),
            _ => None,
        }
    }
}

impl Site {
    /// Resolves `url_path` (as sent by the client, still percent-encoded) and
    /// `query_string` and runs the main query.
    pub async fn query(&self, url_path: &str, query_string: &str) -> Result<QueryResult> {
        let request = request_path(url_path, &self.settings.home_path());

        let mut vars = QueryVars::new();
        let mut matched_rule = None;
        let mut unmatched = false;
        if !request.is_empty() {
            unmatched = true;
            // The script named in the address (`/index.php/...`) takes part in the matching.
            let file = if url_path.trim_matches('/').strip_prefix(self.settings.home_path().trim_matches('/')).unwrap_or(url_path).trim_matches('/').starts_with("index.php") { "index.php" } else { "" };
            for m in self.rewrite.matches_in_file(&request, file) {
                // With "verbose" page rules (`%postname%` permalinks) a rule that
                // yields `pagename` only applies if that page exists.
                if self.use_verbose_page_rules()
                    && m.rule.query.contains("pagename=$matches[")
                    && let Some(path) = m.vars.get("pagename")
                    && self.page_by_path(path).await?.is_none()
                {
                    continue;
                }
                matched_rule = Some(m.rule.pattern.clone());
                // Only the public query variables of a permalink reach the query (`WP::parse_request()`).
                vars = m.vars;
                let keep: Vec<String> = vars.iter().map(|(k, _)| k.to_owned()).filter(|k| PUBLIC_QUERY_VARS.contains(&k.as_str()) || self.types.post_type(k).is_some() || self.types.taxonomy(k).is_some()).collect();
                let dropped: Vec<String> = vars.iter().map(|(k, _)| k.to_owned()).filter(|k| !keep.contains(k)).collect();
                for key in dropped {
                    vars.remove(&key);
                }
                unmatched = false;
                break;
            }
        }

        // Query-string variables win over the ones from the permalink.
        for (key, value) in QueryVars::parse(query_string).iter() {
            if PUBLIC_QUERY_VARS.contains(&key) || self.types.post_type(key).is_some() || self.types.taxonomy(key).is_some() {
                vars.set(key, value);
            }
        }

        self.map_type_vars(&mut vars);
        let mut result = QueryResult::new(vars, matched_rule);
        result.url_path = url_path.to_owned();
        result.raw_query = query_string.to_owned();
        if unmatched {
            let mut not_found = result.into_404();
            not_found.flags.is_embed = not_found.vars.get("embed").is_some();
            return Ok(not_found);
        }
        let mut result = self.run_main_query(result).await?;
        // `WP_Query::parse_query()`: only requests for one post, or ones that failed, can be embeds.
        result.flags.is_embed = result.vars.get("embed").is_some() && (result.flags.is_singular() || result.flags.is_404);
        Ok(result)
    }

    async fn run_main_query(&self, mut r: QueryResult) -> Result<QueryResult> {
        let s = &self.settings;
        let v = r.vars.clone();
        let mut f = Flags { is_feed: v.get("feed").is_some_and(|feed| !feed.is_empty()), ..Flags::default() };
        let mut queried = Queried::None;
        let mut posts: Vec<Post> = Vec::new();
        let mut found = 0;
        let mut q = PostQuery::default();

        let paged = v.u64("paged").filter(|p| *p > 0).unwrap_or(1);
        let mut page_id = v.u64("page_id");
        let pagename = v.get("pagename");

        if v.get("robots").is_some() {
            r.flags.is_robots = true;
            return Ok(r);
        }
        if v.get("sitemap").is_some_and(|s| !s.is_empty()) || v.get("sitemap-stylesheet").is_some_and(|s| !s.is_empty()) {
            r.flags.is_sitemap = true;
            return Ok(r);
        }
        if v.get("favicon").is_some() {
            r.flags.is_favicon = true;
            return Ok(r);
        }

        // --- WP_Query::parse_query: which kind of request is this? ---------------
        if v.get("attachment").is_some() || v.u64("attachment_id").is_some() {
            f.is_single = true;
            f.is_attachment = true;
        } else if v.get("name").is_some() || v.u64("p").is_some_and(|p| p != 0) {
            f.is_single = true;
        } else if pagename.is_some() || page_id.is_some_and(|p| p != 0) {
            f.is_page = true;
        } else {
            f.is_search = v.has("s");

            let num = |key: &str| v.get(key).and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
            let (mut year, mut month, mut day) = (num("year"), num("monthnum"), num("day"));
            if let Some(m) = v.get("m").filter(|m| m.len() >= 4 && m.bytes().all(|b| b.is_ascii_digit())) {
                year = m[..4].parse().unwrap_or(0);
                month = m.get(4..6).and_then(|x| x.parse().ok()).unwrap_or(0);
                day = m.get(6..8).and_then(|x| x.parse().ok()).unwrap_or(0);
            }
            if day != 0 {
                if month != 0 && year != 0 && !valid_date(year, month, day) {
                    r.parse_error_404 = true;
                    return Ok(r.into_404());
                }
                f.is_day = true;
                f.is_date = true;
            } else if month != 0 {
                if month > 12 {
                    r.parse_error_404 = true;
                    return Ok(r.into_404());
                }
                f.is_month = true;
                f.is_date = true;
            } else if year != 0 {
                f.is_year = true;
                f.is_date = true;
            }
            // `hour`, `minute`, `second` and the week `w` are date archives, too.
            let time = |key: &str| v.get(key).map(|x| x.trim().parse::<u32>().unwrap_or(0));
            let (hour, minute, second) = (time("hour"), time("minute"), time("second"));
            let week = v.get("w").map(|w| w.trim().parse::<u32>().unwrap_or(0)).filter(|w| *w != 0);
            if hour.is_some() || minute.is_some() || second.is_some() || week.is_some() {
                f.is_date = true;
            }
            q.hour = hour;
            q.minute = minute;
            q.second = second;
            q.week = week.map(|w| (w, self.options.u64("start_of_week", 0) as u32));
            if f.is_date {
                q.year = (year != 0).then_some(year);
                q.month = (month != 0).then_some(month);
                q.day = (day != 0).then_some(day);
            }

            // Taxonomies. `category_name=tech/rust` filters by the last path segment.
            if let Some(path) = v.get("category_name") {
                let slugs: Vec<String> = split_slugs(path.rsplit('/').next().unwrap_or(path));
                let terms = self.terms_by_slugs("category", &slugs).await?;
                let Some(first) = terms.first().cloned() else { return Ok(if f.is_feed { r.flags = Flags { is_category: true, ..f }; r } else { r.into_404() }) };
                f.is_category = true;
                q.tax_groups.push(self.with_descendants("category", &terms).await?);
                queried = Queried::Term(first);
            } else if let Some(ids) = v.get("cat") {
                // `intval()` of each entry: positive ids select, negative ones exclude.
                let ids: Vec<i64> = ids.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(|i| i.trim().parse().unwrap_or(0)).collect();
                let mut terms = Vec::new();
                let mut excluded = Vec::new();
                for id in &ids {
                    if *id > 0 {
                        terms.extend(self.db.term_by_id(Some("category"), *id as u64).await?);
                    } else if *id < 0 {
                        excluded.extend(self.db.term_by_id(Some("category"), id.unsigned_abs()).await?);
                    }
                }
                if !excluded.is_empty() {
                    q.tax_not.extend(self.with_descendants("category", &excluded).await?);
                }
                if terms.is_empty() && excluded.is_empty() {
                    if f.is_feed {
                        r.flags = Flags { is_category: true, ..f };
                        return Ok(r);
                    }
                    return Ok(r.into_404());
                }
                if let Some(first) = terms.first().cloned() {
                    f.is_category = true;
                    q.tax_groups.push(self.with_descendants("category", &terms).await?);
                    queried = Queried::Term(first);
                }
            }
            if let Some(tag) = v.get("tag") {
                // `a,b` matches either tag, `a b` (from `a+b`) requires both.
                let any = tag.contains(',');
                let slugs = if any { split_slugs(tag) } else { tag.split(|c: char| c == '+' || c.is_whitespace()).filter(|s| !s.is_empty()).map(str::to_owned).collect() };
                let terms = self.terms_by_slugs("post_tag", &slugs).await?;
                if terms.is_empty() || (!any && terms.len() < slugs.len()) {
                    return Ok(if f.is_feed { r.flags = Flags { is_tag: true, ..f }; r } else { r.into_404() });
                }
                f.is_tag = true;
                if any {
                    q.tax_groups.push(terms.iter().map(|t| t.term_taxonomy_id).collect());
                } else {
                    q.tax_groups.extend(terms.iter().map(|t| vec![t.term_taxonomy_id]));
                }
                if matches!(queried, Queried::None) {
                    queried = Queried::Term(terms[0].clone());
                }
            } else if let Some(id) = v.u64("tag_id") {
                let Some(term) = self.db.term_by_id(Some("post_tag"), id).await? else { return Ok(r.into_404()) };
                f.is_tag = true;
                q.tax_groups.push(vec![term.term_taxonomy_id]);
                if matches!(queried, Queried::None) {
                    queried = Queried::Term(term);
                }
            }
            // `post_format=link` is the taxonomy archive of `post-format-link` (`_post_format_request()`).
            let custom_tax = self.types.taxonomies.iter().find_map(|t| {
                v.get(&t.name).map(|term| {
                    // A hierarchical rewrite passes the whole path; the last segment is the term.
                    let term = if t.rewrite_hierarchical { term.rsplit('/').next().unwrap_or(term) } else { term };
                    (t.name.clone(), term.to_owned())
                })
            });
            let tax_vars: Option<(String, String)> = match (v.get("taxonomy"), v.get("term"), v.get("post_format")) {
                _ if custom_tax.is_some() => custom_tax,
                (Some(taxonomy), Some(term), _) => Some((taxonomy.to_owned(), term.to_owned())),
                (_, _, Some(format)) if !format.is_empty() => Some(("post_format".to_owned(), format!("post-format-{}", sanitize_title(format)))),
                _ => None,
            };
            if let Some((taxonomy, term)) = tax_vars {
                let taxonomy = taxonomy.as_str();
                let Some(term) = self.db.term_by_slug(taxonomy, &sanitize_title(&term)).await? else {
                    return Ok(if f.is_feed { r.flags = Flags { is_tax: true, ..f }; r } else { r.into_404() });
                };
                f.is_tax = true;
                q.tax_groups.push(self.with_descendants(taxonomy, std::slice::from_ref(&term)).await?);
                if matches!(queried, Queried::None) {
                    queried = Queried::Term(term);
                }
            }

            // Authors.
            if v.get("author").is_some() || v.get("author_name").is_some() {
                let user = match (v.u64("author"), v.get("author_name")) {
                    (Some(id), _) => self.db.user(id).await?,
                    (None, Some(name)) => self.db.user_by_nicename(&sanitize_title(name)).await?,
                    _ => None,
                };
                let Some(user) = user else { return Ok(r.into_404()) };
                f.is_author = true;
                q.author = Some(user.id);
                if matches!(queried, Queried::None) {
                    queried = Queried::User(user);
                }
            }

            // post_type=book is the archive of a type that has one.
            if let Some(post_type) = v.get("post_type").filter(|t| !t.contains(','))
                && self.types.post_type(post_type).is_some_and(|t| t.archive_slug.is_some())
            {
                f.is_post_type_archive = true;
                if matches!(queried, Queried::None) {
                    queried = Queried::PostType(post_type.to_owned());
                }
            }

            f.is_archive = f.is_date || f.is_author || f.is_category || f.is_tag || f.is_tax || f.is_post_type_archive;
            if !(f.is_archive || f.is_search || f.is_feed) {
                f.is_home = true;
            }
        }

        // A static front page: the home request turns into a page request.
        if f.is_home && s.show_on_front == ShowOnFront::Page && s.page_on_front != 0 {
            const KEEPS_FRONT_PAGE: [&str; 5] = ["preview", "page", "paged", "cpage", "embed"];
            if r.vars.iter().all(|(k, _)| KEEPS_FRONT_PAGE.contains(&k)) {
                f.is_page = true;
                f.is_home = false;
                page_id = Some(s.page_on_front);
                // `<!--nextpage-->` of the front page: its pages are the `paged` of the request.
                if let Some(paged_value) = r.vars.get("paged").map(str::to_owned) {
                    r.vars.set("page", paged_value);
                    r.vars.remove("paged");
                }
            }
        }

        // --- WP_Query::get_posts ---------------------------------------------------
        r.per_page = if f.is_feed { s.posts_per_rss } else { s.posts_per_page };

        if f.is_page {
            let page = match (pagename, page_id) {
                (Some(path), _) => self.page_by_path(path).await?,
                (None, Some(id)) => self.page_by_id(id).await?,
                _ => None,
            };
            let Some(page) = page else {
                // `WP_Query::get_posts()` takes the last segment of the page path as `name`, which is what
                // `redirect_guess_404_permalink()` looks for.
                if let Some(path) = pagename {
                    let name = sanitize_title(path.rsplit('/').next().unwrap_or(path));
                    r.vars.set("name", name);
                }
                if f.is_feed {
                    // A feed is not a 404 even when there is nothing in it.
                    r.flags = f;
                    return Ok(r);
                }
                return Ok(r.into_404());
            };
            if s.show_on_front == ShowOnFront::Page && s.page_for_posts == page.id {
                f.is_page = false;
                f.is_home = true;
                f.is_posts_page = true;
            } else {
                found = 1;
                posts.push(page.clone());
            }
            queried = Queried::Post(Box::new(page));
        } else if f.is_single {
            let post = self.find_single(&v, &mut f).await?;
            let Some(post) = post else { return Ok(r.into_404()) };
            found = 1;
            posts.push(post.clone());
            queried = Queried::Post(Box::new(post));
        }

        // `WP::handle_404`: paged content that exceeds the last `<!--nextpage-->`
        // (or has none) is a 404 whose post stays in the query, so that
        // `redirect_canonical` can send the visitor to the post's base URL.
        let page_var = v.get("page").map(|p| p.trim_matches('/')).filter(|p| !p.is_empty() && *p != "0");
        if let Some(page_var) = page_var {
            let content_found = match posts.first() {
                Some(post) if f.is_singular() => {
                    post.post_content.contains("<!--nextpage-->") && page_var.parse::<u64>().unwrap_or(0) <= post.post_content.matches("<!--nextpage-->").count() as u64 + 1
                }
                _ => !f.is_posts_page,
            };
            if !content_found {
                let mut kept = r.into_404();
                kept.posts = posts;
                return Ok(kept);
            }
        }
        if !f.is_singular() {
            r.paged = paged;
            if let Some(post_type) = v.get("post_type") {
                q.post_types = vec![post_type.to_owned()];
            } else if f.is_tax {
                // Every searchable type that has objects in the queried taxonomies.
                q.post_types = self.tax_post_types(&r.vars);
            }
            if f.is_search {
                if v.get("post_type").is_none() {
                    q.post_types = self.types.searchable_post_types().into_iter().map(str::to_owned).collect();
                }
                if let Some(s) = v.get("s") {
                    q.search = parse_search_terms(s, v.has("sentence"));
                    q.search_phrase = Some(s.to_owned());
                    // Feeds are in date order: `empty( $q['orderby'] ) && ! $this->is_feed`.
                    if !f.is_feed {
                        q.order_by = OrderBy::Relevance;
                    }
                }
            }
            q.limit = Some(r.per_page);
            q.offset = (paged - 1) * r.per_page;
            posts = self.db.posts(&q).await?;
            found = self.db.count_posts(&q).await?;
            r.max_pages = found.div_ceil(r.per_page);
            if f.is_home && paged <= 1 && !s.sticky_posts.is_empty() {
                posts = self.with_sticky_first(posts).await?;
            }
        } else {
            // A single post can be "paged", too (`/post/page/2/`): the title and the classes show it.
            r.paged = paged;
        }
        f.is_paged = paged > 1;

        // --- WP::handle_404 --------------------------------------------------------
        if posts.is_empty() {
            let term_found = matches!(queried, Queried::Term(_));
            let tolerated = !f.is_paged
                && (f.is_author || ((f.is_category || f.is_tag || f.is_tax || f.is_post_type_archive) && (term_found || f.is_post_type_archive)) || f.is_home || f.is_search || f.is_feed);
            if !tolerated {
                return Ok(r.into_404());
            }
        }

        f.is_front_page = (f.is_home && !f.is_posts_page && s.show_on_front == ShowOnFront::Posts)
            || (f.is_page && s.show_on_front == ShowOnFront::Page && s.page_on_front != 0
                && matches!(&queried, Queried::Post(p) if p.id == s.page_on_front));

        r.flags = f;
        r.posts = posts;
        r.queried = queried;
        r.found_posts = found;
        Ok(r)
    }

    /// `WP_Rewrite::$use_verbose_page_rules`: the generic page rules come first unless the permalink
    /// structure starts with a wildcard such as `%post_id%`.
    pub fn use_verbose_page_rules(&self) -> bool {
        static RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"^[^%]*%(?:postname|category|tag|author)%").expect("valid regex"));
        RE.is_match(&self.settings.permalink_structure)
    }

    /// `WP::parse_request()`: the query variable of a custom post type names the type and the slug,
    /// and only publicly queryable types may be asked for.
    fn map_type_vars(&self, vars: &mut QueryVars) {
        for t in &self.types.post_types {
            if let Some(value) = vars.get(&t.name).map(str::to_owned) {
                vars.set("post_type", t.name.clone());
                vars.set("name", value);
            }
        }
        if let Some(post_type) = vars.get("post_type")
            && !self.types.public_post_types().contains(&post_type)
        {
            vars.remove("post_type");
        }
    }

    /// The post types of a taxonomy archive: every searchable type with objects in the taxonomy.
    fn tax_post_types(&self, vars: &QueryVars) -> Vec<String> {
        let taxonomy = vars.get("taxonomy").or_else(|| self.types.taxonomies.iter().find(|t| vars.get(&t.name).is_some()).map(|t| t.name.as_str()));
        let Some(info) = taxonomy.and_then(|name| self.types.taxonomy(name)) else { return vec!["post".into()] };
        let searchable = self.types.searchable_post_types();
        info.object_types.iter().filter(|t| searchable.contains(&t.as_str())).cloned().collect()
    }

    /// `get_page_by_path()`: resolves `parent/child` through the page hierarchy.
    pub async fn page_by_path(&self, path: &str) -> Result<Option<Post>> {
        self.post_by_path(path, "page").await
    }

    /// `get_page_by_path( $path, OBJECT, $post_type )`.
    pub async fn post_by_path(&self, path: &str, post_type: &str) -> Result<Option<Post>> {
        let parts: Vec<String> =
            path.trim_matches('/').split('/').map(sanitize_title).filter(|p| !p.is_empty()).collect();
        let Some(last) = parts.last() else { return Ok(None) };
        let candidates = self
            .db
            .posts(&PostQuery { post_types: vec![post_type.to_owned()], names: parts.clone(), ..PostQuery::default() })
            .await?;
        'candidate: for page in candidates.iter().filter(|p| &p.post_name == last) {
            let mut current = page;
            for wanted in parts[..parts.len() - 1].iter().rev() {
                match candidates.iter().find(|p| p.id == current.post_parent && &p.post_name == wanted) {
                    Some(parent) => current = parent,
                    None => continue 'candidate,
                }
            }
            if current.post_parent == 0 {
                return Ok(Some(page.clone()));
            }
        }
        Ok(None)
    }

    async fn page_by_id(&self, id: u64) -> Result<Option<Post>> {
        Ok(self.db.post(id).await?.filter(|p| p.post_type == "page" && p.post_status == "publish"))
    }

    /// Looks up the post for `name=`, `p=` or `attachment=` requests and
    /// corrects the flags if it turns out to be a page or attachment.
    async fn find_single(&self, v: &QueryVars, f: &mut Flags) -> Result<Option<Post>> {
        let post = if let Some(slug) = v.get("attachment") {
            self.db
                .posts(&PostQuery {
                    post_types: vec!["attachment".into()],
                    statuses: vec!["inherit".into()],
                    name: Some(sanitize_title(slug)),
                    limit: Some(1),
                    ..PostQuery::default()
                })
                .await?
                .pop()
        } else if let Some(name) = v.get("name") {
            let post_type = v.get("post_type").unwrap_or("post").to_owned();
            if self.types.post_type(&post_type).is_some_and(|t| t.hierarchical) {
                return self.post_by_path(name, &post_type).await;
            }
            self.db
                .posts(&PostQuery {
                    post_types: vec![post_type],
                    name: Some(sanitize_title(name)),
                    year: v.u64("year").map(|y| y as u32),
                    month: v.u64("monthnum").map(|y| y as u32),
                    day: v.u64("day").map(|y| y as u32),
                    limit: Some(1),
                    ..PostQuery::default()
                })
                .await?
                .pop()
        } else {
            // `?p=ID` finds only the post type being asked for (`post` unless `post_type` says
            // otherwise); a page or custom type behind `?p=` is a 404 that `redirect_canonical()` fixes.
            let by_attachment_id = v.u64("p").is_none();
            let wanted = v.get("post_type").unwrap_or("post");
            let id = v.u64("p").or(v.u64("attachment_id")).unwrap_or(0);
            self.db.post(id).await?.filter(|p| {
                !is_internal_post_type(&p.post_type)
                    && if by_attachment_id {
                        p.post_type == "attachment" && matches!(p.post_status.as_str(), "publish" | "inherit")
                    } else {
                        p.post_type == wanted && p.post_status == "publish"
                    }
            })
        };
        if let Some(post) = &post {
            match post.post_type.as_str() {
                "page" => {
                    f.is_single = false;
                    f.is_page = true;
                }
                "attachment" => {
                    f.is_single = false;
                    f.is_attachment = true;
                }
                _ => {}
            }
        }
        Ok(post)
    }

    async fn terms_by_slugs(&self, taxonomy: &str, slugs: &[String]) -> Result<Vec<Term>> {
        let mut terms = Vec::new();
        for slug in slugs {
            terms.extend(self.db.term_by_slug(taxonomy, &sanitize_title(slug)).await?);
        }
        Ok(terms)
    }

    /// `term_taxonomy_id`s of the given terms and all their descendants.
    async fn with_descendants(&self, taxonomy: &str, terms: &[Term]) -> Result<Vec<u64>> {
        let all = self.db.terms(taxonomy).await?;
        let mut wanted: Vec<u64> = terms.iter().map(|t| t.term_id).collect();
        let mut i = 0;
        while i < wanted.len() {
            let parent = wanted[i];
            let children: Vec<u64> =
                all.iter().filter(|t| t.parent == parent && !wanted.contains(&t.term_id)).map(|t| t.term_id).collect();
            wanted.extend(children);
            i += 1;
        }
        let mut ids: Vec<u64> = terms.iter().map(|t| t.term_taxonomy_id).collect();
        ids.extend(all.iter().filter(|t| wanted.contains(&t.term_id)).map(|t| t.term_taxonomy_id));
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    /// Sticky posts first: those already on the page in page order, then the
    /// sticky ones that would have been on other pages (newest first).
    async fn with_sticky_first(&self, page: Vec<Post>) -> Result<Vec<Post>> {
        let sticky = &self.settings.sticky_posts;
        let (mut front, rest): (Vec<Post>, Vec<Post>) = page.into_iter().partition(|p| sticky.contains(&p.id));
        let missing: Vec<u64> = sticky.iter().copied().filter(|id| !front.iter().any(|p| p.id == *id)).collect();
        if !missing.is_empty() {
            front.extend(self.db.posts(&PostQuery { ids: missing, ..PostQuery::default() }).await?);
        }
        front.extend(rest);
        Ok(front)
    }
}

fn valid_date(year: u32, month: u32, day: u32) -> bool {
    let leap = (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28 + leap as u32,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// `tag=a,b` / `tag=a b`: slugs separated by commas or whitespace.
fn split_slugs(input: &str) -> Vec<String> {
    input.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

/// `WP_Query::parse_search_terms` for the search string `s`: words and
/// `"quoted phrases"`, without one-letter terms and stop words. If nothing is
/// left, or there are more than 9 terms, the whole string is the only term.
/// An empty string yields no terms, i.e. no restriction.
pub fn parse_search_terms(s: &str, sentence: bool) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    let terms = split_search_terms(s, sentence);
    if terms.is_empty() || terms.len() > 9 { vec![s.to_owned()] } else { terms }
}

fn split_search_terms(s: &str, sentence: bool) -> Vec<String> {
    if sentence {
        return vec![s.to_owned()];
    }
    let mut tokens: Vec<(String, bool)> = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == '"' {
            chars.next();
            let phrase: String = chars.by_ref().take_while(|&c| c != '"').collect();
            tokens.push((phrase, true));
        } else if matches!(c, ' ' | '\t' | ',' | '+') {
            chars.next();
        } else {
            let mut word = String::new();
            while let Some(&c) = chars.peek() {
                if matches!(c, ' ' | '\t' | ',' | '+' | '"') {
                    break;
                }
                word.push(c);
                chars.next();
            }
            tokens.push((word, false));
        }
    }
    tokens
        .into_iter()
        .filter_map(|(token, quoted)| {
            let term = if quoted { token.trim_matches(|c| c == '"' || c == '\'').to_owned() } else { token.trim_matches(|c| matches!(c, '"' | '\'' | ' ')).to_owned() };
            let single_letter = term.chars().count() == 1 && term.chars().all(|c| c.is_ascii_alphabetic() || c == '-');
            let stopword = SEARCH_STOPWORDS.contains(&term.to_lowercase().as_str());
            (!term.is_empty() && !single_letter && !stopword).then_some(term)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_terms_split_like_wordpress() {
        assert_eq!(parse_search_terms("rust wordpress", false), ["rust", "wordpress"]);
        assert_eq!(parse_search_terms("\"exact phrase\" more", false), ["exact phrase", "more"]);
        assert_eq!(parse_search_terms("the a rust", false), ["rust"]);
        assert_eq!(parse_search_terms("the rust", true), ["the rust"]);
        assert!(parse_search_terms("", false).is_empty());
        // Only stop words / one-letter terms left: search for the whole string.
        assert_eq!(parse_search_terms("the", false), ["the"]);
        assert_eq!(parse_search_terms("a+b, c", false), ["a+b, c"]);
        // More than nine terms also fall back to the whole string.
        let long = "aa bb cc dd ee ff gg hh ii jj";
        assert_eq!(parse_search_terms(long, false), [long]);
    }

    #[test]
    fn dates_are_validated() {
        assert!(valid_date(2024, 2, 29));
        assert!(!valid_date(2023, 2, 29));
        assert!(!valid_date(2026, 4, 31));
        assert!(valid_date(2026, 12, 31));
        assert!(!valid_date(2026, 13, 1));
    }

    #[test]
    fn slugs_split_on_commas_and_spaces() {
        assert_eq!(split_slugs("a,b c"), ["a", "b", "c"]);
        assert_eq!(split_slugs(""), Vec::<String>::new());
    }

    #[test]
    fn internal_post_types_are_not_public() {
        assert!(is_internal_post_type("wp_template"));
        assert!(is_internal_post_type("revision"));
        assert!(!is_internal_post_type("post"));
        assert!(!is_internal_post_type("book"));
    }
}
