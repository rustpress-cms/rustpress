//! `serialize_block()`, `serialize_blocks()`, `traverse_and_serialize_blocks()` and `resolve_pattern_blocks()`.

use serde_json::{Map, Value};

use crate::parser::{Block, Content, parse_blocks};
use crate::php_json::{Flags, encode_with};

/// `strip_core_block_namespace()`.
fn strip_core_namespace(name: &str) -> &str {
    name.strip_prefix("core/").unwrap_or(name)
}

/// `serialize_block_attributes()`.
pub fn serialize_block_attributes(attrs: &Map<String, Value>) -> String {
    let encoded = encode_with(&Value::Object(attrs.clone()), Flags { unescaped_slashes: true, unescaped_unicode: true });
    encoded.replace("--", "\\u002d\\u002d").replace('<', "\\u003c").replace('>', "\\u003e").replace('&', "\\u0026").replace("\\\"", "\\u0022")
}

/// `get_comment_delimited_block_content()`.
pub fn comment_delimited(name: Option<&str>, attrs: &Map<String, Value>, content: &str) -> String {
    let Some(name) = name else { return content.to_owned() };
    let name = strip_core_namespace(name);
    let attributes = if attrs.is_empty() { String::new() } else { format!("{} ", serialize_block_attributes(attrs)) };
    if content.is_empty() {
        return format!("<!-- wp:{name} {attributes}/-->");
    }
    format!("<!-- wp:{name} {attributes}-->{content}<!-- /wp:{name} -->")
}

/// `serialize_block()`.
pub fn serialize_block(block: &Block) -> String {
    let mut out = String::new();
    let mut index = 0;
    for chunk in &block.inner_content {
        match chunk {
            Content::Html(html) => out.push_str(html),
            Content::InnerBlock => {
                if let Some(inner) = block.inner_blocks.get(index) {
                    out.push_str(&serialize_block(inner));
                }
                index += 1;
            }
        }
    }
    comment_delimited(block.name.as_deref(), &block.attrs, &out)
}

/// `serialize_blocks()`.
pub fn serialize_blocks(blocks: &[Block]) -> String {
    blocks.iter().map(serialize_block).collect()
}

/// `traverse_and_serialize_blocks()` with a callback that runs before each block (and its children) is written.
pub fn serialize_with(blocks: &mut [Block], before: &dyn Fn(&mut Block)) -> String {
    let mut out = String::new();
    for block in blocks.iter_mut() {
        before(block);
        let mut content = String::new();
        let mut index = 0;
        let chunks = block.inner_content.clone();
        for chunk in &chunks {
            match chunk {
                Content::Html(html) => content.push_str(html),
                Content::InnerBlock => {
                    if let Some(inner) = block.inner_blocks.get_mut(index) {
                        content.push_str(&serialize_with(std::slice::from_mut(inner), before));
                    }
                    index += 1;
                }
            }
        }
        out.push_str(&comment_delimited(block.name.as_deref(), &block.attrs, &content));
    }
    out
}

/// What `resolve_pattern_blocks()` needs to know about a registered pattern.
#[derive(Debug, Clone, Default)]
pub struct PatternRef {
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub content: String,
}

/// `sanitize_text_field()` for the strings of a pattern's metadata.
fn sanitize_text(text: &str) -> String {
    let stripped = rustpress_core::formatting::strip_tags(text);
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `resolve_pattern_blocks()`: `core/pattern` blocks are replaced with the blocks of the pattern.
pub fn resolve_pattern_blocks(blocks: Vec<Block>, lookup: &dyn Fn(&str) -> Option<PatternRef>) -> Vec<Block> {
    fn resolve(mut blocks: Vec<Block>, lookup: &dyn Fn(&str) -> Option<PatternRef>, seen: &mut Vec<String>) -> Vec<Block> {
        let mut i = 0;
        while i < blocks.len() {
            if blocks[i].name.as_deref() == Some("core/pattern") {
                let slug = blocks[i].attrs.get("slug").and_then(Value::as_str).unwrap_or_default().to_owned();
                if slug.is_empty() {
                    i += 1;
                    continue;
                }
                if seen.contains(&slug) {
                    blocks.remove(i);
                    continue;
                }
                let Some(pattern) = lookup(&slug) else {
                    i += 1;
                    continue;
                };
                let mut insert = parse_blocks(pattern.content.trim());
                if insert.len() == 1 {
                    let mut metadata = insert[0].attrs.get("metadata").and_then(Value::as_object).cloned().unwrap_or_default();
                    metadata.insert("patternName".into(), Value::String(slug.clone()));
                    let existing = |key: &str| metadata.get(key).cloned();
                    let name = if pattern.title.is_empty() { existing("name") } else { Some(Value::String(pattern.title.clone())) };
                    let description = if pattern.description.is_empty() { existing("description") } else { Some(Value::String(pattern.description.clone())) };
                    let categories = if pattern.categories.is_empty() { existing("categories") } else { Some(Value::Array(pattern.categories.iter().map(|c| Value::String(c.clone())).collect())) };
                    for (key, value) in [("name", name), ("description", description), ("categories", categories)] {
                        match value {
                            Some(Value::String(s)) if !s.is_empty() => {
                                metadata.insert(key.into(), Value::String(sanitize_text(&s)));
                            }
                            Some(Value::Array(items)) if !items.is_empty() => {
                                metadata.insert(key.into(), Value::Array(items.iter().map(|v| Value::String(sanitize_text(v.as_str().unwrap_or_default()))).collect()));
                            }
                            _ => {}
                        }
                    }
                    insert[0].attrs.insert("metadata".into(), Value::Object(metadata));
                }
                seen.push(slug.clone());
                let insert = resolve(insert, lookup, seen);
                seen.pop();
                let count = insert.len();
                blocks.splice(i..=i, insert);
                i += count;
            } else {
                if !blocks[i].inner_blocks.is_empty() {
                    let inner = std::mem::take(&mut blocks[i].inner_blocks);
                    let before = inner.len();
                    let resolved = resolve(inner, lookup, seen);
                    // The placeholders of the inner content follow the number of inner blocks.
                    if resolved.len() != before {
                        let mut content = Vec::new();
                        let mut placed = 0;
                        for chunk in std::mem::take(&mut blocks[i].inner_content) {
                            match chunk {
                                Content::Html(h) => content.push(Content::Html(h)),
                                Content::InnerBlock => {
                                    placed += 1;
                                    if placed == 1 {
                                        content.extend(std::iter::repeat_n(Content::InnerBlock, resolved.len()));
                                    }
                                }
                            }
                        }
                        blocks[i].inner_content = content;
                    }
                    blocks[i].inner_blocks = resolved;
                }
                i += 1;
            }
        }
        blocks
    }
    resolve(blocks, lookup, &mut Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_survive_a_round_trip() {
        let markup = "<!-- wp:template-part {\"slug\":\"header\"} /-->\n\n<!-- wp:group {\"tagName\":\"main\"} -->\n<main class=\"wp-block-group\"><!-- wp:paragraph -->\n<p>x</p>\n<!-- /wp:paragraph --></main>\n<!-- /wp:group -->\n";
        let blocks = parse_blocks(markup);
        assert_eq!(serialize_blocks(&blocks), markup);
    }

    #[test]
    fn attributes_are_escaped_like_wordpress() {
        let mut attrs = Map::new();
        attrs.insert("a".into(), Value::String("x--<y>&\"z".into()));
        assert_eq!(serialize_block_attributes(&attrs), "{\"a\":\"x\\u002d\\u002d\\u003cy\\u003e\\u0026\\u0022z\"}");
    }

    #[test]
    fn patterns_are_inserted_with_their_name() {
        let blocks = parse_blocks("<!-- wp:pattern {\"slug\":\"t/p\"} /-->");
        let lookup = |slug: &str| (slug == "t/p").then(|| PatternRef { title: "Hero".into(), description: String::new(), categories: vec!["text".into()], content: "<!-- wp:paragraph -->\n<p>Hi</p>\n<!-- /wp:paragraph -->".into() });
        let resolved = resolve_pattern_blocks(blocks, &lookup);
        assert_eq!(serialize_blocks(&resolved), "<!-- wp:paragraph {\"metadata\":{\"patternName\":\"t/p\",\"name\":\"Hero\",\"categories\":[\"text\"]}} -->\n<p>Hi</p>\n<!-- /wp:paragraph -->");
    }
}
