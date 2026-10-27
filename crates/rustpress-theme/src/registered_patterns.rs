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
    /// Patterns of the pattern directory have all their fields set, even the empty ones.
    pub all_fields: bool,
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

/// A header of the pattern file as a list.
fn list(text: &str) -> Vec<String> {
    // `wp_parse_list()`: commas and white space separate the items.
    text.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(str::to_owned).collect()
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
                        "yes" | "true" => Some(true),
                        _ => Some(false),
                    },
                    categories: list(&header("Categories")),
                    keywords: list(&header("Keywords")),
                    block_types: list(&header("Block Types")),
                    post_types: list(&header("Post Types")),
                    template_types: list(&header("Template Types")),
                    source: None,
                    all_fields: false,
                });
            }
        }
        out
    }

    fn translate_with_context(&self, text: &str, context: &str, domain: &str) -> String {
        self.host_ref().translate(text, Some(context), domain)
    }
}

/// A style `register_block_style()` knows: `(block type, name, label, is default)`.
pub type BlockStyle = (String, String, String, bool);

impl Theme {
    /// `WP_Block_Styles_Registry::get_all_registered()`: what the `functions.php` of the theme (and its parent) registers
    /// with `register_block_style()`, then the variations of `styles/`, grouped by block in the order they were first seen.
    pub fn registered_block_styles(&self) -> Vec<(String, Vec<BlockStyle>)> {
        self.registered_block_styles_with(true)
    }

    /// [`Theme::registered_block_styles`], optionally without the variations of `styles/`: WordPress registers those
    /// only once something asks for the theme's `theme.json` data.
    pub fn registered_block_styles_with(&self, partials: bool) -> Vec<(String, Vec<BlockStyle>)> {
        let mut flat: Vec<BlockStyle> = Vec::new();
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        let domain = crate::theme_headers::text_domain(self.stylesheet_dir());
        for dir in &dirs {
            let Some(source) = rustpress_core::files::read(&dir.join("functions.php")) else { continue };
            let domain = crate::theme_headers::text_domain(dir);
            let mut rest: &str = &source;
            while let Some(at) = rest.find("register_block_style(") {
                rest = &rest[at + "register_block_style(".len()..];
                let Some((block, after)) = php_literal(rest) else { continue };
                let end = after.find("register_block_style(").unwrap_or(after.len());
                let body = &after[..end];
                let name = self.php_array_text(body, "name", &domain);
                let label = self.php_array_text(body, "label", &domain);
                if name.is_empty() {
                    continue;
                }
                let is_default = body.find("'is_default'").is_some_and(|i| body[i..].split(',').next().is_some_and(|v| v.contains("true")));
                flat.push((block, name, label, is_default));
            }
        }
        if partials {
            for (block, name, label) in self.block_style_variations(&domain) {
                flat.push((block, name, label, false));
            }
        }
        let mut grouped: Vec<(String, Vec<BlockStyle>)> = Vec::new();
        for style in flat {
            match grouped.iter_mut().find(|(block, _)| *block == style.0) {
                Some((_, list)) => match list.iter_mut().find(|s| s.1 == style.1) {
                    Some(existing) => *existing = style,
                    None => list.push(style),
                },
                None => grouped.push((style.0.clone(), vec![style])),
            }
        }
        grouped
    }
}

impl Theme {
    /// The `inline_style` a `register_block_style()` call of the theme gives the style `name` of `block`.
    pub fn registered_block_style_inline(&self, block: &str, name: &str) -> Option<String> {
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        dirs.iter()
            .filter_map(|dir| rustpress_core::files::read(&dir.join("functions.php")))
            .flat_map(|php| crate::functions_php::block_style_inline_named(&php))
            .find(|(b, n, _)| b == block && n == name)
            .map(|(_, _, css)| css)
    }

    /// `get_theme_support( 'post-formats' )[0]`: the formats the theme supports, none without the support.
    pub fn post_formats(&self) -> Vec<String> {
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        dirs.iter().filter_map(|dir| rustpress_core::files::read(&dir.join("functions.php"))).find_map(|php| crate::functions_php::theme_support_list(&php, "post-formats")).unwrap_or_default()
    }

    /// The block bindings sources the theme and its parent register, the child's first.
    pub fn block_bindings_sources(&self) -> Vec<crate::functions_php::BindingSource> {
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        dirs.iter().filter_map(|dir| rustpress_core::files::read(&dir.join("functions.php"))).flat_map(|php| crate::functions_php::block_bindings_sources(&php)).collect()
    }

    /// `current_theme_supports( $feature )` for what the `functions.php` of the theme (or its parent) adds.
    pub fn adds_theme_support(&self, feature: &str) -> bool {
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        dirs.iter().filter_map(|dir| rustpress_core::files::read(&dir.join("functions.php"))).any(|php| crate::functions_php::adds_theme_support(&php, feature))
    }
}

/// A block pattern category as `WP_Block_Pattern_Categories_Registry` holds it.
#[derive(Debug, Clone)]
pub struct PatternCategory {
    pub name: String,
    pub label: String,
    pub description: String,
}

/// The next argument of a PHP call: the literal after a comma.
fn next_literal(rest: &str) -> Option<(String, &str)> {
    php_literal(rest.trim_start().strip_prefix(',').unwrap_or(rest))
}

/// The single-quoted PHP string at the start of `text` and what follows it.
fn php_literal(text: &str) -> Option<(String, &str)> {
    let rest = text.trim_start().strip_prefix('\'')?;
    let mut out = String::new();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some((_, escaped @ ('\'' | '\\'))) => out.push(escaped),
                Some((_, other)) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return None,
            },
            '\'' => return Some((out, &rest[i + 1..])),
            _ => out.push(c),
        }
    }
    None
}

impl Theme {
    /// The `inline_style` of every `register_block_style()` of the theme, in the order of registration.
    pub fn registered_block_style_inline_styles(&self) -> Vec<String> {
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        let mut out = Vec::new();
        for dir in &dirs {
            let Some(source) = rustpress_core::files::read(&dir.join("functions.php")) else { continue };
            let domain = crate::theme_headers::text_domain(dir);
            let mut rest: &str = &source;
            while let Some(at) = rest.find("register_block_style(") {
                rest = &rest[at + "register_block_style(".len()..];
                let Some((_, after)) = php_literal(rest) else { continue };
                let end = after.find("register_block_style(").unwrap_or(after.len());
                let inline = self.php_array_text(&after[..end], "inline_style", &domain);
                if !inline.is_empty() {
                    out.push(inline);
                }
            }
        }
        out
    }

    /// The text of `'key' => __( 'text', 'domain' )`, `_x( 'text', 'context', 'domain' )` or `'text'` after `from`.
    fn php_array_text(&self, from: &str, key: &str, default_domain: &str) -> String {
        let Some(at) = from.find(&format!("'{key}'")) else { return String::new() };
        let value = from[at + key.len() + 2..].trim_start().strip_prefix("=>").unwrap_or_default().trim_start();
        let (function, args) = if let Some(args) = value.strip_prefix("_x(") {
            ("_x", args)
        } else if let Some(args) = value.strip_prefix("__(") {
            ("__", args)
        } else {
            ("", value)
        };
        let Some((text, rest)) = php_literal(args) else { return String::new() };
        match function {
            "_x" => {
                let Some((context, rest)) = next_literal(rest) else { return text };
                let domain = next_literal(rest).map_or_else(|| default_domain.to_owned(), |(d, _)| d);
                self.translate_with_context(&text, &context, &domain)
            }
            "__" => {
                let domain = next_literal(rest).map_or_else(|| default_domain.to_owned(), |(d, _)| d);
                self.host_ref().translate(&text, None, &domain)
            }
            _ => text,
        }
    }

    fn scan_pattern_categories(&self, source: &str, domain: &str, out: &mut Vec<PatternCategory>) {
        let mut rest = source;
        while let Some(at) = rest.find("register_block_pattern_category(") {
            rest = &rest[at + "register_block_pattern_category(".len()..];
            let Some((name, after)) = php_literal(rest) else { continue };
            let end = after.find("register_block_pattern_category(").unwrap_or(after.len());
            let body = &after[..end];
            let (label, description) = (self.php_array_text(body, "label", domain), self.php_array_text(body, "description", domain));
            if !out.iter().any(|c| c.name == name) {
                out.push(PatternCategory { name, label, description });
            }
        }
    }

    /// `WP_Block_Pattern_Categories_Registry::get_all_registered()`: WordPress' categories, then those the theme (and
    /// its parent) registers in `functions.php`.
    pub fn pattern_categories(&self) -> Vec<PatternCategory> {
        let mut out = Vec::new();
        if let Some(source) = rustpress_core::files::read(&self.wp_root.join("wp-includes").join("block-patterns.php")) {
            self.scan_pattern_categories(&source, "default", &mut out);
        }
        let mut dirs = vec![self.stylesheet_dir().to_path_buf()];
        if self.template_dir() != self.stylesheet_dir() {
            dirs.push(self.template_dir().to_path_buf());
        }
        for dir in dirs {
            if let Some(source) = rustpress_core::files::read(&dir.join("functions.php")) {
                self.scan_pattern_categories(&source, &crate::theme_headers::text_domain(&dir), &mut out);
            }
        }
        out
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
