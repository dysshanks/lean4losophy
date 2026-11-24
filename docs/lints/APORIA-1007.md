# APORIA-1007 — No `process::exit` outside `main`

**Severity:** error
**Tool:** `clippy::exit`
**Scope:** all production code; `main` returns an exit code instead of exiting.
**Status:** enabled

## Why

`std::process::exit` ends the process immediately. It skips destructors, so
buffers are not flushed, temporary files are not removed, and a pending atomic
rename may never complete. From the user's side that is a silent data loss: the
window disappears and the note they were typing is gone with no message.

It also makes code untestable. A function that can exit the process cannot be
called from a test, so the error path is never exercised.

## What it catches

Any call to `std::process::exit`, including `process::exit(0)` on a "success"
path. Returning an error to `main` and letting `main` choose the exit code is
the only sanctioned way to end the program early.

## Alternatives

Return a `Result` and let `main` map it to an exit code:

```rust
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "lean4losophy stopped");
            std::process::ExitCode::FAILURE
        }
    }
}
```

For a CLI command that must stop, return the error from the command function and
let the caller decide whether to continue with the next command.

## Exceptions

- None for production code. An exception needs a `#[expect(clippy::exit,
  reason = "APORIA-1007: ...; issue #123")]` and review, and is expected only in
  a test harness that must simulate a hard exit.
- Test code may use the process exit of a spawned child to assert exit codes;
  this rule applies to the library, not to the child process under test.

## Examples

Bad:

```rust
fn save(note: &Note) -> Result<(), SaveError> {
    write_atomic(note)?;
    std::process::exit(0);
}
```

Good:

```rust
fn save(note: &Note) -> Result<(), SaveError> {
    write_atomic(note)?;
    Ok(())
}
```

## Enforcement

- `./scripts/lint` runs Clippy with `clippy::exit` denied in
  `[workspace.lints]`.
- A finding names this code and points back here; there is no bad/good fixture
  because the compiler enforces the rule directly.
- Related: `APORIA-1003` (panics) and `APORIA-1002` (unwrap and expect) cover the
  other ways a process can abort in the middle of a write.
