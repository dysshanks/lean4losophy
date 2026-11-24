# APORIA-1002 — No `unwrap` or `expect` in production

**Severity:** error
**Tool:** `clippy::unwrap_used`, `clippy::expect_used`
**Scope:** all production code; tests are covered by the central exception below.

## Why

`unwrap()` and `expect()` turn a recoverable condition into a panic. A panic in the editor can lose unsaved notes, skip cleanup, and leave the user with no explanation. It also hides which error cases the code actually handles.

This is not about never failing. It is about failing with a typed error that the caller can report, retry, or recover from.

## Alternatives

Propagate with `?`:

```rust
let note = read_note(path)?;
```

Convert an option into a typed error:

```rust
let note = read_note(path)
    .ok_or_else(|| NoteError::NotFound(path.to_owned()))?;
```

Add context at the boundary:

```rust
let text = source
    .read_to_string()
    .map_err(|source| NoteError::Read { path: path.to_owned(), source })?;
```

If the value truly cannot be absent, encode that in the type instead of asserting it at runtime.

## Exceptions

- Test code may use `expect("why this cannot fail")`; the crate config allows it under `#[cfg(test)]`.
- A production exception needs a `#[expect(clippy::unwrap_used, reason = "APORIA-1002: ...; issue #123")]` and review.
- An exception is not allowed for user-controlled data, file contents, network responses, or database rows.

## Examples

Bad:

```rust
let config = std::fs::read_to_string(path).unwrap();
let value = map.get(key).unwrap();
```

Good:

```rust
let config = std::fs::read_to_string(path)
    .map_err(|source| ConfigError::Read { path: path.to_owned(), source })?;

let value = map
    .get(key)
    .ok_or(ConfigError::MissingKey(key.to_owned()))?;
```
