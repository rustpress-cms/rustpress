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
        None
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
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = php[from..].find("register_block_style(") {
        let mut cursor = Cursor { text: php, at: from + found + "register_block_style(".len() };
        from = cursor.at;
        let Some(block) = cursor.single_quoted() else { continue };
        if !cursor.eat(",") || !(cursor.eat("array(") || cursor.eat("[")) {
            continue;
        }
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
            match cursor.single_quoted() {
                Some(value) if key == "inline_style" => out.push((block.clone(), value)),
                Some(_) => {}
                None => cursor.skip_value(),
            }
        }
        from = cursor.at;
    }
    out
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
    fn malformed_calls_are_skipped() {
        assert!(block_style_inline_styles("register_block_style($name, $args);").is_empty());
        assert!(block_style_inline_styles("register_block_style('core/list'").is_empty());
    }
}
