// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The Aporia Authors

//! Window construction, state, and the actions that tie the shell together.
//!
//! The state lives on the UI thread. Anything slow (parsing Markdown, writing a
//! note) is handed to a worker in [`crate::workers`] and the results are drained
//! on a short timer, so typing never waits on I/O (`APORIA-2001`).

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use aporia_core::preview::Block;
use aporia_core::settings::{
    self, EffectiveSettings, GlobalConfig, NotebookSettings, PreviewLayout,
};
use aporia_core::{Note, Notebook};
use gtk::prelude::*;
use gtk::{gio, glib};
use libadwaita::prelude::*;

use crate::dialogs;
use crate::workers::{RenderJob, RenderOutcome, Renderer, SaveJob, SaveOutcome, Saver};

/// The note currently open in the editor.
struct CurrentNote {
    relative: PathBuf,
    note: Note,
    dirty: bool,
}

/// Widgets the state needs to update.
struct Ui {
    split: libadwaita::OverlaySplitView,
    stack: gtk::Stack,
    list: gtk::ListBox,
    editor: gtk::TextView,
    editor_scroller: gtk::ScrolledWindow,
    preview_scroller: gtk::ScrolledWindow,
    preview_box: gtk::Box,
    paned: gtk::Paned,
    home_buttons: gtk::Box,
    title: gtk::Label,
    status: gtk::Label,
    notebook_label: gtk::Label,
}

/// The whole application state, shared by the window and its actions.
struct AppState {
    window: libadwaita::ApplicationWindow,
    ui: Ui,
    notebook: RefCell<Option<Notebook>>,
    current: RefCell<Option<CurrentNote>>,
    global: RefCell<GlobalConfig>,
    effective: Cell<EffectiveSettings>,
    rows: RefCell<Vec<PathBuf>>,
    saver: Saver,
    renderer: Renderer,
    save_results: RefCell<Receiver<SaveOutcome>>,
    render_results: RefCell<Receiver<RenderOutcome>>,
    save_source: Cell<Option<glib::SourceId>>,
    preview_source: Cell<Option<glib::SourceId>>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    generation: Cell<u64>,
    preview_generation: Cell<u64>,
    preview_visible: Cell<bool>,
    loading: Cell<bool>,
    sidebar_visible: Cell<bool>,
}

/// Build the window and present it.
pub fn activate(app: &libadwaita::Application) {
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("Aporia")
        .default_width(1100)
        .default_height(720)
        .build();

    let title = gtk::Label::builder().label("Aporia").build();
    title.add_css_class("title-4");

    let status = gtk::Label::builder().label("Ready").xalign(0.0).build();
    status.add_css_class("aporia-muted");

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();
    list.add_css_class("navigation-sidebar");

    let editor = gtk::TextView::builder()
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Word)
        .left_margin(24)
        .right_margin(24)
        .top_margin(16)
        .bottom_margin(16)
        .build();
    editor.add_css_class("aporia-editor");
    editor.set_vexpand(true);
    editor.set_hexpand(true);

    let editor_scroller = gtk::ScrolledWindow::builder()
        .child(&editor)
        .vexpand(true)
        .hexpand(true)
        .build();

    let preview_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    preview_box.add_css_class("aporia-preview");
    preview_box.set_margin_top(18);
    preview_box.set_margin_bottom(18);
    preview_box.set_margin_start(28);
    preview_box.set_margin_end(28);

    let preview_scroller = gtk::ScrolledWindow::builder()
        .child(&preview_box)
        .vexpand(true)
        .hexpand(true)
        .build();

    let paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&editor_scroller)
        .end_child(&preview_scroller)
        .position(560)
        .resize_start_child(false)
        .shrink_start_child(false)
        .build();

    let home_buttons = gtk::Box::new(gtk::Orientation::Vertical, 8);
    home_buttons.set_halign(gtk::Align::Center);
    home_buttons.set_margin_top(12);

    let home = libadwaita::StatusPage::builder()
        .title("Aporia")
        .description(
            "A local-first notebook for arguments. Notes are plain Markdown in a folder you own. \
             Lean is optional and can be installed later.",
        )
        .icon_name("accessories-text-editor-symbolic")
        .child(&home_buttons)
        .build();

    let stack = gtk::Stack::new();
    stack.add_named(&home, Some("home"));
    stack.add_named(&paned, Some("editor"));

    let notes_label = gtk::Label::builder().label("Notes").xalign(0.0).build();
    notes_label.add_css_class("heading");
    notes_label.set_hexpand(true);

    let new_note_button = gtk::Button::builder()
        .icon_name("list-add-symbolic")
        .tooltip_text("New note")
        .build();
    new_note_button.set_action_name(Some("win.new-note"));

    let sidebar_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    sidebar_header.append(&notes_label);
    sidebar_header.append(&new_note_button);

    let list_scroller = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .build();

    let notebook_label = gtk::Label::builder()
        .label("No notebook open")
        .xalign(0.0)
        .wrap(true)
        .build();
    notebook_label.add_css_class("aporia-muted");

    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sidebar.set_width_request(260);
    sidebar.append(&sidebar_header);
    sidebar.append(&list_scroller);
    sidebar.append(&notebook_label);

    let header = libadwaita::HeaderBar::new();
    let sidebar_button = gtk::Button::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text("Show or hide the sidebar")
        .build();
    sidebar_button.set_action_name(Some("win.toggle-sidebar"));
    let new_button = gtk::Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text("New note")
        .build();
    new_button.set_action_name(Some("win.new-note"));

    let menu = build_menu();
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Menu")
        .menu_model(&menu)
        .build();

    header.pack_start(&sidebar_button);
    header.pack_end(&menu_button);
    header.pack_end(&new_button);
    header.set_title_widget(Some(&title));

    let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_box.set_margin_top(6);
    status_box.set_margin_bottom(6);
    status_box.set_margin_start(16);
    status_box.set_margin_end(16);
    status_box.append(&status);

    let split = libadwaita::OverlaySplitView::builder()
        .sidebar(&sidebar)
        .content(&stack)
        .min_sidebar_width(240.0)
        .show_sidebar(true)
        .build();

    let toolbar = libadwaita::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.add_bottom_bar(&status_box);
    toolbar.set_content(Some(&split));
    window.set_content(Some(&toolbar));

    let (saver, save_results) = crate::workers::spawn_saver();
    let (renderer, render_results) = crate::workers::spawn_renderer();

    let global = settings::load_global().unwrap_or_else(|error| {
        tracing::warn!(%error, "falling back to default settings");
        GlobalConfig::default()
    });

    let effective = EffectiveSettings::resolve(&global, &NotebookSettings::default());
    let state = Rc::new(AppState {
        window: window.clone(),
        ui: Ui {
            split,
            stack,
            list,
            editor,
            editor_scroller,
            preview_scroller,
            preview_box,
            paned,
            home_buttons,
            title,
            status,
            notebook_label,
        },
        notebook: RefCell::new(None),
        current: RefCell::new(None),
        global: RefCell::new(global),
        effective: Cell::new(effective),
        rows: RefCell::new(Vec::new()),
        saver,
        renderer,
        save_results: RefCell::new(save_results),
        render_results: RefCell::new(render_results),
        save_source: Cell::new(None),
        preview_source: Cell::new(None),
        monitor: RefCell::new(None),
        generation: Cell::new(0),
        preview_generation: Cell::new(0),
        preview_visible: Cell::new(true),
        loading: Cell::new(false),
        sidebar_visible: Cell::new(true),
    });

    connect_signals(&state);
    install_actions(&state, app);
    state.start_ticker();
    state.restore_session();
    window.present();
}

fn build_menu() -> gio::Menu {
    let menu = gio::Menu::new();

    let note_menu = gio::Menu::new();
    note_menu.append(Some("New note"), Some("win.new-note"));
    note_menu.append(Some("Rename note…"), Some("win.rename-note"));
    note_menu.append(Some("Delete note"), Some("win.delete-note"));

    let notebook_menu = gio::Menu::new();
    notebook_menu.append(Some("Open notebook…"), Some("win.open-notebook"));
    notebook_menu.append(Some("Create notebook…"), Some("win.new-notebook"));
    notebook_menu.append(Some("Home"), Some("win.home"));

    let view_menu = gio::Menu::new();
    view_menu.append(Some("Show or hide preview"), Some("win.toggle-preview"));
    view_menu.append(Some("Switch preview layout"), Some("win.toggle-layout"));
    view_menu.append(Some("Switch Guided/Expert"), Some("win.toggle-mode"));

    let app_menu = gio::Menu::new();
    app_menu.append(Some("About Aporia"), Some("win.about"));

    menu.append_section(None, &note_menu);
    menu.append_section(None, &notebook_menu);
    menu.append_section(None, &view_menu);
    menu.append_section(None, &app_menu);
    menu
}

fn connect_signals(state: &Rc<AppState>) {
    {
        let this = Rc::clone(state);
        state.ui.list.connect_row_activated(move |_, row| {
            let index = usize::try_from(row.index()).unwrap_or(usize::MAX);
            this.activate_row(index);
        });
    }
    {
        let this = Rc::clone(state);
        state.ui.editor.buffer().connect_changed(move |_| {
            this.on_buffer_changed();
        });
    }
    {
        let this = Rc::clone(state);
        state.window.connect_close_request(move |_| {
            this.save_now();
            this.persist_global();
            glib::Propagation::Proceed
        });
    }
}

impl AppState {
    fn start_ticker(self: &Rc<Self>) {
        let this = Rc::clone(self);
        glib::timeout_add_local(Duration::from_millis(100), move || {
            this.drain_workers();
            glib::ControlFlow::Continue
        });
    }

    fn restore_session(self: &Rc<Self>) {
        self.apply_layout();
        let recent = self
            .global
            .borrow()
            .recent_notebooks
            .first()
            .cloned()
            .filter(|path| path.join("notes").is_dir());

        match recent {
            Some(path) => self.open_notebook_path(path, false),
            None => self.show_home(),
        }
    }

    fn set_status(&self, text: &str) {
        self.ui.status.set_label(text);
    }

    fn editor_text(&self) -> String {
        let buffer = self.ui.editor.buffer();
        buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string()
    }

    fn current_relative(&self) -> Option<PathBuf> {
        self.current
            .borrow()
            .as_ref()
            .map(|current| current.relative.clone())
    }

    fn refresh_rows(self: &Rc<Self>) {
        let notes = match self.notebook.borrow().as_ref() {
            Some(notebook) => notebook.list_notes().unwrap_or_default(),
            None => Vec::new(),
        };

        while let Some(child) = self.ui.list.first_child() {
            self.ui.list.remove(&child);
        }

        let mut rows = Vec::with_capacity(notes.len());
        for relative in notes {
            let depth = relative.components().count().saturating_sub(2);
            let row = gtk::ListBoxRow::new();
            row.set_tooltip_text(Some(&relative.display().to_string()));

            let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            content.set_margin_start(8 + i32::try_from(depth).unwrap_or(0) * 12);
            content.set_margin_top(4);
            content.set_margin_bottom(4);

            let label = gtk::Label::builder()
                .label(display_title(&relative))
                .xalign(0.0)
                .build();
            content.append(&label);
            row.set_child(Some(&content));
            self.ui.list.append(&row);
            rows.push(relative);
        }

        *self.rows.borrow_mut() = rows;
    }

    fn activate_row(self: &Rc<Self>, index: usize) {
        let Some(relative) = self.rows.borrow().get(index).cloned() else {
            return;
        };
        self.open_note(relative);
    }

    fn open_note(self: &Rc<Self>, relative: PathBuf) {
        let Some(notebook) = self.notebook.borrow().clone() else {
            self.set_status("Open a notebook first.");
            return;
        };

        if self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|current| current.dirty)
        {
            self.save_current();
        }

        let note = match notebook.load_note(&relative) {
            Ok(note) => note,
            Err(error) => {
                self.set_status(&format!("Could not open note: {error}"));
                return;
            }
        };

        let read_only = note.format_version() > 1;
        self.ui.editor.set_editable(!read_only);

        self.loading.set(true);
        self.ui.editor.buffer().set_text(note.body());
        self.loading.set(false);

        self.ui.title.set_label(&note.title());
        self.ui.stack.set_visible_child_name("editor");
        *self.current.borrow_mut() = Some(CurrentNote {
            relative,
            note,
            dirty: false,
        });

        self.monitor_current();
        self.request_preview_immediately();
        self.set_status(if read_only {
            "Opened a newer-format note read-only"
        } else {
            "Opened note"
        });
    }

    fn clear_editor(&self) {
        self.loading.set(true);
        self.ui.editor.buffer().set_text("");
        self.loading.set(false);
        self.ui.editor.set_editable(true);
        *self.current.borrow_mut() = None;
        *self.monitor.borrow_mut() = None;
        self.ui.title.set_label("Aporia");
        while let Some(child) = self.ui.preview_box.first_child() {
            self.ui.preview_box.remove(&child);
        }
    }

    fn on_buffer_changed(self: &Rc<Self>) {
        if self.loading.get() {
            return;
        }
        {
            let mut current = self.current.borrow_mut();
            let Some(current) = current.as_mut() else {
                return;
            };
            current.dirty = true;
        }
        self.set_status("Unsaved changes");
        self.schedule_save();
        self.schedule_preview();
    }

    fn schedule_save(self: &Rc<Self>) {
        if let Some(source) = self.save_source.take() {
            source.remove();
        }
        let delay = self.effective.get().autosave_ms;
        let this = Rc::clone(self);
        let source = glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            this.save_current();
        });
        self.save_source.set(Some(source));
    }

    fn render_current(&self) -> Option<(PathBuf, Vec<u8>)> {
        let text = self.editor_text();
        let mut current = self.current.borrow_mut();
        let current = current.as_mut()?;
        current.note.set_body(text);
        current.note.touch();
        match current.note.render() {
            Ok(rendered) => Some((current.note.path().to_path_buf(), rendered.into_bytes())),
            Err(error) => {
                self.set_status(&format!("Could not serialize the note: {error}"));
                None
            }
        }
    }

    fn save_current(self: &Rc<Self>) {
        let Some((path, bytes)) = self.render_current() else {
            return;
        };
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        let job = SaveJob {
            path: path.clone(),
            bytes: bytes.clone(),
            generation,
        };

        if let Err(job) = self.saver.send(job) {
            match aporia_core::atomic::write_atomic(&job.path, &job.bytes) {
                Ok(()) => {
                    if let Some(current) = self.current.borrow_mut().as_mut() {
                        current.dirty = false;
                    }
                    self.set_status("Saved");
                }
                Err(error) => self.set_status(&format!("Save failed: {error}")),
            }
        }
    }

    /// Save inline before an action that moves or removes the file.
    fn save_now(&self) {
        if let Some((path, bytes)) = self.render_current() {
            match aporia_core::atomic::write_atomic(&path, &bytes) {
                Ok(()) => {
                    if let Some(current) = self.current.borrow_mut().as_mut() {
                        current.dirty = false;
                    }
                }
                Err(error) => self.set_status(&format!("Save failed: {error}")),
            }
        }
    }

    fn schedule_preview(self: &Rc<Self>) {
        if let Some(source) = self.preview_source.take() {
            source.remove();
        }
        let generation = self.preview_generation.get().wrapping_add(1);
        self.preview_generation.set(generation);
        let markdown = self.editor_text();
        let this = Rc::clone(self);
        let source = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            this.request_preview(markdown, generation);
        });
        self.preview_source.set(Some(source));
    }

    fn request_preview_immediately(self: &Rc<Self>) {
        if let Some(source) = self.preview_source.take() {
            source.remove();
        }
        let generation = self.preview_generation.get().wrapping_add(1);
        self.preview_generation.set(generation);
        let markdown = self.editor_text();
        self.request_preview(markdown, generation);
    }

    fn request_preview(&self, markdown: String, generation: u64) {
        let job = RenderJob {
            markdown,
            generation,
        };
        if let Err(job) = self.renderer.send(job) {
            let blocks = aporia_core::preview::blocks(&job.markdown);
            self.apply_preview(&blocks);
        }
    }

    fn apply_preview(&self, blocks: &[Block]) {
        while let Some(child) = self.ui.preview_box.first_child() {
            self.ui.preview_box.remove(&child);
        }
        for block in blocks {
            self.ui.preview_box.append(&block_widget(block));
        }
    }

    fn drain_workers(self: &Rc<Self>) {
        loop {
            let outcome = self.save_results.borrow().try_recv();
            match outcome {
                Ok(SaveOutcome::Saved { path, generation }) => {
                    if generation == self.generation.get() {
                        if let Some(current) = self.current.borrow_mut().as_mut() {
                            current.dirty = false;
                        }
                        let name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| String::from("note"));
                        self.set_status(&format!("Saved {name}"));
                    }
                }
                Ok(SaveOutcome::Failed {
                    path,
                    generation,
                    message,
                }) => {
                    if generation == self.generation.get() {
                        let name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| String::from("note"));
                        self.set_status(&format!("Save failed for {name}: {message}"));
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        loop {
            let outcome = self.render_results.borrow().try_recv();
            match outcome {
                Ok(RenderOutcome { generation, blocks }) => {
                    if generation == self.preview_generation.get() {
                        self.apply_preview(&blocks);
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn monitor_current(self: &Rc<Self>) {
        *self.monitor.borrow_mut() = None;
        let Some(relative) = self.current_relative() else {
            return;
        };
        let Some(notebook) = self.notebook.borrow().clone() else {
            return;
        };
        let Ok(path) = notebook.note_path(&relative) else {
            return;
        };

        let file = gio::File::for_path(&path);
        match file.monitor_file(gio::FileMonitorFlags::empty(), None::<&gio::Cancellable>) {
            Ok(monitor) => {
                let this = Rc::clone(self);
                monitor.connect_changed(move |_, _file, _other, _event| {
                    this.reload_if_changed();
                });
                *self.monitor.borrow_mut() = Some(monitor);
            }
            Err(error) => tracing::warn!(%error, "could not watch the note for external changes"),
        }
    }

    fn reload_if_changed(self: &Rc<Self>) {
        let Some(relative) = self.current_relative() else {
            return;
        };
        let Some(notebook) = self.notebook.borrow().clone() else {
            return;
        };
        let note = match notebook.load_note(&relative) {
            Ok(note) => note,
            Err(error) => {
                self.set_status(&format!(
                    "The note changed on disk and could not be read: {error}"
                ));
                return;
            }
        };

        let editor_text = self.editor_text();
        if note.body() == editor_text {
            // Our own atomic save; the body is unchanged.
            return;
        }

        let dirty = self
            .current
            .borrow()
            .as_ref()
            .is_some_and(|current| current.dirty);
        if dirty {
            self.set_status(
                "This note changed on disk. Your unsaved text is kept; saving will overwrite the file.",
            );
            return;
        }

        self.loading.set(true);
        self.ui.editor.buffer().set_text(note.body());
        self.loading.set(false);
        self.ui.title.set_label(&note.title());
        *self.current.borrow_mut() = Some(CurrentNote {
            relative,
            note,
            dirty: false,
        });
        self.request_preview_immediately();
        self.set_status("Reloaded the note from disk");
    }

    fn toggle_sidebar(self: &Rc<Self>) {
        let visible = !self.sidebar_visible.get();
        self.sidebar_visible.set(visible);
        self.ui.split.set_show_sidebar(visible);
    }

    fn toggle_preview(&self) {
        self.preview_visible.set(!self.preview_visible.get());
        self.apply_layout();
    }

    fn toggle_layout(self: &Rc<Self>) {
        let layout = self.effective.get().preview_layout.toggled();
        let mut effective = self.effective.get();
        effective.preview_layout = layout;
        self.effective.set(effective);
        self.persist_notebook(|settings| settings.preview_layout = Some(layout));
        self.apply_layout();
        self.set_status(match layout {
            PreviewLayout::SideBySide => "Side-by-side preview",
            PreviewLayout::Toggle => "Toggle preview",
        });
    }

    fn apply_layout(&self) {
        match self.effective.get().preview_layout {
            PreviewLayout::SideBySide => {
                self.ui.editor_scroller.set_visible(true);
                self.ui
                    .preview_scroller
                    .set_visible(self.preview_visible.get());
                self.ui.paned.set_position(560);
            }
            PreviewLayout::Toggle => {
                let show_preview = self.preview_visible.get();
                self.ui.editor_scroller.set_visible(!show_preview);
                self.ui.preview_scroller.set_visible(show_preview);
            }
        }
    }

    fn toggle_mode(self: &Rc<Self>) {
        let mode = self.effective.get().mode.toggled();
        let mut effective = self.effective.get();
        effective.mode = mode;
        self.effective.set(effective);
        self.persist_notebook(|settings| settings.mode = Some(mode));
        self.set_status(&format!("{} mode", mode.label()));
    }

    fn persist_notebook(&self, update: impl FnOnce(&mut NotebookSettings)) {
        let mut notebook = self.notebook.borrow_mut();
        let Some(notebook) = notebook.as_mut() else {
            return;
        };
        let mut settings = notebook.settings().clone();
        update(&mut settings);
        if let Err(error) = notebook.update_settings(settings) {
            self.set_status(&format!("Could not save notebook settings: {error}"));
        }
    }

    fn push_recent(&self, root: &Path) {
        let mut global = self.global.borrow_mut();
        global.recent_notebooks.retain(|path| path != root);
        global.recent_notebooks.insert(0, root.to_path_buf());
        global.recent_notebooks.truncate(10);
    }

    fn persist_global(&self) {
        let global = self.global.borrow();
        if let Err(error) = settings::save_global(&global) {
            tracing::warn!(%error, "could not save global settings");
        }
    }

    fn open_notebook_path(self: &Rc<Self>, root: PathBuf, create: bool) {
        let opened = if create {
            Notebook::create(&root).and_then(|notebook| {
                notebook.ensure_sample_notes()?;
                Ok(notebook)
            })
        } else {
            Notebook::open(&root)
        };

        let notebook = match opened {
            Ok(notebook) => notebook,
            Err(error) => {
                self.set_status(&format!("Could not open notebook: {error}"));
                return;
            }
        };

        let effective = EffectiveSettings::resolve(&self.global.borrow(), notebook.settings());
        self.effective.set(effective);

        self.ui
            .notebook_label
            .set_label(&notebook.root().display().to_string());
        self.push_recent(notebook.root());
        *self.notebook.borrow_mut() = Some(notebook.clone());
        self.refresh_rows();
        self.persist_global();
        self.apply_layout();

        let notes = notebook.list_notes().unwrap_or_default();
        match notes.first() {
            Some(first) => {
                self.ui.stack.set_visible_child_name("editor");
                self.open_note(first.clone());
            }
            None => {
                self.clear_editor();
                self.ui.stack.set_visible_child_name("editor");
                self.set_status("Notebook opened. Create a note to begin.");
            }
        }
    }

    fn pick_notebook(self: &Rc<Self>, create: bool) {
        let dialog = gtk::FileDialog::builder()
            .title(if create {
                "Choose a folder for the new notebook"
            } else {
                "Open notebook"
            })
            .modal(true)
            .build();

        let this = Rc::clone(self);
        let window = self.window.clone();
        dialog.select_folder(
            Some(&window),
            None::<&gio::Cancellable>,
            move |result| match result {
                Ok(file) => match file.path() {
                    Some(path) => this.open_notebook_path(path, create),
                    None => this.set_status("That folder is not on the local filesystem."),
                },
                Err(error) => {
                    if !error.matches(gio::IOErrorEnum::Cancelled) {
                        this.set_status(&format!("Folder chooser failed: {}", error.message()));
                    }
                }
            },
        );
    }

    fn prompt_new_note(self: &Rc<Self>) {
        if self.notebook.borrow().is_none() {
            self.set_status("Open or create a notebook first.");
            return;
        }
        let window = self.window.clone();
        let this = Rc::clone(self);
        dialogs::prompt_text(
            &window,
            "New note",
            "Title",
            "",
            "Create note",
            move |title| {
                this.create_note(&title);
            },
        );
    }

    fn create_note(self: &Rc<Self>, title: &str) {
        let Some(notebook) = self.notebook.borrow().clone() else {
            self.set_status("Open or create a notebook first.");
            return;
        };
        match notebook.create_note(title) {
            Ok(note) => {
                self.refresh_rows();
                match aporia_core::paths::relative_to(notebook.root(), note.path()) {
                    Ok(relative) => self.open_note(relative),
                    Err(error) => {
                        self.set_status(&format!("Could not address the new note: {error}"))
                    }
                }
            }
            Err(error) => self.set_status(&format!("Could not create the note: {error}")),
        }
    }

    fn rename_current(self: &Rc<Self>) {
        let Some(relative) = self.current_relative() else {
            self.set_status("Open a note first.");
            return;
        };
        let Some(notebook) = self.notebook.borrow().clone() else {
            return;
        };

        self.save_now();

        let stem = relative
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folder = relative.parent().map(Path::to_path_buf).unwrap_or_default();

        let window = self.window.clone();
        let this = Rc::clone(self);
        dialogs::prompt_text(
            &window,
            "Rename note",
            "Title",
            &stem,
            "Rename note",
            move |title| {
                let slug = aporia_core::slugify(&title);
                let target = folder.join(format!("{slug}.md"));
                match notebook.rename_note(&relative, &target) {
                    Ok(_) => {
                        this.refresh_rows();
                        this.open_note(target);
                        this.set_status("Renamed note");
                    }
                    Err(error) => this.set_status(&format!("Could not rename the note: {error}")),
                }
            },
        );
    }

    fn delete_current(self: &Rc<Self>) {
        let Some(relative) = self.current_relative() else {
            self.set_status("Open a note first.");
            return;
        };
        let Some(notebook) = self.notebook.borrow().clone() else {
            return;
        };

        let window = self.window.clone();
        let this = Rc::clone(self);
        dialogs::confirm(
            &window,
            "Delete this note?",
            "The note file is removed from the notebook. Git history, if the folder uses it, may still have a copy.",
            "Delete note",
            move || match notebook.delete_note(&relative) {
                Ok(()) => {
                    this.clear_editor();
                    this.refresh_rows();
                    this.set_status("Deleted note");
                }
                Err(error) => this.set_status(&format!("Could not delete the note: {error}")),
            },
        );
    }

    fn show_home(self: &Rc<Self>) {
        self.save_current();
        self.ui.stack.set_visible_child_name("home");
        self.refresh_home();
    }

    fn refresh_home(self: &Rc<Self>) {
        while let Some(child) = self.ui.home_buttons.first_child() {
            self.ui.home_buttons.remove(&child);
        }

        let createnew = gtk::Button::with_label("Create notebook");
        createnew.add_css_class("suggested-action");
        createnew.add_css_class("pill");
        {
            let this = Rc::clone(self);
            createnew.connect_clicked(move |_| this.pick_notebook(true));
        }

        let open = gtk::Button::with_label("Open notebook");
        open.add_css_class("pill");
        {
            let this = Rc::clone(self);
            open.connect_clicked(move |_| this.pick_notebook(false));
        }

        self.ui.home_buttons.append(&createnew);
        self.ui.home_buttons.append(&open);

        let recents = self.global.borrow().recent_notebooks.clone();
        let existing: Vec<PathBuf> = recents
            .into_iter()
            .filter(|path| path.join("notes").is_dir())
            .take(5)
            .collect();

        if existing.is_empty() {
            return;
        }

        let label = gtk::Label::builder()
            .label("Recent notebooks")
            .xalign(0.0)
            .build();
        label.add_css_class("heading");
        label.set_margin_top(18);
        self.ui.home_buttons.append(&label);

        for path in existing {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            let button = gtk::Button::with_label(&name);
            button.set_tooltip_text(Some(&path.display().to_string()));
            {
                let this = Rc::clone(self);
                let path = path.clone();
                button.connect_clicked(move |_| this.open_notebook_path(path.clone(), false));
            }
            self.ui.home_buttons.append(&button);
        }
    }

    fn show_about(&self) {
        let dialog = gtk::AlertDialog::builder()
            .modal(true)
            .message("Aporia")
            .detail(format!(
                "Version {}\n\nA local-first notebook for philosophical thinking. \
                 Notes are plain Markdown in a folder you own. No account, no telemetry. \
                 Lean is optional.",
                aporia_core::version()
            ))
            .build();
        dialog.show(Some(&self.window));
    }

    fn install_actions_state(self: &Rc<Self>) {
        let actions: Vec<(&str, gio::SimpleAction)> = {
            let this = Rc::clone(self);
            let new_note = window_action("new-note", move || this.prompt_new_note());

            let this = Rc::clone(self);
            let rename = window_action("rename-note", move || this.rename_current());

            let this = Rc::clone(self);
            let delete = window_action("delete-note", move || this.delete_current());

            let this = Rc::clone(self);
            let open = window_action("open-notebook", move || this.pick_notebook(false));

            let this = Rc::clone(self);
            let create = window_action("new-notebook", move || this.pick_notebook(true));

            let this = Rc::clone(self);
            let home = window_action("home", move || this.show_home());

            let this = Rc::clone(self);
            let toggle_preview = window_action("toggle-preview", move || this.toggle_preview());

            let this = Rc::clone(self);
            let toggle_layout = window_action("toggle-layout", move || this.toggle_layout());

            let this = Rc::clone(self);
            let toggle_mode = window_action("toggle-mode", move || this.toggle_mode());

            let this = Rc::clone(self);
            let about = window_action("about", move || this.show_about());

            let this = Rc::clone(self);
            let toggle_sidebar = window_action("toggle-sidebar", move || this.toggle_sidebar());

            vec![
                ("new-note", new_note),
                ("rename-note", rename),
                ("delete-note", delete),
                ("open-notebook", open),
                ("new-notebook", create),
                ("home", home),
                ("toggle-preview", toggle_preview),
                ("toggle-layout", toggle_layout),
                ("toggle-mode", toggle_mode),
                ("toggle-sidebar", toggle_sidebar),
                ("about", about),
            ]
        };

        for (_, action) in actions {
            self.window.add_action(&action);
        }
    }
}

fn window_action(name: &str, activate: impl Fn() + 'static) -> gio::SimpleAction {
    let action = gio::SimpleAction::new(name, None);
    action.connect_activate(move |_, _| activate());
    action
}

fn install_actions(state: &Rc<AppState>, app: &libadwaita::Application) {
    state.install_actions_state();
    app.set_accels_for_action("win.new-note", &["<Control>n"]);
    app.set_accels_for_action("win.open-notebook", &["<Control>o"]);
    app.set_accels_for_action("win.new-notebook", &["<Control><Shift>n"]);
    app.set_accels_for_action("win.rename-note", &["F2"]);
    app.set_accels_for_action("win.toggle-preview", &["<Control>e"]);
    app.set_accels_for_action("win.toggle-layout", &["<Control><Shift>e"]);
    app.set_accels_for_action("win.toggle-mode", &["<Control>m"]);
    app.set_accels_for_action("win.home", &["<Control>h"]);
}

fn display_title(relative: &Path) -> String {
    let stem = relative
        .file_stem()
        .map(|stem| stem.to_string_lossy().replace(['-', '_'], " "))
        .unwrap_or_else(|| String::from("Untitled"));
    let mut characters = stem.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => stem,
    }
}

fn text_label(text: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .build();
    label.add_css_class(class);
    label
}

fn heading_class(level: u8) -> &'static str {
    match level {
        1 => "aporia-h1",
        2 => "aporia-h2",
        _ => "aporia-h3",
    }
}

fn block_widget(block: &Block) -> gtk::Widget {
    match block {
        Block::Heading { level, text } => {
            let label = text_label(text, heading_class(*level));
            label.set_margin_top(if *level <= 2 { 18 } else { 12 });
            label.set_margin_bottom(6);
            label.upcast()
        }
        Block::Paragraph(text) => {
            let label = text_label(text, "aporia-body");
            label.set_margin_bottom(8);
            label.upcast()
        }
        Block::List {
            ordered,
            start,
            items,
        } => {
            let list = gtk::Box::new(gtk::Orientation::Vertical, 6);
            list.set_margin_bottom(8);
            for (index, item) in items.iter().enumerate() {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let marker = if *ordered {
                    format!(
                        "{}.",
                        start.saturating_add(u64::try_from(index).unwrap_or(0))
                    )
                } else {
                    String::from("•")
                };
                let marker_label = gtk::Label::builder()
                    .label(&marker)
                    .xalign(0.0)
                    .valign(gtk::Align::Start)
                    .build();
                marker_label.add_css_class("aporia-muted");

                let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
                body.set_hexpand(true);
                for inner in item {
                    body.append(&block_widget(inner));
                }

                row.append(&marker_label);
                row.append(&body);
                list.append(&row);
            }
            list.upcast()
        }
        Block::CodeBlock { language, text } => {
            let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
            container.set_margin_bottom(10);
            if let Some(language) = language {
                if !language.is_empty() {
                    container.append(&text_label(language, "aporia-muted"));
                }
            }
            container.append(&text_label(text.trim_end(), "aporia-code"));
            container.upcast()
        }
        Block::Quote(inner) => {
            let quote = gtk::Box::new(gtk::Orientation::Vertical, 4);
            quote.add_css_class("aporia-quote");
            quote.set_margin_bottom(10);
            for inner in inner {
                quote.append(&block_widget(inner));
            }
            quote.upcast()
        }
        Block::Table { headers, rows } => {
            let grid = gtk::Grid::builder()
                .row_spacing(4)
                .column_spacing(18)
                .build();
            grid.set_margin_bottom(10);
            for (column, header) in headers.iter().enumerate() {
                let label = text_label(header, "aporia-table-header");
                grid.attach(&label, i32::try_from(column).unwrap_or(0), 0, 1, 1);
            }
            for (row_index, row) in rows.iter().enumerate() {
                let y = i32::try_from(row_index).unwrap_or(0).saturating_add(1);
                for (column, cell) in row.iter().enumerate() {
                    let label = text_label(cell, "aporia-table-cell");
                    grid.attach(&label, i32::try_from(column).unwrap_or(0), y, 1, 1);
                }
            }
            grid.upcast()
        }
        Block::Rule => gtk::Separator::new(gtk::Orientation::Horizontal).upcast(),
        Block::Footnotes(notes) => {
            let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
            container.set_margin_top(16);
            container.append(&text_label("Notes", "aporia-h3"));
            for (name, text) in notes {
                container.append(&text_label(&format!("[{name}] {text}"), "aporia-caption"));
            }
            container.upcast()
        }
    }
}
