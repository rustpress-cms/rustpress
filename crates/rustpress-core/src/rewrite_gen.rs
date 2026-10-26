//! The rewrite rules WordPress builds from the permalink structure: `WP_Rewrite::rewrite_rules()`
//! and `generate_rewrite_rules()`, with the registrations core makes on `init` (REST, sitemaps,
//! the built-in taxonomies) and the ones the post types and taxonomies of [`Types`] stand for.
//!
//! The result is what WordPress stores in the option `rewrite_rules` (`$matches[1]` in the queries).

use rustpress_phpser::{Key, Value};

use crate::types::Types;

const EP_NONE: u32 = 0;
const EP_PERMALINK: u32 = 1;
const EP_ROOT: u32 = 64;
const EP_COMMENTS: u32 = 128;
const EP_SEARCH: u32 = 256;
const EP_CATEGORIES: u32 = 512;
const EP_TAGS: u32 = 1024;
const EP_AUTHORS: u32 = 2048;
const EP_PAGES: u32 = 4096;
const EP_DATE: u32 = 4;

const FEEDS: [&str; 5] = ["feed", "rdf", "rss", "rss2", "atom"];
const INDEX: &str = "index.php";

/// What the rules depend on besides the structure itself.
#[derive(Debug, Clone)]
pub struct RewriteInput<'a> {
    pub permalink_structure: &'a str,
    /// The `front` of the structure the types were read under: their stored slugs carry it when `with_front` is on.
    pub old_front: &'a str,
    pub category_base: &'a str,
    pub tag_base: &'a str,
    pub page_on_front: u64,
    /// The home address has no path: `robots.txt`, `favicon.ico` and `sitemap.xml` can be answered.
    pub home_at_root: bool,
    /// Search engines are welcome: the sitemaps exist.
    pub blog_public: bool,
    pub types: &'a Types,
}

/// An ordered `array( regex => query )` with the semantics of `array_merge()`: a key keeps its first position.
#[derive(Debug, Clone, Default)]
struct Rules(Vec<(String, String)>);

impl Rules {
    fn of(pairs: &[(String, String)]) -> Self {
        let mut rules = Self::default();
        for (k, v) in pairs {
            rules.set(k.clone(), v.clone());
        }
        rules
    }

    fn set(&mut self, key: String, value: String) {
        match self.0.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = value,
            None => self.0.push((key, value)),
        }
    }

    fn merge(mut self, other: Rules) -> Rules {
        for (k, v) in other.0 {
            self.set(k, v);
        }
        self
    }
}

/// One entry of `$wp_rewrite->extra_permastructs`.
struct Permastruct {
    struct_: String,
    ep_mask: u32,
    feed: bool,
}

/// The tags of `$wp_rewrite->rewritecode` with their regular expressions and query prefixes.
struct Tags {
    code: Vec<String>,
    replace: Vec<String>,
    query: Vec<String>,
}

impl Tags {
    fn core() -> Self {
        let pairs: [(&str, &str, &str); 11] = [
            ("%year%", "([0-9]{4})", "year="),
            ("%monthnum%", "([0-9]{1,2})", "monthnum="),
            ("%day%", "([0-9]{1,2})", "day="),
            ("%hour%", "([0-9]{1,2})", "hour="),
            ("%minute%", "([0-9]{1,2})", "minute="),
            ("%second%", "([0-9]{1,2})", "second="),
            ("%postname%", "([^/]+)", "name="),
            ("%post_id%", "([0-9]+)", "p="),
            ("%author%", "([^/]+)", "author_name="),
            ("%pagename%", "([^/]+?)", "pagename="),
            ("%search%", "(.+)", "s="),
        ];
        Self { code: pairs.iter().map(|p| p.0.to_owned()).collect(), replace: pairs.iter().map(|p| p.1.to_owned()).collect(), query: pairs.iter().map(|p| p.2.to_owned()).collect() }
    }

    /// `add_rewrite_tag()`.
    fn add(&mut self, tag: &str, regex: &str, query: &str) {
        match self.code.iter().position(|c| c == tag) {
            Some(i) => {
                self.replace[i] = regex.to_owned();
                self.query[i] = query.to_owned();
            }
            None => {
                self.code.push(tag.to_owned());
                self.replace.push(regex.to_owned());
                self.query.push(query.to_owned());
            }
        }
    }

    /// `str_replace( $this->rewritecode, $replacements, $subject )`: the pairs apply one after the other.
    fn replace_in(&self, replacements: &[String], subject: &str) -> String {
        let mut out = subject.to_owned();
        for (code, replacement) in self.code.iter().zip(replacements) {
            out = out.replace(code.as_str(), replacement);
        }
        out
    }
}

/// `preg_match_all( '/%.+?%/', $text )`.
fn tokens(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(offset) = bytes.get(i + 2..).and_then(|rest| rest.iter().position(|&b| b == b'%'))
        {
            let end = i + 2 + offset;
            out.push(&text[i..=end]);
            i = end + 1;
            continue;
        }
        i += 1;
    }
    out
}

struct Generator<'a> {
    input: &'a RewriteInput<'a>,
    tags: Tags,
    /// `$wp_rewrite->front` and `root`.
    front: String,
    root: String,
}

fn preg_index(number: usize) -> String {
    format!("$matches[{number}]")
}

impl Generator<'_> {
    fn feed_regex2() -> String {
        format!("({})/?$", FEEDS.join("|"))
    }

    /// `generate_rewrite_rules()` without endpoints (core registers none).
    fn generate(&self, permalink_structure: &str, ep_mask: u32, paged: bool, feed: bool, for_comments: bool, walk_dirs: bool) -> Rules {
        let feedregex2 = Self::feed_regex2();
        let feedregex = format!("feed/{feedregex2}");
        let trackbackregex = "trackback/?$";
        let pageregex = "page/?([0-9]{1,})/?$";
        let commentregex = "comment-page-([0-9]{1,})/?$";
        let embedregex = "embed/?$";

        let front = permalink_structure.find('%').map_or("", |at| &permalink_structure[..at]);
        let toks = tokens(permalink_structure);
        let mut queries: Vec<String> = Vec::new();
        for (i, token) in toks.iter().enumerate() {
            let mut query = if i > 0 { format!("{}&", queries[i - 1]) } else { String::new() };
            query.push_str(&self.tags.replace_in(&self.tags.query, token));
            query.push_str(&preg_index(i + 1));
            queries.push(query);
        }
        let mut structure = permalink_structure.to_owned();
        if front != "/" && !front.is_empty() {
            structure = structure.replace(front, "");
        }
        let structure = structure.trim_matches('/').to_owned();
        let dirs: Vec<&str> = if walk_dirs { structure.split('/').collect() } else { vec![structure.as_str()] };
        let front = front.trim_start_matches('/');

        let mut post_rewrite = Rules::default();
        let mut struct_ = front.to_owned();
        for dir in &dirs {
            struct_.push_str(dir);
            struct_.push('/');
            let struct_trimmed = struct_.trim_start_matches('/').to_owned();
            struct_ = struct_trimmed;
            let m = self.tags.replace_in(&self.tags.replace, &struct_);
            let num_toks = tokens(&struct_).len();
            let query = if num_toks > 0 { queries.get(num_toks - 1).cloned().unwrap_or_default() } else { String::new() };

            let pagematch = format!("{m}{pageregex}");
            let pagequery = format!("{INDEX}?{query}&paged={}", preg_index(num_toks + 1));
            let commentmatch = format!("{m}{commentregex}");
            let commentquery = format!("{INDEX}?{query}&cpage={}", preg_index(num_toks + 1));
            let rootcommentquery = format!("{INDEX}?{query}&page_id={}&cpage={}", self.input.page_on_front, preg_index(num_toks + 1));
            let mut feedquery = format!("{INDEX}?{query}&feed={}", preg_index(num_toks + 1));
            let mut feedquery2 = feedquery.clone();
            let feedmatch = format!("{m}{feedregex}");
            let feedmatch2 = format!("{m}{feedregex2}");
            let embedmatch = format!("{m}{embedregex}");
            let embedquery = format!("{INDEX}?{query}&embed=true");
            if for_comments {
                feedquery.push_str("&withcomments=1");
                feedquery2.push_str("&withcomments=1");
            }

            let mut rewrite = Rules::default();
            if feed {
                rewrite = Rules::of(&[(feedmatch, feedquery), (feedmatch2, feedquery2), (embedmatch, embedquery.clone())]);
            }
            if paged {
                rewrite = rewrite.merge(Rules::of(&[(pagematch, pagequery)]));
            }
            if ep_mask & EP_PAGES != 0 || ep_mask & EP_PERMALINK != 0 {
                rewrite = rewrite.merge(Rules::of(&[(commentmatch, commentquery)]));
            } else if ep_mask & EP_ROOT != 0 && self.input.page_on_front != 0 {
                rewrite = rewrite.merge(Rules::of(&[(format!("{m}{commentregex}"), rootcommentquery)]));
            }

            if num_toks > 0 {
                let mut post = false;
                let mut page = false;
                let has = |tag: &str| struct_.contains(tag);
                if has("%postname%") || has("%post_id%") || has("%pagename%") || (has("%year%") && has("%monthnum%") && has("%day%") && has("%hour%") && has("%minute%") && has("%second%")) {
                    post = true;
                    if has("%pagename%") {
                        page = true;
                    }
                }
                if !post && let Some(ptype) = self.input.types.rewrite_post_types().iter().find(|t| struct_.contains(&format!("%{}%", t.name))) {
                    post = true;
                    page = ptype.hierarchical;
                }

                if post {
                    let trackbackmatch = format!("{m}{trackbackregex}");
                    let trackbackquery = format!("{INDEX}?{query}&tb=1");
                    let embedmatch = format!("{m}{embedregex}");
                    let embedquery = format!("{INDEX}?{query}&embed=true");
                    let m_trimmed = m.trim_end_matches('/').to_owned();
                    let submatchbase = m_trimmed.replace(['(', ')'], "");
                    let sub1 = format!("{submatchbase}/([^/]+)/");
                    let sub2 = format!("{submatchbase}/attachment/([^/]+)/");
                    let subquery = format!("{INDEX}?attachment={}", preg_index(1));
                    let subtbquery = format!("{subquery}&tb=1");
                    let subfeedquery = format!("{subquery}&feed={}", preg_index(2));
                    let subcommentquery = format!("{subquery}&cpage={}", preg_index(2));
                    let subembedquery = format!("{subquery}&embed=true");
                    let variants = |base: &str| {
                        vec![
                            (format!("{base}?$"), subquery.clone()),
                            (format!("{base}{trackbackregex}"), subtbquery.clone()),
                            (format!("{base}{feedregex}"), subfeedquery.clone()),
                            (format!("{base}{feedregex2}"), subfeedquery.clone()),
                            (format!("{base}{commentregex}"), subcommentquery.clone()),
                            (format!("{base}{embedregex}"), subembedquery.clone()),
                        ]
                    };
                    let main_match = format!("{m_trimmed}(?:/([0-9]+))?/?$");
                    let main_query = format!("{INDEX}?{query}&page={}", preg_index(num_toks + 1));
                    rewrite = rewrite.merge(Rules::of(&[(main_match, main_query)]));
                    rewrite = Rules::of(&[(trackbackmatch, trackbackquery)]).merge(rewrite);
                    rewrite = Rules::of(&[(embedmatch, embedquery)]).merge(rewrite);
                    if !page {
                        rewrite = rewrite.merge(Rules::of(&variants(&sub1)));
                    }
                    rewrite = Rules::of(&variants(&sub2)).merge(rewrite);
                } else {
                    let closed = format!("{m}?$");
                    rewrite = rewrite.merge(Rules::of(&[(closed, format!("{INDEX}?{query}"))]));
                }
            }
            post_rewrite = rewrite.merge(post_rewrite);
        }
        post_rewrite
    }
}

impl<'a> Generator<'a> {
    fn new(input: &'a RewriteInput<'a>) -> Self {
        let structure = input.permalink_structure;
        let front = structure.find('%').map_or("", |at| &structure[..at]).to_owned();
        let using_index = structure.trim_start_matches('/').starts_with(INDEX);
        let root = if using_index { format!("{INDEX}/") } else { String::new() };
        Self { input, tags: Tags::core(), front, root }
    }

    fn using_index_permalinks(&self) -> bool {
        !self.root.is_empty()
    }

    /// `get_date_permastruct()`.
    fn date_permastruct(&self) -> String {
        let structure = self.input.permalink_structure;
        if structure.is_empty() {
            return String::new();
        }
        let endians = ["%year%/%monthnum%/%day%", "%day%/%monthnum%/%year%", "%monthnum%/%day%/%year%"];
        let endian = endians.iter().find(|e| structure.contains(**e)).copied().unwrap_or(endians[0]);
        let mut front = self.front.clone();
        for (index, token) in tokens(structure).iter().enumerate() {
            if *token == "%post_id%" && index < 3 {
                front.push_str("date/");
                break;
            }
        }
        format!("{front}{endian}")
    }

    /// The part of a stored slug without the front the structure had when it was read, and whether the type uses the front.
    fn without_old_front<'s>(&self, slug: &'s str) -> (&'s str, bool) {
        let old = self.input.old_front.trim_start_matches('/');
        if !old.is_empty()
            && let Some(rest) = slug.strip_prefix(old)
        {
            return (rest, true);
        }
        (slug, old.is_empty())
    }

    /// `add_permastruct()` with `with_front`.
    fn permastruct(&self, name: &str, slug: &str, with_front: bool, ep_mask: u32, feed: bool) -> Permastruct {
        let prefix = if with_front { &self.front } else { &self.root };
        Permastruct { struct_: format!("{prefix}{slug}/%{name}%"), ep_mask, feed }
    }
}

/// `WP_Rewrite::rewrite_rules()`: the rules for the structure in `input`, in the order WordPress stores them.
pub fn rewrite_rules(input: &RewriteInput<'_>) -> Vec<(String, String)> {
    if input.permalink_structure.is_empty() {
        return Vec::new();
    }
    let mut g = Generator::new(input);
    let mut top = Rules::default();
    let mut extra: Vec<Permastruct> = Vec::new();

    // `rest_api_register_rewrites()` and the sitemaps.
    for (regex, query) in [("^wp-json/?$", "index.php?rest_route=/"), ("^wp-json/(.*)?", "index.php?rest_route=/$matches[1]"), ("^index.php/wp-json/?$", "index.php?rest_route=/"), ("^index.php/wp-json/(.*)?", "index.php?rest_route=/$matches[1]")] {
        top.set(regex.to_owned(), query.to_owned());
    }
    if input.blog_public {
        for (regex, query) in [
            (r"^wp-sitemap\.xml$", "index.php?sitemap=index"),
            (r"^wp-sitemap\.xsl$", "index.php?sitemap-stylesheet=sitemap"),
            (r"^wp-sitemap-index\.xsl$", "index.php?sitemap-stylesheet=index"),
            (r"^wp-sitemap-([a-z]+?)-([a-z\d_-]+?)-(\d+?)\.xml$", "index.php?sitemap=$matches[1]&sitemap-subtype=$matches[2]&paged=$matches[3]"),
            (r"^wp-sitemap-([a-z]+?)-(\d+?)\.xml$", "index.php?sitemap=$matches[1]&paged=$matches[2]"),
        ] {
            top.set(regex.to_owned(), query.to_owned());
        }
    }

    // The built-in taxonomies: `create_initial_taxonomies()`.
    g.tags.add("%category%", "(.+?)", "category_name=");
    let category_base = taxonomy_base(input.category_base);
    let tag_base = taxonomy_base(input.tag_base);
    let using_index = g.using_index_permalinks();
    extra.push(g.permastruct("category", if category_base.is_empty() { "category" } else { &category_base }, category_base.is_empty() || using_index, EP_CATEGORIES, true));
    g.tags.add("%post_tag%", "([^/]+)", "tag=");
    extra.push(g.permastruct("post_tag", if tag_base.is_empty() { "tag" } else { &tag_base }, tag_base.is_empty() || using_index, EP_TAGS, true));
    g.tags.add("%post_format%", "([^/]+)", "post_format=");
    extra.push(g.permastruct("post_format", "type", true, EP_NONE, true));

    // Custom post types: `WP_Post_Type::add_rewrite_rules()`.
    let mut type_permastructs = Vec::new();
    for ptype in input.types.rewrite_post_types() {
        let (slug, with_front) = g.without_old_front(&ptype.slug);
        let slug = slug.trim_matches('/').to_owned();
        let regex = if ptype.hierarchical { "(.+?)" } else { "([^/]+)" };
        let query = match &ptype.query_var {
            Some(var) => format!("{var}="),
            None if ptype.hierarchical => format!("post_type={}&pagename=", ptype.name),
            None => format!("post_type={}&name=", ptype.name),
        };
        g.tags.add(&format!("%{}%", ptype.name), regex, &query);
        if let Some(archive) = &ptype.archive_slug {
            let (base, _) = g.without_old_front(archive);
            let archive_slug = format!("{}{}", if with_front { g.front.trim_start_matches('/') } else { "" }, base.trim_matches('/'));
            let name = &ptype.name;
            let feeds = FEEDS.join("|");
            top.set(format!("{archive_slug}/?$"), format!("index.php?post_type={name}"));
            top.set(format!("{archive_slug}/feed/({feeds})/?$"), format!("index.php?post_type={name}&feed=$matches[1]"));
            top.set(format!("{archive_slug}/({feeds})/?$"), format!("index.php?post_type={name}&feed=$matches[1]"));
            top.set(format!("{archive_slug}/page/([0-9]{{1,}})/?$"), format!("index.php?post_type={name}&paged=$matches[1]"));
        }
        type_permastructs.push(g.permastruct(&ptype.name, &slug, with_front, EP_PERMALINK, ptype.archive_slug.is_some()));
    }
    extra.extend(type_permastructs);

    // Custom taxonomies: `WP_Taxonomy::add_rewrite_rules()`.
    for taxonomy in input.types.rewrite_taxonomies() {
        let (slug, with_front) = g.without_old_front(&taxonomy.slug);
        let slug = slug.trim_matches('/').to_owned();
        let regex = if taxonomy.rewrite_hierarchical { "(.+?)" } else { "([^/]+)" };
        let query = match &taxonomy.query_var {
            Some(var) => format!("{var}="),
            None => format!("taxonomy={}&term=", taxonomy.name),
        };
        g.tags.add(&format!("%{}%", taxonomy.name), regex, &query);
        extra.push(g.permastruct(&taxonomy.name, &slug, with_front, EP_NONE, true));
    }

    let (robots, favicon, sitemap) = if input.home_at_root {
        (
            Rules::of(&[(r"robots\.txt$".to_owned(), format!("{INDEX}?robots=1"))]),
            Rules::of(&[(r"favicon\.ico$".to_owned(), format!("{INDEX}?favicon=1"))]),
            Rules::of(&[(r"sitemap\.xml".to_owned(), format!("{INDEX}?sitemap=index"))]),
        )
    } else {
        Default::default()
    };
    let deprecated = Rules::of(&[
        (r".*wp-(atom|rdf|rss|rss2|feed|commentsrss2)\.php$".to_owned(), format!("{INDEX}?feed=old")),
        (r".*wp-app\.php(/.*)?$".to_owned(), format!("{INDEX}?error=403")),
    ]);
    let registration = Rules::of(&[(r".*wp-register.php$".to_owned(), format!("{INDEX}?register=true"))]);

    let structure = input.permalink_structure;
    let post_rewrite = g.generate(structure, EP_PERMALINK, true, true, false, true);
    let date_rewrite = g.generate(&g.date_permastruct(), EP_DATE, true, true, false, true);
    let root_rewrite = g.generate(&format!("{}/", g.root), EP_ROOT, true, true, false, true);
    let comments_rewrite = g.generate(&format!("{}comments", g.root), EP_COMMENTS, false, true, true, false);
    let search_rewrite = g.generate(&format!("{}search/%search%", g.root), EP_SEARCH, true, true, false, true);
    let author_rewrite = g.generate(&format!("{}author/%author%", g.front), EP_AUTHORS, true, true, false, true);
    g.tags.add("%pagename%", "(.?.+?)", "pagename=");
    let page_rewrite = g.generate(&format!("{}%pagename%", g.root), EP_PAGES, true, true, false, false);

    for permastruct in &extra {
        let rules = g.generate(&permastruct.struct_, permastruct.ep_mask, true, permastruct.feed, false, true);
        top = top.merge(rules);
    }

    // `^[^%]*%(?:postname|category|tag|author)%`
    let verbose_page_rules = {
        let before = structure.find('%').map_or(structure.len(), |at| at);
        let rest = &structure[before..];
        ["%postname%", "%category%", "%tag%", "%author%"].iter().any(|t| rest.starts_with(t))
    };
    let mut rules = top.merge(robots).merge(favicon).merge(sitemap).merge(deprecated).merge(registration).merge(root_rewrite).merge(comments_rewrite).merge(search_rewrite).merge(author_rewrite).merge(date_rewrite);
    rules = if verbose_page_rules { rules.merge(page_rewrite).merge(post_rewrite) } else { rules.merge(post_rewrite).merge(page_rewrite) };
    rules.0
}

/// `_wp_filter_taxonomy_base()`, the filter on `get_option( 'category_base' )` and `'tag_base'`.
pub fn taxonomy_base(base: &str) -> String {
    if base.is_empty() {
        return String::new();
    }
    base.strip_prefix("/index.php/").unwrap_or(base).trim_matches('/').to_owned()
}

/// The option value: `serialize( array( regex => query ) )`.
pub fn serialize_rules(rules: &[(String, String)]) -> String {
    let items = rules.iter().map(|(k, v)| (Key::Str(k.clone().into_bytes()), Value::Str(v.clone().into_bytes()))).collect();
    String::from_utf8_lossy(&rustpress_phpser::serialize(&Value::Array(items))).into_owned()
}

#[cfg(test)]
mod tests {
    use super::tokens;

    #[test]
    fn tokens_of_a_structure_ending_in_a_percent_sign() {
        assert_eq!(tokens("/%year%/%postname%/"), vec!["%year%", "%postname%"]);
        assert_eq!(tokens("/%postname%/%"), vec!["%postname%"]);
        assert_eq!(tokens("%"), Vec::<&str>::new());
        assert_eq!(tokens("%%"), Vec::<&str>::new());
    }
}
