//! `wp-site-health/v1`: the Site Health tests the screen runs behind the scenes, and the directory sizes.

use rustpress_core::health;
use serde_json::Value;

use crate::error::RestError;
use crate::{Outcome, Resp, Rest, Result};

impl Rest<'_> {
    pub(crate) async fn health_route(&self, segments: &[&str]) -> Result<Outcome> {
        if !self.can("view_site_health_checks") {
            return Ok(Err(self.forbidden("rest_forbidden", "Sorry, you are not allowed to do that.")));
        }
        let site = self.site;
        let https = site.settings.siteurl.starts_with("https://");
        let data: Value = match segments {
            ["wp-site-health", "v1", "tests", "background-updates"] => health::background_updates(site),
            ["wp-site-health", "v1", "tests", "loopback-requests"] => health::loopback_requests(site),
            ["wp-site-health", "v1", "tests", "https-status"] => health::https_status(site, https),
            ["wp-site-health", "v1", "tests", "dotorg-communication"] => health::dotorg_communication(site),
            ["wp-site-health", "v1", "tests", "authorization-header"] => {
                let basic = self.request.as_ref().and_then(|r| r.basic.as_ref());
                health::authorization_header(site, basic.map(|(user, password)| (user.as_str(), password.as_str())))
            }
            ["wp-site-health", "v1", "tests", "page-cache"] => {
                // The time the home page takes to be put together, three times, and the caching headers RustPress sends.
                let mut times = Vec::new();
                for _ in 0..3 {
                    let start = std::time::Instant::now();
                    let _ = site.query("/", "").await?;
                    times.push(start.elapsed().as_millis() as u64);
                }
                times.sort_unstable();
                health::page_cache(site, times[1].max(1), &[])
            }
            ["wp-site-health", "v1", "directory-sizes"] => {
                let root = self.access.as_ref().and_then(|a| a.uploads.parent().and_then(|p| p.parent()).map(std::path::Path::to_path_buf)).unwrap_or_default();
                health::directory_sizes(site, &root).await
            }
            _ => return Ok(Err(RestError::new("rest_no_route", "No route was found matching the URL and request method.", 404))),
        };
        Ok(Ok(Resp::item(data)))
    }
}
