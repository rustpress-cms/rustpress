//! The installer speaks of RustPress: its pages are those of WordPress with the name and the logo exchanged.

/// The logo as a `url()` for CSS.
pub fn logo_url() -> String {
    rustpress_core::brand::logo_url()
}

/// Whether the `WordPress` at `rest` names the software (not WordPress.org, its forums or a path).
fn names_software(before: &str, rest: &str) -> bool {
    let after = &rest["WordPress".len()..];
    let next = after.chars().next();
    if before.ends_with(['/', '.', '-', '_'])
        || next.is_some_and(|c| {
            c.is_alphanumeric() || c == '.' && after[1..].starts_with(|c: char| c.is_alphanumeric())
        })
    {
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
        out.push_str(if names_software(before, &rest[at..]) {
            "RustPress"
        } else {
            "WordPress"
        });
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
            let end = rest[open..]
                .find("</script>")
                .map_or(rest.len(), |e| open + e);
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
    let style = format!(
        "<style>#logo,#logo a{{background-image:{}!important}}</style>\n",
        logo_url()
    );
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
