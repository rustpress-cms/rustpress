//! The active block theme: files on disk plus customizations from the database.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rustpress_blocks::env::TemplatePart;
use rustpress_blocks::{Registry, ThemeAccess};
use rustpress_core::Site;
use rustpress_db::PostQuery;
use serde_json::Value;

use crate::global_styles::GlobalStyles;
use crate::phpt::{self, PhpHost};
use crate::theme_json::{ThemeJson, read_file};
use crate::{Error, Result};

/// URLs the pattern evaluator needs.
struct Host {
    template_uri: String,
    stylesheet_uri: String,
    home: String,
    siteurl: String,
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
        let mut partials = Vec::new();
        for dir in [&template_dir, &stylesheet_dir] {
            partials.extend(read_partials(&dir.join("styles").join("blocks")));
        }
        let json = ThemeJson::with_partials(core, parent, child, user, &partials);

        let mut part_areas = HashMap::new();
        for part in json.template_parts() {
            if let (Some(name), Some(area)) = (part.get("name").and_then(Value::as_str), part.get("area").and_then(Value::as_str)) {
                part_areas.insert(name.to_owned(), area.to_owned());
            }
        }

        let mut patterns = HashMap::new();
        for dir in [&template_dir, &stylesheet_dir] {
            scan_patterns(&dir.join("patterns"), &mut patterns);
        }

        let (db_templates, db_parts) = load_db_templates(site, &stylesheet).await?;

        let blocks_dir = wp_root.join("wp-includes").join("blocks");
        let content_url = join_url(&site.settings.siteurl, "wp-content");
        let host = Host {
            template_uri: format!("{content_url}/themes/{template}"),
            stylesheet_uri: format!("{content_url}/themes/{stylesheet}"),
            home: site.settings.home.clone(),
            siteurl: site.settings.siteurl.clone(),
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
            pattern_cache: Mutex::new(HashMap::new()),
        })
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

    pub fn stylesheet_dir(&self) -> &Path {
        &self.stylesheet_dir
    }

    pub fn template_dir(&self) -> &Path {
        &self.template_dir
    }
}

impl ThemeAccess for Theme {
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
            match phpt::render(&source, &self.host) {
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

/// Block style variation partials (`styles/blocks/*.json`).
fn read_partials(dir: &Path) -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json")).collect();
    files.sort();
    files.iter().filter_map(|p| read_file(p)).collect()
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
