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

/// The `version` of a package in `wp-includes/assets/script-loader-packages.php` (`hooks`, `i18n`).
pub fn package_version(wp_root: &std::path::Path, package: &str) -> Option<String> {
    let text = std::fs::read_to_string(wp_root.join("wp-includes").join("assets").join("script-loader-packages.php")).ok()?;
    let at = text.find(&format!("'{package}.js' => array("))?;
    let rest = &text[at..];
    let version = &rest[rest.find("'version' => '")? + "'version' => '".len()..];
    Some(version[..version.find('\'')?].to_owned())
}

/// `WP_Scripts` for one page: every handle is printed once, its dependencies before it.
pub struct ScriptPrinter {
    includes_url: String,
    jquery: String,
    migrate: String,
    /// Versions of `wp-hooks` and `wp-i18n`.
    hooks: String,
    i18n: String,
    /// `_x( 'ltr', 'text direction' )` for the locale data of `wp-i18n`.
    text_direction: String,
    /// For `load_script_textdomain()`: `wp-content/languages`, the URL of the plugins and the locale.
    languages: Option<(std::path::PathBuf, String, String)>,
    printed: Vec<String>,
}

impl ScriptPrinter {
    /// `jquery`/`migrate`: the versions of `jquery-core` and `jquery-migrate`.
    pub fn new(includes_url: &str, jquery: &str, migrate: &str) -> Self {
        Self { includes_url: includes_url.to_owned(), jquery: jquery.to_owned(), migrate: migrate.to_owned(), hooks: String::new(), i18n: String::new(), text_direction: "ltr".to_owned(), languages: None, printed: Vec::new() }
    }

    /// Knows the packages `wp-hooks` and `wp-i18n` of the WordPress files in `wp_root`, and the translations of
    /// the plugin scripts for the site's language.
    pub fn with_packages(mut self, wp_root: &std::path::Path, site: &rustpress_core::Site) -> Self {
        self.hooks = package_version(wp_root, "hooks").unwrap_or_default();
        self.i18n = package_version(wp_root, "i18n").unwrap_or_default();
        self.text_direction = if site.is_rtl() { "rtl" } else { "ltr" }.to_owned();
        let locale = site.locale();
        if locale != "en_US" {
            self.languages = Some((wp_root.join("wp-content").join("languages"), site.site_url("wp-content/plugins"), locale));
        }
        self
    }

    /// `load_script_textdomain()` for a plugin script: the JSON of `<domain>-<locale>-<md5>.json`.
    fn script_translations(&self, script: &Asset, domain: &str) -> Option<String> {
        let (dir, plugins_url, locale) = self.languages.as_ref()?;
        let relative = script.src.strip_prefix(plugins_url.as_str())?.trim_matches('/');
        // The plugin's own directory is not part of the path.
        let relative = relative.split_once('/')?.1;
        let relative = relative.strip_suffix(".min.js").map_or_else(|| relative.to_owned(), |base| format!("{base}.js"));
        let md5 = format!("{:x}", md5::compute(relative.as_bytes()));
        std::fs::read_to_string(dir.join("plugins").join(format!("{domain}-{locale}-{md5}.json"))).ok()
    }

    /// The tags of a core script (with its dependencies), unless they were printed already.
    pub fn core(&mut self, handle: &str) -> String {
        if self.printed.iter().any(|h| h == handle) {
            return String::new();
        }
        self.printed.push(handle.to_owned());
        let includes = self.includes_url.clone();
        match handle {
            "jquery" => format!(
                "<script id=\"jquery-core-js\" src=\"{includes}/js/jquery/jquery.min.js?ver={}\"></script>\n<script id=\"jquery-migrate-js\" src=\"{includes}/js/jquery/jquery-migrate.min.js?ver={}\"></script>\n",
                self.jquery, self.migrate
            ),
            "wp-hooks" => format!("<script id=\"wp-hooks-js\" src=\"{includes}/js/dist/hooks.min.js?ver={}\"></script>\n", self.hooks),
            "wp-i18n" => {
                let hooks = self.core("wp-hooks");
                format!(
                    "{hooks}<script id=\"wp-i18n-js\" src=\"{includes}/js/dist/i18n.min.js?ver={}\"></script>\n<script id=\"wp-i18n-js-after\">\nwp.i18n.setLocaleData( {{ 'text direction\\u0004ltr': [ '{}' ] }} );\n//# sourceURL=wp-i18n-js-after\n</script>\n",
                    self.i18n,
                    self.text_direction
                )
            }
            _ => String::new(),
        }
    }

    /// The plugin scripts of one place (head or footer) in queue order, each after its dependencies.
    pub fn scripts(&mut self, scripts: &[Asset], in_footer: bool) -> String {
        let mut out = String::new();
        for script in scripts.iter().filter(|s| s.in_footer == in_footer && !s.registered_only) {
            out.push_str(&self.script(scripts, script));
        }
        out
    }

    /// One plugin script after its dependencies, which are core scripts or other plugin scripts.
    fn script(&mut self, scripts: &[Asset], script: &Asset) -> String {
        let mut out = String::new();
        for dep in &script.deps {
            match scripts.iter().find(|s| s.handle == *dep) {
                Some(plugin_dep) => out.push_str(&self.script(scripts, plugin_dep)),
                None => out.push_str(&self.core(dep)),
            }
        }
        if self.printed.contains(&script.handle) {
            return out;
        }
        self.printed.push(script.handle.clone());
        let handle = esc_attr(&script.handle);
        if let Some(extra) = &script.extra {
            out.push_str(&format!("<script id=\"{handle}-js-extra\">\n{extra}\n//# sourceURL={handle}-js-extra\n</script>\n"));
        }
        if let Some(json) = script.translations.as_deref().and_then(|domain| self.script_translations(script, domain).map(|json| (domain, json))) {
            let (domain, json) = json;
            out.push_str(&format!(
                "<script id=\"{handle}-js-translations\">\n( function( domain, translations ) {{\n\tvar localeData = translations.locale_data[ domain ] || translations.locale_data.messages;\n\tlocaleData[\"\"].domain = domain;\n\twp.i18n.setLocaleData( localeData, domain );\n}} )( \"{domain}\", {json} );\n//# sourceURL={handle}-js-translations\n</script>\n"
            ));
        }
        if !script.before.is_empty() {
            out.push_str(&format!("<script id=\"{handle}-js-before\">\n{}\n//# sourceURL={handle}-js-before\n</script>\n", script.before.join("\n")));
        }
        out.push_str(&format!("<script id=\"{handle}-js\" src=\"{}\"></script>\n", esc_url(&src(script))));
        if !script.after.is_empty() {
            out.push_str(&format!("<script id=\"{handle}-js-after\">\n{}\n//# sourceURL={handle}-js-after\n</script>\n", script.after.join("\n")));
        }
        out
    }
}
