//! The block render pipeline, following `WP_Block::render()` and the core
//! `render_block*` filters.

use std::sync::Arc;

use futures::future::BoxFuture;
use serde_json::{Map, Value};

use crate::Result;
use crate::dynamic;
use crate::env::{Context, Env};
use crate::layout;
use crate::parser::{Block, Content, parse_blocks};
use crate::registry::BlockType;
use crate::supports::wrapper_attributes_with;

/// Deeper blocks are not rendered (see [`Env::render_block`]).
pub const MAX_RENDER_DEPTH: u32 = 64;

/// Blocks that render their inner blocks themselves (`skip_inner_blocks`).
fn skips_inner_blocks(name: &str) -> bool {
    matches!(name, "core/post-template" | "core/comment-template" | "core/comments" | "core/term-template" | "core/navigation" | "core/navigation-link")
}

/// Everything a dynamic block's render callback receives.
pub struct BlockCtx<'a> {
    pub env: &'a Env<'a>,
    pub block: &'a Block,
    pub block_type: Option<Arc<BlockType>>,
    /// Prepared attributes (`$attributes`): defaults filled in.
    pub attrs: Map<String, Value>,
    /// Attributes as parsed, after `render_block_data` filters (what block supports read).
    pub raw_attrs: Map<String, Value>,
    /// Rendered inner blocks (`$content`).
    pub content: String,
    /// `$block->context`: only the values listed in `usesContext`.
    pub context: Context,
    /// Everything inherited from ancestors, for blocks that build their own children.
    pub available: &'a Context,
}

impl BlockCtx<'_> {
    /// `get_block_wrapper_attributes( $extra )`.
    pub fn wrapper_attributes(&self, extra: &[(&str, String)]) -> String {
        wrapper_attributes_with(self.block_type.as_deref(), &self.raw_attrs, extra, Some(self.env.theme.global_settings()))
    }

    pub fn attr_str(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).and_then(Value::as_str)
    }

    pub fn attr_bool(&self, key: &str) -> bool {
        self.attrs.get(key).and_then(Value::as_bool).unwrap_or(false)
    }

    pub fn attr_u64(&self, key: &str) -> Option<u64> {
        self.attrs.get(key).and_then(Value::as_u64)
    }
}

impl<'a> Env<'a> {
    /// `do_blocks( $content )`.
    pub async fn render_markup(&self, markup: &str, available: &Context) -> Result<String> {
        let blocks = parse_blocks(markup);
        self.render_blocks(&blocks, available, None).await
    }

    pub fn render_blocks<'b>(&'b self, blocks: &'b [Block], available: &'b Context, parent: Option<&'b Block>) -> BoxFuture<'b, Result<String>> {
        Box::pin(async move {
            let mut out = String::new();
            for block in blocks {
                out.push_str(&self.render_block(block, available, parent).await?);
            }
            Ok(out)
        })
    }

    /// Renders one block. Blocks nested deeper than [`MAX_RENDER_DEPTH`] keep their saved HTML: a post of
    /// thousands of nested groups must not exhaust the stack of the worker.
    pub fn render_block<'b>(&'b self, block: &'b Block, available: &'b Context, parent: Option<&'b Block>) -> BoxFuture<'b, Result<String>> {
        Box::pin(async move {
            let depth = self.with_state(|s| {
                s.render_depth += 1;
                s.render_depth
            });
            let result = if depth > MAX_RENDER_DEPTH { Ok(block.inner_html.clone()) } else { self.render_block_unguarded(block, available, parent).await };
            self.with_state(|s| s.render_depth -= 1);
            result
        })
    }

    fn render_block_unguarded<'b>(&'b self, block: &'b Block, available: &'b Context, parent: Option<&'b Block>) -> BoxFuture<'b, Result<String>> {
        Box::pin(async move {
            let Some(name) = block.name.as_deref() else { return Ok(block.inner_html.clone()) };
            // The `render_block_context` filter of the tabs block.
            let provided;
            let available = if name == "core/tabs" {
                provided = dynamic::tabs::provide_context(self, block, available);
                &provided
            } else {
                available
            };
            let block_type = self.registry.get(name);
            let (blocks_before, modules_before) = self.with_state(|s| (s.used_blocks.len(), s.script_modules.clone()));
            self.with_state(|s| {
                if !s.rendered_blocks.iter().any(|b| b == name) {
                    s.rendered_blocks.push(name.to_owned());
                }
            });

            // The first block that supports the Interactivity API processes the directives of all it contains.
            let is_root_interactive = !self.with_state(|s| s.interactive_root)
                && block_type.as_ref().is_some_and(|bt| matches!(bt.support(&["interactivity"]), Some(Value::Bool(true))) || bt.support(&["interactivity", "interactive"]).is_some_and(|v| !crate::style_engine::is_empty(v)));
            if is_root_interactive {
                self.with_state(|s| s.interactive_root = true);
            }

            // `render_block_data` filters.
            let mut raw_attrs = block.attrs.clone();
            let parent_layout = parent.and_then(|p| p.attrs.get("layout")).cloned();
            if let Some(bt) = &block_type {
                crate::elements::prepare_block_data(self, bt, &mut raw_attrs);
                crate::variations::prepare_block_data(self, bt, &mut raw_attrs);
            }
            let prepared = match &block_type {
                Some(bt) => bt.prepare_attributes(&raw_attrs),
                None => raw_attrs.clone(),
            };

            // The block's own context and the one it hands down.
            let mut context = Context { values: Map::new(), post: available.post.clone() };
            if let Some(bt) = &block_type {
                for key in &bt.uses_context {
                    if let Some(v) = available.values.get(key) {
                        context.values.insert(key.clone(), v.clone());
                    }
                }
            }
            let mut child_ctx = available.clone();
            if let Some(bt) = &block_type {
                for (context_name, attribute) in &bt.provides_context {
                    if let Some(v) = attribute.as_str().and_then(|a| prepared.get(a)) {
                        child_ctx.values.insert(context_name.clone(), v.clone());
                    }
                }
            }

            // Inner blocks.
            let mut content = String::new();
            if !skips_inner_blocks(name) || !dynamic::is_dynamic(name) {
                let mut index = 0;
                for chunk in &block.inner_content {
                    match chunk {
                        Content::Html(html) => content.push_str(html),
                        Content::InnerBlock => {
                            // A placeholder without its block (a hand-built tree) renders nothing.
                            let Some(inner) = block.inner_blocks.get(index) else { continue };
                            // `block_core_gallery_data_id_backcompatibility` (a `render_block_data` filter).
                            let with_data_id;
                            let inner = match inner.attrs.get("id") {
                                Some(id) if name == "core/gallery" && inner.name.as_deref() == Some("core/image") && !inner.attrs.contains_key("data-id") => {
                                    let text = id.as_str().map_or_else(|| id.to_string(), str::to_owned);
                                    let mut copy = inner.clone();
                                    copy.attrs.insert("data-id".into(), Value::String(rustpress_core::formatting::esc_attr(&text)));
                                    with_data_id = copy;
                                    &with_data_id
                                }
                                _ => inner,
                            };
                            content.push_str(&self.render_block(inner, &child_ctx, Some(block)).await?);
                            index += 1;
                        }
                    }
                }
            }

            if dynamic::is_dynamic(name) {
                let bc = BlockCtx {
                    env: self,
                    block,
                    block_type: block_type.clone(),
                    attrs: prepared,
                    raw_attrs: raw_attrs.clone(),
                    content,
                    context,
                    available: &child_ctx,
                };
                content = dynamic::render(&bc).await?;
            }

            // `render_block` filters.
            if let Some(bt) = &block_type {
                content = crate::supports::render_typography_support(self, &raw_attrs, content);
                content = crate::elements::render_class_name(&raw_attrs, content);
                let first_chunk = match block.inner_content.first() {
                    Some(Content::Html(html)) if block.inner_content.len() > 1 => Some(html.as_str()),
                    _ => None,
                };
                content = layout::render_layout_support(
                    self,
                    bt,
                    &raw_attrs,
                    parent_layout.as_ref(),
                    first_chunk,
                    !block.inner_blocks.is_empty(),
                    content,
                );
                content = layout::render_position_support(self, bt, &raw_attrs, content);
                content = crate::dimensions::render_dimensions_support(bt, &raw_attrs, content);
                content = crate::duotone::render_duotone_support(self, bt, name, &raw_attrs, content);
                content = crate::variations::render_class_name(&raw_attrs, content);
            }
            content = dynamic::name_filter(name, &raw_attrs, content);
            if is_root_interactive {
                content = self.with_state(|s| {
                    s.interactive_root = false;
                    crate::interactivity::process_directives(&content, &mut s.interactivity)
                });
            }
            // Blocks that rendered nothing do not load their assets; parents come after their children.
            // Assets enqueued while rendering a block that turns out empty are dequeued again,
            // including those of its inner blocks (`WP_Block::render()`).
            self.with_state(|s| s.note_block(name));
            if content.trim().is_empty() {
                self.with_state(|s| {
                    s.used_blocks.truncate(blocks_before);
                    s.script_modules = modules_before.clone();
                });
            }
            Ok(content)
        })
    }
}
