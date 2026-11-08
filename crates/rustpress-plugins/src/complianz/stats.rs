//! The statistics scripts of `cmplz_banner_loader`: `inline_cookie_script()`, `inline_cookie_script_no_warning()`
//! and the actions they fire (`add_gtag_js()`, `add_clicky_js()`, `get_statistics_script()`,
//! `get_tagmanager_script()`), with the templates of `templates/statistics/` read from the plugin (or the theme's
//! `complianz-gdpr/templates/` folder, as `cmplz_get_template()` prefers it).

use std::path::Path;

use super::options::{item, php_string, truthy, Options};

/// What the scripts need besides the options.
pub struct Context<'a> {
    pub options: &'a Options,
    /// `get_statistics_category()`.
    pub category: &'a str,
    pub plugin_dir: &'a Path,
    pub theme_dir: &'a Path,
    /// `get_option( 'complianz_options_custom-scripts' )`.
    pub custom_scripts: &'a rustpress_phpser::Value,
    /// `get_cookie_prefix()`.
    pub cookie_prefix: &'a str,
}

fn esc_attr(text: &str) -> String {
    rustpress_core::formatting::esc_attr(text)
}

/// `esc_js()` for the short texts it gets here (a country code).
fn esc_js(text: &str) -> String {
    let text = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\r', "");
    text.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n")
}

impl Context<'_> {
    /// `cmplz_get_template( "statistics/$name" )`.
    fn template(&self, name: &str) -> String {
        let theme = self.theme_dir.join("complianz-gdpr/templates/statistics").join(name);
        let file = self.plugin_dir.join("templates/statistics").join(name);
        if !file.exists() {
            return String::new();
        }
        let file = if theme.exists() { theme } else { file };
        std::fs::read_to_string(file).unwrap_or_default()
    }

    fn option(&self, id: &str) -> String {
        self.options.string(id)
    }

    fn is(&self, id: &str, value: &str) -> bool {
        self.options.is(id, value)
    }

    /// `do_action( 'cmplz_before_statistics_script' )`: `add_gtag_js()`, then `add_clicky_js()`.
    fn before_statistics_script(&self) -> String {
        let mut out = String::new();
        let statistics = self.option("compile_statistics");
        let code = self.option("ua_code");
        if !self.is("configuration_by_complianz", "no") && statistics == "google-analytics" && truthy(&self.options.get("ua_code")) {
            out.push_str(&format!("\t\t\t\t<script async data-category=\"{}\"\n\t\t\t\t\t\tsrc=\"https://www.googletagmanager.com/gtag/js?id={}\"></script>\n\t\t\t\t\t\t", esc_attr(self.category), esc_attr(&code)));
        }
        if statistics == "clicky" {
            let plain = if self.category == "functional" { "" } else { "type=\"text/plain\"" };
            out.push_str(&format!("\t\t\t\t<script async {plain}\t\t\t\t\t\tdata-category=\"{}\" src=\"//static.getclicky.com/js\"></script>\n\t\t\t\t", esc_attr(self.category)));
        }
        out
    }

    /// `get_statistics_script()`.
    fn statistics_script(&self) -> String {
        if self.is("configuration_by_complianz", "no") {
            return String::new();
        }
        match self.option("compile_statistics").as_str() {
            "google-analytics" => {
                let mut consent_mode = if self.options.consent_mode() { "-consent-mode".to_owned() } else { String::new() };
                if !consent_mode.is_empty() && self.is("gtag-basic-consent-mode", "yes") {
                    consent_mode.push_str("-basic");
                }
                let code = esc_attr(&self.option("ua_code"));
                let mut additional = String::new();
                for tag in self.option("additional_gtags_stats").split(',').map(str::trim) {
                    if !tag.is_empty() && tag != "0" {
                        additional.push_str(&format!("gtag('config', '{}' );", esc_attr(tag)));
                    }
                }
                let more_info = self.options.get("compile_statistics_more_info");
                let ip_blocked = matches!(&more_info, rustpress_phpser::Value::Array(items) if items.iter().any(|(_, v)| php_string(v) == "ip-addresses-blocked" && matches!(v, rustpress_phpser::Value::Str(_))));
                let anonymize = if ip_blocked && !code.to_uppercase().starts_with("G-") { "'anonymize_ip': true" } else { "" };
                let flag = |id: &str| if self.is(id, "yes") { "true" } else { "false" };
                self.template(&format!("gtag{consent_mode}.js"))
                    .replace("{G_code}", &code)
                    .replace("{additional_tags}", &additional)
                    .replace("{anonymize_ip}", anonymize)
                    .replace("{enable_tcf_support}", "false")
                    .replace("{ads_data_redaction}", flag("cmplz-gtag-ads_data_redaction"))
                    .replace("{url_passthrough}", flag("cmplz-gtag-urlpassthrough"))
            }
            "matomo" => {
                let cookieless = if self.is("matomo_anonymized", "yes") { "-cookieless" } else { "" };
                self.template(&format!("matomo{cookieless}.js"))
                    .replace("{site_id}", &esc_attr(&self.option("matomo_site_id")))
                    .replace("{matomo_url}", &rustpress_core::formatting::esc_url_raw(&rustpress_core::urls::trailingslashit(&self.option("matomo_url"))))
            }
            "clicky" => self.template("clicky.js").replace("{site_ID}", &esc_attr(&self.option("clicky_site_id"))),
            "clarity" => {
                let variant = if self.is("clarity_consent_mode", "yes") && self.is("consent_for_anonymous_stats", "yes") { "-consent-mode" } else { "" };
                self.template(&format!("clarity{variant}.js")).replace("{site_ID}", &esc_attr(&self.option("clarity_id")))
            }
            "yandex" => {
                let ecommerce = if self.is("yandex_ecommerce", "yes") { "ecommerce:\"dataLayer\"" } else { "" };
                self.template("yandex.js").replace("{yandex_id}", &self.option("yandex_id")).replace("{ecommerce}", ecommerce)
            }
            _ => String::new(),
        }
    }

    /// `get_tagmanager_script()`.
    fn tagmanager_script(&self) -> String {
        if !self.is("configuration_by_complianz", "yes") {
            return String::new();
        }
        match self.option("compile_statistics").as_str() {
            "google-tag-manager" => {
                let template = if self.is("cmplz-tm-template", "yes") { "-template" } else { "" };
                let consent_mode = if self.options.consent_mode() { format!("-consent-mode{template}") } else { String::new() };
                self.template(&format!("google-tag-manager{consent_mode}.js")).replace("{gtm_code}", &esc_attr(&self.option("gtm_code")))
            }
            "matomo-tag-manager" => self
                .template("matomo-tag-manager.js")
                .replace("{container_id}", &esc_attr(&self.option("matomo_container_id")))
                .replace("{matomo_url}", &rustpress_core::formatting::esc_url_raw(&rustpress_core::urls::trailingslashit(&self.option("matomo_tag_url")))),
            _ => String::new(),
        }
    }

    /// `add_amazon_consent_signal()` (no TCF and no geo IP in the free plugin).
    fn amazon_consent_signal(&self) -> String {
        if !self.is("enable_amazon_consent_signal", "yes") {
            return String::new();
        }
        let country = self.option("country_company");
        let country = if truthy(&self.options.get("country_company")) { country } else { "US".into() };
        let script = self.template("amazon-consent-signal.js").replace("{country_code}", &esc_js(&country));
        format!("<!-- Amazon Consent Signal Complianz GDPR/CCPA -->\n\t\t\t<script data-category=\"functional\">{script}</script>\n\t\t\t")
    }

    /// `inline_cookie_script()`.
    pub fn inline_cookie_script(&self) -> String {
        let mut out = self.amazon_consent_signal();
        let statistics = self.option("compile_statistics");
        let aw_code = self.option("aw_code");
        let mut additional = String::new();
        for tag in self.option("additional_gtags_marketing").split(',').map(str::trim) {
            additional.push_str(&format!("gtag('config', '{}' );", esc_attr(tag)));
        }
        out.push_str(&self.before_statistics_script());
        let comment = "<!-- Statistics script Complianz GDPR/CCPA -->\n";
        if self.is("configuration_by_complianz", "yes") && statistics != "no" {
            if statistics == "google-tag-manager" || statistics == "matomo-tag-manager" {
                let script = self.tagmanager_script();
                if !script.is_empty() && script != "0" {
                    out.push_str(comment);
                    out.push_str(&format!("\t\t\t\t\t\t<script data-category=\"{}\">\n\t\t\t\t\t\t\t{script}\t\t\t\t\t\t</script>\n\t\t\t\t\t\t", esc_attr(self.category)));
                }
            } else {
                let script = self.statistics_script();
                if !script.is_empty() && script != "0" {
                    let plain = if self.category == "functional" { "" } else { "type=\"text/plain\"" };
                    out.push_str(comment);
                    out.push_str(&format!("\t\t\t\t\t\t<script {plain}\t\t\t\t\t\t\tdata-category=\"{}\">{script}</script>\n\t\t\t\t\t\t\t", esc_attr(self.category)));
                }
            }
            if truthy(&self.options.get("aw_code")) && statistics == "google-analytics" {
                let script = self.template("gtag-remarketing.js").replace("{aw_code}", &aw_code).replace("{additional_tags}", &additional);
                if self.options.consent_mode() && !self.is("gtag-basic-consent-mode", "yes") {
                    out.push_str(&format!("\t\t\t\t\t\t<script data-category=\"functional\">{script}</script>\n\t\t\t\t\t\t"));
                } else {
                    out.push_str(&format!("\t\t\t\t\t\t<script type=\"text/plain\" data-category=\"marketing\">{script}</script>\n\t\t\t\t\t\t"));
                }
            }
        }
        if matches!(self.options.get("disable_cookie_block"), rustpress_phpser::Value::Int(1)) {
            return out;
        }
        let Some(rustpress_phpser::Value::Array(scripts)) = item(self.custom_scripts, "add_script") else { return out };
        for (_, script) in scripts {
            if !item(script, "enable").is_some_and(truthy) {
                continue;
            }
            let editor = item(script, "editor").map(php_string).unwrap_or_default();
            if editor.is_empty() || editor == "0" {
                continue;
            }
            let category = item(script, "category").map(php_string).unwrap_or_default();
            let asynchronous = if item(script, "async").is_some_and(truthy) { "async" } else { "" };
            out.push_str(&format!("<!-- Script Center {category} script Complianz GDPR/CCPA -->\n"));
            out.push_str(&format!("\t\t\t\t<script {asynchronous} type=\"text/plain\"\n\t\t\t\t\t\t\t\t\t\t\tdata-category=\"{}\">\n\t\t\t\t\t{editor}\n\t\t\t\t</script>\n\t\t\t\t", esc_attr(&category)));
        }
        out
    }

    /// `inline_cookie_script_no_warning()`.
    pub fn inline_cookie_script_no_warning(&self) -> String {
        format!("{}\t\t\t<script data-category=\"functional\">\n\t\t\t\t{}\t\t\t\t{}\t\t\t</script>\n\t\t\t", self.before_statistics_script(), self.statistics_script(), self.tagmanager_script())
    }

    /// `inject_gtg_gtm_consent_script()` / `inject_gtg_ga_consent_script()` on `wp_head` at priority 0.
    pub fn gtg_consent_script(&self) -> String {
        if self.option("compile_statistics") != "gtg" {
            return String::new();
        }
        let flag = |id: &str| if self.is(id, "yes") { "true" } else { "false" };
        let (comment, script) = match self.option("compile_statistics_more_info_gtg").as_str() {
            "google-tag-manager" => (
                "<!-- GTG GTM script Complianz GDPR/CCPA -->",
                self.template("gtg-tag-manager-consent-mode.js").replace("{ads_data_redaction}", flag("cmplz-gtag-ads_data_redaction")).replace("{url_passthrough}", flag("cmplz-gtag-urlpassthrough")),
            ),
            "google-analytics" => (
                "<!-- GTG GA4 Consent Mode script Complianz GDPR/CCPA -->",
                self.template("gtg-consent-mode.js")
                    .replace("{enable_tcf_support}", "false")
                    .replace("{ads_data_redaction}", flag("cmplz-gtag-ads_data_redaction"))
                    .replace("{url_passthrough}", flag("cmplz-gtag-urlpassthrough"))
                    .replace("{G_code}", &esc_attr(&self.option("ua_code")))
                    .replace("{cookie_prefix}", self.cookie_prefix),
            ),
            _ => return String::new(),
        };
        if script.is_empty() {
            return String::new();
        }
        format!("{comment}\n<script>{script}</script>\n")
    }
}
