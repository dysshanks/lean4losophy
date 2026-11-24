# APORIA-1009 — No `#[allow(...)]`

**Severity:** error
**Tool:** `ast-grep` rule `allow-attribute`, suppression audit, review
**Scope:** all Rust crates; non-Rust files use `lint-ignore` with the same rules.

## Why

`#[allow(...)]` is invisible debt. It tells the next reader nothing about why the finding was acceptable, who agreed, or when it should disappear. It is also the shortest path for an agent or a rushed change to get past the gate, which is exactly the behaviour this policy exists to stop. A stale `allow` then hides a real defect: an ignored `unwrap_used` in the save path, a `dead_code` warning over an unlinked citation resolver, or a growing `clippy::too_many_arguments` on the Typst renderer.

## Alternatives

- Fix the finding. Most allows cover a small refactor: split the function, add a typed error, remove the unused branch.
- Use `#[expect(lint, reason = "APORIA-xxxx: ...; issue #123")]`, which also warns when the lint stops applying.
- Configure test-only allowances once at the crate root, not at each test.
- For non-Rust files, use `lint-ignore: APORIA-xxxx reason: ...` so the audit can find it.

## Exceptions

- `#[expect]` with a code, a reason, and a linked issue or ADR, approved by a reviewer.
- A whole-file or whole-crate exception needs an ADR.
- Suppressions appear in CI output and are reviewed at each milestone; expired ones are deleted.
- `#[allow]` is never approved, including inside macros and generated code.

## Examples

Bad:

```rust
#[allow(clippy::too_many_arguments)]
fn render_paper(ctx: &Context, meta: &Meta, body: &Body, ...) -> Typst { ... }
```

Good, if the signature is genuinely required:

```rust
#[expect(clippy::too_many_arguments, reason = "APORIA-1009: fixed Typst record; issue #88")]
fn render_paper(ctx: &Context, meta: &Meta, body: &Body, ...) -> Typst { ... }
```

Good, by fixing it:

```rust
fn render_paper(template: &PaperTemplate) -> Typst { ... }
```
