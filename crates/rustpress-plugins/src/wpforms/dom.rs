//! The HTML parser and serializer of libxml2 2.9 as PHP's `DOMDocument::loadHTML()` and `saveHTML()` use them:
//! WPForms runs its notification e-mails through both (`Notifications::fix_table_body_markup()` and
//! `CssToInlineStyles`), so the e-mails carry libxml's way of writing HTML.
//!
//! The parser follows `HTMLparser.c` for what the e-mails can contain: tag and attribute names in lower case, the
//! entities of HTML 4, void elements, `<x/>`, the start tags that close open elements (`htmlStartClose`), end tags
//! that are ignored unless the element is open, and kept whitespace. The serializer is `htmlNodeDumpFormatOutput()`
//! with its line breaks around block elements, its escaping and its URI escaping of `href`, `src` and `action`.

/// What a node is.
#[derive(Debug, Clone)]
pub enum Kind {
    Document,
    /// `<!DOCTYPE name>`.
    Doctype(String),
    Element { name: String, attrs: Vec<(String, Option<String>)> },
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub kind: Kind,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// A parsed document; node 0 is the document.
#[derive(Debug, Clone)]
pub struct Dom {
    pub nodes: Vec<Node>,
}

/// `htmlElemDesc` of libxml's `html40ElementTable`: `(empty, saveEndTag, isinline)`.
fn desc(name: &str) -> Option<(bool, bool, bool)> {
    Some(match name {
        "area" | "base" | "col" | "frame" | "hr" | "isindex" | "link" | "meta" | "param" => (true, true, false),
        "basefont" | "br" | "img" | "input" => (true, true, true),
        "li" => (false, true, false),
        "a" | "abbr" | "acronym" | "applet" | "b" | "bdo" | "big" | "button" | "cite" | "code" | "del" | "dfn" | "em" | "embed" | "font" | "i" | "iframe" | "ins"
        | "kbd" | "label" | "map" | "object" | "q" | "s" | "samp" | "script" | "select" | "small" | "span" | "strike" | "strong" | "sub" | "sup" | "textarea" | "tt"
        | "u" | "var" => (false, false, true),
        "address" | "blockquote" | "body" | "caption" | "center" | "colgroup" | "dd" | "dir" | "div" | "dl" | "dt" | "fieldset" | "form" | "frameset" | "h1" | "h2"
        | "h3" | "h4" | "h5" | "h6" | "head" | "html" | "legend" | "menu" | "noframes" | "noscript" | "ol" | "optgroup" | "option" | "p" | "pre" | "style" | "table"
        | "tbody" | "td" | "tfoot" | "th" | "thead" | "title" | "tr" | "ul" => (false, false, false),
        _ => return None,
    })
}

/// `htmlStartClose`: whether the start tag `new` closes the open element `open`.
fn start_closes(new: &str, open: &str) -> bool {
    const BLOCK: &[&str] = &["p", "head", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "listing", "xmp", "a"];
    let list: &[&str] = match new {
        "form" => &["form", "p", "hr", "h1", "h2", "h3", "h4", "h5", "h6", "dl", "ul", "ol", "menu", "dir", "address", "pre", "listing", "xmp", "head"],
        "head" | "title" | "caption" => &["p"],
        "body" => &["head", "style", "script", "title"],
        "frameset" => &["head", "style", "script", "title"],
        "li" => &["p", "h1", "h2", "h3", "h4", "h5", "h6", "dl", "address", "pre", "listing", "xmp", "head", "li"],
        "hr" => &["p", "head"],
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => &["p", "head", "h1", "h2", "h3", "h4", "h5", "h6"],
        "dir" | "address" | "pre" | "listing" | "xmp" | "blockquote" => &["p", "head", "ul"],
        "dl" => &["p", "dt", "menu", "dir", "address", "pre", "listing", "xmp", "head", "dd"],
        "dt" => &["p", "menu", "dir", "address", "pre", "listing", "xmp", "head", "dd"],
        "dd" => &["p", "menu", "dir", "address", "pre", "listing", "xmp", "head", "dt"],
        "ul" => &["p", "head", "ol", "menu", "dir", "address", "pre", "listing", "xmp"],
        "ol" => &["p", "head", "ul"],
        "menu" => &["p", "head", "ul"],
        "p" => &["p", "head", "h1", "h2", "h3", "h4", "h5", "h6"],
        "div" => &["p", "head"],
        "noscript" => &["script"],
        "center" => &["font", "b", "i", "p", "head"],
        "a" => &["a", "head"],
        "colgroup" => &["caption", "colgroup", "col", "p"],
        "col" => &["caption", "col", "p"],
        "table" => BLOCK,
        "fieldset" => &["legend", "p", "head", "h1", "h2", "h3", "h4", "h5", "h6", "pre", "listing", "xmp", "a"],
        "th" | "td" => &["th", "td", "p", "span", "font", "a", "b", "i", "u"],
        "tr" => &["th", "td", "tr", "caption", "col", "colgroup", "p"],
        "thead" => &["caption", "col", "colgroup"],
        "tfoot" => &["th", "td", "tr", "caption", "col", "colgroup", "thead", "tbody", "p"],
        "tbody" => &["th", "td", "tr", "caption", "col", "colgroup", "thead", "tfoot", "tbody", "p"],
        "optgroup" => &["option"],
        "option" => &["option"],
        "tt" | "i" | "b" | "u" | "s" | "strike" | "big" | "small" | "em" | "strong" | "dfn" | "code" | "samp" | "kbd" | "var" | "cite" | "abbr" | "acronym" | "img" | "br"
        | "font" | "basefont" | "span" | "sub" | "sup" | "q" | "map" | "bdo" | "iframe" | "label" | "select" | "textarea" | "button" | "input" | "object" | "applet"
        => &["head"],
        _ => &[],
    };
    list.contains(&open)
}

/// `htmlGetEndPriority()`.
fn end_priority(name: &str) -> u32 {
    match name {
        "div" => 150,
        "td" | "th" => 160,
        "tr" => 170,
        "thead" | "tbody" | "tfoot" => 180,
        "table" => 190,
        "head" | "body" => 200,
        "html" => 220,
        _ => 100,
    }
}

/// The named entities libxml knows (HTML 4, with `apos`).
fn entity(name: &str) -> Option<&'static str> {
    if !rustpress_core::html_entities::ALLOWED.contains(&name) {
        return None;
    }
    let key = format!("{name};");
    rustpress_core::html_entities::NAMED.iter().find(|(n, _)| *n == key).map(|(_, v)| *v)
}

/// `htmlParseCharRef()` and `htmlParseEntityRef()` at `&`: the text and the bytes read, or `None` for a literal `&`.
fn reference(s: &str) -> Option<(String, usize)> {
    let b = s.as_bytes();
    if b.get(1) == Some(&b'#') {
        let (hex, start) = if matches!(b.get(2), Some(b'x' | b'X')) { (true, 3) } else { (false, 2) };
        let mut i = start;
        let mut value: u32 = 0;
        while i < b.len() && b[i] != b';' {
            let d = if hex { (b[i] as char).to_digit(16) } else { (b[i] as char).to_digit(10) };
            let Some(d) = d else { break };
            value = value.saturating_mul(if hex { 16 } else { 10 }).saturating_add(d);
            i += 1;
        }
        if b.get(i) == Some(&b';') {
            i += 1;
        }
        let text = char::from_u32(value).filter(|&c| c != '\0').map(String::from).unwrap_or_default();
        return Some((text, i));
    }
    let mut i = 1;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'_' | b':' | b'.' | b'-')) {
        i += 1;
    }
    if i == 1 || b.get(i) != Some(&b';') {
        return None;
    }
    entity(&s[1..i]).map(|v| (v.to_owned(), i + 1))
}

fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        match reference(rest) {
            Some((text, n)) => {
                out.push_str(&text);
                rest = &rest[n..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_' | b'.')
}

fn is_blank(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// The options of `loadHTML()` that change the tree.
#[derive(Default, Clone, Copy)]
pub struct Options {
    /// `LIBXML_HTML_NOIMPLIED`.
    pub no_implied: bool,
}

struct Parser {
    dom: Dom,
    /// The open elements, innermost last.
    stack: Vec<usize>,
    options: Options,
}

impl Parser {
    fn current(&self) -> usize {
        self.stack.last().copied().unwrap_or(0)
    }

    fn name_of(&self, id: usize) -> &str {
        match &self.dom.nodes[id].kind {
            Kind::Element { name, .. } => name,
            _ => "",
        }
    }

    fn append(&mut self, parent: usize, kind: Kind) -> usize {
        let id = self.dom.nodes.len();
        self.dom.nodes.push(Node { kind, parent: Some(parent), children: Vec::new() });
        self.dom.nodes[parent].children.push(id);
        id
    }

    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let blank = text.bytes().all(is_blank);
        if self.stack.is_empty() {
            if blank {
                return;
            }
            self.start("p", Vec::new());
        }
        if !blank && matches!(self.name_of(self.current()), "html" | "head") {
            // `htmlCheckParagraph()`: text cannot stand in `html` or `head`.
            self.start("p", Vec::new());
        }
        let parent = self.current();
        if let Some(&last) = self.dom.nodes[parent].children.last()
            && let Kind::Text(t) = &mut self.dom.nodes[last].kind
        {
            t.push_str(text);
            return;
        }
        self.append(parent, Kind::Text(text.to_owned()));
    }

    /// `htmlCheckImplied()`: the `html`, `head` and `body` a tag needs.
    fn implied(&mut self, name: &str) {
        if self.options.no_implied || name == "html" {
            return;
        }
        if self.stack.is_empty() {
            let html = self.append(0, Kind::Element { name: "html".into(), attrs: Vec::new() });
            self.stack.push(html);
        }
        if matches!(name, "body" | "head") {
            return;
        }
        let open: Vec<String> = self.stack.iter().map(|&i| self.name_of(i).to_owned()).collect();
        if open.iter().any(|n| n == "body" || n == "head") {
            return;
        }
        let has_body = self.dom.nodes[self.stack[0]].children.iter().any(|&c| matches!(&self.dom.nodes[c].kind, Kind::Element { name, .. } if name == "body" || name == "head"));
        if matches!(name, "script" | "style" | "meta" | "link" | "title" | "base") {
            if !has_body {
                self.start_element("head", Vec::new());
            }
        } else if !has_body || !open.iter().any(|n| n == "body") {
            self.start_element("body", Vec::new());
        }
    }

    fn start_element(&mut self, name: &str, attrs: Vec<(String, Option<String>)>) -> usize {
        let parent = self.current();
        let id = self.append(parent, Kind::Element { name: name.to_owned(), attrs });
        self.stack.push(id);
        id
    }

    fn start(&mut self, name: &str, attrs: Vec<(String, Option<String>)>) -> usize {
        // `htmlAutoClose()`.
        while let Some(&top) = self.stack.last() {
            if start_closes(name, self.name_of(top)) {
                self.stack.pop();
            } else {
                break;
            }
        }
        self.implied(name);
        if name == "html" && !self.stack.is_empty() {
            return self.current();
        }
        self.start_element(name, attrs)
    }

    fn end(&mut self, name: &str) {
        let Some(pos) = self.stack.iter().rposition(|&i| self.name_of(i) == name) else { return };
        let priority = end_priority(name);
        if self.stack[pos + 1..].iter().any(|&i| end_priority(self.name_of(i)) > priority) {
            return;
        }
        self.stack.truncate(pos);
    }
}

/// `DOMDocument::loadHTML()` of a string of HTML.
pub fn parse(html: &str, options: Options) -> Dom {
    let mut p = Parser { dom: Dom { nodes: vec![Node { kind: Kind::Document, parent: None, children: Vec::new() }] }, stack: Vec::new(), options };
    let b = html.as_bytes();
    let mut i = 0;
    let mut text_start = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &html[i..];
        let flush = |p: &mut Parser, end: usize| p.text(&decode(&html[text_start..end]));
        if let Some(after) = rest.strip_prefix("<!--") {
            flush(&mut p, i);
            let end = after.find("-->").map(|e| e + 4);
            let content = &after[..end.map_or(after.len(), |e| e - 4)];
            let parent = p.current();
            p.append(parent, Kind::Comment(content.to_owned()));
            i += end.map_or(rest.len(), |e| e + 3);
            text_start = i;
            continue;
        }
        if rest.len() > 9 && rest[..9].eq_ignore_ascii_case("<!doctype") {
            flush(&mut p, i);
            let end = rest.find('>').unwrap_or(rest.len());
            let name = rest[9..end].split_whitespace().next().unwrap_or("").to_owned();
            if p.stack.is_empty() {
                p.append(0, Kind::Doctype(name));
            }
            i += (end + 1).min(rest.len());
            text_start = i;
            continue;
        }
        if rest.starts_with("</") && rest.as_bytes().get(2).is_some_and(|c| c.is_ascii_alphabetic()) {
            flush(&mut p, i);
            let mut j = 2;
            while j < rest.len() && is_name_byte(rest.as_bytes()[j]) {
                j += 1;
            }
            let name = rest[2..j].to_ascii_lowercase();
            let close = rest[j..].find('>').map_or(rest.len(), |e| j + e + 1);
            p.end(&name);
            i += close;
            text_start = i;
            continue;
        }
        if !rest.as_bytes().get(1).is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_' || *c == b':') {
            // Not a tag: the `<` is text.
            i += 1;
            continue;
        }
        flush(&mut p, i);
        let rb = rest.as_bytes();
        let mut j = 1;
        while j < rb.len() && is_name_byte(rb[j]) {
            j += 1;
        }
        let name = rest[1..j].to_ascii_lowercase();
        let mut attrs: Vec<(String, Option<String>)> = Vec::new();
        let mut self_closing = false;
        loop {
            while j < rb.len() && is_blank(rb[j]) {
                j += 1;
            }
            if j >= rb.len() {
                break;
            }
            if rb[j] == b'>' {
                j += 1;
                break;
            }
            if rb[j] == b'/' && rb.get(j + 1) == Some(&b'>') {
                self_closing = true;
                j += 2;
                break;
            }
            let start = j;
            while j < rb.len() && !is_blank(rb[j]) && !matches!(rb[j], b'=' | b'>') && !(rb[j] == b'/' && rb.get(j + 1) == Some(&b'>')) {
                j += 1;
            }
            if j == start {
                j += 1;
                continue;
            }
            let attr = rest[start..j].to_ascii_lowercase();
            while j < rb.len() && is_blank(rb[j]) {
                j += 1;
            }
            let mut value = None;
            if rb.get(j) == Some(&b'=') {
                j += 1;
                while j < rb.len() && is_blank(rb[j]) {
                    j += 1;
                }
                if matches!(rb.get(j), Some(b'"' | b'\'')) {
                    let q = rb[j] as char;
                    let end = rest[j + 1..].find(q).map_or(rest.len(), |e| j + 1 + e);
                    value = Some(decode(&rest[j + 1..end]));
                    j = (end + 1).min(rest.len());
                } else {
                    let s = j;
                    while j < rb.len() && !is_blank(rb[j]) && rb[j] != b'>' {
                        j += 1;
                    }
                    value = Some(decode(&rest[s..j]));
                }
            }
            if !attrs.iter().any(|(n, _)| *n == attr) {
                attrs.push((attr, value));
            }
        }
        i += j;
        text_start = i;
        let (empty, _, _) = desc(&name).unwrap_or((false, false, false));
        let id = p.start(&name, attrs);
        if empty || self_closing {
            if p.stack.last() == Some(&id) {
                p.stack.pop();
            }
            continue;
        }
        if name == "script" || name == "style" {
            // Raw text up to the end tag.
            let close = format!("</{name}");
            let lower = html[i..].to_ascii_lowercase();
            let end = lower.find(&close).map_or(html.len(), |e| i + e);
            if end > i {
                p.append(id, Kind::Text(html[i..end].to_owned()));
            }
            i = end;
            text_start = i;
        }
    }
    p.text(&decode(&html[text_start..]));
    p.dom
}

impl Dom {
    pub fn name(&self, id: usize) -> Option<&str> {
        match &self.nodes[id].kind {
            Kind::Element { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn attr(&self, id: usize, attr: &str) -> Option<&str> {
        match &self.nodes[id].kind {
            Kind::Element { attrs, .. } => attrs.iter().find(|(n, _)| n == attr).map(|(_, v)| v.as_deref().unwrap_or("")),
            _ => None,
        }
    }

    /// `DOMElement::setAttribute()`: in place when it exists, else at the end.
    pub fn set_attr(&mut self, id: usize, attr: &str, value: &str) {
        if let Kind::Element { attrs, .. } = &mut self.nodes[id].kind {
            match attrs.iter_mut().find(|(n, _)| n == attr) {
                Some(slot) => slot.1 = Some(value.to_owned()),
                None => attrs.push((attr.to_owned(), Some(value.to_owned()))),
            }
        }
    }

    /// The elements in document order.
    pub fn elements(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![0];
        while let Some(id) = stack.pop() {
            if matches!(self.nodes[id].kind, Kind::Element { .. }) {
                out.push(id);
            }
            stack.extend(self.nodes[id].children.iter().rev());
        }
        out
    }

    /// The first element of the document (`documentElement`).
    pub fn document_element(&self) -> Option<usize> {
        self.nodes[0].children.iter().copied().find(|&c| matches!(self.nodes[c].kind, Kind::Element { .. }))
    }

    /// `DOMDocument::saveHTML( $node )` with `formatOutput`.
    pub fn save(&self, id: usize, format: bool) -> String {
        let mut out = String::new();
        self.dump(id, format, &mut out);
        out
    }


    fn is_text(&self, id: usize) -> bool {
        matches!(self.nodes[id].kind, Kind::Text(_))
    }

    fn next(&self, id: usize) -> Option<usize> {
        let parent = self.nodes[id].parent?;
        let siblings = &self.nodes[parent].children;
        let pos = siblings.iter().position(|&c| c == id)?;
        siblings.get(pos + 1).copied()
    }

    /// `htmlNodeDumpFormatOutput()`.
    fn dump(&self, id: usize, format: bool, out: &mut String) {
        let node = &self.nodes[id];
        match &node.kind {
            Kind::Document => {
                for &c in &node.children {
                    self.dump(c, format, out);
                }
                out.push('\n');
            }
            Kind::Doctype(name) => {
                out.push_str(&format!("<!DOCTYPE {name}>\n"));
            }
            Kind::Comment(text) => {
                out.push_str("<!--");
                out.push_str(text);
                out.push_str("-->");
            }
            Kind::Text(text) => {
                let raw = node.parent.and_then(|p| self.name(p)).is_some_and(|n| n == "script" || n == "style");
                if raw { out.push_str(text) } else { out.push_str(&escape_text(text)) }
            }
            Kind::Element { name, attrs } => {
                let info = desc(name);
                out.push('<');
                out.push_str(name);
                for (attr, value) in attrs {
                    out.push(' ');
                    out.push_str(attr);
                    let Some(value) = value else { continue };
                    if is_boolean(attr) {
                        continue;
                    }
                    out.push('=');
                    let escaped = escape_attr(value);
                    if matches!(attr.as_str(), "href" | "action" | "src") || (attr == "name" && name == "a") {
                        out.push_str(&quoted(&uri_escape(escaped.trim_start_matches([' ', '\t', '\n', '\r']))));
                    } else {
                        out.push_str(&quoted(&escaped));
                    }
                }
                let parent_p = node.parent.and_then(|p| self.name(p)).is_some_and(|n| n.starts_with('p'));
                let block = info.is_some_and(|(_, _, inline)| !inline);
                if info.is_some_and(|(empty, _, _)| empty) {
                    out.push('>');
                } else if node.children.is_empty() {
                    if info.is_some_and(|(_, save, _)| save) && name != "html" && name != "body" {
                        out.push('>');
                    } else {
                        out.push_str(&format!("></{name}>"));
                    }
                } else {
                    out.push('>');
                    let first = node.children[0];
                    let last = *node.children.last().unwrap_or(&first);
                    if format && block && !self.is_text(first) && first != last && !name.starts_with('p') {
                        out.push('\n');
                    }
                    for &c in &node.children {
                        self.dump(c, format, out);
                    }
                    if format && block && !self.is_text(last) && first != last && !name.starts_with('p') {
                        out.push('\n');
                    }
                    out.push_str(&format!("</{name}>"));
                }
                if format && block && let Some(next) = self.next(id) && !self.is_text(next) && node.parent.and_then(|p| self.name(p)).is_some() && !parent_p {
                    out.push('\n');
                }
            }
        }
    }

    /// The text of a node (`nodeValue`).
    pub fn text_content(&self, id: usize) -> String {
        match &self.nodes[id].kind {
            Kind::Text(t) | Kind::Comment(t) => t.clone(),
            _ => self.nodes[id].children.iter().map(|&c| if matches!(self.nodes[c].kind, Kind::Comment(_)) { String::new() } else { self.text_content(c) }).collect(),
        }
    }
}

/// `htmlIsBooleanAttr()`.
fn is_boolean(attr: &str) -> bool {
    matches!(attr, "checked" | "compact" | "declare" | "defer" | "disabled" | "ismap" | "multiple" | "nohref" | "noresize" | "noshade" | "nowrap" | "readonly" | "selected")
}

/// `xmlEncodeEntitiesReentrant()` of an HTML document.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 => out.push_str(&format!("&#{};", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// `xmlEncodeAttributeEntities()` (with the `&{...}` of HTML 4 left alone).
fn escape_attr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '&' && rest[1..].starts_with('{') && let Some(end) = rest.find('}') {
            out.push_str(&rest[..=end]);
            rest = &rest[end + 1..];
            continue;
        }
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 => out.push_str(&format!("&#{};", c as u32)),
            c => out.push(c),
        }
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// `xmlURIEscapeStr( $value, "@/:=?;#%&,+<>" )`.
fn uri_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for &b in value.as_bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()@/:=?;#%&,+<>".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `xmlBufWriteQuotedString()`.
fn quoted(value: &str) -> String {
    if value.contains('"') {
        if value.contains('\'') { format!("\"{}\"", value.replace('"', "&quot;")) } else { format!("'{value}'") }
    } else {
        format!("\"{value}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(html: &str) -> String {
        let dom = parse(html, Options::default());
        dom.save(dom.document_element().unwrap(), true).trim().to_owned()
    }

    #[test]
    fn serializes_like_libxml() {
        // The output of PHP 8.3 with libxml 2.9.14 (`saveHTML( $documentElement )` with `formatOutput`).
        assert_eq!(
            roundtrip("<!DOCTYPE html>\n<html lang=\"en\"><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\"><title>T</title></head><body><table><tbody>\n\t\t<tr>\n\t<td>a</td>\n</tr><tr>\n\t<td>b &amp; c &lt; d \" ' J\u{f6}rg \u{2713} &nbsp; &mdash;</td>\n</tr>\n\t</tbody>\n</table></body></html>"),
            "<html lang=\"en\">\n<head>\n<meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\">\n<title>T</title>\n</head>\n<body><table>\n<tbody>\n\t\t<tr>\n\t<td>a</td>\n</tr>\n<tr>\n\t<td>b &amp; c &lt; d \" ' J\u{f6}rg \u{2713} \u{a0} \u{2014}</td>\n</tr>\n\t</tbody>\n</table></body>\n</html>"
        );
        assert_eq!(
            roundtrip("<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\"></head><body><p>a\r\nb<br/>c<br />d</p><a href=\"http://x.de/a b?x=1&amp;y=\u{f6}\">l&ouml;</a><img src=\"x.png\" alt=\"A &amp; &quot;B&quot;\"></body></html>"),
            "<html>\n<head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\"></head>\n<body>\n<p>a\r\nb<br>c<br>d</p>\n<a href=\"http://x.de/a%20b?x=1&amp;y=%C3%B6\">l\u{f6}</a><img src=\"x.png\" alt='A &amp; \"B\"'>\n</body>\n</html>"
        );
        assert_eq!(
            roundtrip("<html><head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\"></head><body><div title=\"a<b>c 'q' &amp;\">x > y</div><a name=\"a b\">n</a><input type=\"checkbox\" checked disabled=\"disabled\"><select><option selected>x</option></select><div></div><span></span><p></p><td></td></body></html>"),
            "<html>\n<head><meta http-equiv=\"Content-Type\" content=\"text/html; charset=UTF-8\"></head>\n<body>\n<div title=\"a&lt;b&gt;c 'q' &amp;\">x &gt; y</div>\n<a name=\"a%20b\">n</a><input type=\"checkbox\" checked disabled><select><option selected>x</option></select><div></div>\n<span></span><p></p>\n<td></td>\n</body>\n</html>"
        );
    }
}
