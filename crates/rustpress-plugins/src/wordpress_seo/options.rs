//! `WPSEO_Options`: the plugin's options (`wpseo`, `wpseo_titles`, `wpseo_social`, ...), each merged over its
//! defaults (`WPSEO_Option::get_option()` with `array_merge( $defaults, $options )`) and then all of them over each
//! other in the order of `WPSEO_Options::$options`.

use std::collections::HashMap;

use rustpress_core::Site;
use rustpress_phpser::{Key, Value};

use super::DOMAIN;

/// The option names in the order `WPSEO_Options::get_all()` merges them (`wpseo_ms` only exists on multisite).
const GROUPS: &[&str] = &["wpseo", "wpseo_titles", "wpseo_social", "wpseo_taxonomy_meta", "wpseo_llmstxt", "wpseo_tracking_only"];

/// All options of the plugin by key.
#[derive(Debug, Default)]
pub struct Options {
    values: HashMap<String, Value>,
    /// `WPSEO_Option_Titles::get_defaults()`, for `Options_Helper::get_title_default()`.
    title_defaults: HashMap<String, Value>,
}

fn s(v: &str) -> Value {
    Value::Str(v.as_bytes().to_vec())
}

/// The keys of a stored PHP array.
fn pairs(value: &Value) -> impl Iterator<Item = (String, &Value)> {
    value.as_array().unwrap_or_default().iter().map(|(k, v)| {
        let key = match k {
            Key::Int(n) => n.to_string(),
            Key::Str(b) => String::from_utf8_lossy(b).into_owned(),
        };
        (key, v)
    })
}

impl Options {
    pub fn load(site: &Site) -> Self {
        let titles = title_defaults(site);
        let mut values = HashMap::new();
        for group in GROUPS {
            let defaults: Vec<(String, Value)> = match *group {
                "wpseo" => WPSEO.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect(),
                "wpseo_titles" => titles.clone(),
                "wpseo_social" => SOCIAL.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect(),
                _ => Vec::new(),
            };
            values.extend(defaults);
            if let Some(stored) = site.options.value(group) {
                for (key, value) in pairs(&stored) {
                    values.insert(key, value.clone());
                }
            }
        }
        Self { values, title_defaults: titles.into_iter().collect() }
    }

    /// `WPSEO_Options::get( $key )`: unset and `null` values are missing.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key).filter(|v| !matches!(v, Value::Null))
    }

    /// The option as PHP would cast it to a string (missing: empty).
    pub fn string(&self, key: &str) -> String {
        self.get(key).map(php_string).unwrap_or_default()
    }

    /// `$this->options->get( $key ) === true`.
    pub fn is_true(&self, key: &str) -> bool {
        matches!(self.get(key), Some(Value::Bool(true)))
    }

    /// `if ( $this->options->get( $key ) )`.
    pub fn truthy(&self, key: &str) -> bool {
        self.get(key).is_some_and(truthy)
    }

    /// `Options_Helper::get_title_default()`.
    pub fn title_default(&self, key: &str) -> String {
        self.title_defaults.get(key).filter(|v| truthy(v)).map(php_string).unwrap_or_default()
    }

    /// `Options_Helper::get_title_separator()`.
    pub fn title_separator(&self) -> String {
        let separator = self.string("separator");
        SEPARATORS.iter().find(|(k, _)| *k == separator).or_else(|| SEPARATORS.iter().find(|(k, _)| *k == "sc-dash")).map(|(_, v)| (*v).to_owned()).unwrap_or_default()
    }

    /// `WPSEO_Taxonomy_Meta::get_term_meta( $term, $taxonomy )`: the plugin's fields of a term over their defaults.
    pub fn term_meta(&self, taxonomy: &str, term_id: u64) -> Vec<(String, String)> {
        let mut meta: Vec<(String, String)> = TERM_DEFAULTS.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        // The terms are keyed by their ID: an integer key of the PHP array.
        let stored = self.values.get(taxonomy).and_then(|tax| tax.as_array()).and_then(|terms| {
            terms.iter().find_map(|(k, v)| match k {
                Key::Int(n) if *n == term_id as i64 => Some(v),
                Key::Str(b) if b == term_id.to_string().as_bytes() => Some(v),
                _ => None,
            })
        });
        for (key, value) in stored.map(|v| pairs(v).collect::<Vec<_>>()).unwrap_or_default() {
            let value = php_string(value);
            match meta.iter_mut().find(|(k, _)| *k == key) {
                Some((_, v)) => *v = value,
                None => meta.push((key, value)),
            }
        }
        meta
    }
}

/// `(string) $value`.
pub fn php_string(value: &Value) -> String {
    match value {
        Value::Str(b) => String::from_utf8_lossy(b).into_owned(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => rustpress_blocks::general::php_float(*f),
        Value::Bool(true) => "1".to_owned(),
        _ => String::new(),
    }
}

/// PHP's truthiness.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(f) => *f != 0.0,
        Value::Str(b) => !(b.is_empty() || b == b"0"),
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

/// `WPSEO_Option_Titles::get_separator_option_list()`.
const SEPARATORS: &[(&str, &str)] = &[
    ("sc-dash", "-"),
    ("sc-ndash", "&#8211;"),
    ("sc-mdash", "&#8212;"),
    ("sc-colon", ":"),
    ("sc-middot", "&#183;"),
    ("sc-bull", "&#8226;"),
    ("sc-star", "*"),
    ("sc-smstar", "&#8902;"),
    ("sc-pipe", "|"),
    ("sc-tilde", "~"),
    ("sc-laquo", "&#171;"),
    ("sc-raquo", "&#187;"),
    ("sc-lt", "&#062;"),
    ("sc-gt", "&#060;"),
];

/// `WPSEO_Taxonomy_Meta::$defaults_per_term`.
const TERM_DEFAULTS: &[(&str, &str)] = &[
    ("wpseo_title", ""),
    ("wpseo_desc", ""),
    ("wpseo_canonical", ""),
    ("wpseo_bctitle", ""),
    ("wpseo_noindex", "default"),
    ("wpseo_focuskw", ""),
    ("wpseo_linkdex", ""),
    ("wpseo_content_score", ""),
    ("wpseo_inclusive_language_score", ""),
    ("wpseo_focuskeywords", "[]"),
    ("wpseo_keywordsynonyms", "[]"),
    ("wpseo_is_cornerstone", "0"),
    ("wpseo_opengraph-title", ""),
    ("wpseo_opengraph-description", ""),
    ("wpseo_opengraph-image", ""),
    ("wpseo_opengraph-image-id", ""),
    ("wpseo_twitter-title", ""),
    ("wpseo_twitter-description", ""),
    ("wpseo_twitter-image", ""),
    ("wpseo_twitter-image-id", ""),
];

/// The defaults of `WPSEO_Option_Wpseo` the website reads.
static WPSEO: std::sync::LazyLock<Vec<(&'static str, Value)>> = std::sync::LazyLock::new(|| {
    vec![
        ("enable_enhanced_slack_sharing", Value::Bool(true)),
        ("enable_schema", Value::Bool(true)),
        ("enable_xml_sitemap", Value::Bool(true)),
        ("dynamic_permalinks", Value::Bool(false)),
        ("baiduverify", s("")),
        ("googleverify", s("")),
        ("msverify", s("")),
        ("yandexverify", s("")),
        ("ahrefsverify", s("")),
    ]
});

/// `WPSEO_Option_Social::$defaults`.
static SOCIAL: std::sync::LazyLock<Vec<(&'static str, Value)>> = std::sync::LazyLock::new(|| {
    vec![
        ("facebook_site", s("")),
        ("instagram_url", s("")),
        ("linkedin_url", s("")),
        ("myspace_url", s("")),
        ("og_default_image", s("")),
        ("og_default_image_id", s("")),
        ("og_frontpage_title", s("")),
        ("og_frontpage_desc", s("")),
        ("og_frontpage_image", s("")),
        ("og_frontpage_image_id", s("")),
        ("opengraph", Value::Bool(true)),
        ("pinterest_url", s("")),
        ("pinterestverify", s("")),
        ("twitter", Value::Bool(true)),
        ("twitter_site", s("")),
        ("twitter_card_type", s("summary_large_image")),
        ("youtube_url", s("")),
        ("wikipedia_url", s("")),
        ("other_social_urls", Value::Array(Vec::new())),
        ("mastodon_url", s("")),
    ]
});

/// `WPSEO_Option_Titles::get_defaults()`: the fixed defaults with `translate_defaults()` and `enrich_defaults()`.
fn title_defaults(site: &Site) -> Vec<(String, Value)> {
    let tr = |text: &str| site.trd(DOMAIN, text);
    let sprintf = |format: &str, args: &[&str]| rustpress_core::i18n::sprintf(&tr(format), args);
    let mut d: Vec<(String, Value)> = Vec::new();
    let mut put = |key: &str, value: Value| d.push((key.to_owned(), value));
    put("forcerewritetitle", Value::Bool(false));
    put("separator", s("sc-dash"));
    put("title-home-wpseo", s("%%sitename%% %%page%% %%sep%% %%sitedesc%%"));
    put("title-author-wpseo", s(&(sprintf("%1$s, Author at %2$s", &["%%name%%", "%%sitename%%"]) + " %%page%% ")));
    put("title-archive-wpseo", s("%%date%% %%page%% %%sep%% %%sitename%%"));
    put("title-search-wpseo", s(&(sprintf("You searched for %s", &["%%searchphrase%%"]) + " %%page%% %%sep%% %%sitename%%")));
    put("title-404-wpseo", s(&(tr("Page not found") + " %%sep%% %%sitename%%")));
    put("social-title-author-wpseo", s("%%name%%"));
    put("social-title-archive-wpseo", s("%%date%%"));
    for key in ["social-description-author-wpseo", "social-description-archive-wpseo", "social-image-url-author-wpseo", "social-image-url-archive-wpseo"] {
        put(key, s(""));
    }
    put("social-image-id-author-wpseo", Value::Int(0));
    put("social-image-id-archive-wpseo", Value::Int(0));
    for key in ["metadesc-home-wpseo", "metadesc-author-wpseo", "metadesc-archive-wpseo", "rssbefore"] {
        put(key, s(""));
    }
    put("rssafter", s(&sprintf("The post %1$s appeared first on %2$s.", &["%%POSTLINK%%", "%%BLOGLINK%%"])));
    put("noindex-author-wpseo", Value::Bool(false));
    put("noindex-author-noposts-wpseo", Value::Bool(true));
    put("noindex-archive-wpseo", Value::Bool(true));
    put("disable-author", Value::Bool(false));
    put("disable-date", Value::Bool(false));
    put("disable-post_format", Value::Bool(false));
    put("disable-attachment", Value::Bool(true));
    put("breadcrumbs-404crumb", s(&tr("Error 404: Page not found")));
    put("breadcrumbs-display-blog-page", Value::Bool(true));
    put("breadcrumbs-boldlast", Value::Bool(false));
    put("breadcrumbs-archiveprefix", s(&tr("Archives for")));
    put("breadcrumbs-enable", Value::Bool(true));
    put("breadcrumbs-home", s(&tr("Home")));
    put("breadcrumbs-prefix", s(""));
    put("breadcrumbs-searchprefix", s(&tr("You searched for")));
    put("breadcrumbs-sep", s("»"));
    for key in ["website_name", "person_name", "person_logo"] {
        put(key, s(""));
    }
    put("person_logo_id", Value::Int(0));
    put("alternate_website_name", s(""));
    put("company_logo", s(""));
    put("company_logo_id", Value::Int(0));
    put("company_logo_meta", Value::Bool(false));
    put("person_logo_meta", Value::Bool(false));
    put("company_name", s(""));
    put("company_alternate_name", s(""));
    put("company_or_person", s("company"));
    put("company_or_person_user_id", Value::Bool(false));
    put("stripcategorybase", Value::Bool(false));
    put("open_graph_frontpage_title", s("%%sitename%%"));
    put("open_graph_frontpage_desc", s(""));
    put("open_graph_frontpage_image", s(""));
    put("open_graph_frontpage_image_id", Value::Int(0));

    // `enrich_defaults()`.
    let types = &site.types;
    let archive = sprintf("%s Archive", &["%%pt_plural%%"]);
    for name in types.public_post_types() {
        put(&format!("title-{name}"), s("%%title%% %%page%% %%sep%% %%sitename%%"));
        put(&format!("metadesc-{name}"), s(""));
        put(&format!("noindex-{name}"), Value::Bool(false));
        put(&format!("display-metabox-pt-{name}"), Value::Bool(true));
        put(&format!("post_types-{name}-maintax"), Value::Int(0));
        put(&format!("schema-page-type-{name}"), s("WebPage"));
        put(&format!("schema-article-type-{name}"), s(if name == "post" { "Article" } else { "None" }));
        if name != "attachment" {
            put(&format!("social-title-{name}"), s("%%title%%"));
            put(&format!("social-description-{name}"), s(""));
            put(&format!("social-image-url-{name}"), s(""));
            put(&format!("social-image-id-{name}"), Value::Int(0));
        }
        let custom_with_archive = types.post_type(name).is_some_and(|t| t.archive_slug.is_some()) && !matches!(name, "post" | "page" | "attachment");
        if custom_with_archive {
            put(&format!("title-ptarchive-{name}"), s(&format!("{archive} %%page%% %%sep%% %%sitename%%")));
            put(&format!("metadesc-ptarchive-{name}"), s(""));
            put(&format!("bctitle-ptarchive-{name}"), s(""));
            put(&format!("noindex-ptarchive-{name}"), Value::Bool(false));
            put(&format!("social-title-ptarchive-{name}"), s(&archive));
            put(&format!("social-description-ptarchive-{name}"), s(""));
            put(&format!("social-image-url-ptarchive-{name}"), s(""));
            put(&format!("social-image-id-ptarchive-{name}"), Value::Int(0));
        }
    }
    let archives = sprintf("%s Archives", &["%%term_title%%"]);
    for name in public_taxonomies(site) {
        put(&format!("title-tax-{name}"), s(&format!("{archives} %%page%% %%sep%% %%sitename%%")));
        put(&format!("metadesc-tax-{name}"), s(""));
        put(&format!("display-metabox-tax-{name}"), Value::Bool(true));
        put(&format!("noindex-tax-{name}"), Value::Bool(name == "post_format"));
        put(&format!("social-title-tax-{name}"), s(&archives));
        put(&format!("social-description-tax-{name}"), s(""));
        put(&format!("social-image-url-tax-{name}"), s(""));
        put(&format!("social-image-id-tax-{name}"), Value::Int(0));
        put(&format!("taxonomy-{name}-ptparent"), Value::Int(0));
    }
    d
}

/// `get_taxonomies( [ 'public' => true ] )`.
pub fn public_taxonomies(site: &Site) -> Vec<String> {
    let mut all: Vec<String> = ["category", "post_tag", "post_format"].map(String::from).to_vec();
    for t in site.types.taxonomies.iter().filter(|t| t.public) {
        if !all.contains(&t.name) {
            all.push(t.name.clone());
        }
    }
    all
}
