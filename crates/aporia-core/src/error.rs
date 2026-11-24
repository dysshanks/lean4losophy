// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! Typed errors for every fallible operation in the core.
//!
//! The core never panics on user input: a malformed note, a missing notebook,
//! or an unreadable settings file is a value the app can report (`APORIA-1003`).

use std::path::PathBuf;

/// A core operation failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A file could not be read.
    #[error("could not read {path}: {source}")]
    Read {
        /// The file that failed.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A file could not be written atomically.
    #[error("could not write {path}: {source}")]
    Write {
        /// The target file.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A file was not valid UTF-8. The core never replaces bytes.
    #[error("{path} is not valid UTF-8; notes must be UTF-8")]
    InvalidUtf8 {
        /// The offending file.
        path: PathBuf,
    },

    /// Front matter did not parse as YAML.
    #[error("front matter in {path} is not valid YAML: {source}")]
    FrontMatterYaml {
        /// The note.
        path: PathBuf,
        /// The YAML parse error.
        #[source]
        source: serde_yaml_ng::Error,
    },

    /// Front matter parsed but was not a mapping.
    #[error("front matter in {path} is not a YAML mapping")]
    FrontMatterShape {
        /// The note.
        path: PathBuf,
    },

    /// A settings file did not parse as TOML.
    #[error("settings in {path} are not valid TOML: {source}")]
    SettingsRead {
        /// The settings file.
        path: PathBuf,
        /// The TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// A settings file could not be serialized.
    #[error("settings for {path} could not be serialized: {source}")]
    SettingsSerialize {
        /// The settings file.
        path: PathBuf,
        /// The TOML serialization error.
        #[source]
        source: toml::ser::Error,
    },

    /// A path resolved outside the notebook root.
    #[error("{path} is outside the notebook root")]
    EscapesRoot {
        /// The rejected path.
        path: PathBuf,
    },

    /// A path was not usable as a note path.
    #[error("{path} is not a usable note path")]
    InvalidNotePath {
        /// The rejected path.
        path: PathBuf,
    },

    /// The directory is not a notebook because `notes/` is missing.
    #[error("{path} is not a notebook: notes/ is missing")]
    NotANotebook {
        /// The directory that was opened.
        path: PathBuf,
    },

    /// The home directory could not be determined for the config path.
    #[error("could not determine a config directory: set XDG_CONFIG_HOME or HOME")]
    NoConfigDir,
}

/// The core result type.
pub type Result<T> = std::result::Result<T, Error>;
