//! Template tags of WordPress core that classic themes call (`paginate_links()`,
//! `the_posts_pagination()`, `get_the_category_list()`, ...).

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::{QueryResult, Site};
use rustpress_db::{Post, Term};

use crate::Result;

/// `_navigation_markup()`.
pub fn navigation_markup(links: &str, css_class: &str, screen_reader_text: &str, aria_label: &str) -> String {
    let aria = if aria_label.is_empty() { screen_reader_text } else { aria_label };
    format!(
        "\n\t<nav class=\"navigation {}\" aria-label=\"{}\">\n\t\t<h2 class=\"screen-reader-text\">{}</h2>\n\t\t<div class=\"nav-links\">{links}</div>\n\t</nav>",
        css_class.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect::<String>(),
        esc_attr(aria),
        esc_html(screen_reader_text)
    )
}

/// The arguments of `the_posts_pagination()` the themes pass.
pub struct Pagination {
    pub prev_text: String,
    pub next_text: String,
    pub before_page_number: String,
}

/// `get_the_posts_pagination()` for the main query.
pub fn posts_pagination(site: &Site, q: &QueryResult, args: &Pagination) -> String {
    if q.max_pages <= 1 {
        return String::new();
    }
    let title = site.tr("Posts pagination");
    let request = format!("{}{}", q.url_path, if q.raw_query.is_empty() { String::new() } else { format!("?{}", q.raw_query) });
    let total = q.max_pages;
    let current = q.paged.max(1);
    let mid_size = 1;
    let end_size = 1;
    let mut links: Vec<String> = Vec::new();
    // `paginate_links()`: with plain permalinks `?paged=N` comes first, then the arguments of the request.
    let plain_args: Vec<(String, String)> = q
        .raw_query
        .split('&')
        .filter_map(|pair| pair.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())).or_else(|| (!pair.is_empty()).then(|| (pair.to_owned(), String::new()))))
        .filter(|(k, _)| k != "paged")
        .collect();
    let page_link = |n: u64| {
        if site.uses_pretty_permalinks() {
            return esc_url(&site.pagenum_link(&request, n));
        }
        let mut link = if n == 1 { site.home_url("/") } else { format!("{}?paged={n}", site.home_url("/")) };
        for (k, v) in &plain_args {
            link = rustpress_core::urls::add_query_arg(&link, k, v);
        }
        esc_url(&link)
    };
    if current > 1 {
        links.push(format!("<a class=\"prev page-numbers\" href=\"{}\">{}</a>", page_link(current - 1), args.prev_text));
    }
    let mut dots = false;
    for n in 1..=total {
        let label = format!("{}{}", args.before_page_number, site.number_i18n(n));
        if n == current {
            links.push(format!("<span aria-current=\"page\" class=\"page-numbers current\">{label}</span>"));
            dots = true;
        } else if n <= end_size || (n + mid_size >= current && n <= current + mid_size) || n > total.saturating_sub(end_size) {
            links.push(format!("<a class=\"page-numbers\" href=\"{}\">{label}</a>", page_link(n)));
            dots = true;
        } else if dots {
            links.push("<span class=\"page-numbers dots\">&hellip;</span>".to_owned());
            dots = false;
        }
    }
    if current < total {
        links.push(format!("<a class=\"next page-numbers\" href=\"{}\">{}</a>", page_link(current + 1), args.next_text));
    }
    navigation_markup(&links.join("\n"), "pagination", &title, &title)
}

/// `the_comments_pagination()`: the links to the other pages of comments.
pub fn comments_pagination(site: &Site, permalink: &str, request_uri: &str, current: u64, total: u64, prev_text: &str, next_text: &str) -> String {
    if total < 2 {
        return String::new();
    }
    let page_link = |n: u64| {
        let base = if site.uses_pretty_permalinks() {
            site.user_trailingslashit(&format!("{}comment-page-{n}", rustpress_core::urls::trailingslashit(permalink)))
        } else {
            rustpress_core::urls::add_query_arg(request_uri, "cpage", &n.to_string())
        };
        esc_url(&format!("{base}#comments"))
    };
    let (mid_size, end_size) = (2, 1);
    let mut links: Vec<String> = Vec::new();
    if current > 1 {
        links.push(format!("<a class=\"prev page-numbers\" href=\"{}\">{prev_text}</a>", page_link(current - 1)));
    }
    let mut dots = false;
    for n in 1..=total {
        let label = site.number_i18n(n);
        if n == current {
            links.push(format!("<span aria-current=\"page\" class=\"page-numbers current\">{label}</span>"));
            dots = true;
        } else if n <= end_size || (n + mid_size >= current && n <= current + mid_size) || n > total.saturating_sub(end_size) {
            links.push(format!("<a class=\"page-numbers\" href=\"{}\">{label}</a>", page_link(n)));
            dots = true;
        } else if dots {
            links.push("<span class=\"page-numbers dots\">&hellip;</span>".to_owned());
            dots = false;
        }
    }
    if current < total {
        links.push(format!("<a class=\"next page-numbers\" href=\"{}\">{next_text}</a>", page_link(current + 1)));
    }
    let title = site.tr("Comments pagination");
    navigation_markup(&links.join("\n"), "comments-pagination", &title, &title)
}

/// `get_the_category_list( $separator )`: categories sorted by name.
pub async fn category_list(site: &Site, post: &Post, separator: &str) -> Result<String> {
    let mut terms = site.db.terms_for_post(post.id, Some("category")).await?;
    if terms.is_empty() {
        // `get_the_category_list()` names "Uncategorized" without a link.
        return Ok(site.tr("Uncategorized"));
    }
    terms.sort_by_key(|t| t.name.to_lowercase());
    let rel = if site.uses_pretty_permalinks() { "category tag" } else { "category" };
    let mut links = Vec::new();
    for term in &terms {
        links.push(format!("<a href=\"{}\" rel=\"{rel}\">{}</a>", esc_url(&site.term_link(term).await?), term.name));
    }
    Ok(links.join(separator))
}

/// `get_the_tag_list( '', $separator )`.
pub async fn tag_list(site: &Site, post: &Post, separator: &str) -> Result<String> {
    let mut terms: Vec<Term> = site.db.terms_for_post(post.id, Some("post_tag")).await?;
    terms.sort_by_key(|t| t.name.to_lowercase());
    let mut links = Vec::new();
    for term in &terms {
        links.push(format!("<a href=\"{}\" rel=\"tag\">{}</a>", esc_url(&site.term_link(term).await?), term.name));
    }
    Ok(links.join(separator))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_markup_names_the_nav_and_hides_the_heading() {
        let html = navigation_markup("<a>1</a>", "pagination", "Posts pagination", "");
        assert!(html.contains("<nav class=\"navigation pagination\" aria-label=\"Posts pagination\">"));
        assert!(html.contains("<h2 class=\"screen-reader-text\">Posts pagination</h2>"));
        assert!(html.contains("<div class=\"nav-links\"><a>1</a></div>"));
    }
}
