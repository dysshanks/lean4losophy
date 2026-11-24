// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Settings split between the XDG config directory and the notebook
//! (`FORMAT.md` section 11, ADR-0004).
//!
//! Unknown keys are preserved: each struct flattens leftover values into an
//! `extra` map, so a newer version can add a setting without older versions
//! dropping it on write.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;
use crate::error::{Error, Result};

/// Guided or Expert presentation, stored per notebook with a global default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Hints, explanations, and a simplified Lean view.
    #[default]
    Guided,
    /// No hints or tutorial overlays.
    Expert,
}

impl Mode {
    /// The stored spelling, matching `FORMAT.md`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Guided => "guided",
            Self::Expert => "expert",
        }
    }

    /// A human-readable label for the interface.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Guided => "Guided",
            Self::Expert => "Expert",
        }
    }

    /// Toggle between the two modes.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Guided => Self::Expert,
            Self::Expert => Self::Guided,
        }
    }
}

/// How the preview shares the window with the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreviewLayout {
    /// Editor and preview side by side.
    #[default]
    SideBySide,
    /// One pane at a time, toggled.
    Toggle,
}

impl PreviewLayout {
    /// The stored spelling, matching `FORMAT.md`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SideBySide => "side-by-side",
            Self::Toggle => "toggle",
        }
    }

    /// Toggle between the layouts.
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::SideBySide => Self::Toggle,
            Self::Toggle => Self::SideBySide,
        }
    }
}

/// Machine-local preferences in `$XDG_CONFIG_HOME/lean4losophy/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// Format version of the config file.
    pub version: u32,
    /// Mode used when a notebook does not set one.
    pub mode_default: Mode,
    /// Whether the launch update check is enabled. Off by default (ADR-0003).
    pub update_check: bool,
    /// Editor font scale multiplier.
    pub editor_font_scale: f64,
    /// Last window width in logical pixels.
    pub window_width: i32,
    /// Last window height in logical pixels.
    pub window_height: i32,
    /// Notebooks opened recently, most recent first.
    pub recent_notebooks: Vec<PathBuf>,
    /// Unknown keys, preserved across a read/write cycle.
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: 1,
            mode_default: Mode::Guided,
            update_check: false,
            editor_font_scale: 1.0,
            window_width: 1100,
            window_height: 720,
            recent_notebooks: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

/// Preferences that travel with a notebook in `.aporia/settings.toml`.
///
/// Every field is optional so an unset key inherits from the global config or
/// the built-in default.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NotebookSettings {
    /// Format version of the file.
    pub version: u32,
    /// Per-notebook mode; `None` inherits `mode_default`.
    pub mode: Option<Mode>,
    /// Per-notebook preview layout; `None` means side by side.
    pub preview_layout: Option<PreviewLayout>,
    /// Autosave debounce in milliseconds; `None` means 400.
    pub autosave_ms: Option<u64>,
    /// Default CSL citation style for papers.
    pub citation_style: Option<String>,
    /// Default paper template.
    pub default_template: Option<String>,
    /// Default paper page size.
    pub paper_page_size: Option<String>,
    /// Unknown keys, preserved across a read/write cycle.
    #[serde(flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

/// The settings actually in effect for the open notebook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveSettings {
    /// Resolved mode.
    pub mode: Mode,
    /// Resolved preview layout.
    pub preview_layout: PreviewLayout,
    /// Resolved autosave debounce in milliseconds.
    pub autosave_ms: u64,
}

impl EffectiveSettings {
    /// Resolve notebook over global over built-in default.
    #[must_use]
    pub fn resolve(global: &GlobalConfig, notebook: &NotebookSettings) -> Self {
        Self {
            mode: notebook.mode.unwrap_or(global.mode_default),
            preview_layout: notebook.preview_layout.unwrap_or_default(),
            autosave_ms: notebook.autosave_ms.unwrap_or(400).max(50),
        }
    }
}

/// The global config directory, `$XDG_CONFIG_HOME/lean4losophy` or `~/.config/lean4losophy`.
///
/// # Errors
///
/// Returns [`Error::NoConfigDir`] when neither variable is set.
pub fn config_dir() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(dir).join("lean4losophy"));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(Error::NoConfigDir)?;
    Ok(home.join(".config").join("lean4losophy"))
}

/// The global config file path.
///
/// # Errors
///
/// Returns [`Error::NoConfigDir`] when the config directory cannot be found.
pub fn global_config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// The notebook settings file path for a notebook root.
#[must_use]
pub fn notebook_settings_path(root: &Path) -> PathBuf {
    root.join(".aporia").join("settings.toml")
}

/// Read a settings file, returning defaults when it is missing.
///
/// # Errors
///
/// Returns [`Error::Read`] for a permission or I/O failure other than
/// not-found, and [`Error::SettingsRead`] for malformed TOML.
pub fn read_toml<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de> + Default,
{
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|source| Error::SettingsRead {
            path: path.to_path_buf(),
            source,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(source) => Err(Error::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Write a settings file atomically.
///
/// # Errors
///
/// Returns [`Error::SettingsSerialize`] when the value cannot be serialized and
/// [`Error::Write`] when the file cannot be written.
pub fn write_toml<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    let text = toml::to_string_pretty(value).map_err(|source| Error::SettingsSerialize {
        path: path.to_path_buf(),
        source,
    })?;
    write_atomic(path, text.as_bytes())
}

/// Load the global config, or return defaults when it does not exist.
///
/// # Errors
///
/// Returns [`Error::NoConfigDir`] when the config path is unknown, plus the
/// errors documented on [`read_toml`].
pub fn load_global() -> Result<GlobalConfig> {
    read_toml(&global_config_path()?)
}

/// Save the global config atomically.
///
/// # Errors
///
/// Returns [`Error::NoConfigDir`] when the config path is unknown, plus the
/// errors documented on [`write_toml`].
pub fn save_global(config: &GlobalConfig) -> Result<()> {
    write_toml(&global_config_path()?, config)
}

/// Load the settings for a notebook root.
///
/// # Errors
///
/// Returns the errors documented on [`read_toml`].
pub fn load_notebook(root: &Path) -> Result<NotebookSettings> {
    read_toml(&notebook_settings_path(root))
}

/// Save the settings for a notebook root.
///
/// # Errors
///
/// Returns the errors documented on [`write_toml`].
pub fn save_notebook(root: &Path, settings: &NotebookSettings) -> Result<()> {
    write_toml(&notebook_settings_path(root), settings)
}
