//! Port of `WP_Block_Parser`: splits post content into blocks.
//!
//! Blocks are delimited by HTML comments, `<!-- wp:name {"json":"attrs"} -->`
//! ... `<!-- /wp:name -->`, or the self-closing `<!-- wp:name /-->`. Content
//! outside of blocks becomes "freeform" blocks without a name.

use serde_json::{Map, Value};

/// One entry of [`Block::inner_content`]: literal HTML or the position of the
/// next entry of [`Block::inner_blocks`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Html(String),
    InnerBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Full name with namespace (`core/paragraph`); `None` for freeform HTML.
    pub name: Option<String>,
    pub attrs: Map<String, Value>,
    pub inner_blocks: Vec<Block>,
    /// The block's own HTML without inner blocks.
    pub inner_html: String,
    /// `inner_html` pieces interleaved with the inner block positions.
    pub inner_content: Vec<Content>,
}

impl Block {
    fn new(name: Option<String>, attrs: Map<String, Value>) -> Self {
        Self { name, attrs, inner_blocks: Vec::new(), inner_html: String::new(), inner_content: Vec::new() }
    }

    fn freeform(html: &str) -> Self {
        Self {
            name: None,
            attrs: Map::new(),
            inner_blocks: Vec::new(),
            inner_html: html.to_owned(),
            inner_content: vec![Content::Html(html.to_owned())],
        }
    }

    /// Attribute helper: a string attribute.
    pub fn attr_str(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).and_then(Value::as_str)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind {
    Void,
    Opener,
    Closer,
}

struct Token {
    kind: TokenKind,
    name: String,
    attrs: Map<String, Value>,
    start: usize,
    len: usize,
}

struct Frame {
    block: Block,
    token_start: usize,
    token_len: usize,
    prev_offset: usize,
    leading_html_start: Option<usize>,
}

/// Parses a document into top-level blocks.
pub fn parse_blocks(document: &str) -> Vec<Block> {
    let mut parser = Parser { doc: document, offset: 0, output: Vec::new(), stack: Vec::new() };
    while parser.proceed() {}
    parser.output
}

struct Parser<'a> {
    doc: &'a str,
    offset: usize,
    output: Vec<Block>,
    stack: Vec<Frame>,
}

impl Parser<'_> {
    fn proceed(&mut self) -> bool {
        let token = self.next_token();
        let depth = self.stack.len();
        let (kind, start, len) = match &token {
            Some(t) => (Some(t.kind), t.start, t.len),
            None => (None, self.doc.len(), 0),
        };
        let leading_html_start = (start > self.offset).then_some(self.offset);

        match (kind, token) {
            (None, _) => {
                if depth == 0 {
                    self.add_freeform(None);
                    return false;
                }
                // Unclosed blocks: close them all, innermost first.
                while !self.stack.is_empty() {
                    self.add_block_from_stack(None);
                }
                false
            }
            (Some(TokenKind::Void), Some(t)) => {
                let block = Block::new(Some(t.name), t.attrs);
                if depth == 0 {
                    if let Some(lead) = leading_html_start {
                        self.output.push(Block::freeform(&self.doc[lead..start]));
                    }
                    self.output.push(block);
                } else {
                    self.add_inner_block(block, start, len, None);
                }
                self.offset = start + len;
                true
            }
            (Some(TokenKind::Opener), Some(t)) => {
                self.stack.push(Frame {
                    block: Block::new(Some(t.name), t.attrs),
                    token_start: start,
                    token_len: len,
                    prev_offset: start + len,
                    leading_html_start,
                });
                self.offset = start + len;
                true
            }
            (Some(TokenKind::Closer), Some(_)) => {
                if depth == 0 {
                    // A closer without an opener: the rest is freeform.
                    self.add_freeform(None);
                    return false;
                }
                if depth == 1 {
                    self.add_block_from_stack(Some(start));
                    self.offset = start + len;
                    return true;
                }
                let mut top = self.stack.pop().expect("depth > 1");
                let html = &self.doc[top.prev_offset..start];
                top.block.inner_html.push_str(html);
                top.block.inner_content.push(Content::Html(html.to_owned()));
                top.prev_offset = start + len;
                self.add_inner_block(top.block, top.token_start, top.token_len, Some(start + len));
                self.offset = start + len;
                true
            }
            _ => {
                self.add_freeform(None);
                false
            }
        }
    }

    fn add_freeform(&mut self, length: Option<usize>) {
        let length = length.unwrap_or(self.doc.len() - self.offset);
        if length == 0 {
            return;
        }
        self.output.push(Block::freeform(&self.doc[self.offset..self.offset + length]));
    }

    fn add_inner_block(&mut self, block: Block, token_start: usize, token_len: usize, last_offset: Option<usize>) {
        let parent = self.stack.last_mut().expect("inner block needs a parent");
        parent.block.inner_blocks.push(block);
        let html = &self.doc[parent.prev_offset..token_start];
        if !html.is_empty() {
            parent.block.inner_html.push_str(html);
            parent.block.inner_content.push(Content::Html(html.to_owned()));
        }
        parent.block.inner_content.push(Content::InnerBlock);
        parent.prev_offset = last_offset.unwrap_or(token_start + token_len);
    }

    fn add_block_from_stack(&mut self, end_offset: Option<usize>) {
        let mut top = self.stack.pop().expect("stack not empty");
        let html = match end_offset {
            Some(end) => &self.doc[top.prev_offset..end],
            None => &self.doc[top.prev_offset..],
        };
        if !html.is_empty() {
            top.block.inner_html.push_str(html);
            top.block.inner_content.push(Content::Html(html.to_owned()));
        }
        if let Some(lead) = top.leading_html_start {
            self.output.push(Block::freeform(&self.doc[lead..top.token_start]));
        }
        self.output.push(top.block);
    }

    /// Finds the next block delimiter at or after `self.offset`.
    fn next_token(&self) -> Option<Token> {
        let bytes = self.doc.as_bytes();
        let mut from = self.offset;
        while let Some(rel) = self.doc[from..].find("<!--") {
            let start = from + rel;
            if let Some(token) = parse_delimiter(self.doc, start) {
                return Some(token);
            }
            from = start + 4;
            if from >= bytes.len() {
                break;
            }
        }
        None
    }
}

/// `[ \t\f\r\n]`: since 7.1.4 the delimiter regexp no longer takes `\s`, which also matched a vertical tab.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c)
}

fn skip_spaces(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_space(bytes[i]) {
        i += 1;
    }
    i
}

/// `[a-z][a-z0-9_-]*` starting at `i`; returns the end offset.
fn read_name(bytes: &[u8], i: usize) -> Option<usize> {
    if i >= bytes.len() || !bytes[i].is_ascii_lowercase() {
        return None;
    }
    let mut j = i + 1;
    while j < bytes.len() && (bytes[j].is_ascii_lowercase() || bytes[j].is_ascii_digit() || matches!(bytes[j], b'_' | b'-')) {
        j += 1;
    }
    Some(j)
}

/// Matches the block delimiter regexp of `WP_Block_Parser::next_token` at `start`:
/// `<!--[ \t\f\r\n]+(/)?wp:(namespace/)?name[ \t\f\r\n]+({attrs}[ \t\f\r\n]+)?(/)?-->`.
fn parse_delimiter(doc: &str, start: usize) -> Option<Token> {
    let bytes = doc.as_bytes();
    let mut i = start + 4;
    let after_ws = skip_spaces(bytes, i);
    if after_ws == i {
        return None;
    }
    i = after_ws;
    let closer = bytes.get(i) == Some(&b'/');
    if closer {
        i += 1;
    }
    if !doc[i..].starts_with("wp:") {
        return None;
    }
    i += 3;
    let first_end = read_name(bytes, i)?;
    let (namespace, name_start, name_end) = if bytes.get(first_end) == Some(&b'/') {
        let name_end = read_name(bytes, first_end + 1)?;
        (Some(&doc[i..first_end]), first_end + 1, name_end)
    } else {
        (None, i, first_end)
    };
    let name = &doc[name_start..name_end];
    i = name_end;
    let after_ws = skip_spaces(bytes, i);
    if after_ws == i {
        return None;
    }
    i = after_ws;

    let mut attrs = Map::new();
    if bytes.get(i) == Some(&b'{') {
        // Attributes end at the first `}` followed by whitespace and `-->` or `/-->`.
        let mut j = i + 1;
        let mut end = None;
        while j < bytes.len() {
            if bytes[j] == b'}' {
                let k = skip_spaces(bytes, j + 1);
                if k > j + 1 && (doc[k..].starts_with("-->") || doc[k..].starts_with("/-->")) {
                    end = Some(j);
                    break;
                }
            }
            j += 1;
        }
        let end = end?;
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&doc[i..=end]) {
            attrs = map;
        }
        i = skip_spaces(bytes, end + 1);
    }

    let void = bytes.get(i) == Some(&b'/');
    if void {
        i += 1;
    }
    if !doc[i..].starts_with("-->") {
        return None;
    }
    i += 3;

    let full_name = match namespace {
        Some(ns) => format!("{ns}/{name}"),
        None => format!("core/{name}"),
    };
    let kind = if closer {
        TokenKind::Closer
    } else if void {
        TokenKind::Void
    } else {
        TokenKind::Opener
    };
    // The regexp does not allow `/` for closers with attributes; WordPress ignores that case too.
    Some(Token { kind, name: full_name, attrs, start, len: i - start })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(blocks: &[Block]) -> Vec<Option<&str>> {
        blocks.iter().map(|b| b.name.as_deref()).collect()
    }

    #[test]
    fn a_vertical_tab_is_not_delimiter_space() {
        assert_eq!(names(&parse_blocks("<!--\x0bwp:paragraph -->x<!-- /wp:paragraph -->")), vec![None]);
        assert_eq!(names(&parse_blocks("<!--\x0cwp:paragraph\t-->x<!--\r\n/wp:paragraph -->")), vec![Some("core/paragraph")]);
    }

    #[test]
    fn plain_html_is_one_freeform_block() {
        let blocks = parse_blocks("<p>hi</p>");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].name, None);
        assert_eq!(blocks[0].inner_html, "<p>hi</p>");
        assert_eq!(parse_blocks(""), Vec::new());
    }

    #[test]
    fn single_block_with_attrs() {
        let blocks = parse_blocks("<!-- wp:heading {\"level\":3} -->\n<h3>T</h3>\n<!-- /wp:heading -->");
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.name.as_deref(), Some("core/heading"));
        assert_eq!(b.attrs["level"], 3);
        assert_eq!(b.inner_html, "\n<h3>T</h3>\n");
        assert_eq!(b.inner_content, [Content::Html("\n<h3>T</h3>\n".into())]);
    }

    #[test]
    fn namespaces_and_void_blocks() {
        let blocks = parse_blocks("<!-- wp:my-plugin/thing {\"a\":1} /--><!-- wp:separator /-->");
        assert_eq!(names(&blocks), [Some("my-plugin/thing"), Some("core/separator")]);
        assert!(blocks[0].inner_html.is_empty());
        assert!(blocks[1].inner_content.is_empty());
    }

    #[test]
    fn whitespace_between_blocks_is_freeform() {
        let blocks = parse_blocks("<!-- wp:a /-->\n\n<!-- wp:b /-->\n");
        assert_eq!(names(&blocks), [Some("core/a"), None, Some("core/b"), None]);
        assert_eq!(blocks[1].inner_html, "\n\n");
        assert_eq!(blocks[3].inner_html, "\n");
    }

    #[test]
    fn nested_blocks_keep_inner_content_layout() {
        let doc = "<!-- wp:group -->\n<div class=\"g\">\n<!-- wp:paragraph -->\n<p>a</p>\n<!-- /wp:paragraph -->\n\n<!-- wp:paragraph -->\n<p>b</p>\n<!-- /wp:paragraph -->\n</div>\n<!-- /wp:group -->";
        let blocks = parse_blocks(doc);
        assert_eq!(blocks.len(), 1);
        let group = &blocks[0];
        assert_eq!(group.inner_blocks.len(), 2);
        assert_eq!(
            group.inner_content,
            [
                Content::Html("\n<div class=\"g\">\n".into()),
                Content::InnerBlock,
                Content::Html("\n\n".into()),
                Content::InnerBlock,
                Content::Html("\n</div>\n".into()),
            ]
        );
        assert_eq!(group.inner_html, "\n<div class=\"g\">\n\n\n\n</div>\n");
        assert_eq!(group.inner_blocks[1].inner_html, "\n<p>b</p>\n");
    }

    #[test]
    fn deeply_nested_and_siblings() {
        let doc = "<!-- wp:a --><!-- wp:b --><!-- wp:c /--><!-- /wp:b --><!-- wp:d /--><!-- /wp:a -->";
        let blocks = parse_blocks(doc);
        assert_eq!(blocks.len(), 1);
        let a = &blocks[0];
        assert_eq!(names(&a.inner_blocks), [Some("core/b"), Some("core/d")]);
        assert_eq!(names(&a.inner_blocks[0].inner_blocks), [Some("core/c")]);
        assert_eq!(a.inner_content, [Content::InnerBlock, Content::InnerBlock]);
    }

    #[test]
    fn leading_and_trailing_html_around_blocks() {
        let blocks = parse_blocks("before<!-- wp:a -->x<!-- /wp:a -->after");
        assert_eq!(names(&blocks), [None, Some("core/a"), None]);
        assert_eq!(blocks[0].inner_html, "before");
        assert_eq!(blocks[2].inner_html, "after");
    }

    #[test]
    fn unclosed_blocks_are_closed_at_the_end() {
        // Like WordPress, the open blocks are flushed innermost first, not nested.
        let blocks = parse_blocks("<!-- wp:a --><!-- wp:b -->text");
        assert_eq!(names(&blocks), [Some("core/b"), Some("core/a")]);
        assert_eq!(blocks[0].inner_html, "text");
    }

    #[test]
    fn stray_closer_turns_the_rest_into_freeform() {
        let blocks = parse_blocks("<p>x</p><!-- /wp:paragraph -->");
        assert_eq!(names(&blocks), [None]);
        assert_eq!(blocks[0].inner_html, "<p>x</p><!-- /wp:paragraph -->");
    }

    #[test]
    fn attributes_may_contain_braces_and_comment_markers() {
        let doc = "<!-- wp:html {\"a\":\"}\\u002d\\u002d>\",\"b\":{\"c\":1}} -->x<!-- /wp:html -->";
        let blocks = parse_blocks(doc);
        assert_eq!(blocks[0].attrs["a"], "}-->");
        assert_eq!(blocks[0].attrs["b"]["c"], 1);
    }

    #[test]
    fn invalid_json_and_plain_comments_are_tolerated() {
        let blocks = parse_blocks("<!-- hello --><!-- wp:a {nope} -->x<!-- /wp:a -->");
        assert_eq!(names(&blocks), [None, Some("core/a")]);
        assert!(blocks[1].attrs.is_empty());
        assert_eq!(blocks[0].inner_html, "<!-- hello -->");
        // Not a delimiter: no whitespace after the name or after `<!--`.
        assert_eq!(parse_blocks("<!--wp:a --><!-- wp:a-->").len(), 1);
    }

    #[test]
    fn multibyte_content_is_kept_intact() {
        let blocks = parse_blocks("<!-- wp:paragraph -->\n<p>äöü “x”</p>\n<!-- /wp:paragraph -->");
        assert_eq!(blocks[0].inner_html, "\n<p>äöü “x”</p>\n");
    }
}
