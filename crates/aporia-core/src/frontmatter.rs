// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! Note front matter (`FORMAT.md` section 3).
//!
//! The raw YAML mapping is kept, so unknown keys from other tools survive a
//! read/write cycle. Known keys are read and written through typed accessors
//! that never panic on a wrong-shaped value.

use std::path::Path;

use serde_yaml_ng::{Mapping, Value};

use crate::error::{Error, Result};

const KEY_FORMAT: &str = "format";
const KEY_ID: &str = "id";
const KEY_TITLE: &str = "title";
const KEY_ALIASES: &str = "aliases";
const KEY_TAGS: &str = "tags";
const KEY_CREATED: &str = "created";
const KEY_UPDATED: &str = "updated";
const KEY_LEAN_ENTRY: &str = "lean_entry";

/// The YAML mapping at the top of a note.
#[derive(Debug, Clone, Default)]
pub struct FrontMatter {
    map: Mapping,
}

impl FrontMatter {
    /// An empty front matter block.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Parse a YAML mapping.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FrontMatterYaml`] for invalid YAML and
    /// [`Error::FrontMatterShape`] when the document is not a mapping.
    pub fn parse(yaml: &str, path: &Path) -> Result<Self> {
        let value: Value =
            serde_yaml_ng::from_str(yaml).map_err(|source| Error::FrontMatterYaml {
                path: path.to_path_buf(),
                source,
            })?;

        match value {
            Value::Mapping(map) => Ok(Self { map }),
            Value::Null => Ok(Self::empty()),
            _ => Err(Error::FrontMatterShape {
                path: path.to_path_buf(),
            }),
        }
    }

    /// Serialize back to YAML, ending with a newline.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FrontMatterYaml`] if the mapping cannot be serialized.
    pub fn to_yaml(&self, path: &Path) -> Result<String> {
        serde_yaml_ng::to_string(&self.map).map_err(|source| Error::FrontMatterYaml {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Whether the block has no keys.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The declared format version, defaulting to 1 when absent.
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.map
            .get(Value::String(KEY_FORMAT.to_owned()))
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .unwrap_or(1)
    }

    /// The stable ULID, if present and well formed.
    #[must_use]
    pub fn id(&self) -> Option<String> {
        self.string(KEY_ID)
    }

    /// The explicit title, if present and non-empty.
    #[must_use]
    pub fn title(&self) -> Option<String> {
        self.string(KEY_TITLE)
    }

    /// The aliases, in stored order.
    #[must_use]
    pub fn aliases(&self) -> Vec<String> {
        self.string_list(KEY_ALIASES)
    }

    /// The tags, without a leading `#`.
    #[must_use]
    pub fn tags(&self) -> Vec<String> {
        self.string_list(KEY_TAGS)
    }

    /// The `created` timestamp, as stored.
    #[must_use]
    pub fn created(&self) -> Option<String> {
        self.string(KEY_CREATED)
    }

    /// The `updated` timestamp, as stored.
    #[must_use]
    pub fn updated(&self) -> Option<String> {
        self.string(KEY_UPDATED)
    }

    /// The generated Lean module for this note, if any.
    #[must_use]
    pub fn lean_entry(&self) -> Option<String> {
        self.string(KEY_LEAN_ENTRY)
    }

    /// Set `format`.
    pub fn set_format(&mut self, version: u32) {
        self.set(
            KEY_FORMAT,
            Value::Number(serde_yaml_ng::Number::from(u64::from(version))),
        );
    }

    /// Set `id`.
    pub fn set_id(&mut self, id: &str) {
        self.set(KEY_ID, Value::String(id.to_owned()));
    }

    /// Set `title`, or remove it when empty.
    pub fn set_title(&mut self, title: &str) {
        let title = title.trim();
        if title.is_empty() {
            self.map.remove(Value::String(KEY_TITLE.to_owned()));
        } else {
            self.set(KEY_TITLE, Value::String(title.to_owned()));
        }
    }

    /// Set `aliases`.
    pub fn set_aliases(&mut self, aliases: &[String]) {
        self.set_list(KEY_ALIASES, aliases);
    }

    /// Set `tags`.
    pub fn set_tags(&mut self, tags: &[String]) {
        self.set_list(KEY_TAGS, tags);
    }

    /// Set `created`.
    pub fn set_created(&mut self, timestamp: &str) {
        self.set(KEY_CREATED, Value::String(timestamp.to_owned()));
    }

    /// Set `updated`.
    pub fn set_updated(&mut self, timestamp: &str) {
        self.set(KEY_UPDATED, Value::String(timestamp.to_owned()));
    }

    /// Set `lean_entry`.
    pub fn set_lean_entry(&mut self, entry: &str) {
        self.set(KEY_LEAN_ENTRY, Value::String(entry.to_owned()));
    }

    fn set(&mut self, key: &str, value: Value) {
        self.map.insert(Value::String(key.to_owned()), value);
    }

    fn set_list(&mut self, key: &str, values: &[String]) {
        let sequence = values
            .iter()
            .map(|value| Value::String(value.clone()))
            .collect::<Vec<_>>();
        self.set(key, Value::Sequence(sequence));
    }

    fn string(&self, key: &str) -> Option<String> {
        let value = self.map.get(Value::String(key.to_owned()))?;
        match value {
            Value::String(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            }
            _ => None,
        }
    }

    fn string_list(&self, key: &str) -> Vec<String> {
        match self.map.get(Value::String(key.to_owned())) {
            Some(Value::Sequence(items)) => items
                .iter()
                .filter_map(|item| item.as_str())
                .map(|item| item.trim().trim_start_matches('#').to_owned())
                .filter(|item| !item.is_empty())
                .collect(),
            Some(Value::String(single)) => {
                let trimmed = single.trim().trim_start_matches('#').to_owned();
                if trimmed.is_empty() {
                    Vec::new()
                } else {
                    vec![trimmed]
                }
            }
            _ => Vec::new(),
        }
    }
}
