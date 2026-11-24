# FORMAT.md — Aporia notebook format

**Version:** 1 (draft, v0.1)
**Status:** Normative for the bytes on disk once `format: 1` is accepted.
**Related:** ADR-0004 (identity, settings, concurrency), ADR-0002 (Markdown dialect).
**Supersedes:** `SPEC.md` section 7 as a summary only. Where this document and any
other disagree, this document wins and the other is corrected.

The key words MUST, MUST NOT, SHOULD, and MAY are used as in RFC 2119.

A notebook is a directory the user owns. Every non-derived file in it is UTF-8
text that a human can read, diff, and edit in another editor. Nothing in a
notebook requires Aporia to be recoverable.

---

## 1. Notebook layout

```text
<notebook>/
├── notes/                 # note files; the only required directory
│   ├── some-note.md
│   └── a-folder/          # folders are optional and may nest
│       └── another.md
├── refs/
│   └── library.bib        # optional BibTeX database
├── papers/
│   └── <paper>/           # optional paper project, see section 9
│       ├── paper.md
│       └── assets/
├── lean/                  # optional Lake project, see section 10
│   ├── lean-toolchain
│   ├── lakefile.lean
│   └── Aporia/
└── .aporia/               # optional app state; safe to delete except settings
    ├── settings.toml      # notebook settings, see section 11
    └── cache.sqlite       # derived index, see section 12
```

Rules:

- `notes/` MUST exist for a directory to be opened as a notebook by the app.
  Creating a notebook creates it.
- Notes MUST be files ending in `.md` under `notes/`. Other extensions are
  ignored and never rewritten.
- Directories whose name starts with `.` under `notes/` are ignored.
- Symlinks are followed for reading only if their target resolves inside the
  notebook root. A link that escapes the root MUST be ignored and reported.
- The app MUST NOT write outside the notebook root except to the global config
  directory (section 11) and temporary files in the same directory as their
  target (section 13).
- File names are case-sensitive even on case-insensitive filesystems when
  resolving links; a case-insensitive match is reported as ambiguous, never
  resolved silently.

## 2. Encoding

- Files MUST be UTF-8 without a byte-order mark. Invalid UTF-8 MUST be rejected
  with a typed error that names the file. Bytes are never replaced.
- Line endings: LF (`\n`). On read, CRLF is accepted and normalized to LF. On
  write, the file is LF only.
- A file SHOULD end with exactly one trailing newline. The app writes one.
- NUL bytes are invalid and are treated as invalid UTF-8 input.
- Maximum note size the app must handle without truncation: 8 MiB.

## 3. Front matter

Front matter is an optional YAML mapping at the very start of a note.

```markdown
---
format: 1
id: 01J8Z9K3T7VQ2M8N4B6C0XYD5R
title: The mortality argument
aliases: [mortality, socrates argument]
tags: [logic, lecture-2]
created: 2026-11-20T09:14:00Z
updated: 2026-11-24T17:02:11Z
---

# The mortality argument

Body text starts here.
```

Rules:

- The opening delimiter MUST be the first line of the file: exactly `---`.
  The closing delimiter is a line that is exactly `---` or `...`.
- If the file does not begin with `---`, it has no front matter. A note without
  front matter is valid and is treated as `format: 1`.
- The block between the delimiters MUST parse as a YAML 1.2 mapping of string
  keys. Parse errors MUST be reported; the note opens read-only in that case
  and is never rewritten automatically.
- Keys are case-sensitive and matched exactly. Unknown keys MUST be preserved.
  Key order and YAML comments are not preserved on write; values are.
- Values MUST round-trip through the types below. A value of the wrong type is a
  reported error, not a silent coercion.

### 3.1 Known keys

| Key | Type | Required | Rules |
| --- | --- | --- | --- |
| `format` | integer | no | Format version. Absent means `1`. Values above 1 open read-only (section 14). |
| `id` | string | no | ULID, 26 characters, Crockford base32 `[0-9A-HJKMNP-TV-Z]`. Assigned on first save if absent; never changed after. |
| `title` | string | no | Single line, no leading/trailing whitespace. Empty is treated as absent. |
| `aliases` | list of strings | no | Single lines. Case-insensitive duplicates are an error. |
| `tags` | list of strings | no | Trimmed, no leading `#`, no internal whitespace. |
| `created` | string | no | RFC 3339 timestamp. Set on first save; never changed after. |
| `updated` | string | no | RFC 3339 timestamp in UTC. Refreshed on each save. |
| `lean_entry` | string | no | Path under `lean/` of the generated module for this note, if any. |

Lists MAY also be written as a single string; it is read as a one-element list.
A note whose front matter is a YAML sequence or scalar instead of a mapping is
invalid and opens read-only.

### 3.2 Title resolution

The display title is the first non-empty value of:

1. `title` in front matter;
2. the text of the first ATX heading level 1 (`# ...`) in the body, with inline
   Markdown formatting removed;
3. the file stem, with `-` and `_` replaced by spaces.

The title is display data only. It is not an identifier, and two notes MAY share
a title (section 5 handles the ambiguity).

## 4. Note identity

- `id` is a ULID: 48-bit millisecond timestamp followed by 80 bits of entropy,
  rendered in Crockford base32, uppercase, 26 characters.
- An ID MUST NOT be reused, derived from content, or renumbered.
- A note without an `id` is addressed by path. The app assigns an ID on the
  first save it performs, writing `format: 1`, `id`, `created`, and `updated`.
- Copies of a note file carry their ID unless the user duplicates the note in
  the app, which assigns a new one. Two files with the same ID are a reported
  conflict; the app never guesses which is canonical.
- Deleting a note does not free its ID.

## 5. Links

### 5.1 Syntax

A link is a span of the form:

```text
[[target]]
[[target|alias text]]
[[target#Heading]]
[[target#^block-id]]
```

| Part | Rules |
| --- | --- |
| `target` | Note ID, title, alias, or path. Trimmed. MUST NOT contain `[`, `]`, `\|`, `#`, or a newline. MAY be empty only in `[[#Heading]]`, which targets the current note. |
| `alias` | Display text. MUST NOT contain `]]` or a newline. MAY contain spaces, `#`, and `\|`. May be empty. |
| heading | Exact heading text after anchor normalization (section 5.4), or the raw heading text; both resolve. |
| block-id | The identifier of a block marked with `^id` (section 5.3), written `#^id`. |

### 5.2 Parsing rules

- A span is a link only in body text: not inside a fenced code block, an indented
  code block, an inline code span, a raw HTML block, or an autolink.
- `![[...]]` is not a link in v1 (transclusion is post-v1). It renders literally
  and produces no backlink.
- An unclosed `[[` is literal text, not an error.
- Nested links are not supported. `[[a [[b]]]]` parses as the outer text plus a
  separate link; parsers MUST treat the first `]]` as the end.
- A link with a malformed target (forbidden character, empty target without a
  heading) is reported as a malformed link and renders as literal text.
- Link extraction is done on the parsed AST, not with a text regex over the whole
  file, so code spans can never produce links.

### 5.3 Block IDs

A block ID marks a block so links can address it.

- It is written at the end of the block's last line as a space, a caret, and the
  ID: `... ^my-block`.
- Applies to paragraphs, list items, block quotes, tables, and Lean blocks.
- ID grammar: `[A-Za-z][A-Za-z0-9-]{0,63}`; case-sensitive.
- A block ID MUST be unique within its note. A duplicate is reported; the first
  occurrence wins for resolution and the rest are ignored.
- The ID is removed from the rendered text of the block.
- Trailing punctuation after the ID makes it part of the text, not an ID.

### 5.4 Heading anchors

Heading anchors are derived from heading text:

1. Trim whitespace and remove inline Markdown formatting (emphasis, code,
   links keep their text).
2. Unicode-normalize to NFC.
3. Lowercase.
4. Keep letters, digits, spaces, hyphens, and underscores; drop other characters.
5. Replace runs of spaces with a single hyphen.
6. Collapse runs of hyphens to one.
7. If the result is empty, use `section`.
8. Within a note, a repeated anchor gets `-1`, `-2`, ... in document order.

### 5.5 Resolution order

A link target resolves in this order, and the first successful step wins:

1. **ID.** If the target is 26 characters matching the ULID grammar, match `id`.
2. **Title.** Exact match after NFC normalization and case folding.
3. **Alias.** Exact match against any alias, after the same normalization.
4. **Path.** Relative to the notebook root, with or without `.md`, matching the
   normalized path separators.

Outcomes:

- **Resolved:** exactly one note matches.
- **Ambiguous:** more than one note matches at the winning step. The app MUST
  report every candidate and MUST NOT choose one. The link stays unchanged.
- **Broken:** nothing matches. The app offers to create a note; it MUST NOT
  create one without a user action.
- When a note has an ID, renames update incoming links that resolved by path.
  Links resolved by title or alias are not rewritten by the app, because the
  user may have meant different text.

## 6. Markdown dialect

- The body is CommonMark 0.31.2 plus exactly two extensions: tables and
  footnotes. See ADR-0002.
- Raw HTML is not passed to the preview. It is escaped and sanitized.
- The following are app conventions layered on Markdown and defined here:
  links (section 5), block IDs (section 5.3), tags (section 7), arguments
  (section 8), and fenced Lean blocks (section 10).
- No other custom syntax is part of format 1. Adding one is a format change
  (section 14).

## 7. Tags

- Tags live in front matter only. A `#word` in the body is not a tag in format 1;
  it is plain text. This avoids a collision with ATX headings and keeps the
  body portable.
- A tag is a single token without whitespace. `/` is allowed for nesting
  (`logic/modal`). `#` is not part of the stored value.
- Matching is case-insensitive; the first stored spelling is the display form.
- An empty `tags` list is valid.

## 8. Argument blocks

An argument is a Markdown list convention, not a new syntax. It must remain
readable as an ordinary list in any editor.

```markdown
- Argument: The mortality argument {#arg-mortality}
  1. All humans are mortal.
  2. Socrates is a human.
  3. ∴ Socrates is mortal.
```

### 8.1 Grammar

An **argument item** is a list item (ordered or unordered) whose first block is a
paragraph whose text, after trimming, begins with the exact word `Argument`.

- The marker is followed by `:` or the end of the paragraph. `Argument: Title`
  sets the title to the remainder, trimmed. An empty title is allowed.
- An optional argument ID may end the paragraph: `{#arg-<id>}` with
  `id` matching `[A-Za-z0-9-]{1,64}`. It is stored as a block ID on the item and
  is addressable with `[[note#^arg-id]]`.
- The argument item MUST contain exactly one nested list, immediately after the
  marker paragraph. That list is the body.
- Each body item's first block MUST be a non-empty paragraph. Deeper nesting is
  allowed and is preserved as text, not modeled.
- The **conclusion** is the body item whose paragraph begins with `∴`
  (U+2234). The remainder of that paragraph is the conclusion text.
- Every other body item is a **premise**, in document order. Premises are
  numbered from 1 in that order.
- A body list MAY be ordered or unordered; ordering in the source does not
  change premise order and is renumbered on insertion by the editor.
- Arguments do not nest. An argument item inside an argument body is an error.

### 8.2 Validation

Parsers MUST produce these diagnostics and MUST NOT crash on any input:

| Code | Condition | Effect |
| --- | --- | --- |
| `ARG001` | No body list | Argument is ignored; marker renders as text |
| `ARG002` | No conclusion | Argument is parsed with `conclusion: None`; card shows "no conclusion" |
| `ARG003` | More than one conclusion | The first is used; the note is flagged |
| `ARG004` | No premises | Argument is parsed with an empty premise list |
| `ARG005` | A premise or conclusion paragraph is empty after the marker | Reported; the item is kept as text |
| `ARG006` | Nested argument | The inner marker is treated as text |

The stored status of an argument is **not** part of the file. It is derived:
`unformalized` when no Lean block is associated, `formalized` when one is and it
contains `sorry`, `checked` when Lean reports no errors. See section 10.

### 8.3 Round-trip

Reading then writing an unchanged argument MUST reproduce the same Markdown,
modulo list renumbering when premises were added or removed. Golden fixtures
under `tests/fixtures/format/arguments/` are the conformance set.

## 9. Paper projects

```text
papers/<paper-slug>/
├── paper.md
└── assets/            # images and other local assets
```

- `<paper-slug>` matches `[a-z0-9][a-z0-9-]{0,63}`.
- `paper.md` uses the same front matter rules as a note, with these keys:

| Key | Type | Notes |
| --- | --- | --- |
| `format` | integer | Required, currently `1`. |
| `title` | string | Required for a valid paper. |
| `subtitle` | string | Optional. |
| `authors` | list of mappings | `name` required; `affiliation`, `email`, `orcid` optional. |
| `abstract` | string | Plain Markdown, no headings. |
| `keywords` | list of strings | Optional. |
| `language` | string | BCP 47, e.g. `en-GB`. Defaults to `en`. |
| `date` | string | `YYYY-MM-DD` or RFC 3339. Defaults to the build date. |
| `template` | string | `essay` or `article` in v1. |
| `citation_style` | string | CSL style name or a path to a `.csl` file under `papers/`. |
| `page_size` | string | `a4` (default) or `letter`. |

- Sections are ATX headings in `paper.md`. Section numbering is generated, not
  stored.
- An inserted note keeps a link back to its source note; the v1 format has no
  live transclusion syntax.
- Exports are written outside the notebook by default. Any export written into
  the notebook goes under `.aporia/export/` and is not indexed.

## 10. Lean project and blocks

The optional Lean project lives at `lean/` and is a normal Lake project.

- `lean/lean-toolchain` pins the exact toolchain, one line.
- `lean/lakefile.lean` declares the package and the `Aporia` library.
- `lean/Aporia/` holds the shipped library, copied into the notebook without
  overwriting files the user has edited.
- Generated note modules go under `lean/notes/` and are derived: the app may
  rewrite them freely. They are not the source of truth.

Lean blocks in notes:

````markdown
```lean {#lean-mortality}
theorem mortal (h : Human socrates) : Mortal socrates := by
  sorry
```
````

- A block is a Lean block when the fence info string starts with `lean`.
- After `lean`, an optional attribute `{#id}` gives the block a block ID under
  the grammar in section 5.3.
- The block's source is the fence content verbatim, with the leading indentation
  of the fence removed from each line.
- Association with an argument: a Lean block whose `#id` equals the argument's
  ID belongs to that argument. No other association is stored.
- Extracting a block writes it to `lean/notes/<note-id>/<block-id>.lean` with a
  line map back to the note; the line map is derived and cached, not stored in
  the note.

## 11. Settings

### 11.1 Global config

Location: `$XDG_CONFIG_HOME/aporia/config.toml` (`~/.config/aporia/config.toml`
when `XDG_CONFIG_HOME` is unset).

```toml
version = 1
mode_default = "guided"          # "guided" | "expert"
update_check = false             # off by default; consent in ADR-0003
editor_font_scale = 1.0
window_width = 1100
window_height = 720
recent_notebooks = [
  "/home/user/Documents/Philosophy",
]
```

- Missing file means defaults.
- Unknown keys are preserved.
- `recent_notebooks` is machine-local and MUST NOT be written to the notebook.

### 11.2 Notebook settings

Location: `.aporia/settings.toml`.

```toml
version = 1
mode = "expert"                  # "guided" | "expert"
preview_layout = "side-by-side"  # "side-by-side" | "toggle"
autosave_ms = 400
citation_style = "chicago-notes"
default_template = "essay"
paper_page_size = "a4"
```

- Missing file means defaults.
- Unknown keys are preserved so newer versions can add settings.
- The file is diffable and meant to travel with the notebook.

### 11.3 Precedence

1. Notebook setting, when the key exists.
2. Global setting, when the key exists.
3. Built-in default.

- The UI MUST show which scope a setting comes from when both exist.
- Reading never rewrites a file. A malformed settings file is reported, defaults
  are used, and the file is left untouched until the user saves settings.
- No secrets, tokens, or identifiers are stored in either file.

## 12. Cache

Location: `.aporia/cache.sqlite`. It is derived, disposable, and never the
source of truth.

- Created with `PRAGMA journal_mode = WAL`, `PRAGMA foreign_keys = ON`, and
  `PRAGMA user_version = 1`.
- A missing, unreadable, corrupt, or older cache triggers a full rebuild. The
  cache is replaced, never migrated in place.
- Deleting `.aporia/cache.sqlite` MUST lose no user data.
- The cache MUST NOT be committed; `.gitignore` covers `.aporia/` in this
  repository's sample notebooks.

Schema version 1:

```sql
CREATE TABLE notes (
    note_row      INTEGER PRIMARY KEY,
    id            TEXT UNIQUE,          -- ULID, NULL for path-only notes
    path          TEXT NOT NULL UNIQUE, -- relative to the notebook root, '/'
    title         TEXT NOT NULL,
    body_hash     BLOB NOT NULL,        -- BLAKE3 of the note bytes
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
    PRIMARY KEY (note_row, ordinal)
);

CREATE TABLE blocks (
    note_row INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    block_id TEXT NOT NULL,
    kind     TEXT NOT NULL,             -- paragraph, list-item, quote, table, lean, argument
    line     INTEGER NOT NULL,
    PRIMARY KEY (note_row, block_id)
);

CREATE TABLE links (
    from_row    INTEGER NOT NULL REFERENCES notes(note_row) ON DELETE CASCADE,
    line        INTEGER NOT NULL,
    column      INTEGER NOT NULL,
    raw         TEXT NOT NULL,          -- inside the brackets, as written
    target      TEXT NOT NULL,
    alias       TEXT,
    heading     TEXT,
    block_id    TEXT,
    to_row      INTEGER REFERENCES notes(note_row) ON DELETE SET NULL,
    resolution  TEXT NOT NULL           -- resolved, ambiguous, broken, malformed
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
```

- `notes_external` mirrors `notes` with the note body text and is kept in sync
  by the indexer, not by triggers, so a failed index does not corrupt notes.
- Indexing is incremental on save and on external change; the body hash decides
  whether a note needs reindexing.
- A rebuild scans `notes/`, replaces the database atomically (write to
  `cache.sqlite.tmp`, fsync, rename), and reports progress.
- Search uses `bm25()` ranking with column weights title > body.

## 13. Writing files

Every write of a note, settings file, or paper follows this procedure:

1. Serialize to bytes (UTF-8, LF, one trailing newline).
2. Create a temporary file in the same directory as the target, named
   `.<target-name>.tmp-<pid>-<random>`.
3. Write all bytes, then `fsync` the file.
4. Preserve the target's permission bits if it exists; otherwise use `0644`.
5. `rename` the temporary file over the target.
6. `fsync` the containing directory when the platform allows it.
7. On any failure, remove the temporary file and leave the target untouched.

- Writes are serialized per file. Two concurrent saves of the same note queue;
  the last one wins.
- Autosave is debounced: `autosave_ms` (default 400 ms) after the last edit, with
  a maximum delay of 5 s during continuous typing.
- A crash at any point leaves either the old complete file or the new complete
  file. A leftover `.tmp-` file is stale state and is deleted on the next open
  of that notebook, not treated as a note.
- Saving never rewrites a note it did not parse successfully (section 3).

## 14. Versioning and migrations

- `format` currently is `1`.
- A change that alters how existing bytes are interpreted MUST bump `format`,
  ship a migration, and add a fixture. This is `APORIA-5002`.
- Migrations are forward-only, explicit, and idempotent where possible. A
  migration runs only with user consent and writes a backup copy under
  `.aporia/backup/` first.
- A note or notebook with a higher `format` opens read-only with a clear message.
  It MUST NOT be rewritten.
- Unknown front-matter keys and unknown settings keys survive a read/write cycle,
  which is how minor additions stay backward compatible without a bump.

| Format | Change |
| --- | --- |
| 1 | Initial format: notes, links, block IDs, arguments, settings, cache |

## 15. Conformance

A parser conforms when it satisfies these fixtures and rules:

| Area | Fixture set |
| --- | --- |
| Front matter | `tests/fixtures/format/front-matter/` |
| Links and block IDs | `tests/fixtures/format/links/` |
| Argument grammar | `tests/fixtures/format/arguments/` |
| Lean blocks and extraction | `tests/fixtures/format/lean/` |
| Settings precedence | `tests/fixtures/format/settings/` |
| Invalid input and error codes | `tests/fixtures/format/invalid/` |

Required properties:

- Invalid input produces a typed error, never a panic (`APORIA-1003`).
- Parsing is idempotent: `parse(write(parse(x)))` equals `parse(x)` for every
  valid fixture.
- Link resolution is deterministic and never resolves an ambiguity silently.
- The parser is fuzzed; a fuzz crash is a bug, not a format question.

## 16. Changelog

| Version | Date | Change |
| --- | --- | --- |
| 1 (draft v0.1) | 2026-11-24 | First written specification; freezes the M0 decisions |
