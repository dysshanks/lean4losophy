# ADR-0005 — Lint policy and the `APORIA-xxxx` scheme

- **Status:** Accepted
- **Date:** 2026-11-24
- **Gate:** G1 (packaging and policy), then every milestone
- **Related:** ADR-0001, ADR-0003

## Context

Most code is written by agents, with human review at gates. Warning-only debt accumulates silently, and a tool's native message rarely tells the reader what to do next. The project also has rules that no off-the-shelf linter enforces: no telemetry, no network outside the consent gate, no format change without a migration, no `sorry` in the Lean library.

`LINT-POLICY.md` already defines the policy and the initial rule set. This ADR accepts it as binding and records why.

## Decision

Accept `LINT-POLICY.md` as project policy, including:

- **Code scheme:** `APORIA-<category><three digits>`, with categories:
  - `1xxx` safety and correctness;
  - `2xxx` architecture and maintainability;
  - `3xxx` security, privacy, and licensing;
  - `4xxx` Lean core and formal library;
  - `5xxx` documentation and data format;
  - `6xxx` design review rules that require judgment.
- **Codes are permanent.** A retired rule keeps its page and is marked retired. A code is never reused for a different rule.
- **One page per code** in `docs/lints/APORIA-xxxx.md`, explaining what it catches, why it matters, the alternatives, the exception process, and examples.
- **No rule without a docs page and a failing fixture.** A rule is enabled only after both exist.
- **Warnings are errors.** CI fails on warnings; the `lint` job is required and has no `continue-on-error`.
- **One entry point:** `scripts/lint` runs the same checks locally and in CI, and enriches tool output with the code and doc link.
- **No `#[allow(...)]`.** Exceptions use `#[expect(lint, reason = "APORIA-xxxx: reason; issue #123")]`; non-Rust files use `lint-ignore: APORIA-xxxx reason: ...`.
- **Every suppression needs a code, a reason, and a linked issue or ADR.** A suppression audit runs in CI.
- **`unsafe` only in the FFI crate**, with an ADR, a `// SAFETY:` comment for every block, and security review. Plan B keeps that crate optional (ADR-0001).
- **Network and privacy exceptions are owner-approved** and never added by an agent alone.
- **Whole-file or whole-crate exceptions need an ADR.** A line-level exception is not a substitute for a design fix.
- **Policy changes are reviewed like code.** Lowering the gate, disabling a rule, or changing a threshold is a policy change, not a workaround.
- **No bare `TODO`, `FIXME`, `HACK`, or `XXX`.** Use `TODO(#123): ...` with an issue, or fix it.

## Consequences

Positive:

- Every failure names a rule, a reason, and an alternative, so an agent can fix it without guessing.
- Privacy, format, and Lean rules that tools cannot detect still have a searchable code and a review hook.
- Suppressions are visible and expire; debt cannot hide behind `#[allow]`.
- The policy is testable: custom rules have bad and good fixtures.

Negative:

- The first changes are slower: docs pages, fixtures, and CI wiring come before product code.
- The gate can be noisy or overfit; rules need tuning and quarterly review.
- Agents must read the rule page and choose an alternative instead of silencing the tool.
- Some rules are judgment calls (`6xxx`) and cannot be fully automated; reviewers carry that load.

Neutral and follow-ups:

- P0 lands the first rule set listed in `LINT-POLICY.md` section 14.
- Each milestone re-reviews suppressions; expired exceptions are deleted or renewed.
- A rule that fights real code is changed through the policy process, not bypassed in a change.

## Alternatives considered

### Warnings as warnings

Common, but warning debt becomes permanent and CI stops meaning anything. Rejected.

### Tool-native messages only

Faster to set up, but messages do not carry a stable code, a rationale, or an alternative, and cannot cover privacy or format rules. Rejected.

### Adopt an off-the-shelf ruleset wholesale

Useful as a base (Clippy, `cargo-deny`, `ast-grep`), but it does not cover no-telemetry, network consent, format migrations, or Lean library rules. Rejected as a complete policy; used as tooling underneath.

### Allow suppressions with review

Review alone does not scale and leaves no searchable record. Rejected; `#[expect]` with a code and reason is the narrow exception.

## References

- `LINT-POLICY.md` for the policy, rule table, and tooling.
- `docs/lints/` for the per-code pages.
- `AGENTS.md` for the agent checklist.
- `IMPLEMENTATION-PLAN.md` sections 7 (M0) and 16 (testing and quality).
