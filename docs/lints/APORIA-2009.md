# APORIA-2009 — No `String` for filesystem paths

**Severity:** error
**Tool:** `clippy::ptr_arg`, `ast-grep` rule `path-as-string`, review
**Scope:** all crates that touch the notebook, cache, Lean project, templates, or exports.

## Why

A notebook is a folder: `notes/*.md`, `refs/library.bib`, `lean/`, `.aporia/`, `papers/<name>/paper.md`. Paths built with `String` and `format!` break on real user data:

- A note titled with Greek, Sanskrit, or CJK characters goes through a lossy UTF-8 conversion and points at the wrong file.
- `format!("{root}/notes/{name}.md")` produces doubled separators and fails when the notebook path contains a space.
- Containment checks for APORIA-3004 need real path components; a string prefix check accepts `notes/../../etc`.
- `Path`/`PathBuf` keeps the platform's rules, including non-UTF-8 filenames, and makes joins explicit.

## Alternatives

- Take `&Path` and return `PathBuf`; join with `Path::join`.
- Store notebook-relative paths as validated `String` only in front matter, and convert at the boundary.
- Use `components()` for containment checks under the notebook root.
- Use `OsStr`/`OsString` when a name may not be UTF-8.
- Use `display()` or `to_string_lossy()` only when showing a path to the user.

## Exceptions

- Format fields that `FORMAT.md` defines as text (for example a link target) may be `String` until parsed; conversion must be fallible.
- An exception needs owner approval, a reason, and a test with a non-ASCII path.
- No exception for the notebook root, note lookup, cache paths, or Typst template paths.

## Examples

Bad:

```rust
fn note_path(root: String, title: String) -> String {
    format!("{root}/notes/{title}.md")
}
```

Good:

```rust
fn note_path(root: &Path, slug: &str) -> PathBuf {
    root.join("notes").join(format!("{slug}.md"))
}
```
