// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Markdown to a preview block model.
//!
//! One parser serves the preview: comrak with only the table and footnote
//! extensions enabled (ADR-0002). Raw HTML is parsed but never surfaced, so a
//! note cannot inject markup into the preview.
//!
//! The output is deliberately backend-neutral. M1 renders it with GTK widgets;
//! the WebKitGTK renderer named in ADR-0002 can consume the same blocks later
//! without a second parser or a second security model.

use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{Arena, Options, parse_document};

/// One block in the rendered preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A heading with its level (1 to 6) and plain text.
    Heading {
        /// Heading level.
        level: u8,
        /// Heading text with inline markup removed.
        text: String,
    },
    /// A paragraph with inline markup removed.
    Paragraph(String),
    /// A list; each item is a run of blocks.
    List {
        /// Whether the list is ordered.
        ordered: bool,
        /// The first number of an ordered list.
        start: u64,
        /// Items in source order.
        items: Vec<Vec<Block>>,
    },
    /// A fenced or indented code block.
    CodeBlock {
        /// The info string's first word, when present.
        language: Option<String>,
        /// The code, byte for byte.
        text: String,
    },
    /// A block quote.
    Quote(Vec<Block>),
    /// A table: the header row and the body rows.
    Table {
        /// Header cells.
        headers: Vec<String>,
        /// Body rows.
        rows: Vec<Vec<String>>,
    },
    /// A thematic break.
    Rule,
    /// Footnote definitions, appended at the end of the document.
    Footnotes(Vec<(String, String)>),
}

/// Parse Markdown into preview blocks.
///
/// Never fails: malformed Markdown still produces a document. HTML is dropped.
#[must_use]
pub fn blocks(markdown: &str) -> Vec<Block> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.footnotes = true;

    // Block IDs are addressing metadata, not content: mask them before parsing
    // so they never render (`FORMAT.md` section 5.3).
    let (masked, _) = crate::links::mask_block_ids(markdown);

    let arena = Arena::new();
    let root = parse_document(&arena, &masked, &options);

    let mut blocks = Vec::new();
    let mut footnotes = Vec::new();
    collect(root, &mut blocks, &mut footnotes);

    if !footnotes.is_empty() {
        blocks.push(Block::Footnotes(footnotes));
    }

    blocks
}

fn collect<'a>(
    node: &'a AstNode<'a>,
    blocks: &mut Vec<Block>,
    footnotes: &mut Vec<(String, String)>,
) {
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Heading(heading) => blocks.push(Block::Heading {
                level: heading.level,
                text: inline_text(child),
            }),
            NodeValue::Paragraph => blocks.push(Block::Paragraph(inline_text(child))),
            NodeValue::CodeBlock(code) => blocks.push(Block::CodeBlock {
                language: code.info.split_whitespace().next().map(str::to_owned),
                text: code.literal.clone(),
            }),
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                let mut inner = Vec::new();
                collect(child, &mut inner, footnotes);
                blocks.push(Block::Quote(inner));
            }
            NodeValue::List(list) => {
                let mut items = Vec::new();
                for item in child.children() {
                    let mut item_blocks = Vec::new();
                    collect(item, &mut item_blocks, footnotes);
                    items.push(item_blocks);
                }
                blocks.push(Block::List {
                    ordered: list.list_type == ListType::Ordered,
                    start: u64::try_from(list.start).unwrap_or(1),
                    items,
                });
            }
            NodeValue::Table(table) => blocks.push(block_table(child, table.num_columns)),
            NodeValue::ThematicBreak => blocks.push(Block::Rule),
            NodeValue::FootnoteDefinition(definition) => {
                footnotes.push((definition.name.clone(), inline_text(child)));
            }
            NodeValue::HtmlBlock(_) | NodeValue::HtmlInline(_) => {}
            NodeValue::Item(_) => {
                // A list item reached outside a list: keep its content.
                collect(child, blocks, footnotes);
            }
            NodeValue::Document => collect(child, blocks, footnotes),
            _ => {
                // Inline nodes at block level, and extensions we do not model,
                // contribute their children rather than disappearing silently.
                if child.first_child().is_some() {
                    collect(child, blocks, footnotes);
                }
            }
        }
    }
}

fn block_table<'a>(table: &'a AstNode<'a>, num_columns: usize) -> Block {
    let mut headers = Vec::new();
    let mut rows = Vec::new();

    for row in table.children() {
        let is_header = match &row.data.borrow().value {
            NodeValue::TableRow(header) => *header,
            _ => false,
        };
        let mut cells = Vec::new();
        for cell in row.children() {
            cells.push(inline_text(cell));
        }
        cells.resize(num_columns, String::new());

        if is_header && headers.is_empty() {
            headers = cells;
        } else {
            rows.push(cells);
        }
    }

    Block::Table { headers, rows }
}

fn inline_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    collect_inline(node, &mut text);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_inline<'a>(node: &'a AstNode<'a>, text: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(content) => text.push_str(content),
        NodeValue::Code(code) => {
            text.push('`');
            text.push_str(&code.literal);
            text.push('`');
        }
        NodeValue::SoftBreak | NodeValue::LineBreak => text.push(' '),
        NodeValue::FootnoteReference(reference) => {
            text.push_str(&format!("[{}]", reference.ix.saturating_add(1)));
        }
        NodeValue::HtmlInline(_) | NodeValue::HtmlBlock(_) => {}
        NodeValue::Link(link) if !link.title.is_empty() => {
            for child in node.children() {
                collect_inline(child, text);
            }
            text.push_str(" (");
            text.push_str(&link.title);
            text.push(')');
        }
        _ => {
            for child in node.children() {
                collect_inline(child, text);
            }
        }
    }
}
