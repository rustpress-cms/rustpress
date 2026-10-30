//! Printing what plugins enqueue on `wp_enqueue_scripts` (`WP_Styles`/`WP_Scripts` for the handles of
//! [`rustpress_core::native::Assets`]): stylesheets as links, scripts after the core scripts they depend on.

use rustpress_core::formatting::{esc_attr, esc_url};
use rustpress_core::native::Asset;

/// The `src` with its `ver` (`add_query_arg( 'ver', $ver, $src )`).
fn src(asset: &Asset) -> String {
    match &asset.ver {
        Some(ver) => rustpress_core::urls::add_query_arg(&asset.src, "ver", ver),
        None => asset.src.clone(),
    }
}

/// `WP_Styles::do_item()` for each stylesheet.
pub fn style_links(styles: &[Asset]) -> String {
    styles.iter().map(|style| format!("<link rel='stylesheet' id='{}-css' href='{}' media='{}' />\n", esc_attr(&style.handle), esc_url(&src(style)), esc_attr(&style.media))).collect()
}

/// `WP_Scripts` for one page: every handle is printed once, its dependencies before it.
pub struct ScriptPrinter {
    includes_url: String,
    jquery: String,
    migrate: String,
    printed: Vec<String>,
}

impl ScriptPrinter {
    /// `jquery`/`migrate`: the versions of `jquery-core` and `jquery-migrate`.
    pub fn new(includes_url: &str, jquery: &str, migrate: &str) -> Self {
        Self { includes_url: includes_url.to_owned(), jquery: jquery.to_owned(), migrate: migrate.to_owned(), printed: Vec::new() }
    }

    /// The tags of a core script (with its dependencies), unless they were printed already.
    pub fn core(&mut self, handle: &str) -> String {
        if self.printed.iter().any(|h| h == handle) {
            return String::new();
        }
        self.printed.push(handle.to_owned());
        let includes = &self.includes_url;
        match handle {
            "jquery" => format!(
                "<script id=\"jquery-core-js\" src=\"{includes}/js/jquery/jquery.min.js?ver={}\"></script>\n<script id=\"jquery-migrate-js\" src=\"{includes}/js/jquery/jquery-migrate.min.js?ver={}\"></script>\n",
                self.jquery, self.migrate
            ),
            _ => String::new(),
        }
    }

    /// The plugin scripts of one place (head or footer) in queue order, each after its dependencies.
    pub fn scripts(&mut self, scripts: &[Asset], in_footer: bool) -> String {
        let mut out = String::new();
        for script in scripts.iter().filter(|s| s.in_footer == in_footer) {
            for dep in &script.deps {
                out.push_str(&self.core(dep));
            }
            if self.printed.contains(&script.handle) {
                continue;
            }
            self.printed.push(script.handle.clone());
            let handle = esc_attr(&script.handle);
            if let Some(extra) = &script.extra {
                out.push_str(&format!("<script id=\"{handle}-js-extra\">\n{extra}\n//# sourceURL={handle}-js-extra\n</script>\n"));
            }
            if !script.before.is_empty() {
                out.push_str(&format!("<script id=\"{handle}-js-before\">\n{}\n//# sourceURL={handle}-js-before\n</script>\n", script.before.join("\n")));
            }
            out.push_str(&format!("<script id=\"{handle}-js\" src=\"{}\"></script>\n", esc_url(&src(script))));
            if !script.after.is_empty() {
                out.push_str(&format!("<script id=\"{handle}-js-after\">\n{}\n//# sourceURL={handle}-js-after\n</script>\n", script.after.join("\n")));
            }
        }
        out
    }
}
