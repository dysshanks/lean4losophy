//! Window construction and the open/save actions.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gio;
use gtk::prelude::*;

/// Shared state for the shell.
///
/// `current_file` is `None` until a note has been opened or saved. `dirty`
/// tracks buffer changes so the title can show an unsaved marker. Both are
/// single-threaded because every user of them runs on the GTK main context.
#[derive(Clone)]
struct Shell {
    window: gtk::ApplicationWindow,
    buffer: gtk::TextBuffer,
    title: gtk::Label,
    status: gtk::Label,
    current_file: Rc<RefCell<Option<gio::File>>>,
    dirty: Rc<Cell<bool>>,
}

pub fn build(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("lean4losophy — GTK shell spike")
        .default_width(1100)
        .default_height(720)
        .build();

    let title = gtk::Label::builder().label("Untitled").build();
    let status = gtk::Label::builder()
        .label("No note open")
        .xalign(0.0)
        .build();
    status.add_css_class("dim-label");

    let buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);

    let shell = Shell {
        window: window.clone(),
        buffer: buffer.clone(),
        title: title.clone(),
        status: status.clone(),
        current_file: Rc::new(RefCell::new(None)),
        dirty: Rc::new(Cell::new(false)),
    };

    let header = build_header(&shell);
    window.set_titlebar(Some(&header));

    let sidebar = build_sidebar();
    let editor = build_editor(&buffer);
    let paned = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(&sidebar)
        .end_child(&editor)
        .position(260)
        .resize_start_child(false)
        .shrink_start_child(false)
        .build();

    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_bar.set_margin_top(6);
    status_bar.set_margin_bottom(6);
    status_bar.set_margin_start(16);
    status_bar.set_margin_end(16);
    status_bar.append(&status);

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&paned);
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    root.append(&status_bar);
    window.set_child(Some(&root));

    let save_action = gio::SimpleAction::new("save", None);
    {
        let shell = shell.clone();
        save_action.connect_activate(move |_, _| save_note(&shell));
    }
    window.add_action(&save_action);

    let open_action = gio::SimpleAction::new("open", None);
    {
        let shell = shell.clone();
        open_action.connect_activate(move |_, _| open_note(&shell));
    }
    window.add_action(&open_action);

    app.set_accels_for_action("win.open", &["<Control>o"]);
    app.set_accels_for_action("win.save", &["<Control>s"]);

    {
        let shell = shell.clone();
        buffer.connect_changed(move |_| {
            shell.dirty.set(true);
            let name = shell.title.label();
            if !name.starts_with('*') {
                shell.title.set_label(&format!("*{name}"));
            }
        });
    }

    window.present();
}

fn build_header(shell: &Shell) -> gtk::HeaderBar {
    let open_button = gtk::Button::builder()
        .icon_name("document-open-symbolic")
        .tooltip_text("Open note (Ctrl+O)")
        .build();
    {
        let shell = shell.clone();
        open_button.connect_clicked(move |_| open_note(&shell));
    }

    let save_button = gtk::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Save note (Ctrl+S)")
        .build();
    save_button.add_css_class("suggested-action");
    {
        let shell = shell.clone();
        save_button.connect_clicked(move |_| save_note(&shell));
    }

    let header = gtk::HeaderBar::builder()
        .title_widget(&shell.title)
        .build();
    header.pack_start(&open_button);
    header.pack_end(&save_button);
    header
}

fn build_sidebar() -> gtk::Widget {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 8);
    sidebar.set_width_request(240);
    sidebar.set_margin_top(12);
    sidebar.set_margin_bottom(12);
    sidebar.set_margin_start(12);
    sidebar.set_margin_end(12);

    let heading = gtk::Label::builder().label("Notes").xalign(0.0).build();
    heading.add_css_class("heading");
    sidebar.append(&heading);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    for name in ["Welcome", "Sample note", "Reading list"] {
        let row = gtk::Label::builder().label(name).xalign(0.0).build();
        row.set_margin_top(6);
        row.set_margin_bottom(6);
        row.set_margin_start(8);
        row.set_margin_end(8);
        list.append(&row);
    }

    let list_scroller = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .has_frame(true)
        .build();
    sidebar.append(&list_scroller);

    let placeholder = gtk::Label::builder()
        .label("Placeholder list. M1 replaces it with the notebook file tree.")
        .wrap(true)
        .xalign(0.0)
        .build();
    placeholder.add_css_class("dim-label");
    sidebar.append(&placeholder);

    sidebar.upcast()
}

fn build_editor(buffer: &gtk::TextBuffer) -> gtk::Widget {
    let editor = gtk::TextView::builder()
        .buffer(buffer)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Word)
        .left_margin(24)
        .right_margin(24)
        .top_margin(16)
        .bottom_margin(16)
        .build();
    editor.set_vexpand(true);
    editor.set_hexpand(true);

    let scroller = gtk::ScrolledWindow::builder()
        .child(&editor)
        .vexpand(true)
        .hexpand(true)
        .build();
    scroller.upcast()
}

fn open_note(shell: &Shell) {
    let dialog = gtk::FileDialog::builder()
        .title("Open note")
        .modal(true)
        .build();

    let filters = gio::ListStore::new::<gtk::FileFilter>();
    let markdown = gtk::FileFilter::new();
    markdown.set_name(Some("Markdown notes"));
    markdown.add_suffix("md");
    markdown.add_suffix("markdown");
    filters.append(&markdown);
    dialog.set_filters(Some(&filters));
    dialog.set_default_filter(Some(&markdown));

    let shell = shell.clone();
    let window = shell.window.clone();
    dialog.open(
        Some(&window),
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(file) => load_file(&shell, file),
            Err(error) => {
                if !error.matches(gio::IOErrorEnum::Cancelled) {
                    show_error(
                        &shell.window,
                        "Could not open note",
                        &format!("The file chooser failed: {}", error.message()),
                    );
                }
            }
        },
    );
}

fn load_file(shell: &Shell, file: gio::File) {
    let shell = shell.clone();
    let source = file.clone();
    file.load_contents_async(None::<&gio::Cancellable>, move |result| match result {
        Ok((bytes, _etag)) => match String::from_utf8(bytes.to_vec()) {
            Ok(text) => {
                shell.buffer.set_text(&text);
                shell.dirty.set(false);
                let name = file_name(&source);
                shell.title.set_label(&name);
                shell.status.set_label(&format!("Opened {name}"));
                shell.window.set_title(Some(&format!("{name} — lean4losophy spike")));
                *shell.current_file.borrow_mut() = Some(source);
            }
            Err(_) => show_error(
                &shell.window,
                "Could not open note",
                "The selected file is not valid UTF-8. lean4losophy stores notes as UTF-8.",
            ),
        },
        Err(error) => show_error(
            &shell.window,
            "Could not open note",
            &format!("Reading the file failed: {}", error.message()),
        ),
    });
}

fn save_note(shell: &Shell) {
    let existing = shell.current_file.borrow().clone();
    if let Some(file) = existing {
        write_file(shell, file);
        return;
    }

    let dialog = gtk::FileDialog::builder()
        .title("Save note")
        .modal(true)
        .initial_name("untitled.md")
        .build();

    let shell = shell.clone();
    let window = shell.window.clone();
    dialog.save(
        Some(&window),
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(file) => {
                *shell.current_file.borrow_mut() = Some(file.clone());
                write_file(&shell, file);
            }
            Err(error) => {
                if !error.matches(gio::IOErrorEnum::Cancelled) {
                    show_error(
                        &shell.window,
                        "Could not save note",
                        &format!("The file chooser failed: {}", error.message()),
                    );
                }
            }
        },
    );
}

fn write_file(shell: &Shell, file: gio::File) {
    let text = shell
        .buffer
        .text(&shell.buffer.start_iter(), &shell.buffer.end_iter(), true);
    let bytes = text.as_bytes().to_vec();
    let name = file_name(&file);

    let shell = shell.clone();
    file.replace_contents_async(
        bytes,
        None,
        false,
        gio::FileCreateFlags::REPLACE_DESTINATION,
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok((_file, _etag)) => {
                shell.dirty.set(false);
                shell.title.set_label(&name);
                shell.status.set_label(&format!("Saved {name}"));
                shell
                    .window
                    .set_title(Some(&format!("{name} — lean4losophy spike")));
            }
            Err((_bytes, error)) => show_error(
                &shell.window,
                "Could not save note",
                &format!("Writing the file failed: {}", error.message()),
            ),
        },
    );
}

fn file_name(file: &gio::File) -> String {
    match file.path() {
        Some(path) => path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
        None => file.uri().to_string(),
    }
}

fn show_error(window: &gtk::ApplicationWindow, message: &str, detail: &str) {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(message)
        .detail(detail)
        .build();
    dialog.show(Some(window));
}
