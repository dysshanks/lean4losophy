# ADR-0004 — Data format, identity, settings, and concurrency

- **Status:** Accepted
- **Date:** 2026-11-24
- **Gate:** G3 (notes shell complete)
- **Related:** ADR-0002, ADR-0003

## Context

A notebook is a folder the user owns. It must open in any editor, diff cleanly, work with Git, and survive the app being deleted. That rules out a database as the source of truth and rules out opaque binary state.

The open questions settle the shape:

- Q9: hybrid note identity.
- Q10: settings split between the app config directory and the notebook.
- Q11: last writer wins for external edits and multiple instances.
- Q12: the on-disk format is documented in a separate `FORMAT.md`.

The format is also a compatibility contract. Once notes exist, a silent change breaks them.

## Decision

### `FORMAT.md` is normative

`FORMAT.md` (in `format/`) is the source of truth for the bytes on disk. It defines:

- notebook layout: `notes/`, `refs/`, `papers/`, `lean/`, `.aporia/`;
- note front matter and unknown-key preservation;
- note identity and link resolution;
- link forms, block IDs, and heading anchors;
- the argument grammar;
- settings files and precedence;
- the cache schema and rebuild rules;
- write procedure and encoding;
- format version and migrations.

This ADR records the choices and points at `FORMAT.md`. Where the two disagree, `FORMAT.md` wins and this ADR is corrected. `SPEC.md` section 7 is a summary only.

### Hybrid identity

- Every note may carry a stable ULID in front matter: `id: 01J8Z9...`.
- Resolution order is: stable ID, then title, then alias, then path.
- A note without an ID is addressed by path; the app adds an ID on first save.
- Renames update incoming links where the target has an ID. Without an ID, a rename can break links, and the app reports the broken link rather than guessing.
- Ambiguous matches (two notes with the same title or alias) are reported, never resolved silently.
- IDs are never reused and never derived from content.

### Split settings

- **Global:** `config.toml` under the XDG config directory. Holds UI mode default, update consent, recent notebooks, window state, and other machine-local preferences.
- **Notebook:** `.aporia/settings.toml`. Holds Guided/Expert mode, citation style, template defaults, and preferences that should travel with the folder and be diffable.
- Notebook settings win over global settings for the same key. The precedence is documented in `FORMAT.md`.
- No secrets are stored in either file; the app has no accounts and no tokens.
- A missing or invalid settings file falls back to defaults and is reported, never silently rewritten on read.

### Concurrency

- **Last writer wins.** No file locking and no merge UI in v1.
- Every save is atomic: write a temp file in the same directory, fsync, rename over the target. A crash mid-save leaves the previous note intact.
- A watcher detects external changes. A clean buffer reloads silently. A dirty buffer keeps the user's text and shows a non-blocking warning with the choice to reload or overwrite.
- Multiple app instances are allowed; the same last-writer-wins rule applies.
- Optional local history is post-v1. It must not be required for correctness.

### Versioning

- Notes carry `format: 1` in front matter.
- A format change bumps the number, ships a migration, and adds a test fixture. This is `APORIA-5002`.
- Migrations are explicit, forward-only, and idempotent where possible. A notebook from a newer format opens read-only with a clear message instead of being rewritten.
- Unknown front-matter keys are preserved on read/write so other tools can add metadata.
- The SQLite cache at `.aporia/cache.sqlite` is disposable. It carries `PRAGMA user_version`; a missing, stale, or corrupt cache triggers a rebuild from the files.
- Encoding is UTF-8 with LF line endings. Invalid UTF-8 is rejected with a clear error, not replaced.

## Consequences

Positive:

- The notebook is portable, diffable, and recoverable with ordinary tools.
- Stable IDs make renames and links robust; path fallback keeps hand-written files working.
- Splitting settings keeps machine-local state out of Git and notebook state with the notebook.
- Atomic writes plus last-writer-wins is simple enough to implement correctly and test.
- A disposable cache means no migration risk for derived data.

Negative:

- Last-writer-wins can lose a concurrent edit. This is accepted for v1 and mitigated by atomic writes, the external-change warning, and optional history later.
- Hybrid identity has two resolution paths and therefore more tests and more error messages.
- Two settings locations can confuse users; the UI must show which scope a setting belongs to.
- Preserving unknown keys means the writer cannot simply serialize a fixed struct.
- Path fallback links can break when a file is renamed outside the app.

Neutral and follow-ups:

- `FORMAT.md` v0.1 is a P0 deliverable; this ADR is not a substitute for it.
- Golden fixtures cover front matter, links, block IDs, arguments, settings precedence, migrations, and invalid input.
- The parser is fuzzed; malformed input produces a typed error, not a panic.

## Alternatives considered

### Path-only identity

Simplest, but every rename breaks links and Git moves are invisible. Rejected by Q9.

### Stable IDs only

Robust, but a hand-written note without front matter would be invisible until the app touched it. Rejected; hybrid keeps plain files first-class.

### Database as the source of truth

Fast queries, but it breaks the plain-file promise and makes recovery depend on the app. Rejected; SQLite is a cache only.

### File locking or a merge UI

Would reduce lost edits, but adds cross-platform locking, stale-lock recovery, and a merge interface that is out of v1 scope. Rejected by Q11; revisit only if real data loss appears.

### All settings in the notebook

Makes every notebook carry machine-specific state and pollutes Git. Rejected by Q10.

## References

- `SPEC.md` sections 7, 16.2 (E5), 17.3.
- `IMPLEMENTATION-PLAN.md` sections 15 (Format work) and 20 (open decisions).
- `OPEN-QUESTIONS.md` Q9, Q10, Q11, Q12, Q17.
- `FORMAT.md` for the normative definition.
- `APORIA-5002`, `APORIA-6003`.
