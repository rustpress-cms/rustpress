//! `Red_Regex`: a source URL as a PCRE pattern (`@...@s`, `i` when the case is ignored).
//!
//! Differs from PHP: `fancy-regex` matches characters rather than bytes and knows Unicode classes (`\w` also
//! matches `ü`, PCRE without `/u` does not); a pattern it cannot compile matches nothing, like a PCRE error.

use super::php;

pub struct PcreRegex {
    re: Option<fancy_regex::Regex>,
}

impl PcreRegex {
    pub fn new(pattern: &str, case_insensitive: bool) -> Self {
        let pattern = php::rawurldecode(pattern);
        let flags = if case_insensitive { "(?si)" } else { "(?s)" };
        Self { re: fancy_regex::Regex::new(&format!("{flags}{pattern}")).ok() }
    }

    /// `preg_match() > 0`.
    pub fn is_match(&self, target: &str) -> bool {
        self.re.as_ref().is_some_and(|re| re.is_match(target).unwrap_or(false))
    }

    /// `Red_Regex::replace()`: `preg_replace()` of every match, then spaces encoded (`%20` in the path, `+` in the
    /// query).
    pub fn replace(&self, replacement: &str, target: &str) -> String {
        let Some(re) = &self.re else { return target.to_owned() };
        let mut out = String::new();
        let mut last = 0;
        for caps in re.captures_iter(target) {
            let Ok(caps) = caps else { return target.to_owned() };
            let Some(whole) = caps.get(0) else { continue };
            out.push_str(&target[last..whole.start()]);
            out.push_str(&expand(&caps, replacement));
            last = whole.end();
        }
        out.push_str(&target[last..]);
        let split: Vec<&str> = out.split('?').collect();
        if split.len() == 2 {
            format!("{}?{}", split[0].replace(' ', "%20"), split[1].replace(' ', "+"))
        } else {
            out.replace(' ', "%20")
        }
    }
}

/// The replacement of `preg_replace()`: `\n`, `$n` and `${n}` with up to two digits; a group that does not exist is
/// empty.
fn expand(caps: &fancy_regex::Captures<'_, str>, replacement: &str) -> String {
    let bytes = replacement.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let group = |n: usize| caps.get(n).map_or("", |m| m.as_str());
    let digits = |at: usize| -> Option<(usize, usize)> {
        let first = (*bytes.get(at)? as char).to_digit(10)? as usize;
        match bytes.get(at + 1).and_then(|b| (*b as char).to_digit(10)) {
            Some(second) => Some((first * 10 + second as usize, 2)),
            None => Some((first, 1)),
        }
    };
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' || b == b'$' {
            if let Some((n, len)) = digits(i + 1) {
                out.push_str(group(n));
                i += 1 + len;
                continue;
            }
            if b == b'$'
                && bytes.get(i + 1) == Some(&b'{')
                && let Some((n, len)) = digits(i + 2)
                && bytes.get(i + 2 + len) == Some(&b'}')
            {
                out.push_str(group(n));
                i += 3 + len;
                continue;
            }
        }
        let ch = replacement[i..].chars().next().unwrap_or_default();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_like_preg_replace() {
        let re = PcreRegex::new(r"^/blog/(\d+)/(.*)$", true);
        assert!(re.is_match("/blog/12/abc"));
        assert_eq!(re.replace("/$2?id=$1", "/blog/12/abc"), "/abc?id=12");
        assert_eq!(PcreRegex::new("^/rx/(.*)", false).replace(r"/x/\1/y", "/rx/Q?z=1"), "/x/Q?z=1/y");
        assert_eq!(PcreRegex::new("^/a/(.*)", false).replace("/b/${1}x $9", "/a/c d"), "/b/c%20dx%20");
        assert!(!PcreRegex::new("(", false).is_match("("));
    }
}
