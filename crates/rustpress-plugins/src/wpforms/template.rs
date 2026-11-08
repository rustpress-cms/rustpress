//! The e-mail templates of the plugin (`templates/emails/*.php`), run as `Templates::get_html()` runs them: the
//! template is read from the plugin's directory and its PHP is interpreted. The templates use a small part of PHP
//! (echo, `printf`, `require`, assignments, `if`/`else`/`endif` and a few WordPress and WPForms functions), and that
//! is what this interpreter knows; anything else is skipped. A copy of a template in the theme
//! (`Templates::get_theme_template_paths()`) is not looked for (see PLAN.md).

use std::path::{Path, PathBuf};

use rustpress_core::formatting::{esc_attr, esc_html, esc_url};

/// A PHP value of a template.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<(String, Val)>),
}

impl Val {
    pub fn str(s: impl Into<String>) -> Self {
        Val::Str(s.into())
    }

    /// `(string) $value`.
    fn to_str(&self) -> String {
        match self {
            Val::Null | Val::Bool(false) => String::new(),
            Val::Bool(true) => "1".into(),
            Val::Int(n) => n.to_string(),
            Val::Str(s) => s.clone(),
            Val::Arr(_) => "Array".into(),
        }
    }

    /// `! empty( $value )`.
    fn truthy(&self) -> bool {
        match self {
            Val::Null => false,
            Val::Bool(b) => *b,
            Val::Int(n) => *n != 0,
            Val::Str(s) => !s.is_empty() && s != "0",
            Val::Arr(a) => !a.is_empty(),
        }
    }

    fn index(&self, key: &str) -> Val {
        match self {
            Val::Arr(a) => a.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or(Val::Null),
            _ => Val::Null,
        }
    }
}

/// What the functions of a template print and return.
pub struct Site<'a> {
    /// `WPFORMS_PLUGIN_DIR` (the templates and the CSS files).
    pub plugin_dir: &'a Path,
    /// `__( $text, 'wpforms-lite' )`.
    pub tr: &'a (dyn Fn(&str) -> String + Sync),
    pub blogname: &'a str,
    pub charset: &'a str,
    /// `get_language_attributes()`.
    pub language_attributes: &'a str,
    pub rtl: bool,
    /// `home_url()`.
    pub home_url: &'a str,
}

/// `Templates::get_html( $template_name, $args, true )`: `''` when the template does not exist.
pub fn get_html(site: &Site<'_>, name: &str, args: Vec<(String, Val)>) -> String {
    let path = site.plugin_dir.join("templates").join(format!("{name}.php"));
    let Ok(source) = std::fs::read_to_string(&path) else { return String::new() };
    let mut run = Run { site, vars: args, out: String::new(), depth: 0 };
    run.source(&source);
    run.out
}

/// Whether `templates/{$name}.php` exists (`Templates::locate()`).
pub fn exists(site: &Site<'_>, name: &str) -> bool {
    site.plugin_dir.join("templates").join(format!("{name}.php")).is_file()
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Inline(String),
    /// `?>`, which ends a statement.
    Close,
    Str(Vec<Part>),
    Int(i64),
    Var(String),
    Ident(String),
    Punct(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Lit(String),
    Var(String),
}

const PUNCTS: &[&str] = &["===", "!==", "=>", "&&", "||", "==", "!=", "(", ")", "[", "]", ",", ";", ":", "?", "!", ".", "=", "{", "}"];

fn tokenize(source: &str) -> Vec<Tok> {
    let b = source.as_bytes();
    let mut toks = Vec::new();
    let mut i = 0;
    loop {
        // HTML up to the next `<?php`.
        let start = i;
        let open = source[i..].find("<?php").map(|p| i + p);
        let end = open.unwrap_or(b.len());
        if end > start {
            toks.push(Tok::Inline(source[start..end].to_owned()));
        }
        let Some(open) = open else { break };
        i = open + 5;
        // PHP code up to `?>`.
        loop {
            if i >= b.len() {
                return toks;
            }
            let c = b[i];
            if c.is_ascii_whitespace() {
                i += 1;
            } else if source[i..].starts_with("?>") {
                toks.push(Tok::Close);
                i += 2;
                // The newline right after `?>` belongs to the tag.
                if source[i..].starts_with("\r\n") {
                    i += 2;
                } else if source[i..].starts_with('\n') {
                    i += 1;
                }
                break;
            } else if source[i..].starts_with("//") || c == b'#' {
                while i < b.len() && b[i] != b'\n' && !source[i..].starts_with("?>") {
                    i += 1;
                }
            } else if source[i..].starts_with("/*") {
                i = source[i + 2..].find("*/").map_or(b.len(), |p| i + 2 + p + 2);
            } else if c == b'\'' {
                let mut s = String::new();
                i += 1;
                while i < b.len() && b[i] != b'\'' {
                    if b[i] == b'\\' && i + 1 < b.len() && (b[i + 1] == b'\'' || b[i + 1] == b'\\') {
                        i += 1;
                    }
                    let len = utf8_len(b[i]);
                    s.push_str(&source[i..i + len]);
                    i += len;
                }
                i += 1;
                toks.push(Tok::Str(vec![Part::Lit(s)]));
            } else if c == b'"' {
                let mut parts = Vec::new();
                let mut s = String::new();
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' && i + 1 < b.len() {
                        let esc = match b[i + 1] {
                            b'n' => Some('\n'),
                            b't' => Some('\t'),
                            b'r' => Some('\r'),
                            b'\\' => Some('\\'),
                            b'"' => Some('"'),
                            b'$' => Some('$'),
                            _ => None,
                        };
                        if let Some(e) = esc {
                            s.push(e);
                            i += 2;
                            continue;
                        }
                    }
                    let braced = source[i..].starts_with("{$");
                    if braced || (b[i] == b'$' && b.get(i + 1).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')) {
                        let from = if braced { i + 2 } else { i + 1 };
                        let mut j = from;
                        while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                            j += 1;
                        }
                        parts.push(Part::Lit(std::mem::take(&mut s)));
                        parts.push(Part::Var(source[from..j].to_owned()));
                        i = if braced && b.get(j) == Some(&b'}') { j + 1 } else { j };
                        continue;
                    }
                    let len = utf8_len(b[i]);
                    s.push_str(&source[i..i + len]);
                    i += len;
                }
                i += 1;
                parts.push(Part::Lit(s));
                toks.push(Tok::Str(parts));
            } else if c == b'$' {
                let mut j = i + 1;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                    j += 1;
                }
                toks.push(Tok::Var(source[i + 1..j].to_owned()));
                i = j;
            } else if c.is_ascii_digit() {
                let mut j = i;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                toks.push(Tok::Int(source[i..j].parse().unwrap_or(0)));
                i = j;
            } else if c.is_ascii_alphabetic() || c == b'_' || c == b'\\' {
                let mut j = i;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'\\') {
                    j += 1;
                }
                let name = source[i..j].trim_start_matches('\\');
                toks.push(Tok::Ident(name.rsplit('\\').next().unwrap_or(name).to_owned()));
                i = j;
            } else if let Some(p) = PUNCTS.iter().find(|p| source[i..].starts_with(**p)) {
                toks.push(Tok::Punct(p));
                i += p.len();
            } else {
                i += utf8_len(c);
            }
        }
    }
    toks
}

fn utf8_len(first: u8) -> usize {
    match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

#[derive(Debug, Clone)]
enum Expr {
    Str(Vec<Part>),
    Int(i64),
    Var(String, Vec<Expr>),
    Const(String),
    Call(String, Vec<Expr>),
    Array(Vec<(Option<Expr>, Expr)>),
    Not(Box<Expr>),
    Concat(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Same(Box<Expr>, Box<Expr>, bool),
    Ternary(Box<Expr>, Option<Box<Expr>>, Box<Expr>),
}

#[derive(Debug, Clone)]
enum Stmt {
    Inline(String),
    Echo(Vec<Expr>),
    Expr(Expr),
    Assign(String, Expr),
    Require(Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    Exit,
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat(&mut self, p: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Punct(q)) if *q == p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn is_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(n)) if n.eq_ignore_ascii_case(name))
    }

    fn end_statement(&mut self) {
        if !self.eat(";") && matches!(self.peek(), Some(Tok::Close)) {
            self.pos += 1;
        }
    }

    /// Statements up to one of the words in `until` (not consumed) or the end.
    fn block(&mut self, until: &[&str]) -> Vec<Stmt> {
        let mut out = Vec::new();
        while self.pos < self.toks.len() {
            if until.iter().any(|u| if *u == "}" { matches!(self.peek(), Some(Tok::Punct("}"))) } else { self.is_ident(u) }) {
                break;
            }
            if let Some(s) = self.statement() {
                out.push(s);
            }
        }
        out
    }

    fn statement(&mut self) -> Option<Stmt> {
        match self.peek()?.clone() {
            Tok::Inline(s) => {
                self.pos += 1;
                Some(Stmt::Inline(s))
            }
            Tok::Close | Tok::Punct(";") => {
                self.pos += 1;
                None
            }
            Tok::Ident(name) => {
                let lower = name.to_ascii_lowercase();
                match lower.as_str() {
                    "echo" | "print" => {
                        self.pos += 1;
                        let mut list = vec![self.expr()];
                        while self.eat(",") {
                            list.push(self.expr());
                        }
                        self.end_statement();
                        Some(Stmt::Echo(list))
                    }
                    "require" | "require_once" | "include" | "include_once" => {
                        self.pos += 1;
                        let e = self.expr();
                        self.end_statement();
                        Some(Stmt::Require(e))
                    }
                    "exit" | "die" => {
                        self.pos += 1;
                        if self.eat("(") {
                            while self.pos < self.toks.len() && !self.eat(")") {
                                self.pos += 1;
                            }
                        }
                        self.end_statement();
                        Some(Stmt::Exit)
                    }
                    "if" => {
                        self.pos += 1;
                        self.eat("(");
                        let cond = self.expr();
                        self.eat(")");
                        if self.eat("{") {
                            let then = self.block(&["}"]);
                            self.eat("}");
                            let otherwise = if self.is_ident("else") {
                                self.pos += 1;
                                if self.eat("{") {
                                    let b = self.block(&["}"]);
                                    self.eat("}");
                                    b
                                } else {
                                    self.statement().into_iter().collect()
                                }
                            } else {
                                Vec::new()
                            };
                            return Some(Stmt::If(cond, then, otherwise));
                        }
                        self.eat(":");
                        let then = self.block(&["else", "endif"]);
                        let otherwise = if self.is_ident("else") {
                            self.pos += 1;
                            self.eat(":");
                            self.block(&["endif"])
                        } else {
                            Vec::new()
                        };
                        if self.is_ident("endif") {
                            self.pos += 1;
                            self.end_statement();
                        }
                        Some(Stmt::If(cond, then, otherwise))
                    }
                    _ => {
                        let e = self.expr();
                        self.end_statement();
                        Some(Stmt::Expr(e))
                    }
                }
            }
            Tok::Var(name) if matches!(self.toks.get(self.pos + 1), Some(Tok::Punct("="))) => {
                self.pos += 2;
                let e = self.expr();
                self.end_statement();
                Some(Stmt::Assign(name, e))
            }
            _ => {
                let start = self.pos;
                let e = self.expr();
                if self.pos == start {
                    self.pos += 1;
                    return None;
                }
                self.end_statement();
                Some(Stmt::Expr(e))
            }
        }
    }

    fn expr(&mut self) -> Expr {
        let cond = self.or();
        if self.eat("?") {
            if self.eat(":") {
                let b = self.expr();
                return Expr::Ternary(Box::new(cond), None, Box::new(b));
            }
            let a = self.expr();
            self.eat(":");
            let b = self.expr();
            return Expr::Ternary(Box::new(cond), Some(Box::new(a)), Box::new(b));
        }
        cond
    }

    fn or(&mut self) -> Expr {
        let mut left = self.and();
        while self.eat("||") {
            left = Expr::Or(Box::new(left), Box::new(self.and()));
        }
        left
    }

    fn and(&mut self) -> Expr {
        let mut left = self.same();
        while self.eat("&&") {
            left = Expr::And(Box::new(left), Box::new(self.same()));
        }
        left
    }

    fn same(&mut self) -> Expr {
        let left = self.concat();
        if self.eat("===") || self.eat("==") {
            return Expr::Same(Box::new(left), Box::new(self.concat()), true);
        }
        if self.eat("!==") || self.eat("!=") {
            return Expr::Same(Box::new(left), Box::new(self.concat()), false);
        }
        left
    }

    fn concat(&mut self) -> Expr {
        let mut left = self.unary();
        while self.eat(".") {
            left = Expr::Concat(Box::new(left), Box::new(self.unary()));
        }
        left
    }

    fn unary(&mut self) -> Expr {
        if self.eat("!") {
            return Expr::Not(Box::new(self.unary()));
        }
        self.primary()
    }

    fn args(&mut self, close: &str) -> Vec<Expr> {
        let mut out = Vec::new();
        while self.pos < self.toks.len() && !self.eat(close) {
            out.push(self.expr());
            if !self.eat(",") {
                self.eat(close);
                break;
            }
        }
        out
    }

    fn primary(&mut self) -> Expr {
        match self.peek().cloned() {
            Some(Tok::Str(parts)) => {
                self.pos += 1;
                Expr::Str(parts)
            }
            Some(Tok::Int(n)) => {
                self.pos += 1;
                Expr::Int(n)
            }
            Some(Tok::Var(name)) => {
                self.pos += 1;
                let mut subs = Vec::new();
                while self.eat("[") {
                    subs.push(self.expr());
                    self.eat("]");
                }
                Expr::Var(name, subs)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if self.eat("(") {
                    let args = self.args(")");
                    return Expr::Call(name.to_ascii_lowercase(), args);
                }
                Expr::Const(name)
            }
            Some(Tok::Punct("(")) => {
                self.pos += 1;
                let e = self.expr();
                self.eat(")");
                e
            }
            Some(Tok::Punct("[")) => {
                self.pos += 1;
                let mut items = Vec::new();
                while self.pos < self.toks.len() && !self.eat("]") {
                    let first = self.expr();
                    if self.eat("=>") {
                        items.push((Some(first), self.expr()));
                    } else {
                        items.push((None, first));
                    }
                    if !self.eat(",") {
                        self.eat("]");
                        break;
                    }
                }
                Expr::Array(items)
            }
            _ => Expr::Const(String::new()),
        }
    }
}

struct Run<'a, 'b> {
    site: &'a Site<'b>,
    vars: Vec<(String, Val)>,
    out: String,
    depth: usize,
}

/// `exit` was reached.
struct Exit;

impl Run<'_, '_> {
    fn source(&mut self, source: &str) {
        let mut parser = Parser { toks: tokenize(source), pos: 0 };
        let program = parser.block(&[]);
        let _ = self.stmts(&program);
    }

    fn stmts(&mut self, stmts: &[Stmt]) -> Result<(), Exit> {
        for s in stmts {
            match s {
                Stmt::Inline(text) => self.out.push_str(text),
                Stmt::Echo(list) => {
                    for e in list {
                        let v = self.eval(e).to_str();
                        self.out.push_str(&v);
                    }
                }
                Stmt::Expr(e) => {
                    self.eval(e);
                }
                Stmt::Assign(name, e) => {
                    let v = self.eval(e);
                    match self.vars.iter_mut().find(|(n, _)| n == name) {
                        Some(slot) => slot.1 = v,
                        None => self.vars.push((name.clone(), v)),
                    }
                }
                Stmt::Require(e) => {
                    let path = PathBuf::from(self.eval(e).to_str());
                    if self.depth < 4
                        && let Ok(source) = std::fs::read_to_string(&path)
                    {
                        self.depth += 1;
                        self.source(&source);
                        self.depth -= 1;
                    }
                }
                Stmt::If(cond, then, otherwise) => {
                    if self.eval(cond).truthy() {
                        self.stmts(then)?;
                    } else {
                        self.stmts(otherwise)?;
                    }
                }
                Stmt::Exit => return Err(Exit),
            }
        }
        Ok(())
    }

    fn eval(&mut self, e: &Expr) -> Val {
        match e {
            Expr::Str(parts) => Val::Str(
                parts
                    .iter()
                    .map(|p| match p {
                        Part::Lit(s) => s.clone(),
                        Part::Var(name) => self.var(name).to_str(),
                    })
                    .collect(),
            ),
            Expr::Int(n) => Val::Int(*n),
            Expr::Var(name, subs) => {
                let mut v = self.var(name);
                for s in subs {
                    let key = self.eval(s).to_str();
                    v = v.index(&key);
                }
                v
            }
            Expr::Const(name) => match name.as_str() {
                "WPFORMS_PLUGIN_DIR" => Val::Str(format!("{}/", self.site.plugin_dir.to_string_lossy().trim_end_matches(['/', '\\']))),
                "true" | "TRUE" => Val::Bool(true),
                "false" | "FALSE" => Val::Bool(false),
                _ => Val::Null,
            },
            Expr::Array(items) => {
                let mut out: Vec<(String, Val)> = Vec::new();
                let mut next = 0i64;
                for (k, v) in items {
                    let key = match k {
                        Some(k) => self.eval(k).to_str(),
                        None => {
                            next += 1;
                            (next - 1).to_string()
                        }
                    };
                    let v = self.eval(v);
                    out.push((key, v));
                }
                Val::Arr(out)
            }
            Expr::Not(e) => Val::Bool(!self.eval(e).truthy()),
            Expr::Concat(a, b) => Val::Str(self.eval(a).to_str() + &self.eval(b).to_str()),
            Expr::And(a, b) => Val::Bool(self.eval(a).truthy() && self.eval(b).truthy()),
            Expr::Or(a, b) => Val::Bool(self.eval(a).truthy() || self.eval(b).truthy()),
            Expr::Same(a, b, same) => Val::Bool((self.eval(a) == self.eval(b)) == *same),
            Expr::Ternary(c, a, b) => {
                let cond = self.eval(c);
                if cond.truthy() {
                    match a {
                        Some(a) => self.eval(a),
                        None => cond,
                    }
                } else {
                    self.eval(b)
                }
            }
            Expr::Call(name, args) => self.call(name, args),
        }
    }

    fn var(&self, name: &str) -> Val {
        self.vars.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone()).unwrap_or(Val::Null)
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Val {
        let site = self.site;
        let mut values: Vec<Val> = args.iter().map(|a| self.eval(a)).collect();
        let arg = |i: usize| values.get(i).map(Val::to_str).unwrap_or_default();
        let s = match name {
            "empty" => return Val::Bool(!values.first().is_some_and(Val::truthy)),
            "isset" => return Val::Bool(values.iter().all(|v| *v != Val::Null)),
            "defined" => return Val::Bool(true),
            "is_rtl" => return Val::Bool(site.rtl),
            "absint" => return Val::Int(super::php::leading_int(&arg(0)).abs()),
            "apply_filters" => return values.get(1).cloned().unwrap_or(Val::Null),
            "esc_html" => esc_html(&arg(0)),
            "esc_attr" => esc_attr(&arg(0)),
            "esc_url" => esc_url(&arg(0)),
            "__" => (site.tr)(&arg(0)),
            "esc_html__" => esc_html(&(site.tr)(&arg(0))),
            "esc_attr__" => esc_attr(&(site.tr)(&arg(0))),
            "get_bloginfo" => bloginfo(site, &arg(0)),
            "bloginfo" => {
                let v = bloginfo(site, &arg(0));
                self.out.push_str(&v);
                return Val::Null;
            }
            "language_attributes" => {
                self.out.push_str(site.language_attributes);
                return Val::Null;
            }
            "home_url" => format!("{}{}", site.home_url, arg(0)),
            "wp_kses_post" => rustpress_core::kses::kses_post(&arg(0)),
            "wp_kses" => kses_with(&arg(0), values.get(1).unwrap_or(&Val::Null)),
            "wp_specialchars_decode" => specialchars_decode(&arg(0)),
            "sanitize_hex_color" => sanitize_hex_color(&arg(0)),
            "wpforms_generate_contrasting_color" => {
                let factor = |i: usize, d: i64| values.get(i).map_or(d, |v| super::php::leading_int(&v.to_str()));
                generate_contrasting_color(&arg(0), factor(1, 30), factor(2, 30))
            }
            "wpforms_get_min_suffix" => ".min".into(),
            "trim" => arg(0).trim_matches([' ', '\t', '\n', '\r', '\0', '\u{b}']).to_owned(),
            "sprintf" | "printf" => {
                let format = arg(0);
                let rest: Vec<String> = values.drain(..).skip(1).map(|v| v.to_str()).collect();
                let text = sprintf(&format, &rest);
                if name == "printf" {
                    self.out.push_str(&text);
                    return Val::Int(text.len() as i64);
                }
                text
            }
            _ => return Val::Null,
        };
        Val::Str(s)
    }
}

/// `get_bloginfo( $show )` of the values the templates ask for.
fn bloginfo(site: &Site<'_>, show: &str) -> String {
    match show {
        "name" | "" => site.blogname.to_owned(),
        "charset" => site.charset.to_owned(),
        "url" => site.home_url.to_owned(),
        _ => String::new(),
    }
}

/// `wp_kses( $text, [ 'a' => [ 'href' => [] ] ] )`, the only allowlist the templates pass.
fn kses_with(text: &str, allowed: &Val) -> String {
    use rustpress_core::kses::{Allowed, kses};
    static LINK: std::sync::OnceLock<Allowed> = std::sync::OnceLock::new();
    let link = Val::Arr(vec![("a".into(), Val::Arr(vec![("href".into(), Val::Arr(Vec::new()))]))]);
    if *allowed == link {
        return kses(text, LINK.get_or_init(|| Allowed::tags(&[("a", &["href"])])));
    }
    kses(text, &Allowed::none())
}

/// `wp_specialchars_decode( $text )` (`ENT_NOQUOTES`).
pub fn specialchars_decode(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"&(?:amp|lt|gt|#0*38|#0*60|#0*62|#[xX]0*26|#[xX]0*3[cC]|#[xX]0*3[eE]);").expect("valid regex"));
    re.replace_all(text, |c: &regex::Captures<'_>| {
        let m = c[0].to_ascii_lowercase();
        match m.as_str() {
            "&amp;" => "&",
            "&lt;" => "<",
            "&gt;" => ">",
            _ if m.ends_with("38;") || m.ends_with("26;") => "&",
            _ if m.ends_with("60;") || m.ends_with("3c;") => "<",
            _ => ">",
        }
        .to_owned()
    })
    .into_owned()
}

/// `sanitize_hex_color()`.
pub fn sanitize_hex_color(color: &str) -> String {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"^#([A-Fa-f0-9]{3}){1,2}$").expect("valid regex"));
    if re.is_match(color) { color.to_owned() } else { String::new() }
}

/// `hexdec()`: the characters that are no hex digits are ignored.
fn hexdec(s: &str) -> i64 {
    s.chars().filter_map(|c| c.to_digit(16)).fold(0i64, |n, d| n.saturating_mul(16).saturating_add(d as i64))
}

/// `wpforms_hex_to_rgb( $hex, false )`.
fn hex_to_rgb(hex: &str) -> [i64; 3] {
    let hex = hex.trim_start_matches('#');
    let chars: Vec<char> = hex.chars().collect();
    let full: Vec<char> = if chars.len() == 3 { chars.iter().flat_map(|c| [*c, *c]).collect() } else { chars };
    let pair = |i: usize| full.get(i..i + 2).map(|p| hexdec(&p.iter().collect::<String>())).unwrap_or_else(|| full.get(i).map_or(0, |c| hexdec(&c.to_string())));
    [pair(0), pair(2), pair(4)]
}

/// `dechex()` padded to two digits.
fn dechex(n: i64) -> String {
    format!("{:02x}", n.max(0))
}

/// PHP's `round()` (half away from zero).
fn round(x: f64) -> i64 {
    x.round() as i64
}

/// `wpforms_generate_contrasting_color()`.
pub fn generate_contrasting_color(color: &str, light: i64, dark: i64) -> String {
    let hex = color.replace('#', "");
    let part = |from: usize| hex.get(from..(from + 2).min(hex.len())).map_or(0, hexdec);
    let brightness = (part(0) * 299 + part(2) * 587 + part(4) * 114) as f64 / 1000.0;
    let rgb = hex_to_rgb(color);
    let mut out = String::from("#");
    for c in rgb {
        let n = if brightness > 155.0 { c - round(c as f64 / 100.0 * dark as f64) } else { c + round((255 - c) as f64 / 100.0 * light as f64) };
        out.push_str(&dechex(n));
    }
    out
}

/// `sprintf()` with `%s`, `%d`, `%%` and numbered arguments.
fn sprintf(format: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut next = 0;
    let mut rest = format;
    while let Some(p) = rest.find('%') {
        out.push_str(&rest[..p]);
        rest = &rest[p + 1..];
        if let Some(r) = rest.strip_prefix('%') {
            out.push('%');
            rest = r;
            continue;
        }
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let index = if !digits.is_empty() && rest[digits.len()..].starts_with('$') {
            rest = &rest[digits.len() + 1..];
            digits.parse::<usize>().unwrap_or(1).saturating_sub(1)
        } else {
            next += 1;
            next - 1
        };
        let Some(kind) = rest.chars().next() else { break };
        rest = &rest[kind.len_utf8()..];
        let value = args.get(index).cloned().unwrap_or_default();
        match kind {
            'd' => out.push_str(&super::php::leading_int(&value).to_string()),
            _ => out.push_str(&value),
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_the_template_code() {
        let tr = |s: &str| s.to_owned();
        let site = Site { plugin_dir: Path::new("/nonexistent"), tr: &tr, blogname: "A &amp; B", charset: "UTF-8", language_attributes: "lang=\"en-US\"", rtl: false, home_url: "http://example.com" };
        let mut run = Run { site: &site, vars: vec![("title".into(), Val::str("Hi")), ("img".into(), Val::Arr(vec![("url".into(), Val::str(""))]))], out: String::new(), depth: 0 };
        run.source("<?php\nif ( ! defined( 'ABSPATH' ) ) {\n\texit;\n}\n$min = 'x';\n?>\n\n<html <?php language_attributes(); ?>>\n<t><?php echo esc_html( $title ); ?></t>\n<?php if ( ! empty( $img['url'] ) ) : ?>\nIMG\n<?php endif; ?>\n<b class=\"<?php echo is_rtl() ? 'rtl' : 'ltr'; ?>\"><?php echo \"a{$min}b\"; ?></b>\n<?php\necho apply_filters( 'f', sprintf( wp_kses( __( 'Sent from <a href=\"%1$s\">%2$s</a>' ), [ 'a' => [ 'href' => [] ] ] ), esc_url( home_url() ), wp_specialchars_decode( get_bloginfo( 'name' ) ) ) );\n?>\n</html>\n");
        assert_eq!(run.out, "\n<html lang=\"en-US\">\n<t>Hi</t>\n<b class=\"ltr\">axb</b>\nSent from <a href=\"http://example.com\">A & B</a></html>\n");
    }

    #[test]
    fn colors_like_wpforms() {
        assert_eq!(generate_contrasting_color("#333333", 86, 72), "#e2e2e2");
        assert_eq!(generate_contrasting_color("#333333", 72, 63), "#c6c6c6");
        assert_eq!(generate_contrasting_color("#abc", 50, 45), "#d5dde6");
        assert_eq!(generate_contrasting_color("#dddddd", 86, 72), "#3e3e3e");
        assert_eq!(sanitize_hex_color("#abc"), "#abc");
        assert_eq!(sanitize_hex_color("abc"), "");
    }
}
