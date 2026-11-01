//! The presenters of `Front_End_Integration::present_head()` and the `wp_robots` filter of `WP_Robots_Integration`.

use rustpress_core::Result;
use rustpress_core::formatting::{esc_attr, esc_html, esc_url_raw, wp_strip_all_tags};
use rustpress_core::native::Robot;

use super::presentation::{Kind, Presentation};
use super::replace_vars::{self, php_trim};
use super::{Page, schema};

/// `Abstract_Indexable_Tag_Presenter::present()`: nothing for an empty value.
fn tag(format: Format, key: &str, value: &str, class: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    match format {
        Format::Property => format!("<meta property=\"{key}\" content=\"{}\"{class} />", esc_attr(value)),
        Format::Name => format!("<meta name=\"{key}\" content=\"{}\"{class} />", esc_attr(value)),
        Format::Link => format!("<link rel=\"{key}\" href=\"{}\"{class} />", esc_url_raw(value)),
        Format::NameUrl => format!("<meta name=\"{key}\" content=\"{}\"{class} />", esc_url_raw(value)),
    }
}

#[derive(Clone, Copy)]
enum Format {
    Property,
    Name,
    Link,
    /// A `name` tag whose content is a URL (`$escaping = 'url'`).
    NameUrl,
}

/// `Title_Presenter::get_title()`.
pub async fn title(page: &Page<'_>, p: &Presentation) -> Result<String> {
    let title = replace_vars::replace(page, &p.title, p.source()).await?;
    Ok(php_trim(&wp_strip_all_tags(&title)).to_owned())
}

/// `stripslashes()`.
fn stripslashes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('0') => out.push('\0'),
                Some(next) => out.push(next),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// What `call_wpseo_head()` prints.
pub async fn present(page: &Page<'_>, p: &Presentation, title_tag: bool) -> Result<String> {
    let o = page.options;
    let class = if page.admin_bar { " class=\"yoast-seo-meta-tag\"" } else { "" };
    let source = p.source();
    let kind = p.kind;
    let opengraph = o.is_true("opengraph");
    let singular = matches!(kind, Kind::PostType | Kind::StaticHomePage);
    let noindex = p.robots.iter().any(|(_, v)| v == "noindex");
    let mut out: Vec<String> = Vec::new();
    out.push(format!("<!-- This site is optimized with the Yoast SEO plugin v{} - https://yoast.com/product/yoast-seo-wordpress/ -->", super::VERSION));
    // The title presenter only prints for a theme with `title-tag` support (or with `forcerewritetitle`).
    if title_tag || o.truthy("forcerewritetitle") {
        let title = title(page, p).await?;
        if !title.is_empty() {
            out.push(format!("<title>{}</title>", esc_html(&title)));
        }
    }
    let description = replace_vars::replace(page, &p.meta_description, source).await?;
    out.push(tag(Format::Name, "description", php_trim(&wp_strip_all_tags(&stripslashes(&description))), class));
    if kind == Kind::ErrorPage {
        if opengraph {
            out.push(tag(Format::Property, "og:locale", &locale(page), class));
            let og_title = replace_vars::replace(page, &p.og_title, source).await?;
            out.push(tag(Format::Property, "og:title", &wp_strip_all_tags(php_trim(&og_title)), class));
            out.push(tag(Format::Property, "og:site_name", php_trim(&p.og_site_name), class));
        }
    } else {
        if !noindex {
            out.push(tag(Format::Link, "canonical", &super::urldecode(php_trim(&p.canonical)), class));
            out.push(tag(Format::Link, "prev", php_trim(&adjacent_rel_url(page, p, &p.rel_prev, page.pagination_previous.as_deref())), class));
            out.push(tag(Format::Link, "next", php_trim(&adjacent_rel_url(page, p, &p.rel_next, page.pagination_next.as_deref())), class));
        }
        if opengraph {
            out.push(tag(Format::Property, "og:locale", &locale(page), class));
            out.push(tag(Format::Property, "og:type", p.og_type, class));
            let og_title = replace_vars::replace(page, &p.og_title, source).await?;
            out.push(tag(Format::Property, "og:title", &wp_strip_all_tags(php_trim(&og_title)), class));
            let og_description = replace_vars::replace(page, &p.og_description, source).await?;
            out.push(tag(Format::Property, "og:description", php_trim(&wp_strip_all_tags(&stripslashes(&og_description))), class));
            out.push(tag(Format::Property, "og:url", &super::urldecode(&p.og_url), class));
            out.push(tag(Format::Property, "og:site_name", php_trim(&p.og_site_name), class));
            if singular {
                out.push(tag(Format::Property, "article:publisher", php_trim(&p.article_publisher), class));
                out.push(tag(Format::Property, "article:author", php_trim(&p.article_author), class));
                out.push(tag(Format::Property, "article:published_time", &p.article_published_time, class));
                out.push(tag(Format::Property, "article:modified_time", &p.article_modified_time, class));
            }
            out.push(og_images(page, p, class));
            if singular {
                out.push(meta_author(page, p, class));
            }
        }
        if o.is_true("twitter") {
            out.push(tag(Format::Name, "twitter:card", "summary_large_image", class));
            let twitter_title = replace_vars::replace(page, &p.twitter_title, source).await?;
            out.push(tag(Format::Name, "twitter:title", php_trim(&twitter_title), class));
            let twitter_description = replace_vars::replace(page, &p.twitter_description, source).await?;
            out.push(tag(Format::Name, "twitter:description", &twitter_description, class));
            out.push(tag(Format::NameUrl, "twitter:image", &p.twitter_image, class));
            if singular {
                out.push(tag(Format::Name, "twitter:creator", &p.twitter_creator, class));
            }
            out.push(tag(Format::Name, "twitter:site", &twitter_site(&p.twitter_site), class));
        }
        if o.is_true("enable_enhanced_slack_sharing") && kind == Kind::PostType {
            out.push(slack(page, p, class).await?);
        }
    }
    if o.get("enable_schema").is_none_or(|v| !matches!(v, rustpress_phpser::Value::Bool(false))) {
        out.push(schema_tag(page, p).await?);
    }
    if matches!(kind, Kind::HomePage | Kind::StaticHomePage) {
        for (key, name) in [("ahrefsverify", "ahrefs-site-verification"), ("baiduverify", "baidu-site-verification"), ("msverify", "msvalidate.01"), ("googleverify", "google-site-verification"), ("pinterestverify", "p:domain_verify"), ("yandexverify", "yandex-verification")] {
            out.push(tag(Format::Name, name, &o.string(key), class));
        }
    }
    out.push("<!-- / Yoast SEO plugin. -->".to_owned());
    let mut html = String::from("\n");
    for line in out.into_iter().filter(|l| !l.is_empty()) {
        html.push('\t');
        html.push_str(&line);
        html.push('\n');
    }
    html.push_str("\n\n");
    Ok(html)
}

/// `Front_End_Integration::adjacent_rel_url()` (the `wpseo_adjacent_rel_url` filter, added when a query block that
/// does not inherit the main query was rendered): the link of the query's pagination block.
fn adjacent_rel_url(page: &Page<'_>, p: &Presentation, link: &str, html: Option<&str>) -> String {
    static HREF: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r#"(?is)<a\b[^>]*?\shref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).expect("valid pattern"));
    if !link.is_empty() || !page.custom_query_loop {
        return link.to_owned();
    }
    let Some(caps) = html.and_then(|h| HREF.captures(h)) else { return link.to_owned() };
    let href = rustpress_core::formatting::html_entity_decode(caps.get(1).or(caps.get(2)).or(caps.get(3)).map_or("", |m| m.as_str()));
    if href.is_empty() {
        return link.to_owned();
    }
    if href.starts_with("http") {
        return href;
    }
    if href.starts_with('/') {
        return format!("{}{}", p.permalink, href.rsplit('/').next().unwrap_or_default());
    }
    link.to_owned()
}

/// `Twitter\Site_Presenter`: the account name at the end of the setting.
fn twitter_site(value: &str) -> String {
    static ID: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"([A-Za-z0-9_]{1,25})$").expect("valid pattern"));
    match ID.captures(value) {
        Some(c) => format!("@{}", &c[1]),
        None => String::new(),
    }
}

/// `Open_Graph\Image_Presenter::present()`.
fn og_images(page: &Page<'_>, p: &Presentation, class: &str) -> String {
    let mut out = String::new();
    for image in &p.og_images {
        let url = if page.query.flags.is_attachment { page.site.home_url(page.query.url_path.trim_matches('/')) } else { image.url.trim().to_owned() };
        if !out.is_empty() {
            out.push_str("\n\t");
        }
        out.push_str(&format!("<meta property=\"og:image\" content=\"{}\"{class} />", esc_url_raw(&url)));
        if image.width > 0 {
            out.push_str(&format!("\n\t<meta property=\"og:image:width\" content=\"{}\"{class} />", image.width));
        }
        if image.height > 0 {
            out.push_str(&format!("\n\t<meta property=\"og:image:height\" content=\"{}\"{class} />", image.height));
        }
        if !image.mime.trim().is_empty() {
            out.push_str(&format!("\n\t<meta property=\"og:image:type\" content=\"{}\"{class} />", esc_attr(image.mime.trim())));
        }
    }
    out
}

/// `Meta_Author_Presenter`.
fn meta_author(page: &Page<'_>, p: &Presentation, class: &str) -> String {
    if p.model.object_sub_type != "post" {
        return String::new();
    }
    let Some(user) = p.post().and_then(|post| page.users.get(&post.post_author)) else { return String::new() };
    tag(Format::Name, "author", php_trim(&schema::smart_strip_tags(&user.display_name)), class)
}

/// `Slack\Enhanced_Data_Presenter`.
async fn slack(page: &Page<'_>, p: &Presentation, class: &str) -> Result<String> {
    let mut data: Vec<(String, String)> = Vec::new();
    if p.model.object_sub_type == "post"
        && let Some(post) = p.post()
        && post.post_author != 0
    {
        data.push((page.site.trd(super::DOMAIN, "Written by"), replace_vars::author_meta(page, post.post_author, "display_name").await?));
    }
    if let Some(minutes) = p.reading_time.filter(|m| *m != 0) {
        let text = rustpress_core::i18n::sprintf(&page.site.trn("%s minute", "%s minutes", minutes), &[&minutes.to_string()]);
        data.push((page.site.trd(super::DOMAIN, "Est. reading time"), text));
    }
    let mut tags = String::new();
    for (i, (label, value)) in data.iter().enumerate() {
        tags.push_str(&format!("\t<meta name=\"twitter:label{}\" content=\"{}\"{class} />\n", i + 1, esc_attr(label)));
        tags.push_str(&format!("\t<meta name=\"twitter:data{}\" content=\"{}\"{class} />\n", i + 1, esc_attr(value)));
    }
    Ok(php_trim(&tags).to_owned())
}

/// `Schema_Presenter::present()`.
async fn schema_tag(page: &Page<'_>, p: &Presentation) -> Result<String> {
    let mut ctx = schema::context(page, p).await?;
    let graph = schema::graph(page, &mut ctx).await?;
    let flags = rustpress_blocks::php_json::Flags { unescaped_slashes: false, unescaped_unicode: true };
    // `WPSEO_Utils::is_development_mode()`: a site address without a dot is a development site.
    let json = if page.site.site_url("").contains('.') { rustpress_blocks::php_json::encode_with(&graph, flags) } else { rustpress_blocks::php_json::encode_pretty(&graph, flags) };
    Ok(format!("<script type=\"application/ld+json\" class=\"yoast-schema-graph\">{}</script>", json.replace('\n', "\n\t")))
}

/// `Open_Graph_Locale_Generator::generate()`.
pub fn locale(page: &Page<'_>) -> String {
    let locale = page.site.locale();
    let fixed = match locale.as_str() {
        "ca" => Some("ca_ES"),
        "en" => Some("en_US"),
        "el" => Some("el_GR"),
        "et" => Some("et_EE"),
        "ja" => Some("ja_JP"),
        "sq" => Some("sq_AL"),
        "uk" => Some("uk_UA"),
        "vi" => Some("vi_VN"),
        "zh" => Some("zh_CN"),
        _ => None,
    };
    if let Some(fixed) = fixed {
        return fixed.to_owned();
    }
    let locale = if locale.len() == 2 { format!("{}_{}", locale.to_lowercase(), locale.to_uppercase()) } else { locale };
    if FB_LOCALES.contains(&locale.as_str()) {
        return locale;
    }
    let two: String = locale.chars().take(2).collect();
    let guess = format!("{}_{}", two.to_lowercase(), two.to_uppercase());
    if FB_LOCALES.contains(&guess.as_str()) { guess } else { "en_US".to_owned() }
}

/// The locales Facebook knows (`$fb_valid_fb_locales`).
const FB_LOCALES: &[&str] = &[
    "af_ZA", "ak_GH", "am_ET", "ar_AR", "as_IN", "ay_BO", "az_AZ", "be_BY", "bg_BG", "bp_IN", "bn_IN", "br_FR", "bs_BA", "ca_ES", "cb_IQ", "ck_US", "co_FR", "cs_CZ", "cx_PH", "cy_GB", "da_DK", "de_DE", "el_GR",
    "en_GB", "en_PI", "en_UD", "en_US", "em_ZM", "eo_EO", "es_ES", "es_LA", "es_MX", "et_EE", "eu_ES", "fa_IR", "fb_LT", "ff_NG", "fi_FI", "fo_FO", "fr_CA", "fr_FR", "fy_NL", "ga_IE", "gl_ES", "gn_PY", "gu_IN",
    "gx_GR", "ha_NG", "he_IL", "hi_IN", "hr_HR", "hu_HU", "ht_HT", "hy_AM", "id_ID", "ig_NG", "is_IS", "it_IT", "ik_US", "iu_CA", "ja_JP", "ja_KS", "jv_ID", "ka_GE", "kk_KZ", "km_KH", "kn_IN", "ko_KR", "ks_IN",
    "ku_TR", "ky_KG", "la_VA", "lg_UG", "li_NL", "ln_CD", "lo_LA", "lt_LT", "lv_LV", "mg_MG", "mi_NZ", "mk_MK", "ml_IN", "mn_MN", "mr_IN", "ms_MY", "mt_MT", "my_MM", "nb_NO", "nd_ZW", "ne_NP", "nl_BE", "nl_NL",
    "nn_NO", "nr_ZA", "ns_ZA", "ny_MW", "om_ET", "or_IN", "pa_IN", "pl_PL", "ps_AF", "pt_BR", "pt_PT", "qc_GT", "qu_PE", "qr_GR", "qz_MM", "rm_CH", "ro_RO", "ru_RU", "rw_RW", "sa_IN", "sc_IT", "se_NO", "si_LK",
    "su_ID", "sk_SK", "sl_SI", "sn_ZW", "so_SO", "sq_AL", "sr_RS", "ss_SZ", "st_ZA", "sv_SE", "sw_KE", "sy_SY", "sz_PL", "ta_IN", "te_IN", "tg_TJ", "th_TH", "tk_TM", "tl_PH", "tl_ST", "tn_BW", "tr_TR", "ts_ZA",
    "tt_RU", "tz_MA", "uk_UA", "ur_PK", "uz_UZ", "ve_ZA", "vi_VN", "wo_SN", "xh_ZA", "yi_DE", "yo_NG", "zh_CN", "zh_HK", "zh_TW", "zu_ZA", "zz_TR",
];

/// `WP_Robots_Integration::add_robots()`: the plugin's directives merged into those of core.
pub fn wp_robots(core: &[(String, Robot)], yoast: &[(String, String)]) -> Vec<(String, Robot)> {
    let mut robots: Vec<(String, Option<Robot>)> = core.iter().map(|(k, v)| (k.clone(), Some(v.clone()))).collect();
    fn put(robots: &mut Vec<(String, Option<Robot>)>, key: &str, value: Option<Robot>) {
        match robots.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => robots.push((key.to_owned(), value)),
        }
    }
    // format_robots()
    let mut formatted: Vec<(String, Option<Robot>)> = yoast.iter().map(|(k, v)| (k.clone(), Some(Robot::Value(v.clone())))).collect();
    for (key, value) in yoast {
        if let Some(colon) = value.find(':') {
            put(&mut formatted, key, Some(Robot::Value(value[colon + 1..].to_owned())));
        } else if value.starts_with("no") {
            put(&mut formatted, key, Some(Robot::Flag(false)));
            put(&mut formatted, value, Some(Robot::Flag(true)));
        } else if key == value {
            put(&mut formatted, key, Some(Robot::Flag(true)));
        }
    }
    for (key, value) in formatted {
        put(&mut robots, &key, value);
    }
    // enforce_robots_congruence()
    let on = |robots: &Vec<(String, Option<Robot>)>, key: &str| robots.iter().any(|(k, v)| k == key && truthy(v));
    if on(&robots, "nofollow") {
        put(&mut robots, "follow", None);
    }
    if on(&robots, "noarchive") {
        put(&mut robots, "archive", None);
    }
    if on(&robots, "noimageindex") {
        put(&mut robots, "imageindex", None);
        if robots.iter().any(|(k, v)| k == "max-image-preview" && v.is_some()) {
            put(&mut robots, "max-image-preview", Some(Robot::Value("none".into())));
        }
    }
    if on(&robots, "nosnippet") {
        put(&mut robots, "snippet", None);
    }
    if on(&robots, "noindex") {
        for key in ["index", "imageindex", "noimageindex", "archive", "noarchive", "snippet", "nosnippet", "max-snippet", "max-image-preview", "max-video-preview"] {
            put(&mut robots, key, None);
        }
    }
    // sort_robots(): index, noindex, follow, nofollow, then the rest in their order.
    let rank = |k: &str| match k {
        "index" => 0,
        "noindex" => 1,
        "follow" => 2,
        "nofollow" => 3,
        _ => 4,
    };
    robots.sort_by_key(|(k, _)| rank(k));
    robots.into_iter().filter(|(_, v)| truthy(v)).map(|(k, v)| (k, v.unwrap_or(Robot::Flag(false)))).collect()
}

fn truthy(value: &Option<Robot>) -> bool {
    match value {
        Some(Robot::Flag(b)) => *b,
        Some(Robot::Value(v)) => !v.is_empty() && v != "0",
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag_of(robots: &[(String, Robot)]) -> String {
        rustpress_core::native::robots_tag(robots)
    }

    fn yoast(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
    }

    #[test]
    fn merges_robots_like_wp_robots_integration() {
        let core = vec![("max-image-preview".to_owned(), Robot::Value("large".into()))];
        let indexed = yoast(&[("index", "index"), ("follow", "follow"), ("max-snippet", "max-snippet:-1"), ("max-image-preview", "max-image-preview:large"), ("max-video-preview", "max-video-preview:-1")]);
        assert_eq!(tag_of(&wp_robots(&core, &indexed)), "<meta name='robots' content='index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1' />\n");
        // A search: core says `noindex, follow`, the plugin `noindex` without the previews.
        let core = vec![("noindex".to_owned(), Robot::Flag(true)), ("follow".to_owned(), Robot::Flag(true)), ("max-image-preview".to_owned(), Robot::Value("large".into()))];
        let search = yoast(&[("index", "noindex"), ("follow", "follow")]);
        assert_eq!(tag_of(&wp_robots(&core, &search)), "<meta name='robots' content='noindex, follow' />\n");
        // `noimageindex` turns the image preview off instead of dropping it.
        let core = vec![("max-image-preview".to_owned(), Robot::Value("large".into()))];
        let images = yoast(&[("index", "index"), ("follow", "nofollow"), ("max-snippet", "max-snippet:-1"), ("max-video-preview", "max-video-preview:-1"), ("imageindex", "noimageindex")]);
        assert_eq!(tag_of(&wp_robots(&core, &images)), "<meta name='robots' content='index, nofollow, max-image-preview:none, max-snippet:-1, max-video-preview:-1, noimageindex' />\n");
    }

    #[test]
    fn twitter_site_is_the_last_account_name() {
        assert_eq!(twitter_site("https://x.com/rpfirma"), "@rpfirma");
        assert_eq!(twitter_site("rpsite"), "@rpsite");
        assert_eq!(twitter_site(""), "");
    }
}
