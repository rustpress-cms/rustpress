/// Mirrors `sanitize_title_for_query()` for the cases that matter for lookups:
/// the form in which WordPress stores `post_name` and term slugs.
///
/// Lowercases, percent-encodes non-ASCII bytes with lowercase hex (as
/// `utf8_uri_encode` does), turns `.` and whitespace into `-`, drops other
/// punctuation and collapses repeated dashes.
pub fn sanitize_title(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii() {
            encoded.push(ch.to_ascii_lowercase());
        } else {
            let mut buf = [0u8; 4];
            for b in ch.encode_utf8(&mut buf).bytes() {
                encoded.push_str(&format!("%{b:02x}"));
            }
        }
    }
    // An already percent-encoded input (`caf%C3%A9`) is normalised to lowercase hex.
    let mut out = String::with_capacity(encoded.len());
    let chars: Vec<char> = encoded.chars().collect();
    for (i, ch) in chars.iter().copied().enumerate() {
        match ch {
            '.' | '/' | ' ' | '\t' | '\n' | '\r' => out.push('-'),
            'a'..='z' | '0'..='9' | '_' | '-' => out.push(ch),
            // A percent sign stays only as part of an octet (`%c3`).
            '%' if chars.get(i + 1).is_some_and(char::is_ascii_hexdigit) && chars.get(i + 2).is_some_and(char::is_ascii_hexdigit) => out.push(ch),
            _ => {}
        }
    }
    let mut collapsed = String::with_capacity(out.len());
    for ch in out.chars() {
        if ch == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(ch);
    }
    collapsed.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_slugs_are_lowercased() {
        assert_eq!(sanitize_title("Hello-World"), "hello-world");
        assert_eq!(sanitize_title("ueber-uns"), "ueber-uns");
        assert_eq!(sanitize_title("a  b"), "a-b");
        assert_eq!(sanitize_title("v1.2"), "v1-2");
        assert_eq!(sanitize_title("--x--"), "x");
        assert_eq!(sanitize_title("a<b>c!"), "abc");
        assert_eq!(sanitize_title("core/group"), "core-group");
    }

    #[test]
    fn non_ascii_is_stored_percent_encoded() {
        assert_eq!(sanitize_title("café"), "caf%c3%a9");
        assert_eq!(sanitize_title("caf%C3%A9"), "caf%c3%a9");
        assert_eq!(sanitize_title("100%;U+0"), "100u0");
        assert_eq!(sanitize_title("100%;U+0"), "100u0");
    }
}

/// `remove_accents( $text, $locale )`: Latin letters with accents become their plain forms (German, Danish,
/// Catalan and Serbian/Bosnian spell some of them out). Text already in NFD is not composed first.
pub fn remove_accents(text: &str, locale: &str) -> String {
    if text.is_ascii() {
        return text.to_owned();
    }
    let special: &[(char, &str)] = if locale.starts_with("de") {
        &[('Ä', "Ae"), ('ä', "ae"), ('Ö', "Oe"), ('ö', "oe"), ('Ü', "Ue"), ('ü', "ue"), ('ẞ', "SS"), ('ß', "ss")]
    } else if locale == "da_DK" {
        &[('Æ', "Ae"), ('æ', "ae"), ('Ø', "Oe"), ('ø', "oe"), ('Å', "Aa"), ('å', "aa")]
    } else if locale == "sr_RS" || locale == "bs_BA" {
        &[('Đ', "DJ"), ('đ', "dj")]
    } else {
        &[]
    };
    let text = if locale == "ca" { text.replace("l·l", "ll") } else { text.to_owned() };
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match special.iter().find(|(k, _)| *k == c).or_else(|| crate::accents_table::ACCENTS.iter().find(|(k, _)| *k == c)) {
            Some((_, plain)) => out.push_str(plain),
            None => out.push(c),
        }
    }
    out
}

/// `sanitize_title( $title )` in its default `save` context: [`remove_accents`], then [`sanitize_title`].
pub fn sanitize_title_save(title: &str, locale: &str) -> String {
    sanitize_title(&remove_accents(title, locale))
}

#[cfg(test)]
mod accents_tests {
    use super::*;

    #[test]
    fn accents_are_removed_like_wordpress() {
        assert_eq!(sanitize_title_save("Über uns", "en_US"), "uber-uns");
        assert_eq!(sanitize_title_save("Über uns", "de_DE"), "ueber-uns");
        assert_eq!(sanitize_title_save("Passwortgeschützt", "en_US"), "passwortgeschutzt");
        assert_eq!(sanitize_title_save("Straße", "de_DE_formal"), "strasse");
        assert_eq!(sanitize_title_save("Œuvre ½", "en_US"), "oeuvre-%c2%bd");
        assert_eq!(sanitize_title_save("Ελληνικά", "en_US"), sanitize_title("Ελληνικά"));
    }
}
