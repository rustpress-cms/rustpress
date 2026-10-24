//! Translations: gettext `.mo` catalogs from `wp-content/languages` (core) and
//! `wp-content/languages/themes` plus the theme's own `languages` directory.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// The `Plural-Forms` expression of a catalog, e.g. `n != 1`.
#[derive(Debug, Clone, PartialEq)]
enum Expr {
    N,
    Num(i64),
    Not(Box<Expr>),
    Bin(Box<Expr>, &'static str, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Expr {
    fn eval(&self, n: i64) -> i64 {
        match self {
            Expr::N => n,
            Expr::Num(v) => *v,
            Expr::Not(e) => i64::from(e.eval(n) == 0),
            Expr::Cond(c, a, b) => {
                if c.eval(n) != 0 {
                    a.eval(n)
                } else {
                    b.eval(n)
                }
            }
            Expr::Bin(a, op, b) => {
                let (x, y) = (a.eval(n), b.eval(n));
                match *op {
                    "||" => i64::from(x != 0 || y != 0),
                    "&&" => i64::from(x != 0 && y != 0),
                    "==" => i64::from(x == y),
                    "!=" => i64::from(x != y),
                    "<" => i64::from(x < y),
                    ">" => i64::from(x > y),
                    "<=" => i64::from(x <= y),
                    ">=" => i64::from(x >= y),
                    "+" => x + y,
                    "-" => x - y,
                    "*" => x * y,
                    "/" => x.checked_div(y).unwrap_or(0),
                    "%" => x.checked_rem(y).unwrap_or(0),
                    _ => 0,
                }
            }
        }
    }
}

struct ExprParser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl ExprParser<'_> {
    fn skip(&mut self) {
        while self.src.get(self.pos).is_some_and(u8::is_ascii_whitespace) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip();
        if self.src[self.pos..].starts_with(token.as_bytes()) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }

    fn ternary(&mut self) -> Option<Expr> {
        let cond = self.binary(0)?;
        if self.eat("?") {
            let a = self.ternary()?;
            if !self.eat(":") {
                return None;
            }
            let b = self.ternary()?;
            return Some(Expr::Cond(Box::new(cond), Box::new(a), Box::new(b)));
        }
        Some(cond)
    }

    fn binary(&mut self, level: usize) -> Option<Expr> {
        const LEVELS: &[&[&str]] = &[&["||"], &["&&"], &["==", "!="], &["<=", ">=", "<", ">"], &["+", "-"], &["*", "/", "%"]];
        let Some(operators) = LEVELS.get(level) else { return self.unary() };
        let mut left = self.binary(level + 1)?;
        'outer: loop {
            for op in *operators {
                // `<` must not swallow the first half of `<=` (handled by listing `<=` first).
                if self.eat(op) {
                    let right = self.binary(level + 1)?;
                    left = Expr::Bin(Box::new(left), op, Box::new(right));
                    continue 'outer;
                }
            }
            return Some(left);
        }
    }

    fn unary(&mut self) -> Option<Expr> {
        self.skip();
        if self.eat("!") {
            return Some(Expr::Not(Box::new(self.unary()?)));
        }
        if self.eat("(") {
            let inner = self.ternary()?;
            return self.eat(")").then_some(inner);
        }
        if self.src.get(self.pos) == Some(&b'n') {
            self.pos += 1;
            return Some(Expr::N);
        }
        let start = self.pos;
        while self.src.get(self.pos).is_some_and(u8::is_ascii_digit) {
            self.pos += 1;
        }
        std::str::from_utf8(&self.src[start..self.pos]).ok()?.parse().ok().map(Expr::Num)
    }
}

/// One gettext catalog.
#[derive(Debug, Default)]
pub struct Catalog {
    /// `context\x04msgid` (or just `msgid`) -> the translations (one per plural form).
    entries: HashMap<String, Vec<String>>,
    plural: Option<Expr>,
    plural_count: usize,
}

fn read_u32(bytes: &[u8], at: usize, big: bool) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(if big { u32::from_be_bytes(raw) } else { u32::from_le_bytes(raw) })
}

impl Catalog {
    /// Parses a binary `.mo` file; `None` if it is not one.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let big = match read_u32(bytes, 0, false)? {
            0x9504_12de => false,
            0xde12_0495 => true,
            _ => return None,
        };
        let count = read_u32(bytes, 8, big)? as usize;
        let originals = read_u32(bytes, 12, big)? as usize;
        let translations = read_u32(bytes, 16, big)? as usize;
        let string = |table: usize, i: usize| -> Option<&[u8]> {
            let length = read_u32(bytes, table + i * 8, big)? as usize;
            let offset = read_u32(bytes, table + i * 8 + 4, big)? as usize;
            bytes.get(offset..offset + length)
        };
        let mut catalog = Catalog::default();
        for i in 0..count {
            let (Some(original), Some(translation)) = (string(originals, i), string(translations, i)) else { continue };
            let original = String::from_utf8_lossy(original).into_owned();
            let translation = String::from_utf8_lossy(translation).into_owned();
            if original.is_empty() {
                catalog.read_header(&translation);
                continue;
            }
            // Plural entries are `singular\0plural`; the lookup key is the singular.
            let key = original.split('\0').next().unwrap_or_default().to_owned();
            catalog.entries.insert(key, translation.split('\0').map(str::to_owned).collect());
        }
        Some(catalog)
    }

    fn read_header(&mut self, header: &str) {
        for line in header.lines() {
            let Some(rest) = line.strip_prefix("Plural-Forms:") else { continue };
            for part in rest.split(';') {
                let part = part.trim();
                if let Some(n) = part.strip_prefix("nplurals=") {
                    self.plural_count = n.trim().parse().unwrap_or(2);
                } else if let Some(expr) = part.strip_prefix("plural=") {
                    let mut parser = ExprParser { src: expr.as_bytes(), pos: 0 };
                    self.plural = parser.ternary();
                }
            }
        }
    }

    pub fn get(&self, context: Option<&str>, text: &str) -> Option<&str> {
        let key = match context {
            Some(c) => format!("{c}\u{4}{text}"),
            None => text.to_owned(),
        };
        self.entries.get(&key).and_then(|forms| forms.first()).map(String::as_str).filter(|t| !t.is_empty())
    }

    /// Whether the catalog has an entry (translated or not) for this message.
    pub fn contains(&self, context: Option<&str>, text: &str) -> bool {
        match context {
            Some(c) => self.entries.contains_key(&format!("{c}\u{4}{text}")),
            None => self.entries.contains_key(text),
        }
    }

    pub fn get_plural(&self, context: Option<&str>, singular: &str, n: i64) -> Option<&str> {
        let key = match context {
            Some(c) => format!("{c}\u{4}{singular}"),
            None => singular.to_owned(),
        };
        let forms = self.entries.get(&key)?;
        let index = match &self.plural {
            Some(expr) => usize::try_from(expr.eval(n)).unwrap_or(0).min(forms.len().saturating_sub(1)),
            None => usize::from(n != 1),
        };
        forms.get(index).map(String::as_str).filter(|t| !t.is_empty())
    }
}

/// All catalogs of the site by text domain (`default` is WordPress itself).
#[derive(Debug, Default)]
pub struct I18n {
    pub locale: String,
    domains: HashMap<String, Arc<Catalog>>,
    data: std::sync::OnceLock<crate::locale::LocaleData>,
    texturize: std::sync::OnceLock<crate::formatting::Texturize>,
}

impl I18n {
    /// Loads the core catalog and those of `themes` (`(slug, directory)`) for `locale`.
    pub fn load(wp_root: &Path, locale: &str, themes: &[(String, std::path::PathBuf)]) -> Self {
        let mut i18n = Self { locale: locale.to_owned(), domains: HashMap::new(), data: std::sync::OnceLock::new(), texturize: std::sync::OnceLock::new() };
        if locale.is_empty() || locale == "en_US" || locale.contains(['/', '\\', '.']) {
            return i18n;
        }
        let languages = wp_root.join("wp-content").join("languages");
        let read = |path: std::path::PathBuf| std::fs::read(path).ok().and_then(|b| Catalog::parse(&b));
        if let Some(core) = read(languages.join(format!("{locale}.mo"))) {
            i18n.domains.insert("default".to_owned(), Arc::new(core));
        }
        for (slug, dir) in themes {
            // The language pack in wp-content/languages wins over the theme's bundled files.
            let catalog = read(languages.join("themes").join(format!("{slug}-{locale}.mo"))).or_else(|| read(dir.join("languages").join(format!("{locale}.mo"))));
            if let Some(catalog) = catalog {
                i18n.domains.insert(slug.clone(), Arc::new(catalog));
            }
        }
        i18n
    }

    /// The rules of `wptexturize()` with this language's quotes and dashes, built once.
    pub fn texturize(&self) -> &crate::formatting::Texturize {
        self.texturize.get_or_init(|| {
            let x = |text: &str, context: &str| self.pgettext("default", context, text);
            let words = x("'tain't,'twere,'twas,'tis,'twill,'til,'bout,'nuff,'round,'cause,'em", "Comma-separated list of words to texturize in your language");
            let replacements = x(
                "&#8217;tain&#8217;t,&#8217;twere,&#8217;twas,&#8217;tis,&#8217;twill,&#8217;til,&#8217;bout,&#8217;nuff,&#8217;round,&#8217;cause,&#8217;em",
                "Comma-separated list of replacement words in your language",
            );
            crate::formatting::Texturize::new(crate::formatting::TexturizeChars {
                opening_quote: x("&#8220;", "opening curly double quote"),
                closing_quote: x("&#8221;", "closing curly double quote"),
                apos: x("&#8217;", "apostrophe"),
                prime: x("&#8242;", "prime"),
                double_prime: x("&#8243;", "double prime"),
                opening_single_quote: x("&#8216;", "opening curly single quote"),
                closing_single_quote: x("&#8217;", "closing curly single quote"),
                en_dash: x("&#8211;", "en dash"),
                em_dash: x("&#8212;", "em dash"),
                cockney: crate::formatting::TexturizeChars::cockney_from(&words, &replacements),
            })
        })
    }

    /// Whether the `domain` catalog knows this message (for checking message ids against a language pack).
    pub fn has(&self, domain: &str, context: Option<&str>, text: &str) -> bool {
        self.domains.get(domain).is_some_and(|c| c.contains(context, text))
    }

    /// The names and conventions of the locale (`WP_Locale`), read from the catalog once.
    pub fn locale_data(&self) -> &crate::locale::LocaleData {
        self.data.get_or_init(|| crate::locale::LocaleData::from_i18n(self))
    }

    pub fn is_translated(&self) -> bool {
        !self.domains.is_empty()
    }

    /// `__( $text, $domain )`.
    pub fn gettext(&self, domain: &str, text: &str) -> String {
        self.domains.get(domain).and_then(|c| c.get(None, text)).unwrap_or(text).to_owned()
    }

    /// `_x( $text, $context, $domain )`.
    pub fn pgettext(&self, domain: &str, context: &str, text: &str) -> String {
        self.domains.get(domain).and_then(|c| c.get(Some(context), text)).unwrap_or(text).to_owned()
    }

    /// `_n( $single, $plural, $number, $domain )`.
    pub fn ngettext(&self, domain: &str, singular: &str, plural: &str, n: i64) -> String {
        let fallback = if n == 1 { singular } else { plural };
        self.domains.get(domain).and_then(|c| c.get_plural(None, singular, n)).unwrap_or(fallback).to_owned()
    }

    /// `_nx( $single, $plural, $number, $context, $domain )`.
    pub fn npgettext(&self, domain: &str, context: &str, singular: &str, plural: &str, n: i64) -> String {
        let fallback = if n == 1 { singular } else { plural };
        self.domains.get(domain).and_then(|c| c.get_plural(Some(context), singular, n)).unwrap_or(fallback).to_owned()
    }
}

/// PHP's `sprintf` for `%s`, `%d` and positional `%1$s`; `%%` is a percent sign. Other
/// conversions are copied as they are.
pub fn sprintf(template: &str, args: &[&str]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0usize;
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'%') {
            chars.next();
            out.push('%');
            continue;
        }
        let mut digits = String::new();
        while let Some(&d) = chars.peek().filter(|d| d.is_ascii_digit()) {
            digits.push(d);
            chars.next();
        }
        let position = if chars.peek() == Some(&'$') && !digits.is_empty() {
            chars.next();
            digits.parse::<usize>().ok().map(|n| n.saturating_sub(1))
        } else {
            None
        };
        match chars.peek() {
            Some('s' | 'd') => {
                chars.next();
                let index = position.unwrap_or_else(|| {
                    next += 1;
                    next - 1
                });
                out.push_str(args.get(index).copied().unwrap_or(""));
            }
            _ => {
                out.push('%');
                out.push_str(&digits);
                if position.is_some() {
                    out.push('$');
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprintf_fills_plain_and_positional_arguments() {
        assert_eq!(sprintf("%s response", &["3"]), "3 response");
        assert_eq!(sprintf("%2$s von %1$s", &["a", "b"]), "b von a");
        assert_eq!(sprintf("100%% %s", &["x"]), "100% x");
        assert_eq!(sprintf("%s %s", &["a"]), "a ");
    }

    /// Builds a little-endian `.mo` file from `(msgid, msgstr)` pairs (sorted by the caller).
    fn mo(pairs: &[(&str, &str)]) -> Vec<u8> {
        let n = pairs.len() as u32;
        let header_len = 28u32;
        let tables = header_len + n * 16;
        let mut strings = Vec::new();
        let mut originals = Vec::new();
        let mut translations = Vec::new();
        for (original, _) in pairs {
            originals.push((original.len() as u32, tables + strings.len() as u32));
            strings.extend_from_slice(original.as_bytes());
            strings.push(0);
        }
        for (_, translation) in pairs {
            translations.push((translation.len() as u32, tables + strings.len() as u32));
            strings.extend_from_slice(translation.as_bytes());
            strings.push(0);
        }
        let mut out = Vec::new();
        for v in [0x9504_12deu32, 0, n, header_len, header_len + n * 8, 0, 0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for (len, off) in originals.iter().chain(translations.iter()) {
            out.extend_from_slice(&len.to_le_bytes());
            out.extend_from_slice(&off.to_le_bytes());
        }
        out.extend_from_slice(&strings);
        out
    }

    fn catalog() -> Catalog {
        let bytes = mo(&[
            ("", "Plural-Forms: nplurals=2; plural=n != 1;\n"),
            ("Comments", "Kommentare"),
            ("Search\u{4}label", "Suche"),
            ("One comment\0%s comments", "Ein Kommentar\0%s Kommentare"),
        ]);
        Catalog::parse(&bytes).expect("valid mo")
    }

    #[test]
    fn strings_contexts_and_plurals_are_found() {
        let c = catalog();
        assert_eq!(c.get(None, "Comments"), Some("Kommentare"));
        assert_eq!(c.get(Some("Search"), "label"), Some("Suche"));
        assert_eq!(c.get(None, "Missing"), None);
        assert_eq!(c.get_plural(None, "One comment", 1), Some("Ein Kommentar"));
        assert_eq!(c.get_plural(None, "One comment", 3), Some("%s Kommentare"));
    }

    #[test]
    fn plural_expressions_are_evaluated() {
        let eval = |expr: &str, n: i64| ExprParser { src: expr.as_bytes(), pos: 0 }.ternary().expect("parses").eval(n);
        assert_eq!(eval("n != 1", 1), 0);
        assert_eq!(eval("n != 1", 2), 1);
        // Polish.
        let polish = "(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2)";
        assert_eq!([1, 2, 5, 22, 25, 112].map(|n| eval(polish, n)), [0, 1, 2, 1, 2, 2]);
        // French.
        assert_eq!([0, 1, 2].map(|n| eval("n > 1", n)), [0, 0, 1]);
    }

    #[test]
    fn untranslated_text_falls_back_to_english() {
        let mut i18n = I18n { locale: "de_DE".into(), ..I18n::default() };
        i18n.domains.insert("default".into(), Arc::new(catalog()));
        assert_eq!(i18n.gettext("default", "Comments"), "Kommentare");
        assert_eq!(i18n.gettext("default", "Other"), "Other");
        assert_eq!(i18n.gettext("unknown-domain", "Comments"), "Comments");
        assert_eq!(i18n.ngettext("default", "One comment", "%s comments", 1), "Ein Kommentar");
        assert_eq!(I18n::default().ngettext("default", "One", "Many", 2), "Many");
    }

    #[test]
    fn broken_files_are_rejected() {
        assert!(Catalog::parse(b"not a mo file").is_none());
        assert!(Catalog::parse(&[]).is_none());
    }
}
