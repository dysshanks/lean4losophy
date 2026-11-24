// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Crash-safe file writes (`FORMAT.md` section 13).
//!
//! A write goes to a hidden temporary file in the same directory, is flushed,
//! and is renamed over the target. A crash leaves either the old complete file
//! or the new complete file, never a half-written note.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

/// Write `bytes` to `path` atomically.
///
/// Creates the parent directory when missing, preserves the target's
/// permissions when it already exists, and removes the temporary file if
/// anything fails. The bytes are the caller's responsibility: UTF-8 and LF are
/// enforced before this call, not here.
///
/// # Errors
///
/// Returns [`Error::Write`] for any I/O failure, and [`Error::InvalidNotePath`]
/// when `path` has no file name.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    fs::create_dir_all(&parent).map_err(|source| Error::Write {
        path: parent.clone(),
        source,
    })?;

    let name = path.file_name().ok_or_else(|| Error::InvalidNotePath {
        path: path.to_path_buf(),
    })?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    let mut tmp_name = OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(format!(".tmp-{}-{stamp}", std::process::id()));
    let tmp = parent.join(tmp_name);

    let outcome = write_then_rename(path, &parent, &tmp, bytes);
    if outcome.is_err() {
        // Best effort: the target was never touched.
        let _ = fs::remove_file(&tmp);
    }
    outcome
}

fn write_then_rename(path: &Path, parent: &Path, tmp: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = File::create(tmp).map_err(|source| Error::Write {
        path: tmp.to_path_buf(),
        source,
    })?;
    file.write_all(bytes).map_err(|source| Error::Write {
        path: tmp.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| Error::Write {
        path: tmp.to_path_buf(),
        source,
    })?;

    if let Ok(metadata) = fs::metadata(path) {
        fs::set_permissions(tmp, metadata.permissions()).map_err(|source| Error::Write {
            path: tmp.to_path_buf(),
            source,
        })?;
    }

    fs::rename(tmp, path).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;

    // Persist the directory entry where the platform allows it. A failure here
    // does not invalidate the write that already succeeded.
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }

    Ok(())
}
