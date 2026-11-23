//! Text from visitors and authors must never panic, overflow the stack or take long, whatever it holds: the
//! functions that filter and format it are called with random mixes of markup, shortcodes, entities, serialized
//! values and characters whose case changes their length, and with deeply nested or very long input.
//!
//! The inputs come from a fixed seed, so a failure repeats; it names the function and the shortest input found.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use rustpress_content::sanitize;
use rustpress_core::{
    canonical, formatting, kses, php_lit, rewrite, safe_css, shortcodes, slug, text,
};

/// xorshift64: the same inputs in every run.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, limit: usize) -> usize {
        (self.next() % limit as u64) as usize
    }
}

const TOKENS: &[&str] = &[
    "<",
    ">",
    "</",
    "/>",
    "<!--",
    "-->",
    "<![CDATA[",
    "]]>",
    "&",
    "&amp;",
    "&#",
    "&#x",
    ";",
    "&#106;",
    "&hellip;",
    "&nbsp",
    "\"",
    "'",
    "=",
    " ",
    "\n",
    "\r\n",
    "\t",
    "\0",
    "\\",
    "[",
    "]",
    "[/",
    "[[",
    "]]",
    "[caption",
    "[gallery ids=\"1,2\"]",
    "[embed]",
    "[/embed]",
    "[video",
    "[playlist",
    "<!-- wp:",
    "<!-- /wp:",
    " /-->",
    "paragraph",
    "group",
    "{",
    "}",
    "{\"a\":",
    "\"x\"",
    ":",
    ",",
    "core/",
    "navigation-link",
    "a",
    "p",
    "div",
    "script",
    "style",
    "pre",
    "img",
    "svg",
    "textarea",
    "table",
    "tr",
    "td",
    "br",
    "hr",
    "blockquote",
    "form",
    "object",
    "math",
    "select",
    "option",
    "href",
    "src",
    "onerror",
    "style=",
    "class",
    "id",
    "javascript:",
    "data:text/html,",
    "http://",
    "https://",
    "//",
    "www.",
    "ftp.",
    "mailto:",
    "example.com",
    "@",
    ".",
    "?",
    "#",
    "%",
    "%zz",
    "%00",
    "%2e%2e",
    "+",
    // Characters whose lowercase or uppercase form has another length, combining marks, separators, astral planes.
    "İ",
    "ß",
    "ǅ",
    "ſ",
    "K",
    "é",
    "\u{0301}",
    "\u{2028}",
    "\u{feff}",
    "\u{200b}",
    "😀",
    "漢",
    "\u{10ffff}",
    "ΐ",
    "ŉ",
    "ǰ",
    "a:1:{",
    "s:3:\"",
    "i:",
    "b:1;",
    "N;",
    "d:0.5;",
    "O:8:\"stdClass\":",
    "a:2:{i:0;",
    "9999999999999999999",
    "-",
    "0",
    "1",
    "e",
    "E99999",
    "--",
    "boundary=",
    "multipart/form-data",
    "Content-Disposition: form-data; name=\"",
    "filename=\"",
    "\r\n\r\n",
    "url(",
    ")",
    "expression(",
    "/*",
    "*/",
    "!important",
    "@import",
    "var(--",
    "'s",
    "''",
    "\"\"",
    "...",
    "---",
    "(c)",
    "1x1",
    "12'",
    "9\"",
    "\\'",
    "`",
    "^",
    "$",
    "|",
    "(",
    "*",
    "~",
    "array(",
    "=>",
    "<?php",
    "?>",
    "<?xml",
    "<!DOCTYPE",
    "<![",
    "xlink:href",
    "<use",
    "<a href=\"",
    "\">",
    "</a>",
    "<p>",
    "</p>",
    "<br />",
    "\n\n",
    "<li>",
    "<ul>",
    "</ul>",
];

fn random_text(random: &mut Random) -> String {
    let tokens = match random.below(10) {
        0 => 0,
        1..=6 => random.below(12),
        7 | 8 => random.below(60),
        _ => random.below(400),
    };
    let mut out = String::new();
    for _ in 0..tokens {
        if random.below(12) == 0 {
            out.push(char::from_u32((random.next() % 0x3000) as u32).unwrap_or('x'));
        } else {
            out.push_str(TOKENS[random.below(TOKENS.len())]);
        }
    }
    // Sometimes the text ends in the middle of a token.
    if random.below(8) == 0 && !out.is_empty() {
        let mut cut = random.below(out.len());
        while !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
    }
    out
}

type Target = (&'static str, fn(&str));

/// The functions under test, by the WordPress function or the part of the request they stand for.
const TARGETS: &[Target] = &[
    ("wp_kses_post", |s| drop(kses::kses_post(s))),
    ("wp_kses_data", |s| drop(kses::kses_data(s))),
    ("comment content filters", |s| {
        kses::comment_content_filter(s, "http://example.com", true);
        kses::comment_content_filter(s, "http://example.com", false);
        kses::rel_ugc(s, "http://example.com");
    }),
    ("kses entities and protocols", |s| {
        kses::normalize_entities(s);
        kses::decode_entities(s);
        kses::convert_invalid_entities(s);
        kses::pre_kses_less_than(s);
        kses::decode_attribute(s);
        kses::bad_protocol(s, &["http", "https", "mailto"]);
    }),
    ("wpautop", |s| drop(formatting::wpautop(s))),
    ("wptexturize", |s| drop(formatting::wptexturize(s))),
    ("tags", |s| {
        formatting::html_split(s);
        formatting::strip_tags(s);
        formatting::wp_strip_all_tags(s);
        formatting::wp_trim_words(s, 5, "…");
    }),
    ("escaping", |s| {
        formatting::esc_url(s);
        formatting::esc_url_raw(s);
        formatting::esc_html(s);
        formatting::esc_attr(s);
        formatting::esc_js(s);
        formatting::convert_chars(s);
        formatting::ent2ncr(s);
        formatting::html_entity_decode(s);
        formatting::sanitize_html_class(s);
        formatting::capital_p_dangit(s, false);
        formatting::specialchars_noquotes(s);
    }),
    ("make_clickable", |s| {
        drop(text::make_clickable(
            s,
            &text::Clickable {
                home_host: "example.com",
                comment: true,
            },
        ))
    }),
    ("convert_smilies", |s| {
        drop(text::convert_smilies(s, "http://example.com/wp-includes/"))
    }),
    ("shortcodes", |s| {
        shortcodes::parse_atts(s);
        shortcodes::strip_shortcodes(s);
        shortcodes::shortcode_unautop(s);
        shortcodes::protect_html(s, false);
        shortcodes::protect_html(s, true);
        shortcodes::unescape_invalid(s);
        shortcodes::find(s, 0, &["caption", "gallery", "embed", "video"]);
    }),
    ("sanitize_title", |s| {
        slug::sanitize_title(s);
        slug::sanitize_title_save(s, "de_DE");
        slug::remove_accents(s, "de_DE");
    }),
    ("safecss_filter_attr", |s| {
        drop(safe_css::safecss_filter_attr(s))
    }),
    ("the request path", |s| {
        rewrite::urldecode(s.as_bytes());
        rewrite::request_path(s, "/");
        rewrite::request_path(s, "/blog");
        rewrite::request_path(s, "/é");
        canonical::split_url(s);
        rustpress_core::query::parse_cookies([s]);
    }),
    ("PHP literals", |s| drop(php_lit::parse(s))),
    ("sanitize_*", |s| {
        sanitize::text_field(s);
        sanitize::textarea_field(s);
        sanitize::user_login(s, true);
        sanitize::user_login(s, false);
        sanitize::key(s);
        sanitize::file_name(s, "de_DE");
        sanitize::esc_textarea(s);
        sanitize::is_email(s);
        sanitize::sanitize_email(s);
    }),
    ("parse_blocks", |s| {
        rustpress_blocks::parse_blocks(s);
        rustpress_blocks::parser::block_delimiters(s);
    }),
    ("multipart/form-data", |s| {
        rustpress_admin::multipart::boundary(s);
        rustpress_admin::multipart::parse("--", s.as_bytes());
        rustpress_admin::multipart::parse("x", s.as_bytes());
    }),
    ("unserialize", |s| {
        drop(rustpress_phpser::unserialize(s.as_bytes()))
    }),
];

/// A call may take this long (a debug build on a busy runner; the slowest seen is a few milliseconds).
const SLOW: Duration = Duration::from_secs(5);

#[test]
fn random_input_never_panics() {
    let mut random = Random(0x9E37_79B9_7F4A_7C15);
    let mut failures: Vec<(&str, String)> = Vec::new();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    for _ in 0..4000 {
        let input = random_text(&mut random);
        for (name, call) in TARGETS {
            let started = Instant::now();
            let panicked = catch_unwind(AssertUnwindSafe(|| call(&input))).is_err();
            if panicked || started.elapsed() > SLOW {
                match failures.iter_mut().find(|(failed, _)| failed == name) {
                    Some((_, shortest)) if input.len() < shortest.len() => {
                        *shortest = input.clone()
                    }
                    Some(_) => {}
                    None => failures.push((name, input.clone())),
                }
            }
        }
    }
    std::panic::set_hook(hook);
    assert!(
        failures.is_empty(),
        "panicked or took over {SLOW:?}: {failures:#?}"
    );
}

/// Nested or repeated thousands of times: a recursive function would overflow the stack (which ends the whole
/// server, not one request), a quadratic one would take minutes.
#[test]
fn deep_and_long_input_is_handled() {
    let nested = |open: &str, close: &str, depth: usize| open.repeat(depth) + &close.repeat(depth);
    let inputs = [
        nested("<div>", "</div>", 20_000),
        nested("<blockquote>", "</blockquote>", 5_000),
        nested("<!-- wp:group -->", "<!-- /wp:group -->", 20_000),
        nested("a:1:{i:0;", "}", 20_000),
        nested("[caption]", "[/caption]", 5_000),
        nested("array(", ")", 20_000),
        nested("[", "]", 20_000),
        "<".repeat(50_000),
        "&".repeat(50_000),
        "\"'".repeat(20_000),
        "www.".repeat(20_000),
        "<a href=\"http://a.b/".repeat(5_000),
        "\n\n<p>".repeat(10_000),
        "1 ? ".repeat(20_000),
    ];
    // The stack of a request handler (`thread_stack_size` in the server's `main`).
    let worker = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            for input in &inputs {
                for (name, call) in TARGETS {
                    let started = Instant::now();
                    call(input);
                    let took = started.elapsed();
                    assert!(
                        took < Duration::from_secs(30),
                        "{name} took {took:?} for {} bytes starting with {:?}",
                        input.len(),
                        &input[..20]
                    );
                }
            }
        });
    worker
        .expect("the thread starts")
        .join()
        .expect("no function panics");
}
