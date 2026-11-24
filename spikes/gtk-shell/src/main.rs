//! Plan B spike: a GTK4 notes shell in Rust.
//!
//! Spikes are exempt from the production lint gate, but this code still avoids
//! `unwrap`, `expect`, `panic!`, and `println!` so it can move into
//! `crates/aporia` with few changes. The one deliberate shortcut is that the
//! shell is a single small window: no libadwaita, no Markdown preview, no
//! notebook model.

mod window;

use gtk::prelude::*;

/// Placeholder application ID; the product name is deferred.
const APP_ID: &str = "io.github.aporia.Notebook";

fn main() -> gtk::glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(window::build);
    app.run()
}
