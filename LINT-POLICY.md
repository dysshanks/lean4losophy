# Lint Policy

**Status:** Draft
**Applies to:** Rust, Lean, C/FFI, SQL, TOML/YAML, Markdown, shell, Nix, and CI configuration.
**Enforced by:** `scripts/lint`, CI, and review.

## 1. Purpose

The codebase must not accumulate warning-only debt. A bad practice is either fixed, or it is an approved exception with a written reason.

Every rule has:

- a stable `APORIA-xxxx` code;
- a page in `docs/lints/` explaining what it catches and why it matters;
- at least one alternative that a person or agent can apply;
- a defined exception process.

The goal is not to maximize the number of rules. The goal is that a failing message tells the reader what to do next.

## 2. Enforcement

- Lint findings are **errors**. Warnings fail CI.
- `scripts/lint` is the single entry point. CI runs the same script.
- The CI `lint` job is required. No `continue-on-error`.
- Formatting is part of the gate, not a separate preference.
- Dependencies, licenses, secrets, and advisories are part of the gate.
- A rule is not enabled until its docs page and a failing fixture exist.
- A suppression without a code and reason is itself an error.

## 3. Error code format

`APORIA-<category><three digits>`

| Range | Category |
| --- | --- |
| `APORIA-1xxx` | Safety and correctness |
| `APORIA-2xxx` | Architecture and maintainability |
| `APORIA-3xxx` | Security, privacy, and licensing |
| `APORIA-4xxx` | Lean core and formal library |
| `APORIA-5xxx` | Documentation and data format |
| `APORIA-6xxx` | Design review rules that require judgment |

Rules:

- Codes are permanent. Never reuse a code for a different rule.
- A retired rule keeps its page and is marked retired.
- Every code has exactly one page: `docs/lints/APORIA-xxxx.md`.
- A rule change that alters meaning gets a new code or a documented version note.

## 4. Message format

Every failure should end up in this shape, even when the underlying tool has its own format:

```text
error[APORIA-1002]: `unwrap()` can abort the app
  --> crates/aporia-core/src/note.rs:42:9
  = why: a panic loses the user's unsaved note and gives no recovery path
  = help: use `?`, `ok_or`, or a typed error; see docs/lints/APORIA-1002.md
  = note: clippy::unwrap_used
```

Built-in tools such as Clippy, `cargo-deny`, and `ast-grep` do not all support custom codes. `scripts/lint` enriches their output, and the CI summary lists every code that failed.

## 5. Suppression policy

- Do **not** use `#[allow(...)]`.
- Use `#[expect(lint, reason = "APORIA-xxxx: reason; issue #123")]`.
- `#[expect]` is preferred because it warns when the lint stops applying, so stale exceptions are removed.
- Non-Rust files use `lint-ignore: APORIA-xxxx reason: ...` and the suppression audit checks it.
- Every suppression needs a code, a reason, and a linked issue or ADR.
- `unsafe` exceptions also need an ADR, a `// SAFETY:` comment, and security review.
- Network and privacy exceptions are reviewed by the owner; they are never added by an agent alone.
- A whole-file or whole-crate exception needs an ADR. A line-level exception is not a substitute for a design fix.
- Suppressions are listed in CI output and reviewed at each milestone.

### Test code

`unwrap` and `expect` are allowed in `#[cfg(test)]` code, but prefer `expect("why this cannot fail")`. The allow is configured centrally in the crate, not sprinkled through tests.

### Unsafe code

- `unsafe_code` is denied at every crate root.
- Only the FFI crate may `expect` it, with an ADR and a safety comment for every block.
- `unsafe_op_in_unsafe_fn` is denied.
- FFI functions document ownership, lifetime, threading, and error behavior.
- Miri runs on the FFI tests where possible.

## 6. What agents must do

`AGENTS.md` repeats this in short form:

1. Run `./scripts/lint` before proposing a change.
2. Read the page for every failing code.
3. Apply an alternative from the page.
4. Do not add `#[allow]`, `unsafe`, `unwrap`, `expect`, `panic!`, or a suppression to make a gate pass.
5. If no alternative is valid, propose a policy change or an ADR. Do not bypass the gate.
6. If a rule is wrong, fix the rule and its docs in a separate change.
7. Never leave a bare `TODO`, `FIXME`, `HACK`, or `XXX`.

A change that lowers the lint gate is a policy change and needs review like code.

## 7. What humans must do

- Reviewers check that failures were fixed, not hidden.
- A reviewer may cite a `6xxx` design code in review even when no tool can detect it.
- New dependencies, network calls, global state, and file formats trigger a design-review code.
- Exceptions are revisited at each milestone; an expired exception is deleted or renewed with a reason.

## 8. Adding a rule

1. Name the incident or risk.
2. Pick the next free code in the right range.
3. Write `docs/lints/APORIA-xxxx.md` with why, alternatives, exceptions, and examples.
4. Add the tool rule or review checklist item.
5. Add a failing fixture and a passing fixture.
6. Wire it into `scripts/lint`.
7. Make CI fail on it.

No rule without a docs page. No docs page without a test fixture.

## 9. Rule page template

~~~~markdown
# APORIA-xxxx — Short title

**Severity:** error
**Tool:** clippy::lint_name / ast-grep rule / review
**Scope:** crates, files, or situations

## Why

What can go wrong, in user terms.

## Alternatives

Concrete replacements, with a short example.

## Exceptions

Who may approve one, what it must contain, and when it expires.

## Examples

Bad:

```rust
// bad
```

Good:

```rust
// good
```
~~~~

## 10. Initial rule set

This is the starting set. It grows when a real incident shows a gap. Codes are not reused.

### Safety and correctness (`1xxx`)

| Code | Rule | Why | Alternative |
| --- | --- | --- | --- |
| APORIA-1001 | No `unsafe` outside the FFI crate | Memory corruption, UB, and crashes are hard to reproduce | Safe wrapper; isolate in `ffi`; ADR plus `// SAFETY:` |
| APORIA-1002 | No `unwrap` or `expect` in production | A panic can lose unsaved notes and gives no recovery path | `?`, `ok_or`, `ok_or_else`, typed error |
| APORIA-1003 | No `panic!`, `todo!`, or `unimplemented!` | Panics and unfinished paths ship as crashes | Return `Result`; link an issue; implement it |
| APORIA-1004 | No `mem::transmute`, `mem::forget`, or `ManuallyDrop` without approval | Bypasses ownership and leaks or corrupts data | Safe conversion; `Pin`; owned types |
| APORIA-1005 | No lossy numeric casts | Truncation and sign bugs are silent | `TryFrom`, `u32::try_from`, checked arithmetic |
| APORIA-1006 | No `static mut` | Data races and aliasing UB | `OnceLock`, `Mutex`, atomics, dependency injection |
| APORIA-1007 | No `process::exit` outside `main` | Skips destructors and makes testing impossible | Return an error to `main`; let the caller decide |
| APORIA-1008 | No `println!` or `dbg!` for logging | Unstructured output bypasses log levels and privacy rules | `tracing` spans and events; CLI output module |
| APORIA-1009 | No `#[allow(...)]` | Hides debt and makes exceptions invisible | `#[expect(..., reason = "APORIA-xxxx: ...")]` |
| APORIA-1010 | No unbounded recursion | Stack overflow on user input | Iteration, explicit stack, depth limit |

### Architecture and maintainability (`2xxx`)

| Code | Rule | Why | Alternative |
| --- | --- | --- | --- |
| APORIA-2001 | No blocking I/O or CPU work on the UI thread | The window freezes and typing latency breaks | Worker thread, async task, progress state |
| APORIA-2002 | No shell interpolation | Command injection and quoting bugs | `Command` with an argv array; no `sh -c` |
| APORIA-2003 | No network calls outside the network module | Breaks the privacy allowlist and auditability | Add the call to the allowlisted client |
| APORIA-2004 | No SQL outside the store module | Scattered queries break schema ownership and migrations | Repository or store API |
| APORIA-2005 | No bare `TODO`, `FIXME`, `HACK`, or `XXX` | Permanent temporary code | `TODO(#123): ...` with an issue; or fix it |
| APORIA-2006 | No commented-out code | Confuses readers and rots | Delete it; Git keeps history |
| APORIA-2007 | No `anyhow` in library crates | Erases error types and recovery options | `thiserror` enum; `anyhow` only at the app boundary |
| APORIA-2008 | No `serde_json::Value` in domain models | Weak typing and silent schema drift | Typed structs; convert at the protocol edge |
| APORIA-2009 | No `String` for filesystem paths | Lossy conversions and platform bugs | `PathBuf` and `Path` |
| APORIA-2010 | No global mutable state | Hidden coupling and test interference | Pass dependencies; `OnceLock` for immutable config |
| APORIA-2011 | No duplicate error types per layer | Inconsistent messages and lost causes | One domain error plus context at the boundary |
| APORIA-2012 | No unbounded cache, queue, or log growth | Memory exhaustion over a long session | Explicit limits, eviction, rotation |

### Security, privacy, and licensing (`3xxx`)

| Code | Rule | Why | Alternative |
| --- | --- | --- | --- |
| APORIA-3001 | No telemetry, analytics, or crash-upload crates | Violates the no-telemetry promise | Local logs; opt-in export |
| APORIA-3002 | No network call without the consent gate | Breaks offline-by-default and user trust | Allowlisted update or Lean download flow |
| APORIA-3003 | No remote content in the preview | Tracks users and leaks note content | Local assets; block scripts, images, and fonts |
| APORIA-3004 | No unchecked path joins from user input | Path traversal outside the notebook | Canonicalize and verify under the notebook root |
| APORIA-3005 | No secrets in the repository | Leaks credentials | Secret scanner; environment or secret store |
| APORIA-3006 | No license outside the allowlist | Distribution and Apache-2.0 compatibility | `cargo-deny` allowlist; replace the dependency |
| APORIA-3007 | No command execution with untrusted input | Arbitrary code execution | Argv arrays; allowlisted binaries; validation |

### Lean core (`4xxx`)

| Code | Rule | Why | Alternative |
| --- | --- | --- | --- |
| APORIA-4001 | No `sorry` or `admit` in `Aporia.Core` | The library would claim proofs it does not have | Prove it, or mark the theorem as a hypothesis |
| APORIA-4002 | No `axiom` without an ADR | Adds trust beyond the kernel | Prove it; use a documented typeclass assumption |
| APORIA-4003 | No `native_decide` in the core | Trusts the compiler and native execution | Kernel proof; `decide` where feasible |
| APORIA-4004 | No `partial` or `unsafe` definitions | Nontermination and unsoundness | Structural recursion; total definitions |
| APORIA-4005 | No `#eval` in the library | Compile-time side effects and surprises | Tests or `#guard` in a test module |
| APORIA-4006 | Lean warnings are errors | Warnings become ignored debt | Fix the warning; do not suppress |

User notebooks are exempt from `4xxx`; `sorry` there is a visible status, not a library defect.

### Documentation and format (`5xxx`)

| Code | Rule | Why | Alternative |
| --- | --- | --- | --- |
| APORIA-5001 | Markdown and docs must pass the docs linter | Broken docs are worse than no docs | Fix the structure or link |
| APORIA-5002 | A `FORMAT.md` change needs a version and migration note | Silent format changes break notebooks | Bump `format`, add a migration, test it |
| APORIA-5003 | Public APIs need doc comments | Users cannot discover behavior or errors | Add `///` docs with errors and panics |

### Design review (`6xxx`)

These are review rules. A tool cannot decide them, but a reviewer cites the code, and the PR template asks about them.

| Code | Rule | Why | Alternative |
| --- | --- | --- | --- |
| APORIA-6001 | No new dependency without justification | Size, license, maintenance, and attack surface | Standard library; a small local function |
| APORIA-6002 | No new thread without an owner and shutdown path | Leaks and races | Scoped worker with a join or cancellation token |
| APORIA-6003 | No new file format outside `FORMAT.md` | Breaks portability and migrations | Extend the documented format |
| APORIA-6004 | No new network call without a privacy review | Breaks the offline promise | Local data; explicit consent |
| APORIA-6005 | No temporary hack without an issue and removal plan | Hacks become permanent | Fix the design; link a removal issue |
| APORIA-6006 | No duplicated logic across crates | Divergence and double fixes | Shared function or trait |
| APORIA-6007 | No unbounded resource growth | Long sessions exhaust memory or disk | Limits, eviction, backpressure |
| APORIA-6008 | No new public API without a use case and docs | API surface is forever | Keep it private until needed |

## 11. Tooling

`scripts/lint` runs, in order:

1. `cargo fmt --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo deny check`
4. `cargo audit`
5. `cargo machete`
6. `gitleaks detect`
7. `ast-grep scan` with the rules in `lints/`, plus a bad/good fixture test for
   every custom rule
8. `python3 scripts/gen-design-tokens --check` for stale tokens and contrast
   failures
9. `typos`
10. `markdownlint` or `rumdl`
11. `shellcheck` on scripts
12. `actionlint` on CI files
13. `statix` and `deadnix` on Nix files
14. `lake build` plus `scripts/lean-lint`
15. `sqlfluff` on SQL migrations, if present

Rust lints live in `[workspace.lints]` in `Cargo.toml`; thresholds live in `clippy.toml`. A rule is enabled only after its docs page and failing fixture exist.

Custom rules:

- `ast-grep` handles syntactic patterns: bare `TODO`, `#[allow]`, `sh -c`, `println!`, `unsafe` outside `ffi`, network imports outside `net`, SQL outside `store`.
- `dylint` is optional later for semantic rules that `ast-grep` cannot express.
- `scripts/lean-lint` scans `lean/Aporia` for banned tokens and parses `lake` diagnostics.
- The suppression audit scans for `#[allow]` and for `expect` without a code.

CI:

- The `lint` job runs the same script and is required for merge.
- The job fails on warnings.
- The summary lists each `APORIA-xxxx` code that failed.
- Custom rules have their own tests: a bad fixture must fail, a good fixture must pass.

## 12. Review checklist for “solutions in general”

Not every bad solution is a lint. Reviewers use these questions and cite the matching `6xxx` code:

- Does this add a dependency? Is it worth its size, license, and attack surface?
- Does this add a thread, task, or global? Who owns it and how does it stop?
- Does this change a file format? Where is the migration?
- Does this add a network call? Is it in the allowlist and behind consent?
- Is this a temporary hack? Where is the removal issue?
- Is this logic duplicated? Why not shared?
- Can this grow without a limit?
- Is this public API necessary?
- Is there a simpler solution that uses the standard library or an existing module?

A review comment without a code is still valid, but a code makes the decision searchable.

## 13. Examples

Bad:

```rust
let note = read_note(path).unwrap();
```

Good:

```rust
let note = read_note(path).ok_or(NoteError::NotFound(path.to_owned()))?;
```

Bad:

```rust
let output = Command::new("sh").arg("-c").arg(user_input).output()?;
```

Good:

```rust
let output = Command::new("git").arg("status").arg("--short").output()?;
```

Bad:

```rust
#[allow(clippy::unwrap_used)]
fn parse(...) { ... }
```

Good:

```rust
#[expect(clippy::unwrap_used, reason = "APORIA-1002: fixture invariant; issue #42")]
fn parse_fixture(...) { ... }
```

## 14. First rules to implement

P0 should land these before product code:

- APORIA-1001, 1002, 1003, 1008, 1009
- APORIA-2001, 2005, 2009
- APORIA-3001, 3002, 3005
- APORIA-4001, 4002, 4003
- APORIA-5001, 5002

The rest can follow as the relevant modules appear. The code and docs page are required before the rule is enabled.
