//! The administration speaks of RustPress, not of WordPress: a deliberate difference from WordPress, so that nobody
//! takes a RustPress site for a WordPress installation and no WordPress trademark is used for RustPress.
//!
//! While a request runs in [`scope`] (the server does that for `wp-admin`, `wp-login.php` and the toolbar, unless
//! `branding = false` in the configuration), the texts of WordPress' own text domain (`default`) that `Site::tr()`
//! and its relatives return name RustPress, in English and in every translation, and the links to the home, the
//! support forums and the feedback forum of WordPress lead to RustPress. What does not name the software stays as it
//! is: WordPress.org and its plugin and theme directories, the WordPress community, events and news, versions of
//! WordPress (`WordPress 7.1`, the release whose files RustPress uses) and the documentation articles about the
//! individual screens, which describe the same screens. Texts of plugins and themes and the content of the site are
//! never changed.

use std::borrow::Cow;

/// The home page of RustPress.
pub const HOME_URL: &str = "https://rustpress-cms.github.io/rustpress/";
/// The German home page of RustPress.
pub const HOME_URL_DE: &str = "https://rustpress-cms.github.io/rustpress/de/";
/// The repository: code, documentation and support.
pub const REPOSITORY_URL: &str = "https://github.com/rustpress-cms/rustpress";
/// Where users ask for help and report problems.
pub const SUPPORT_URL: &str = "https://github.com/rustpress-cms/rustpress/issues";
/// Where users send wishes and feedback.
pub const FEEDBACK_URL: &str = "https://github.com/rustpress-cms/rustpress/issues/new/choose";
/// The documentation (the README links the guides).
pub const DOCUMENTATION_URL: &str = "https://github.com/rustpress-cms/rustpress#readme";

/// A gear around an R (the R of Liberation Serif Bold, SIL Open Font License), grey like the logo of WordPress.
pub const LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><g fill="#3c434a"><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(0.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(20.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(40.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(60.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(80.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(100.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(120.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(140.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(160.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(180.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(200.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(220.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(240.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(260.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(280.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(300.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(320.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(340.0 50 50)"/><circle cx="50" cy="50" r="41"/></g><circle cx="50" cy="50" r="36" fill="#fff"/><path fill="#3c434a" d="M40.94 53.82V71.27L47.35 72.28V75H23.23V72.28L29.16 71.27V28.69L22.74 27.72V25H46.64Q57.68 25 63.12 28.39Q68.57 31.79 68.57 38.98Q68.57 49.72 58.5 52.78L71.85 71.27L77.26 72.28V75H61.07L47.13 53.82ZM56.94 39.06Q56.94 33.46 54.62 31.28Q52.31 29.1 46.16 29.1H40.94V49.72H46.35Q52.09 49.72 54.51 47.33Q56.94 44.95 56.94 39.06Z"/></svg>"##;

/// The logo as a mask (black on transparent, the inside of the gear open), so that CSS can paint it in the color of
/// the toolbar's icons.
const MASK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><g fill="#000"><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(0.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(20.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(40.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(60.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(80.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(100.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(120.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(140.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(160.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(180.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(200.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(220.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(240.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(260.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(280.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(300.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(320.0 50 50)"/><rect x="45.5" y="1" width="9" height="12" rx="1.5" transform="rotate(340.0 50 50)"/><path d="M40.94 53.82V71.27L47.35 72.28V75H23.23V72.28L29.16 71.27V28.69L22.74 27.72V25H46.64Q57.68 25 63.12 28.39Q68.57 31.79 68.57 38.98Q68.57 49.72 58.5 52.78L71.85 71.27L77.26 72.28V75H61.07L47.13 53.82ZM56.94 39.06Q56.94 33.46 54.62 31.28Q52.31 29.1 46.16 29.1H40.94V49.72H46.35Q52.09 49.72 54.51 47.33Q56.94 44.95 56.94 39.06Z"/></g><circle cx="50" cy="50" r="38.5" fill="none" stroke="#000" stroke-width="5"/></svg>"##;

/// An SVG as a `url()` for CSS.
fn svg_url(svg: &str) -> String {
    let svg = svg
        .trim()
        .replace('"', "'")
        .replace('#', "%23")
        .replace('<', "%3C")
        .replace('>', "%3E");
    format!("url(\"data:image/svg+xml,{svg}\")")
}

/// The logo as a `url()` for CSS.
pub fn logo_url() -> String {
    svg_url(LOGO)
}

/// The style that puts the RustPress logo into the toolbar, in place of the WordPress logo (a Dashicons glyph).
pub fn admin_bar_style() -> String {
    let mask = svg_url(MASK);
    format!(
        "<style id=\"rustpress-brand\">#wpadminbar #wp-admin-bar-wp-logo>.ab-item .ab-icon:before{{content:\"\";display:inline-block;width:20px;height:20px;background-color:currentColor;-webkit-mask:{mask} center/contain no-repeat;mask:{mask} center/contain no-repeat}}</style>\n"
    )
}

/// The style that puts the RustPress logo above the form of `wp-login.php`.
pub fn login_style() -> String {
    format!(
        "<style id=\"rustpress-brand\">.login h1 a{{background-image:{}!important}}</style>\n",
        logo_url()
    )
}

tokio::task_local! {
    static BRANDED: bool;
}

/// Runs `future` with the RustPress branding of the administration's texts.
pub async fn scope<F: std::future::Future>(future: F) -> F::Output {
    BRANDED.scope(true, future).await
}

/// Runs `f` with the RustPress branding (for the parts of a page that belong to the administration, such as the
/// toolbar on the site).
pub fn sync_scope<R>(f: impl FnOnce() -> R) -> R {
    BRANDED.sync_scope(true, f)
}

/// Whether the running request shows the administration as RustPress.
pub fn active() -> bool {
    BRANDED.try_with(|b| *b).unwrap_or(false)
}

/// A text of WordPress' text domain as the running request shows it: with RustPress in [`scope`], else unchanged.
pub fn text(text: String) -> String {
    if !active() || !(text.contains("WordPress") || text.contains("wordpress.org")) {
        return text;
    }
    match rebrand(&text) {
        Cow::Borrowed(_) => text,
        Cow::Owned(changed) => changed,
    }
}

/// Where a link of WordPress (the whole URL, as `__()` translates it) leads in RustPress, if it changes.
fn url(link: &str) -> Option<&'static str> {
    let rest = link
        .strip_prefix("https://")
        .or_else(|| link.strip_prefix("http://"))?;
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let german = match host {
        "wordpress.org" => false,
        "de.wordpress.org" => true,
        _ if host.ends_with(".wordpress.org") && host.len() <= "xx-xx.wordpress.org".len() => false,
        _ => return None,
    };
    let path = path.split(['#', '?']).next().unwrap_or_default();
    Some(match path.trim_end_matches('/') {
        "" | "about" => {
            if german {
                HOME_URL_DE
            } else {
                HOME_URL
            }
        }
        "support/forum/requests-and-feedback" => FEEDBACK_URL,
        "support" | "support/forums" => SUPPORT_URL,
        forum if forum.starts_with("support/forum/") => SUPPORT_URL,
        "documentation" | "support/documentation" => DOCUMENTATION_URL,
        _ => return None,
    })
}

/// What may follow `WordPress` when it does not name the software, in lower case (English and German).
const NOT_THE_SOFTWARE: [&str; 24] = [
    "plugin directory",
    "plugin-verzeichnis",
    "theme directory",
    "theme-verzeichnis",
    "event",
    "veranstaltung",
    "mobile",
    "constant",
    "konstante",
    "news",
    "community",
    "foundation",
    "core",
    "team",
    "project",
    "projekt",
    "codex",
    "planet",
    "meetup",
    "contributor",
    "mitwirkende",
    "trademark",
    "marke",
    "release",
];

/// Whether the `WordPress` at the start of `rest` names the software, after `before`.
fn names_software(before: &str, rest: &str) -> bool {
    let after = &rest["WordPress".len()..];
    let mut chars = after.chars();
    let next = chars.next();
    // Part of a word, a host name (WordPress.org), a path or an identifier; Learn WordPress (learn.wordpress.org)
    // and the default tagline, which Site Health compares with the stored one.
    if before.ends_with("Learn ")
        || before.ends_with("another ") && after.starts_with(" site")
        || before
            .ends_with(|c: char| c.is_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | '@'))
        || next.is_some_and(|c| c.is_alphanumeric() || c == '_')
        || next == Some('.') && chars.next().is_some_and(char::is_alphanumeric)
    {
        return false;
    }
    let word = after
        .trim_start_matches(|c: char| c.is_whitespace() || c == '-')
        .to_lowercase();
    // A version of WordPress: `WordPress 7.1`, `WordPress %s` (in German also with a no-break space).
    if after.starts_with(char::is_whitespace)
        && word.starts_with(|c: char| c.is_ascii_digit() || c == '%')
    {
        return false;
    }
    !NOT_THE_SOFTWARE.iter().any(|w| word.starts_with(w))
}

/// `text` with RustPress where `WordPress` names the software and the links of WordPress replaced.
fn rebrand(text: &str) -> Cow<'_, str> {
    if let Some(to) = url(text.trim()) {
        return Cow::Owned(text.replacen(text.trim(), to, 1));
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut changed = false;
    loop {
        let name = rest.find("WordPress");
        let link = rest.find("://").map(|at| {
            let start = rest[..at]
                .rfind(|c: char| !c.is_ascii_alphabetic())
                .map_or(0, |s| s + 1);
            let end = rest[at..]
                .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'))
                .map_or(rest.len(), |e| at + e);
            (start, end)
        });
        match (name, link) {
            (Some(n), Some((start, end))) if start <= n => {
                out.push_str(&rest[..start]);
                let link = &rest[start..end];
                match url(link) {
                    Some(to) => {
                        out.push_str(to);
                        changed = true;
                    }
                    None => out.push_str(link),
                }
                rest = &rest[end..];
            }
            (Some(n), _) => {
                out.push_str(&rest[..n]);
                let before = &text[..text.len() - rest.len() + n];
                if names_software(before, &rest[n..]) {
                    out.push_str("RustPress");
                    changed = true;
                } else {
                    out.push_str("WordPress");
                }
                rest = &rest[n + "WordPress".len()..];
            }
            (None, Some((start, end))) => {
                out.push_str(&rest[..start]);
                let link = &rest[start..end];
                match url(link) {
                    Some(to) => {
                        out.push_str(to);
                        changed = true;
                    }
                    None => out.push_str(link),
                }
                rest = &rest[end..];
            }
            (None, None) => break,
        }
    }
    if !changed {
        return Cow::Borrowed(text);
    }
    out.push_str(rest);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branded(s: &str) -> String {
        sync_scope(|| text(s.to_owned()))
    }

    #[test]
    fn only_in_scope() {
        assert_eq!(text("About WordPress".to_owned()), "About WordPress");
        assert_eq!(branded("About WordPress"), "About RustPress");
    }

    #[test]
    fn names_of_the_software_change() {
        for (from, to) in [
            (
                "%1$s &lsaquo; %2$s &#8212; WordPress",
                "%1$s &lsaquo; %2$s &#8212; RustPress",
            ),
            (
                "Thank you for creating with <a href=\"%s\">WordPress</a>.",
                "Thank you for creating with <a href=\"%s\">RustPress</a>.",
            ),
            ("Powered by WordPress", "Powered by RustPress"),
            ("Welcome to WordPress!", "Welcome to RustPress!"),
            ("Willkommen bei WordPress!", "Willkommen bei RustPress!"),
            (
                "Deine WordPress-Installation",
                "Deine RustPress-Installation",
            ),
            (
                "Plugins extend and expand the functionality of WordPress.",
                "Plugins extend and expand the functionality of RustPress.",
            ),
        ] {
            assert_eq!(branded(from), to, "{from}");
        }
    }

    #[test]
    fn other_meanings_stay() {
        for s in [
            "WordPress.org",
            "Something may be wrong with WordPress.org or this server&#8217;s configuration.",
            "WordPress Plugin Directory",
            "WordPress-Plugin-Verzeichnis",
            "WordPress Events and News",
            "WordPress %s",
            "Update to WordPress 7.2",
            "seit WordPress\u{a0}6.3 veraltet",
            "Learn WordPress",
            "Just another WordPress site",
            "WordPress Constants",
            "the WordPress mobile apps",
            "organize a WordPress event",
            "https://wordpress.org/documentation/article/settings-general-screen/",
            "https://wordpress.org/plugins/",
            "https://api.wordpress.org/core/version-check/1.7/",
            "wordpress_logged_in",
            "the-WordPress-thing/WordPress",
        ] {
            assert_eq!(branded(s), s, "{s}");
        }
    }

    #[test]
    fn links_change() {
        assert_eq!(branded("https://wordpress.org/"), HOME_URL);
        assert_eq!(branded("https://de.wordpress.org/"), HOME_URL_DE);
        assert_eq!(
            branded("https://wordpress.org/support/forums/"),
            SUPPORT_URL
        );
        assert_eq!(branded("https://de.wordpress.org/support/"), SUPPORT_URL);
        assert_eq!(
            branded("https://de.wordpress.org/support/forum/allgemeine-fragen/"),
            SUPPORT_URL
        );
        assert_eq!(
            branded("https://wordpress.org/support/forum/requests-and-feedback"),
            FEEDBACK_URL
        );
        assert_eq!(
            branded("https://wordpress.org/documentation/"),
            DOCUMENTATION_URL
        );
        assert_eq!(
            branded(
                "Try the <a href=\"https://wordpress.org/support/forums/\">support forums</a> of WordPress."
            ),
            format!("Try the <a href=\"{SUPPORT_URL}\">support forums</a> of RustPress.")
        );
    }
}
