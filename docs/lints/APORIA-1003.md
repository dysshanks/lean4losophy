# APORIA-1003 — No `panic!`, `todo!`, or `unimplemented!`

**Severity:** error
**Tool:** `clippy::panic`, `clippy::todo`, `clippy::unimplemented`, `ast-grep` rule `panic-family`
**Scope:** all production code; test code is covered by the central exception.

## Why

These macros end the process instead of returning to the caller, so a note can be lost mid-edit and the user gets no message they can act on. Concretely:

- `todo!()` in the block parser fires when a hobbyist opens a note with an argument form the release does not handle.
- `unimplemented!()` in the Typst worker kills a paper export after the user has waited for it.
- `panic!()` on an unexpected Lean diagnostic takes down the window while a proof is being checked.

## Alternatives

- Return `Result<T, E>` with a typed error and let the UI show a message.
- Replace `todo!()` with a real implementation or an explicit `Unsupported` variant.
- Defer work with a linked issue as described in APORIA-2005, never with a panic.

## Exceptions

- Test code may panic; prefer `expect("why this cannot fail")` per APORIA-1002.
- A production exception needs owner review, an `#[expect(..., reason = "APORIA-1003: ...; issue #123")]`, and a removal plan.
- No exception for `todo!()` or `unimplemented!()` on a path reachable from a note, paper, or Lean block.

## Examples

Bad:

```rust
match block {
    Block::Argument(arg) => render_argument(arg)?,
    Block::Lean(_) => todo!(),
}
```

Good:

```rust
match block {
    Block::Argument(arg) => render_argument(arg)?,
    Block::Lean(block) => return Err(BlockError::Unsupported(block.kind())),
}
```
