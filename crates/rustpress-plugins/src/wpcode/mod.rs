//! WPCode Lite 2.4.1 (`insert-headers-and-footers`), what it adds to the website.
//!
//! - The global scripts of "Code Snippets > Header & Footer" (`global-output.php`): the options `ihaf_insert_header`
//!   (`wp_head`), `ihaf_insert_body` (`wp_body_open` at priority 1) and `ihaf_insert_footer` (`wp_footer`), printed
//!   with `wp_unslash()`.
//! - The snippets of the auto-insert locations "Site Wide Header", "Site Wide Body" and "Site Wide Footer"
//!   (`WPCode_Auto_Insert_Site_Wide`, hooked on `wp`), read from the snippet cache (the option `wpcode_snippets`,
//!   ordered by priority), with the conditional logic of WPCode Lite (`rules.rs`).
//! - The shortcode `[wpcode id="…"]` (`wpcode_shortcode_handler()`), which reads the snippet from its post.
//! - The code types HTML, text (`do_shortcode()`), JavaScript (`<script>`), CSS (`<style class="wpcode-css-snippet">`)
//!   and universal snippets without PHP in them; "Headers & Footers mode" (`headers_footers_mode`) turns everything
//!   but the global scripts off.
//!
//! Not ported: PHP snippets and universal snippets with PHP (no PHP runs; they print nothing), the locations in and
//! around the content, excerpts and posts (`WPCode_Auto_Insert_Single`, `_Archive`), the PHP-only locations, the
//! administration with the snippet editor and the library, the toolbar menu and the footer data of
//! `WPCode_Admin_Bar_Info` for signed-in administrators, safe mode (`?wpcode-safe-mode=1`) and the preview of the
//! snippet editor. See the WPCode section of PLAN.md.

mod rules;

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, Head, HeadContext, NativePlugin, Robot, ShortcodeContext};
use rustpress_core::shortcodes::Atts;
use rustpress_core::{Result, Site};
use rustpress_phpser::Value;

use crate::acf::php;

pub struct Wpcode;

/// `WPCODE_VERSION`.
const VERSION: &str = "2.4.1";

impl NativePlugin for Wpcode {
    fn file(&self) -> &'static str {
        "insert-headers-and-footers/ihaf.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some("insert-headers-and-footers")
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let raw = |name: &'static str| async move { site.options.fetch(&site.db, name).await };
            let global = |code: Option<String>| global_code(code.as_deref().map(rustpress_core::options::maybe_unserialize).as_ref());
            let header = global(raw("ihaf_insert_header").await?);
            let body = global(raw("ihaf_insert_body").await?);
            let footer = global(raw("ihaf_insert_footer").await?);
            // `WPCode_Settings::load_options()`: anything but an array gives the defaults (without the mode).
            let settings = raw("wpcode_settings").await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
            let headers_footers_mode = settings.as_ref().filter(|s| s.as_array().is_some()).is_some_and(|s| php::truthy(s.get("headers_footers_mode")));
            let cache = raw("wpcode_snippets").await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
            let snippets = if headers_footers_mode { Locations::default() } else { Locations::from_cache(cache.as_ref()) };
            Ok(Box::new(Active { header, body, footer, headers_footers_mode, snippets }) as Box<dyn ActivePlugin>)
        })
    }
}

/// `wpcode_global_script_output()`: nothing for an empty or blank option, else `wp_unslash()` of it.
fn global_code(value: Option<&Value>) -> String {
    if !php::truthy(value) {
        return String::new();
    }
    let code = php::strval(value);
    let trimmed = code.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b']);
    if trimmed.is_empty() || trimmed == "0" {
        return String::new();
    }
    crate::wordpress_seo::head::stripslashes(&code)
}

/// A snippet as `WPCode_Snippet` holds it: from the cache or from its post.
#[derive(Debug, Clone)]
struct Snippet {
    code: String,
    code_type: String,
    use_rules: bool,
    rules: Value,
    priority: i64,
}

impl Snippet {
    /// `WPCode_Snippet::load_from_array()` of a cached snippet: the keys that are set.
    fn from_cache(data: &Value) -> Self {
        let get = |key: &str| data.get(key).filter(|v| !matches!(v, Value::Null));
        Self {
            code: php::strval(get("code")),
            code_type: php::strval(get("code_type")),
            use_rules: get("use_rules").is_some_and(|v| php::truthy(Some(v))),
            rules: get("rules").cloned().unwrap_or(Value::Array(Vec::new())),
            priority: get("priority").map_or(10, |v| php::intval(Some(v))),
        }
    }
}

/// The cached snippets of the site-wide locations.
#[derive(Debug, Default)]
struct Locations {
    header: Vec<Snippet>,
    body: Vec<Snippet>,
    footer: Vec<Snippet>,
}

impl Locations {
    /// `WPCode_Snippet_Cache::get_snippets_from_db_cache()` (`(array) get_option( 'wpcode_snippets' )`, each location
    /// sorted by priority with `usort()`, which keeps the order of equal ones) for the locations of
    /// `WPCode_Auto_Insert_Site_Wide`.
    fn from_cache(cache: Option<&Value>) -> Self {
        let location = |name: &str| {
            let mut snippets: Vec<Snippet> = cache.and_then(|c| c.get(name)).and_then(Value::as_array).map(|items| items.iter().map(|(_, v)| Snippet::from_cache(v)).collect()).unwrap_or_default();
            snippets.sort_by_key(|s| s.priority);
            snippets
        };
        Self { header: location("site_wide_header"), body: location("site_wide_body"), footer: location("site_wide_footer") }
    }
}

struct Active {
    header: String,
    body: String,
    footer: String,
    headers_footers_mode: bool,
    snippets: Locations,
}

/// What a snippet sees of the page it is printed on.
struct Page<'a> {
    site: &'a Site,
    query: &'a rustpress_core::QueryResult,
    services: &'a dyn rustpress_core::native::PageServices,
    /// `global $post`.
    post: Option<&'a rustpress_db::Post>,
    /// `wp_get_current_user()->roles` (only looked up when a snippet has rules).
    roles: Vec<String>,
    /// `WPCode_Conditional_Page::get_term()` (only looked up when a snippet has rules).
    terms: Vec<u64>,
}

impl<'a> Page<'a> {
    async fn new(site: &'a Site, query: &'a rustpress_core::QueryResult, services: &'a dyn rustpress_core::native::PageServices, rules: bool) -> Result<Self> {
        let mut page = Self { site, query, services, post: query.posts.first(), roles: Vec::new(), terms: Vec::new() };
        if !rules {
            return Ok(page);
        }
        if let Some(viewer) = &query.viewer {
            page.roles = current_roles(site, viewer.id).await?;
        }
        let flags = &query.flags;
        page.terms = match (&query.queried, flags.is_tax || flags.is_category || flags.is_tag) {
            (rustpress_core::query::Queried::Term(term), true) => vec![term.term_id],
            (_, true) => Vec::new(),
            // `get_terms( array( 'object_ids' => get_the_ID(), 'fields' => 'ids' ) )`: the terms of every taxonomy,
            // by name, without empty ones (`hide_empty`).
            _ if flags.is_singular() => match page.post {
                Some(post) => site.db.terms_for_post(post.id, None).await?.into_iter().filter(|t| t.count > 0).map(|t| t.term_id).collect(),
                None => Vec::new(),
            },
            _ => Vec::new(),
        };
        Ok(page)
    }
}

/// The roles of a signed-in user (`WP_User::get_role_caps()`).
async fn current_roles(site: &Site, user_id: u64) -> Result<Vec<String>> {
    let Some(user) = site.db.auth_user_by_id(user_id).await? else { return Ok(Vec::new()) };
    let prefix = site.db.table_prefix().to_owned();
    let roles = rustpress_auth::Roles::from_option(&site.options.fetch(&site.db, &format!("{prefix}user_roles")).await?.unwrap_or_default());
    Ok(rustpress_auth::CurrentUser::load(&site.db, &roles, &user).await?.roles)
}

/// `WPCode_Snippet_Execute::get_snippet_output()` with the type classes of WPCode Lite.
async fn snippet_output(page: &Page<'_>, snippet: &Snippet) -> Result<String> {
    let code = &snippet.code;
    Ok(match snippet.code_type.as_str() {
        "html" => code.clone(),
        // `wpcode_text_execute_shortcodes` is on.
        "text" => page.services.do_shortcode(page.post, code).await?,
        // `! empty( $code )`: an empty code or `"0"` stays as it is.
        "js" if code.is_empty() || code == "0" => code.clone(),
        "js" => format!("<script>{code}</script>"),
        "css" if code.trim_matches([' ', '\t', '\n', '\r', '\0', '\x0b']).is_empty() => String::new(),
        "css" => format!("<style class=\"wpcode-css-snippet\">{code}</style>"),
        // `eval( '?>' . $code . '<?php ' )`: without PHP in it the code is printed, less the line break that follows
        // the closing tag.
        "universal" if !code.contains("<?") => code.strip_prefix("\r\n").or_else(|| code.strip_prefix('\n')).unwrap_or(code).to_owned(),
        // PHP does not run; `blocks` and `scss` have no class in WPCode Lite.
        _ => String::new(),
    })
}

/// `WPCode_Auto_Insert_Type::output_location()`: the snippets whose rules let them through.
async fn output_location(page: &Page<'_>, snippets: &[Snippet]) -> Result<String> {
    let mut out = String::new();
    for snippet in snippets {
        if rules::shows(page, snippet) {
            out.push_str(&snippet_output(page, snippet).await?);
        }
    }
    Ok(out)
}

impl ActivePlugin for Active {
    fn shortcode_tags(&self) -> &'static [&'static str] {
        &["wpcode"]
    }

    fn shortcode<'a>(&'a self, _tag: &'a str, atts: &'a Atts, _content: Option<&'a str>, ctx: &'a ShortcodeContext<'a>) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            // `0 === $atts['id']`: only a missing ID stops here; `absint()` of any other.
            let Some(id) = atts.get("id") else { return Ok(String::new()) };
            let id = crate::redirection::php::intval(id).unsigned_abs();
            if id == 0 || self.headers_footers_mode {
                return Ok(String::new());
            }
            let site = ctx.site;
            let Some(post) = site.db.post(id).await? else { return Ok(String::new()) };
            if post.post_type != "wpcode" || post.post_status != "publish" {
                return Ok(String::new());
            }
            let meta = site.db.post_meta(id).await?;
            let single = |key: &str| meta.iter().find(|m| m.meta_key.as_deref() == Some(key)).and_then(|m| m.meta_value.as_deref()).map(rustpress_core::options::maybe_unserialize);
            let code_type = site.db.terms_for_post(id, Some("wpcode_type")).await?.into_iter().next().map(|t| t.slug).unwrap_or_default();
            let rules = single("_wpcode_conditional_logic").filter(|v| php::truthy(Some(v))).unwrap_or(Value::Array(Vec::new()));
            let use_rules = single("_wpcode_conditional_logic_enabled").is_some_and(|v| php::truthy(Some(&v)));
            let snippet = Snippet { code: post.post_content.clone(), code_type, use_rules, rules, priority: 10 };
            let global = match ctx.global_post {
                Some(id) => ctx.query.posts.iter().find(|p| p.id == id).cloned().or(site.db.post(id).await?),
                None => None,
            };
            let mut page = Page::new(site, ctx.query, ctx.services, false).await?;
            page.post = global.as_ref();
            if snippet.use_rules {
                page = Page { post: global.as_ref(), ..Page::new(site, ctx.query, ctx.services, true).await? };
            }
            // The rules count only when they are on, and here `show`/`hide` is not asked.
            if snippet.use_rules && !rules::are_met(&page, &snippet.rules) {
                return Ok(String::new());
            }
            snippet_output(&page, &snippet).await
        })
    }

    fn wp_head<'a>(&'a self, ctx: &'a HeadContext<'a>, _robots: &'a [(String, Robot)]) -> BoxFuture<'a, Result<Option<Head>>> {
        Box::pin(async move {
            let rules = [&self.snippets.header, &self.snippets.body, &self.snippets.footer].iter().any(|list| list.iter().any(|s| s.use_rules));
            let page = Page::new(ctx.site, ctx.query, ctx.services, rules).await?;
            let mut head = Head::default();
            // Hooked when the plugin loads: `wp_head` and `wp_footer` at 10, `wp_body_open` at 1.
            head.late.push_str(&self.header);
            head.body_open.push_str(&self.body);
            head.footer.push_str(&self.footer);
            // `WPCode_Auto_Insert_Site_Wide::hooks()` on `wp`, all at priority 10.
            head.on_wp.push_str(&output_location(&page, &self.snippets.header).await?);
            head.body_open.push_str(&output_location(&page, &self.snippets.body).await?);
            head.footer_on_wp.push_str(&output_location(&page, &self.snippets.footer).await?);
            Ok(Some(head))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(serialized: &str) -> Value {
        rustpress_core::options::maybe_unserialize(serialized)
    }

    #[test]
    fn global_scripts_are_unslashed() {
        assert_eq!(global_code(Some(&php::string("<script>var a = \\'x\\';</script>"))), "<script>var a = 'x';</script>");
        assert_eq!(global_code(Some(&php::string(" \n\t"))), "");
        assert_eq!(global_code(Some(&php::string(" 0 "))), "");
        assert_eq!(global_code(None), "");
    }

    #[test]
    fn cached_snippets_are_sorted_by_priority() {
        let cache = value(r#"a:1:{s:16:"site_wide_header";a:3:{i:0;a:3:{s:4:"code";s:1:"a";s:9:"code_type";s:4:"html";s:8:"priority";i:20;}i:1;a:3:{s:4:"code";s:1:"b";s:9:"code_type";s:4:"html";s:8:"priority";i:5;}i:2;a:2:{s:4:"code";s:1:"c";s:9:"code_type";s:4:"html";}}}"#);
        let locations = Locations::from_cache(Some(&cache));
        let codes: Vec<&str> = locations.header.iter().map(|s| s.code.as_str()).collect();
        assert_eq!(codes, ["b", "c", "a"]);
        assert!(locations.body.is_empty() && locations.footer.is_empty());
    }
}
