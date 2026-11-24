# APORIA-5002 — A `FORMAT.md` change needs a version and migration note

**Severity:** error
**Tool:** review, `scripts/lint` format check, golden fixtures
**Scope:** `format/FORMAT.md` and any parser, writer, migration, or template that depends on it.

## Why

`FORMAT.md` is the contract for a notebook the user owns. A silent change breaks files that already exist on disk, often weeks later: a hobbyist reopens a notebook and a note no longer resolves its links, or a paper's argument block loses its conclusion because the grammar moved. The spec freezes the argument grammar and requires explicit, tested migrations. Note front matter already carries `format: 1`; the cache carries `PRAGMA user_version`, so a reader can refuse or convert instead of guessing.

## Alternatives

- Bump the format version and write the migration:

```toml
format = 2
# migration 1 -> 2: block IDs move from trailing ^id to #^id
```

- Add a before/after golden fixture under `tests/fixtures/format/` and run it in CI.
- Keep unknown front-matter keys so an older reader can round-trip a newer note.
- Treat `.aporia/cache.sqlite` as rebuildable: bump `PRAGMA user_version` and rebuild.
- Record the change in the `FORMAT.md` changelog with the version and reader behaviour.

## Exceptions

- The format owner may allow a purely additive, optional key that old readers preserve, with no bump, if it is documented and tested.
- No exception for link syntax, block IDs, the argument grammar, note identity, file layout, or encoding.
- A migration that cannot be reversed needs an ADR and a backup step in the UI.

## Examples

Bad:

```markdown
<!-- FORMAT.md still says format: 1; the parser now expects #^id -->
Block IDs: trailing `^id` is replaced by `#^id`.
```

Good:

```markdown
## Format 2 (migration from 1)
- Block IDs: `^id` becomes `#^id` at the end of a block.
- Migration: rewrite on open, keep a `.bak` beside each note.
- Fixture: `tests/fixtures/format/v1-to-v2/`.
```
