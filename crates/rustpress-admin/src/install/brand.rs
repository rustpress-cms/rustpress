//! The installer speaks of RustPress: its pages are those of WordPress with the name and the logo exchanged.

/// A gear around an R (the R of Liberation Serif Bold, SIL Open Font License), grey like the logo of WordPress.
const LOGO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><g fill="#3c434a"><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(0.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(20.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(40.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(60.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(80.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(100.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(120.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(140.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(160.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(180.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(200.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(220.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(240.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(260.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(280.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(300.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(320.0 50 50)"/><rect x="45.5" y="1" width="9" height="10" rx="1.5" transform="rotate(340.0 50 50)"/><circle cx="50" cy="50" r="41"/></g><circle cx="50" cy="50" r="36" fill="#fff"/><path fill="#3c434a" d="M40.94 53.82V71.27L47.35 72.28V75H23.23V72.28L29.16 71.27V28.69L22.74 27.72V25H46.64Q57.68 25 63.12 28.39Q68.57 31.79 68.57 38.98Q68.57 49.72 58.5 52.78L71.85 71.27L77.26 72.28V75H61.07L47.13 53.82ZM56.94 39.06Q56.94 33.46 54.62 31.28Q52.31 29.1 46.16 29.1H40.94V49.72H46.35Q52.09 49.72 54.51 47.33Q56.94 44.95 56.94 39.06Z"/></svg>"##;

/// The logo as a `url()` for CSS.
pub fn logo_url() -> String {
    let svg = LOGO.trim().replace('"', "'").replace('#', "%23").replace('<', "%3C").replace('>', "%3E");
    format!("url(\"data:image/svg+xml,{svg}\")")
}

/// Whether the `WordPress` at `rest` names the software (not WordPress.org, its forums or a path).
fn names_software(before: &str, rest: &str) -> bool {
    let after = &rest["WordPress".len()..];
    let next = after.chars().next();
    if before.ends_with(['/', '.', '-', '_']) || next.is_some_and(|c| c.is_alphanumeric() || c == '.' && after[1..].starts_with(|c: char| c.is_alphanumeric())) {
        return false;
    }
    let lower = after.to_lowercase();
    !(lower.starts_with(" support") || lower.starts_with("-support") || lower.starts_with(" codex"))
}

fn replace_names(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("WordPress") {
        out.push_str(&rest[..at]);
        let before = &text[..text.len() - rest.len() + at];
        out.push_str(if names_software(before, &rest[at..]) { "RustPress" } else { "WordPress" });
        rest = &rest[at + "WordPress".len()..];
    }
    out.push_str(rest);
    out
}

/// `html` with RustPress in the text, the titles and the button labels, and the RustPress logo.
pub fn rebrand(html: &str) -> String {
    let mut out = String::with_capacity(html.len() + 2048);
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        out.push_str(&replace_names(&rest[..open]));
        let Some(close) = rest[open..].find('>') else {
            rest = &rest[open..];
            break;
        };
        let tag = &rest[open..open + close + 1];
        // Text inside scripts stays as it is; in tags only the labels of buttons change.
        if tag.starts_with("<script") {
            let end = rest[open..].find("</script>").map_or(rest.len(), |e| open + e);
            out.push_str(&rest[open..end]);
            rest = &rest[end..];
            continue;
        }
        if tag.starts_with("<input") && tag.contains("type=\"submit\"") {
            out.push_str(&replace_names(tag));
        } else {
            out.push_str(tag);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(&replace_names(rest));
    let style = format!("<style>#logo,#logo a{{background-image:{}!important}}</style>\n", logo_url());
    if let Some(at) = out.find("</head>") {
        out.insert_str(at, &style);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::rebrand;

    #[test]
    fn names_change_but_links_stay() {
        let html = "<html><head><title>WordPress &rsaquo; Installation</title></head><body><p id=\"logo\">WordPress</p><p>Welcome to WordPress. Visit the <a href=\"https://wordpress.org/support/forums/\">WordPress support forums</a> or WordPress.org.</p><input type=\"submit\" value=\"Install WordPress\" /><input type=\"hidden\" value=\"WordPress\" /><script>var a = 'WordPress';</script></body></html>";
        let out = rebrand(html);
        assert!(out.contains("<title>RustPress &rsaquo; Installation</title>"));
        assert!(out.contains("<p id=\"logo\">RustPress</p>"));
        assert!(out.contains("Welcome to RustPress."));
        assert!(out.contains("WordPress support forums</a> or WordPress.org."));
        assert!(out.contains("value=\"Install RustPress\""));
        assert!(out.contains("type=\"hidden\" value=\"WordPress\""));
        assert!(out.contains("var a = 'WordPress';"));
        assert!(out.contains("#logo,#logo a{background-image:url(\"data:image/svg+xml,"));
    }
}
