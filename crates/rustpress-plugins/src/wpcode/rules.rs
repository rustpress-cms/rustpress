//! The conditional logic of WPCode Lite (`WPCode_Conditional_Logic`, `WPCode_Conditional_Type` and its types).

use rustpress_core::query::Queried;
use rustpress_phpser::Value;

use super::{Page, Snippet};
use crate::acf::php;

/// `WPCode_Conditional_Logic::check_snippets_conditions()` for one snippet: whether it is printed.
pub(super) fn shows(page: &Page<'_>, snippet: &Snippet) -> bool {
    if !snippet.use_rules {
        return true;
    }
    let Some(show) = snippet.rules.get("show").filter(|v| !matches!(v, Value::Null)) else { return false };
    let show = show.as_bytes() == Some(b"show".as_slice());
    are_met(page, &snippet.rules) == show
}

/// `are_snippet_rules_met()`: no groups, or one group whose rows all match.
pub(super) fn are_met(page: &Page<'_>, rules: &Value) -> bool {
    let Some(groups) = rules.as_array().and_then(|_| rules.get("groups")).filter(|g| php::truthy(Some(g))) else { return true };
    let Some(groups) = groups.as_array() else { return false };
    groups.iter().any(|(_, group)| group.as_array().is_none_or(|rows| rows.iter().all(|(_, row)| row_matches(page, row))))
}

/// `are_group_rules_met()` for one row: rows of an unknown type are skipped.
fn row_matches(page: &Page<'_>, row: &Value) -> bool {
    let Some(kind) = row.get("type").filter(|v| !matches!(v, Value::Null)) else { return true };
    let option = php::strval(row.get("option"));
    let relation = php::strval(row.get("relation"));
    let expected = row.get("value").cloned().unwrap_or(Value::Null);
    let actual = match php::strval(Some(kind)).as_str() {
        "user" => match option.as_str() {
            "logged_in" => Some(Value::Bool(page.query.viewer.is_some())),
            "user_role" => Some(Value::Array(page.roles.iter().enumerate().map(|(i, r)| (rustpress_phpser::Key::Int(i as i64), php::string(r))).collect())),
            // A Pro option: it has no callback.
            "user_meta" => None,
            _ => return true,
        },
        "page" => match option.as_str() {
            "type_of_page" => Some(php::string(type_of_page(&page.query.flags))),
            "post_type" => Some(page.post.map_or(Value::Bool(false), |p| php::string(&p.post_type))),
            "referrer" => Some(php::string(page.query.referer.as_deref().unwrap_or_default())),
            "taxonomy_page" => Some(php::string(match &page.query.queried {
                Queried::Term(term) => &term.taxonomy,
                _ => "",
            })),
            "taxonomy_term" => Some(ids(&page.terms)),
            "page_url" => Some(php::string(&page_url(page))),
            "post_meta" | "post_id" | "page_template" | "post_author" => None,
            _ => return true,
        },
        // The types of WPCode Pro that Lite lists with an upgrade notice: their options have no callback, other options
        // pass.
        "device" if ["device_type", "browser", "os", "cookie_name", "cookie_value"].contains(&option.as_str()) => None,
        "location" if ["country", "continent"].contains(&option.as_str()) => None,
        "schedule" if ["date_is", "time_is", "weekday", "time"].contains(&option.as_str()) => None,
        "snippet" if ["snippet_loaded"].contains(&option.as_str()) => None,
        "woocommerce" if ["wc_page", "wc_cart"].contains(&option.as_str()) => None,
        "edd" if ["edd_page"].contains(&option.as_str()) => None,
        "memberpress" if ["memberpress_page", "memberpress_user"].contains(&option.as_str()) => None,
        _ => return true,
    };
    let Some(actual) = actual else { return false };
    compare(&actual, &expected, &relation)
}

/// `get_type_of_page()`.
fn type_of_page(flags: &rustpress_core::query::Flags) -> &'static str {
    if flags.is_front_page {
        "is_front_page"
    } else if flags.is_home {
        "is_home"
    } else if flags.is_singular() {
        "is_single"
    } else if flags.is_author {
        "is_author"
    } else if flags.is_archive {
        "is_archive"
    } else if flags.is_search {
        "is_search"
    } else if flags.is_404 {
        "is_404"
    } else {
        ""
    }
}

fn ids(terms: &[u64]) -> Value {
    Value::Array(terms.iter().enumerate().map(|(i, id)| (rustpress_phpser::Key::Int(i as i64), Value::Int(*id as i64))).collect())
}

/// `get_page_url()` on the website: `trailingslashit( home_url( $wp->request ) )` (`$wp->request` is only set with
/// pretty permalinks) with every query parameter added again, `sanitize_key()` of the name and
/// `sanitize_text_field()` of the value.
fn page_url(page: &Page<'_>) -> String {
    let site = page.site;
    let mut url = if site.settings.permalink_structure.is_empty() {
        String::new()
    } else {
        let home_path = site.settings.home.split_once("://").and_then(|(_, rest)| rest.find('/').map(|i| &rest[i..])).unwrap_or("").trim_matches('/');
        let request = page.query.url_path.trim_matches('/');
        let request = request.strip_prefix(home_path).unwrap_or(request).trim_matches('/');
        rustpress_core::urls::trailingslashit(&site.home_url(request))
    };
    for (key, value) in rustpress_core::canonical::parse_qs(&page.query.raw_query) {
        let key = rustpress_content::sanitize::key(&key);
        url = rustpress_core::urls::add_query_arg(&url, &key, &rustpress_content::sanitize::text_field(&value));
    }
    url
}

/// `get_relation_comparison()`.
fn compare(actual: &Value, expected: &Value, relation: &str) -> bool {
    match relation {
        "=" => equals(actual, expected),
        "!=" => does_not_equal(actual, expected),
        "contains" => contains(actual, expected),
        "notcontains" => !contains(actual, expected),
        _ => true,
    }
}

fn items(value: &Value) -> Option<Vec<&Value>> {
    value.as_array().map(|items| items.iter().map(|(_, v)| v).collect())
}

/// `array_intersect()` compares the string forms.
fn intersects(a: &[&Value], b: &[&Value]) -> bool {
    a.iter().any(|x| b.iter().any(|y| php::strval(Some(x)) == php::strval(Some(y))))
}

/// `in_array()` with loose comparison.
fn in_array(needle: &Value, haystack: &[&Value]) -> bool {
    haystack.iter().any(|v| php::loose_eq(needle, v))
}

fn equals(a: &Value, b: &Value) -> bool {
    match (items(a), items(b)) {
        (Some(x), Some(y)) => intersects(&x, &y),
        (Some(x), None) => in_array(b, &x),
        (None, Some(y)) => in_array(a, &y),
        (None, None) => php::loose_eq(a, b),
    }
}

fn does_not_equal(a: &Value, b: &Value) -> bool {
    match (items(a), items(b)) {
        (Some(x), Some(y)) => !intersects(&x, &y),
        (Some(x), None) => !in_array(b, &x),
        (None, Some(y)) => !in_array(a, &y),
        (None, None) => !php::loose_eq(a, b),
    }
}

/// `contains()`: `strpos()` of the string forms; an empty needle never matches.
fn contains(haystack: &Value, needle: &Value) -> bool {
    if !php::truthy(Some(needle)) || haystack.as_array().is_some() {
        return false;
    }
    php::strval(Some(haystack)).contains(&php::strval(Some(needle)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparisons_are_loose() {
        let roles = Value::Array(vec![(rustpress_phpser::Key::Int(0), php::string("editor"))]);
        assert!(equals(&roles, &php::string("editor")));
        assert!(!equals(&roles, &php::string("administrator")));
        assert!(does_not_equal(&roles, &php::string("administrator")));
        assert!(equals(&Value::Bool(true), &Value::Bool(true)));
        assert!(equals(&Value::Bool(false), &Value::Bool(false)));
        assert!(!equals(&Value::Bool(false), &Value::Bool(true)));
        // `true == "false"` in PHP.
        assert!(equals(&Value::Bool(true), &php::string("false")));
        assert!(contains(&php::string("https://example.org/?ref=news"), &php::string("ref=")));
        assert!(!contains(&php::string("abc"), &php::string("")));
        assert!(equals(&ids(&[3, 7]), &Value::Array(vec![(rustpress_phpser::Key::Int(0), php::string("7"))])));
    }
}
