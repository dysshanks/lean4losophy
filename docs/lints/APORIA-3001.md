# APORIA-3001 — No telemetry, analytics, or crash-upload crates

**Severity:** error
**Tool:** `cargo-deny` banned crates, dependency review, `ast-grep` rule `telemetry-crate`
**Scope:** the workspace, including build scripts, CI helpers, and bundled templates.

## Why

The product promise is no account and no telemetry, offline by default. Notes are private philosophical drafts, reading lists, and source quotes; usage data would reveal what a user is writing about, when, and for how long. A crash uploader is worse: a panic in the note parser can carry a stack trace, a file path, or a fragment of the note body to a third party, often before the user can decide.

Adding an SDK also changes the Flatpak permission story and the privacy statement, so it is a product decision, not a dependency choice.

## Alternatives

- Log locally with `tracing` to `.aporia/logs`, rotated and bounded (APORIA-2012).
- Offer a diagnostics export the user triggers and can read before sending: version, OS, enabled features, and recent local log lines with note content removed.
- Ask for a bug report with reproduction steps instead of automatic collection.
- Use CI-only metrics for build health; never ship them in the app.

## Exceptions

- No exception for telemetry, analytics, session recording, or automatic crash upload in v1.
- A future opt-in export needs the owner, a privacy review, an ADR, a visible consent screen, and documentation of exactly what leaves the machine.
- Consent is never pre-checked, never bundled with install, and revocable offline.
- Any exception also triggers APORIA-3002 and APORIA-6004.

## Examples

Bad:

```rust
sentry::init(("https://key@sentry.example/1", sentry::ClientOptions::default()));
analytics::track("note_opened", json!({ "title": note.title }));
```

Good:

```rust
tracing::error!(error = %err, "failed to parse note");
// User-initiated: writes a reviewable bundle under .aporia/diagnostics/
let bundle = diagnostics::export(&workspace, DiagnosticsOptions::redact_notes())?;
```
