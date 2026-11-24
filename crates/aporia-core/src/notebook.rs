// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! A notebook: a user-owned folder of Markdown notes (`FORMAT.md` section 1).
//!
//! The notebook is the only writer of note files. Every path that comes from
//! the interface or from disk is resolved under the root and checked, so a
//! crafted path cannot escape the folder (`APORIA-3004`).

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::note::Note;
use crate::paths::{join_under, relative_to, validate_note_path};
use crate::settings::{self, NotebookSettings};

/// An open notebook folder.
#[derive(Debug, Clone)]
pub struct Notebook {
    root: PathBuf,
    settings: NotebookSettings,
}

impl Notebook {
    /// Open an existing notebook. The folder must contain `notes/`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NotANotebook`] when `notes/` is missing, and
    /// [`Error::Read`] when the root cannot be canonicalized or the settings
    /// cannot be read.
    pub fn open(root: &Path) -> Result<Self> {
        let canonical = root.canonicalize().map_err(|source| Error::Read {
            path: root.to_path_buf(),
            source,
        })?;

        let notes_dir = canonical.join("notes");
        if !notes_dir.is_dir() {
            return Err(Error::NotANotebook { path: canonical });
        }

        let settings = settings::load_notebook(&canonical)?;
        Ok(Self {
            root: canonical,
            settings,
        })
    }

    /// Create a notebook at `root`, including `notes/` and a settings file.
    ///
    /// Existing content is left alone; opening an existing notebook is the same
    /// call.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Write`] when a directory or the settings file cannot be
    /// created.
    pub fn create(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root.join("notes")).map_err(|source| Error::Write {
            path: root.join("notes"),
            source,
        })?;

        let canonical = root.canonicalize().map_err(|source| Error::Read {
            path: root.to_path_buf(),
            source,
        })?;

        let notebook = Self {
            root: canonical.clone(),
            settings: NotebookSettings {
                version: 1,
                ..NotebookSettings::default()
            },
        };

        if !settings::notebook_settings_path(&canonical).exists() {
            settings::save_notebook(&canonical, &notebook.settings)?;
        }

        Ok(notebook)
    }

    /// The absolute notebook root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `notes/` directory.
    #[must_use]
    pub fn notes_dir(&self) -> PathBuf {
        self.root.join("notes")
    }

    /// The notebook settings as loaded.
    #[must_use]
    pub fn settings(&self) -> &NotebookSettings {
        &self.settings
    }

    /// Replace and persist the notebook settings.
    ///
    /// # Errors
    ///
    /// Returns the write errors documented on [`settings::save_notebook`].
    pub fn update_settings(&mut self, settings: NotebookSettings) -> Result<()> {
        settings::save_notebook(&self.root, &settings)?;
        self.settings = settings;
        Ok(())
    }

    /// All note paths, relative to the notebook root, sorted.
    ///
    /// Skips dot-directories, non-`.md` files, and symlinks. Symlinks are
    /// skipped because following one could leave the notebook; `FORMAT.md`
    /// requires an escaping link to be ignored rather than resolved.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Read`] when `notes/` or a subdirectory cannot be read.
    pub fn list_notes(&self) -> Result<Vec<PathBuf>> {
        let mut notes = Vec::new();
        collect_notes(&self.notes_dir(), &self.root, &mut notes)?;
        notes.sort();
        Ok(notes)
    }

    /// Resolve a notebook-relative note path to an absolute path.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidNotePath`] for a non-`.md` path and
    /// [`Error::EscapesRoot`] for an absolute or parent-traversing path.
    pub fn note_path(&self, relative: &Path) -> Result<PathBuf> {
        validate_note_path(relative)?;
        join_under(&self.root, relative)
    }

    /// Read a note.
    ///
    /// # Errors
    ///
    /// Returns the errors from [`Notebook::note_path`] and [`Note::load`].
    pub fn load_note(&self, relative: &Path) -> Result<Note> {
        Note::load(&self.note_path(relative)?)
    }

    /// Save a note, making sure its path stays inside this notebook.
    ///
    /// # Errors
    ///
    /// Returns [`Error::EscapesRoot`] when the note path is outside the root,
    /// plus the errors from [`Note::save`].
    pub fn save_note(&self, note: &Note) -> Result<()> {
        relative_to(&self.root, note.path())?;
        note.save()
    }

    /// Create a new note with a unique file name and persist it.
    ///
    /// # Errors
    ///
    /// Returns the errors from [`Note::new`] and [`Note::save`], plus
    /// [`Error::Read`] when the notes directory cannot be listed.
    pub fn create_note(&self, title: &str) -> Result<Note> {
        let relative = self.unique_note_path(title)?;
        let path = self.note_path(&relative)?;
        let note = Note::new(path, title)?;
        note.save()?;
        Ok(note)
    }

    /// Rename or move a note inside the notebook.
    ///
    /// Link rewriting is M2; this only moves the file, creating the target
    /// folder when needed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidNotePath`] for a bad target, [`Error::Read`] when
    /// the target exists, and [`Error::Write`] when the move fails.
    pub fn rename_note(&self, from: &Path, to: &Path) -> Result<PathBuf> {
        let from_path = self.note_path(from)?;
        let to_path = self.note_path(to)?;

        if to_path.exists() {
            return Err(Error::Write {
                path: to_path,
                source: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "a note with this name already exists",
                ),
            });
        }

        if let Some(parent) = to_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        std::fs::rename(&from_path, &to_path).map_err(|source| Error::Write {
            path: to_path.clone(),
            source,
        })?;

        Ok(to_path)
    }

    /// Delete a note file. The file moves to the trash only if the caller does;
    /// this removes it from the notebook.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Write`] when the file cannot be removed.
    pub fn delete_note(&self, relative: &Path) -> Result<()> {
        let path = self.note_path(relative)?;
        std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })
    }

    /// Ensure the sample notebook content exists: a welcome note and one example
    /// argument. Existing notes are never overwritten.
    ///
    /// # Errors
    ///
    /// Returns the errors from [`Notebook::create_note`] and [`Note::save`].
    pub fn ensure_sample_notes(&self) -> Result<()> {
        if !self.list_notes()?.is_empty() {
            return Ok(());
        }

        let mut welcome = self.create_note("Welcome to Aporia")?;
        welcome.set_body(
            "# Welcome to Aporia\n\n\
             This notebook is a folder of plain Markdown files. Edit them here or \
             in any text editor; nothing is locked away.\n\n\
             ## Try this\n\n\
             - Write a note with **New note** in the header.\n\
             - Link to another note later with `[[Its title]]` (links arrive in M2).\n\
             - Switch between Guided and Expert in the menu.\n\n\
             The preview on the right renders tables and footnotes too.\n"
                .to_owned(),
        );
        welcome.save()?;

        let mut example = self.create_note("The mortality argument")?;
        example.set_body(
            "# The mortality argument\n\n\
             An argument written as a plain list, so it stays readable anywhere.\n\n\
             - Argument: The mortality argument {#arg-mortality}\n\
             \x20 1. All humans are mortal.\n\
             \x20 2. Socrates is a human.\n\
             \x20 3. ∴ Socrates is mortal.\n\n\
             Arguments get a map and optional Lean checking in M3 and M4.\n"
                .to_owned(),
        );
        example.save()?;

        Ok(())
    }

    fn unique_note_path(&self, title: &str) -> Result<PathBuf> {
        let slug = slugify(title);
        let mut candidate = PathBuf::from("notes").join(format!("{slug}.md"));
        let mut counter = 2_u32;

        while self.note_path(&candidate)?.exists() {
            candidate = PathBuf::from("notes").join(format!("{slug}-{counter}.md"));
            counter = counter.saturating_add(1);
        }

        Ok(candidate)
    }
}

fn collect_notes(dir: &Path, root: &Path, notes: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| Error::Read {
            path: entry.path(),
            source,
        })?;

        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        if file_type.is_dir() {
            let hidden = path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'));
            if !hidden {
                collect_notes(&path, root, notes)?;
            }
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            notes.push(relative_to(root, &path)?);
        }
    }

    Ok(())
}

/// Turn a note title into a file stem.
///
/// Lowercases, keeps letters and digits, replaces runs of other characters with
/// one hyphen, and falls back to `note` when nothing usable remains.
#[must_use]
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut pending_separator = false;

    for character in title.trim().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            pending_separator = false;
            for lowered in character.to_lowercase() {
                slug.push(lowered);
            }
        } else {
            pending_separator = true;
        }
    }

    if slug.is_empty() {
        String::from("note")
    } else {
        slug
    }
}
