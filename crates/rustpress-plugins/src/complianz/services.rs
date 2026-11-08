//! The service integrations of `integrations/services/*.php` and `integrations/statistics/*.php`: what each one adds
//! to the filters of the cookie blocker (`cmplz_known_script_tags`, `cmplz_known_style_tags`, `cmplz_image_tags`,
//! `cmplz_whitelisted_script_tags`, `cmplz_post_scribe_tags`), the inline scripts it attaches to
//! `cmplz-cookiebanner` and the CSS it prints in the footer.
//!
//! `cmplz_integrations()` loads the file of a service when `cmplz_uses_thirdparty( $service )`, going through the keys
//! of `thirdparty_service_markers` and then of `social_media_markers`; the filters run in that order.

use super::options::Options;

/// The keys of `cmplz_config::$thirdparty_service_markers` and their markers, in order.
pub const THIRDPARTY_MARKERS: &[(&str, &[&str])] = &[
    ("google-maps", &["apis.google.com/js/platform.js", "new google.maps.", "google.com/maps", "maps.google.com", "wp-google-maps", "new google.maps.InfoWindow", "new google.maps.Marker", "new google.maps.Map", "var mapOptions", "var map", "var Latlng"]),
    ("soundcloud", &["w.soundcloud.com/player"]),
    ("openstreetmaps", &["openstreetmap.org", "osm/js/osm"]),
    ("vimeo", &["player.vimeo.com", "i.vimeocdn.com"]),
    ("google-recaptcha", &["google.com/recaptcha", "grecaptcha", "recaptcha.js", "recaptcha/api"]),
    ("youtube", &["youtube.com", "youtube-nocookie.com", "youtu.be", "yotuwp"]),
    ("videopress", &["videopress.com/embed", "videopress.com/videopress-iframe.js"]),
    ("dailymotion", &["dailymotion.com/embed/video/"]),
    ("hotjar", &["static.hotjar.com"]),
    ("spotify", &["open.spotify.com/embed"]),
    ("google-fonts", &["fonts.googleapis.com"]),
    ("paypal", &["www.paypal.com/tagmanager/pptm.js", "www.paypalobjects.com/api/checkout.js"]),
    ("disqus", &["disqus.com"]),
    ("addthis", &["addthis.com"]),
    ("addtoany", &["static.addtoany.com/menu/page.js"]),
    ("sharethis", &["sharethis.com"]),
    ("microsoftads", &["bat.bing.com"]),
    ("livechat", &["cdn.livechatinc.com/tracking.js"]),
    ("hubspot", &["js.hs-scripts.com/", "hbspt.forms.create", "js.hsforms.net", "track.hubspot.com", "js.hs-analytics.net"]),
    ("calendly", &["assets.calendly.com"]),
    ("twitch", &["twitch.tv", "player.twitch.tv"]),
    ("adobe-fonts", &["p.typekit.net", "use.typekit.net"]),
];

/// `cmplz_config::$social_media_markers`.
pub const SOCIAL_MARKERS: &[(&str, &[&str])] = &[
    ("linkedin", &["platform.linkedin.com", "addthis_widget.js", "linkedin.com/embed/feed", "linkedin.com", "snap.licdn.com", "insight.min.js"]),
    ("twitter", &["super-socializer", "sumoSiteId", "addthis_widget.js", "platform.twitter.com", "platform.x.com", "twitter-widgets.js", "ads-twitter.com", "uwt.js"]),
    ("facebook", &["fbq", "super-socializer", "sumoSiteId", "addthis_widget.js", "fb-root", "<!-- Facebook Pixel Code -->", "connect.facebook.net", "www.facebook.com/plugins", "pixel-caffeine", "facebook.com/plugins"]),
    ("pinterest", &["super-socializer", "assets.pinterest.com"]),
    ("disqus", &["disqus.com"]),
    ("tiktok", &["tiktok.com"]),
    ("instagram", &["instawidget.net/js/instawidget.js", "instagram.com"]),
    ("snapchat", &["snapchat.com"]),
];

/// `cmplz_config::$stats_markers`.
pub const STATS_MARKERS: &[(&str, &[&str])] = &[
    ("google-analytics", &["google-analytics.com/ga.js", "www.google-analytics.com/analytics.js", "_getTracker", "gtag('js'"]),
    ("google-tag-manager", &["gtm.start", "gtm.js"]),
    ("matomo", &["piwik.js", "matomo.js"]),
    ("clicky", &["static.getclicky.com/js", "clicky_site_ids"]),
    ("yandex", &["mc.yandex.ru/metrika/watch.js"]),
    ("clarity", &["clarity.ms"]),
];

/// An entry of `cmplz_known_script_tags` / `cmplz_known_style_tags` after the defaults of
/// `cmplz_cookie_blocker::blocked_scripts()` (`wp_parse_args()` with `'general'`, `'marketing'`, placeholder on).
#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub placeholder: String,
    pub category: String,
    pub urls: Vec<String>,
    pub enable_placeholder: bool,
    pub placeholder_class: String,
    pub enable_dependency: bool,
    /// `'dependency'`: `[ 'wait-for-this' => 'script-that-waits' ]`.
    pub dependency: Vec<(String, String)>,
}

/// An entry as a service file writes it, before the defaults.
struct Entry {
    name: &'static str,
    placeholder: Option<&'static str>,
    category: &'static str,
    urls: &'static [&'static str],
    enable_placeholder: bool,
    placeholder_class: &'static str,
    dependency: &'static [(&'static str, &'static str)],
}

const fn entry(name: &'static str, placeholder: Option<&'static str>, category: &'static str, urls: &'static [&'static str]) -> Entry {
    Entry { name, placeholder, category, urls, enable_placeholder: true, placeholder_class: "", dependency: &[] }
}

impl Entry {
    const fn class(mut self, class: &'static str) -> Self {
        self.placeholder_class = class;
        self
    }

    const fn no_placeholder(mut self) -> Self {
        self.enable_placeholder = false;
        self
    }

    const fn waits(mut self, dependency: &'static [(&'static str, &'static str)]) -> Self {
        self.dependency = dependency;
        self
    }

    fn tag(&self) -> Tag {
        Tag {
            name: self.name.to_owned(),
            placeholder: self.placeholder.unwrap_or("").to_owned(),
            category: self.category.to_owned(),
            urls: self.urls.iter().map(|u| (*u).to_owned()).collect(),
            enable_placeholder: self.enable_placeholder,
            placeholder_class: self.placeholder_class.to_owned(),
            enable_dependency: !self.dependency.is_empty(),
            dependency: self.dependency.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
        }
    }
}

/// What one tag of `cmplz_known_script_tags` is: a full entry, or a bare URL that `blocked_scripts()` turns into
/// one with `cmplz_get_service_by_src()` as name and placeholder.
pub enum Known {
    Entry(Tag),
    Url(String),
}

/// The services whose integration file is loaded, in the order `cmplz_integrations()` requires them.
pub fn loaded(options: &Options) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for (service, _) in THIRDPARTY_MARKERS.iter().chain(SOCIAL_MARKERS) {
        if options.uses_thirdparty(service) && FILES.contains(service) && !out.contains(service) {
            out.push(service);
        }
    }
    out
}

/// The files of `integrations/services/`.
const FILES: &[&str] = &[
    "addthis", "adobe-fonts", "calendly", "dailymotion", "disqus", "facebook", "google-fonts", "google-maps", "google-recaptcha", "hotjar", "hubspot", "instagram", "linkedin", "livechat", "microsoftads", "openstreetmaps", "pinterest", "sharethis", "snapchat",
    "soundcloud", "spotify", "tiktok", "twitch", "twitter", "videopress", "vimeo", "youtube",
];

/// `apply_filters( 'cmplz_known_script_tags', array() )` of the loaded services and of the statistics integration.
/// `cf7` tells whether Contact Form 7 (5.4 or later) is active: `google-recaptcha` then adds nothing.
pub fn known_script_tags(options: &Options, cf7: bool) -> Vec<Known> {
    let mut tags = Vec::new();
    for service in loaded(options) {
        let entries: Vec<Entry> = match service {
            "addthis" => vec![entry("addthis", None, "marketing", &["addthis.com"])],
            "calendly" => vec![entry("calendly", Some("calendly"), "marketing", &["assets.calendly.com", "calendly.com"]).class("calendly-inline-widget")],
            "dailymotion" => vec![entry("dailymotion", Some("dailymotion"), "marketing", &["dailymotion.com/embed/video/", "geo.dailymotion.com/player.html", "dailymotion.com/video/"])],
            "disqus" => vec![entry("disqus", None, "marketing", &["disqus.com"])],
            "facebook" => vec![entry("facebook", Some("facebook"), "marketing", &["connect.facebook.net", "facebook.com/plugins"]).class("fb-page,fb-post")],
            "google-fonts" if options.is("self_host_google_fonts", "block") => {
                vec![entry("google-fonts", None, "marketing", &["fonts.googleapis.com", "ajax.googleapis.com/ajax/libs/webfont", "fonts.gstatic.com"]).no_placeholder().waits(&[("fonts.googleapis.com", "WebFont.load"), ("fonts.googleapis.", "WebFontConfig")])]
            }
            "google-maps" => vec![entry("google-maps", Some("google-maps"), "marketing", &["new google.maps.", "maps.google.com", "google.com/maps", "apis.google.com", "maps.google.de"])],
            "google-recaptcha" if !cf7 && options.is("block_recaptcha_service", "yes") => vec![entry(
                "google-recaptcha",
                Some("google-recaptcha"),
                "marketing",
                &["google.com/recaptcha", "grecaptcha", "recaptcha.js", "recaptcha/api", "apis.google.com/js/platform.js", "gstatic.com/recaptcha"],
            )
            .class("recaptcha-invisible,g-recaptcha")],
            "hotjar" => {
                if options.is("eu_consent_regions", "yes") || !options.is("hotjar_privacyfriendly", "yes") {
                    tags.push(Known::Url("static.hotjar.com".into()));
                }
                Vec::new()
            }
            "hubspot" => {
                tags.push(Known::Url("track.hubspot.com".into()));
                tags.push(Known::Url("js.hs-analytics.net".into()));
                Vec::new()
            }
            "instagram" => vec![entry("instagram", Some("instagram"), "marketing", &["instawidget.net/js/instawidget.js", "instagram.com"]).class("instagram-media")],
            "linkedin" => vec![entry("linkedin", Some("linkedin"), "marketing", &["platform.linkedin.com/in.js", "linkedin.com/embed/feed/update", "linkedin.com", "snap.licdn.com"]).class("share-update-card")],
            "livechat" => vec![entry("livechat", Some("livechat"), "marketing", &["cdn.livechatinc.com/tracking.js"])],
            "microsoftads" => {
                tags.push(Known::Url("bat.bing.com".into()));
                tags.push(Known::Url("bing.com".into()));
                Vec::new()
            }
            "openstreetmaps" => vec![entry("openstreetmaps", Some("openstreetmaps"), "marketing", &["openstreetmap.org"])],
            "pinterest" => vec![entry("pinterest", None, "marketing", &["assets.pinterest.com", "pinmarklet.js", "pinterest.com", "s.pinimg.com"])],
            "sharethis" => {
                tags.push(Known::Url("sharethis.com".into()));
                Vec::new()
            }
            "snapchat" => vec![entry("snapchat", Some("snapchat"), "marketing", &["snapchat.com", "snapchat.com/embed.js", "snapchat.com/spotlight"]).class("snapchat-embed")],
            "soundcloud" => vec![entry("soundcloud", Some("soundcloud"), "marketing", &["soundcloud.com/player"])],
            "spotify" => vec![entry("spotify", Some("spotify"), "marketing", &["open.spotify.com/embed"])],
            "tiktok" => vec![entry("tiktok", Some("tiktok"), "marketing", &["tiktok.com"]).class("tiktok-embed")],
            "twitch" => vec![entry("twitch", Some("twitch"), "marketing", &["player.twitch.tv", "twitch.tv", "new Twitch.Embed"]).class("twitch-embed").waits(&[("twitch.tv", "new Twitch.Embed")])],
            "twitter" => vec![entry("twitter", Some("twitter"), "marketing", &["platform.twitter.com", "platform.x.com", "twitter-widgets.js", "ads-twitter.com"]).class("twitter-tweet,twitter-timeline")],
            "videopress" => vec![entry("videopress", Some("videopress"), "marketing", &["videopress.com/videopress-iframe", "videopress.com/embed"])],
            "vimeo" => vec![entry("vimeo", Some("vimeo"), "statistics", &["vimeo.com", "i.vimeocdn.com"])],
            "youtube" => vec![entry("youtube", Some("youtube"), "marketing", &["www.youtube.com/iframe_api", "youtube.com", "youtube-nocookie.com", "youtu.be"])],
            _ => Vec::new(),
        };
        tags.extend(entries.iter().map(|e| Known::Entry(e.tag())));
    }
    let statistics = options.string("compile_statistics");
    if statistics == "google-analytics" {
        tags.push(Known::Entry(entry("google-analytics", None, "statistics", &["google-analytics.com/ga.js", "www.google-analytics.com/analytics.js", "www.googletagmanager.com/gtag/js", "_getTracker", "apis.google.com/js/platform.js"]).tag()));
    }
    if statistics == "matomo" && !options.is("configuration_by_complianz", "yes") {
        tags.push(Known::Entry(entry("matomo", None, "statistics", &["matomo.js", "piwik.js"]).tag()));
    }
    tags
}

/// `apply_filters( 'cmplz_known_style_tags', array() )`.
pub fn known_style_tags(options: &Options) -> Vec<Tag> {
    let mut tags = Vec::new();
    for service in loaded(options) {
        match service {
            "adobe-fonts" => tags.push(entry("adobe-fonts", None, "marketing", &["use.typekit.net", "p.typekit.net"]).no_placeholder().tag()),
            "google-fonts" if options.is("self_host_google_fonts", "block") => tags.push(entry("google-fonts", None, "marketing", &["fonts.googleapis.com", "fonts.gstatic.com"]).no_placeholder().tag()),
            _ => {}
        }
    }
    tags
}

/// `apply_filters( 'cmplz_image_tags', array() )`.
pub fn image_tags(options: &Options) -> Vec<String> {
    loaded(options).iter().filter(|s| **s == "google-maps").map(|_| "maps.googleapis.com/maps/api/staticmap".to_owned()).collect()
}

/// `apply_filters( 'cmplz_whitelisted_script_tags', array( 'user_banner_id' ) )`.
pub fn whitelisted_script_tags(options: &Options) -> Vec<String> {
    let mut tags = vec!["user_banner_id".to_owned()];
    if loaded(options).contains(&"vimeo") {
        tags.push("dnt=1".into());
        tags.push("dnt=true".into());
    }
    tags
}

/// `apply_filters( 'cmplz_post_scribe_tags', array() )`.
pub fn post_scribe_tags(options: &Options) -> Vec<String> {
    loaded(options).iter().filter(|s| **s == "instagram").map(|_| "instawidget.net/js/instawidget.js".to_owned()).collect()
}

/// The scripts the services add to `cmplz-cookiebanner` with `wp_add_inline_script()` on `wp_enqueue_scripts` at
/// `PHP_INT_MAX`: `cmplz_add_twitter_js()`, `cmplz_microsoftads_uet_consent()`, `cmplz_hubspot_clicker()`.
pub fn inline_scripts(options: &Options) -> Vec<String> {
    let mut out = Vec::new();
    for service in loaded(options) {
        match service {
            "twitter" => out.push(TWITTER_JS.to_owned()),
            "microsoftads" if options.in_list("thirdparty_services_on_site", "microsoftads") => out.push(MICROSOFTADS_JS.to_owned()),
            "hubspot" if options.is("block_hubspot_service", "yes") => out.push(HUBSPOT_JS.to_owned()),
            _ => {}
        }
    }
    out
}

/// What the services print on `wp_footer` (after the banner): `cmplz_hubspot_css()` and `cmplz_snapchat_css()`.
pub fn footer_css(options: &Options) -> String {
    let mut out = String::new();
    for service in loaded(options) {
        match service {
            "hubspot" if options.is("block_hubspot_service", "yes") => out.push_str(HUBSPOT_CSS),
            "snapchat" => out.push_str(SNAPCHAT_CSS),
            _ => {}
        }
    }
    out
}

const TWITTER_JS: &str = "\n\tlet cmplzBlockedContent = document.querySelector('.cmplz-blocked-content-notice');\n\tif ( cmplzBlockedContent) {\n\t        cmplzBlockedContent.addEventListener('click', function(event) {\n            event.stopPropagation();\n        });\n\t}\n    ";

const MICROSOFTADS_JS: &str = "\n\t\twindow.uetq = window.uetq || [];\n\t\twindow.uetq.push('consent', 'default', {\n\t\t\t'ad_storage': 'denied'\n\t\t});\n\n\t\tdocument.addEventListener('cmplz_fire_categories', function(e) {\n\t\t\tvar consentedCategories = e.detail.categories;\n\t\t\tlet marketing = 'denied';\n\t\t\tif (cmplz_in_array('marketing', consentedCategories)) {\n\t\t\t\tmarketing = 'granted';\n\t\t\t}\n\t\t\twindow.uetq.push('consent', 'update', {\n\t\t\t\t'ad_storage': marketing\n\t\t\t});\n\t\t});\n\n\t\tdocument.addEventListener('cmplz_revoke', function(e) {\n\t\t\twindow.uetq.push('consent', 'update', {\n\t\t\t\t'ad_storage': 'denied'\n\t\t\t});\n\t\t});\n\t";

const HUBSPOT_JS: &str = "\t\t\n\t\t\tdocument.addEventListener(\"cmplz_enable_category\", cmplzHubspotScriptHandler);\n\t\t\tdocument.addEventListener(\"cmplz_status_change_service\", cmplzHubspotScriptHandler);\n\t\t\tdocument.addEventListener(\"cmplz_status_change\", cmplzHubspotScriptHandler);\n\t\t\tfunction cmplzHubspotScriptHandler(consentData) {\n\t\t\t\tlet hubspotAcceptBtn = document.getElementById(\"hs-eu-confirmation-button\");\n\t\t\t\tlet hubspotDeclinetBtn = document.getElementById(\"hs-eu-decline-button\");\n\t\t\t\tif ( consentData.detail.category === 'marketing' ) {\n\t\t\t\t\tif ( hubspotAcceptBtn != null ) {\n\t\t\t\t\t\thubspotAcceptBtn.click();\n\t\t\t\t\t}\n\t\t\t\t} else {\n\t\t\t\t\tif ( hubspotDeclinetBtn != null && !consentData.detail.categories.includes(\"marketing\") ) {\n\t\t\t\t\t\thubspotDeclinetBtn.click();\n\t\t\t\t\t}\n\t\t\t\t}\n\t\t\t\t// if ( hubspotAcceptBtn ) {\n\t\t\t\t// \thubspotAcceptBtn.parentNode.removeChild(hubspotAcceptBtn);\n\t\t\t\t// }\n\t\t\t}\n\t\t\n\t\t";

const HUBSPOT_CSS: &str = "\t\t<style>\n\t\t\tdiv#hs-eu-cookie-confirmation {display: none;}\n\t\t</style>\n\t";

const SNAPCHAT_CSS: &str = "\t\t<style>    \n\t\t.snapchat-embed > div  {\n\t\t\tdisplay: none !important;\n\t\t}\n\t\t</style>   \n\t";
