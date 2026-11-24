// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! Aporia: a GTK4 notebook for Markdown notes and structured arguments.
//!
//! M1 is the notes shell: open or create a notebook, edit a note with autosave,
//! preview it, and keep settings with the notebook. Links, backlinks, and search
//! arrive in M2; Lean is optional and arrives in M4.

mod app;
mod dialogs;
mod workers;

use gtk::prelude::*;

/// Temporary application ID until the product name is fixed (ADR-0003).
pub(crate) const APP_ID: &str = "io.github.aporia.Notebook";

/// App-specific rules layered over the generated design tokens.
const APP_CSS: &str = r#"
textview.aporia-editor,
textview.aporia-editor text {
  background: @aporia_bg;
  color: @aporia_text;
}

.aporia-editor {
  font-family: monospace;
  font-size: 15px;
}

.aporia-preview {
  background: @aporia_bg;
}

.aporia-preview label {
  color: @aporia_text;
}

.aporia-muted {
  color: @aporia_muted;
}

.aporia-code {
  font-family: monospace;
  background: @aporia_surface;
  padding: 10px 14px;
  border-radius: 8px;
}

.aporia-quote {
  border-left: 3px solid @aporia_border;
  padding-left: 14px;
}

.aporia-table-header {
  font-weight: 700;
}

.aporia-table-cell {
  padding: 2px 8px;
}

.aporia-card {
  background: @aporia_surface;
  border-radius: 12px;
  padding: 12px 16px;
}
"#;

fn main() -> gtk::glib::ExitCode {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("aporia=info"));
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
        tracing::warn!("no display; rendering without the Aporia theme");
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
