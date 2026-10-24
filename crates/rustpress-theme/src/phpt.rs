//! A tiny evaluator for the PHP found in block theme pattern files.
//!
//! Patterns are HTML with `<?php ... ?>` islands that almost always only echo
//! translated strings and URLs: `esc_html_e( 'Text', 'domain' )`,
//! `echo esc_url( get_theme_file_uri( 'assets/a.png' ) )`, `printf(...)`.
//! This supports exactly that: string literals, `.` concatenation, a table of
//! known functions and `echo`/`print`/`printf` statements. Anything else is
//! reported as an error instead of being guessed at.

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

/// Values the helper functions need from the surrounding site.
pub trait PhpHost {
    fn template_directory_uri(&self) -> String;
    fn stylesheet_directory_uri(&self) -> String;
    fn home_url(&self, path: &str) -> String;
    fn site_url(&self, path: &str) -> String;
    /// `__( $text, $domain )` and friends; the default is "no translation".
    fn translate(&self, text: &str, _context: Option<&str>) -> String {
        text.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhpError {
    pub message: String,
    pub offset: usize,
}

impl std::fmt::Display for PhpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at offset {})", self.message, self.offset)
    }
}

impl std::error::Error for PhpError {}

/// Renders a PHP template file to its output.
pub fn render(source: &str, host: &dyn PhpHost) -> Result<String, PhpError> {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut base = 0usize;
    while let Some(open) = rest.find("<?") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let (code_start, echo_tag) = if after.starts_with("php") {
            (open + 5, false)
        } else if after.starts_with('=') {
            (open + 3, true)
        } else {
            return Err(PhpError { message: "unsupported PHP open tag".into(), offset: base + open });
        };
        let code_area = &rest[code_start..];
        let Some(close) = find_close_tag(code_area) else {
            // An unclosed trailing block runs to the end of the file.
            let code = code_area;
            run_block(code, echo_tag, host, &mut out, base + code_start)?;
            return Ok(out);
        };
        run_block(&code_area[..close], echo_tag, host, &mut out, base + code_start)?;
        let mut consumed = code_start + close + 2;
        // A newline directly after `?>` is swallowed, like PHP does.
        if rest[consumed..].starts_with("\r\n") {
            consumed += 2;
        } else if rest[consumed..].starts_with('\n') {
            consumed += 1;
        }
        base += consumed;
        rest = &rest[consumed..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Finds `?>` that is not inside a string or comment.
fn find_close_tag(code: &str) -> Option<usize> {
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' | b'"' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    if bytes[i] == b'?' && bytes.get(i + 1) == Some(&b'>') {
                        return Some(i);
                    }
                    i += 1;
                }
            }
            b'?' if bytes.get(i + 1) == Some(&b'>') => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn run_block(code: &str, echo_tag: bool, host: &dyn PhpHost, out: &mut String, offset: usize) -> Result<(), PhpError> {
    let mut parser = Parser { src: code, pos: 0, base: offset, host };
    parser.skip_ws();
    if echo_tag {
        let value = parser.expression()?;
        out.push_str(&value);
        parser.skip_ws();
        parser.eat(';');
        return Ok(());
    }
    while parser.pos < parser.src.len() {
        parser.skip_ws();
        if parser.pos >= parser.src.len() {
            break;
        }
        parser.statement(out)?;
    }
    Ok(())
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    base: usize,
    host: &'a dyn PhpHost,
}

impl Parser<'_> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T, PhpError> {
        Err(PhpError { message: message.into(), offset: self.base + self.pos })
    }

    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn skip_ws(&mut self) {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            self.pos += rest.len() - trimmed.len();
            let rest = self.rest();
            if rest.starts_with("/*") {
                match rest.find("*/") {
                    Some(end) => self.pos += end + 2,
                    None => self.pos = self.src.len(),
                }
            } else if rest.starts_with("//") || rest.starts_with('#') {
                match rest.find('\n') {
                    Some(end) => self.pos += end + 1,
                    None => self.pos = self.src.len(),
                }
            } else {
                return;
            }
        }
    }

    fn eat(&mut self, c: char) -> bool {
        self.skip_ws();
        if self.rest().starts_with(c) {
            self.pos += c.len_utf8();
            true
        } else {
            false
        }
    }

    fn identifier(&mut self) -> Option<String> {
        self.skip_ws();
        let rest = self.rest();
        let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '\\')).unwrap_or(rest.len());
        if end == 0 || rest.as_bytes()[0].is_ascii_digit() {
            return None;
        }
        let name = rest[..end].trim_start_matches('\\').to_owned();
        self.pos += end;
        Some(name)
    }

    fn statement(&mut self, out: &mut String) -> Result<(), PhpError> {
        self.skip_ws();
        let start = self.pos;
        let Some(name) = self.identifier() else {
            if self.eat(';') {
                return Ok(());
            }
            return self.err(format!("unsupported PHP statement: {}", self.rest().chars().take(30).collect::<String>()));
        };
        match name.as_str() {
            "echo" | "print" => {
                loop {
                    let v = self.expression()?;
                    out.push_str(&v);
                    if !self.eat(',') {
                        break;
                    }
                }
                self.eat(';');
                Ok(())
            }
            _ => {
                self.pos = start;
                // A function call statement: `esc_html_e( ... );`, `printf( ... );`
                let call = self.call_or_literal()?;
                if let Value::Printed(text) = call {
                    out.push_str(&text);
                }
                self.eat(';');
                Ok(())
            }
        }
    }

    fn expression(&mut self) -> Result<String, PhpError> {
        let mut text = self.term()?;
        while {
            self.skip_ws();
            self.rest().starts_with('.') && !self.rest().starts_with("..")
        } {
            self.pos += 1;
            text.push_str(&self.term()?);
        }
        Ok(text)
    }

    fn term(&mut self) -> Result<String, PhpError> {
        match self.call_or_literal()? {
            Value::Str(s) => Ok(s),
            Value::Printed(_) => Ok(String::new()),
        }
    }

    fn call_or_literal(&mut self) -> Result<Value, PhpError> {
        self.skip_ws();
        let rest = self.rest();
        if rest.starts_with('\'') || rest.starts_with('"') {
            return self.string_literal().map(Value::Str);
        }
        if rest.starts_with('(') {
            self.pos += 1;
            let v = self.expression()?;
            if !self.eat(')') {
                return self.err("missing ) in expression");
            }
            return Ok(Value::Str(v));
        }
        let Some(name) = self.identifier() else {
            return self.err(format!("unsupported PHP expression: {}", self.rest().chars().take(30).collect::<String>()));
        };
        if !self.eat('(') {
            return match name.as_str() {
                "true" | "TRUE" => Ok(Value::Str("1".into())),
                "false" | "FALSE" | "null" | "NULL" => Ok(Value::Str(String::new())),
                _ => self.err(format!("unsupported constant {name}")),
            };
        }
        let mut args = Vec::new();
        if !self.eat(')') {
            loop {
                args.push(self.expression()?);
                if self.eat(',') {
                    // trailing comma
                    if self.eat(')') {
                        break;
                    }
                    continue;
                }
                if !self.eat(')') {
                    return self.err("missing ) in function call");
                }
                break;
            }
        }
        self.apply(&name, &args)
    }

    fn string_literal(&mut self) -> Result<String, PhpError> {
        let quote = self.rest().chars().next().expect("quote");
        self.pos += 1;
        let mut out = String::new();
        let mut chars = self.rest().char_indices();
        let mut end = None;
        while let Some((i, c)) = chars.next() {
            if c == '\\' {
                let Some((_, next)) = chars.next() else { break };
                if quote == '\'' {
                    match next {
                        '\'' | '\\' => out.push(next),
                        other => {
                            out.push('\\');
                            out.push(other);
                        }
                    }
                } else {
                    match next {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        '"' | '\\' | '$' => out.push(next),
                        other => {
                            out.push('\\');
                            out.push(other);
                        }
                    }
                }
            } else if c == quote {
                end = Some(i);
                break;
            } else if c == '$' && quote == '"' {
                return self.err("variable interpolation is not supported");
            } else {
                out.push(c);
            }
        }
        match end {
            Some(i) => {
                self.pos += i + 1;
                Ok(out)
            }
            None => self.err("unterminated string"),
        }
    }

    fn apply(&mut self, name: &str, args: &[String]) -> Result<Value, PhpError> {
        let arg = |i: usize| args.get(i).map(String::as_str).unwrap_or("");
        let host = self.host;
        let string = |s: String| Ok(Value::Str(s));
        match name {
            "__" | "_e" | "esc_html__" | "esc_html_e" | "esc_attr__" | "esc_attr_e" => {
                let text = host.translate(arg(0), None);
                let value = match name {
                    "esc_html__" | "esc_html_e" => esc_html(&text),
                    "esc_attr__" | "esc_attr_e" => esc_attr(&text),
                    _ => text,
                };
                if name.ends_with("_e") { Ok(Value::Printed(value)) } else { string(value) }
            }
            "_x" | "_ex" | "esc_html_x" | "esc_attr_x" => {
                let text = host.translate(arg(0), Some(arg(1)));
                let value = match name {
                    "esc_html_x" => esc_html(&text),
                    "esc_attr_x" => esc_attr(&text),
                    _ => text,
                };
                if name == "_ex" { Ok(Value::Printed(value)) } else { string(value) }
            }
            "esc_url" | "esc_url_raw" => string(esc_url(arg(0))),
            "esc_html" => string(esc_html(arg(0))),
            "esc_attr" => string(esc_attr(arg(0))),
            "wp_kses_post" | "trim" | "strval" | "wp_kses_data" => string(arg(0).to_owned()),
            "strtolower" => string(arg(0).to_lowercase()),
            "strtoupper" => string(arg(0).to_uppercase()),
            "get_template_directory_uri" | "get_parent_theme_file_uri_base" => string(host.template_directory_uri()),
            "get_stylesheet_directory_uri" => string(host.stylesheet_directory_uri()),
            "get_theme_file_uri" | "get_parent_theme_file_uri" => {
                let base = if name == "get_parent_theme_file_uri" { host.template_directory_uri() } else { host.stylesheet_directory_uri() };
                let path = arg(0);
                string(if path.is_empty() { base } else { format!("{base}/{}", path.trim_start_matches('/')) })
            }
            "home_url" => string(host.home_url(arg(0))),
            "site_url" => string(host.site_url(arg(0))),
            "sprintf" | "printf" => {
                let formatted = php_sprintf(arg(0), &args[1.min(args.len())..]);
                if name == "printf" { Ok(Value::Printed(formatted)) } else { string(formatted) }
            }
            other => self.err(format!("unsupported PHP function {other}()")),
        }
    }
}

enum Value {
    Str(String),
    /// Output already produced by an `*_e` function or `printf`.
    Printed(String),
}

/// `sprintf` with `%s`, `%d`, `%1$s` and `%%`.
fn php_sprintf(format: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    let mut next_arg = 0;
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('%') => {
                chars.next();
                out.push('%');
            }
            Some(d) if d.is_ascii_digit() => {
                let mut number = String::new();
                while let Some(d) = chars.peek().filter(|d| d.is_ascii_digit()) {
                    number.push(*d);
                    chars.next();
                }
                if chars.peek() == Some(&'$') {
                    chars.next();
                    chars.next();
                    let index: usize = number.parse().unwrap_or(1);
                    out.push_str(args.get(index.saturating_sub(1)).map(String::as_str).unwrap_or(""));
                }
            }
            Some(_) => {
                chars.next();
                out.push_str(args.get(next_arg).map(String::as_str).unwrap_or(""));
                next_arg += 1;
            }
            None => out.push('%'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Host;
    impl PhpHost for Host {
        fn template_directory_uri(&self) -> String {
            "http://x/wp-content/themes/t".into()
        }
        fn stylesheet_directory_uri(&self) -> String {
            "http://x/wp-content/themes/t".into()
        }
        fn home_url(&self, path: &str) -> String {
            format!("http://x/{}", path.trim_start_matches('/'))
        }
        fn site_url(&self, path: &str) -> String {
            format!("http://x/{}", path.trim_start_matches('/'))
        }
    }

    fn run(src: &str) -> String {
        render(src, &Host).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn html_without_php_is_unchanged() {
        assert_eq!(run("<p>hi</p>\n"), "<p>hi</p>\n");
    }

    #[test]
    fn the_file_header_block_produces_no_output() {
        let src = "<?php\n/**\n * Title: X\n * Slug: t/x\n */\n\n?>\n<!-- wp:group /-->\n";
        assert_eq!(run(src), "<!-- wp:group /-->\n");
    }

    #[test]
    fn translation_helpers_escape_correctly() {
        assert_eq!(run("<p><?php esc_html_e( 'Tom & \"Jerry\"', 'td' ); ?></p>"), "<p>Tom &amp; &quot;Jerry&quot;</p>");
        assert_eq!(run("<i alt=\"<?php echo esc_attr__( 'a\"b', 'td' ); ?>\">"), "<i alt=\"a&quot;b\">");
        assert_eq!(run("<?php echo esc_html_x( 'Search', 'Search form label.', 'td' ); ?>"), "Search");
        assert_eq!(run("<?php echo esc_attr_x( 'Type here...', 'placeholder', 'td' ); ?>"), "Type here...");
        assert_eq!(run("<?= __( 'Raw & text', 'td' ) ?>"), "Raw & text");
    }

    #[test]
    fn escaped_quotes_inside_php_strings() {
        assert_eq!(run("<?php echo esc_html_x( 'doesn\\'t exist', '404', 'td' ); ?>"), "doesn&#039;t exist");
    }

    #[test]
    fn uri_helpers() {
        assert_eq!(run("<img src=\"<?php echo esc_url( get_template_directory_uri() ); ?>/a.webp\"/>"), "<img src=\"http://x/wp-content/themes/t/a.webp\"/>");
        assert_eq!(run("<?php echo esc_url( get_theme_file_uri( 'assets/b.png' ) ); ?>"), "http://x/wp-content/themes/t/assets/b.png");
        assert_eq!(run("<?php echo esc_url( home_url( '/' ) ); ?>"), "http://x/");
    }

    #[test]
    fn printf_with_concatenation_and_comments() {
        let src = "<p>\n<?php\nprintf(\n\t/* translators: %s: link */\n\tesc_html__( 'Designed with %s', 'td' ),\n\t'<a href=\"' . esc_url( __( 'https://wordpress.org', 'td' ) ) . '\" rel=\"nofollow\">WordPress</a>'\n);\n?>\n</p>";
        assert_eq!(run(src), "<p>\nDesigned with <a href=\"https://wordpress.org\" rel=\"nofollow\">WordPress</a></p>");
    }

    #[test]
    fn sprintf_positions() {
        assert_eq!(php_sprintf("%2$s-%1$s %% %s", &["a".into(), "b".into()]), "b-a % a");
    }

    #[test]
    fn unsupported_code_is_an_error() {
        assert!(render("<?php if ( $x ) : ?>a<?php endif; ?>", &Host).is_err());
        assert!(render("<?php echo $foo; ?>", &Host).is_err());
        assert!(render("<?php echo mystery(); ?>", &Host).is_err());
    }

    #[test]
    fn close_tag_in_a_string_does_not_end_the_block() {
        assert_eq!(run("<?php echo '?>'; ?>x"), "?>x");
    }
}
