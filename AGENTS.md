# Agent Instructions

Read `LINT-POLICY.md` before changing code. It defines the `APORIA-xxxx` rules, why they exist, and the accepted alternatives.

## Before proposing a change

1. Run `./scripts/lint`.
2. Fix every finding using the alternative on its `docs/lints/APORIA-xxxx.md` page.
3. Re-run until the gate is clean.

## Never do these to pass a gate

- Add `#[allow(...)]`; use `#[expect(..., reason = "APORIA-xxxx: ...")]` only when approved.
- Add `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, or `println!` in production code.
- Disable a lint, lower a threshold, or add `continue-on-error`.
- Leave a bare `TODO`, `FIXME`, `HACK`, or `XXX`.
- Add a network call outside the consent gate.
- Add a dependency without checking license, size, and maintenance.

## If a rule is wrong

Propose a policy change with an ADR or issue. Do not bypass the rule in code. A change to the lint gate is reviewed like code.

## When reporting a failure

Include the `APORIA-xxxx` code, the file and line, the chosen alternative, and any approved exception. If no alternative is valid, say why and propose a rule change.
