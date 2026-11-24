// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Preview block tests: the supported CommonMark subset, tables, footnotes, and
//! the rule that raw HTML never reaches the renderer.

use lean4losophy_core::preview::{Block, blocks};

#[test]
fn headings_and_paragraphs() {
    let parsed = blocks("# Title\n\nSome *emphasis* and **strong** text.\n");
    assert_eq!(
        parsed,
        vec![
            Block::Heading {
                level: 1,
                text: "Title".to_owned()
            },
            Block::Paragraph("Some emphasis and strong text.".to_owned()),
        ]
    );
}

#[test]
fn bullet_and_ordered_lists() {
    let parsed = blocks("- one\n- two\n\n1. first\n2. second\n");
    assert_eq!(parsed.len(), 2);

    match &parsed[0] {
        Block::List { ordered, items, .. } => {
            assert!(!ordered);
            assert_eq!(items.len(), 2);
            assert_eq!(items[0], vec![Block::Paragraph("one".to_owned())]);
        }
        other => assert_eq!(other, &Block::Rule, "expected a bullet list"),
    }

    match &parsed[1] {
        Block::List { ordered, start, .. } => {
            assert!(ordered);
            assert_eq!(*start, 1);
        }
        other => assert_eq!(other, &Block::Rule, "expected an ordered list"),
    }
}

#[test]
fn code_blocks_keep_their_language_and_text() {
    let parsed = blocks("```lean\ntheorem t : True := trivial\n```\n");
    assert_eq!(
        parsed,
        vec![Block::CodeBlock {
            language: Some("lean".to_owned()),
            text: "theorem t : True := trivial\n".to_owned(),
        }]
    );
}

#[test]
fn tables_have_headers_and_rows() {
    let parsed = blocks("| Name | Year |\n| --- | --- |\n| Anscombe | 1957 |\n");
    assert_eq!(
        parsed,
        vec![Block::Table {
            headers: vec!["Name".to_owned(), "Year".to_owned()],
            rows: vec![vec!["Anscombe".to_owned(), "1957".to_owned()]],
        }]
    );
}

#[test]
fn quotes_nest_their_blocks() {
    let parsed = blocks("> A quoted claim.\n");
    assert_eq!(
        parsed,
        vec![Block::Quote(vec![Block::Paragraph(
            "A quoted claim.".to_owned()
        )])]
    );
}

#[test]
fn footnotes_are_collected_at_the_end() {
    let parsed = blocks("A claim.[^1]\n\n[^1]: The source.\n");
    match parsed.last() {
        Some(Block::Footnotes(notes)) => {
            assert_eq!(notes.len(), 1);
            assert_eq!(notes[0].0, "1");
            assert_eq!(notes[0].1, "The source.");
        }
        other => assert_eq!(other, None, "expected a footnotes block"),
    }
}

#[test]
fn raw_html_never_appears() {
    let parsed = blocks("<script>alert('x')</script>\n\nText <b>bold</b>.\n");
    let rendered = format!("{parsed:?}");
    assert!(
        !rendered.contains("<script>"),
        "script tag leaked: {rendered}"
    );
    assert!(!rendered.contains("<b>"), "inline HTML leaked: {rendered}");
    assert!(rendered.contains("Text bold."), "text was lost: {rendered}");
}

#[test]
fn wiki_links_stay_literal_until_m2() {
    let parsed = blocks("See [[Another note]].\n");
    assert_eq!(
        parsed,
        vec![Block::Paragraph("See [[Another note]].".to_owned())]
    );
}

#[test]
fn arguments_stay_readable_lists() {
    let parsed =
        blocks("- Argument: Mortality\n  1. All humans are mortal.\n  2. ∴ Socrates is mortal.\n");
    let rendered = format!("{parsed:?}");
    assert!(rendered.contains("Argument: Mortality"));
    assert!(rendered.contains("∴ Socrates is mortal."));
}
