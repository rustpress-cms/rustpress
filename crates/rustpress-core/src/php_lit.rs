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

/// What a caller knows about the PHP environment: functions it can evaluate and object properties
/// (`$scripts->base_url` is `props["scripts->base_url"]`).
pub type HookFunction = Box<dyn Fn(&[Lit]) -> Option<Lit> + Send + Sync>;

#[derive(Default)]
pub struct Hooks {
    pub functions: HashMap<String, HookFunction>,
    pub props: HashMap<String, Lit>,
}

/// Translates a message id, with the context of `_x()` when there is one.
pub type Translator<'a> = &'a dyn Fn(&str, Option<&str>) -> String;

pub struct Parser<'a> {
    text: &'a str,
    pub at: usize,
    pub vars: &'a Vars,
    hooks: Option<&'a Hooks>,
    translator: Option<Translator<'a>>,
    /// Inside the middle part of `? :`, where a `:` ends the expression.
    colon_stops: u32,
}

/// Functions whose first argument is the text they translate.
const TRANSLATION: [&str; 7] = ["__", "_x", "esc_html__", "esc_attr__", "esc_html_x", "esc_attr_x", "_e"];

impl<'a> Parser<'a> {
    pub fn new(text: &'a str, at: usize, vars: &'a Vars) -> Self {
        Self { text, at, vars, hooks: None, translator: None, colon_stops: 0 }
    }

    /// A parser whose translation calls return the translation instead of the message id.
    pub fn with_translator(mut self, translator: Translator<'a>) -> Self {
        self.translator = Some(translator);
        self
    }

    /// A parser that also evaluates the functions and properties of `hooks`.
    pub fn with_hooks(mut self, hooks: &'a Hooks) -> Self {
        self.hooks = Some(hooks);
        self
    }

    /// The arguments of a call whose `(` was consumed, up to and including the `)`.
    pub fn call_args(&mut self) -> Vec<Lit> {
        let mut args = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                None => return args,
                Some(')') => {
                    self.at += 1;
                    return args;
                }
                Some(',') => {
                    self.at += 1;
                    continue;
                }
                _ => {}
            }
            let before = self.at;
            args.push(self.expr());
            if self.at == before {
                // Not something we can read: skip to the end of the call.
                self.skip_call_rest();
                return args;
            }
        }
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
                // Numbers join as their PHP string form.
                let text = |lit: &Lit| match lit {
                    Lit::Str(s) => Some(s.clone()),
                    Lit::Int(i) => Some(i.to_string()),
                    Lit::Float(f) => Some(f.to_string()),
                    _ => None,
                };
                value = match (text(&value), text(&next)) {
                    (Some(a), Some(b)) => Lit::Str(format!("{a}{b}")),
                    _ => Lit::Unknown,
                };
                continue;
            }
            break;
        }
        // `condition ? a : b` with a condition that is known.
        self.skip_space();
        if self.rest().starts_with('?') && !self.rest().starts_with("??") && !self.rest().starts_with("?->") {
            self.at += 1;
            self.colon_stops += 1;
            let then = self.expr();
            self.colon_stops -= 1;
            self.skip_space();
            if self.rest().starts_with(':') && !self.rest().starts_with("::") {
                self.at += 1;
            }
            let otherwise = self.expr();
            return match value.truthy() {
                Some(true) => then,
                Some(false) => otherwise,
                None => Lit::Unknown,
            };
        }
        // Other operators and the like: the whole expression is unknown.
        let colon = self.colon_stops > 0 && self.rest().starts_with(':') && !self.rest().starts_with("::");
        if !matches!(self.peek(), None | Some(',' | ')' | ']' | ';' | '}')) && !colon && !self.rest().starts_with("=>") {
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
            '(' => {
                // A cast: `(int) _x( '3', … )`.
                let rest = self.rest();
                let Some(close) = rest.find(')') else { return Lit::Unknown };
                let kind = rest[1..close].trim().to_ascii_lowercase();
                if !matches!(kind.as_str(), "int" | "integer" | "string" | "bool" | "boolean") {
                    return Lit::Unknown;
                }
                self.at += close + 1;
                let value = self.primary();
                match (kind.as_str(), value) {
                    ("int" | "integer", Lit::Int(i)) => Lit::Int(i),
                    ("int" | "integer", Lit::Str(s)) => {
                        let s = s.trim_start();
                        let sign = usize::from(s.starts_with(['-', '+']));
                        let digits = s[sign..].bytes().take_while(u8::is_ascii_digit).count();
                        Lit::Int(s[..sign + digits].parse().unwrap_or(0))
                    }
                    ("int" | "integer", Lit::Bool(b)) => Lit::Int(i64::from(b)),
                    ("string", Lit::Str(s)) => Lit::Str(s),
                    ("string", Lit::Int(i)) => Lit::Str(i.to_string()),
                    ("bool" | "boolean", value) => value.truthy().map_or(Lit::Unknown, Lit::Bool),
                    _ => Lit::Unknown,
                }
            }
            '$' => {
                self.at += 1;
                let name = self.identifier();
                // `$a['x']` and `$this->x` are not simple variables.
                self.skip_space();
                if let Some(after) = self.rest().strip_prefix("->")
                    && let Some(hooks) = self.hooks
                {
                    let property: String = after.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    let followed = after.trim_start()[property.len()..].trim_start();
                    if let Some(value) = hooks.props.get(&format!("{name}->{property}")).filter(|_| !followed.starts_with('(') && !followed.starts_with('[')) {
                        self.at += self.rest().len() - after.trim_start().len() + property.len();
                        return value.clone();
                    }
                }
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
            if let Some(translate) = self.translator {
                let args = self.call_args();
                let Some(text) = args.first().and_then(Lit::as_str) else { return Lit::Unknown };
                let context = if word.ends_with("_x") { args.get(1).and_then(Lit::as_str) } else { None };
                return Lit::Str(translate(text, context));
            }
            let first = self.expr();
            self.skip_call_rest();
            return first;
        }
        // No filters are registered: `apply_filters( $name, $value, … )` is the value.
        if is_call && lower == "apply_filters" {
            self.at += 1;
            let args = self.call_args();
            return args.get(1).cloned().unwrap_or(Lit::Unknown);
        }
        if is_call {
            self.at += 1;
            if let Some(hooks) = self.hooks
                && let Some(function) = hooks.functions.get(word)
            {
                let args = self.call_args();
                return function(&args).unwrap_or(Lit::Unknown);
            }
            // `sprintf()` of known strings and numbers.
            if lower == "sprintf" {
                let args = self.call_args();
                let texts: Option<Vec<String>> = args
                    .iter()
                    .map(|a| match a {
                        Lit::Str(s) => Some(s.clone()),
                        Lit::Int(i) => Some(i.to_string()),
                        _ => None,
                    })
                    .collect();
                return match texts.as_deref() {
                    Some([format, rest @ ..]) => Lit::Str(crate::i18n::sprintf(format, &rest.iter().map(String::as_str).collect::<Vec<_>>())),
                    _ => Lit::Unknown,
                };
            }
            self.skip_call_rest();
            return Lit::Unknown;
        }
        if let Some(hooks) = self.hooks
            && let Some(value) = hooks.props.get(&format!("const::{word}"))
        {
            return value.clone();
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
                ':' if depth == 0 && self.colon_stops > 0 && !self.rest().starts_with("::") => return,
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

    /// A double-quoted string; `None` (the string is consumed) if it interpolates something unknown.
    /// `$name` and `{$name}` take the string a variable was assigned.
    fn double_quoted(&mut self) -> Option<String> {
        self.skip_space();
        if !self.rest().starts_with('"') {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        let mut unknown = false;
        let rest = self.rest();
        let mut chars = rest.char_indices().peekable();
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
                return if unknown { None } else { Some(out) };
            } else if c == '$' && chars.peek().is_some_and(|&(_, n)| n.is_alphabetic() || n == '_') {
                let mut name = String::new();
                while let Some(&(_, n)) = chars.peek() {
                    if n.is_alphanumeric() || n == '_' {
                        name.push(n);
                        chars.next();
                    } else {
                        break;
                    }
                }
                match self.vars.get(&name) {
                    Some(Lit::Str(value)) => out.push_str(value),
                    _ => unknown = true,
                }
            } else if c == '{' && rest[i + 1..].starts_with('$') {
                let close = rest[i..].find('}').map_or(rest.len(), |e| i + e);
                let inner = rest[i + 2..close].trim();
                match self.vars.get(inner) {
                    Some(Lit::Str(value)) if inner.chars().all(|ch| ch.is_alphanumeric() || ch == '_') => out.push_str(value),
                    _ => unknown = true,
                }
                while chars.peek().is_some_and(|&(j, _)| j <= close) {
                    chars.next();
                }
            } else {
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
    fn translator_and_sprintf() {
        let translate = |text: &str, context: Option<&str>| format!("{}{text}", context.map_or(String::new(), |c| format!("[{c}]")));
        let vars = Vars::new();
        let value = Parser::new("array( 'a' => _x( 'One', 'ctx' ), 'b' => sprintf( __( 'Words: %s' ), '{0}' ) )", 0, &vars).with_translator(&translate).expr();
        assert_eq!(value.get("a"), Some(&Lit::Str("[ctx]One".into())));
        assert_eq!(value.get("b"), Some(&Lit::Str("Words: {0}".into())));
    }

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
