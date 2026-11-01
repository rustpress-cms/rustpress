//! The breadcrumbs of Yoast SEO: `Breadcrumbs_Presenter`, the `yoast-seo/breadcrumbs` block and the
//! `[wpseo_breadcrumb]` shortcode.

use rustpress_core::Result;

use super::presentation::Presentation;
use super::{Page, php_trim, schema};

/// `Breadcrumbs_Presenter::present()` (without filters: the wrapper and the element are `span`, no id or class).
pub async fn present(page: &Page<'_>, p: &Presentation) -> Result<String> {
    let crumbs = schema::breadcrumbs(page, p).await?;
    let total = crumbs.len();
    // `should_link_target_blank()`.
    let blank = page.get.iter().any(|(k, v)| k == "context" && v == "edit");
    let links: Vec<String> = crumbs
        .iter()
        .enumerate()
        .map(|(index, crumb)| {
            if crumb.text.is_empty() {
                return String::new();
            }
            let text = php_trim(&crumb.text);
            let url = crumb.url.as_deref().filter(|url| !url.is_empty());
            let link = match url {
                Some(url) if index + 1 < total => {
                    let target = if blank { " target=\"_blank\"" } else { "" };
                    format!("<span><a{target} href=\"{}\">{text}</a></span>", rustpress_core::formatting::esc_url(url))
                }
                _ if index + 1 == total => {
                    let text = if page.options.is_true("breadcrumbs-boldlast") { format!("<strong>{text}</strong>") } else { text.to_owned() };
                    format!("<span class=\"breadcrumb_last\" aria-current=\"page\">{text}</span>")
                }
                _ => format!("<span>{text}</span>"),
            };
            php_trim(&link).to_owned()
        })
        .filter(|link| !link.is_empty() && link != "0")
        .collect();
    let output = links.join(&format!(" {} ", page.options.string("breadcrumbs-sep")));
    if output.is_empty() || output == "0" {
        return Ok(String::new());
    }
    let mut output = format!("<span>{output}</span>");
    let prefix = page.options.string("breadcrumbs-prefix");
    if !prefix.is_empty() {
        output = format!("\t{prefix}\n{output}");
    }
    Ok(output)
}

/// `Breadcrumbs_Block::present()` on the front end.
pub fn block(attrs: &serde_json::Map<String, serde_json::Value>, presented: &str) -> String {
    let mut class = "yoast-breadcrumbs".to_owned();
    let name = match attrs.get("className") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(true)) => "1".to_owned(),
        _ => String::new(),
    };
    if !name.is_empty() && name != "0" {
        class.push(' ');
        class.push_str(&rustpress_core::formatting::esc_attr(&name));
    }
    format!("<div class=\"{class}\">{presented}</div>")
}
