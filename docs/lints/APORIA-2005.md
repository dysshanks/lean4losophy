# APORIA-2005 — No bare `TODO`, `FIXME`, `HACK`, or `XXX`

**Severity:** error
**Tool:** `ast-grep` rule `bare-marker`, `typos`, review
**Scope:** all source, docs, templates, scripts, and CI files.

## Why

A bare marker records a feeling, not a task. Nobody owns it, no issue tracks it, and it survives every milestone. In this codebase the markers would sit on the paths that matter most: transclusion cycle handling in the paper renderer, a fallback when the Lean server exits, or a missing citation check before export. A user then meets the unfinished behaviour with no warning, and a reviewer cannot tell whether it is a known gap or an accident.

## Alternatives

- Fix the problem now; it is usually smaller than the marker suggests.
- Link an open issue: `TODO(#214): reject transclusion cycles deeper than 8`.
- Put the design gap in an ADR or in `OPEN-QUESTIONS.md` instead of the code.
- For a deferred feature, return a typed `Unsupported` error (APORIA-1003) and link the issue in the message.
- For a rule you think is wrong, change the rule through the policy process.

## Exceptions

- None for a bare marker.
- A linked marker is allowed only while the issue is open; the reviewer checks that the number exists.
- A marker still present at the next milestone is either fixed or escalated in review.
- Markers are not a substitute for a failing test or a fixture.

## Examples

Bad:

```rust
// TODO: handle transclusion cycles
// HACK: Lean server may be dead here
```

Good:

```rust
// TODO(#214): reject transclusion cycles deeper than 8; see ADR-0007
// TODO(#231): restart the Lean server once before showing diagnostics
```
