// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 The lean4losophy Authors

//! The derived notebook index: links, backlinks, and full-text search
//! (`FORMAT.md` section 12).
//!
//! The cache is never the source of truth. It is a SQLite database under
//! `.aporia/cache.sqlite`, rebuildable at any time; deleting it loses no note.
//! Indexing is incremental on save and on external change, and a full rebuild
//! writes a temporary database and renames it into place.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{Error, Result};
use crate::links::{self, WikiLink};
use crate::note::Note;
use crate::notebook::Notebook;

/// The cache schema version. A different value forces a rebuild.
pub const SCHEMA_VERSION: i64 = 1;

/// The default number of search results.
pub const SEARCH_LIMIT: usize = 50;

const SCHEMA: &str = r#"
CREATE TABLE notes (
    note_row      INTEGER PRIMARY KEY,
    id            TEXT UNIQUE,
    path          TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    body_hash     BLOB NOT NULL,
    modified_ns   INTEGER NOT NULL,
    size_bytes    INTEGER NOT NULL
);

CREATE TABLE aliases (
    note_row INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    alias    TEXT NOT NULL,
    PRIMARY KEY (note_row, alias)
);

CREATE TABLE tags (
    note_row INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    tag      TEXT NOT NULL,
    PRIMARY KEY (note_row, tag)
);

CREATE TABLE headings (
    note_row INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    ordinal  INTEGER NOT NULL,
    level    INTEGER NOT NULL,
    text     TEXT NOT NULL,
    anchor   TEXT NOT NULL,
    line     INTEGER NOT NULL,
    PRIMARY KEY (note_row, ordinal)
);

CREATE TABLE blocks (
    note_row INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    block_id TEXT NOT NULL,
    kind     TEXT NOT NULL,
    line     INTEGER NOT NULL,
    PRIMARY KEY (note_row, block_id)
);

CREATE TABLE links (
    from_row    INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    line        INTEGER NOT NULL,
    column      INTEGER NOT NULL,
    raw         TEXT NOT NULL,
    target      TEXT NOT NULL,
    alias       TEXT,
    heading     TEXT,
    block_id    TEXT,
    to_row      INTEGER REFERENCES notes(note_row) ON DELETE SET NULL,
    resolution  TEXT NOT NULL
);

CREATE TABLE notes_external (
    note_row INTEGER PRIMARY KEY,
    title    TEXT NOT NULL,
    body     TEXT NOT NULL
);

CREATE VIRTUAL TABLE notes_fts USING fts5(
    title,
    body,
    content = 'notes_external',
    content_rowid = 'note_row'
);

CREATE INDEX links_from ON links(from_row);
CREATE INDEX links_to ON links(to_row);
CREATE INDEX tags_tag ON tags(tag);
CREATE INDEX headings_anchor ON headings(note_row, anchor);

PRAGMA user_version = 1;
"#;

/// A full-text search request.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// The words to search for. Empty lists notes by title.
    pub text: String,
    /// Restrict to one tag, case-insensitively.
    pub tag: Option<String>,
    /// Restrict to notes under this folder, relative to `notes/`.
    pub folder: Option<String>,
    /// Maximum results.
    pub limit: usize,
}

impl SearchQuery {
    /// A query with the default limit and no filters.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tag: None,
            folder: None,
            limit: SEARCH_LIMIT,
        }
    }
}

/// One search result.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// Notebook-relative note path.
    pub path: PathBuf,
    /// Display title.
    pub title: String,
    /// Snippet with `\u{1}` and `\u{2}` around matches.
    pub snippet: String,
    /// The raw `bm25` score (lower is a better match).
    pub score: f64,
}

/// An incoming link to a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink {
    /// The note that contains the link.
    pub from: PathBuf,
    /// Its display title.
    pub title: String,
    /// The 1-based body line of the link.
    pub line: u32,
    /// The link target as written.
    pub target: String,
    /// The source line, trimmed, for context.
    pub context: String,
}

/// Where a link pointed at open time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkResolution {
    /// One note, and the line of a heading or block when one was named.
    Resolved {
        /// Notebook-relative note path.
        path: PathBuf,
        /// The 1-based body line to scroll to.
        line: Option<u32>,
    },
    /// More than one note matched; the caller must report, never choose.
    Ambiguous {
        /// Every candidate.
        candidates: Vec<PathBuf>,
    },
    /// Nothing matched.
    Broken,
}

/// A link row read back for resolution: row id, source row, target, heading,
/// block id, and the current resolution label.
type LinkRow = (i64, i64, String, Option<String>, Option<String>, String);

/// A broken or ambiguous link, for the issues view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkProblem {
    /// The note that contains the link.
    pub note: PathBuf,
    /// Its display title.
    pub title: String,
    /// The 1-based body line.
    pub line: u32,
    /// The target as written.
    pub target: String,
    /// `broken` or `ambiguous`.
    pub resolution: String,
}

/// Counts for the index status panel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IndexCounts {
    /// Indexed notes.
    pub notes: usize,
    /// Stored links.
    pub links: usize,
    /// Links that matched nothing.
    pub broken: usize,
    /// Links that matched more than one note.
    pub ambiguous: usize,
}

/// An open notebook index.
pub struct Index {
    conn: Connection,
    root: PathBuf,
}

impl Index {
    /// Open the cache for a notebook, rebuilding it when missing or stale.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] when SQLite cannot open or build the cache, and
    /// the read errors documented on [`Notebook::list_notes`].
    pub fn open(notebook: &Notebook) -> Result<Self> {
        let cache = cache_path(notebook.root());
        if cache.exists()
            && let Ok(mut index) = Self::open_file(&cache, notebook.root())
        {
            index.sync(notebook)?;
            return Ok(index);
        }
        Self::rebuild(notebook)
    }

    /// Rebuild the cache from the notebook files.
    ///
    /// The new database is written to `cache.sqlite.tmp`, synced, and renamed
    /// over the old cache, so an interrupted rebuild leaves the old cache or
    /// none, never a half-written one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures and [`Error::Write`] when
    /// the cache file cannot be replaced.
    pub fn rebuild(notebook: &Notebook) -> Result<Self> {
        let directory = notebook.root().join(".aporia");
        std::fs::create_dir_all(&directory).map_err(|source| Error::Write {
            path: directory.clone(),
            source,
        })?;

        let target = directory.join("cache.sqlite");
        let temporary = directory.join("cache.sqlite.tmp");
        remove_if_present(&temporary)?;

        let conn = Connection::open(&temporary).map_err(sql)?;
        conn.execute_batch("PRAGMA journal_mode = DELETE; PRAGMA foreign_keys = ON;")
            .map_err(sql)?;
        conn.execute_batch(SCHEMA).map_err(sql)?;

        let mut index = Self {
            conn,
            root: notebook.root().to_path_buf(),
        };
        index.sync(notebook)?;
        drop(index);

        sync_path(&temporary)?;
        remove_if_present(&sidecar(&target, "-wal"))?;
        remove_if_present(&sidecar(&target, "-shm"))?;
        std::fs::rename(&temporary, &target).map_err(|source| Error::Write {
            path: target.clone(),
            source,
        })?;
        if let Ok(directory_handle) = std::fs::File::open(&directory) {
            let _ = directory_handle.sync_all();
        }

        let index = Self::open_file(&target, notebook.root())?;
        // The freshly built database is already current.
        index
            .conn
            .execute_batch("PRAGMA journal_mode = WAL;")
            .map_err(sql)?;
        Ok(index)
    }

    fn open_file(path: &Path, root: &Path) -> Result<Self> {
        let conn = Connection::open(path).map_err(sql)?;
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(sql)?;
        if version != SCHEMA_VERSION {
            return Err(Error::Index {
                message: format!("cache schema is {version}, expected {SCHEMA_VERSION}"),
            });
        }
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")
            .map_err(sql)?;
        Ok(Self {
            conn,
            root: root.to_path_buf(),
        })
    }

    /// Bring the cache in line with the files on disk.
    ///
    /// Unchanged files are skipped by modification time and size; a changed
    /// file is rehashed and skipped when its content is identical.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Read`] for unreadable notes, and [`Error::Index`] for
    /// SQLite failures.
    pub fn sync(&mut self, notebook: &Notebook) -> Result<()> {
        let notes = notebook.list_notes()?;
        let existing: Vec<(i64, String, i64, i64)> = {
            let mut statement = self
                .conn
                .prepare("SELECT note_row, path, modified_ns, size_bytes FROM notes")
                .map_err(sql)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .map_err(sql)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sql)?
        };
        let keys: Vec<String> = notes.iter().map(|relative| path_key(relative)).collect();

        // Remove rows for files that are gone before inserting anything new, so
        // a renamed note cannot collide with its old row on the unique id.
        for (row, path, _, _) in &existing {
            if !keys.contains(path) {
                self.remove_row(*row)?;
            }
        }

        let mut known: HashMap<String, (i64, i64, i64)> = existing
            .into_iter()
            .filter(|(_, path, _, _)| keys.contains(path))
            .map(|(row, path, modified, size)| (path, (row, modified, size)))
            .collect();

        for relative in notes {
            let key = path_key(&relative);
            let absolute = notebook.note_path(&relative)?;
            let metadata = std::fs::metadata(&absolute).map_err(|source| Error::Read {
                path: absolute.clone(),
                source,
            })?;
            let modified = modified_ns(&metadata);
            let size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);

            if let Some((row, old_modified, old_size)) = known.remove(&key) {
                if old_modified == modified && old_size == size {
                    continue;
                }

                let note = notebook.load_note(&relative)?;
                let hash = hash_of(&note)?;
                let stored: Option<Vec<u8>> = self
                    .conn
                    .query_row(
                        "SELECT body_hash FROM notes WHERE note_row = ?1",
                        [row],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(sql)?;
                if stored.as_deref() == Some(hash.as_slice()) {
                    self.conn
                        .execute(
                            "UPDATE notes SET modified_ns = ?1, size_bytes = ?2 WHERE note_row = ?3",
                            params![modified, size, row],
                        )
                        .map_err(sql)?;
                    continue;
                }
                self.upsert(&relative, &note, &hash)?;
            } else {
                let note = notebook.load_note(&relative)?;
                let hash = hash_of(&note)?;
                self.upsert(&relative, &note, &hash)?;
            }
        }

        for (_, (row, _, _)) in known {
            self.remove_row(row)?;
        }

        self.resolve_links()
    }

    /// Reindex one note after a save or an external change.
    ///
    /// # Errors
    ///
    /// Returns the errors from [`Notebook::load_note`] and SQLite failures.
    pub fn reindex_note(&mut self, notebook: &Notebook, relative: &Path) -> Result<()> {
        let note = notebook.load_note(relative)?;
        let hash = hash_of(&note)?;
        self.upsert(relative, &note, &hash)?;
        self.resolve_links()
    }

    /// Drop one note from the cache after a delete.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures.
    pub fn remove_note(&mut self, relative: &Path) -> Result<()> {
        let key = path_key(relative);
        let row: Option<i64> = self
            .conn
            .query_row(
                "SELECT note_row FROM notes WHERE path = ?1",
                [&key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql)?;
        if let Some(row) = row {
            self.remove_row(row)?;
        }
        self.resolve_links()
    }

    /// Resolve a link target as the preview and backlinks need it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures.
    pub fn resolve(
        &self,
        from: &Path,
        target: &str,
        heading: Option<&str>,
        block_id: Option<&str>,
    ) -> Result<LinkResolution> {
        let lookup = self.lookup()?;
        let Some(from_row) = lookup.rows_by_path.get(&path_key(from)).copied() else {
            return Ok(LinkResolution::Broken);
        };

        match resolve_target(&lookup, from_row, target) {
            Candidate::One(row) => {
                let line = match fragment_line(&lookup, row, heading, block_id) {
                    Fragment::None => None,
                    Fragment::Found(line) => Some(line),
                    Fragment::Missing => return Ok(LinkResolution::Broken),
                };
                let Some(path) = lookup.path_by_row.get(&row) else {
                    return Ok(LinkResolution::Broken);
                };
                Ok(LinkResolution::Resolved {
                    path: PathBuf::from(path),
                    line,
                })
            }
            Candidate::Many(rows) => Ok(LinkResolution::Ambiguous {
                candidates: rows
                    .iter()
                    .filter_map(|row| lookup.path_by_row.get(row))
                    .map(PathBuf::from)
                    .collect(),
            }),
            Candidate::None => Ok(LinkResolution::Broken),
        }
    }

    /// Incoming resolved links for a note.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures.
    pub fn backlinks(&self, relative: &Path) -> Result<Vec<Backlink>> {
        let lookup = self.lookup()?;
        let Some(row) = lookup.rows_by_path.get(&path_key(relative)).copied() else {
            return Ok(Vec::new());
        };

        let mut statement = self
            .conn
            .prepare(
                "SELECT n.path, n.title, l.line, l.target
                 FROM links l
                 JOIN notes n ON n.note_row = l.from_row
                 WHERE l.to_row = ?1 AND l.resolution = 'resolved'
                 ORDER BY n.title COLLATE NOCASE, l.line",
            )
            .map_err(sql)?;
        let rows = statement
            .query_map([row], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(sql)?;

        let mut backlinks = Vec::new();
        for row in rows {
            let (path, title, line, target) = row.map_err(sql)?;
            let line = u32::try_from(line).unwrap_or(u32::MAX);
            let context = self.body_line(Path::new(&path), line).unwrap_or_default();
            backlinks.push(Backlink {
                from: PathBuf::from(path),
                title,
                line,
                target,
                context,
            });
        }

        Ok(backlinks)
    }

    /// Search the notebook.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures.
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>> {
        let tag = query
            .tag
            .as_deref()
            .map(str::trim)
            .filter(|tag| !tag.is_empty())
            .map(|tag| tag.trim_start_matches('#').to_owned());
        let folder = query
            .folder
            .as_deref()
            .map(str::trim)
            .filter(|folder| !folder.is_empty())
            .map(folder_like);
        let limit = i64::try_from(query.limit).unwrap_or(i64::from(i32::MAX));

        let text = query.text.trim();
        if text.is_empty() {
            let mut statement = self
                .conn
                .prepare(
                    "SELECT n.path, n.title
                     FROM notes n
                     WHERE (?1 IS NULL OR EXISTS (
                         SELECT 1 FROM tags t
                         WHERE t.note_row = n.note_row AND t.tag = ?1 COLLATE NOCASE))
                       AND (?2 IS NULL OR n.path LIKE ?2 ESCAPE '\\')
                     ORDER BY n.title COLLATE NOCASE
                     LIMIT ?3",
                )
                .map_err(sql)?;
            let rows = statement
                .query_map(params![tag, folder, limit], |row| {
                    Ok(SearchHit {
                        path: PathBuf::from(row.get::<_, String>(0)?),
                        title: row.get(1)?,
                        snippet: String::new(),
                        score: 0.0,
                    })
                })
                .map_err(sql)?;
            return rows
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sql);
        }

        let matches = fts_match(text);
        let mut statement = self
            .conn
            .prepare(
                "SELECT n.path, n.title,
                        snippet(notes_fts, 1, char(1), char(2), '…', 12),
                        bm25(notes_fts, 10.0, 1.0)
                 FROM notes_fts
                 JOIN notes n ON n.note_row = notes_fts.rowid
                 WHERE notes_fts MATCH ?1
                   AND (?2 IS NULL OR EXISTS (
                       SELECT 1 FROM tags t
                       WHERE t.note_row = n.note_row AND t.tag = ?2 COLLATE NOCASE))
                   AND (?3 IS NULL OR n.path LIKE ?3 ESCAPE '\\')
                 ORDER BY bm25(notes_fts, 10.0, 1.0)
                 LIMIT ?4",
            )
            .map_err(sql)?;
        let rows = statement
            .query_map(params![matches, tag, folder, limit], |row| {
                Ok(SearchHit {
                    path: PathBuf::from(row.get::<_, String>(0)?),
                    title: row.get(1)?,
                    snippet: row.get(2)?,
                    score: row.get(3)?,
                })
            })
            .map_err(sql)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sql)
    }

    /// Counts for the index status panel.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures.
    pub fn counts(&self) -> Result<IndexCounts> {
        let notes: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .map_err(sql)?;
        let links: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))
            .map_err(sql)?;
        let broken: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM links WHERE resolution = 'broken'",
                [],
                |row| row.get(0),
            )
            .map_err(sql)?;
        let ambiguous: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM links WHERE resolution = 'ambiguous'",
                [],
                |row| row.get(0),
            )
            .map_err(sql)?;

        Ok(IndexCounts {
            notes: usize::try_from(notes).unwrap_or(0),
            links: usize::try_from(links).unwrap_or(0),
            broken: usize::try_from(broken).unwrap_or(0),
            ambiguous: usize::try_from(ambiguous).unwrap_or(0),
        })
    }

    /// Broken and ambiguous links across the notebook.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Index`] for SQLite failures.
    pub fn problems(&self) -> Result<Vec<LinkProblem>> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT n.path, n.title, l.line, l.target, l.resolution
                 FROM links l
                 JOIN notes n ON n.note_row = l.from_row
                 WHERE l.resolution IN ('broken', 'ambiguous')
                 ORDER BY n.title COLLATE NOCASE, l.line
                 LIMIT 500",
            )
            .map_err(sql)?;
        let rows = statement
            .query_map([], |row| {
                Ok(LinkProblem {
                    note: PathBuf::from(row.get::<_, String>(0)?),
                    title: row.get(1)?,
                    line: u32::try_from(row.get::<_, i64>(2)?).unwrap_or(u32::MAX),
                    target: row.get(3)?,
                    resolution: row.get(4)?,
                })
            })
            .map_err(sql)?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sql)
    }

    /// The absolute cache path.
    #[must_use]
    pub fn cache_path(&self) -> PathBuf {
        cache_path(&self.root)
    }

    /// Rename a note, first rewriting incoming links that resolved by path.
    ///
    /// Links resolved by title or alias are not rewritten (`FORMAT.md` section
    /// 5.5). Returns the notes whose body changed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Write`] when the target exists or the move fails, and
    /// the errors from note reads and saves.
    pub fn rename_note(
        &mut self,
        notebook: &Notebook,
        from: &Path,
        to: &Path,
    ) -> Result<Vec<PathBuf>> {
        let target = notebook.note_path(to)?;
        if target.exists() {
            return Err(Error::Write {
                path: target,
                source: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "a note with this name already exists",
                ),
            });
        }

        let old_key = path_key(from);
        let new_key = path_key(to);
        let from_row: Option<i64> = self
            .conn
            .query_row(
                "SELECT note_row FROM notes WHERE path = ?1",
                [&old_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql)?;

        let mut rewritten = Vec::new();
        if let Some(from_row) = from_row {
            let sources: Vec<String> = {
                let mut statement = self
                    .conn
                    .prepare(
                        "SELECT DISTINCT n.path
                         FROM links l
                         JOIN notes n ON n.note_row = l.from_row
                         WHERE l.to_row = ?1 AND l.resolution = 'resolved'
                         ORDER BY n.path",
                    )
                    .map_err(sql)?;
                let rows = statement
                    .query_map([from_row], |row| row.get::<_, String>(0))
                    .map_err(sql)?;
                rows.collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(sql)?
            };

            for source in sources {
                let relative = PathBuf::from(&source);
                let absolute = notebook.note_path(&relative)?;
                let mut note = Note::load(&absolute)?;
                let (body, changed) = rewrite_path_links(note.body(), &old_key, &new_key);
                if changed == 0 {
                    continue;
                }
                note.set_body(body);
                note.touch();
                note.save()?;
                rewritten.push(relative);
            }
        }

        notebook.rename_note(from, to)?;
        self.sync(notebook)?;
        Ok(rewritten)
    }

    fn lookup(&self) -> Result<Lookup> {
        let mut lookup = Lookup::default();

        {
            let mut statement = self
                .conn
                .prepare("SELECT note_row, id, path, title FROM notes")
                .map_err(sql)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(sql)?;
            for row in rows {
                let (row_id, note_id, path, title) = row.map_err(sql)?;
                if let Some(note_id) = note_id {
                    lookup.ids.insert(note_id, row_id);
                }
                lookup
                    .titles
                    .entry(links::normalize_key(&title))
                    .or_default()
                    .push(row_id);
                lookup.paths.entry(path.clone()).or_default().push(row_id);
                lookup
                    .paths_lower
                    .entry(path.to_lowercase())
                    .or_default()
                    .push(row_id);
                lookup.path_by_row.insert(row_id, path.clone());
                lookup.rows_by_path.insert(path, row_id);
            }
        }

        {
            let mut statement = self
                .conn
                .prepare("SELECT note_row, alias FROM aliases")
                .map_err(sql)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(sql)?;
            for row in rows {
                let (row_id, alias) = row.map_err(sql)?;
                lookup
                    .aliases
                    .entry(links::normalize_key(&alias))
                    .or_default()
                    .push(row_id);
            }
        }

        {
            let mut statement = self
                .conn
                .prepare("SELECT note_row, text, anchor, line FROM headings")
                .map_err(sql)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .map_err(sql)?;
            for row in rows {
                let (row_id, text, anchor, line) = row.map_err(sql)?;
                let line = u32::try_from(line).unwrap_or(u32::MAX);
                lookup.heading_anchors.insert((row_id, anchor), line);
                lookup
                    .heading_text
                    .insert((row_id, links::normalize_key(&text)), line);
            }
        }

        {
            let mut statement = self
                .conn
                .prepare("SELECT note_row, block_id, line FROM blocks")
                .map_err(sql)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })
                .map_err(sql)?;
            for row in rows {
                let (row_id, block_id, line) = row.map_err(sql)?;
                lookup
                    .blocks
                    .insert((row_id, block_id), u32::try_from(line).unwrap_or(u32::MAX));
            }
        }

        Ok(lookup)
    }

    fn resolve_links(&mut self) -> Result<()> {
        let lookup = self.lookup()?;
        let rows: Vec<LinkRow> = {
            let mut statement = self
                .conn
                .prepare("SELECT rowid, from_row, target, heading, block_id, resolution FROM links")
                .map_err(sql)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })
                .map_err(sql)?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sql)?
        };

        for (row_id, from_row, target, heading, block_id, resolution) in rows {
            if resolution == "malformed" {
                continue;
            }

            let (to_row, label) = match resolve_target(&lookup, from_row, &target) {
                Candidate::One(row) => {
                    match fragment_line(&lookup, row, heading.as_deref(), block_id.as_deref()) {
                        Fragment::Missing => (None, "broken"),
                        _ => (Some(row), "resolved"),
                    }
                }
                Candidate::Many(_) => (None, "ambiguous"),
                Candidate::None => (None, "broken"),
            };

            self.conn
                .execute(
                    "UPDATE links SET to_row = ?1, resolution = ?2 WHERE rowid = ?3",
                    params![to_row, label, row_id],
                )
                .map_err(sql)?;
        }

        Ok(())
    }

    fn upsert(&mut self, relative: &Path, note: &Note, hash: &[u8]) -> Result<()> {
        let path = path_key(relative);
        let absolute = self.root.join(relative);
        let metadata = std::fs::metadata(&absolute).map_err(|source| Error::Read {
            path: absolute,
            source,
        })?;
        let modified = modified_ns(&metadata);
        let size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);

        let parsed = links::parse(note.body());
        let title = note.title();
        let id = note.front_matter().id();
        let aliases = note.front_matter().aliases();
        let tags = note.tags();
        let body = note.body().to_owned();

        let tx = self.conn.transaction().map_err(sql)?;

        let existing: Option<i64> = tx
            .query_row(
                "SELECT note_row FROM notes WHERE path = ?1",
                [&path],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql)?;

        let row_id = match existing {
            Some(row_id) => {
                tx.execute(
                    "UPDATE notes
                     SET id = ?1, path = ?2, title = ?3, body_hash = ?4,
                         modified_ns = ?5, size_bytes = ?6
                     WHERE note_row = ?7",
                    params![id, path, title, hash, modified, size, row_id],
                )
                .map_err(sql)?;
                row_id
            }
            None => {
                tx.execute(
                    "INSERT INTO notes (id, path, title, body_hash, modified_ns, size_bytes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, path, title, hash, modified, size],
                )
                .map_err(sql)?;
                tx.last_insert_rowid()
            }
        };

        let old_external: Option<(String, String)> = tx
            .query_row(
                "SELECT title, body FROM notes_external WHERE note_row = ?1",
                [row_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql)?;

        tx.execute("DELETE FROM aliases WHERE note_row = ?1", [row_id])
            .map_err(sql)?;
        tx.execute("DELETE FROM tags WHERE note_row = ?1", [row_id])
            .map_err(sql)?;
        tx.execute("DELETE FROM headings WHERE note_row = ?1", [row_id])
            .map_err(sql)?;
        tx.execute("DELETE FROM blocks WHERE note_row = ?1", [row_id])
            .map_err(sql)?;
        tx.execute("DELETE FROM links WHERE from_row = ?1", [row_id])
            .map_err(sql)?;
        tx.execute("DELETE FROM notes_external WHERE note_row = ?1", [row_id])
            .map_err(sql)?;

        if let Some((old_title, old_body)) = old_external {
            tx.execute(
                "INSERT INTO notes_fts (notes_fts, rowid, title, body)
                 VALUES ('delete', ?1, ?2, ?3)",
                params![row_id, old_title, old_body],
            )
            .map_err(sql)?;
        }

        for alias in &aliases {
            tx.execute(
                "INSERT OR IGNORE INTO aliases (note_row, alias) VALUES (?1, ?2)",
                params![row_id, alias],
            )
            .map_err(sql)?;
        }
        for tag in &tags {
            tx.execute(
                "INSERT OR IGNORE INTO tags (note_row, tag) VALUES (?1, ?2)",
                params![row_id, tag],
            )
            .map_err(sql)?;
        }
        for (ordinal, heading) in parsed.headings.iter().enumerate() {
            tx.execute(
                "INSERT INTO headings (note_row, ordinal, level, text, anchor, line)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    row_id,
                    i64::try_from(ordinal).unwrap_or(i64::MAX),
                    i64::from(heading.level),
                    heading.text,
                    heading.anchor,
                    i64::from(heading.line)
                ],
            )
            .map_err(sql)?;
        }
        for block in &parsed.blocks {
            tx.execute(
                "INSERT OR IGNORE INTO blocks (note_row, block_id, kind, line)
                 VALUES (?1, ?2, ?3, ?4)",
                params![row_id, block.id, block.kind.as_str(), i64::from(block.line)],
            )
            .map_err(sql)?;
        }
        for link in &parsed.links {
            tx.execute(
                "INSERT INTO links
                     (from_row, line, column, raw, target, alias, heading, block_id,
                      to_row, resolution)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9)",
                params![
                    row_id,
                    i64::from(link.line),
                    i64::from(link.column),
                    link.inside(),
                    link.target,
                    link.alias,
                    link.heading,
                    link.block_id,
                    if link.malformed {
                        "malformed"
                    } else {
                        "broken"
                    }
                ],
            )
            .map_err(sql)?;
        }

        tx.execute(
            "INSERT INTO notes_external (note_row, title, body) VALUES (?1, ?2, ?3)",
            params![row_id, title, body],
        )
        .map_err(sql)?;
        tx.execute(
            "INSERT INTO notes_fts (rowid, title, body) VALUES (?1, ?2, ?3)",
            params![row_id, title, body],
        )
        .map_err(sql)?;

        tx.commit().map_err(sql)
    }

    fn remove_row(&mut self, row_id: i64) -> Result<()> {
        let old_external: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT title, body FROM notes_external WHERE note_row = ?1",
                [row_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql)?;

        if let Some((old_title, old_body)) = old_external {
            self.conn
                .execute(
                    "INSERT INTO notes_fts (notes_fts, rowid, title, body)
                     VALUES ('delete', ?1, ?2, ?3)",
                    params![row_id, old_title, old_body],
                )
                .map_err(sql)?;
        }
        self.conn
            .execute("DELETE FROM notes_external WHERE note_row = ?1", [row_id])
            .map_err(sql)?;
        self.conn
            .execute("DELETE FROM notes WHERE note_row = ?1", [row_id])
            .map_err(sql)?;
        Ok(())
    }

    fn body_line(&self, relative: &Path, line: u32) -> Option<String> {
        let note = Note::load(&self.root.join(relative)).ok()?;
        let index = usize::try_from(line).ok()?.checked_sub(1)?;
        note.body()
            .lines()
            .nth(index)
            .map(|text| text.trim().to_owned())
    }
}

#[derive(Default)]
struct Lookup {
    ids: HashMap<String, i64>,
    titles: HashMap<String, Vec<i64>>,
    aliases: HashMap<String, Vec<i64>>,
    paths: HashMap<String, Vec<i64>>,
    paths_lower: HashMap<String, Vec<i64>>,
    path_by_row: HashMap<i64, String>,
    rows_by_path: HashMap<String, i64>,
    heading_anchors: HashMap<(i64, String), u32>,
    heading_text: HashMap<(i64, String), u32>,
    blocks: HashMap<(i64, String), u32>,
}

enum Candidate {
    One(i64),
    Many(Vec<i64>),
    None,
}

enum Fragment {
    None,
    Found(u32),
    Missing,
}

fn resolve_target(lookup: &Lookup, from_row: i64, target: &str) -> Candidate {
    let target = target.trim();
    if target.is_empty() {
        return Candidate::One(from_row);
    }

    if links::is_ulid(target)
        && let Some(row) = lookup.ids.get(target)
    {
        return Candidate::One(*row);
    }

    let key = links::normalize_key(target);

    if let Some(rows) = lookup.titles.get(&key) {
        if rows.len() == 1 {
            return Candidate::One(rows[0]);
        }
        if rows.len() > 1 {
            return Candidate::Many(rows.clone());
        }
    }

    if let Some(rows) = lookup.aliases.get(&key) {
        if rows.len() == 1 {
            return Candidate::One(rows[0]);
        }
        if rows.len() > 1 {
            return Candidate::Many(rows.clone());
        }
    }

    let variants = path_variants(target);
    let mut found = Vec::new();
    for variant in &variants {
        if let Some(rows) = lookup.paths.get(variant) {
            for row in rows {
                if !found.contains(row) {
                    found.push(*row);
                }
            }
        }
    }
    if found.len() == 1 {
        return Candidate::One(found[0]);
    }
    if found.len() > 1 {
        return Candidate::Many(found);
    }

    let mut folded = Vec::new();
    for variant in &variants {
        if let Some(rows) = lookup.paths_lower.get(&variant.to_lowercase()) {
            for row in rows {
                if !folded.contains(row) {
                    folded.push(*row);
                }
            }
        }
    }
    if folded.is_empty() {
        Candidate::None
    } else {
        // `FORMAT.md` section 1: a case-insensitive path match is ambiguous,
        // never resolved silently.
        Candidate::Many(folded)
    }
}

fn fragment_line(
    lookup: &Lookup,
    row: i64,
    heading: Option<&str>,
    block_id: Option<&str>,
) -> Fragment {
    if let Some(block_id) = block_id {
        return match lookup.blocks.get(&(row, block_id.to_owned())) {
            Some(line) => Fragment::Found(*line),
            None => Fragment::Missing,
        };
    }

    if let Some(heading) = heading {
        let key = (row, links::normalize_anchor(heading));
        if let Some(line) = lookup.heading_anchors.get(&key) {
            return Fragment::Found(*line);
        }
        let key = (row, links::normalize_key(heading));
        if let Some(line) = lookup.heading_text.get(&key) {
            return Fragment::Found(*line);
        }
        return Fragment::Missing;
    }

    Fragment::None
}

fn path_variants(target: &str) -> Vec<String> {
    let target = normalize_relative(target);
    let mut variants = Vec::new();
    push_unique(&mut variants, target.clone());
    if !target.ends_with(".md") {
        push_unique(&mut variants, format!("{target}.md"));
    }
    if !target.starts_with("notes/") {
        push_unique(&mut variants, format!("notes/{target}"));
        if !target.ends_with(".md") {
            push_unique(&mut variants, format!("notes/{target}.md"));
        }
    }
    variants
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn normalize_relative(value: &str) -> String {
    value.trim().trim_start_matches("./").replace('\\', "/")
}

fn folder_like(folder: &str) -> String {
    let folder = folder.trim().trim_matches('/');
    let folder = folder.strip_prefix("notes/").unwrap_or(folder);
    let escaped = folder
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("notes/{escaped}/%")
}

fn fts_match(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            let escaped = token.replace('"', "\"\"");
            format!("\"{escaped}\"*")
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn rewrite_path_links(body: &str, old_key: &str, new_key: &str) -> (String, usize) {
    let parsed = links::parse(body);
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();

    for link in &parsed.links {
        if link.malformed || !target_matches_path(&link.target, old_key) {
            continue;
        }
        let original = format!("[[{}]]", link.inside());
        if body.get(link.span.clone()) != Some(original.as_str()) {
            continue;
        }
        edits.push((
            link.span.clone(),
            format!("[[{}]]", rewritten_link(link, old_key, new_key)),
        ));
    }

    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0.start));

    let mut rewritten = body.to_owned();
    for (span, replacement) in &edits {
        rewritten.replace_range(span.clone(), replacement);
    }
    (rewritten, edits.len())
}

fn rewritten_link(link: &WikiLink, old_key: &str, new_key: &str) -> String {
    let mut inside = rewritten_target(&link.target, old_key, new_key);
    if let Some(heading) = &link.heading {
        inside.push('#');
        inside.push_str(heading);
    }
    if let Some(block_id) = &link.block_id {
        inside.push_str("#^");
        inside.push_str(block_id);
    }
    if let Some(alias) = &link.alias {
        inside.push('|');
        inside.push_str(alias);
    }
    inside
}

fn target_matches_path(target: &str, old_key: &str) -> bool {
    let target = normalize_relative(target);
    let old = normalize_relative(old_key);
    let old_no_ext = old.strip_suffix(".md").unwrap_or(&old);
    let old_notes = old.strip_prefix("notes/").unwrap_or(&old);
    let old_notes_no_ext = old_notes.strip_suffix(".md").unwrap_or(old_notes);

    target == old || target == old_no_ext || target == old_notes || target == old_notes_no_ext
}

fn rewritten_target(target: &str, old_key: &str, new_key: &str) -> String {
    let target = normalize_relative(target);
    let old = normalize_relative(old_key);
    let new = normalize_relative(new_key);
    let old_no_ext = old.strip_suffix(".md").unwrap_or(&old);

    let root_relative = target == old || target == old_no_ext;
    let mut result = if root_relative {
        new.clone()
    } else {
        new.strip_prefix("notes/").unwrap_or(&new).to_owned()
    };

    if target.ends_with(".md") {
        if !result.ends_with(".md") {
            result.push_str(".md");
        }
    } else if let Some(stripped) = result.strip_suffix(".md") {
        result = stripped.to_owned();
    }

    result
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(".aporia").join("cache.sqlite")
}

fn path_key(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn modified_ns(metadata: &std::fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn hash_of(note: &Note) -> Result<Vec<u8>> {
    let rendered = note.render()?;
    Ok(blake3::hash(rendered.as_bytes()).as_bytes().to_vec())
}

fn sidecar(target: &Path, suffix: &str) -> PathBuf {
    let mut name = target.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Write {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn sync_path(path: &Path) -> Result<()> {
    let file = std::fs::File::open(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

fn sql(error: rusqlite::Error) -> Error {
    Error::Index {
        message: error.to_string(),
    }
}
