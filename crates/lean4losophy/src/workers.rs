// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Background workers for saving and preview rendering.
//!
//! The UI thread never parses Markdown or touches the filesystem (`APORIA-2001`).
//! Each worker owns exactly one thread with a job channel; when the app drops the
//! handle, the channel closes and the thread exits. Results come back over a
//! second channel that the window drains on a timer, so the thread needs no
//! access to GTK.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use lean4losophy_core::preview::{self, Block};

/// A request to write a note file.
pub struct SaveJob {
    /// Absolute note path.
    pub path: PathBuf,
    /// Rendered UTF-8 bytes.
    pub bytes: Vec<u8>,
    /// Increments with each save so stale results can be ignored.
    pub generation: u64,
}

/// The result of a save.
pub enum SaveOutcome {
    /// The file was written atomically.
    Saved {
        /// The path written.
        path: PathBuf,
        /// The save generation.
        generation: u64,
    },
    /// The write failed; the note is still dirty.
    Failed {
        /// The path that failed.
        path: PathBuf,
        /// The save generation.
        generation: u64,
        /// A message for the status bar.
        message: String,
    },
}

/// A request to render Markdown blocks.
pub struct RenderJob {
    /// The note body.
    pub markdown: String,
    /// Increments with each render; stale results are dropped.
    pub generation: u64,
}

/// The result of a render.
pub struct RenderOutcome {
    /// The render generation.
    pub generation: u64,
    /// Parsed blocks.
    pub blocks: Vec<Block>,
}

/// Handle to the save worker.
pub struct Saver {
    sender: Option<Sender<SaveJob>>,
}

impl Saver {
    /// Send a save job. Returns the job back when the worker is unavailable, so
    /// the caller can fall back to a direct write without losing the bytes.
    ///
    /// # Errors
    ///
    /// Returns `Err(job)` when the worker thread has stopped.
    pub fn send(&self, job: SaveJob) -> std::result::Result<(), SaveJob> {
        match self.sender.as_ref() {
            Some(sender) => sender.send(job).map_err(|error| error.0),
            None => Err(job),
        }
    }
}

/// Handle to the preview worker.
pub struct Renderer {
    sender: Option<Sender<RenderJob>>,
}

impl Renderer {
    /// Send a render job. Returns the job back when the worker is unavailable,
    /// so the caller can render on the UI thread instead.
    ///
    /// # Errors
    ///
    /// Returns `Err(job)` when the worker thread has stopped.
    pub fn send(&self, job: RenderJob) -> std::result::Result<(), RenderJob> {
        match self.sender.as_ref() {
            Some(sender) => sender.send(job).map_err(|error| error.0),
            None => Err(job),
        }
    }
}

/// Start the save worker.
pub fn spawn_saver() -> (Saver, Receiver<SaveOutcome>) {
    let (job_tx, job_rx) = mpsc::channel::<SaveJob>();
    let (out_tx, out_rx) = mpsc::channel::<SaveOutcome>();

    let spawned = thread::Builder::new()
        .name(String::from("lean4losophy-save"))
        .spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let outcome = match lean4losophy_core::atomic::write_atomic(&job.path, &job.bytes) {
                    Ok(()) => SaveOutcome::Saved {
                        path: job.path,
                        generation: job.generation,
                    },
                    Err(error) => SaveOutcome::Failed {
                        path: job.path,
                        generation: job.generation,
                        message: error.to_string(),
                    },
                };
                if out_tx.send(outcome).is_err() {
                    break;
                }
            }
        });

    let saver = match spawned {
        Ok(_) => Saver {
            sender: Some(job_tx),
        },
        Err(error) => {
            tracing::error!(%error, "save worker could not start; saves will run on the UI thread");
            Saver { sender: None }
        }
    };

    (saver, out_rx)
}

/// Start the preview worker.
pub fn spawn_renderer() -> (Renderer, Receiver<RenderOutcome>) {
    let (job_tx, job_rx) = mpsc::channel::<RenderJob>();
    let (out_tx, out_rx) = mpsc::channel::<RenderOutcome>();

    let spawned = thread::Builder::new()
        .name(String::from("lean4losophy-preview"))
        .spawn(move || {
            while let Ok(job) = job_rx.recv() {
                let outcome = RenderOutcome {
                    generation: job.generation,
                    blocks: preview::blocks(&job.markdown),
                };
                if out_tx.send(outcome).is_err() {
                    break;
                }
            }
        });

    let renderer = match spawned {
        Ok(_) => Renderer {
            sender: Some(job_tx),
        },
        Err(error) => {
            tracing::error!(%error, "preview worker could not start; rendering will run on the UI thread");
            Renderer { sender: None }
        }
    };

    (renderer, out_rx)
}
