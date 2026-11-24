# APORIA-1001 — No `unsafe` outside the FFI crate

**Severity:** error
**Tool:** `clippy::undocumented_unsafe_blocks`, `ast-grep` rule `unsafe-outside-ffi`, review
**Scope:** all crates; only the FFI shim may contain `unsafe`, and only with an ADR.

## Why

`unsafe` moves the burden of proof from the compiler to the author. The boundary here is real: GTK owns GObject refcounts, Lean owns its runtime objects, and the editor, preview, and Lean server exchange strings and buffers. A use-after-free can crash the app mid-save, corrupt the link index, or show a wrong diagnostic as if it were checked. Undefined behaviour is hard to reproduce, so a crash report may never be fixable.

## Alternatives

- Put the raw call in the FFI crate and expose a safe function; callers use ordinary types.
- Convert strings at the boundary with `CStr`/`CString` and return an owned `String`.
- Prefer `Arc`, `Rc`, `Pin`, and owned handles over raw pointers.
- If unavoidable: isolate it in the FFI crate, add an ADR, and a `// SAFETY:` comment naming the invariant, ownership, lifetime, and threading.

## Exceptions

- Only the FFI crate may carry `#[expect(unsafe_code, reason = "APORIA-1001: ...; ADR-000x")]`.
- Each block needs a `// SAFETY:` comment and security review by the owner.
- A whole-crate exception needs an ADR and is revisited at every milestone.
- No exception for `lean4losophy-core`, `lean4losophy-paper`, `lean4losophy-lean`, or the GTK app crate.

## Examples

Bad:

```rust
// lean4losophy-core: trust a buffer from the Lean runtime
let title = unsafe { std::str::from_utf8_unchecked(bytes) };
```

Good:

```rust
// ffi: one audited conversion behind a safe signature
pub fn lean_string_to_owned(ptr: *const c_char) -> Result<String, FfiError> {
    // SAFETY: `ptr` is a NUL-terminated Lean string valid for this call;
    // we copy it and never retain the pointer.
    let text = unsafe { CStr::from_ptr(ptr) };
    text.to_str().map(str::to_owned).map_err(|_| FfiError::NotUtf8)
}
```
