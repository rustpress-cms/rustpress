//! `[audio]`, `[video]` and `[embed]` (`wp_audio_shortcode()`, `wp_video_shortcode()`, `WP_Embed::shortcode()`).

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};
use rustpress_core::shortcodes::Atts;
use rustpress_core::urls::add_query_arg;

use crate::env::Env;

const AUDIO_EXTENSIONS: [&str; 5] = ["mp3", "ogg", "flac", "m4a", "wav"];
const VIDEO_EXTENSIONS: [&str; 5] = ["mp4", "m4v", "webm", "ogv", "flv"];

/// `wp_check_filetype()` for media: `(extension, mime type)` of a URL.
fn check_filetype(url: &str) -> Option<(String, &'static str)> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let ext = path.rsplit_once('.')?.1.to_ascii_lowercase();
    let mime = match ext.as_str() {
        "mp3" | "m4a" | "m4b" => "audio/mpeg",
        "aac" => "audio/aac",
        "ra" | "ram" => "audio/x-realaudio",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "flac" => "audio/flac",
        "mid" | "midi" => "audio/midi",
        "wma" => "audio/x-ms-wma",
        "wax" => "audio/x-ms-wax",
        "mka" => "audio/x-matroska",
        "asf" | "asx" => "video/x-ms-asf",
        "wmv" => "video/x-ms-wmv",
        "wmx" => "video/x-ms-wmx",
        "wm" => "video/x-ms-wm",
        "avi" => "video/avi",
        "divx" => "video/divx",
        "flv" => "video/x-flv",
        "mov" | "qt" => "video/quicktime",
        "mpeg" | "mpg" | "mpe" => "video/mpeg",
        "mp4" | "m4v" => "video/mp4",
        "ogv" => "video/ogg",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "3gp" | "3gpp" => "video/3gpp",
        "3g2" | "3gp2" => "video/3gpp2",
        _ => return None,
    };
    Some((ext, mime))
}

fn validate_boolean(value: &str) -> bool {
    !matches!(value.to_ascii_lowercase().as_str(), "" | "false" | "0" | "off" | "no")
}

fn attr_value<'a>(atts: &'a Atts, name: &str, default: &'a str) -> &'a str {
    atts.get(name).unwrap_or(default)
}

/// `wp_mediaelement_fallback()`.
fn fallback(url: &str) -> String {
    format!("<a href=\"{0}\">{0}</a>", esc_url(url))
}

/// The attributes of the media element, in order; `true` values stand for bare attributes.
fn element_attributes(pairs: &[(&str, String)], bare: &[&str]) -> String {
    let mut out = Vec::new();
    for (name, value) in pairs {
        if bare.contains(name) {
            if value == "1" {
                out.push(esc_attr(name));
            }
        } else if *name == "preload" {
            if matches!(value.as_str(), "none" | "metadata" | "auto") {
                out.push(format!("preload=\"{}\"", esc_attr(value)));
            }
        } else if !value.is_empty() {
            out.push(format!("{}=\"{}\"", esc_attr(name), esc_attr(value)));
        }
    }
    out.join(" ")
}

/// `wp_audio_shortcode()`; `None` is PHP's `null` (an empty string in the content).
pub fn audio(env: &Env<'_>, post_id: u64, atts: &Atts) -> Option<String> {
    let instance = env.with_state(|s| {
        s.audio_instances += 1;
        s.audio_instances
    });
    let src = attr_value(atts, "src", "");
    let mut types: Vec<&str> = AUDIO_EXTENSIONS.to_vec();
    let mut primary = false;
    if !src.is_empty() {
        match check_filetype(src) {
            Some((ext, _)) if AUDIO_EXTENSIONS.contains(&ext.as_str()) => {}
            _ => return Some(format!("<a class=\"wp-embedded-audio\" href=\"{}\">{}</a>", esc_url(src), esc_html(src))),
        }
        primary = true;
        types.insert(0, "src");
    } else {
        for ext in &AUDIO_EXTENSIONS {
            if let Some(url) = atts.get(ext).filter(|u| !u.is_empty())
                && check_filetype(url).is_some_and(|(e, _)| e == *ext)
            {
                primary = true;
            }
        }
    }
    if !primary {
        // `get_attached_media( 'audio' )` is not supported: the shortcode produces nothing.
        return None;
    }
    env.with_state(|s| {
        if !s.mediaelement {
            s.mediaelement = true;
            s.note_block("wp:mediaelement");
        }
    });
    let pairs = [
        ("class", attr_value(atts, "class", "wp-audio-shortcode").to_owned()),
        ("id", format!("audio-{post_id}-{instance}")),
        ("loop", u8::from(validate_boolean(attr_value(atts, "loop", ""))).to_string()),
        ("autoplay", u8::from(validate_boolean(attr_value(atts, "autoplay", ""))).to_string()),
        ("muted", u8::from(validate_boolean(attr_value(atts, "muted", "false"))).to_string()),
        ("preload", attr_value(atts, "preload", "none").to_owned()),
        ("style", attr_value(atts, "style", "width: 100%;").to_owned()),
    ];
    // Booleans that are off are left out; `preload` stays only when it is not empty.
    let pairs: Vec<(&str, String)> = pairs.into_iter().filter(|(n, v)| !(matches!(*n, "loop" | "autoplay" | "muted") && v == "0")).collect();
    let mut html = format!("<audio {} controls=\"controls\">", element_attributes(&pairs, &["loop", "autoplay", "muted"]));
    let mut fileurl = String::new();
    for kind in types {
        let Some(url) = atts.get(kind).filter(|u| !u.is_empty()) else { continue };
        if fileurl.is_empty() {
            fileurl = url.to_owned();
        }
        let mime = check_filetype(url).map_or("", |(_, m)| m);
        html.push_str(&format!("<source type=\"{mime}\" src=\"{}\" />", esc_url(&add_query_arg(url, "_", &instance.to_string()))));
    }
    html.push_str(&fallback(&fileurl));
    html.push_str("</audio>");
    Some(html)
}

/// `wp_video_shortcode()`.
pub fn video(env: &Env<'_>, post_id: u64, atts: &Atts, content: &str) -> Option<String> {
    let instance = env.with_state(|s| {
        s.video_instances += 1;
        s.video_instances
    });
    let src = attr_value(atts, "src", "").to_owned();
    let mut types: Vec<&str> = VIDEO_EXTENSIONS.to_vec();
    let (mut is_vimeo, mut is_youtube) = (false, false);
    let mut primary = false;
    if !src.is_empty() {
        is_vimeo = regex::Regex::new(r"^https?://(.+\.)?vimeo\.com/.*").is_ok_and(|re| re.is_match(&src));
        is_youtube = regex::Regex::new(r"^https?://(?:www\.)?(?:youtube\.com/watch|youtu\.be/)").is_ok_and(|re| re.is_match(&src));
        if !is_youtube && !is_vimeo {
            match check_filetype(&src) {
                Some((ext, _)) if VIDEO_EXTENSIONS.contains(&ext.as_str()) => {}
                _ => return Some(format!("<a class=\"wp-embedded-video\" href=\"{}\">{}</a>", esc_url(&src), esc_html(&src))),
            }
        }
        primary = true;
        types.insert(0, "src");
    } else {
        for ext in &VIDEO_EXTENSIONS {
            if let Some(url) = atts.get(ext).filter(|u| !u.is_empty())
                && check_filetype(url).is_some_and(|(e, _)| e == *ext)
            {
                primary = true;
            }
        }
    }
    if !primary {
        return None;
    }
    env.with_state(|s| {
        if !s.mediaelement {
            s.mediaelement = true;
            s.note_block("wp:mediaelement");
        }
        s.vimeo = true;
    });
    let mut src = src;
    if is_youtube {
        src = rustpress_core::urls::remove_query_arg(&src, "feature");
        if let Some(rest) = src.strip_prefix("http://") {
            src = format!("https://{rest}");
        }
    } else if is_vimeo {
        let parsed = src.split_once("://").map_or("", |(_, r)| r);
        let (host, path) = parsed.split_once('/').map_or((parsed, ""), |(h, p)| (h, p));
        let path = path.split(['?', '#']).next().unwrap_or("");
        let loop_value = if validate_boolean(attr_value(atts, "loop", "")) { "1" } else { "0" };
        src = add_query_arg(&format!("https://{host}/{path}"), "loop", loop_value);
    }
    let number = |name: &str, default: u64| attr_value(atts, name, "").trim().parse::<u64>().unwrap_or(default);
    let width = number("width", 640);
    let height = number("height", 360);
    let poster = attr_value(atts, "poster", "");
    let pairs = [
        ("class", attr_value(atts, "class", "wp-video-shortcode").to_owned()),
        ("id", format!("video-{post_id}-{instance}")),
        ("width", width.to_string()),
        ("height", height.to_string()),
        ("poster", if poster.is_empty() { String::new() } else { esc_url(poster) }),
        ("loop", u8::from(validate_boolean(attr_value(atts, "loop", ""))).to_string()),
        ("autoplay", u8::from(validate_boolean(attr_value(atts, "autoplay", ""))).to_string()),
        ("muted", u8::from(validate_boolean(attr_value(atts, "muted", "false"))).to_string()),
        ("preload", attr_value(atts, "preload", "metadata").to_owned()),
    ];
    let pairs: Vec<(&str, String)> = pairs.into_iter().filter(|(n, v)| !(matches!(*n, "loop" | "autoplay" | "muted") && v == "0")).collect();
    let mut html = format!("<video {} controls=\"controls\">", element_attributes(&pairs, &["loop", "autoplay", "muted"]));
    let mut fileurl = String::new();
    for kind in types {
        let url = if kind == "src" { src.as_str() } else { attr_value(atts, kind, "") };
        if url.is_empty() {
            continue;
        }
        if fileurl.is_empty() {
            fileurl = url.to_owned();
        }
        let mime = if kind == "src" && is_youtube {
            "video/youtube"
        } else if kind == "src" && is_vimeo {
            "video/vimeo"
        } else {
            check_filetype(url).map_or("", |(_, m)| m)
        };
        html.push_str(&format!("<source type=\"{mime}\" src=\"{}\" />", esc_url(&add_query_arg(url, "_", &instance.to_string()))));
    }
    if !content.is_empty() {
        let content = if content.contains('\n') { content.replace("\r\n", "").replace(['\n', '\t'], "") } else { content.to_owned() };
        html.push_str(content.trim());
    }
    html.push_str(&fallback(&fileurl));
    html.push_str("</video>");
    let width_rule = if width != 0 { format!("width: {width}px;") } else { String::new() };
    Some(format!("<div style=\"{width_rule}\" class=\"wp-video\">{html}</div>"))
}

/// `[embed]`: without a network the address becomes a link (`WP_Embed::maybe_make_link()`).
pub fn embed(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return String::new();
    }
    format!("<a href=\"{0}\">{0}</a>", esc_url(url))
}
