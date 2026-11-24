// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! lean4losophy: a GTK4 notebook for Markdown notes and structured arguments.
//!
//! M1 is the notes shell: open or create a notebook, edit a note with autosave,
//! preview it, and keep settings with the notebook. Links, backlinks, and search
//! arrive in M2; Lean is optional and arrives in M4.

mod app;
mod dialogs;
mod workers;

use gtk::prelude::*;

/// Temporary application ID until the product name is fixed (ADR-0003).
pub(crate) const APP_ID: &str = "io.github.dysshanks.lean4losophy";

/// App-specific rules layered over the generated design tokens.
const APP_CSS: &str = r#"
textview.lean4losophy-editor,
textview.lean4losophy-editor text {
  background: @lean4losophy_bg;
  color: @lean4losophy_text;
}

.lean4losophy-editor {
  font-family: monospace;
  font-size: 15px;
}

.lean4losophy-preview {
  background: @lean4losophy_bg;
}

.lean4losophy-preview label {
  color: @lean4losophy_text;
}

.lean4losophy-muted {
  color: @lean4losophy_muted;
}

.lean4losophy-code {
  font-family: monospace;
  background: @lean4losophy_surface;
  padding: 10px 14px;
  border-radius: 8px;
}

.lean4losophy-quote {
  border-left: 3px solid @lean4losophy_border;
  padding-left: 14px;
}

.lean4losophy-table-header {
  font-weight: 700;
}

.lean4losophy-numeric {
  font-feature-settings: "tnum" 1, "lnum" 1;
}

.lean4losophy-table-cell {
  padding: 2px 8px;
}

.lean4losophy-card {
  background: @lean4losophy_surface;
  border-radius: 12px;
  padding: 12px 16px;
}

/* Workbench chrome: VSCode density, Obsidian content canvas. */
.lean4losophy-workbench {
  background-color: @lean4losophy_bg;
}

.lean4losophy-activity-bar {
  background-color: @lean4losophy_panel;
  border-right: 1px solid @lean4losophy_border;
  padding: 6px 5px;
}

.lean4losophy-activity-bar button {
  min-width: 32px;
  min-height: 32px;
  padding: 0;
  border-radius: 6px;
  background: transparent;
  color: @lean4losophy_muted;
}

.lean4losophy-activity-bar button:hover {
  background-color: alpha(@lean4losophy_text, 0.07);
  color: @lean4losophy_text;
}

.lean4losophy-activity-active {
  background-color: alpha(@lean4losophy_accent, 0.14);
  color: @lean4losophy_accent;
}

.lean4losophy-sidebar {
  background-color: @lean4losophy_panel;
  border-right: 1px solid @lean4losophy_border;
  padding: 8px 8px 6px;
}

.lean4losophy-sidebar list {
  background: transparent;
}

.lean4losophy-sidebar list row {
  border-radius: 6px;
  padding: 2px 6px;
}

.lean4losophy-sidebar list row:selected {
  background-color: alpha(@lean4losophy_accent, 0.12);
}

.lean4losophy-sidebar .heading {
  font-size: 11px;
  font-weight: 700;
  color: @lean4losophy_muted;
}

.lean4losophy-command-center {
  background-color: @lean4losophy_panel;
  border: 1px solid @lean4losophy_border;
  border-radius: 6px;
  padding: 2px 10px;
  min-width: 280px;
}

.lean4losophy-command-center:hover {
  border-color: @lean4losophy_accent;
}

.lean4losophy-command-title {
  font-size: 12px;
  font-weight: 600;
  color: @lean4losophy_text;
}

.lean4losophy-command-hint {
  font-size: 10px;
  color: @lean4losophy_muted;
}

.lean4losophy-statusbar {
  background-color: @lean4losophy_panel;
  border-top: 1px solid @lean4losophy_border;
  padding: 3px 10px;
  min-height: 24px;
}

.lean4losophy-statusbar label {
  font-size: 11px;
  color: @lean4losophy_muted;
}

.lean4losophy-backlinks {
  background-color: @lean4losophy_panel;
  border-top: 1px solid @lean4losophy_border;
}

.lean4losophy-palette {
  background-color: @lean4losophy_surface_raised;
  border: 1px solid @lean4losophy_border;
  border-radius: 10px;
}

.lean4losophy-palette-body {
  background-color: @lean4losophy_surface_raised;
  padding: 10px;
}

.lean4losophy-palette-search {
  font-size: 15px;
  padding: 8px 6px;
}

.lean4losophy-palette-list {
  background: transparent;
}

.lean4losophy-palette-list row {
  border-radius: 6px;
  padding: 6px 8px;
}

.lean4losophy-palette-list row:selected {
  background-color: alpha(@lean4losophy_accent, 0.14);
}
"#;

fn main() -> gtk::glib::ExitCode {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("lean4losophy=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .build();
    app.connect_startup(|_| install_css());
    app.connect_activate(app::activate);
    app.run()
}

fn install_css() {
    let display = gtk::gdk::Display::default();
    let Some(display) = display else {
        tracing::warn!("no display; rendering without the lean4losophy theme");
        return;
    };

    let provider = gtk::CssProvider::new();
    reload_theme(&provider);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let provider = provider.clone();
    libadwaita::StyleManager::default().connect_dark_notify(move |_| reload_theme(&provider));
}

fn reload_theme(provider: &gtk::CssProvider) {
    let base = if libadwaita::StyleManager::default().is_dark() {
        include_str!("../../../design/theme-dark.css")
    } else {
        include_str!("../../../design/theme.css")
    };
    provider.load_from_data(&format!("{base}\n{APP_CSS}"));
}
