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
    /// Translations of the site's locale; empty (English) until [Site::init_i18n] ran.
    i18n: std::sync::OnceLock<std::sync::Arc<crate::i18n::I18n>>,
}

impl Site {
    pub async fn load(db: Db) -> Result<Self> {
        let options = Options::load(&db).await?;
        Ok(Self::from_parts(db, options))
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
        Self { db, options, settings, rewrite, timezone, i18n: std::sync::OnceLock::new() }
    }
}
