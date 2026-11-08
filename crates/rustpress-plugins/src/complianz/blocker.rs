//! `cmplz_cookie_blocker`: the output buffer that blocks third party content until consent (`replace_tags()`), and the
//! lists it works with (`blocked_scripts()`, `blocked_styles()`, `placeholder_markers()`, `dependencies()`,
//! `whitelisted_scripts()`), plus the placeholders of `cmplz_placeholder()`.
//!
//! The patterns are PCRE patterns without `/u`; they are ported for UTF-8 text, where they match the same (the class
//! of `replace_src()`, which lists accented letters byte by byte, is matched on bytes).

use std::sync::OnceLock;

use regex::Regex;
use rustpress_phpser::Value;

use super::options::{item, php_string, truthy, Options};
use super::services::{self, Known, Tag, SOCIAL_MARKERS, STATS_MARKERS, THIRDPARTY_MARKERS};

/// `esc_attr()`.
fn esc_attr(text: &str) -> String {
    rustpress_core::formatting::esc_attr(text)
}

/// `cmplz_strpos_arr( $haystack, $needles )` of a list (numeric keys): the first needle that occurs.
pub fn strpos_arr<'n>(haystack: &str, needles: impl IntoIterator<Item = &'n str>) -> Option<&'n str> {
    if haystack.is_empty() || haystack == "0" {
        return None;
    }
    needles.into_iter().find(|n| !n.is_empty() && haystack.contains(*n))
}

/// `cmplz_strpos_arr()` of an array with string keys (the dependencies): the key of the first value that occurs.
fn strpos_arr_keyed<'n>(haystack: &str, needles: &'n [(String, String)]) -> Option<&'n str> {
    if haystack.is_empty() || haystack == "0" {
        return None;
    }
    needles.iter().find(|(_, v)| !v.is_empty() && haystack.contains(v.as_str())).map(|(k, _)| k.as_str())
}

/// `cmplz_sanitize_title_preserve_uppercase()` (`sanitize_service_name()` with `'general'` for an empty name).
pub fn sanitize_service_name(title: &str) -> String {
    if title.is_empty() || title == "0" {
        return "general".into();
    }
    static ENTITY: OnceLock<Regex> = OnceLock::new();
    static OTHER: OnceLock<Regex> = OnceLock::new();
    static SPACE: OnceLock<Regex> = OnceLock::new();
    static DASHES: OnceLock<Regex> = OnceLock::new();
    let title = ENTITY.get_or_init(|| Regex::new("&.+?;").expect("valid regex")).replace_all(title, "");
    let title = title.replace('.', "-");
    let title = OTHER.get_or_init(|| Regex::new("[^%a-zA-Z0-9 _-]").expect("valid regex")).replace_all(&title, "");
    let title = SPACE.get_or_init(|| Regex::new(r"[\t\n\x0B\x0C\r ]+").expect("valid regex")).replace_all(&title, "-");
    let title = DASHES.get_or_init(|| Regex::new("-+").expect("valid regex")).replace_all(&title, "-");
    // remove_accents() has nothing left to change; sanitize_text_field() drops percent-encoded octets and trims.
    static OCTET: OnceLock<Regex> = OnceLock::new();
    let octet = OCTET.get_or_init(|| Regex::new("(?i)%[a-f0-9]{2}").expect("valid regex"));
    let mut title = title.into_owned();
    while let Some(m) = octet.find(&title) {
        let found = m.as_str().to_owned();
        title = title.replace(&found, "");
    }
    title.trim().replace(' ', "-")
}

/// The lists `replace_tags()` works with, built once per snapshot of the site.
pub struct Blocker {
    /// `blocked_styles()`: URL => tag, in order.
    styles: Vec<(String, Tag)>,
    /// `blocked_scripts()` (or the transient `cmplz_blocked_scripts`): URL => tag, in order.
    pub scripts: Vec<(String, Tag)>,
    /// `placeholder_markers()`.
    markers: Vec<Tag>,
    /// `whitelisted_scripts()`.
    whitelist: Vec<String>,
    /// `dependencies()`: wait for key => script that waits.
    dependencies: Vec<(String, String)>,
    post_scribe: Vec<String>,
    image_tags: Vec<String>,
    pub(super) placeholders: Placeholders,
    consent_per_service: bool,
    youtube_or_vimeo: bool,
}

/// What `cmplz_placeholder()` and `cmplz_use_placeholder()` read.
pub struct Placeholders {
    /// `CMPLZ_URL . 'assets/images/placeholders/'` and its directory.
    pub url: String,
    pub dir: std::path::PathBuf,
    /// The stylesheet directory of the theme and its URL (`complianz-gdpr/{img}` overrides).
    pub theme_dir: std::path::PathBuf,
    pub theme_url: String,
    /// `cmplz_upload_url()` and `cmplz_upload_dir()`.
    pub upload_url: String,
    pub upload_dir: String,
    pub style: String,
    pub maps_format: String,
    pub dont_use: bool,
    /// `get_option( 'cmplz_disabled_placeholders' )`.
    pub disabled: Vec<String>,
    /// The services whose integration is loaded (their `cmplz_placeholder_{service}` filters).
    pub loaded: Vec<&'static str>,
    /// `cmplz_get_transient()` of the cached placeholder images (`cmplz_youtube_image_{id}`, ...).
    pub transients: Vec<(String, String)>,
}

impl Placeholders {
    fn transient(&self, name: &str) -> Option<&str> {
        self.transients.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()).filter(|v| !v.is_empty() && *v != "0")
    }

    /// `cmplz_file_exists_on_url()`.
    fn file_exists_on_url(&self, url: &str) -> bool {
        std::path::Path::new(&url.replace(&self.upload_url, &self.upload_dir)).exists()
    }

    /// `cmplz_default_placeholder( $type )`.
    pub fn default(&self, kind: &str) -> String {
        let kind = if kind == "openstreetmaps" { "google-maps" } else { kind };
        let style = &self.style;
        let ratio = if kind == "google-maps" { self.maps_format.as_str() } else { "" };
        let img = [format!("{kind}-{style}-{ratio}.jpg"), format!("{kind}-{style}.jpg")].into_iter().find(|img| self.dir.join(img).exists()).unwrap_or_else(|| format!("default-{style}.jpg"));
        if self.theme_dir.join("complianz-gdpr").join(&img).exists() {
            return format!("{}/complianz-gdpr/{img}", self.theme_url.trim_end_matches('/'));
        }
        format!("{}{img}", self.url)
    }

    /// `cmplz_placeholder( $type, $src )`.
    pub fn placeholder(&self, kind: Option<&str>, src: &str) -> String {
        let kind = match kind.filter(|k| !k.is_empty() && *k != "0") {
            Some(kind) => kind.to_owned(),
            None => service_by_src(src),
        };
        let new_src = self.default(&kind);
        if !self.loaded.contains(&kind.as_str()) {
            return new_src;
        }
        match kind.as_str() {
            "youtube" => self.youtube(new_src, src),
            "vimeo" => self.vimeo(new_src, src),
            "dailymotion" => self.dailymotion(new_src, src),
            "google-maps" => self.google_maps(new_src, src),
            _ => new_src,
        }
    }

    /// A cached image, or what `cmplz_download_to_site()` falls back to: RustPress does not download placeholders.
    fn cached(&self, name: &str) -> String {
        match self.transient(name) {
            Some(url) if self.file_exists_on_url(url) => url.to_owned(),
            _ => self.default("default"),
        }
    }

    /// `cmplz_youtube_placeholder()`.
    fn youtube(&self, new_src: String, src: &str) -> String {
        static ID: OnceLock<Regex> = OnceLock::new();
        static SERIES: OnceLock<Regex> = OnceLock::new();
        let Some(caps) = ID.get_or_init(|| Regex::new(r"(?i).*(?:youtu.be/|v/|u/[A-Za-z0-9_]/|embed/videoseries\?list=RD|embed/|watch\?v=)([^#&?]*).*").expect("valid regex")).captures(src) else {
            return new_src;
        };
        let mut id = caps[1].to_owned();
        if id == "videoseries" {
            // The first video of a series is looked up online; only a cached answer is known here.
            id = SERIES
                .get_or_init(|| Regex::new(r"(?i).*(?:youtu.be/|v/|u/[A-Za-z0-9_]/|embed/videoseries\?list=RD|embed/|watch\?v=)[^#&?]*\?list=(.*)").expect("valid regex"))
                .captures(src)
                .and_then(|c| self.transient(&format!("cmplz_youtube_videoseries_video_id_{}", &c[1])).map(str::to_owned))
                .unwrap_or_default();
        }
        self.cached(&format!("cmplz_youtube_image_{id}"))
    }

    /// `cmplz_vimeo_placeholder()`: a failed lookup keeps the placeholder of the type.
    fn vimeo(&self, placeholder_src: String, src: &str) -> String {
        static ID: OnceLock<Regex> = OnceLock::new();
        let re = ID.get_or_init(|| Regex::new(r"(?i)(?:www\.|player\.)?vimeo.com/(?:channels/(?:[A-Za-z0-9_]+/)?|groups/(?:[^/]*)/videos/|album/(?:[0-9]+)/video/|video/|)([0-9]+)(?:[a-zA-Z0-9_\-]+)?").expect("valid regex"));
        let Some(caps) = re.captures(src) else { return placeholder_src };
        match self.transient(&format!("cmplz_vimeo_image_{}", &caps[1])) {
            Some(url) if url != "failed" && self.file_exists_on_url(url) => url.to_owned(),
            _ => placeholder_src,
        }
    }

    /// `cmplz_dailymotion_placeholder()`.
    fn dailymotion(&self, new_src: String, src: &str) -> String {
        static ID: OnceLock<Regex> = OnceLock::new();
        let re = ID.get_or_init(|| {
            Regex::new(r"(?i)dailymotion\.com/(embed/video)/([^_]+)[^#]*\?|dailymotion\.com/(embed/video|video|hub)/([^_]+)[^#]*(#video=([^_&]+))?|(dai\.ly/([^_]+))!").expect("valid regex")
        });
        let Some(caps) = re.captures(src) else { return new_src };
        // `preg_match()` leaves out unmatched groups at the end only.
        let last = (1..caps.len()).rev().find(|i| caps.get(*i).is_some()).unwrap_or(0);
        let group = |i: usize| caps.get(i).map_or("", |m| m.as_str());
        let id = if last >= 6 {
            group(6)
        } else if last >= 4 {
            group(4)
        } else {
            group(2)
        };
        self.cached(&format!("cmplz_dailymotion_image_{id}"))
    }

    /// `cmplz_google_maps_placeholder()`: only static maps images have an image of their own.
    fn google_maps(&self, new_src: String, src: &str) -> String {
        if !src.to_ascii_lowercase().contains("maps.googleapis.com/maps/api/staticmap") {
            return new_src;
        }
        let id = src.replace("http://", "").replace("https://", "").replace("maps.googleapis.com/maps/api/staticmap", "");
        self.cached(&format!("cmplz_googlemaps_image_{}", rustpress_core::slug::sanitize_title(&id)))
    }

    /// `cmplz_placeholder_disabled( $service )`.
    fn disabled(&self, service: &str) -> bool {
        self.disabled.iter().any(|d| d == service || *d == service.replace('-', "_"))
    }

    /// `cmplz_use_placeholder( $src )`.
    pub fn use_placeholder(&self, src: Option<&str>) -> bool {
        if self.dont_use {
            return false;
        }
        let Some(src) = src.filter(|s| !s.is_empty() && *s != "0") else { return true };
        if src.contains("like.php") {
            return false;
        }
        !self.disabled(&service_by_src(src))
    }
}

/// `cmplz_get_service_by_src( $src )`.
pub fn service_by_src(src: &str) -> String {
    let mut seen: Vec<&str> = Vec::new();
    for (service, markers) in THIRDPARTY_MARKERS.iter().chain(SOCIAL_MARKERS).chain(STATS_MARKERS) {
        // `$services + $social + $stats` keeps the first array's entry of a key.
        if seen.contains(service) {
            continue;
        }
        seen.push(service);
        if strpos_arr(src, markers.iter().copied()).is_some() {
            return (*service).to_owned();
        }
    }
    "general".into()
}

/// A tag of the transient `cmplz_blocked_scripts` (a list `blocked_scripts()` stored earlier).
fn tag_from_value(value: &Value) -> Tag {
    let s = |key: &str| item(value, key).map(php_string).unwrap_or_default();
    let flag = |key: &str| item(value, key).is_some_and(|v| super::options::loose_eq_int(v, 1));
    let dependency = match item(value, "dependency") {
        Some(Value::Array(items)) => items.iter().map(|(k, v)| (key_string(k), php_string(v))).collect(),
        _ => Vec::new(),
    };
    Tag {
        name: s("name"),
        placeholder: s("placeholder"),
        category: s("category"),
        urls: super::options::strings(item(value, "urls").unwrap_or(&Value::Null)),
        enable_placeholder: flag("enable_placeholder"),
        placeholder_class: s("placeholder_class"),
        enable_dependency: flag("enable_dependency"),
        dependency,
    }
}

fn key_string(key: &rustpress_phpser::Key) -> String {
    match key {
        rustpress_phpser::Key::Int(n) => n.to_string(),
        rustpress_phpser::Key::Str(b) => String::from_utf8_lossy(b).into_owned(),
    }
}

/// `$array[ $key ] = $value` of a PHP array: an existing key keeps its place.
fn set(list: &mut Vec<(String, Tag)>, key: String, tag: Tag) {
    match list.iter_mut().find(|(k, _)| *k == key) {
        Some((_, slot)) => *slot = tag,
        None => list.push((key, tag)),
    }
}

/// The enabled entries of a list of the Script Center (`complianz_options_custom-scripts`).
fn custom_scripts(custom: &Value, list: &str) -> Vec<Value> {
    match item(custom, list) {
        Some(Value::Array(items)) => items.iter().map(|(_, v)| v.clone()).filter(|v| item(v, "enable").is_some_and(truthy)).collect(),
        _ => Vec::new(),
    }
}

impl Blocker {
    /// `custom` is `get_option( 'complianz_options_custom-scripts' )`, `transient` the value of
    /// `cmplz_get_transient( 'cmplz_blocked_scripts' )`, `cf7` whether Contact Form 7 5.4+ is active.
    pub fn new(options: &Options, custom: &Value, transient: Option<&Value>, cf7: bool, placeholders: Placeholders) -> Self {
        let styles = blocked(services::known_style_tags(options).into_iter().map(Known::Entry).collect(), &placeholders, false);
        let scripts = match transient.filter(|v| truthy(v)) {
            Some(Value::Array(items)) => items.iter().map(|(k, v)| (key_string(k), tag_from_value(v))).collect(),
            _ => {
                let mut known = services::known_script_tags(options, cf7);
                known.extend(custom_scripts(custom, "block_script").iter().map(|v| Known::Entry(tag_from_value(v))));
                blocked(known, &placeholders, true)
            }
        };
        // placeholder_markers(): the `cmplz_placeholder_markers` filter is empty; the Script Center's `add_script`
        // entries come next, and only entries with a placeholder class stay.
        let mut markers: Vec<Tag> = scripts.iter().map(|(_, t)| t.clone()).collect();
        markers.extend(custom_scripts(custom, "add_script").iter().map(tag_from_value));
        markers.retain(|t| t.enable_placeholder && !t.placeholder_class.is_empty() && t.placeholder_class != "0");
        let mut dependencies: Vec<(String, String)> = Vec::new();
        let mut waiting: Vec<Tag> = scripts.iter().map(|(_, t)| t.clone()).collect();
        waiting.extend(custom_scripts(custom, "block_script").iter().map(tag_from_value));
        for tag in waiting.iter().filter(|t| t.enable_dependency && !t.dependency.is_empty()) {
            for (k, v) in &tag.dependency {
                // array_merge(): string keys are overwritten in place, numeric keys appended.
                match dependencies.iter_mut().find(|(key, _)| key == k && k.parse::<i64>().is_err()) {
                    Some((_, slot)) => *slot = v.clone(),
                    None => dependencies.push((k.clone(), v.clone())),
                }
            }
        }
        let mut whitelist = Vec::new();
        for entry in custom_scripts(custom, "whitelist_script") {
            whitelist.extend(super::options::strings(item(&entry, "urls").unwrap_or(&Value::Null)));
        }
        whitelist.extend(services::whitelisted_script_tags(options));
        Self {
            styles,
            scripts,
            markers,
            whitelist,
            dependencies,
            post_scribe: services::post_scribe_tags(options),
            image_tags: services::image_tags(options),
            placeholders,
            consent_per_service: options.is("consent_per_service", "yes"),
            youtube_or_vimeo: options.uses_thirdparty("youtube") || options.uses_thirdparty("vimeo"),
        }
    }

    /// `cmplz_cookie_blocker::replace_tags( $output )`. `blocked_content_text` is `blocked_content_text()`.
    pub fn replace_tags(&self, mut output: String, blocked_content_text: &str) -> String {
        self.images(&mut output);
        self.styles(&mut output);
        self.iframes(&mut output);
        if self.youtube_or_vimeo {
            self.videos(&mut output);
        }
        if self.placeholders.use_placeholder(None) {
            self.markers(&mut output);
        }
        self.scripts(&mut output, blocked_content_text);
        let id = if self.consent_per_service { 2 } else { 1 };
        output.replace("<body ", &format!("<body data-cmplz={id} "))
    }

    fn images(&self, output: &mut String) {
        static IMAGE: OnceLock<Regex> = OnceLock::new();
        let re = IMAGE.get_or_init(|| Regex::new(r#"(?s)<img.*?src=['|"]([\s\S]*?)['|"].*?>"#).expect("valid regex"));
        let matches: Vec<(String, String)> = re.captures_iter(output).map(|c| (c[0].to_owned(), c[1].to_owned())).collect();
        for (total, url) in matches {
            if strpos_arr(&url, self.image_tags.iter().map(String::as_str)).is_none() {
                continue;
            }
            let placeholder = self.placeholders.placeholder(None, &url);
            let service = service_by_src(&url);
            let mut new = add_data(&total, "img", "src-cmplz", &url);
            new = add_data(&new, "img", "service", &service);
            new = add_data(&new, "img", "category", "marketing");
            new = new.replace("loading=\"lazy\"", "data-deferlazy=\"1\"");
            new = add_class(&new, "img", "cmplz-image");
            new = replace_src(&new, &placeholder);
            if self.placeholders.use_placeholder(Some(&url)) {
                new = add_class(&new, "img", " cmplz-placeholder-element ");
                new = format!("<div class=\"cmplz-placeholder-parent\">{new}</div>");
            }
            *output = output.replace(&total, &new);
        }
    }

    fn styles(&self, output: &mut String) {
        static STYLE: OnceLock<Regex> = OnceLock::new();
        let re = STYLE.get_or_init(|| Regex::new(r#"(?i)<link.*?rel=['|"]stylesheet['|"][^>].*?href=['|"]([\s\S]*?)['|"][^>]*?>"#).expect("valid regex"));
        let matches: Vec<(String, String)> = re.captures_iter(output).map(|c| (c[0].to_owned(), c[1].to_owned())).collect();
        for (total, url) in matches {
            if total.contains("data-category=\"functional\"") || strpos_arr(&total, self.whitelist.iter().map(String::as_str)).is_some() {
                continue;
            }
            let Some(found) = strpos_arr(&url, self.styles.iter().map(|(k, _)| k.as_str())) else { continue };
            let tag = &self.styles.iter().find(|(k, _)| k == found).expect("found among the keys").1;
            let mut new = add_data(&total, "link", "category", &tag.category);
            new = add_data(&new, "link", "service", &sanitize_service_name(&tag.name));
            new = new.replace("href=", "data-href=");
            *output = output.replace(&total, &new);
        }
    }

    fn tag_for(&self, haystack: &str) -> Option<&Tag> {
        let found = strpos_arr(haystack, self.scripts.iter().map(|(k, _)| k.as_str()))?;
        self.scripts.iter().find(|(k, _)| k == found).map(|(_, t)| t)
    }

    fn iframes(&self, output: &mut String) {
        static IFRAME: OnceLock<Regex> = OnceLock::new();
        let re = IFRAME.get_or_init(|| Regex::new(r#"(?is)<(iframe)[^>].*?src=['"](.*?)['"].*?>.*?</iframe>"#).expect("valid regex"));
        static OPEN: OnceLock<Regex> = OnceLock::new();
        let open = OPEN.get_or_init(|| Regex::new(r"(?i)<iframe[\t\n\x0B\x0C\r ]").expect("valid regex"));
        let matches: Vec<(String, String)> = re.captures_iter(output).map(|c| (c[0].to_owned(), c[2].to_owned())).collect();
        for (total, src) in matches {
            let Some(tag) = self.tag_for(&src) else { continue };
            if tag.category == "functional" {
                continue;
            }
            let is_video = ["dailymotion", "youtube", "vimeo"].iter().any(|v| src.contains(v));
            let service = sanitize_service_name(&tag.name);
            let mut new = open.replacen(&total, 1, regex::NoExpand(&format!("<iframe data-cmplz-target=\"src\" data-src-cmplz=\"{src}\" "))).into_owned();
            new = new.replace("loading=\"lazy\"", "data-deferlazy=\"1\"");
            if strpos_arr(&total, self.whitelist.iter().map(String::as_str)).is_some() {
                continue;
            }
            let video_class = if is_video { "cmplz-video" } else { "cmplz-no-video" };
            new = replace_src(&new, "about:blank");
            new = add_class(&new, "iframe", &format!("cmplz-iframe cmplz-iframe-styles {video_class} "));
            new = add_data(&new, "iframe", "service", &service);
            new = add_data(&new, "iframe", "category", &tag.category);
            if self.placeholders.use_placeholder(Some(&src)) {
                let placeholder = self.placeholders.placeholder(Some(&tag.placeholder), &src);
                new = add_class(&new, "iframe", "cmplz-placeholder-element");
                new = add_data(&new, "iframe", "placeholder-image", &placeholder);
                if !is_video && !src.contains("soundcloud") {
                    new = format!("<div class=\"cmplz-placeholder-parent\">{new}</div>");
                }
            }
            *output = output.replace(&total, &new);
        }
    }

    fn videos(&self, output: &mut String) {
        static VIDEO: OnceLock<Regex> = OnceLock::new();
        static CLASS: OnceLock<Regex> = OnceLock::new();
        let re = VIDEO.get_or_init(|| Regex::new(r#"(?is)<video class="wp-video-shortcode".*?<(source) type="video.*?src="(.*?)".*?>.*?</video>"#).expect("valid regex"));
        let class = CLASS.get_or_init(|| Regex::new(r#"(?is)(["| ])(wp-video)(["| ])"#).expect("valid regex"));
        let matches: Vec<(String, String)> = re.captures_iter(output).map(|c| (c[0].to_owned(), c[2].to_owned())).collect();
        for (total, src) in matches {
            let Some(tag) = self.tag_for(&src) else { continue };
            if tag.category == "functional" || strpos_arr(&total, self.whitelist.iter().map(String::as_str)).is_some() {
                continue;
            }
            let service = rustpress_core::slug::sanitize_title(&tag.name);
            *output = class.replace_all(output, "${1}wp-video cmplz-wp-video${3}").into_owned();
            let mut new = add_class(&total, "video", " cmplz-video ");
            new = add_data(&new, "video", "service", &service);
            new = add_data(&new, "video", "category", &tag.category);
            new = new.replace("wp-video-shortcode", "cmplz-wp-video-shortcode").replace("controls=\"controls\"", "");
            if self.placeholders.use_placeholder(Some(&src)) {
                let placeholder = self.placeholders.placeholder(Some(&tag.placeholder), &src);
                new = add_class(&new, "video", "cmplz-placeholder-element");
                new = add_data(&new, "video", "placeholder-image", &placeholder);
            }
            *output = output.replace(&total, &new);
        }
    }

    fn markers(&self, output: &mut String) {
        for marker in &self.markers {
            for class in marker.placeholder_class.split(',').map(str::trim) {
                let Ok(re) = fancy_regex::Regex::new(&format!(r#"(?is)<([a-zA-Z][a-zA-Z0-9-]*)[^>]*(id|class)=['" ]*[^>]*({class})['" ].*?>"#)) else { continue };
                let matches: Vec<(String, String)> = re.captures_iter(output.as_str()).filter_map(Result::ok).map(|c| (c[0].to_owned(), c[1].to_owned())).collect();
                for (html, el) in matches {
                    let image = self.placeholders.placeholder(Some(&marker.placeholder), class);
                    let mut new = add_data(&html, &el, "placeholder-image", &image);
                    new = add_data(&new, &el, "category", &marker.category);
                    new = add_data(&new, &el, "service", &marker.name);
                    new = add_class(&new, &el, "cmplz-placeholder-element");
                    *output = output.replace(&html, &new);
                }
            }
        }
    }

    fn scripts(&self, output: &mut String, blocked_content_text: &str) {
        static SCRIPT: OnceLock<Regex> = OnceLock::new();
        static SRC: OnceLock<Regex> = OnceLock::new();
        let re = SCRIPT.get_or_init(|| Regex::new(r"(?is)(<script.*?>)([\s\S]*?)</script>").expect("valid regex"));
        let src_re = SRC.get_or_init(|| Regex::new(r#"(?is)<script[^>]*?src=['"]([A-Za-z0-9_.,;ß@?^=%&:()/~+#!\-*]*?)['"].*?>"#).expect("valid regex"));
        let matches: Vec<(String, String, String)> = re.captures_iter(output).map(|c| (c[0].to_owned(), c[1].to_owned(), c[2].to_owned())).collect();
        let mut index = 0;
        for (total, open, content) in matches {
            if open.contains("data-category=\"functional\"") || open.contains("application/ld+json") {
                continue;
            }
            if strpos_arr(&total, self.whitelist.iter().map(String::as_str)).is_some() {
                continue;
            }
            if !content.is_empty() && content != "0" {
                if content.contains("avia_preview") {
                    continue;
                }
                if let Some(tag) = self.tag_for(&content)
                    && tag.category != "functional" {
                        let mut new = add_data(&total, "script", "category", &tag.category);
                        new = add_data(&new, "script", "service", &sanitize_service_name(&tag.name));
                        new = set_javascript_to_plain(&new);
                        if let Some(waitfor) = strpos_arr_keyed(&content, &self.dependencies) {
                            new = add_data(&new, "script", "waitfor", waitfor);
                        }
                        *output = output.replace(&total, &new);
                    }
            }
            let sources: Vec<String> = src_re.captures_iter(&total).map(|c| c[1].to_owned()).collect();
            for src in sources {
                let Some(found) = strpos_arr(&src, self.scripts.iter().map(|(k, _)| k.as_str())) else { continue };
                let tag = &self.scripts.iter().find(|(k, _)| k == found).expect("found among the keys").1;
                let mut new = add_data(&total, "script", "category", &tag.category);
                new = add_data(&new, "script", "service", &sanitize_service_name(&tag.name));
                if !new.contains("data-category=\"functional\"") {
                    new = set_javascript_to_plain(&new);
                    new = new.replace("src=", "data-cmplz-src=");
                    if strpos_arr(found, self.post_scribe.iter().map(String::as_str)).is_some() {
                        // The plugin also sets the option `cmplz_post_scribe_required` here, which RustPress does not write.
                        index += 1;
                        new = add_data(&new, "script", "post_scribe_id", &format!("cmplz-ps-{index}"));
                        new.push_str(&format!("<div class=\"cmplz-blocked-content-container\">{blocked_content_text}<div id=\"cmplz-ps-{index}\"><img src=\"{}\"></div></div>", self.placeholders.placeholder(Some("div"), "")));
                    }
                    if let Some(waitfor) = strpos_arr_keyed(&src, &self.dependencies) {
                        new = add_data(&new, "script", "waitfor", waitfor);
                    }
                }
                *output = output.replace(&total, &new);
            }
        }
    }

    /// Whether the list of blocked scripts is not empty (`site_shares_data()`).
    pub fn has_scripts(&self) -> bool {
        !self.scripts.is_empty()
    }
}

/// `blocked_scripts()` / `blocked_styles()`: the defaults of each entry, then one entry per URL.
fn blocked(known: Vec<Known>, placeholders: &Placeholders, scripts: bool) -> Vec<(String, Tag)> {
    let mut out = Vec::new();
    for known in known {
        let mut tag = match known {
            Known::Entry(tag) => tag,
            Known::Url(url) => {
                let service = service_by_src(&url);
                Tag { name: service.clone(), placeholder: service, category: "marketing".into(), urls: vec![url], enable_placeholder: true, placeholder_class: String::new(), enable_dependency: false, dependency: Vec::new() }
            }
        };
        if tag.category.is_empty() {
            tag.category = "marketing".into();
        }
        tag.name = sanitize_service_name(&tag.name);
        if scripts && placeholders.disabled(&tag.name) {
            tag.enable_placeholder = false;
        }
        for url in tag.urls.clone() {
            set(&mut out, url, tag.clone());
        }
    }
    out
}

/// `preg_quote( $text, '/' )`.
fn preg_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if ".\\+*?[^]$(){}=!<>|:-#/".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `cmplz_cookie_blocker::add_data( $html, $el, $id, $content )`.
pub fn add_data(html: &str, el: &str, id: &str, content: &str) -> String {
    let content = esc_attr(content);
    let id = esc_attr(id);
    let pattern = format!(r#"(?i)<{}[^>].*?(data-{}=['|"]{}['|"])(?=.*>)"#, preg_quote(el), preg_quote(&id), preg_quote(&content));
    let exists = fancy_regex::Regex::new(&pattern).ok().and_then(|re| re.is_match(html).ok()).unwrap_or(false);
    if exists {
        return html.to_owned();
    }
    match html.find(&format!("<{el}")) {
        Some(pos) => format!("{}<{el} data-{id}=\"{content}\"{}", &html[..pos], &html[pos + el.len() + 1..]),
        None => html.to_owned(),
    }
}

/// `cmplz_cookie_blocker::add_class( $html, $el, $class )`.
pub fn add_class(html: &str, el: &str, class: &str) -> String {
    let classes: Vec<&str> = class.split(' ').filter(|c| !c.is_empty() && *c != "0").collect();
    let check = Regex::new(&format!(r#"(?i)<{}[^>]*[\t\n\x0B\x0C\r ]+(class=")(.*)""#, preg_quote(el))).expect("valid regex");
    let mut html = html.to_owned();
    if let Some(caps) = check.captures(&html) {
        let existing = caps[2].to_owned();
        let insert = Regex::new(&format!(r#"(?i)(<{}[^>]*[\t\n\x0B\x0C\r ]+)(class=")"#, preg_quote(el))).expect("valid regex");
        for class in classes {
            if !existing.contains(class) && !class.trim().is_empty() {
                let replacement = format!("${{1}}class=\"{} ", esc_attr(class).replace('$', "$$"));
                html = insert.replacen(&html, 1, replacement.as_str()).into_owned();
            }
        }
    } else if let Some(pos) = html.find(&format!("<{el}")) {
        html = format!("{}<{el} class=\"{}\"{}", &html[..pos], esc_attr(class), &html[pos + el.len() + 1..]);
    }
    html
}

/// `cmplz_cookie_blocker::set_javascript_to_plain( $script )`.
pub fn set_javascript_to_plain(script: &str) -> String {
    if script.contains("text/plain") {
        return script.to_owned();
    }
    static JS: OnceLock<fancy_regex::Regex> = OnceLock::new();
    static MODULE: OnceLock<Regex> = OnceLock::new();
    let js = JS.get_or_init(|| fancy_regex::Regex::new(r#"(?i)(<script[^>].*?)(type=['|"]text/javascript['|"])(?=.*>)"#).expect("valid regex"));
    if js.is_match(script).unwrap_or(false) {
        return js.replacen(script, 1, "${1}type=\"text/plain\"").into_owned();
    }
    let module = MODULE.get_or_init(|| Regex::new(r#"(?i)(<script[^>]*?)(type=['|"]module['|"])([^>]*?>)"#).expect("valid regex"));
    if module.is_match(script) {
        return module.replacen(script, 1, "${1} type=\"text/plain\" data-script-type=\"module\" ${3}").into_owned();
    }
    match script.find("<script") {
        Some(pos) => format!("{}<script type=\"text/plain\"{}", &script[..pos], &script[pos + 7..]),
        None => script.to_owned(),
    }
}

/// `cmplz_cookie_blocker::replace_src( $script, $new_src )`. The pattern's class lists accented letters, which PCRE
/// without `/u` reads byte by byte; it is matched on bytes here.
pub fn replace_src(script: &str, new_src: &str) -> String {
    static SRC: OnceLock<regex::bytes::Regex> = OnceLock::new();
    let re = SRC.get_or_init(|| {
        let bytes = |chars: &str| chars.bytes().map(|b| format!("\\x{b:02X}")).collect::<String>();
        let accents = bytes("êëèéēėęàáâæãåāäöôòóœøüÄÖÜß");
        let first = format!(r"[\s\w{accents}.,@!?^=%&:/~+#-;]");
        let last = r"[\w@!?^=%&/~+#-;]".to_string();
        regex::bytes::Regex::new(&format!(r#"(?i-u)src=['"](http://|https://|//)({first}*{last}?)['"]"#)).expect("valid regex")
    });
    let replacement = format!(" src=\"{new_src}\" ");
    let out = re.replace_all(script.as_bytes(), regex::bytes::NoExpand(replacement.as_bytes()));
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_scripts() {
        assert_eq!(set_javascript_to_plain("<script src=\"x\"></script>"), "<script type=\"text/plain\" src=\"x\"></script>");
        assert_eq!(set_javascript_to_plain("<script type='text/javascript' src=\"x\"></script>"), "<script type=\"text/plain\" src=\"x\"></script>");
        assert_eq!(set_javascript_to_plain("<script type=\"module\" src=\"x\"></script>"), "<script  type=\"text/plain\" data-script-type=\"module\"  src=\"x\"></script>");
    }

    #[test]
    fn data_and_classes() {
        assert_eq!(add_data("<div class=\"a\">", "div", "service", "x"), "<div data-service=\"x\" class=\"a\">");
        assert_eq!(add_data("<div data-service=\"x\" class=\"a\">", "div", "service", "x"), "<div data-service=\"x\" class=\"a\">");
        assert_eq!(add_class("<div class=\"a\">", "div", "b c"), "<div class=\"c b a\">");
        assert_eq!(add_class("<div>", "div", "b"), "<div class=\"b\">");
    }

    #[test]
    fn sources() {
        assert_eq!(replace_src("<iframe src=\"https://www.youtube.com/embed/x\" width=\"5\">", "about:blank"), "<iframe  src=\"about:blank\"  width=\"5\">");
        assert_eq!(replace_src("<iframe src='https://a.b/c' width='5'>", "about:blank"), "<iframe  src=\"about:blank\" >");
    }
}
