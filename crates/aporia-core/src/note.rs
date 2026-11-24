// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! A single Markdown note with front matter (`FORMAT.md` sections 2 to 4).
//!
//! Parsing never loses bytes it does not understand: unknown front-matter keys
//! and the whole body round-trip. Saving is atomic (`FORMAT.md` section 13).

use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::atomic::write_atomic;
use crate::error::{Error, Result};
use crate::frontmatter::FrontMatter;

/// A note file: a path, its front matter, and the Markdown body.
#[derive(Debug, Clone)]
pub struct Note {
    path: PathBuf,
    front: FrontMatter,
    body: String,
    had_front_matter: bool,
}

impl Note {
    /// Parse note text read from `path`.
    ///
    /// # Errors
    ///
    /// Returns the front-matter errors documented on [`FrontMatter::parse`].
    pub fn parse(path: PathBuf, text: &str) -> Result<Self> {
        match split_front_matter(text) {
            Some((yaml, body)) => Ok(Self {
                front: FrontMatter::parse(yaml, &path)?,
                path,
                body: body.to_owned(),
                had_front_matter: true,
            }),
            None => Ok(Self {
                path,
                front: FrontMatter::empty(),
                body: text.to_owned(),
                had_front_matter: false,
            }),
        }
    }

    /// Read and parse a note file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Read`] for I/O failures, [`Error::InvalidUtf8`] for
    /// non-UTF-8 bytes, and the parse errors from [`Note::parse`].
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|source| Error::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let text = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8 {
            path: path.to_path_buf(),
        })?;
        Self::parse(path.to_path_buf(), &text)
    }

    /// Create a new note with a fresh ULID, timestamps, and an H1 heading.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FrontMatterYaml`] if rendering the new front matter
    /// fails, which cannot happen for the fixed fields used here.
    pub fn new(path: PathBuf, title: &str) -> Result<Self> {
        let title = title.trim();
        let title = if title.is_empty() { "Untitled" } else { title };
        let timestamp = now();

        let mut front = FrontMatter::empty();
        front.set_format(1);
        front.set_id(&ulid::Ulid::generate().to_string());
        front.set_title(title);
        front.set_created(&timestamp);
        front.set_updated(&timestamp);

        Ok(Self {
            path,
            front,
            body: format!("# {title}\n\n"),
            had_front_matter: true,
        })
    }

    /// The absolute path of the note.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The Markdown body, without front matter.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    /// Replace the body.
    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    /// The front matter block.
    #[must_use]
    pub fn front_matter(&self) -> &FrontMatter {
        &self.front
    }

    /// Mutable front matter, for metadata edits.
    pub fn front_matter_mut(&mut self) -> &mut FrontMatter {
        &mut self.front
    }

    /// The display title: front matter, then the first H1, then the file stem.
    #[must_use]
    pub fn title(&self) -> String {
        if let Some(title) = self.front.title() {
            return title;
        }

        for line in self.body.lines() {
            if let Some(heading) = line.strip_prefix("# ") {
                let heading = heading.trim();
                if !heading.is_empty() {
                    return heading.to_owned();
                }
            }
        }

        self.path
            .file_stem()
            .map(|stem| stem.to_string_lossy().replace(['-', '_'], " "))
            .filter(|stem| !stem.trim().is_empty())
            .unwrap_or_else(|| "Untitled".to_owned())
    }

    /// The tags from front matter.
    #[must_use]
    pub fn tags(&self) -> Vec<String> {
        self.front.tags()
    }

    /// The format version declared by the note.
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.front.format_version()
    }

    /// Give the note an identity and timestamps if it has none.
    ///
    /// Called before the first save of a hand-written note, so a note without
    /// front matter gains `format`, `id`, and `created` without losing its body.
    pub fn ensure_identity(&mut self) {
        if self.front.id().is_none() {
            self.front.set_id(&ulid::Ulid::generate().to_string());
        }
        if self.front.created().is_none() {
            self.front.set_created(&now());
        }
        self.front.set_format(1);
        self.front.set_updated(&now());
    }

    /// Refresh `updated` and ensure `format`.
    pub fn touch(&mut self) {
        self.front.set_format(1);
        self.front.set_updated(&now());
    }

    /// Render the note back to file text.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FrontMatterYaml`] if the front matter cannot be
    /// serialized.
    pub fn render(&self) -> Result<String> {
        if !self.had_front_matter && self.front.is_empty() {
            return Ok(self.body.clone());
        }

        let yaml = self.front.to_yaml(&self.path)?;
        let mut text = String::with_capacity(yaml.len() + self.body.len() + 8);
        text.push_str("---\n");
        text.push_str(&yaml);
        text.push_str("---\n");
        text.push_str(&self.body);
        Ok(text)
    }

    /// Save the note atomically.
    ///
    /// # Errors
    ///
    /// Returns the render error from [`Note::render`] and the write errors from
    /// [`write_atomic`].
    pub fn save(&self) -> Result<()> {
        let text = self.render()?;
        write_atomic(&self.path, text.as_bytes())
    }
}

fn now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// Split a note into its YAML front matter and body.
///
/// Front matter exists only when the first line is exactly `---` and a later
/// line is exactly `---` or `...`. Anything else is all body.
fn split_front_matter(text: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    let mut yaml_start = None;

    for line in text.split_inclusive('\n') {
        let line_without_ending = line.trim_end_matches(['\n', '\r']);

        match yaml_start {
            None => {
                if line_without_ending != "---" {
                    return None;
                }
                yaml_start = Some(offset + line.len());
            }
            Some(start) => {
                if line_without_ending == "---" || line_without_ending == "..." {
                    return Some((&text[start..offset], &text[offset + line.len()..]));
                }
            }
        }

        offset += line.len();
    }

    None
}
