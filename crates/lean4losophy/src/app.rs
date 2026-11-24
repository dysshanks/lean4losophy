// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

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

use gtk::prelude::*;
use gtk::{gio, glib};
use lean4losophy_core::index::{
    Backlink, Index, LinkProblem, LinkResolution, SearchHit, SearchQuery,
};
use lean4losophy_core::links;
use lean4losophy_core::preview::Block;
use lean4losophy_core::settings::{
    self, EffectiveSettings, GlobalConfig, NotebookSettings, PreviewLayout,
};
use lean4losophy_core::{Note, Notebook};
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
    editor_column: gtk::Box,
    preview_scroller: gtk::ScrolledWindow,
    preview_box: gtk::Box,
    paned: gtk::Paned,
    home_buttons: gtk::Box,
    title: gtk::Label,
    status: gtk::Label,
    notebook_label: gtk::Label,
    search_entry: gtk::SearchEntry,
    tag_entry: gtk::Entry,
    folder_entry: gtk::Entry,
    filters: gtk::Box,
    results_label: gtk::Label,
    results_list: gtk::ListBox,
    sidebar_stack: gtk::Stack,
    backlinks_revealer: gtk::Revealer,
    backlinks_list: gtk::ListBox,
    activity_notes: gtk::Button,
    activity_search: gtk::Button,
    mode_label: gtk::Label,
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
    save_pending: Cell<bool>,
    preview_source: Cell<Option<glib::SourceId>>,
    preview_pending: Cell<bool>,
    monitor: RefCell<Option<gio::FileMonitor>>,
    index: RefCell<Option<Index>>,
    backlinks_rows: RefCell<Vec<Backlink>>,
    search_rows: RefCell<Vec<PathBuf>>,
    search_source: Cell<Option<glib::SourceId>>,
    search_pending: Cell<bool>,
    backlinks_visible: Cell<bool>,
    generation: Cell<u64>,
    preview_generation: Cell<u64>,
    preview_visible: Cell<bool>,
    loading: Cell<bool>,
    sidebar_visible: Cell<bool>,
}

struct PaletteCommand {
    label: &'static str,
    shortcut: &'static str,
    action: &'static str,
}

const ACTIVITY_ACTIVE: &str = "lean4losophy-activity-active";

type PaletteRebuild = Rc<dyn Fn(&str)>;
type PaletteAction = Rc<dyn Fn(String)>;

const PALETTE_COMMANDS: &[PaletteCommand] = &[
    PaletteCommand {
        label: "New note",
        shortcut: "Ctrl+N",
        action: "win.new-note",
    },
    PaletteCommand {
        label: "New notebook",
        shortcut: "Ctrl+Shift+N",
        action: "win.new-notebook",
    },
    PaletteCommand {
        label: "Open notebook",
        shortcut: "Ctrl+O",
        action: "win.open-notebook",
    },
    PaletteCommand {
        label: "Show notes",
        shortcut: "",
        action: "win.show-notes",
    },
    PaletteCommand {
        label: "Find note",
        shortcut: "Ctrl+F",
        action: "win.search",
    },
    PaletteCommand {
        label: "Backlinks",
        shortcut: "Ctrl+Shift+B",
        action: "win.toggle-backlinks",
    },
    PaletteCommand {
        label: "Toggle preview",
        shortcut: "Ctrl+E",
        action: "win.toggle-preview",
    },
    PaletteCommand {
        label: "Switch preview layout",
        shortcut: "Ctrl+Shift+E",
        action: "win.toggle-layout",
    },
    PaletteCommand {
        label: "Switch Guided/Expert mode",
        shortcut: "Ctrl+M",
        action: "win.toggle-mode",
    },
    PaletteCommand {
        label: "Notebook index",
        shortcut: "",
        action: "win.index-status",
    },
    PaletteCommand {
        label: "Home",
        shortcut: "Ctrl+H",
        action: "win.home",
    },
    PaletteCommand {
        label: "Rename note",
        shortcut: "F2",
        action: "win.rename-note",
    },
    PaletteCommand {
        label: "Delete note",
        shortcut: "",
        action: "win.delete-note",
    },
    PaletteCommand {
        label: "About lean4losophy",
        shortcut: "",
        action: "win.about",
    },
];

/// Build the window and present it.
pub fn activate(app: &libadwaita::Application) {
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("lean4losophy")
        .default_width(1100)
        .default_height(720)
        .build();

    let title = gtk::Label::builder()
        .label("lean4losophy")
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .max_width_chars(36)
        .build();
    title.add_css_class("lean4losophy-command-title");
    title.set_hexpand(true);

    let status = gtk::Label::builder().label("Ready").xalign(0.0).build();
    status.add_css_class("lean4losophy-muted");

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
    editor.add_css_class("lean4losophy-editor");
    editor.set_vexpand(true);
    editor.set_hexpand(true);

    let editor_scroller = gtk::ScrolledWindow::builder()
        .child(&editor)
        .vexpand(true)
        .hexpand(true)
        .build();

    let preview_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
    preview_box.add_css_class("lean4losophy-preview");
    preview_box.set_margin_top(18);
    preview_box.set_margin_bottom(18);
    preview_box.set_margin_start(28);
    preview_box.set_margin_end(28);

    let preview_scroller = gtk::ScrolledWindow::builder()
        .child(&preview_box)
        .vexpand(true)
        .hexpand(true)
        .build();

    let backlinks_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();
    backlinks_list.add_css_class("boxed-list");

    let backlinks_title = gtk::Label::builder()
        .label("Linked from")
        .xalign(0.0)
        .hexpand(true)
        .build();
    backlinks_title.add_css_class("heading");
    let backlinks_close = gtk::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text("Hide backlinks")
        .build();
    backlinks_close.set_action_name(Some("win.toggle-backlinks"));
    let backlinks_header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    backlinks_header.append(&backlinks_title);
    backlinks_header.append(&backlinks_close);

    let backlinks_scroller = gtk::ScrolledWindow::builder()
        .child(&backlinks_list)
        .min_content_height(120)
        .vexpand(true)
        .build();
    let backlinks_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    backlinks_box.add_css_class("lean4losophy-backlinks");
    backlinks_box.set_margin_top(10);
    backlinks_box.set_margin_bottom(10);
    backlinks_box.set_margin_start(12);
    backlinks_box.set_margin_end(12);
    backlinks_box.append(&backlinks_header);
    backlinks_box.append(&backlinks_scroller);

    let backlinks_revealer = gtk::Revealer::new();
    backlinks_revealer.set_transition_type(gtk::RevealerTransitionType::SlideUp);
    backlinks_revealer.set_child(Some(&backlinks_box));
    backlinks_revealer.set_reveal_child(false);

    let editor_column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor_column.append(&editor_scroller);
    editor_column.append(&backlinks_revealer);

    let paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&editor_column)
        .end_child(&preview_scroller)
        .position(560)
        .resize_start_child(false)
        .shrink_start_child(false)
        .build();

    let home_buttons = gtk::Box::new(gtk::Orientation::Vertical, 8);
    home_buttons.set_halign(gtk::Align::Center);
    home_buttons.set_margin_top(12);

    let home = libadwaita::StatusPage::builder()
        .title("lean4losophy")
        .description(
            "A local-first notebook for arguments. Notes are plain Markdown in a folder you own. \
             Lean is optional and can be installed later.",
        )
        .icon_name("accessories-text-editor-symbolic")
        .child(&home_buttons)
        .build();

    let empty_buttons = gtk::Box::new(gtk::Orientation::Vertical, 8);
    empty_buttons.set_halign(gtk::Align::Center);
    empty_buttons.set_margin_top(12);
    let empty_create = gtk::Button::with_label("Create first note");
    empty_create.add_css_class("suggested-action");
    empty_create.add_css_class("pill");
    empty_create.set_action_name(Some("win.new-note"));
    let empty_open = gtk::Button::with_label("Open notebook…");
    empty_open.add_css_class("pill");
    empty_open.set_action_name(Some("win.open-notebook"));
    empty_buttons.append(&empty_create);
    empty_buttons.append(&empty_open);

    let empty = libadwaita::StatusPage::builder()
        .title("No notes yet")
        .description("This notebook has no notes. Create the first one, or open another folder.")
        .icon_name("document-new-symbolic")
        .child(&empty_buttons)
        .build();

    let stack = gtk::Stack::new();
    stack.add_named(&home, Some("home"));
    stack.add_named(&paned, Some("editor"));
    stack.add_named(&empty, Some("empty"));

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

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("Search notes")
        .hexpand(true)
        .build();

    let tag_entry = gtk::Entry::builder().placeholder_text("Tag").build();
    let folder_entry = gtk::Entry::builder().placeholder_text("Folder").build();
    let tag_label = gtk::Label::builder().label("Tag").xalign(0.0).build();
    tag_label.add_css_class("lean4losophy-muted");
    let folder_label = gtk::Label::builder().label("Folder").xalign(0.0).build();
    folder_label.add_css_class("lean4losophy-muted");
    let filters = gtk::Box::new(gtk::Orientation::Vertical, 4);
    filters.append(&tag_label);
    filters.append(&tag_entry);
    filters.append(&folder_label);
    filters.append(&folder_entry);
    filters.set_visible(false);

    let results_label = gtk::Label::builder().xalign(0.0).build();
    results_label.add_css_class("lean4losophy-muted");
    results_label.set_visible(false);

    let results_list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();
    results_list.add_css_class("navigation-sidebar");
    let results_scroller = gtk::ScrolledWindow::builder()
        .child(&results_list)
        .vexpand(true)
        .build();

    let sidebar_stack = gtk::Stack::new();
    sidebar_stack.add_named(&list_scroller, Some("notes"));
    sidebar_stack.add_named(&results_scroller, Some("results"));
    sidebar_stack.set_visible_child_name("notes");
    sidebar_stack.set_vexpand(true);

    let notebook_label = gtk::Label::builder()
        .label("No notebook open")
        .xalign(0.0)
        .wrap(true)
        .build();
    notebook_label.add_css_class("lean4losophy-muted");

    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sidebar.add_css_class("lean4losophy-sidebar");
    sidebar.set_width_request(260);
    sidebar.append(&sidebar_header);
    sidebar.append(&search_entry);
    sidebar.append(&filters);
    sidebar.append(&results_label);
    sidebar.append(&sidebar_stack);
    sidebar.append(&notebook_label);

    let activity_notes = activity_button("view-list-symbolic", "Notes", "win.show-notes");
    activity_notes.add_css_class(ACTIVITY_ACTIVE);
    let activity_search = activity_button(
        "system-search-symbolic",
        "Search notes (Ctrl+F)",
        "win.search",
    );
    let activity_backlinks = activity_button(
        "insert-link-symbolic",
        "Backlinks (Ctrl+Shift+B)",
        "win.toggle-backlinks",
    );
    let activity_index = activity_button(
        "view-refresh-symbolic",
        "Notebook index",
        "win.index-status",
    );
    let activity_home = activity_button("go-home-symbolic", "Home (Ctrl+H)", "win.home");

    let activity_bar = gtk::Box::new(gtk::Orientation::Vertical, 4);
    activity_bar.add_css_class("lean4losophy-activity-bar");
    activity_bar.set_width_request(46);
    activity_bar.set_vexpand(true);
    activity_bar.append(&activity_notes);
    activity_bar.append(&activity_search);
    activity_bar.append(&activity_backlinks);
    activity_bar.append(&activity_index);
    let activity_spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    activity_spacer.set_vexpand(true);
    activity_bar.append(&activity_spacer);
    activity_bar.append(&activity_home);

    let header = libadwaita::HeaderBar::new();
    let sidebar_button = gtk::Button::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text("Show or hide the sidebar")
        .build();
    sidebar_button.set_action_name(Some("win.toggle-sidebar"));
    let backlinks_button = gtk::Button::builder()
        .icon_name("view-list-symbolic")
        .tooltip_text("Backlinks")
        .build();
    backlinks_button.set_action_name(Some("win.toggle-backlinks"));
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

    let command_icon = gtk::Image::from_icon_name("system-search-symbolic");
    let command_hint = gtk::Label::builder().label("Ctrl K").build();
    command_hint.add_css_class("lean4losophy-command-hint");
    let command_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    command_box.append(&command_icon);
    command_box.append(&title);
    command_box.append(&command_hint);

    let command_center = gtk::Button::new();
    command_center.set_child(Some(&command_box));
    command_center.set_tooltip_text(Some("Search or run a command (Ctrl+K)"));
    command_center.set_action_name(Some("win.command-palette"));
    command_center.add_css_class("flat");
    command_center.add_css_class("lean4losophy-command-center");

    let preview_button = gtk::Button::builder()
        .icon_name("view-reveal-symbolic")
        .tooltip_text("Toggle preview (Ctrl+E)")
        .build();
    preview_button.set_action_name(Some("win.toggle-preview"));

    header.pack_start(&sidebar_button);
    header.pack_start(&backlinks_button);
    header.pack_end(&menu_button);
    header.pack_end(&new_button);
    header.pack_end(&preview_button);
    header.set_title_widget(Some(&command_center));

    let mode_label = gtk::Label::builder().label("Guided").build();
    mode_label.add_css_class("lean4losophy-muted");

    let status_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_box.add_css_class("lean4losophy-statusbar");
    status.set_hexpand(true);
    status_box.append(&status);
    status_box.append(&mode_label);

    let split = libadwaita::OverlaySplitView::builder()
        .sidebar(&sidebar)
        .content(&stack)
        .min_sidebar_width(240.0)
        .show_sidebar(true)
        .build();

    let workbench = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    workbench.add_css_class("lean4losophy-workbench");
    workbench.append(&activity_bar);
    workbench.append(&split);

    let toolbar = libadwaita::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.add_bottom_bar(&status_box);
    toolbar.set_content(Some(&workbench));
    window.set_content(Some(&toolbar));

    let (saver, save_results) = crate::workers::spawn_saver();
    let (renderer, render_results) = crate::workers::spawn_renderer();

    let global = settings::load_global().unwrap_or_else(|error| {
        tracing::warn!(%error, "falling back to default settings");
        GlobalConfig::default()
    });

    window.set_default_size(global.window_width.max(800), global.window_height.max(520));
    window.set_size_request(760, 480);

    let effective = EffectiveSettings::resolve(&global, &NotebookSettings::default());
    mode_label.set_label(effective.mode.label());
    let state = Rc::new(AppState {
        window: window.clone(),
        ui: Ui {
            split,
            stack,
            list,
            editor,
            editor_scroller,
            editor_column,
            preview_scroller,
            preview_box,
            paned,
            home_buttons,
            title,
            status,
            notebook_label,
            search_entry,
            tag_entry,
            folder_entry,
            filters,
            results_label,
            results_list,
            sidebar_stack,
            backlinks_revealer,
            backlinks_list,
            activity_notes,
            activity_search,
            mode_label,
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
        save_pending: Cell::new(false),
        preview_source: Cell::new(None),
        preview_pending: Cell::new(false),
        monitor: RefCell::new(None),
        index: RefCell::new(None),
        backlinks_rows: RefCell::new(Vec::new()),
        search_rows: RefCell::new(Vec::new()),
        search_source: Cell::new(None),
        search_pending: Cell::new(false),
        backlinks_visible: Cell::new(false),
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
    view_menu.append(Some("Command palette…"), Some("win.command-palette"));
    view_menu.append(Some("Find note…"), Some("win.search"));
    view_menu.append(Some("Backlinks"), Some("win.toggle-backlinks"));
    view_menu.append(Some("Show or hide preview"), Some("win.toggle-preview"));
    view_menu.append(Some("Switch preview layout"), Some("win.toggle-layout"));
    view_menu.append(Some("Switch Guided/Expert"), Some("win.toggle-mode"));

    let app_menu = gio::Menu::new();
    app_menu.append(Some("Notebook index…"), Some("win.index-status"));
    app_menu.append(Some("About lean4losophy"), Some("win.about"));

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
        state
            .ui
            .search_entry
            .connect_search_changed(move |_| this.schedule_search());
    }
    {
        let this = Rc::clone(state);
        state
            .ui
            .tag_entry
            .connect_changed(move |_| this.schedule_search());
    }
    {
        let this = Rc::clone(state);
        state
            .ui
            .folder_entry
            .connect_changed(move |_| this.schedule_search());
    }
    {
        let this = Rc::clone(state);
        state
            .ui
            .search_entry
            .connect_activate(move |_| this.open_first_result());
    }
    {
        let this = Rc::clone(state);
        state.ui.results_list.connect_row_activated(move |_, row| {
            let index = usize::try_from(row.index()).unwrap_or(usize::MAX);
            this.activate_search_row(index);
        });
    }
    {
        let this = Rc::clone(state);
        state
            .ui
            .backlinks_list
            .connect_row_activated(move |_, row| {
                let index = usize::try_from(row.index()).unwrap_or(usize::MAX);
                this.activate_backlink(index);
            });
    }
    {
        let this = Rc::clone(state);
        let key = gtk::EventControllerKey::new();
        key.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape && this.ui.search_entry.has_focus() {
                this.clear_search();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        state.ui.search_entry.add_controller(key);
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
            {
                let mut global = this.global.borrow_mut();
                let width = this.window.width();
                let height = this.window.height();
                if width > 0 {
                    global.window_width = width;
                }
                if height > 0 {
                    global.window_height = height;
                }
            }
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
        let Some(relative) = row_at(&self.rows, index) else {
            return;
        };
        self.open_note(relative);
    }

    fn open_note(self: &Rc<Self>, relative: PathBuf) {
        self.open_note_at(relative, None);
    }

    fn open_note_at(self: &Rc<Self>, relative: PathBuf, line: Option<u32>) {
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
        if let Some(line) = line {
            self.scroll_to_line(line);
        }
        self.refresh_backlinks();
        self.set_status(if read_only {
            "Opened a newer-format note read-only"
        } else {
            "Opened note"
        });
    }

    fn scroll_to_line(&self, line: u32) {
        let buffer = self.ui.editor.buffer();
        let target = i32::try_from(line.saturating_sub(1)).unwrap_or(i32::MAX);
        let iter = buffer
            .iter_at_line(target)
            .unwrap_or_else(|| buffer.end_iter());
        buffer.place_cursor(&iter);
        self.ui.editor.scroll_mark_onscreen(&buffer.get_insert());
    }

    fn clear_editor(&self) {
        self.loading.set(true);
        self.ui.editor.buffer().set_text("");
        self.loading.set(false);
        self.ui.editor.set_editable(true);
        *self.current.borrow_mut() = None;
        *self.monitor.borrow_mut() = None;
        self.ui.title.set_label("lean4losophy");
        while let Some(child) = self.ui.preview_box.first_child() {
            self.ui.preview_box.remove(&child);
        }
        while let Some(child) = self.ui.backlinks_list.first_child() {
            self.ui.backlinks_list.remove(&child);
        }
        self.backlinks_rows.borrow_mut().clear();
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
        cancel_pending(&self.save_source, &self.save_pending);
        self.save_pending.set(true);
        let delay = self.effective.get().autosave_ms;
        let this = Rc::clone(self);
        let source = glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            this.save_pending.set(false);
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
            match lean4losophy_core::atomic::write_atomic(&job.path, &job.bytes) {
                Ok(()) => {
                    if let Some(current) = self.current.borrow_mut().as_mut() {
                        current.dirty = false;
                    }
                    if let Some(relative) = self.current_relative() {
                        self.reindex_and_refresh(&relative);
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
            match lean4losophy_core::atomic::write_atomic(&path, &bytes) {
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
        cancel_pending(&self.preview_source, &self.preview_pending);
        self.preview_pending.set(true);
        let generation = self.preview_generation.get().wrapping_add(1);
        self.preview_generation.set(generation);
        let markdown = self.editor_text();
        let this = Rc::clone(self);
        let source = glib::timeout_add_local_once(Duration::from_millis(150), move || {
            this.preview_pending.set(false);
            this.request_preview(markdown, generation);
        });
        self.preview_source.set(Some(source));
    }

    fn request_preview_immediately(self: &Rc<Self>) {
        cancel_pending(&self.preview_source, &self.preview_pending);
        let generation = self.preview_generation.get().wrapping_add(1);
        self.preview_generation.set(generation);
        let markdown = self.editor_text();
        self.request_preview(markdown, generation);
    }

    fn request_preview(self: &Rc<Self>, markdown: String, generation: u64) {
        let job = RenderJob {
            markdown,
            generation,
        };
        if let Err(job) = self.renderer.send(job) {
            let blocks = lean4losophy_core::preview::blocks(&job.markdown);
            self.apply_preview(&blocks);
        }
    }

    fn apply_preview(self: &Rc<Self>, blocks: &[Block]) {
        while let Some(child) = self.ui.preview_box.first_child() {
            self.ui.preview_box.remove(&child);
        }
        for block in blocks {
            self.ui.preview_box.append(&block_widget(block, self));
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
                        let root = self
                            .notebook
                            .borrow()
                            .as_ref()
                            .map(|notebook| notebook.root().to_path_buf());
                        if let Some(root) = root
                            && let Ok(relative) = path.strip_prefix(&root)
                        {
                            let relative = relative.to_path_buf();
                            self.reindex_and_refresh(&relative);
                        }
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
            relative: relative.clone(),
            note,
            dirty: false,
        });
        self.request_preview_immediately();
        self.reindex_and_refresh(&relative);
        self.set_status("Reloaded the note from disk");
    }

    fn focus_search(self: &Rc<Self>) {
        if self.notebook.borrow().is_none() {
            self.set_status("Open a notebook first.");
            return;
        }
        self.ui.split.set_show_sidebar(true);
        self.sidebar_visible.set(true);
        self.set_activity("search");
        self.ui.search_entry.grab_focus();
        self.ui.search_entry.select_region(0, -1);
    }

    fn show_notes(self: &Rc<Self>) {
        if self.notebook.borrow().is_none() {
            self.set_status("Open a notebook first.");
            return;
        }
        self.ui.split.set_show_sidebar(true);
        self.sidebar_visible.set(true);
        self.clear_search();
    }

    fn clear_search(self: &Rc<Self>) {
        self.ui.search_entry.set_text("");
        self.ui.tag_entry.set_text("");
        self.ui.folder_entry.set_text("");
        self.ui.sidebar_stack.set_visible_child_name("notes");
        self.ui.filters.set_visible(false);
        self.ui.results_label.set_visible(false);
        self.set_activity("notes");
    }

    fn schedule_search(self: &Rc<Self>) {
        cancel_pending(&self.search_source, &self.search_pending);
        self.search_pending.set(true);
        let this = Rc::clone(self);
        let source = glib::timeout_add_local_once(Duration::from_millis(180), move || {
            this.search_pending.set(false);
            this.run_search();
        });
        self.search_source.set(Some(source));
    }

    fn run_search(self: &Rc<Self>) {
        let text = self.ui.search_entry.text().to_string();
        let tag = self.ui.tag_entry.text().trim().to_owned();
        let folder = self.ui.folder_entry.text().trim().to_owned();
        let active = !text.trim().is_empty() || !tag.is_empty() || !folder.is_empty();
        if active {
            self.set_activity("search");
        }

        if !active {
            self.ui.sidebar_stack.set_visible_child_name("notes");
            self.ui.filters.set_visible(false);
            self.ui.results_label.set_visible(false);
            return;
        }

        self.ui.filters.set_visible(true);

        let mut query = SearchQuery::new(text);
        query.tag = if tag.is_empty() { None } else { Some(tag) };
        query.folder = if folder.is_empty() {
            None
        } else {
            Some(folder)
        };

        let index = self.index.borrow();
        let Some(index) = index.as_ref() else {
            self.set_status("Search needs the notebook index, which is unavailable.");
            return;
        };

        match index.search(&query) {
            Ok(hits) => self.render_results(&hits),
            Err(error) => self.set_status(&format!("Search failed: {error}")),
        }
    }

    fn render_results(self: &Rc<Self>, hits: &[SearchHit]) {
        while let Some(child) = self.ui.results_list.first_child() {
            self.ui.results_list.remove(&child);
        }
        self.search_rows.borrow_mut().clear();

        for hit in hits {
            self.search_rows.borrow_mut().push(hit.path.clone());

            let row = gtk::ListBoxRow::new();
            row.set_tooltip_text(Some(&hit.path.display().to_string()));
            let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
            content.set_margin_start(8);
            content.set_margin_end(8);
            content.set_margin_top(8);
            content.set_margin_bottom(8);

            let title = gtk::Label::builder()
                .use_markup(true)
                .label(format!("<b>{}</b>", glib::markup_escape_text(&hit.title)))
                .xalign(0.0)
                .build();
            content.append(&title);

            if !hit.snippet.is_empty() {
                let snippet = gtk::Label::builder()
                    .use_markup(true)
                    .label(snippet_markup(&hit.snippet))
                    .xalign(0.0)
                    .wrap(true)
                    .max_width_chars(32)
                    .build();
                snippet.add_css_class("lean4losophy-muted");
                content.append(&snippet);
            } else {
                let path = gtk::Label::builder()
                    .label(hit.path.display().to_string())
                    .xalign(0.0)
                    .build();
                path.add_css_class("lean4losophy-muted");
                content.append(&path);
            }

            row.set_child(Some(&content));
            self.ui.results_list.append(&row);
        }

        self.ui.results_label.set_label(&format!(
            "{} result{}",
            hits.len(),
            if hits.len() == 1 { "" } else { "s" }
        ));
        self.ui.results_label.set_visible(true);
        self.ui.sidebar_stack.set_visible_child_name("results");

        if hits.is_empty() {
            let row = gtk::ListBoxRow::new();
            row.set_activatable(false);
            let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
            content.set_margin_start(8);
            content.set_margin_end(8);
            content.set_margin_top(8);
            content.set_margin_bottom(8);

            let label = gtk::Label::builder()
                .label("No notes match.")
                .xalign(0.0)
                .wrap(true)
                .build();
            label.add_css_class("lean4losophy-muted");

            let clear = gtk::Button::with_label("Clear search");
            clear.add_css_class("pill");
            clear.set_halign(gtk::Align::Start);
            let this = Rc::clone(self);
            clear.connect_clicked(move |_| {
                this.clear_search();
                this.ui.search_entry.grab_focus();
            });

            content.append(&label);
            content.append(&clear);
            row.set_child(Some(&content));
            self.ui.results_list.append(&row);
        }
    }

    fn open_first_result(self: &Rc<Self>) {
        if let Some(path) = row_at(&self.search_rows, 0) {
            self.open_note(path);
        }
    }

    fn activate_search_row(self: &Rc<Self>, index: usize) {
        let Some(path) = row_at(&self.search_rows, index) else {
            return;
        };
        self.open_note(path);
    }

    fn toggle_backlinks(self: &Rc<Self>) {
        let visible = !self.backlinks_visible.get();
        self.backlinks_visible.set(visible);
        self.ui.backlinks_revealer.set_reveal_child(visible);
        if visible {
            self.refresh_backlinks();
        }
    }

    fn refresh_backlinks(self: &Rc<Self>) {
        while let Some(child) = self.ui.backlinks_list.first_child() {
            self.ui.backlinks_list.remove(&child);
        }
        self.backlinks_rows.borrow_mut().clear();

        let placeholder = |text: &str| {
            let row = gtk::ListBoxRow::new();
            row.set_activatable(false);
            row.set_selectable(false);
            let label = gtk::Label::builder()
                .label(text)
                .xalign(0.0)
                .wrap(true)
                .build();
            label.add_css_class("lean4losophy-muted");
            label.set_margin_start(8);
            label.set_margin_end(8);
            label.set_margin_top(6);
            label.set_margin_bottom(6);
            row.set_child(Some(&label));
            row
        };

        let Some(relative) = self.current_relative() else {
            self.ui.backlinks_list.append(&placeholder("No note open."));
            return;
        };

        let result = {
            let index = self.index.borrow();
            index.as_ref().map(|index| index.backlinks(&relative))
        };

        match result {
            Some(Ok(backlinks)) => {
                *self.backlinks_rows.borrow_mut() = backlinks.clone();
                if backlinks.is_empty() {
                    let row = gtk::ListBoxRow::new();
                    row.set_activatable(false);
                    row.set_selectable(false);
                    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
                    content.set_margin_start(8);
                    content.set_margin_end(8);
                    content.set_margin_top(8);
                    content.set_margin_bottom(8);

                    let label = gtk::Label::builder()
                        .label("No notes link here yet.")
                        .xalign(0.0)
                        .wrap(true)
                        .build();
                    label.add_css_class("lean4losophy-muted");

                    let find = gtk::Button::with_label("Find notes to link");
                    find.add_css_class("pill");
                    find.set_halign(gtk::Align::Start);
                    let this = Rc::clone(self);
                    find.connect_clicked(move |_| this.focus_search());

                    content.append(&label);
                    content.append(&find);
                    row.set_child(Some(&content));
                    self.ui.backlinks_list.append(&row);
                    return;
                }
                for link in backlinks {
                    let row = gtk::ListBoxRow::new();
                    row.set_tooltip_text(Some(&format!("{}:{}", link.from.display(), link.line)));
                    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
                    content.set_margin_start(8);
                    content.set_margin_end(8);
                    content.set_margin_top(8);
                    content.set_margin_bottom(8);

                    let title = gtk::Label::builder()
                        .use_markup(true)
                        .label(format!("<b>{}</b>", glib::markup_escape_text(&link.title)))
                        .xalign(0.0)
                        .build();
                    let context = gtk::Label::builder()
                        .label(&link.context)
                        .xalign(0.0)
                        .wrap(true)
                        .max_width_chars(32)
                        .build();
                    context.add_css_class("lean4losophy-muted");

                    content.append(&title);
                    content.append(&context);
                    row.set_child(Some(&content));
                    self.ui.backlinks_list.append(&row);
                }
            }
            Some(Err(error)) => {
                self.set_status(&format!("Could not read backlinks: {error}"));
                self.ui
                    .backlinks_list
                    .append(&placeholder("Backlinks are unavailable."));
            }
            None => {
                self.ui
                    .backlinks_list
                    .append(&placeholder("Backlinks are unavailable for this notebook."));
            }
        }
    }

    fn activate_backlink(self: &Rc<Self>, index: usize) {
        let Some(link) = row_at(&self.backlinks_rows, index) else {
            return;
        };
        self.open_note_at(link.from, Some(link.line));
    }

    fn reindex_note(&self, relative: &Path) {
        let Some(notebook) = self.notebook.borrow().clone() else {
            return;
        };
        let result = match self.index.borrow_mut().as_mut() {
            Some(index) => index.reindex_note(&notebook, relative),
            None => Ok(()),
        };
        if let Err(error) = result {
            self.set_status(&format!("The index could not be updated: {error}"));
        }
    }

    fn reindex_and_refresh(self: &Rc<Self>, relative: &Path) {
        self.reindex_note(relative);
        if self.current_relative().as_deref() == Some(relative) {
            self.refresh_backlinks();
        }
    }

    fn follow_link(self: &Rc<Self>, raw: &str) {
        let Some(parts) = links::parse_inside(raw) else {
            return;
        };
        if parts.malformed {
            self.set_status("That link target is not valid.");
            return;
        }
        let Some(from) = self.current_relative() else {
            return;
        };

        let resolution = {
            let index = self.index.borrow();
            let Some(index) = index.as_ref() else {
                self.set_status("Links need the notebook index, which is unavailable.");
                return;
            };
            match index.resolve(
                &from,
                &parts.target,
                parts.heading.as_deref(),
                parts.block_id.as_deref(),
            ) {
                Ok(resolution) => resolution,
                Err(error) => {
                    self.set_status(&format!("Could not resolve the link: {error}"));
                    return;
                }
            }
        };

        match resolution {
            LinkResolution::Resolved { path, line } => self.open_note_at(path, line),
            LinkResolution::Ambiguous { candidates } => {
                let names = candidates
                    .iter()
                    .take(3)
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.set_status(&format!("This link matches more than one note: {names}"));
            }
            LinkResolution::Broken => {
                if parts.heading.is_some() || parts.block_id.is_some() {
                    self.set_status("The linked note, heading, or block was not found.");
                    return;
                }
                if parts.target.trim().is_empty() {
                    self.set_status("The link has no target.");
                    return;
                }
                self.confirm_create_from_link(parts.target.clone());
            }
        }
    }

    fn confirm_create_from_link(self: &Rc<Self>, target: String) {
        let window = self.window.clone();
        let this = Rc::clone(self);
        let title = target.clone();
        dialogs::confirm(
            &window,
            "Create this note?",
            &format!("No note matches \"{target}\". Create it now?"),
            "Don't create",
            "Create note",
            move || this.create_note(&title),
        );
    }

    fn linked_text_label(self: &Rc<Self>, text: &str, class: &str) -> gtk::Label {
        let label = gtk::Label::builder()
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .build();
        label.add_css_class(class);
        label.set_markup(&link_markup(text));

        let this = Rc::clone(self);
        label.connect_activate_link(move |_, uri| {
            if let Some(encoded) = uri.strip_prefix("lean4losophy:") {
                if let Some(raw) = percent_decode(encoded) {
                    this.follow_link(&raw);
                }
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        label
    }

    fn index_summary(&self) -> String {
        let Some(notebook) = self.notebook.borrow().clone() else {
            return String::from("No notebook open.");
        };
        let counts = self.index.borrow().as_ref().map(|index| index.counts());

        match counts {
            Some(Ok(counts)) => format!(
                "Notes indexed: {}\nLinks: {}\nBroken: {}\nAmbiguous: {}\n\nCache: {}",
                counts.notes,
                counts.links,
                counts.broken,
                counts.ambiguous,
                notebook
                    .root()
                    .join(".aporia")
                    .join("cache.sqlite")
                    .display()
            ),
            Some(Err(error)) => format!("The index could not be read: {error}"),
            None => String::from("Search and backlinks are unavailable for this notebook."),
        }
    }

    fn populate_problems(&self, list: &gtk::ListBox) {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }

        let problems: Vec<LinkProblem> = {
            let index = self.index.borrow();
            match index.as_ref().map(Index::problems) {
                Some(Ok(problems)) => problems,
                Some(Err(error)) => {
                    self.set_status(&format!("Could not read link problems: {error}"));
                    Vec::new()
                }
                None => Vec::new(),
            }
        };

        if problems.is_empty() {
            let row = gtk::ListBoxRow::new();
            row.set_activatable(false);
            let label = gtk::Label::builder()
                .label("No broken or ambiguous links.")
                .xalign(0.0)
                .wrap(true)
                .build();
            label.add_css_class("lean4losophy-muted");
            row.set_child(Some(&label));
            list.append(&row);
            return;
        }

        for problem in problems {
            let row = gtk::ListBoxRow::new();
            row.set_activatable(false);
            row.set_tooltip_text(Some(&format!(
                "{}:{}",
                problem.note.display(),
                problem.line
            )));
            let label = gtk::Label::builder()
                .label(format!(
                    "{} — {} ({})",
                    problem.title, problem.target, problem.resolution
                ))
                .xalign(0.0)
                .wrap(true)
                .build();
            label.set_margin_start(8);
            label.set_margin_end(8);
            row.set_child(Some(&label));
            list.append(&row);
        }
    }

    fn show_index_status(self: &Rc<Self>) {
        let summary = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .label(self.index_summary())
            .build();

        let problems_label = gtk::Label::builder()
            .label("Broken and ambiguous links")
            .xalign(0.0)
            .build();
        problems_label.add_css_class("heading");

        let problems_list = gtk::ListBox::new();
        problems_list.add_css_class("boxed-list");
        self.populate_problems(&problems_list);
        let problems_scroller = gtk::ScrolledWindow::builder()
            .child(&problems_list)
            .min_content_height(160)
            .vexpand(true)
            .build();

        let window = gtk::Window::builder()
            .title("Notebook index")
            .transient_for(&self.window)
            .modal(true)
            .default_width(540)
            .default_height(480)
            .build();

        let rebuild = gtk::Button::with_label("Rebuild index");
        rebuild.add_css_class("suggested-action");
        let close = gtk::Button::with_label("Close");
        {
            let window = window.clone();
            close.connect_clicked(move |_| window.close());
        }
        {
            let this = Rc::clone(self);
            let summary = summary.clone();
            let problems_list = problems_list.clone();
            rebuild.connect_clicked(move |_| {
                let Some(notebook) = this.notebook.borrow().clone() else {
                    return;
                };
                *this.index.borrow_mut() = None;
                match Index::rebuild(&notebook) {
                    Ok(index) => {
                        *this.index.borrow_mut() = Some(index);
                        summary.set_label(&this.index_summary());
                        this.populate_problems(&problems_list);
                        this.refresh_rows();
                        this.refresh_backlinks();
                        this.set_status("Rebuilt the notebook index");
                    }
                    Err(error) => {
                        summary.set_label(&format!("Rebuild failed: {error}"));
                        this.set_status(&format!("Index rebuild failed: {error}"));
                    }
                }
            });
        }

        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_halign(gtk::Align::End);
        buttons.append(&rebuild);
        buttons.append(&close);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(18);
        content.set_margin_end(18);
        content.append(&summary);
        content.append(&problems_label);
        content.append(&problems_scroller);
        content.append(&buttons);
        window.set_child(Some(&content));
        window.present();
    }

    fn set_activity(&self, panel: &str) {
        let (active, inactive) = if panel == "search" {
            (&self.ui.activity_search, &self.ui.activity_notes)
        } else {
            (&self.ui.activity_notes, &self.ui.activity_search)
        };
        active.add_css_class(ACTIVITY_ACTIVE);
        inactive.remove_css_class(ACTIVITY_ACTIVE);
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
                self.ui.editor_column.set_visible(true);
                self.ui.editor_scroller.set_visible(true);
                self.ui
                    .preview_scroller
                    .set_visible(self.preview_visible.get());
                self.ui.paned.set_position(560);
            }
            PreviewLayout::Toggle => {
                let show_preview = self.preview_visible.get();
                self.ui.editor_column.set_visible(!show_preview);
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
        self.ui.mode_label.set_label(mode.label());
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
        self.ui.mode_label.set_label(effective.mode.label());

        self.ui
            .notebook_label
            .set_label(&notebook.root().display().to_string());
        self.push_recent(notebook.root());
        *self.notebook.borrow_mut() = Some(notebook.clone());

        match Index::open(&notebook) {
            Ok(index) => *self.index.borrow_mut() = Some(index),
            Err(error) => {
                *self.index.borrow_mut() = None;
                self.set_status(&format!(
                    "Search and backlinks are unavailable for this notebook: {error}"
                ));
            }
        }

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
                self.ui.stack.set_visible_child_name("empty");
                self.set_status("This notebook has no notes yet.");
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
                match lean4losophy_core::paths::relative_to(notebook.root(), note.path()) {
                    Ok(relative) => {
                        self.reindex_note(&relative);
                        self.open_note(relative);
                    }
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
                let slug = lean4losophy_core::slugify(&title);
                let target = folder.join(format!("{slug}.md"));
                let renamed = match this.index.borrow_mut().as_mut() {
                    Some(index) => index.rename_note(&notebook, &relative, &target),
                    None => notebook
                        .rename_note(&relative, &target)
                        .map(|_| Vec::<PathBuf>::new()),
                };
                match renamed {
                    Ok(rewritten) => {
                        this.refresh_rows();
                        this.open_note(target);
                        if rewritten.is_empty() {
                            this.set_status("Renamed note");
                        } else {
                            this.set_status(&format!(
                                "Renamed note; updated {} incoming link(s)",
                                rewritten.len()
                            ));
                        }
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
            "Keep note",
            "Delete note",
            move || match notebook.delete_note(&relative) {
                Ok(()) => {
                    if let Some(index) = this.index.borrow_mut().as_mut()
                        && let Err(error) = index.remove_note(&relative)
                    {
                        tracing::warn!(%error, "could not drop the deleted note from the index");
                    }
                    this.clear_editor();
                    this.refresh_rows();
                    match notebook.list_notes().unwrap_or_default().first() {
                        Some(next) => this.open_note(next.clone()),
                        None => this.ui.stack.set_visible_child_name("empty"),
                    }
                    this.set_status("Deleted note");
                }
                Err(error) => this.set_status(&format!("Could not delete the note: {error}")),
            },
        );
    }

    fn show_command_palette(self: &Rc<Self>) {
        let palette = gtk::Window::builder()
            .title("Command palette")
            .modal(true)
            .decorated(false)
            .resizable(false)
            .default_width(560)
            .default_height(420)
            .build();
        palette.set_transient_for(Some(&self.window));
        palette.add_css_class("lean4losophy-palette");

        let entry = gtk::SearchEntry::builder()
            .placeholder_text("Search notes or run a command")
            .hexpand(true)
            .build();
        entry.add_css_class("lean4losophy-palette-search");

        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();
        list.add_css_class("lean4losophy-palette-list");
        let scroller = gtk::ScrolledWindow::builder()
            .child(&list)
            .vexpand(true)
            .min_content_height(260)
            .build();

        let footer = gtk::Label::builder()
            .label("Type to filter · ↑↓ to move · Enter to run · Esc to close")
            .xalign(0.0)
            .build();
        footer.add_css_class("lean4losophy-muted");
        footer.add_css_class("lean4losophy-caption");

        let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
        root.add_css_class("lean4losophy-palette-body");
        root.append(&entry);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&scroller);
        root.append(&footer);
        palette.set_child(Some(&root));

        let rebuild: PaletteRebuild = {
            let list = list.clone();
            Rc::new(move |query: &str| {
                while let Some(child) = list.first_child() {
                    if let Some(row) = child.downcast_ref::<gtk::ListBoxRow>() {
                        list.remove(row);
                    } else {
                        break;
                    }
                }
                let query = query.trim().to_lowercase();
                for command in PALETTE_COMMANDS {
                    if !query.is_empty() && !command.label.to_lowercase().contains(&query) {
                        continue;
                    }
                    let row = gtk::ListBoxRow::new();
                    row.set_activatable(true);
                    row.set_widget_name(command.action);
                    let line = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    let label = gtk::Label::builder()
                        .label(command.label)
                        .xalign(0.0)
                        .hexpand(true)
                        .build();
                    let shortcut = gtk::Label::builder()
                        .label(command.shortcut)
                        .xalign(1.0)
                        .build();
                    shortcut.add_css_class("lean4losophy-muted");
                    shortcut.add_css_class("lean4losophy-caption");
                    line.append(&label);
                    line.append(&shortcut);
                    row.set_child(Some(&line));
                    list.append(&row);
                }
                if let Some(row) = list.row_at_index(0) {
                    list.select_row(Some(&row));
                }
            })
        };
        rebuild("");
        {
            let rebuild = Rc::clone(&rebuild);
            entry.connect_search_changed(move |entry| rebuild(entry.text().as_str()));
        }

        let run_action: PaletteAction = {
            let palette = palette.clone();
            let window = self.window.clone();
            Rc::new(move |action: String| {
                palette.close();
                if !action.is_empty()
                    && let Err(error) =
                        gtk::prelude::WidgetExt::activate_action(&window, action.as_str(), None)
                {
                    tracing::warn!(
                        %error,
                        action = action.as_str(),
                        "command palette action failed"
                    );
                }
            })
        };
        {
            let run_action = Rc::clone(&run_action);
            list.connect_row_activated(move |_, row| {
                run_action(row.widget_name().to_string());
            });
        }
        {
            let run_action = Rc::clone(&run_action);
            let list = list.clone();
            entry.connect_activate(move |_| {
                if let Some(row) = list.selected_row() {
                    run_action(row.widget_name().to_string());
                }
            });
        }
        {
            let palette_for_keys = palette.clone();
            let list = list.clone();
            let run_action = Rc::clone(&run_action);
            let key = gtk::EventControllerKey::new();
            key.set_propagation_phase(gtk::PropagationPhase::Capture);
            key.connect_key_pressed(move |_, key, _, _| match key {
                gtk::gdk::Key::Escape => {
                    palette_for_keys.close();
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Up => {
                    move_palette_selection(&list, -1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Down => {
                    move_palette_selection(&list, 1);
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::Return => {
                    if let Some(row) = list.selected_row() {
                        run_action(row.widget_name().to_string());
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            });
            palette.add_controller(key);
        }

        palette.present();
        entry.grab_focus();
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
            .message("lean4losophy")
            .detail(format!(
                "Version {}\n\nA local-first notebook for philosophical thinking. \
                 Notes are plain Markdown in a folder you own. No account, no telemetry. \
                 Lean is optional.",
                lean4losophy_core::version()
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

            let this = Rc::clone(self);
            let search = window_action("search", move || this.focus_search());

            let this = Rc::clone(self);
            let toggle_backlinks =
                window_action("toggle-backlinks", move || this.toggle_backlinks());

            let this = Rc::clone(self);
            let index_status = window_action("index-status", move || this.show_index_status());

            let this = Rc::clone(self);
            let show_notes = window_action("show-notes", move || this.show_notes());

            let this = Rc::clone(self);
            let command_palette =
                window_action("command-palette", move || this.show_command_palette());

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
                ("search", search),
                ("show-notes", show_notes),
                ("command-palette", command_palette),
                ("toggle-backlinks", toggle_backlinks),
                ("index-status", index_status),
                ("about", about),
            ]
        };

        for (_, action) in actions {
            self.window.add_action(&action);
        }
    }
}

/// Cancel a scheduled one-shot timeout, but only if it has not fired yet.
///
/// `SourceId::remove` panics when the source is already gone, and a one-shot
/// removes itself when it runs. The pending flag is cleared inside the timeout
/// callback, so this never removes a source that already fired.
fn cancel_pending(source: &Cell<Option<glib::SourceId>>, pending: &Cell<bool>) {
    if let Some(source) = source.take()
        && pending.replace(false)
    {
        source.remove();
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
    app.set_accels_for_action("win.search", &["<Control>f"]);
    app.set_accels_for_action("win.command-palette", &["<Control>k", "<Control><Shift>p"]);
    app.set_accels_for_action("win.toggle-backlinks", &["<Control><Shift>b"]);
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

fn activity_button(icon_name: &str, tooltip: &str, action: &str) -> gtk::Button {
    let button = gtk::Button::builder()
        .icon_name(icon_name)
        .tooltip_text(tooltip)
        .build();
    button.set_action_name(Some(action));
    button.add_css_class("flat");
    button.add_css_class("lean4losophy-activity-button");
    button
}

/// Clone a row out of its list before activating it.
///
/// The activation path can rebuild the same list. Keeping the guard inside this
/// helper drops the borrow first, so a refresh cannot hit `RefCell` re-entrantly.
fn row_at<T: Clone>(rows: &RefCell<Vec<T>>, index: usize) -> Option<T> {
    rows.borrow().get(index).cloned()
}

fn move_palette_selection(list: &gtk::ListBox, delta: i32) {
    let current = list.selected_row().map_or(0, |row| row.index());
    let target = current.saturating_add(delta).max(0);
    if let Some(row) = list.row_at_index(target) {
        list.select_row(Some(&row));
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

fn numeric_columns(columns: usize, rows: &[Vec<String>]) -> Vec<bool> {
    (0..columns)
        .map(|column| {
            let mut saw_value = false;
            let numeric = rows.iter().all(|row| {
                let Some(cell) = row.get(column) else {
                    return true;
                };
                let text = cell.trim();
                if text.is_empty() {
                    return true;
                }
                saw_value = true;
                is_numeric_cell(text)
            });
            numeric && saw_value
        })
        .collect()
}

fn is_numeric_cell(text: &str) -> bool {
    let text = text.trim();
    let text = text.strip_suffix('%').unwrap_or(text);
    let normalized = text.replace(',', "");
    !normalized.is_empty() && normalized.parse::<f64>().is_ok()
}

fn heading_class(level: u8) -> &'static str {
    match level {
        1 => "lean4losophy-h1",
        2 => "lean4losophy-h2",
        _ => "lean4losophy-h3",
    }
}

fn block_widget(block: &Block, state: &Rc<AppState>) -> gtk::Widget {
    match block {
        Block::Heading { level, text } => {
            let label = state.linked_text_label(text, heading_class(*level));
            label.set_margin_top(if *level <= 2 { 18 } else { 12 });
            label.set_margin_bottom(6);
            label.upcast()
        }
        Block::Paragraph(text) => {
            let label = state.linked_text_label(text, "lean4losophy-body");
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
                marker_label.add_css_class("lean4losophy-muted");

                let body = gtk::Box::new(gtk::Orientation::Vertical, 4);
                body.set_hexpand(true);
                for inner in item {
                    body.append(&block_widget(inner, state));
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
            if let Some(language) = language
                && !language.is_empty()
            {
                container.append(&text_label(language, "lean4losophy-muted"));
            }
            container.append(&text_label(text.trim_end(), "lean4losophy-code"));
            container.upcast()
        }
        Block::Quote(inner) => {
            let quote = gtk::Box::new(gtk::Orientation::Vertical, 4);
            quote.add_css_class("lean4losophy-quote");
            quote.set_margin_bottom(10);
            for inner in inner {
                quote.append(&block_widget(inner, state));
            }
            quote.upcast()
        }
        Block::Table { headers, rows } => {
            let numeric = numeric_columns(headers.len(), rows);
            let grid = gtk::Grid::builder()
                .row_spacing(4)
                .column_spacing(18)
                .build();
            grid.set_margin_bottom(10);
            for (column, header) in headers.iter().enumerate() {
                let label = state.linked_text_label(header, "lean4losophy-table-header");
                if numeric.get(column).copied().unwrap_or(false) {
                    label.set_xalign(1.0);
                    label.add_css_class("lean4losophy-numeric");
                }
                grid.attach(&label, i32::try_from(column).unwrap_or(0), 0, 1, 1);
            }
            for (row_index, row) in rows.iter().enumerate() {
                let y = i32::try_from(row_index).unwrap_or(0).saturating_add(1);
                for (column, cell) in row.iter().enumerate() {
                    let label = state.linked_text_label(cell, "lean4losophy-table-cell");
                    if numeric.get(column).copied().unwrap_or(false) {
                        label.set_xalign(1.0);
                        label.add_css_class("lean4losophy-numeric");
                    }
                    grid.attach(&label, i32::try_from(column).unwrap_or(0), y, 1, 1);
                }
            }
            grid.upcast()
        }
        Block::Rule => gtk::Separator::new(gtk::Orientation::Horizontal).upcast(),
        Block::Footnotes(notes) => {
            let container = gtk::Box::new(gtk::Orientation::Vertical, 4);
            container.set_margin_top(16);
            container.append(&text_label("Notes", "lean4losophy-h3"));
            for (name, text) in notes {
                container.append(&text_label(
                    &format!("[{name}] {text}"),
                    "lean4losophy-caption",
                ));
            }
            container.upcast()
        }
    }
}

fn link_markup(text: &str) -> String {
    let mut markup = String::new();

    for (index, segment) in text.split('`').enumerate() {
        if index > 0 {
            markup.push('`');
        }

        if index % 2 == 1 {
            markup.push_str(&glib::markup_escape_text(segment));
            continue;
        }

        let mut offset = 0;
        for link in links::scan_text(segment) {
            if link.malformed {
                continue;
            }
            markup.push_str(&glib::markup_escape_text(&segment[offset..link.span.start]));
            markup.push_str("<a href=\"lean4losophy:");
            markup.push_str(&percent_encode(&link.inside()));
            markup.push_str("\"><u>");
            markup.push_str(&glib::markup_escape_text(&link.display()));
            markup.push_str("</u></a>");
            offset = link.span.end;
        }
        markup.push_str(&glib::markup_escape_text(&segment[offset..]));
    }

    markup
}

fn snippet_markup(snippet: &str) -> String {
    glib::markup_escape_text(snippet)
        .replace('\u{1}', "<b>")
        .replace('\u{2}', "</b>")
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push(high * 16 + low);
            index = index.saturating_add(3);
        } else {
            decoded.push(bytes[index]);
            index = index.saturating_add(1);
        }
    }

    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use gtk::glib;

    use super::{cancel_pending, row_at};

    #[test]
    fn row_lookup_releases_its_borrow() {
        let rows = RefCell::new(vec![String::from("note.md")]);
        let row = row_at(&rows, 0);

        // Activation refreshes the list; the lookup must not hold a guard here.
        rows.borrow_mut().clear();

        assert_eq!(row.as_deref(), Some("note.md"));
        assert!(row_at(&rows, 0).is_none());
    }

    #[test]
    fn cancelling_a_fired_timeout_is_a_no_op() {
        let pending = Rc::new(Cell::new(true));
        let flag = Rc::clone(&pending);
        let source = glib::timeout_add_local_once(Duration::ZERO, move || {
            flag.set(false);
        });
        let stored = Cell::new(Some(source));

        let context = glib::MainContext::default();
        for _ in 0..1_000 {
            if !pending.get() {
                break;
            }
            let _ = context.iteration(false);
        }
        assert!(!pending.get(), "the one-shot timeout did not run");

        // The source has fired and removed itself, but its id is still stored.
        // `SourceId::remove` used to panic here, aborting inside a GTK callback.
        cancel_pending(&stored, &pending);
        assert!(stored.take().is_none());
    }
}
