// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Core data model for lean4losophy.
//!
//! This crate owns the parts of the application that do not depend on the GUI
//! or on Lean: the notebook folder, Markdown notes, front matter, settings,
//! crash-safe writes, wiki links, and the derivable SQLite index used for
//! backlinks and search. Structured arguments arrive in M3.
//!
//! # Guarantees
//!
//! - No `unsafe` code. The workspace denies it, and this crate has no FFI.
//! - No network access. A notebook is local files only.
//! - Errors are returned as values; user input never panics the core.
//! - Every write is atomic: a crash leaves the old note or the new note, never
//!   a partial one.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let notebook = lean4losophy_core::Notebook::create(Path::new("/tmp/lean4losophy-example"))?;
//! let mut note = notebook.create_note("Reading list")?;
//! note.set_body("# Reading list\n\n- Anscombe, *Intention*\n".to_owned());
//! notebook.save_note(&note)?;
//! # Ok(())
//! # }
//! ```

pub mod atomic;
pub mod error;
pub mod frontmatter;
pub mod index;
pub mod links;
pub mod note;
pub mod notebook;
pub mod paths;
pub mod preview;
pub mod settings;

pub use error::{Error, Result};
pub use frontmatter::FrontMatter;
pub use index::{Index, IndexCounts, LinkProblem, LinkResolution, SearchHit, SearchQuery};
pub use links::{Heading, ParsedBody, WikiLink};
pub use note::Note;
pub use notebook::{Notebook, slugify};
pub use settings::{EffectiveSettings, GlobalConfig, Mode, NotebookSettings, PreviewLayout};

/// The version of `lean4losophy-core` compiled into this binary.
///
/// The value comes from Cargo at build time and is always a valid semantic
/// version string.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_matches_the_manifest() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
