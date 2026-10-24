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
    for ch in encoded.chars() {
        match ch {
            '.' | '/' | ' ' | '\t' | '\n' | '\r' => out.push('-'),
            'a'..='z' | '0'..='9' | '%' | '_' | '-' => out.push(ch),
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
    }
}
