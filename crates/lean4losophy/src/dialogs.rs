// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Small modal dialogs: a text prompt and a destructive confirmation.
//!
//! Buttons use specific verbs and destructive actions are styled and separated
//! (`DESIGN-PLAN.md` section 5).

use gtk::prelude::*;

/// Ask for a single line of text.
pub fn prompt_text(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    field_label: &str,
    initial: &str,
    accept_label: &str,
    on_accept: impl Fn(String) + 'static,
) {
    let dialog = gtk::Window::builder()
        .title(title)
        .modal(true)
        .default_width(420)
        .build();
    dialog.set_transient_for(Some(parent));

    let entry = gtk::Entry::builder()
        .text(initial)
        .activates_default(true)
        .hexpand(true)
        .build();

    let cancel = gtk::Button::with_label("Cancel");
    let accept = gtk::Button::with_label(accept_label);
    accept.add_css_class("suggested-action");

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    buttons.append(&cancel);
    buttons.append(&accept);

    // A persistent visible label, not just a placeholder (DESIGN-PLAN.md rule 10).
    let label = gtk::Label::builder().label(field_label).xalign(0.0).build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&label);
    content.append(&entry);
    content.append(&buttons);
    dialog.set_child(Some(&content));

    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| dialog.close());
    }
    {
        let dialog = dialog.clone();
        let entry_for_read = entry.clone();
        let on_accept = std::rc::Rc::new(on_accept);
        let activate = std::rc::Rc::new(move || {
            let text = entry_for_read.text().to_string();
            if !text.trim().is_empty() {
                on_accept(text);
                dialog.close();
            }
        });
        {
            let activate = std::rc::Rc::clone(&activate);
            accept.connect_clicked(move |_| activate());
        }
        entry.connect_activate(move |_| activate());
    }

    dialog.present();
    entry.grab_focus();
}

/// Confirm an irreversible action.
///
/// `cancel_label` names the safe outcome ("Keep note", "Don't create"), so the
/// pair of buttons reads as a choice rather than a generic OK/Cancel.
pub fn confirm(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    body: &str,
    cancel_label: &str,
    accept_label: &str,
    on_accept: impl Fn() + 'static,
) {
    let dialog = gtk::Window::builder()
        .title(title)
        .modal(true)
        .default_width(440)
        .build();
    dialog.set_transient_for(Some(parent));

    let heading = gtk::Label::builder()
        .label(title)
        .xalign(0.0)
        .wrap(true)
        .build();
    heading.add_css_class("title-3");

    let detail = gtk::Label::builder()
        .label(body)
        .xalign(0.0)
        .wrap(true)
        .build();
    detail.add_css_class("dim-label");

    let cancel = gtk::Button::with_label(cancel_label);
    let accept = gtk::Button::with_label(accept_label);
    accept.add_css_class("destructive-action");

    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    // Destructive action is separated from the safe action.
    buttons.append(&cancel);
    buttons.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    buttons.append(&accept);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(16);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&heading);
    content.append(&detail);
    content.append(&buttons);
    dialog.set_child(Some(&content));

    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| dialog.close());
    }
    {
        let dialog = dialog.clone();
        let on_accept = std::rc::Rc::new(on_accept);
        accept.connect_clicked(move |_| {
            on_accept();
            dialog.close();
        });
    }

    dialog.present();
}
