// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! M1 performance fixture: a 10,000-note notebook must list without blocking.
//!
//! Ignored by default because it creates 10,000 files. Run it with:
//! `cargo test -p lean4losophy-core --test performance -- --ignored --nocapture`

use std::time::{Duration, Instant};

use lean4losophy_core::Notebook;
use lean4losophy_core::index::{Index, SearchQuery};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
#[ignore = "creates 10,000 files; run explicitly for the M1 performance check"]
fn ten_thousand_notes_list_in_under_two_seconds() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = Notebook::create(dir.path())?;
    let notes_dir = notebook.notes_dir();

    for index in 0..10_000_u32 {
        std::fs::write(notes_dir.join(format!("note-{index:05}.md")), "# Note\n")?;
    }

    let start = Instant::now();
    let listed = notebook.list_notes()?;
    let elapsed = start.elapsed();

    assert_eq!(listed.len(), 10_000);
    assert!(
        elapsed < Duration::from_secs(2),
        "listing 10,000 notes took {elapsed:?}"
    );
    Ok(())
}

#[test]
#[ignore = "creates 10,000 files; run explicitly for the M2 search check"]
fn ten_thousand_notes_search_in_under_100ms() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = Notebook::create(dir.path())?;
    let notes_dir = notebook.notes_dir();

    for number in 0..10_000_u32 {
        let title = format!("Note {number}");
        let tag = if number % 2 == 0 {
            "logic"
        } else {
            "metaphysics"
        };
        let link = format!("notes/note-{:05}.md", (number + 1) % 10_000);
        let text = format!(
            "---\nformat: 1\ntitle: {title}\ntags: [{tag}]\n---\n\n\
             # {title}\n\n\
             Philosophy note number {number} about arguments, validity, and truth. \
             See [[{link}]].\n"
        );
        std::fs::write(notes_dir.join(format!("note-{number:05}.md")), text)?;
    }

    let build_start = Instant::now();
    let index = Index::open(&notebook)?;
    let build = build_start.elapsed();

    let search_start = Instant::now();
    let hits = index.search(&SearchQuery::new("validity"))?;
    let search = search_start.elapsed();

    assert!(!hits.is_empty());
    assert!(
        build < Duration::from_secs(30),
        "building the 10,000-note index took {build:?}"
    );
    assert!(
        search < Duration::from_millis(100),
        "searching 10,000 notes took {search:?}"
    );

    let counts = index.counts()?;
    assert_eq!(counts.notes, 10_000);
    assert_eq!(counts.links, 10_000);
    assert_eq!(counts.broken, 0);
    Ok(())
}
