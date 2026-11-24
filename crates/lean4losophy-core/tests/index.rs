// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! Links, backlinks, resolution, and search (`FORMAT.md` sections 5 and 12).

use std::error::Error;
use std::path::{Path, PathBuf};

use lean4losophy_core::index::{Index, LinkResolution, SearchQuery};
use lean4losophy_core::paths::relative_to;
use lean4losophy_core::{Notebook, slugify};
use tempfile::TempDir;

type TestResult = std::result::Result<(), Box<dyn Error>>;

fn note_in(notebook: &Notebook, title: &str) -> std::result::Result<PathBuf, Box<dyn Error>> {
    let note = notebook.create_note(title)?;
    Ok(relative_to(notebook.root(), note.path())?)
}

fn fixture() -> std::result::Result<(TempDir, Notebook), Box<dyn Error>> {
    let directory = TempDir::new()?;
    let notebook = Notebook::create(&directory.path().join("notebook"))?;

    let mut plato = notebook.create_note("Plato")?;
    plato
        .front_matter_mut()
        .set_tags(&[String::from("philosophy")]);
    plato
        .front_matter_mut()
        .set_aliases(&[String::from("the teacher")]);
    plato.set_body(
        "# Plato\n\n## The Cave\n\nAllegory and shadows. ^cave-1\n\nA second cave paragraph.\n"
            .to_owned(),
    );
    plato.save()?;

    let mut republic = notebook.create_note("Republic")?;
    republic
        .front_matter_mut()
        .set_tags(&[String::from("politics")]);
    republic.set_body(
        "# Republic\n\nSee [[Plato]] and [[Plato#The Cave]] and [[Plato#^cave-1]].\n".to_owned(),
    );
    republic.save()?;

    let mut other = notebook.create_note("Other")?;
    other.set_body("Unrelated text about justice.\n".to_owned());
    other.save()?;

    Ok((directory, notebook))
}

fn plato_path() -> PathBuf {
    PathBuf::from(format!("notes/{}.md", slugify("Plato")))
}

#[test]
fn resolves_id_title_alias_path_heading_and_block() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let index = Index::open(&notebook)?;
    let plato = plato_path();

    let by_title = index.resolve(Path::new("notes/republic.md"), "Plato", None, None)?;
    assert_eq!(
        by_title,
        LinkResolution::Resolved {
            path: plato.clone(),
            line: None
        }
    );

    let mut alias = index.resolve(Path::new("notes/republic.md"), "the teacher", None, None)?;
    assert!(matches!(alias, LinkResolution::Resolved { .. }));
    if let LinkResolution::Resolved { path, .. } = &mut alias {
        assert_eq!(path, &plato);
    }

    let note = notebook.load_note(&plato)?;
    let id = note.front_matter().id().ok_or("note has no id")?;
    let by_id = index.resolve(Path::new("notes/republic.md"), &id, None, None)?;
    assert!(matches!(by_id, LinkResolution::Resolved { .. }));

    let by_path = index.resolve(Path::new("notes/republic.md"), "notes/plato.md", None, None)?;
    assert_eq!(
        by_path,
        LinkResolution::Resolved {
            path: plato.clone(),
            line: None
        }
    );

    let heading = index.resolve(
        Path::new("notes/republic.md"),
        "Plato",
        Some("The Cave"),
        None,
    )?;
    assert!(matches!(
        heading,
        LinkResolution::Resolved { line: Some(3), .. }
    ));

    let block = index.resolve(
        Path::new("notes/republic.md"),
        "Plato",
        None,
        Some("cave-1"),
    )?;
    assert!(matches!(
        block,
        LinkResolution::Resolved { line: Some(5), .. }
    ));

    let missing_heading = index.resolve(
        Path::new("notes/republic.md"),
        "Plato",
        Some("No such heading"),
        None,
    )?;
    assert_eq!(missing_heading, LinkResolution::Broken);

    Ok(())
}

#[test]
fn reports_broken_and_ambiguous_links() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let note = notebook.create_note("Twin")?;
    let mut twin = note;
    twin.front_matter_mut().set_title("Same");
    twin.save()?;
    let mut second = notebook.create_note("Same")?;
    second.front_matter_mut().set_title("Same");
    second.save()?;

    let mut source = notebook.create_note("Source")?;
    source.set_body("[[Missing note]] and [[Same]].\n".to_owned());
    source.save()?;

    let mut index = Index::open(&notebook)?;
    index.sync(&notebook)?;

    let broken = index.resolve(Path::new("notes/source.md"), "Missing note", None, None)?;
    assert_eq!(broken, LinkResolution::Broken);

    let ambiguous = index.resolve(Path::new("notes/source.md"), "Same", None, None)?;
    match ambiguous {
        LinkResolution::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
        other => return Err(format!("expected ambiguous, got {other:?}").into()),
    }

    let counts = index.counts()?;
    assert!(counts.broken >= 1);
    assert!(counts.ambiguous >= 1);

    let problems = index.problems()?;
    assert!(
        problems
            .iter()
            .any(|problem| problem.target == "Missing note")
    );
    assert!(problems.iter().any(|problem| problem.target == "Same"));

    Ok(())
}

#[test]
fn backlinks_include_context_and_line() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let index = Index::open(&notebook)?;
    let backlinks = index.backlinks(&plato_path())?;

    assert_eq!(backlinks.len(), 3);
    assert!(backlinks.iter().all(|link| link.title == "Republic"));
    assert!(
        backlinks
            .iter()
            .any(|link| link.context.contains("[[Plato]]"))
    );
    assert!(backlinks.iter().all(|link| link.line == 3));

    Ok(())
}

#[test]
fn search_ranks_filters_and_snippets() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let index = Index::open(&notebook)?;

    let hits = index.search(&SearchQuery::new("allegory"))?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Plato");
    assert!(hits[0].snippet.to_lowercase().contains("allegory"));

    let mut tagged = SearchQuery::new("");
    tagged.tag = Some(String::from("philosophy"));
    let tagged_hits = index.search(&tagged)?;
    assert_eq!(tagged_hits.len(), 1);
    assert_eq!(tagged_hits[0].title, "Plato");

    let no_match = index.search(&SearchQuery::new("nonesuchword"))?;
    assert!(no_match.is_empty());

    // FTS operators in user input must not become syntax errors.
    let quoted = index.search(&SearchQuery::new("\"allegory\" AND"))?;
    assert!(quoted.is_empty() || quoted.iter().all(|hit| hit.title == "Plato"));

    Ok(())
}

#[test]
fn folder_filter_matches_nested_notes() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let mut republic = notebook.load_note(Path::new("notes/republic.md"))?;
    republic.set_body("See [[Plato]].\n".to_owned());
    republic.save()?;
    notebook.rename_note(
        Path::new("notes/republic.md"),
        Path::new("notes/lectures/republic.md"),
    )?;

    let index = Index::open(&notebook)?;
    let mut query = SearchQuery::new("");
    query.folder = Some(String::from("lectures"));
    let hits = index.search(&query)?;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, PathBuf::from("notes/lectures/republic.md"));

    let backlinks = index.backlinks(&plato_path())?;
    assert_eq!(backlinks.len(), 1);
    assert_eq!(
        backlinks[0].from,
        PathBuf::from("notes/lectures/republic.md")
    );

    Ok(())
}

#[test]
fn rename_rewrites_path_links_only() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let source = note_in(&notebook, "Source")?;
    let mut source_note = notebook.load_note(&source)?;
    source_note.set_body(
        "# Source\n\nPath [[notes/other]] and title [[Other]] and alias [[the teacher]].\n"
            .to_owned(),
    );
    source_note.save()?;

    let mut index = Index::open(&notebook)?;
    let rewritten = index.rename_note(
        &notebook,
        Path::new("notes/other.md"),
        Path::new("notes/renamed.md"),
    )?;

    assert_eq!(rewritten, vec![source.clone()]);
    let body = notebook.load_note(&source)?.body().to_owned();
    assert!(body.contains("[[notes/renamed]]"), "body: {body}");
    assert!(body.contains("[[Other]]"), "body: {body}");
    assert!(body.contains("[[the teacher]]"), "body: {body}");
    assert!(!body.contains("[[notes/other]]"));

    // The title link still resolves after the file moves.
    let resolution = index.resolve(&source, "Other", None, None)?;
    assert_eq!(
        resolution,
        LinkResolution::Resolved {
            path: PathBuf::from("notes/renamed.md"),
            line: None
        }
    );

    Ok(())
}

#[test]
fn cache_is_rebuildable_without_data_loss() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let index = Index::open(&notebook)?;
    let counts = index.counts()?;
    assert_eq!(counts.notes, 3);
    drop(index);

    std::fs::remove_file(notebook.root().join(".aporia/cache.sqlite"))?;
    let rebuilt = Index::open(&notebook)?;
    assert_eq!(rebuilt.counts()?.notes, 3);
    assert_eq!(rebuilt.search(&SearchQuery::new("allegory"))?.len(), 1);

    Ok(())
}

#[test]
fn incremental_sync_notices_external_edits_and_deletes() -> TestResult {
    let (_directory, notebook) = fixture()?;
    let mut index = Index::open(&notebook)?;

    let other = Path::new("notes/other.md");
    let mut note = notebook.load_note(other)?;
    note.set_body("A corrected sentence about equity.\n".to_owned());
    note.save()?;

    notebook.delete_note(Path::new("notes/republic.md"))?;
    index.sync(&notebook)?;

    assert_eq!(index.search(&SearchQuery::new("equity"))?.len(), 1);
    assert_eq!(index.counts()?.notes, 2);
    assert!(index.backlinks(&plato_path())?.is_empty());

    Ok(())
}
