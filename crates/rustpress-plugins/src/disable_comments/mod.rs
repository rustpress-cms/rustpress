//! Disable Comments 2.9.1 (`disable-comments`), what it changes on the website and in the REST API.
//!
//! The settings are the option `disable_comments_options`. Comments are off for a post when "everywhere" is on
//! (`remove_everywhere`) or its post type is in `disabled_post_types`, unless the viewer's role is excluded
//! (`exclude_by_role`, with `logged-out-users` for visitors); the conditional rules (taxonomy terms, page templates
//! and the auto-close age) override that for single posts. On such a post:
//!
//! - `comments_open()` and `pings_open()` are false, and `get_comments_number()` is 0, or the number of approved
//!   comments of the allowed types (`allowed_comment_types`); `comments_array` keeps only those. With "show existing
//!   comments" (`show_existing_comments`) the number and the list stay as they are.
//! - The post type loses its `trackbacks` support, and its `comments` support unless existing comments or allowed
//!   types are shown (`init_wploaded_filters()`); the REST API leaves `comment_status` and `ping_status` out then.
//! - Its singular page has no post comments feed link (`check_comment_template()` removes `feed_links_extra`).
//!
//! "Everywhere" also removes the recent comments widget and its style, the comments feed link, the comments menu of
//! the toolbar, answers every comment feed with a 403 `wp_die()` (`filter_query()`), and answers the REST routes of
//! `/wp/v2/comments` with `rest_comment_disabled` (`filter_rest_comment_dispatch()`), which "disable comments via the
//! REST API" (`remove_rest_API_comments`) does on its own as well. Requests for an allowed comment type and, without
//! the REST switch, for a post a conditional rule keeps open, pass.
//!
//! Not ported: the settings and tools pages (deleting and exporting comments, the theme scan), the changes of the
//! administration (menus, dashboard, discussion settings, the editor's blocks), the Abilities API, the counter of
//! blocked attempts (written on `shutdown`), the blocklist of comment types (`blocked_comment_types`, which refuses
//! submissions), XML-RPC (`remove_xmlrpc_comments`; RustPress has no `xmlrpc.php`), WooCommerce reviews and network
//! (multisite) settings. See the Disable Comments section of PLAN.md.

use std::collections::{HashMap, HashSet};

use futures::future::BoxFuture;
use rustpress_core::native::{ActivePlugin, CommentsOff, Die, NativePlugin, RestAnswer, RestPreDispatch};
use rustpress_core::{QueryResult, Result, Site, Viewer};
use rustpress_db::Post;
use rustpress_phpser::Value;

use crate::acf::php;

pub struct DisableComments;

/// `DC_VERSION`.
const VERSION: &str = "2.9.1";
const DOMAIN: &str = "disable-comments";

impl NativePlugin for DisableComments {
    fn file(&self) -> &'static str {
        "disable-comments/disable-comments.php"
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn text_domain(&self) -> Option<&'static str> {
        Some(DOMAIN)
    }

    fn load<'a>(&'a self, site: &'a Site) -> BoxFuture<'a, Result<Box<dyn ActivePlugin>>> {
        Box::pin(async move {
            let options = site.options.fetch(&site.db, "disable_comments_options").await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
            let mut settings = Settings::from_option(options.as_ref());
            if settings.db_version < 2 {
                // `check_db_upgrades()` (`DB_VERSION` 8) from 0.2.1 or earlier: the post types came from their own option.
                let old = site.options.fetch(&site.db, "disable_comments_post_types").await?.map(|raw| rustpress_core::options::maybe_unserialize(&raw));
                settings.disabled_post_types = strings(old.as_ref());
            }
            let mut data = Data::default();
            if settings.conditional {
                for rule in &settings.rules {
                    match &rule.test {
                        // `taxonomy_exists()`: a rule of a taxonomy that is gone matches nothing.
                        Test::Terms { taxonomy, terms } if site.types.is_taxonomy(taxonomy) => {
                            let by_term = data.terms.entry(taxonomy.clone()).or_default();
                            for id in terms {
                                if !by_term.contains_key(id) {
                                    by_term.insert(*id, site.db.object_ids_in_terms(taxonomy, &[*id]).await?.into_iter().collect());
                                }
                            }
                        }
                        Test::Template(_) if data.templates.is_empty() => {
                            data.templates = site.db.post_meta_by_key("_wp_page_template").await?.into_iter().collect();
                        }
                        _ => {}
                    }
                }
            }
            let allowed = settings.allowed_types();
            if !allowed.is_empty() {
                data.allowed_counts = site.db.approved_comment_counts_of_types(&allowed).await?.into_iter().collect();
            }
            let closed = site.trd(DOMAIN, "Comments are closed.");
            let disabled = site.trd(DOMAIN, "Comments are disabled.");
            Ok(Box::new(Active { settings, allowed, data, closed, disabled }) as Box<dyn ActivePlugin>)
        })
    }
}

/// The option `disable_comments_options`.
#[derive(Debug, Default)]
struct Settings {
    remove_everywhere: bool,
    disabled_post_types: Vec<String>,
    show_existing_comments: bool,
    remove_rest_api_comments: bool,
    allowed_comment_types: Vec<String>,
    blocked_comment_types: Vec<String>,
    /// `enable_exclude_by_role` and a non-empty `exclude_by_role`: the roles, else empty.
    exclude_by_role: Vec<String>,
    /// `enable_conditional_rules`.
    conditional: bool,
    rules: Vec<Rule>,
    auto_close_days: i64,
    db_version: i64,
}

#[derive(Debug)]
struct Rule {
    enable: bool,
    test: Test,
}

#[derive(Debug)]
enum Test {
    Terms { taxonomy: String, terms: Vec<u64> },
    Template(String),
    /// A rule of an unknown type, or with an empty taxonomy, terms or template: it never matches.
    Never,
}

/// The string values of a PHP array.
fn strings(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_array).map(|items| items.iter().map(|(_, v)| php::strval(Some(v))).collect()).unwrap_or_default()
}

impl Settings {
    fn from_option(option: Option<&Value>) -> Self {
        let Some(option) = option.filter(|o| o.as_array().is_some()) else { return Self::default() };
        let get = |name: &str| option.get(name);
        let exclude_by_role = if php::truthy(get("enable_exclude_by_role")) && php::truthy(get("exclude_by_role")) { strings(get("exclude_by_role")) } else { Vec::new() };
        let conditional = php::truthy(get("enable_conditional_rules"));
        let rules = get("conditional_rules").and_then(Value::as_array).map(|rules| rules.iter().map(|(_, rule)| Rule::from_value(rule)).collect()).unwrap_or_default();
        Self {
            remove_everywhere: php::truthy(get("remove_everywhere")),
            disabled_post_types: strings(get("disabled_post_types")),
            show_existing_comments: php::truthy(get("show_existing_comments")),
            remove_rest_api_comments: get("remove_rest_API_comments").is_some_and(|v| !matches!(v, Value::Null)) && php::intval(get("remove_rest_API_comments")) == 1,
            allowed_comment_types: if get("allowed_comment_types").and_then(Value::as_array).is_some() { strings(get("allowed_comment_types")) } else { Vec::new() },
            blocked_comment_types: if get("blocked_comment_types").and_then(Value::as_array).is_some() { strings(get("blocked_comment_types")) } else { Vec::new() },
            exclude_by_role,
            conditional,
            rules,
            auto_close_days: if conditional { php::intval(get("auto_close_days")).max(0) } else { 0 },
            db_version: php::intval(get("db_version")),
        }
    }

    /// `get_allowed_comment_types()`: the allowlist less the blocklist.
    fn allowed_types(&self) -> Vec<String> {
        self.allowed_comment_types.iter().filter(|t| !self.blocked_comment_types.contains(t)).cloned().collect()
    }

    /// `has_conditional_rules()`.
    fn has_conditional_rules(&self) -> bool {
        self.conditional && (!self.rules.is_empty() || self.auto_close_days > 0)
    }
}

impl Rule {
    /// A rule as `sanitize_conditional_rules()` stored it.
    fn from_value(rule: &Value) -> Self {
        let field = |name: &str| php::strval(rule.get(name));
        let enable = field("action") == "enable";
        let test = match field("type").as_str() {
            "taxonomy" => {
                let terms: Vec<u64> = rule.get("terms").map_or_else(Vec::new, |t| match t.as_array() {
                    Some(items) => items.iter().map(|(_, v)| php::intval(Some(v))).filter(|id| *id > 0).map(|id| id as u64).collect(),
                    None => vec![php::intval(Some(t))].into_iter().filter(|id| *id > 0).map(|id| id as u64).collect(),
                });
                let taxonomy = field("taxonomy");
                if taxonomy.is_empty() || terms.is_empty() { Test::Never } else { Test::Terms { taxonomy, terms } }
            }
            "template" => {
                let template = field("template");
                if template.is_empty() { Test::Never } else { Test::Template(template) }
            }
            _ => Test::Never,
        };
        Self { enable, test }
    }
}

/// What the rules need to know of the posts, read once per snapshot.
#[derive(Default)]
struct Data {
    /// The objects of each term of the taxonomy rules, by taxonomy (only taxonomies that exist).
    terms: HashMap<String, HashMap<u64, HashSet<u64>>>,
    /// `_wp_page_template` of the posts that have one.
    templates: HashMap<u64, String>,
    /// The approved comments of the allowed types per post.
    allowed_counts: HashMap<u64, u64>,
}

struct Active {
    settings: Settings,
    /// `get_allowed_comment_types()`.
    allowed: Vec<String>,
    data: Data,
    closed: String,
    disabled: String,
}

impl Active {
    /// `is_exclude_by_role()`.
    fn is_exclude_by_role(&self, viewer: Option<&Viewer>) -> bool {
        let roles = &self.settings.exclude_by_role;
        if roles.is_empty() {
            return false;
        }
        match viewer {
            Some(viewer) => roles.iter().any(|r| viewer.roles.contains(r)) || (roles.iter().any(|r| r == "administrator") && viewer.can("delete_users")),
            None => roles.iter().any(|r| r == "logged-out-users"),
        }
    }

    /// `is_remove_everywhere()`.
    fn is_remove_everywhere(&self, viewer: Option<&Viewer>) -> bool {
        !self.is_exclude_by_role(viewer) && self.settings.remove_everywhere
    }

    /// `is_post_type_disabled()` without the role check.
    fn post_type_disabled(&self, post_type: &str) -> bool {
        !post_type.is_empty() && self.settings.disabled_post_types.iter().any(|t| php::loose_eq(&php::string(t), &php::string(post_type)))
    }

    /// `conditional_rule_matches()`.
    fn rule_matches(&self, rule: &Rule, post: &Post) -> bool {
        match &rule.test {
            Test::Terms { taxonomy, terms } => self.data.terms.get(taxonomy).is_some_and(|by_term| terms.iter().any(|id| by_term.get(id).is_some_and(|objects| objects.contains(&post.id)))),
            Test::Template(template) => {
                // `get_page_template_slug()`: no template and `default` are ''.
                let slug = self.data.templates.get(&post.id).filter(|t| !t.is_empty() && *t != "default").map_or("", String::as_str);
                template == slug
            }
            Test::Never => false,
        }
    }

    /// `is_past_auto_close()`.
    fn is_past_auto_close(&self, post: &Post) -> bool {
        let days = self.settings.auto_close_days;
        if days < 1 {
            return false;
        }
        // `get_post_time( 'U', true )`: false for a post without a date.
        let Some(published) = rustpress_core::datetime::SiteTimezone::gmt_timestamp(&post.post_date_gmt).filter(|_| post.post_date_gmt != "0000-00-00 00:00:00") else { return false };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        now - published > days * 86_400
    }

    /// `match_conditional_rules()`: `Some(true)` for an "enable" rule, `Some(false)` for "disable".
    fn match_conditional_rules(&self, post: &Post) -> Option<bool> {
        let mut result = None;
        if self.settings.conditional {
            for rule in &self.settings.rules {
                if !self.rule_matches(rule, post) {
                    continue;
                }
                if rule.enable {
                    return Some(true);
                }
                result = Some(false);
            }
        }
        if result.is_none() && self.is_past_auto_close(post) {
            result = Some(false);
        }
        result
    }

    /// `is_disabled_for_post()`.
    fn is_disabled_for_post(&self, post: &Post, viewer: Option<&Viewer>) -> bool {
        if self.is_exclude_by_role(viewer) {
            return false;
        }
        let disabled = self.settings.remove_everywhere || self.post_type_disabled(&post.post_type);
        if !self.settings.has_conditional_rules() {
            return disabled;
        }
        match self.match_conditional_rules(post) {
            Some(enable) => !enable,
            None => disabled,
        }
    }

    /// Whether the REST filters are on (`init_filters()`): "everywhere" or the REST switch.
    fn blocks_rest(&self, viewer: Option<&Viewer>) -> bool {
        self.is_remove_everywhere(viewer) || self.settings.remove_rest_api_comments
    }
}

impl ActivePlugin for Active {
    /// `filter_comment_status()`, `filter_comments_number()` and `filter_existing_comments()`.
    fn comments_off(&self, post: &Post, viewer: Option<&Viewer>) -> Option<CommentsOff> {
        if !self.is_disabled_for_post(post, viewer) {
            return None;
        }
        if self.settings.show_existing_comments {
            return Some(CommentsOff { count: None, keep_types: None });
        }
        let count = if self.allowed.is_empty() { 0 } else { self.data.allowed_counts.get(&post.id).copied().unwrap_or(0) as i64 };
        Some(CommentsOff { count: Some(count), keep_types: Some(self.allowed.clone()) })
    }

    fn comments_off_everywhere(&self, viewer: Option<&Viewer>) -> bool {
        self.is_remove_everywhere(viewer)
    }

    /// `init_wploaded_filters()` (for a visitor: the support is taken once per request).
    fn removes_post_type_support(&self, post_type: &str, feature: &str) -> bool {
        if !self.post_type_disabled(post_type) || self.is_exclude_by_role(None) {
            return false;
        }
        feature == "trackbacks" || (feature == "comments" && !self.settings.show_existing_comments && self.allowed.is_empty())
    }

    /// `check_comment_template()` on a singular page.
    fn closes_singular_comments(&self, post: &Post, viewer: Option<&Viewer>) -> bool {
        self.is_disabled_for_post(post, viewer)
    }

    /// `filter_query()`: every comment feed is a 403.
    fn template_redirect_die(&self, _site: &Site, query: &QueryResult) -> Option<Die> {
        if !self.is_remove_everywhere(query.viewer.as_ref()) || !query.is_comment_feed() {
            return None;
        }
        Some(Die { title: String::new(), message: rustpress_core::formatting::esc_html(&self.closed), status: 403 })
    }

    /// `filter_rest_comment_query()`. The `post` parameter is a list by now, so the conditional exception of
    /// `get_request_post_id()` never applies to the collection.
    fn rest_comment_query_empty<'a>(&'a self, request: &'a RestPreDispatch<'a>) -> BoxFuture<'a, Result<bool>> {
        Box::pin(async move { Ok(self.blocks_rest(request.user) && !self.is_allowed_type_request(request).await?) })
    }

    /// `filter_rest_comment_dispatch()`.
    fn rest_pre_dispatch<'a>(&'a self, request: &'a RestPreDispatch<'a>) -> BoxFuture<'a, Result<Option<RestAnswer>>> {
        Box::pin(async move {
            if !self.blocks_rest(request.user) || !request.route.contains("/wp/v2/comments") {
                return Ok(None);
            }
            if self.is_allowed_type_request(request).await? {
                return Ok(None);
            }
            // `rest_blocking_is_conditional()`: a post an exception keeps open, unless the REST switch blocks.
            if self.settings.has_conditional_rules() && !self.settings.remove_rest_api_comments {
                let post_id = self.request_post_id(request).await?;
                if post_id > 0
                    && let Some(post) = request.site.db.post(post_id).await?
                    && !self.is_disabled_for_post(&post, request.user)
                {
                    return Ok(None);
                }
            }
            Ok(Some(RestAnswer::error("rest_comment_disabled", &self.disabled, 403)))
        })
    }
}

impl Active {
    /// `$request->get_param( $name )` before the route's own parameters are known.
    fn param<'a>(request: &'a RestPreDispatch<'_>, name: &str) -> Option<&'a str> {
        request.params.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// `is_allowed_comment_type_request()`.
    async fn is_allowed_type_request(&self, request: &RestPreDispatch<'_>) -> Result<bool> {
        let mut comment_type = Self::param(request, "type").filter(|t| !t.is_empty() && *t != "0").map(str::to_owned);
        if comment_type.is_none()
            && let Some(id) = Self::param(request, "id").filter(|t| !t.is_empty() && *t != "0")
            && let Ok(id) = id.trim().parse::<u64>()
            && let Some(comment) = request.site.db.comment_any(id).await?
        {
            comment_type = Some(comment.comment_type);
        }
        if comment_type.is_none() && request.method == "DELETE" {
            let last = request.route.rsplit('/').next().unwrap_or_default();
            if let Ok(id) = last.parse::<u64>()
                && let Some(comment) = request.site.db.comment_any(id).await?
            {
                comment_type = Some(comment.comment_type);
            }
        }
        Ok(comment_type.filter(|t| !t.is_empty() && t != "0").is_some_and(|t| self.allowed.contains(&t)))
    }

    /// `get_request_post_id()` without a prepared comment.
    async fn request_post_id(&self, request: &RestPreDispatch<'_>) -> Result<u64> {
        if let Some(post) = Self::param(request, "post").and_then(|p| p.trim().parse::<i64>().ok()).filter(|p| *p > 0) {
            return Ok(post as u64);
        }
        if let Some(id) = Self::param(request, "id").and_then(|p| p.trim().parse::<i64>().ok()).filter(|p| *p > 0)
            && let Some(comment) = request.site.db.comment_any(id as u64).await?
        {
            return Ok(comment.comment_post_id);
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(text: &str) -> Value {
        rustpress_core::options::maybe_unserialize(text)
    }

    #[test]
    fn settings_follow_the_option() {
        let value = option(
            r#"a:6:{s:17:"remove_everywhere";b:0;s:19:"disabled_post_types";a:1:{i:0;s:4:"page";}s:22:"show_existing_comments";b:0;s:24:"remove_rest_API_comments";i:1;s:21:"allowed_comment_types";a:2:{i:0;s:4:"note";i:1;s:6:"review";}s:21:"blocked_comment_types";a:1:{i:0;s:6:"review";}}"#,
        );
        let settings = Settings::from_option(Some(&value));
        assert!(!settings.remove_everywhere);
        assert_eq!(settings.disabled_post_types, ["page"]);
        assert!(settings.remove_rest_api_comments);
        assert_eq!(settings.allowed_types(), ["note"]);
    }

    #[test]
    fn rules_drop_what_cannot_match() {
        let rule = Rule::from_value(&option(r#"a:4:{s:4:"type";s:8:"taxonomy";s:6:"action";s:6:"enable";s:8:"taxonomy";s:8:"category";s:5:"terms";a:2:{i:0;i:3;i:1;s:1:"0";}}"#));
        assert!(rule.enable);
        assert!(matches!(rule.test, Test::Terms { ref terms, .. } if terms == &[3]));
        let rule = Rule::from_value(&option(r#"a:3:{s:4:"type";s:8:"template";s:6:"action";s:7:"disable";s:8:"template";s:0:"";}"#));
        assert!(!rule.enable);
        assert!(matches!(rule.test, Test::Never));
    }
}
