# Contributing

## Before you start

Read these, in order:

1. [SPEC.md](SPEC.md) for what the product is and who it serves.
2. [IMPLEMENTATION-PLAN.md](IMPLEMENTATION-PLAN.md) for the current milestone,
   its acceptance criteria, and its gate.
3. [LINT-POLICY.md](LINT-POLICY.md) for the `APORIA-xxxx` rules, the accepted
   alternatives, and the suppression process.
4. [DESIGN-PLAN.md](DESIGN-PLAN.md) before changing anything users see.
5. [AGENTS.md](AGENTS.md) if you use a coding agent; the same rules apply to
   human contributors.

Do not work outside the current milestone without agreeing on the scope change
first.

## Workflow

1. Create a branch from `main` with a short, specific name.
2. Make one focused change. Keep unrelated refactors out of the same change.
3. Run `./scripts/lint` and `cargo test --workspace`. Both must pass.
4. If the change touches a public interface, a file format, a dependency, the
   network boundary, or `unsafe`, write or update an ADR (see below).
5. Open a pull request using the template. Describe the behavior change, the
   tests, and any `APORIA-xxxx` code you had to address.
6. Keep the pull request small enough to review. Split it when it grows.

Do not commit generated caches, build output, or notebook caches. `.gitignore`
covers the usual cases.

## The lint gate

`./scripts/lint` is the single entry point. CI runs the same script and the
`lint` job is required.

- Findings are errors. A warning is not debt to fix later.
- Fix the cause. Read the page for the failing code in `docs/lints/`.
- Do not add `#[allow(...)]`, `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`,
  `unimplemented!`, `dbg!`, or `println!` to make the gate pass.
- Do not disable a lint, lower a threshold, or add `continue-on-error`.
- Do not leave a bare `TODO`, `FIXME`, `HACK`, or `XXX`. Use a tracked form such
  as `TODO(#123): ...` with an issue, or fix it.
- If a rule is wrong, change the rule and its docs page in a separate change.
  A change to the gate is reviewed like code.

Approved exceptions use `#[expect(lint, reason = "APORIA-xxxx: reason; issue #123")]`
and must be listed in the pull request. `#[expect]` is preferred because it
warns when the exception becomes stale. Non-Rust files use
`lint-ignore: APORIA-xxxx reason: ...`. A whole-file or whole-crate exception
needs an ADR.

## Architecture decision records

Write an ADR in `docs/adr/` before implementing a decision that is expensive to
reverse. This includes:

- the UI architecture and process model;
- a new file format or a change to `FORMAT.md`;
- a new dependency that is large, unusual, or license-sensitive;
- a network call, consent flow, or privacy boundary;
- `unsafe` code or an FFI boundary;
- a change to the lint policy or the quality gate.

Use the next free four-digit number and a short kebab-case title:
`docs/adr/0002-markdown-preview-stack.md`. An ADR contains:

- **Status:** proposed, accepted, superseded, or rejected.
- **Context:** the forces and constraints.
- **Decision:** what was chosen.
- **Alternatives:** what was rejected and why.
- **Consequences:** costs, risks, and follow-up work.

Link the ADR from the pull request. A superseded ADR stays in the repository and
points at its replacement.

## Tests

- Add a test for every behavior change, including the failure path.
- Prefer unit tests next to the code and integration tests under the crate's
  `tests/` directory.
- Golden tests cover Markdown to HTML, argument to Lean skeleton, and paper to
  Typst. Update goldens only when the change is intended, and review the diff.
- Do not use `unwrap` or `expect` to skip error handling in production code.
  In `#[cfg(test)]` code, prefer `expect("why this cannot fail")`.
- Tests must not require the network, a display server, or a Lean toolchain
  unless the test is explicitly marked and documented as such.

## Documentation and design

- Update the relevant document when behavior, format, or design rules change.
- Public Rust APIs need doc comments that state errors and panics.
- User-visible changes follow `DESIGN-PLAN.md`; the pull request template
  includes its checklist.
- Do not put time estimates in issues, plans, pull requests, or reviews.
  Describe scope and order instead.

## License

Contributions are licensed under Apache-2.0, the same license as the project.
By submitting a pull request you confirm that you have the right to contribute
the change under that license. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
