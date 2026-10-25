//! The active block theme: files on disk plus customizations from the database.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rustpress_blocks::env::TemplatePart;
use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::Site;
use rustpress_db::PostQuery;
use serde_json::{Map, Value};

use crate::global_styles::GlobalStyles;
use crate::phpt::{self, PhpHost};
use crate::theme_json::{ThemeJson, read_file};
use crate::{Error, Result};

/// URLs the pattern evaluator needs.
pub(crate) struct Host {
    template_uri: String,
    stylesheet_uri: String,
    home: String,
    siteurl: String,
    i18n: std::sync::Arc<rustpress_core::i18n::I18n>,
}

impl PhpHost for Host {
    fn template_directory_uri(&self) -> String {
        self.template_uri.clone()
    }
    fn stylesheet_directory_uri(&self) -> String {
        self.stylesheet_uri.clone()
    }
    fn home_url(&self, path: &str) -> String {
        join_url(&self.home, path)
    }
    fn site_url(&self, path: &str) -> String {
        join_url(&self.siteurl, path)
    }
    fn translate(&self, text: &str, context: Option<&str>, domain: &str) -> String {
        match context {
            Some(context) => self.i18n.pgettext(domain, context, text),
            None => self.i18n.gettext(domain, text),
        }
    }
}

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if path.is_empty() { base.to_owned() } else { format!("{base}/{path}") }
}

pub struct Theme {
    pub wp_root: PathBuf,
    pub stylesheet: String,
    pub template: String,
    stylesheet_dir: PathBuf,
    template_dir: PathBuf,
    host: Host,
    pub json: ThemeJson,
    registry: Registry,
    settings: Value,
    styles: Value,
    part_areas: HashMap<String, String>,
    patterns: HashMap<String, PathBuf>,
    db_templates: HashMap<String, String>,
    db_parts: HashMap<String, String>,
    block_style_inline: Vec<(String, String)>,
    block_stylesheets: Vec<crate::functions_php::BlockStylesheet>,
    theme_style_handle: Option<String>,
    /// The theme gave its stylesheet a `path` (`wp_style_add_data()`), so WordPress may inline it.
    theme_style_path: bool,
    /// `add_theme_support( 'wp-block-styles' )`: core blocks load their `theme.css`, too.
    wp_block_styles: bool,
    pattern_cache: Mutex<HashMap<String, Option<String>>>,
}

impl Theme {
    /// Loads the theme named by the `template`/`stylesheet` options.
    pub async fn load(site: &Site, wp_root: impl Into<PathBuf>) -> Result<Self> {
        let wp_root = wp_root.into();
        let stylesheet = site.settings.stylesheet.clone();
        let template = site.settings.template.clone();
        let themes = wp_root.join("wp-content").join("themes");
        let stylesheet_dir = themes.join(&stylesheet);
        let template_dir = themes.join(&template);
        if !template_dir.is_dir() {
            return Err(Error::ThemeMissing(template));
        }

        let user = load_user_styles(site, &stylesheet).await?;
        let core = read_file(&wp_root.join("wp-includes").join("theme.json")).unwrap_or_default();
        let parent = read_file(&template_dir.join("theme.json"));
        let child = if stylesheet != template { read_file(&stylesheet_dir.join("theme.json")) } else { None };
        // Block style variations: every `styles/**.json` that names `blockTypes` (a child theme's file wins by name).
        let partials = read_partials(&[stylesheet_dir.join("styles"), template_dir.join("styles")]);
        let blocks_data = Registry::new(wp_root.join("wp-includes").join("blocks")).block_data();
        let json = ThemeJson::with_layers(core, blocks_data, parent, child, user, &partials);

        let mut part_areas = HashMap::new();
        for part in json.template_parts() {
            if let (Some(name), Some(area)) = (part.get("name").and_then(Value::as_str), part.get("area").and_then(Value::as_str)) {
                part_areas.insert(name.to_owned(), area.to_owned());
            }
        }

        let mut patterns = HashMap::new();
        for dir in [&template_dir, &stylesheet_dir] {
            scan_patterns(&dir.join("patterns"), &mut patterns);
            scan_registered_patterns(dir, &mut patterns);
        }

        let (db_templates, db_parts) = load_db_templates(site, &stylesheet).await?;

        // `register_block_style()` calls with inline CSS; a child theme's `functions.php` runs first.
        let mut block_style_inline = Vec::new();
        let mut block_stylesheets = Vec::new();
        let mut theme_style_handle = None;
        let mut theme_style_path = false;
        let mut wp_block_styles = false;
        let mut dirs = vec![&stylesheet_dir];
        if template_dir != stylesheet_dir {
            dirs.push(&template_dir);
        }
        for dir in dirs {
            if let Ok(php) = std::fs::read_to_string(dir.join("functions.php")) {
                block_style_inline.extend(crate::functions_php::block_style_inline_styles(&php));
                block_stylesheets.extend(crate::functions_php::block_stylesheets(&php));
                wp_block_styles |= crate::functions_php::adds_theme_support(&php, "wp-block-styles");
                theme_style_handle = theme_style_handle.or_else(|| crate::functions_php::theme_stylesheet_handle(&php));
                if let Some(handle) = &theme_style_handle {
                    theme_style_path |= crate::functions_php::theme_stylesheet_has_path(&php, handle);
                }
            }
        }

        // Translations: WordPress itself and the text domains of the active theme(s).
        let mut domains = vec![(template.clone(), template_dir.clone())];
        if stylesheet != template {
            domains.push((stylesheet.clone(), stylesheet_dir.clone()));
        }
        site.init_i18n(&wp_root, &domains);
        let blocks_dir = wp_root.join("wp-includes").join("blocks");
        let content_url = join_url(&site.settings.siteurl, "wp-content");
        let host = Host {
            template_uri: format!("{content_url}/themes/{template}"),
            stylesheet_uri: format!("{content_url}/themes/{stylesheet}"),
            home: site.settings.home.clone(),
            siteurl: site.settings.siteurl.clone(),
            i18n: site.i18n(),
        };
        Ok(Self {
            wp_root,
            settings: json.settings(),
            styles: json.styles(),
            registry: Registry::new(blocks_dir),
            json,
            stylesheet,
            template,
            stylesheet_dir,
            template_dir,
            host,
            part_areas,
            patterns,
            db_templates,
            db_parts,
            block_style_inline,
            block_stylesheets,
            theme_style_handle,
            theme_style_path,
            wp_block_styles,
            pattern_cache: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn host_ref(&self) -> &Host {
        &self.host
    }

    /// The JSON files of `styles/` of the theme and its parent, sorted by path (`ksort( $variation_files )`).
    fn variation_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        let child = self.stylesheet_dir.join("styles");
        let parent = self.template_dir.join("styles");
        json_files(&child, &mut files);
        if parent != child {
            let mut inherited = Vec::new();
            json_files(&parent, &mut inherited);
            let own: Vec<std::ffi::OsString> = files.iter().filter_map(|c| c.file_name().map(|n| n.to_owned())).collect();
            files.extend(inherited.into_iter().filter(|p| !p.file_name().is_some_and(|n| own.iter().any(|o| o == n))));
        }
        files.sort();
        files
    }

    /// The style variations of single blocks (`styles/**/*.json` with `blockTypes`): `(block type, name, label)`.
    pub fn block_style_variations(&self, text_domain: &str) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for path in self.variation_files() {
            let Some(raw) = read_file(&path) else { continue };
            let Some(map) = raw.as_object() else { continue };
            let Some(block_types) = map.get("blockTypes").and_then(Value::as_array) else { continue };
            let title = map.get("title").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
            let title = self.host_ref().translate(&title, Some("Style variation name"), text_domain);
            let name = map.get("slug").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| kebab_case(&title));
            for block in block_types.iter().filter_map(Value::as_str) {
                out.push((block.to_owned(), name.clone(), title.clone()));
            }
        }
        out
    }

    /// `WP_Theme_JSON_Resolver::get_style_variations()`: the whole-theme style variations of `styles/` (any depth),
    /// by path, a child theme's file replacing the parent's of the same name.
    pub fn style_variations(&self) -> Vec<Value> {
        let files = self.variation_files();
        let partials = read_partials(&[self.stylesheet_dir.join("styles"), self.template_dir.join("styles")]);
        let mut out = Vec::new();
        for path in files {
            let Some(raw) = read_file(&path) else { continue };
            let Some(map) = raw.as_object() else { continue };
            // Variations of single blocks have a scope of their own.
            if map.contains_key("blockTypes") {
                continue;
            }
            let mut variation = serde_json::Map::new();
            variation.insert("version".into(), map.get("version").cloned().unwrap_or(Value::from(3)));
            let title = map.get("title").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
            variation.insert("title".into(), Value::String(title));
            if let Some(slug) = map.get("slug") {
                variation.insert("slug".into(), slug.clone());
            }
            if let Some(settings) = map.get("settings") {
                let mut settings = settings.clone();
                crate::theme_json::wrap_preset_origins(&mut settings, "theme");
                variation.insert("settings".into(), settings);
            }
            if let Some(styles) = map.get("styles") {
                let mut styles = styles.clone();
                unwrap_shared_variations(&mut styles, &partials);
                crate::theme_json::resolve_custom_vars(&mut styles);
                variation.insert("styles".into(), styles);
            }
            out.push(Value::Object(variation));
        }
        out
    }

    /// Markup of a template, preferring the user's database version over files.
    pub fn template_markup(&self, slug: &str) -> Option<String> {
        if let Some(markup) = self.db_templates.get(slug) {
            return Some(markup.clone());
        }
        self.read_theme_file(&format!("templates/{slug}.html"))
    }

    /// The first existing template of a candidate list.
    pub fn pick_template(&self, candidates: &[String]) -> Option<(String, String)> {
        candidates.iter().find_map(|slug| self.template_markup(slug).map(|markup| (slug.clone(), markup)))
    }

    /// The child theme's file wins over the parent's.
    fn read_theme_file(&self, relative: &str) -> Option<String> {
        if relative.contains("..") {
            return None;
        }
        std::fs::read_to_string(self.stylesheet_dir.join(relative))
            .or_else(|_| std::fs::read_to_string(self.template_dir.join(relative)))
            .ok()
    }

    /// Whether the theme asked for responsive embeds. Block themes with a
    /// `theme.json` get `wp-embed-responsive` on `<body>`.
    pub fn supports_responsive_embeds(&self) -> bool {
        true
    }

    /// `(block name, CSS)` of the `inline_style` of the theme's registered block styles.
    pub fn block_style_inline(&self) -> &[(String, String)] {
        &self.block_style_inline
    }

    /// Stylesheets the theme enqueues for single blocks (`wp_enqueue_block_style()`).
    pub fn block_stylesheets(&self) -> &[crate::functions_php::BlockStylesheet] {
        &self.block_stylesheets
    }

    /// The handle under which the theme enqueues its `style.css`, if its `functions.php` does.
    pub fn theme_style_handle(&self) -> Option<&str> {
        self.theme_style_handle.as_deref()
    }

    /// `current_theme_supports( 'wp-block-styles' )`.
    pub fn wp_block_styles(&self) -> bool {
        self.wp_block_styles
    }

    /// Whether the theme's `style.css` can be inlined (it has a `path`).
    pub fn theme_style_path(&self) -> bool {
        self.theme_style_path
    }

    /// A file of the theme (the child theme's wins), if it exists.
    pub fn theme_file(&self, relative: &str) -> Option<PathBuf> {
        if relative.contains("..") {
            return None;
        }
        [&self.stylesheet_dir, &self.template_dir].into_iter().map(|d| d.join(relative)).find(|p| p.is_file())
    }

    /// `Version:` of the theme's `style.css` header.
    pub fn version(&self) -> String {
        let text = std::fs::read_to_string(self.template_dir.join("style.css")).unwrap_or_default();
        text.lines().find_map(|l| l.trim().strip_prefix("Version:").map(|v| v.trim().to_owned())).unwrap_or_default()
    }

    pub fn stylesheet_dir(&self) -> &Path {
        &self.stylesheet_dir
    }

    pub fn template_dir(&self) -> &Path {
        &self.template_dir
    }
}

impl ThemeAccess for Theme {
    fn image_size(&self, name: &str) -> Option<(u64, u64)> {
        crate::classic::image_size(&self.template, name)
    }

    fn filter_calculated_sizes(&self, site: &Site, query: &rustpress_core::QueryResult, sizes: Option<String>, width: u64) -> Option<String> {
        if crate::classic::is_ported(self) { crate::classic::seventeen::calculated_sizes(site, query, &self.stylesheet, sizes, width) } else { sizes }
    }

    fn excerpt_more(&self) -> Option<String> {
        crate::classic::is_ported(self).then(|| crate::classic::seventeen::excerpt_more_template(&self.host.i18n))
    }

    fn filter_attachment_sizes(&self, site: &Site, query: &rustpress_core::QueryResult, _current: Option<&str>) -> Option<String> {
        crate::classic::is_ported(self).then(|| crate::classic::seventeen::attachment_sizes_attr(site, query))
    }

    fn global_settings(&self) -> &Value {
        &self.settings
    }

    fn global_styles(&self) -> &Value {
        &self.styles
    }

    fn template_part(&self, slug: &str) -> Option<TemplatePart> {
        if slug.contains(['/', '\\']) || slug.contains("..") {
            return None;
        }
        let content = match self.db_parts.get(slug) {
            Some(markup) => markup.clone(),
            None => self.read_theme_file(&format!("parts/{slug}.html"))?,
        };
        let area = self.part_areas.get(slug).cloned().unwrap_or_else(|| "uncategorized".to_owned());
        Some(TemplatePart { slug: slug.to_owned(), content, area })
    }

    fn pattern(&self, slug: &str) -> Option<String> {
        if let Some(cached) = self.pattern_cache.lock().ok()?.get(slug) {
            return cached.clone();
        }
        let result = self.patterns.get(slug).and_then(|path| {
            let source = std::fs::read_to_string(path).ok()?;
            let rendered = if source.contains("return array(") && source.contains("'content'") { phpt::pattern_array_content(&source, &self.host) } else { phpt::render(&source, &self.host) };
            match rendered {
                Ok(markup) => Some(markup),
                Err(e) => {
                    tracing::warn!(pattern = slug, error = %e, "cannot evaluate pattern");
                    None
                }
            }
        });
        self.pattern_cache.lock().ok()?.insert(slug.to_owned(), result.clone());
        result
    }

    fn style_variation(&self, block_name: &str, variation: &str) -> Option<Value> {
        crate::theme_json::find_variation(&self.styles, block_name, variation).cloned()
    }

    fn variation_css(&self, block_name: &str, variation: &Value, scope: &str) -> String {
        let mut resolved = variation.clone();
        crate::theme_json::resolve_custom_vars(&mut resolved);
        GlobalStyles::new(&self.json, &self.registry).variation_stylesheet(block_name, &resolved, scope)
    }

    fn template_directory_uri(&self) -> String {
        self.host.template_uri.clone()
    }

    fn stylesheet_directory_uri(&self) -> String {
        self.host.stylesheet_uri.clone()
    }
}

/// `WP_Theme_JSON::unwrap_shared_block_style_variations()`: `styles.variations.<name>` is copied to every block
/// that the style partial of that name is registered for.
fn unwrap_shared_variations(styles: &mut Value, partials: &[Value]) {
    let Some(shared) = styles.as_object_mut().and_then(|m| m.remove("variations")).and_then(|v| v.as_object().cloned()) else { return };
    // Block names in alphabetical order, the variations of each in the order of the partials.
    let mut targets: Vec<(String, String)> = Vec::new();
    for partial in partials {
        let Some(name) = partial.get("slug").and_then(Value::as_str) else { continue };
        for block in partial.get("blockTypes").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
            targets.push((block.to_owned(), name.to_owned()));
        }
    }
    targets.sort_by(|a, b| a.0.cmp(&b.0));
    for (block, name) in targets {
        let Some(top) = shared.get(&name) else { continue };
        let mut merged = top.clone();
        if let Some(existing) = styles.get("blocks").and_then(|b| b.get(&block)).and_then(|b| b.get("variations")).and_then(|v| v.get(&name)) {
            crate::theme_json::deep_merge(&mut merged, existing);
        }
        let mut patch = Value::Object(Map::new());
        patch["blocks"][&block]["variations"][&name] = merged;
        // The variation replaces what was there instead of being merged into it.
        if let Some(slot) = styles.get_mut("blocks").and_then(|b| b.get_mut(&block)).and_then(|b| b.get_mut("variations")).and_then(Value::as_object_mut) {
            slot.remove(&name);
        }
        crate::theme_json::deep_merge(styles, &patch);
    }
}

/// All `.json` files below a directory, recursively.
fn json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            json_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

/// Block style variation partials (`WP_Theme_JSON_Resolver::get_style_variations( 'block' )`): the files of
/// `styles/` (any depth) that declare `blockTypes`, sorted by path; the first directory (the child theme) wins
/// for files with the same name.
fn read_partials(style_dirs: &[PathBuf]) -> Vec<Value> {
    let mut files: Vec<PathBuf> = Vec::new();
    for (i, dir) in style_dirs.iter().enumerate() {
        if i > 0 && style_dirs[0] == *dir {
            continue;
        }
        let mut found = Vec::new();
        json_files(dir, &mut found);
        for path in found {
            if i > 0 && files.iter().any(|f| f.file_name() == path.file_name()) {
                continue;
            }
            files.push(path);
        }
    }
    files.sort();
    files.iter().filter_map(|p| read_file(p)).filter(|v| v.get("blockTypes").is_some()).collect()
}

/// Patterns a classic block theme registers from `inc/block-patterns.php`:
/// `$block_patterns = array( 'name', ... )` and `register_block_pattern( 'theme/' . $block_pattern, require ... )`.
fn scan_registered_patterns(theme_dir: &Path, patterns: &mut HashMap<String, PathBuf>) {
    let Ok(php) = std::fs::read_to_string(theme_dir.join("inc").join("block-patterns.php")) else { return };
    let Some(prefix) = php.find("register_block_pattern(").map(|i| &php[i..]).and_then(|rest| {
        let rest = rest["register_block_pattern(".len()..].trim_start();
        let rest = rest.strip_prefix('\'')?;
        let end = rest.find('\'')?;
        rest[end + 1..].trim_start().starts_with('.').then(|| rest[..end].to_owned())
    }) else { return };
    let Some(list_start) = php.find("$block_patterns = array(") else { return };
    let list = &php[list_start + "$block_patterns = array(".len()..];
    let Some(list_end) = list.find(");") else { return };
    for name in list[..list_end].split(',').filter_map(|item| item.trim().strip_prefix('\'').and_then(|i| i.strip_suffix('\''))) {
        let path = theme_dir.join("inc").join("patterns").join(format!("{name}.php"));
        if path.is_file() {
            patterns.entry(format!("{prefix}{name}")).or_insert(path);
        }
    }
}

/// Records `Slug:` from the docblock of every pattern file.
fn scan_patterns(dir: &Path, patterns: &mut HashMap<String, PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("php") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let header: String = text.chars().take(4096).collect();
        if let Some(slug) = header.lines().find_map(|l| l.trim().trim_start_matches('*').trim().strip_prefix("Slug:")) {
            patterns.insert(slug.trim().to_owned(), path);
        }
    }
}

/// The user's `wp_global_styles` post for this theme, as parsed JSON.
async fn load_user_styles(site: &Site, stylesheet: &str) -> Result<Option<Value>> {
    let posts = site
        .db
        .posts(&PostQuery { post_types: vec!["wp_global_styles".into()], statuses: vec!["publish".into()], ..PostQuery::default() })
        .await?;
    for post in posts {
        let belongs = site.db.terms_for_post(post.id, Some("wp_theme")).await?.iter().any(|t| t.name == stylesheet);
        if belongs
            && let Ok(value) = serde_json::from_str::<Value>(&post.post_content) {
                // Stored as {"version":.., "settings":.., "styles":..}.
                return Ok(Some(value));
            }
    }
    Ok(None)
}

/// User-edited templates and template parts (`wp_template`, `wp_template_part`).
async fn load_db_templates(site: &Site, stylesheet: &str) -> Result<(HashMap<String, String>, HashMap<String, String>)> {
    let mut templates = HashMap::new();
    let mut parts = HashMap::new();
    let posts = site
        .db
        .posts(&PostQuery { post_types: vec!["wp_template".into(), "wp_template_part".into()], statuses: vec!["publish".into()], ..PostQuery::default() })
        .await?;
    for post in posts {
        let belongs = site.db.terms_for_post(post.id, Some("wp_theme")).await?.iter().any(|t| t.name == stylesheet);
        if !belongs {
            continue;
        }
        if post.post_type == "wp_template" {
            templates.insert(post.post_name.clone(), post.post_content.clone());
        } else {
            parts.insert(post.post_name.clone(), post.post_content.clone());
        }
    }
    Ok((templates, parts))
}

/// `_wp_to_kebab_case()` for a title.
fn kebab_case(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}
