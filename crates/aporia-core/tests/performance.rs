// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! M1 performance fixture: a 10,000-note notebook must list without blocking.
//!
//! Ignored by default because it creates 10,000 files. Run it with:
//! `cargo test -p aporia-core --test performance -- --ignored --nocapture`

use std::time::{Duration, Instant};

use aporia_core::Notebook;
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
