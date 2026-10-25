//! The block patterns WordPress has registered: its own (`wp-includes/block-patterns`) and the theme's
//! (`patterns/*.php`, headers read like `WP_Theme::get_block_patterns()`).

use crate::phpt::{self, PhpHost};
use crate::theme::Theme;

/// One entry of `WP_Block_Patterns_Registry`.
#[derive(Debug, Clone, Default)]
pub struct PatternEntry {
    pub name: String,
    pub title: String,
    pub content: String,
    pub description: String,
    pub viewport_width: Option<i64>,
    pub inserter: Option<bool>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub block_types: Vec<String>,
    pub post_types: Vec<String>,
    pub template_types: Vec<String>,
    pub source: Option<String>,
}

/// The patterns `_register_core_block_patterns_and_categories()` registers, in its order.
const CORE_PATTERNS: &[&str] = &[
    "query-standard-posts",
    "query-medium-posts",
    "query-small-posts",
    "query-grid-posts",
    "query-large-title-posts",
    "query-offset-posts",
    "navigation-overlay",
    "navigation-overlay-black-bg",
    "navigation-overlay-accent-bg",
    "navigation-overlay-centered",
    "navigation-overlay-centered-with-extras",
];

/// A comma-separated header as a list.
fn list(text: &str) -> Vec<String> {
    text.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

/// The strings of `array( 'a', 'b' )` after `key =>`.
fn php_array(source: &str, key: &str) -> Vec<String> {
    let Some(at) = source.find(&format!("'{key}'")) else { return Vec::new() };
    let rest = &source[at..];
    let Some(open) = rest.find("array(") else { return Vec::new() };
    let Some(close) = rest[open..].find(')') else { return Vec::new() };
    rest[open + 6..open + close].split(',').map(|s| s.trim().trim_matches('\'').trim_matches('"').to_owned()).filter(|s| !s.is_empty()).collect()
}

impl Theme {
    /// Everything in the pattern registry: WordPress' patterns first, then those of the theme and its parent.
    pub fn registered_patterns(&self) -> Vec<PatternEntry> {
        let mut out: Vec<PatternEntry> = Vec::new();
        let dir = self.wp_root.join("wp-includes").join("block-patterns");
        for name in CORE_PATTERNS {
            let Ok(source) = std::fs::read_to_string(dir.join(format!("{name}.php"))) else { continue };
            let title = source
                .find("'title'")
                .and_then(|at| source[at..].find("_x(").map(|x| &source[at + x + 3..]))
                .and_then(|rest| {
                    let text = rest.trim_start().strip_prefix('\'')?;
                    let end = text.find('\'')?;
                    Some(text[..end].to_owned())
                })
                .unwrap_or_default();
            let title = self.translate_with_context(&title, "Block pattern title", "default");
            let content = phpt::pattern_array_content(&source, self.host_ref()).unwrap_or_default();
            out.push(PatternEntry {
                name: format!("core/{name}"),
                title,
                content,
                categories: php_array(&source, "categories"),
                block_types: php_array(&source, "blockTypes"),
                source: Some("core".into()),
                ..Default::default()
            });
        }
        let mut themes = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            themes.push(self.template_dir().to_path_buf());
        }
        for theme_dir in themes {
            let text_domain = crate::theme_headers::text_domain(&theme_dir);
            let Ok(entries) = std::fs::read_dir(theme_dir.join("patterns")) else { continue };
            let mut files: Vec<_> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("php")).collect();
            files.sort();
            for path in files {
                let Ok(source) = std::fs::read_to_string(&path) else { continue };
                let head: String = source.chars().take(8192).collect();
                let header = |name: &str| rustpress_header(&head, name);
                let (title, slug) = (header("Title"), header("Slug"));
                if title.is_empty() || slug.is_empty() || out.iter().any(|p| p.name == slug) {
                    continue;
                }
                let content = if source.contains("return array(") && source.contains("'content'") { phpt::pattern_array_content(&source, self.host_ref()) } else { phpt::render(&source, self.host_ref()) }.unwrap_or_default();
                let description = header("Description");
                out.push(PatternEntry {
                    name: slug,
                    title: self.translate_with_context(&title, "Pattern title", &text_domain),
                    content,
                    description: if description.is_empty() { String::new() } else { self.translate_with_context(&description, "Pattern description", &text_domain) },
                    viewport_width: header("Viewport Width").parse().ok(),
                    inserter: match header("Inserter").to_lowercase().as_str() {
                        "" => None,
                        "no" | "false" | "0" => Some(false),
                        _ => Some(true),
                    },
                    categories: list(&header("Categories")),
                    keywords: list(&header("Keywords")),
                    block_types: list(&header("Block Types")),
                    post_types: list(&header("Post Types")),
                    template_types: list(&header("Template Types")),
                    source: None,
                });
            }
        }
        out
    }

    fn translate_with_context(&self, text: &str, context: &str, domain: &str) -> String {
        self.host_ref().translate(text, Some(context), domain)
    }
}

/// `Header: value` in the first lines of a pattern file.
fn rustpress_header(head: &str, name: &str) -> String {
    head.lines()
        .find_map(|line| {
            let line = line.trim().trim_start_matches(['*', '/', '#']).trim();
            let (key, value) = line.split_once(':')?;
            (key.trim().eq_ignore_ascii_case(name)).then(|| value.trim().to_owned())
        })
        .unwrap_or_default()
}
