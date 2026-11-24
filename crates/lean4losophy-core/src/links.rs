// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Wiki links, heading anchors, and block IDs (`FORMAT.md` section 5).
//!
//! Links are extracted from the parsed Markdown AST, not with a regular
//! expression over the file, so a `[[...]]` inside a code span or a fenced
//! block can never become a link. Block IDs are found on block end lines and
//! masked out of the text before links and headings are read, which keeps every
//! remaining byte offset stable for link rewriting.

use std::ops::Range;

use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};
use unicode_normalization::UnicodeNormalization;

/// What kind of block carries a block ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// A paragraph.
    Paragraph,
    /// A list item, identified by its last paragraph.
    ListItem,
    /// A block quote.
    Quote,
    /// A table.
    Table,
    /// A fenced code block, including Lean blocks.
    Code,
}

impl BlockKind {
    /// The value stored in the index `blocks.kind` column.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Paragraph => "paragraph",
            Self::ListItem => "list-item",
            Self::Quote => "quote",
            Self::Table => "table",
            Self::Code => "lean",
        }
    }
}

/// A block marked with `^id` at the end of its last line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRef {
    /// The identifier, without the caret.
    pub id: String,
    /// The block kind.
    pub kind: BlockKind,
    /// The 1-based line of the block ID.
    pub line: u32,
}

/// A heading with the anchor a link can target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    /// Heading level, 1 to 6.
    pub level: u8,
    /// Text with inline markup removed.
    pub text: String,
    /// The normalized anchor (`FORMAT.md` section 5.4).
    pub anchor: String,
    /// The 1-based line of the heading.
    pub line: u32,
}

/// One `[[...]]` link as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    /// The note part, trimmed. Empty only for `[[#Heading]]`.
    pub target: String,
    /// Display text after `|`, when present and non-empty.
    pub alias: Option<String>,
    /// Heading text after `#`, when the fragment is not a block ID.
    pub heading: Option<String>,
    /// Block ID after `#^`, when present.
    pub block_id: Option<String>,
    /// 1-based line.
    pub line: u32,
    /// 1-based byte column.
    pub column: u32,
    /// Byte range of the whole `[[...]]` span in the masked body.
    pub span: Range<usize>,
    /// True when the target has a forbidden character or an empty note part
    /// without a heading.
    pub malformed: bool,
}

impl WikiLink {
    /// The text inside the brackets, for display and rewriting.
    #[must_use]
    pub fn inside(&self) -> String {
        let mut inside = self.target.clone();
        if let Some(heading) = &self.heading {
            inside.push('#');
            inside.push_str(heading);
        }
        if let Some(block_id) = &self.block_id {
            inside.push_str("#^");
            inside.push_str(block_id);
        }
        if let Some(alias) = &self.alias {
            inside.push('|');
            inside.push_str(alias);
        }
        inside
    }

    /// The text shown for the link in the preview and backlink lists.
    #[must_use]
    pub fn display(&self) -> String {
        if let Some(alias) = &self.alias {
            return alias.clone();
        }
        if let Some(heading) = &self.heading {
            let note = if self.target.is_empty() {
                heading.clone()
            } else {
                format!("{} › {heading}", self.target)
            };
            return note;
        }
        if let Some(block_id) = &self.block_id {
            if self.target.is_empty() {
                return format!("^{block_id}");
            }
            return format!("{} › ^{block_id}", self.target);
        }
        self.target.clone()
    }
}

/// The parts of a link target, before resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkParts {
    /// The note part.
    pub target: String,
    /// Optional alias.
    pub alias: Option<String>,
    /// Optional heading fragment.
    pub heading: Option<String>,
    /// Optional block fragment.
    pub block_id: Option<String>,
    /// Whether the target itself is malformed.
    pub malformed: bool,
}

/// Everything the index needs from one note body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedBody {
    /// Links in document order.
    pub links: Vec<WikiLink>,
    /// Headings in document order, with unique anchors.
    pub headings: Vec<Heading>,
    /// Blocks that carry an ID.
    pub blocks: Vec<BlockRef>,
}

/// Parse a note body for links, headings, and block IDs.
///
/// Never fails; malformed Markdown is still parsed as CommonMark.
#[must_use]
pub fn parse(body: &str) -> ParsedBody {
    let (masked, blocks) = mask_block_ids(body);

    let mut options = Options::default();
    options.extension.table = true;
    options.extension.footnotes = true;

    let arena = Arena::new();
    let root = parse_document(&arena, &masked, &options);
    let line_index = LineIndex::new(&masked);

    let mut links = Vec::new();
    let mut headings = Vec::new();
    let mut anchors: Vec<String> = Vec::new();

    collect(
        root,
        &masked,
        &line_index,
        &mut links,
        &mut headings,
        &mut anchors,
    );

    ParsedBody {
        links,
        headings,
        blocks,
    }
}

/// Replace trailing `^id` block markers with spaces, preserving every offset.
///
/// The preview renders the masked text so block IDs never appear as content.
#[must_use]
pub fn mask_block_ids(body: &str) -> (String, Vec<BlockRef>) {
    let blocks = find_block_ids(body);
    let mut masked = body.as_bytes().to_vec();

    for span in find_block_id_spans(body, &blocks) {
        if let Some(slice) = masked.get_mut(span) {
            slice.fill(b' ');
        }
    }

    // The original was valid UTF-8 and only ASCII bytes were replaced.
    let text = String::from_utf8(masked).unwrap_or_else(|_| body.to_owned());
    (text, blocks)
}

/// Normalize text for title and alias matching: NFC and lowercase, trimmed.
#[must_use]
pub fn normalize_key(text: &str) -> String {
    text.trim().nfc().collect::<String>().to_lowercase()
}

/// Derive a heading anchor exactly as `FORMAT.md` section 5.4 describes.
#[must_use]
pub fn normalize_anchor(text: &str) -> String {
    let normalized = text.trim().nfc().collect::<String>().to_lowercase();
    let mut anchor = String::with_capacity(normalized.len());

    for character in normalized.chars() {
        if character.is_alphanumeric() || character == '-' || character == '_' {
            anchor.push(character);
        } else if character.is_whitespace() {
            anchor.push(' ');
        }
    }

    let mut collapsed = String::with_capacity(anchor.len());
    let mut pending_space = false;
    let mut pending_hyphen = false;
    for character in anchor.chars() {
        match character {
            ' ' => {
                pending_space = true;
            }
            '-' => {
                pending_space = false;
                pending_hyphen = true;
            }
            _ => {
                if (pending_space || pending_hyphen) && !collapsed.is_empty() {
                    collapsed.push('-');
                }
                pending_space = false;
                pending_hyphen = false;
                collapsed.push(character);
            }
        }
    }

    if collapsed.is_empty() {
        String::from("section")
    } else {
        collapsed
    }
}

/// Whether a target is a 26-character Crockford base32 ULID.
#[must_use]
pub fn is_ulid(target: &str) -> bool {
    target.len() == 26
        && target
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'A'..='H' | 'J' | 'K' | 'M' | 'N' | 'P'..='T' | 'V'..='Z'))
}

/// Parse the text inside `[[...]]`.
///
/// Returns `None` only for an empty span. A malformed target is returned with
/// `malformed: true` so the caller can report it rather than guess.
#[must_use]
pub fn parse_inside(inside: &str) -> Option<LinkParts> {
    if inside.is_empty() {
        return None;
    }

    let (target_part, alias) = match inside.split_once('|') {
        Some((target, alias)) => {
            let alias = alias.trim();
            let alias = if alias.is_empty() {
                None
            } else {
                Some(alias.to_owned())
            };
            (target, alias)
        }
        None => (inside, None),
    };

    let (note, fragment) = match target_part.split_once('#') {
        Some((note, fragment)) => (note, Some(fragment)),
        None => (target_part, None),
    };

    let target = note.trim().to_owned();
    let mut malformed = target.contains(['[', ']', '|', '#', '\n'])
        || alias.as_deref().is_some_and(|text| text.contains('\n'));

    let fragment = fragment.map(str::trim).filter(|text| !text.is_empty());
    let mut heading = None;
    let mut block_id = None;

    if let Some(fragment) = fragment {
        if let Some(id) = fragment.strip_prefix('^') {
            if is_block_id(id) {
                block_id = Some(id.to_owned());
            } else {
                malformed = true;
            }
        } else {
            heading = Some(fragment.to_owned());
        }
    }

    if target.is_empty() && heading.is_none() && block_id.is_none() {
        malformed = true;
    }

    Some(LinkParts {
        target,
        alias,
        heading,
        block_id,
        malformed,
    })
}

/// Scan plain text for `[[...]]` links, without an AST.
///
/// This is for preview text that has already been stripped of Markdown; code
/// spans are skipped only when the caller splits them first. Slices returned in
/// `span` are relative to `text`.
#[must_use]
pub fn scan_text(text: &str) -> Vec<WikiLink> {
    let mut links = Vec::new();
    let mut offset = 0;

    while let Some(open) = text[offset..].find("[[") {
        let open = offset + open;
        let Some(close) = text[open + 2..].find("]]") else {
            break;
        };
        let close = open + 2 + close;
        let inside = &text[open + 2..close];

        if open > 0 && text.as_bytes().get(open.saturating_sub(1)) == Some(&b'!') {
            offset = close + 2;
            continue;
        }

        if let Some(parts) = parse_inside(inside) {
            links.push(WikiLink {
                target: parts.target,
                alias: parts.alias,
                heading: parts.heading,
                block_id: parts.block_id,
                line: 1,
                column: 1,
                span: open..close + 2,
                malformed: parts.malformed,
            });
        }

        offset = close + 2;
    }

    links
}

fn is_block_id(id: &str) -> bool {
    let mut characters = id.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    let rest = characters.as_str();
    !rest.contains(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        && id.len() <= 64
}

struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (offset, character) in text.char_indices() {
            if character == '\n' {
                starts.push(offset + 1);
            }
        }
        Self { starts }
    }

    fn line_start(&self, line: u32) -> Option<usize> {
        usize::try_from(line)
            .ok()
            .and_then(|line| line.checked_sub(1))
            .and_then(|index| self.starts.get(index).copied())
    }

    fn line_of(&self, offset: usize) -> u32 {
        let line = self.starts.partition_point(|start| *start <= offset);
        u32::try_from(line).unwrap_or(u32::MAX)
    }
}

fn find_block_ids(body: &str) -> Vec<BlockRef> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.footnotes = true;

    let arena = Arena::new();
    let root = parse_document(&arena, body, &options);
    let lines: Vec<&str> = body.split('\n').collect();
    let mut blocks = Vec::new();
    collect_block_ids(root, &lines, &mut blocks);

    let mut unique: Vec<BlockRef> = Vec::with_capacity(blocks.len());
    for block in blocks {
        if !unique.iter().any(|known| known.id == block.id) {
            unique.push(block);
        }
    }
    unique.sort_by_key(|block| block.line);
    unique
}

fn collect_block_ids<'a>(node: &'a AstNode<'a>, lines: &[&str], blocks: &mut Vec<BlockRef>) {
    for child in node.children() {
        let value = &child.data.borrow().value;

        match value {
            NodeValue::Paragraph => {
                let line =
                    u32::try_from(child.data.borrow().sourcepos.end.line).unwrap_or(u32::MAX);
                if let Some(id) = lines
                    .get(
                        usize::try_from(line)
                            .unwrap_or(usize::MAX)
                            .saturating_sub(1),
                    )
                    .and_then(|source| trailing_block_id(source))
                {
                    let kind = ancestor_kind(child).unwrap_or(BlockKind::Paragraph);
                    blocks.push(BlockRef { id, kind, line });
                }
            }
            NodeValue::Table(_) => {
                let line =
                    u32::try_from(child.data.borrow().sourcepos.end.line).unwrap_or(u32::MAX);
                if let Some(id) = lines
                    .get(
                        usize::try_from(line)
                            .unwrap_or(usize::MAX)
                            .saturating_sub(1),
                    )
                    .and_then(|source| trailing_block_id(source))
                {
                    blocks.push(BlockRef {
                        id,
                        kind: BlockKind::Table,
                        line,
                    });
                }
            }
            NodeValue::CodeBlock(code) => {
                if let Some(id) = info_block_id(&code.info) {
                    let line =
                        u32::try_from(child.data.borrow().sourcepos.start.line).unwrap_or(u32::MAX);
                    blocks.push(BlockRef {
                        id,
                        kind: BlockKind::Code,
                        line,
                    });
                }
            }
            _ => {}
        }

        collect_block_ids(child, lines, blocks);
    }
}

fn ancestor_kind<'a>(node: &'a AstNode<'a>) -> Option<BlockKind> {
    let mut current = node.parent();
    while let Some(parent) = current {
        match &parent.data.borrow().value {
            NodeValue::Item(_) => return Some(BlockKind::ListItem),
            NodeValue::BlockQuote => return Some(BlockKind::Quote),
            NodeValue::Table(_) => return Some(BlockKind::Table),
            NodeValue::Document => return None,
            _ => current = parent.parent(),
        }
    }
    None
}

fn trailing_block_id(line: &str) -> Option<String> {
    let trimmed = line.trim_end();
    let space = trimmed.rfind(' ')?;
    let id = trimmed[space + 1..].strip_prefix('^')?;
    if is_block_id(id) {
        Some(id.to_owned())
    } else {
        None
    }
}

fn info_block_id(info: &str) -> Option<String> {
    let start = info.find("{#")?;
    let rest = &info[start + 2..];
    let end = rest.find('}')?;
    let id = &rest[..end];
    if is_block_id(id) {
        Some(id.to_owned())
    } else {
        None
    }
}

fn find_block_id_spans(body: &str, blocks: &[BlockRef]) -> Vec<Range<usize>> {
    let line_index = LineIndex::new(body);
    let lines: Vec<&str> = body.split('\n').collect();
    let mut spans = Vec::new();

    for block in blocks {
        let index = match usize::try_from(block.line)
            .ok()
            .and_then(|line| line.checked_sub(1))
        {
            Some(index) => index,
            None => continue,
        };
        let Some(line) = lines.get(index) else {
            continue;
        };
        let Some(line_start) = line_index.line_start(block.line) else {
            continue;
        };
        let trimmed = line.trim_end();
        if let Some(space) = trimmed.rfind(' ') {
            let token = &trimmed[space + 1..];
            if let Some(id) = token.strip_prefix('^')
                && id == block.id
            {
                spans.push(line_start + space + 1..line_start + trimmed.len());
            }
        }
    }

    spans
}

fn collect<'a>(
    node: &'a AstNode<'a>,
    masked: &str,
    line_index: &LineIndex,
    links: &mut Vec<WikiLink>,
    headings: &mut Vec<Heading>,
    anchors: &mut Vec<String>,
) {
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Text(text) => collect_text_links(text, child, masked, line_index, links),
            NodeValue::Heading(heading) => {
                let text = inline_text(child);
                let base = normalize_anchor(&text);
                let mut anchor = base.clone();
                let mut counter = 1_u32;
                while anchors.iter().any(|known| known == &anchor) {
                    anchor = format!("{base}-{counter}");
                    counter = counter.saturating_add(1);
                }
                anchors.push(anchor.clone());
                headings.push(Heading {
                    level: heading.level,
                    text,
                    anchor,
                    line: u32::try_from(child.data.borrow().sourcepos.start.line)
                        .unwrap_or(u32::MAX),
                });
            }
            _ => {}
        }

        collect(child, masked, line_index, links, headings, anchors);
    }
}

fn collect_text_links(
    text: &str,
    node: &AstNode<'_>,
    masked: &str,
    line_index: &LineIndex,
    links: &mut Vec<WikiLink>,
) {
    let sourcepos = node.data.borrow().sourcepos;
    let Some(line_start) =
        line_index.line_start(u32::try_from(sourcepos.start.line).unwrap_or(u32::MAX))
    else {
        return;
    };
    let node_offset = line_start + sourcepos.start.column.saturating_sub(1);

    let mut offset = 0;
    while let Some(open) = text[offset..].find("[[") {
        let open = offset + open;
        let Some(close) = text[open + 2..].find("]]") else {
            break;
        };
        let close = open + 2 + close;

        if open > 0 && text.as_bytes().get(open - 1) == Some(&b'!') {
            offset = close + 2;
            continue;
        }

        let inside = &text[open + 2..close];
        let absolute = node_offset + open;
        let span = absolute..absolute + (close - open) + 2;

        // Entity references and backslash escapes change the literal length, so
        // only accept a span that still points at the source brackets.
        if !masked
            .get(span.clone())
            .is_some_and(|slice| slice.starts_with("[["))
        {
            offset = close + 2;
            continue;
        }

        if let Some(parts) = parse_inside(inside) {
            links.push(WikiLink {
                target: parts.target,
                alias: parts.alias,
                heading: parts.heading,
                block_id: parts.block_id,
                line: line_index.line_of(absolute),
                column: u32::try_from(sourcepos.start.column + open).unwrap_or(u32::MAX),
                span,
                malformed: parts.malformed,
            });
        }

        offset = close + 2;
    }
}

fn inline_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    collect_inline(node, &mut text);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_inline<'a>(node: &'a AstNode<'a>, text: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(content) => text.push_str(content),
        NodeValue::Code(code) => text.push_str(&code.literal),
        NodeValue::SoftBreak | NodeValue::LineBreak => text.push(' '),
        NodeValue::HtmlInline(_) | NodeValue::HtmlBlock(_) => {}
        _ => {
            for child in node.children() {
                collect_inline(child, text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlockKind, is_ulid, mask_block_ids, normalize_anchor, normalize_key, parse, parse_inside,
        scan_text,
    };

    #[test]
    fn parses_the_four_link_forms() {
        let body = "See [[Plato]], [[Plato|the teacher]], [[Plato#Cave]], [[Plato#^cave-1]].\n";
        let parsed = parse(body);
        assert_eq!(parsed.links.len(), 4);
        assert_eq!(parsed.links[0].target, "Plato");
        assert_eq!(parsed.links[1].alias.as_deref(), Some("the teacher"));
        assert_eq!(parsed.links[2].heading.as_deref(), Some("Cave"));
        assert_eq!(parsed.links[3].block_id.as_deref(), Some("cave-1"));
        assert!(parsed.links.iter().all(|link| !link.malformed));
    }

    #[test]
    fn ignores_links_in_code_and_non_links() {
        let body = "`[[code]]` and\n\n```\n[[fenced]]\n```\n\n![[embed]] and [[real]].\n";
        let parsed = parse(body);
        assert_eq!(parsed.links.len(), 1);
        assert_eq!(parsed.links[0].target, "real");
    }

    #[test]
    fn reports_malformed_targets() {
        let parsed = parse("[[bad|alias with ]] end]] and [[ ]].\n");
        assert_eq!(parsed.links.len(), 2);
        assert!(!parsed.links[0].malformed);
        assert!(parsed.links[1].malformed);
    }

    #[test]
    fn anchors_follow_the_spec() {
        assert_eq!(normalize_anchor("  The  Cave,  again! "), "the-cave-again");
        assert_eq!(normalize_anchor("——"), "section");
        assert_eq!(normalize_anchor("Greek: Λόγος"), "greek-λόγος");
    }

    #[test]
    fn duplicate_anchors_are_numbered() {
        let parsed = parse("# Same\n\n# Same\n\n# Same\n");
        let anchors: Vec<&str> = parsed
            .headings
            .iter()
            .map(|heading| heading.anchor.as_str())
            .collect();
        assert_eq!(anchors, ["same", "same-1", "same-2"]);
    }

    #[test]
    fn block_ids_are_masked_and_typed() {
        let body = "A paragraph ^para-1\n\n- item ^item-1\n\n> quote ^quote-1\n\n| a | b |\n| - | - |\n| 1 | 2 | ^table-1\n\n```lean {#lean-1}\nexample : True := trivial\n```\n";
        let (masked, blocks) = mask_block_ids(body);
        assert!(!masked.contains("^para-1"));
        assert!(!masked.contains("^table-1"));
        assert!(masked.starts_with("A paragraph"));
        let kinds: Vec<BlockKind> = blocks.iter().map(|block| block.kind).collect();
        assert_eq!(
            kinds,
            [
                BlockKind::Paragraph,
                BlockKind::ListItem,
                BlockKind::Quote,
                BlockKind::Table,
                BlockKind::Code
            ]
        );
        assert_eq!(blocks[3].id, "table-1");
    }

    #[test]
    fn ulid_and_key_normalization() {
        assert!(is_ulid("01J8Z9K3T7VQ2M8N4B6C0XYD5R"));
        assert!(!is_ulid("01J8Z9K3T7VQ2M8N4B6C0XYD5"));
        assert_eq!(normalize_key("  Mortality  "), "mortality");
        assert!(parse_inside("Plato#^cave-1").is_some());
        assert_eq!(scan_text("see [[Plato]].").len(), 1);
    }
}
