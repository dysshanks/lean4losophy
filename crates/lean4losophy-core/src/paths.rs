// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Path handling for notebook-relative paths (`APORIA-3004`, `APORIA-2009`).
//!
//! Notes are addressed by a relative [`Path`] under the notebook root. Every
//! join rejects absolute paths and `..`, and existing targets are checked
//! against the canonical root so a symlink cannot escape the notebook.

use std::path::{Component, Path, PathBuf};

use crate::error::{Error, Result};

/// Join a notebook-relative path onto the root, rejecting anything that could
/// escape it.
///
/// # Errors
///
/// Returns [`Error::EscapesRoot`] for an absolute path or a path containing
/// `..` or a root component.
pub fn join_under(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute() {
        return Err(Error::EscapesRoot {
            path: relative.to_path_buf(),
        });
    }

    let mut joined = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => joined.push(part),
            Component::CurDir => {}
            _ => {
                return Err(Error::EscapesRoot {
                    path: relative.to_path_buf(),
                });
            }
        }
    }
    Ok(joined)
}

/// Verify that an existing path stays inside the canonical notebook root.
///
/// Used after a link or path is resolved, before the app reads a file.
///
/// # Errors
///
/// Returns [`Error::EscapesRoot`] when the path resolves outside the root, and
/// [`Error::Read`] when either path cannot be canonicalized.
pub fn ensure_under(root: &Path, candidate: &Path) -> Result<()> {
    let canonical_root = root.canonicalize().map_err(|source| Error::Read {
        path: root.to_path_buf(),
        source,
    })?;
    let canonical_candidate = candidate.canonicalize().map_err(|source| Error::Read {
        path: candidate.to_path_buf(),
        source,
    })?;

    if canonical_candidate.starts_with(&canonical_root) {
        Ok(())
    } else {
        Err(Error::EscapesRoot {
            path: candidate.to_path_buf(),
        })
    }
}

/// Strip the notebook root from a path, producing the stored relative form.
///
/// # Errors
///
/// Returns [`Error::EscapesRoot`] when `path` is not under `root`.
pub fn relative_to(root: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|_| Error::EscapesRoot {
            path: path.to_path_buf(),
        })
}

/// Reject a relative path that is not a `.md` note directly usable by the app.
///
/// # Errors
///
/// Returns [`Error::InvalidNotePath`] when the path is empty, has no file name,
/// or does not end in `.md`.
pub fn validate_note_path(relative: &Path) -> Result<()> {
    let valid = relative
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        && relative.file_name().is_some()
        && relative
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));

    if valid {
        Ok(())
    } else {
        Err(Error::InvalidNotePath {
            path: relative.to_path_buf(),
        })
    }
}
