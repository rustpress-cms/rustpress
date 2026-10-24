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
}

impl Site {
    pub async fn load(db: Db) -> Result<Self> {
        let options = Options::load(&db).await?;
        Ok(Self::from_parts(db, options))
    }

    pub fn from_parts(db: Db, options: Options) -> Self {
        let settings = Settings::from_options(&options);
        let rewrite = options.value("rewrite_rules").map(|v| RewriteRules::from_option(&v)).unwrap_or_default();
        for pattern in &rewrite.skipped {
            tracing::warn!(pattern, "rewrite rule uses regex features that are not supported; skipped");
        }
        let timezone = SiteTimezone::from_options(&options);
        Self { db, options, settings, rewrite, timezone }
    }
}
