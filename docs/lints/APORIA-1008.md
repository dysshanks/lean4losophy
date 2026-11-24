# APORIA-1008 — No `println!` or `dbg!` for logging

**Severity:** error
**Tool:** `clippy::print_stdout`, `clippy::dbg_macro`, `ast-grep` rule `stdout-logging`
**Scope:** all crates; the CLI output module is the only place allowed to write to stdout on purpose.

## Why

`println!` and `dbg!` bypass log levels, the log file, and the privacy rules. Notes are private drafts: a `dbg!(note)` in the save path writes a paragraph, a source quote, or a title into the terminal, and under Flatpak into the journal where it outlives the session. Users cannot turn it off, and support cannot ask for "debug logs" without also receiving note text.

## Alternatives

- Use `tracing` events with fields, never note bodies:

```rust
tracing::debug!(note_id = %note.id, bytes = text.len(), "saving note");
```

- Attach a span to a long operation so timing lands in the local log.
- Report errors through the UI error channel, not stdout.
- For intentional machine output, use the CLI output module in `lean4losophy-cli`.

## Exceptions

- No exception for logging note content, paths under a user's home, or Lean source.
- An intentional stdout path (for example `lean4losophy export --stdout`) needs owner approval, a reason, an issue, and a test asserting the exact bytes.
- Logs stay local; anything else also needs APORIA-3001 and APORIA-3002 review.

## Examples

Bad:

```rust
dbg!(&path);
println!("saving note {:?}: {}", note.title, text);
```

Good:

```rust
tracing::debug!(note_id = %note.id, path = %path.display(), "saving note");
```
