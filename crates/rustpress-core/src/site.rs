use rustpress_db::Db;

use crate::Result;
use crate::datetime::SiteTimezone;
use crate::options::Options;
use crate::rewrite::RewriteRules;
use crate::settings::Settings;

/// Everything loaded once per process: database handle, autoloaded options,
/// parsed settings and the compiled permalink rules.
pub struct Site {
    pub db: Db,
    pub options: Options,
    pub settings: Settings,
    pub rewrite: RewriteRules,
    pub timezone: SiteTimezone,
    /// Custom post types and taxonomies, inferred from the rewrite rules.
    pub types: crate::types::Types,
    /// Translations of the site's locale; empty (English) until [Site::init_i18n] ran.
    i18n: std::sync::OnceLock<std::sync::Arc<crate::i18n::I18n>>,
}

impl Site {
    pub async fn load(db: Db) -> Result<Self> {
        let options = Options::load(&db).await?;
        let mut site = Self::from_parts(db, options);
        if !site.rewrite.is_empty() {
            let post_types = site.db.used_post_types().await?;
            let taxonomies = site.db.used_taxonomies().await?;
            let relations = site.db.taxonomy_post_types().await?;
            let with_parents = site.db.taxonomies_with_parents().await?;
            site.types = crate::types::Types::infer(&site.rewrite, &post_types, &taxonomies, &relations, &with_parents);
        }
        Ok(site)
    }

    /// [Site::load] plus what the plugins and the theme under `wp_root` register: labels and
    /// REST settings of the custom post types and taxonomies.
    pub async fn load_at(db: Db, wp_root: &std::path::Path) -> Result<Self> {
        let mut site = Self::load(db).await?;
        if !site.types.post_types.is_empty() || !site.types.taxonomies.is_empty() {
            let plugins: Vec<String> = site
                .options
                .value("active_plugins")
                .and_then(|v| v.as_array().map(|items| items.iter().filter_map(|(_, p)| p.as_str().map(str::to_owned)).collect()))
                .unwrap_or_default();
            let themes = wp_root.join("wp-content").join("themes");
            let mut dirs = vec![themes.join(&site.settings.stylesheet)];
            if site.settings.template != site.settings.stylesheet {
                dirs.push(themes.join(&site.settings.template));
            }
            let registrations = crate::types::scan_registrations(wp_root, &plugins, &dirs);
            site.types.apply(&registrations);
        }
        Ok(site)
    }

    /// `get_post_type_object( $name )->labels->name`.
    pub fn post_type_label(&self, name: &str) -> String {
        match name {
            "post" => self.trx("Posts", "post type general name"),
            "page" => self.trx("Pages", "post type general name"),
            "attachment" => self.trx("Media", "post type general name"),
            other => self.types.post_type(other).map_or_else(|| other.to_owned(), |t| t.plural_label()),
        }
    }

    /// `is_post_type_viewable( $post_type )`.
    pub fn is_post_type_viewable(&self, post_type: &str) -> bool {
        self.types.is_viewable(post_type, !self.rewrite.is_empty())
    }

    /// `post_type_supports( $post_type, $feature )`.
    pub fn post_type_supports(&self, post_type: &str, feature: &str) -> bool {
        match post_type {
            "post" => matches!(feature, "title" | "editor" | "author" | "thumbnail" | "excerpt" | "trackbacks" | "custom-fields" | "comments" | "revisions" | "post-formats"),
            "page" => matches!(feature, "title" | "editor" | "author" | "thumbnail" | "page-attributes" | "custom-fields" | "comments" | "revisions" | "excerpt"),
            "attachment" => matches!(feature, "title" | "author" | "comments"),
            other => self.types.post_type(other).is_none_or(|t| t.supports(feature)),
        }
    }

    /// `get_taxonomy( $name )->labels->singular_name`.
    pub fn taxonomy_singular_label(&self, name: &str) -> String {
        match name {
            "category" => self.trx("Category", "taxonomy singular name"),
            "post_tag" => self.trx("Tag", "taxonomy singular name"),
            "post_format" => self.trx("Format", "post format"),
            other => self.types.taxonomy(other).map_or_else(|| other.to_owned(), |t| t.singular_label()),
        }
    }

    /// `get_taxonomy( $name )->labels->name`.
    pub fn taxonomy_label(&self, name: &str) -> String {
        match name {
            "category" => self.trx("Categories", "taxonomy general name"),
            "post_tag" => self.trx("Tags", "taxonomy general name"),
            "post_format" => self.trx("Formats", "post format"),
            other => self.types.taxonomy(other).map_or_else(|| other.to_owned(), |t| t.plural_label()),
        }
    }

    /// The locale (`WPLANG`), `en_US` if unset.
    pub fn locale(&self) -> String {
        self.options.get("WPLANG").filter(|l| !l.is_empty()).unwrap_or("en_US").to_owned()
    }

    /// Loads the catalogs for the site's locale from `wp_root` (once; later calls do nothing).
    pub fn init_i18n(&self, wp_root: &std::path::Path, themes: &[(String, std::path::PathBuf)]) {
        let _ = self.i18n.set(std::sync::Arc::new(crate::i18n::I18n::load(wp_root, &self.locale(), themes)));
    }

    pub fn i18n(&self) -> std::sync::Arc<crate::i18n::I18n> {
        self.i18n.get().cloned().unwrap_or_default()
    }

    /// `__( $text )`.
    pub fn tr(&self, text: &str) -> String {
        match self.i18n.get() {
            Some(i18n) => i18n.gettext("default", text),
            None => text.to_owned(),
        }
    }

    /// `_x( $text, $context )`.
    pub fn trx(&self, text: &str, context: &str) -> String {
        match self.i18n.get() {
            Some(i18n) => i18n.pgettext("default", context, text),
            None => text.to_owned(),
        }
    }

    /// `_n( $single, $plural, $n )`.
    pub fn trn(&self, singular: &str, plural: &str, n: i64) -> String {
        match self.i18n.get() {
            Some(i18n) => i18n.ngettext("default", singular, plural, n),
            None => (if n == 1 { singular } else { plural }).to_owned(),
        }
    }
    /// The names and conventions of the site's language (`$wp_locale`).
    pub fn locale_data(&self) -> std::borrow::Cow<'_, crate::locale::LocaleData> {
        match self.i18n.get() {
            Some(i18n) => std::borrow::Cow::Borrowed(i18n.locale_data()),
            None => std::borrow::Cow::Owned(crate::locale::LocaleData::from_i18n(&crate::i18n::I18n::default())),
        }
    }

    /// The name of a term as `get_term()` returns it: post formats have their translated name.
    pub fn term_name(&self, term: &rustpress_db::Term) -> String {
        if term.taxonomy != "post_format" {
            return term.name.clone();
        }
        match term.slug.as_str() {
            "post-format-aside" => self.trx("Aside", "Post format"),
            "post-format-chat" => self.trx("Chat", "Post format"),
            "post-format-gallery" => self.trx("Gallery", "Post format"),
            "post-format-link" => self.trx("Link", "Post format"),
            "post-format-image" => self.trx("Image", "Post format"),
            "post-format-quote" => self.trx("Quote", "Post format"),
            "post-format-status" => self.trx("Status", "Post format"),
            "post-format-video" => self.trx("Video", "Post format"),
            "post-format-audio" => self.trx("Audio", "Post format"),
            _ => term.name.clone(),
        }
    }

    /// `get_bloginfo( 'language' )`: the translation of `html_lang_attribute`, else the locale with `-`.
    pub fn language_tag(&self) -> String {
        match self.i18n.get() {
            Some(i18n) => i18n.locale_data().language_tag.clone(),
            None => self.locale().replace('_', "-"),
        }
    }

    /// `wptexturize()` with the quotes and dashes of the site's language.
    pub fn texturize(&self, text: &str) -> String {
        match self.i18n.get().filter(|i| i.is_translated()) {
            Some(i18n) => crate::formatting::wptexturize_with(text, i18n.texturize()),
            None => crate::formatting::wptexturize(text),
        }
    }

    /// The default excerpt length: `_x( '55', 'excerpt_length' )`.
    pub fn excerpt_length(&self) -> usize {
        self.trx("55", "excerpt_length").trim().parse().unwrap_or(55)
    }

    /// `is_rtl()`.
    pub fn is_rtl(&self) -> bool {
        self.i18n.get().is_some_and(|i18n| i18n.locale_data().rtl)
    }

    /// `number_format_i18n( $n )` for whole numbers.
    pub fn number_i18n(&self, n: u64) -> String {
        self.locale_data().number(n)
    }

    /// `wp_date( $format, $timestamp )`: month and weekday names, abbreviations and `am`/`pm`
    /// come from the site's translation; months are declined where the language needs it.
    pub fn date(&self, format: &str, timestamp: i64) -> String {
        match self.i18n.get().filter(|i| i.is_translated()) {
            Some(i18n) => i18n.locale_data().date(&self.timezone, format, timestamp),
            None => self.timezone.format(format, timestamp),
        }
    }

    /// `date_i18n( $format, $timestamp_with_offset )`: the timestamp already carries the site's
    /// offset, so it is formatted as UTC.
    pub fn date_utc(&self, format: &str, timestamp: i64) -> String {
        let utc = crate::datetime::SiteTimezone::utc();
        match self.i18n.get().filter(|i| i.is_translated()) {
            Some(i18n) => i18n.locale_data().date(&utc, format, timestamp),
            None => utc.format(format, timestamp),
        }
    }

    pub fn from_parts(db: Db, options: Options) -> Self {
        let settings = Settings::from_options(&options);
        let rewrite = options.value("rewrite_rules").map(|v| RewriteRules::from_option(&v)).unwrap_or_default();
        for pattern in &rewrite.skipped {
            tracing::warn!(pattern, "rewrite rule uses regex features that are not supported; skipped");
        }
        let timezone = SiteTimezone::from_options(&options);
        Self { db, options, settings, rewrite, timezone, types: crate::types::Types::default(), i18n: std::sync::OnceLock::new() }
    }
}
