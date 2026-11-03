/// `sanitize_title_with_dashes( $title, '', 'save' )`: the form in which WordPress stores `post_name` and term slugs
/// (without the `remove_accents()` of `sanitize_title()`, see [`sanitize_title_save`]).
///
/// Strips tags, keeps percent-encoded octets and drops other percent signs, lowercases and percent-encodes non-ASCII
/// bytes (`utf8_uri_encode( $title, 200 )`), applies the replacements of the `save` context, removes HTML entities,
/// turns `.` and spaces into `-`, drops other characters and collapses repeated dashes.
pub fn sanitize_title(input: &str) -> String {
    let title = crate::formatting::strip_tags(input);
    // Preserve escaped octets, remove percent signs that are not part of one.
    let bytes = title.as_bytes();
    let mut kept = String::with_capacity(title.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit) && bytes.get(i + 2).is_some_and(u8::is_ascii_hexdigit) {
                kept.push_str(&title[i..i + 3]);
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }
        let ch = title[i..].chars().next().unwrap_or(' ');
        kept.push(ch);
        i += ch.len_utf8();
    }
    // `mb_strtolower()`, then `utf8_uri_encode( $title, 200 )`: at most 200 bytes once encoded.
    let lower = if kept.is_ascii() { kept.to_ascii_lowercase() } else { kept.to_lowercase() };
    let mut encoded = String::with_capacity(lower.len());
    for ch in lower.chars() {
        let width = if ch.is_ascii() { 1 } else { ch.len_utf8() * 3 };
        if encoded.len() + width > 200 {
            break;
        }
        if ch.is_ascii() {
            encoded.push(ch);
        } else {
            let mut buf = [0u8; 4];
            for b in ch.encode_utf8(&mut buf).bytes() {
                encoded.push_str(&format!("%{b:02x}"));
            }
        }
    }
    let mut title = encoded.to_ascii_lowercase();
    // The `save` context.
    for from in ["%c2%a0", "%e2%80%91", "%e2%80%93", "%e2%80%94", "&nbsp;", "&#8209;", "&#160;", "&ndash;", "&#8211;", "&mdash;", "&#8212;", "/"] {
        title = title.replace(from, "-");
    }
    const STRIPPED: &[&str] = &[
        "%c2%ad", "%c2%a1", "%c2%bf", "%c2%ab", "%c2%bb", "%e2%80%b9", "%e2%80%ba", "%e2%80%98", "%e2%80%99", "%e2%80%9c", "%e2%80%9d", "%e2%80%9a", "%e2%80%9b", "%e2%80%9e", "%e2%80%9f", "%e2%80%a2", "%c2%a9", "%c2%ae", "%c2%b0", "%e2%80%a6", "%e2%84%a2", "%c2%b4",
        "%cb%8a", "%cc%81", "%cd%81", "%cc%80", "%cc%84", "%cc%8c", "%e2%80%8b", "%e2%80%8c", "%e2%80%8d", "%e2%80%8e", "%e2%80%8f", "%e2%80%aa", "%e2%80%ab", "%e2%80%ac", "%e2%80%ad", "%e2%80%ae", "%ef%bb%bf", "%ef%bf%bc",
    ];
    for from in STRIPPED {
        title = title.replace(from, "");
    }
    for from in ["%e2%80%80", "%e2%80%81", "%e2%80%82", "%e2%80%83", "%e2%80%84", "%e2%80%85", "%e2%80%86", "%e2%80%87", "%e2%80%88", "%e2%80%89", "%e2%80%8a", "%e2%80%a8", "%e2%80%a9", "%e2%80%af"] {
        title = title.replace(from, "-");
    }
    title = title.replace("%c3%97", "x");
    // Remove HTML entities (`/&.+?;/`: the shortest run up to a semicolon, not across a line break).
    let mut out = String::with_capacity(title.len());
    let mut rest = title.as_str();
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        match after.char_indices().skip(1).find(|(_, c)| *c == ';' || *c == '\n').filter(|(_, c)| *c == ';') {
            Some((end, _)) if !after.starts_with('\n') => rest = &after[end + 1..],
            _ => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    // `.` to dashes, everything but `%a-z0-9 _-` goes, whitespace runs become a dash, dashes collapse.
    let kept: String = out.replace('.', "-").chars().filter(|c| matches!(c, '%' | 'a'..='z' | '0'..='9' | ' ' | '_' | '-')).collect();
    let mut collapsed = String::with_capacity(kept.len());
    for ch in kept.chars() {
        let ch = if ch == ' ' { '-' } else { ch };
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
        assert_eq!(sanitize_title("ueber-uns"), "ueber-uns");
    }

    /// `sanitize_title_with_dashes( $in, '', 'save' )` of WordPress 7.1.
    #[test]
    fn slugs_like_wordpress() {
        let cases: &[(&str, &str)] = &[
        ("Hello-World", "hello-world"),
        ("a  b", "a-b"),
        ("v1.2", "v1-2"),
        ("--x--", "x"),
        ("a<b>c!", "ac"),
        ("Added &amp; co", "added-co"),
        ("a\tb &nbsp;c&mdash;d", "ab-c-d"),
        ("&\n;x", "x"),
        ("core/group", "core-group"),
        ("café", "caf%c3%a9"),
        ("caf%C3%A9", "caf%c3%a9"),
        ("100%;U+0", "100u0"),
        ("ÄÖÜ Straße", "%c3%a4%c3%b6%c3%bc-stra%c3%9fe"),
        ("x–y—z", "x-y-z"),
        ("“Quoted” ‘text’", "quoted-text"),
        ("a&b;c&d", "acd"),
        ("Ünïcödé ÉCOLE", "%c3%bcn%c3%afc%c3%b6d%c3%a9-%c3%a9cole"),
        ("éééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééééé", "%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9%c3%a9"),
        ("A × B", "a-x-b"),
        ("Tab\there", "tabhere"),
        ("a%2fb", "a%2fb"),
        ("50% off!", "50-off"),
        ("<script>x</script>y", "xy"),
        ("a&;;b", "ab"),
        ("line\nbreak", "linebreak"),
        ("xabababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababab", "xabababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababababa"),
        ("<!-- c -->d", "d"),
        ("İstanbul", "i%cc%87stanbul"),
        ];
        for (input, expected) in cases {
            assert_eq!(sanitize_title(input), *expected, "{input:?}");
        }
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
