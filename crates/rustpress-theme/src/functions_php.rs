//! What a block theme's `functions.php` can contribute without running PHP: the
//! `inline_style` of `register_block_style()` calls. The calls are read as literals; anything
//! computed is ignored.

/// The backslash, which PHP's single-quoted strings use to escape `'` and itself.
fn backslash() -> char {
    char::from(92u8)
}

struct Cursor<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Cursor<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn skip_space(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.at += c.len_utf8();
            } else if self.rest().starts_with("//") || self.rest().starts_with('#') {
                self.at += self.rest().find('\n').unwrap_or(self.rest().len());
            } else if self.rest().starts_with("/*") {
                self.at += self.rest().find("*/").map_or(self.rest().len(), |e| e + 2);
            } else {
                break;
            }
        }
    }

    fn eat(&mut self, token: &str) -> bool {
        self.skip_space();
        if self.rest().starts_with(token) {
            self.at += token.len();
            true
        } else {
            false
        }
    }

    /// A single-quoted PHP string: only `\'` and a doubled backslash are escapes.
    fn single_quoted(&mut self) -> Option<String> {
        self.skip_space();
        if !self.rest().starts_with('\'') {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        let mut chars = self.rest().char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c == backslash() {
                match chars.peek() {
                    Some(&(_, next)) if next == '\'' || next == backslash() => {
                        out.push(next);
                        chars.next();
                    }
                    _ => out.push(c),
                }
            } else if c == '\'' {
                self.at += i + 1;
                return Some(out);
            } else {
                out.push(c);
            }
        }
        // No closing quote: the rest of the file is the string, so the callers cannot loop on the same spot.
        self.at = self.text.len();
        None
    }

    /// A double-quoted PHP string without variables; `None` if it interpolates.
    fn double_quoted(&mut self) -> Option<String> {
        self.skip_space();
        if !self.rest().starts_with('"') {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        let mut chars = self.rest().char_indices();
        while let Some((i, c)) = chars.next() {
            if c == backslash() {
                match chars.next() {
                    Some((_, 'n')) => out.push('\n'),
                    Some((_, 't')) => out.push('\t'),
                    Some((_, 'r')) => out.push('\r'),
                    Some((_, next)) if next == '"' || next == '$' || next == backslash() => out.push(next),
                    Some((_, next)) => {
                        out.push(c);
                        out.push(next);
                    }
                    None => return None,
                }
            } else if c == '"' {
                self.at += i + 1;
                return Some(out);
            } else if c == '$' {
                return None;
            } else {
                out.push(c);
            }
        }
        // No closing quote: the rest of the file is the string, so the callers cannot loop on the same spot.
        self.at = self.text.len();
        None
    }

    /// Any plain string literal.
    fn string(&mut self) -> Option<String> {
        self.skip_space();
        if self.rest().starts_with('"') { self.double_quoted() } else { self.single_quoted() }
    }

    /// Skips a value that is not a plain string: up to the next top-level `,` or closing bracket.
    fn skip_value(&mut self) {
        let mut depth = 0i32;
        while let Some(c) = self.peek() {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' if depth == 0 => return,
                ')' | ']' => depth -= 1,
                ',' if depth == 0 => return,
                '\'' => {
                    let _ = self.single_quoted();
                    continue;
                }
                _ => {}
            }
            self.at += c.len_utf8();
        }
    }
}

/// `(block name, inline_style)` for every `register_block_style()` call with a literal
/// `inline_style`, in source order.
pub fn block_style_inline_styles(php: &str) -> Vec<(String, String)> {
    block_style_inline_named(php).into_iter().map(|(block, _, css)| (block, css)).collect()
}

/// `(block name, style name, inline_style)` for every `register_block_style()` call with a literal `inline_style`;
/// the style name is empty when it is no literal.
pub fn block_style_inline_named(php: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = php[from..].find("register_block_style(") {
        let mut cursor = Cursor { text: php, at: from + found + "register_block_style(".len() };
        from = cursor.at;
        let Some(block) = cursor.single_quoted() else { continue };
        if !cursor.eat(",") || !(cursor.eat("array(") || cursor.eat("[")) {
            continue;
        }
        let (mut name, mut inline) = (String::new(), None);
        loop {
            cursor.skip_space();
            if cursor.peek().is_none() || cursor.eat(")") || cursor.eat("]") {
                break;
            }
            if cursor.eat(",") {
                continue;
            }
            let Some(key) = cursor.single_quoted() else { break };
            if !cursor.eat("=>") {
                break;
            }
            cursor.skip_space();
            let at = cursor.at;
            match cursor.string() {
                Some(value) if key == "inline_style" => inline = Some(value),
                Some(value) if key == "name" => name = value,
                Some(_) => {}
                None => {
                    cursor.at = at;
                    cursor.skip_value();
                }
            }
        }
        if let Some(css) = inline {
            out.push((block.clone(), name, css));
        }
        from = cursor.at;
    }
    out
}

/// The argument text of every `name( ... )` call: from after the `(` to the matching `)`.
fn call_arguments<'a>(php: &'a str, name: &str) -> Vec<&'a str> {
    let needle = format!("{name}(");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = php[from..].find(&needle) {
        let at = from + found;
        let start = at + needle.len();
        from = start;
        // Not `my_wp_enqueue_style(`, a method call, or a function definition.
        let before = php[..at].chars().next_back();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '>') || php[..at].trim_end().ends_with("function") {
            continue;
        }
        let mut cursor = Cursor { text: php, at: start };
        let mut depth = 0i32;
        while let Some(c) = cursor.peek() {
            match c {
                '(' => depth += 1,
                ')' if depth == 0 => break,
                ')' => depth -= 1,
                '\'' => {
                    let _ = cursor.single_quoted();
                    continue;
                }
                _ => {}
            }
            cursor.at += c.len_utf8();
        }
        out.push(&php[start..cursor.at.min(php.len())]);
        from = cursor.at;
    }
    out
}

/// The quoted string after `key =>` in an argument list.
fn string_after(arguments: &str, key: &str) -> Option<String> {
    let at = arguments.find(key)? + key.len();
    let mut cursor = Cursor { text: arguments, at };
    if !cursor.eat("=>") {
        return None;
    }
    cursor.string()
}

/// A block stylesheet from `wp_enqueue_block_style()`: loaded when the block is rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockStylesheet {
    pub block: String,
    pub handle: String,
    /// File below the theme directory (from `get_parent_theme_file_path()` / `get_theme_file_path()`).
    pub path: String,
}

/// `wp_enqueue_block_style( $block, array( 'handle' => ..., 'path' => get_..._file_path( 'x.css' ) ) )`.
pub fn block_stylesheets(php: &str) -> Vec<BlockStylesheet> {
    let mut out = Vec::new();
    for arguments in call_arguments(php, "wp_enqueue_block_style") {
        let mut cursor = Cursor { text: arguments, at: 0 };
        let Some(block) = cursor.single_quoted() else { continue };
        let Some(handle) = string_after(arguments, "'handle'") else { continue };
        let Some(at) = arguments.find("'path'") else { continue };
        let rest = &arguments[at..];
        let Some(open) = ["get_parent_theme_file_path(", "get_theme_file_path("].iter().find_map(|f| rest.find(f).map(|i| i + f.len())) else { continue };
        let mut path_cursor = Cursor { text: rest, at: open };
        let Some(path) = path_cursor.single_quoted() else { continue };
        out.push(BlockStylesheet { block, handle, path });
    }
    out
}

/// The handle under which the theme's `style.css` is enqueued (`wp_enqueue_style( 'x', get_stylesheet_uri() )`).
pub fn theme_stylesheet_handle(php: &str) -> Option<String> {
    call_arguments(php, "wp_enqueue_style").into_iter().find_map(|arguments| {
        let mut cursor = Cursor { text: arguments, at: 0 };
        let handle = cursor.single_quoted()?;
        (handle.ends_with("-style") || arguments.contains("style.css") || arguments.contains("get_stylesheet_uri(")).then_some(handle)
    })
}

/// The literal list a theme passes with `add_theme_support( 'feature', array( … ) )` (the post formats).
pub fn theme_support_list(php: &str, feature: &str) -> Option<Vec<String>> {
    let quoted = format!("'{feature}'");
    let arguments = call_arguments(php, "add_theme_support").into_iter().find(|a| a.trim_start().starts_with(&quoted))?;
    let rest = &arguments.trim_start()[quoted.len()..];
    Some(rest.split('\'').skip(1).step_by(2).map(str::to_owned).collect())
}

/// Whether the theme calls `add_theme_support( 'feature' )`.
pub fn adds_theme_support(php: &str, feature: &str) -> bool {
    let quoted = format!("'{feature}'");
    call_arguments(php, "add_theme_support").into_iter().any(|arguments| arguments.trim_start().starts_with(&quoted))
}

/// A source of block bindings the theme registers (`register_block_bindings_source()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingSource {
    pub name: String,
    pub label: String,
    /// The context of `_x()` around the label.
    pub label_context: Option<String>,
    pub uses_context: Option<Vec<String>>,
}

/// The `register_block_bindings_source()` calls with a literal name and label, in source order.
pub fn block_bindings_sources(php: &str) -> Vec<BindingSource> {
    let quoted = r"'((?:[^'\\]|\\.)*)'";
    let label_re = regex::Regex::new(&format!(r"'label'\s*=>\s*(_x|__)\(\s*{quoted}(?:\s*,\s*{quoted})?")).expect("valid pattern");
    let context_re = regex::Regex::new(r"'uses_context'\s*=>\s*(?:array\(|\[)([^\])]*)").expect("valid pattern");
    let string_re = regex::Regex::new(quoted).expect("valid pattern");
    let unescape = |s: &str| s.replace("\\'", "'").replace("\\\\", "\\");
    call_arguments(php, "register_block_bindings_source")
        .into_iter()
        .filter_map(|arguments| {
            let name = string_re.captures(arguments)?.get(1)?.as_str().to_owned();
            let label = label_re.captures(arguments)?;
            let label_context = if &label[1] == "_x" { label.get(3).map(|m| unescape(m.as_str())) } else { None };
            let uses_context = context_re.captures(arguments).map(|c| string_re.captures_iter(&c[1]).map(|s| unescape(&s[1])).collect());
            Some(BindingSource { name, label: unescape(&label[2]), label_context, uses_context })
        })
        .collect()
}

/// Whether `wp_style_add_data( $handle, 'path', ... )` lets WordPress inline the stylesheet.
pub fn theme_stylesheet_has_path(php: &str, handle: &str) -> bool {
    let quoted = format!("'{handle}'");
    call_arguments(php, "wp_style_add_data").into_iter().any(|arguments| arguments.contains(&quoted) && arguments.contains("'path'"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_styles_of_block_styles_are_read_as_literals() {
        let php = "<?php\nfunction styles() {\n\tregister_block_style(\n\t\t'core/list',\n\t\tarray(\n\t\t\t'name' => 'checkmark-list',\n\t\t\t'label' => __( 'Checkmark', 'x' ),\n\t\t\t'inline_style' => '\n\t\t\tul.is-style-checkmark-list { list-style-type: \"\\2713\"; }',\n\t\t)\n\t);\n}\n";
        let styles = block_style_inline_styles(php);
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].0, "core/list");
        // PHP keeps a backslash that does not escape a quote or another backslash.
        assert_eq!(styles[0].1, "\n\t\t\tul.is-style-checkmark-list { list-style-type: \"\\2713\"; }");
    }

    #[test]
    fn escaped_quotes_and_styles_without_inline_css() {
        let php = "register_block_style('core/quote', ['name' => 'x', 'inline_style' => 'a::before{content:\\'q\\'}']); register_block_style('core/code', array('name' => 'plain'));";
        let styles = block_style_inline_styles(php);
        assert_eq!(styles, vec![("core/quote".to_owned(), "a::before{content:'q'}".to_owned())]);
    }

    #[test]
    fn double_quoted_inline_styles_are_read_unless_they_interpolate() {
        let php = "register_block_style('core/heading', array('name' => 'a', 'inline_style' => \"x:before { content: ''; }\")); register_block_style('core/code', array('inline_style' => \"a { b: $c; }\"));";
        assert_eq!(block_style_inline_styles(php), vec![("core/heading".to_owned(), "x:before { content: ''; }".to_owned())]);
    }

    #[test]
    fn enqueued_block_and_theme_stylesheets() {
        let php = "function s() { wp_enqueue_block_style( 'core/button', array( 'handle' => 'tt-outline', 'src' => get_parent_theme_file_uri( 'assets/css/b.css' ), 'path' => get_parent_theme_file_path( 'assets/css/b.css' ) ) ); wp_enqueue_style( 'tt-style', get_parent_theme_file_uri( 'style.css' ), array(), '1' ); }";
        assert_eq!(block_stylesheets(php), vec![BlockStylesheet { block: "core/button".into(), handle: "tt-outline".into(), path: "assets/css/b.css".into() }]);
        assert_eq!(theme_stylesheet_handle(php).as_deref(), Some("tt-style"));
        assert_eq!(theme_stylesheet_handle("wp_enqueue_style( 'x', get_theme_file_uri( 'a.css' ) );"), None);
        assert!(block_stylesheets("function wp_enqueue_block_style( $a ) {}").is_empty());
    }

    #[test]
    fn malformed_calls_are_skipped() {
        assert!(block_style_inline_styles("register_block_style($name, $args);").is_empty());
        assert!(block_style_inline_styles("register_block_style('core/list'").is_empty());
    }
}
