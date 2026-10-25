//! Reads PHP values from source text without running it: strings, numbers, booleans, `null`,
//! nested `array()` / `[]` literals, the translation functions (`__( 'text' )` is its text),
//! `.` concatenation of strings and variables that were assigned a literal earlier. Everything
//! else (function calls, constants, closures) is [`Lit::Unknown`]; the parser skips over it and
//! carries on, so one computed value does not hide its neighbours.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    /// `(key, value)`; the key is `None` for list items.
    Array(Vec<(Option<Lit>, Lit)>),
    Unknown,
}

impl Lit {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Lit::Str(s) => Some(s),
            _ => None,
        }
    }

    /// PHP truthiness of what is known; `None` for unknown values.
    pub fn truthy(&self) -> Option<bool> {
        match self {
            Lit::Null => Some(false),
            Lit::Bool(b) => Some(*b),
            Lit::Int(i) => Some(*i != 0),
            Lit::Float(f) => Some(*f != 0.0),
            Lit::Str(s) => Some(!s.is_empty() && s != "0"),
            Lit::Array(items) => Some(!items.is_empty()),
            Lit::Unknown => None,
        }
    }

    /// The value of a string key of an array.
    pub fn get(&self, key: &str) -> Option<&Lit> {
        match self {
            Lit::Array(items) => items.iter().find(|(k, _)| k.as_ref().and_then(Lit::as_str) == Some(key)).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string items of a list (`array( 'title', 'editor' )`); other items are left out.
    pub fn strings(&self) -> Vec<String> {
        match self {
            Lit::Array(items) => items.iter().filter_map(|(_, v)| v.as_str().map(str::to_owned)).collect(),
            _ => Vec::new(),
        }
    }
}

/// The variables assigned a literal so far, by name without `$`.
pub type Vars = HashMap<String, Lit>;

pub struct Parser<'a> {
    text: &'a str,
    pub at: usize,
    pub vars: &'a Vars,
}

/// Functions whose first argument is the text they translate.
const TRANSLATION: [&str; 7] = ["__", "_x", "esc_html__", "esc_attr__", "esc_html_x", "esc_attr_x", "_e"];

impl<'a> Parser<'a> {
    pub fn new(text: &'a str, at: usize, vars: &'a Vars) -> Self {
        Self { text, at, vars }
    }

    fn rest(&self) -> &'a str {
        &self.text[self.at..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    pub fn skip_space(&mut self) {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            self.at += rest.len() - trimmed.len();
            let rest = self.rest();
            if rest.starts_with("//") || rest.starts_with('#') {
                self.at += rest.find('\n').unwrap_or(rest.len());
            } else if rest.starts_with("/*") {
                self.at += rest.find("*/").map_or(rest.len(), |e| e + 2);
            } else {
                break;
            }
        }
    }

    pub fn eat(&mut self, token: &str) -> bool {
        self.skip_space();
        if self.rest().starts_with(token) {
            self.at += token.len();
            true
        } else {
            false
        }
    }

    /// A value, including `.` concatenation, followed by whatever stays of the expression.
    pub fn expr(&mut self) -> Lit {
        let mut value = self.primary();
        loop {
            self.skip_space();
            let rest = self.rest();
            if rest.starts_with('.') && !rest.starts_with("..") && !rest.starts_with(".=") {
                self.at += 1;
                let next = self.primary();
                value = match (&value, &next) {
                    (Lit::Str(a), Lit::Str(b)) => Lit::Str(format!("{a}{b}")),
                    _ => Lit::Unknown,
                };
                continue;
            }
            break;
        }
        // Operators, ternaries and the like: the whole expression is unknown.
        self.skip_space();
        if !matches!(self.peek(), None | Some(',' | ')' | ']' | ';' | '}')) && !self.rest().starts_with("=>") {
            self.skip_value();
            return Lit::Unknown;
        }
        value
    }

    fn primary(&mut self) -> Lit {
        self.skip_space();
        let Some(c) = self.peek() else { return Lit::Unknown };
        match c {
            '\'' => self.single_quoted().map_or(Lit::Unknown, Lit::Str),
            '"' => self.double_quoted().map_or(Lit::Unknown, Lit::Str),
            '[' => {
                self.at += 1;
                self.array_items(']')
            }
            '-' | '0'..='9' => self.number(),
            '$' => {
                self.at += 1;
                let name = self.identifier();
                // `$a['x']` and `$this->x` are not simple variables.
                self.skip_space();
                if self.rest().starts_with('[') || self.rest().starts_with("->") || self.rest().starts_with("::") {
                    return Lit::Unknown;
                }
                self.vars.get(&name).cloned().unwrap_or(Lit::Unknown)
            }
            c if c.is_alphabetic() || c == '_' || c == '\\' => self.word(),
            _ => Lit::Unknown,
        }
    }

    fn identifier(&mut self) -> String {
        let rest = self.rest();
        let len = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
        self.at += len;
        rest[..len].to_owned()
    }

    fn number(&mut self) -> Lit {
        let rest = self.rest();
        let len = rest.char_indices().find(|&(i, c)| !(c.is_ascii_digit() || c == '.' || (i == 0 && c == '-'))).map_or(rest.len(), |(i, _)| i);
        let text = &rest[..len];
        if text == "-" || text.is_empty() {
            return Lit::Unknown;
        }
        self.at += len;
        if let Ok(i) = text.parse::<i64>() {
            Lit::Int(i)
        } else if let Ok(f) = text.parse::<f64>() {
            Lit::Float(f)
        } else {
            Lit::Unknown
        }
    }

    /// A bare word: `true`, `array(`, a translation call, or something that is not understood.
    fn word(&mut self) -> Lit {
        let start = self.at;
        let rest = self.rest();
        let len = rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '\\')).unwrap_or(rest.len());
        let word = rest[..len].trim_start_matches('\\');
        self.at += len;
        let lower = word.to_ascii_lowercase();
        self.skip_space();
        let is_call = self.rest().starts_with('(');
        match lower.as_str() {
            "true" if !is_call => return Lit::Bool(true),
            "false" if !is_call => return Lit::Bool(false),
            "null" if !is_call => return Lit::Null,
            "array" if is_call => {
                self.at += 1;
                return self.array_items(')');
            }
            _ => {}
        }
        if is_call && TRANSLATION.contains(&word) {
            self.at += 1;
            let first = self.expr();
            self.skip_call_rest();
            return first;
        }
        if is_call {
            self.at += 1;
            self.skip_call_rest();
            return Lit::Unknown;
        }
        // A constant or `Class::CONSTANT`: leave it where it is for the caller to skip.
        if self.rest().starts_with("::") {
            self.skip_value();
        }
        let _ = start;
        Lit::Unknown
    }

    /// After a call's first argument: skips to the matching `)`.
    fn skip_call_rest(&mut self) {
        let mut depth = 0i32;
        while let Some(c) = self.peek() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' if depth == 0 => {
                    self.at += 1;
                    return;
                }
                ')' | ']' | '}' => depth -= 1,
                '\'' => {
                    let _ = self.single_quoted();
                    continue;
                }
                '"' => {
                    let _ = self.double_quoted();
                    continue;
                }
                _ => {}
            }
            self.at += c.len_utf8();
        }
    }

    /// Items up to the closing bracket, which is consumed.
    fn array_items(&mut self, close: char) -> Lit {
        let mut items = Vec::new();
        loop {
            self.skip_space();
            if self.peek().is_none() {
                return Lit::Unknown;
            }
            if self.peek() == Some(close) {
                self.at += 1;
                return Lit::Array(items);
            }
            if self.rest().starts_with("...") {
                self.at += 3;
                let _ = self.expr();
                items.push((None, Lit::Unknown));
            } else {
                let first = self.expr();
                self.skip_space();
                if self.rest().starts_with("=>") {
                    self.at += 2;
                    let value = self.expr();
                    items.push((Some(first), value));
                } else {
                    items.push((None, first));
                }
            }
            self.skip_space();
            match self.peek() {
                Some(',') => self.at += 1,
                Some(c) if c == close => {}
                // Not a list we can follow: give up on the rest of it.
                _ => {
                    self.skip_value();
                    if self.peek() == Some(',') {
                        self.at += 1;
                    } else if self.peek() != Some(close) {
                        return Lit::Unknown;
                    }
                }
            }
        }
    }

    /// Skips up to the next top-level `,` or closing bracket.
    fn skip_value(&mut self) {
        let mut depth = 0i32;
        while let Some(c) = self.peek() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' if depth == 0 => return,
                ')' | ']' | '}' => depth -= 1,
                ',' | ';' if depth == 0 => return,
                '\'' => {
                    let _ = self.single_quoted();
                    continue;
                }
                '"' => {
                    let _ = self.double_quoted();
                    continue;
                }
                _ => {}
            }
            self.at += c.len_utf8();
        }
    }

    /// A single-quoted string: only `\'` and `\\` are escapes.
    fn single_quoted(&mut self) -> Option<String> {
        self.skip_space();
        if !self.rest().starts_with('\'') {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        let mut chars = self.rest().char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            if c == '\\' {
                match chars.peek() {
                    Some(&(_, next)) if next == '\'' || next == '\\' => {
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
        self.at = self.text.len();
        None
    }

    /// A double-quoted string; `None` (the string is consumed) if it interpolates.
    fn double_quoted(&mut self) -> Option<String> {
        self.skip_space();
        if !self.rest().starts_with('"') {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        let mut interpolates = false;
        let mut chars = self.rest().char_indices();
        while let Some((i, c)) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some((_, 'n')) => out.push('\n'),
                    Some((_, 't')) => out.push('\t'),
                    Some((_, 'r')) => out.push('\r'),
                    Some((_, next)) if next == '"' || next == '$' || next == '\\' => out.push(next),
                    Some((_, next)) => {
                        out.push(c);
                        out.push(next);
                    }
                    None => break,
                }
            } else if c == '"' {
                self.at += i + 1;
                return if interpolates { None } else { Some(out) };
            } else {
                if c == '$' || (c == '{' && self.rest()[i + 1..].starts_with('$')) {
                    interpolates = true;
                }
                out.push(c);
            }
        }
        self.at = self.text.len();
        None
    }
}

/// Parses one value at the start of `text`.
pub fn parse(text: &str) -> Lit {
    let vars = Vars::new();
    Parser::new(text, 0, &vars).expr()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_and_translations() {
        assert_eq!(parse("'a\\'b'"), Lit::Str("a'b".into()));
        assert_eq!(parse("__( 'Books', 'my-domain' )"), Lit::Str("Books".into()));
        assert_eq!(parse("_x( 'Books', 'post type general name', 'd' )"), Lit::Str("Books".into()));
        assert_eq!(parse("'a' . 'b' . __('c')"), Lit::Str("abc".into()));
        assert_eq!(parse("true"), Lit::Bool(true));
        assert_eq!(parse("-5"), Lit::Int(-5));
        assert_eq!(parse("\"x $y\""), Lit::Unknown);
    }

    #[test]
    fn arrays_keep_going_after_unknown_values() {
        let value = parse("array( 'label' => 'B', 'rewrite' => array( 'slug' => 'buecher' ), 'x' => some_call( 1, array( 2 ) ), 'public' => true, 'supports' => ['title', 'editor'], )");
        assert_eq!(value.get("label").and_then(Lit::as_str), Some("B"));
        assert_eq!(value.get("rewrite").and_then(|r| r.get("slug")).and_then(Lit::as_str), Some("buecher"));
        assert_eq!(value.get("x"), Some(&Lit::Unknown));
        assert_eq!(value.get("public").and_then(Lit::truthy), Some(true));
        assert_eq!(value.get("supports").map(Lit::strings), Some(vec!["title".to_owned(), "editor".to_owned()]));
    }

    #[test]
    fn variables_resolve_to_earlier_literals() {
        let mut vars = Vars::new();
        vars.insert("labels".into(), parse("array( 'name' => 'Books' )"));
        let value = Parser::new("array( 'labels' => $labels, 'other' => $missing )", 0, &vars).expr();
        assert_eq!(value.get("labels").and_then(|l| l.get("name")).and_then(Lit::as_str), Some("Books"));
        assert_eq!(value.get("other"), Some(&Lit::Unknown));
    }
}
