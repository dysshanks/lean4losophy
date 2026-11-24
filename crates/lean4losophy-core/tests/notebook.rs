// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Behaviour tests for the M1 core: parse/render round trips, atomic writes,
//! settings precedence, path safety, and notebook operations.

use std::path::Path;

use lean4losophy_core::{
    EffectiveSettings, GlobalConfig, Mode, Note, Notebook, NotebookSettings, PreviewLayout,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn notebook(dir: &TempDir) -> Result<Notebook, Box<dyn std::error::Error>> {
    Ok(Notebook::create(dir.path())?)
}

#[test]
fn note_round_trips_unknown_front_matter_keys() -> TestResult {
    let source =
        "---\nformat: 1\ntitle: A note\nmood: curious\ntags: [logic, reading]\n---\n\nBody text.\n";
    let note = Note::parse("note.md".into(), source)?;

    assert_eq!(note.title(), "A note");
    assert_eq!(note.tags(), vec!["logic", "reading"]);
    assert_eq!(note.body(), "\nBody text.\n");

    let rendered = note.render()?;
    assert!(rendered.starts_with("---\n"));
    assert!(rendered.contains("mood: curious"), "unknown key was lost");
    assert!(rendered.ends_with("Body text.\n"));

    let reparsed = Note::parse("note.md".into(), &rendered)?;
    assert_eq!(reparsed.body(), note.body());
    assert_eq!(reparsed.front_matter().id(), note.front_matter().id());
    Ok(())
}

#[test]
fn note_without_front_matter_uses_heading_and_stays_plain() -> TestResult {
    let source = "# Hand written\n\nNo front matter here.\n";
    let note = Note::parse("hand-written.md".into(), source)?;

    assert_eq!(note.title(), "Hand written");
    assert_eq!(note.format_version(), 1);
    assert_eq!(note.render()?, source);
    Ok(())
}

#[test]
fn title_falls_back_to_the_file_stem() -> TestResult {
    let note = Note::parse("reflective-equilibrium.md".into(), "Just text.\n")?;
    assert_eq!(note.title(), "reflective equilibrium");
    Ok(())
}

#[test]
fn malformed_front_matter_is_a_typed_error() {
    let error = Note::parse("note.md".into(), "---\ntitle: [unclosed\n---\nBody\n");
    assert!(error.is_err(), "invalid YAML must be reported");
}

#[test]
fn front_matter_must_be_a_mapping() {
    let error = Note::parse("note.md".into(), "---\n- a\n- b\n---\nBody\n");
    assert!(error.is_err(), "a YAML sequence is not front matter");
}

#[test]
fn save_is_atomic_and_leaves_no_temporary_file() -> TestResult {
    let dir = TempDir::new()?;
    let path = dir.path().join("note.md");
    let mut note = Note::new(path.clone(), "Atomic")?;
    note.set_body("# Atomic\n\nFirst version.\n".to_owned());
    note.save()?;

    assert_eq!(std::fs::read_to_string(&path)?, note.render()?);

    note.set_body("# Atomic\n\nSecond version.\n".to_owned());
    note.save()?;
    assert!(std::fs::read_to_string(&path)?.contains("Second version."));

    let leftovers = std::fs::read_dir(dir.path())?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
        .count();
    assert_eq!(leftovers, 0, "temporary file was left behind");
    Ok(())
}

#[test]
fn notebook_create_lists_notes_and_creates_unique_names() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = notebook(&dir)?;

    let first = notebook.create_note("A note")?;
    let second = notebook.create_note("A note")?;

    assert_ne!(first.path(), second.path());
    assert!(first.path().ends_with("notes/a-note.md"));
    assert!(second.path().ends_with("notes/a-note-2.md"));

    let listed = notebook.list_notes()?;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0], Path::new("notes/a-note-2.md"));
    assert_eq!(listed[1], Path::new("notes/a-note.md"));
    Ok(())
}

#[test]
fn notebook_ignores_non_markdown_and_hidden_directories() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = notebook(&dir)?;
    notebook.create_note("Visible")?;

    std::fs::write(notebook.notes_dir().join("notes.txt"), "no")?;
    std::fs::create_dir_all(notebook.notes_dir().join(".trash"))?;
    std::fs::write(
        notebook.notes_dir().join(".trash").join("hidden.md"),
        "# Hidden\n",
    )?;
    std::fs::create_dir_all(notebook.notes_dir().join("topics"))?;
    std::fs::write(
        notebook.notes_dir().join("topics").join("nested.md"),
        "# Nested\n",
    )?;

    let listed = notebook.list_notes()?;
    assert_eq!(
        listed,
        vec![
            Path::new("notes/topics/nested.md"),
            Path::new("notes/visible.md")
        ]
    );
    Ok(())
}

#[test]
fn parent_traversal_is_rejected() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = notebook(&dir)?;

    let escape = notebook.note_path(Path::new("../outside.md"));
    assert!(escape.is_err(), "traversal must be rejected");

    let absolute = notebook.note_path(Path::new("/etc/passwd.md"));
    assert!(absolute.is_err(), "absolute paths must be rejected");

    let wrong_extension = notebook.note_path(Path::new("notes/note.txt"));
    assert!(
        wrong_extension.is_err(),
        "non-markdown paths must be rejected"
    );
    Ok(())
}

#[test]
fn notebook_settings_override_global_settings() -> TestResult {
    let global = GlobalConfig {
        mode_default: Mode::Guided,
        ..GlobalConfig::default()
    };

    let notebook = NotebookSettings {
        mode: Some(Mode::Expert),
        preview_layout: Some(PreviewLayout::Toggle),
        autosave_ms: Some(750),
        ..NotebookSettings::default()
    };

    let effective = EffectiveSettings::resolve(&global, &notebook);
    assert_eq!(effective.mode, Mode::Expert);
    assert_eq!(effective.preview_layout, PreviewLayout::Toggle);
    assert_eq!(effective.autosave_ms, 750);

    let inherited = EffectiveSettings::resolve(&global, &NotebookSettings::default());
    assert_eq!(inherited.mode, Mode::Guided);
    assert_eq!(inherited.preview_layout, PreviewLayout::SideBySide);
    assert_eq!(inherited.autosave_ms, 400);
    Ok(())
}

#[test]
fn notebook_settings_round_trip_and_preserve_unknown_keys() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = notebook(&dir)?;

    let path = notebook.root().join(".aporia").join("settings.toml");
    let original = "version = 1\nmode = \"expert\"\nfuture_setting = 42\n";
    std::fs::write(&path, original)?;

    let mut settings = lean4losophy_core::settings::load_notebook(notebook.root())?;
    assert_eq!(settings.mode, Some(Mode::Expert));
    settings.autosave_ms = Some(600);
    lean4losophy_core::settings::save_notebook(notebook.root(), &settings)?;

    let written = std::fs::read_to_string(&path)?;
    assert!(written.contains("mode = \"expert\""));
    assert!(written.contains("autosave_ms = 600"));
    assert!(
        written.contains("future_setting = 42"),
        "unknown key was lost"
    );
    Ok(())
}

#[test]
fn rename_moves_the_note_and_refuses_collisions() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = notebook(&dir)?;
    notebook.create_note("First")?;
    notebook.create_note("Second")?;

    let moved = notebook.rename_note(
        Path::new("notes/first.md"),
        Path::new("notes/topics/first-renamed.md"),
    )?;
    assert!(moved.exists(), "renamed note is missing");
    assert!(!notebook.note_path(Path::new("notes/first.md"))?.exists());

    let collision = notebook.rename_note(
        Path::new("notes/second.md"),
        Path::new("notes/topics/first-renamed.md"),
    );
    assert!(
        collision.is_err(),
        "an existing note must not be overwritten"
    );
    Ok(())
}

#[test]
fn create_then_open_finds_the_same_settings() -> TestResult {
    let dir = TempDir::new()?;
    {
        let mut notebook = notebook(&dir)?;
        let mut settings = notebook.settings().clone();
        settings.mode = Some(Mode::Expert);
        notebook.update_settings(settings)?;
    }

    let reopened = Notebook::open(dir.path())?;
    assert_eq!(reopened.settings().mode, Some(Mode::Expert));
    Ok(())
}

#[test]
fn opening_a_plain_folder_is_reported_not_panicking() -> TestResult {
    let dir = TempDir::new()?;
    let result = Notebook::open(dir.path());
    assert!(result.is_err(), "a folder without notes/ is not a notebook");
    Ok(())
}

#[test]
fn sample_notes_are_created_once() -> TestResult {
    let dir = TempDir::new()?;
    let notebook = notebook(&dir)?;
    notebook.ensure_sample_notes()?;
    let first = notebook.list_notes()?;
    assert_eq!(first.len(), 2);

    notebook.ensure_sample_notes()?;
    assert_eq!(notebook.list_notes()?.len(), 2, "samples were duplicated");
    Ok(())
}
