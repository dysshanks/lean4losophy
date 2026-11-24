# Architecture Decision Records

One file per decision. An ADR records why a choice was made, what it costs, and when it is revisited. It is not a tutorial and it does not duplicate `SPEC.md`, `FORMAT.md`, `DESIGN-PLAN.md`, or `LINT-POLICY.md`.

## Index

| ADR | Title | Status | Gate |
| --- | --- | --- | --- |
| [0001](0001-ui-architecture.md) | UI architecture: Rust GTK4 with a Lean sidecar | Proposed | G0 |
| [0002](0002-markdown-preview.md) | Markdown dialect and preview stack | Accepted | G2 |
| [0003](0003-packaging-update.md) | Packaging and update model | Accepted | G1, G8 |
| [0004](0004-data-format.md) | Data format, identity, settings, and concurrency | Accepted | G3 |
| [0005](0005-lint-policy.md) | Lint policy and the `APORIA-xxxx` scheme | Accepted | G1 |

## Status values

- **Proposed:** recommended, not yet proven. The gate named in the ADR closes it.
- **Accepted:** binding for the current milestone.
- **Superseded:** replaced by a later ADR; keep the file and link the replacement.
- **Rejected:** considered and not adopted; keep the reason.

## When to write one

Write an ADR when a decision:

- changes the architecture, a process boundary, or a packaging channel;
- adds a file format, a network call, or a dependency with license or privacy impact;
- needs an exception to `LINT-POLICY.md`;
- is expensive to reverse after M1.

Small reversible choices stay in code review or in the relevant plan.

## Template

```markdown
# ADR-0000 — Title

- **Status:** Proposed
- **Date:** YYYY-MM-DD
- **Gate:** G0
- **Related:** links

## Context

The forces at play, in user terms.

## Decision

What is chosen, in the present tense.

## Consequences

Positive, negative, and follow-ups.

## Alternatives considered

What was rejected and why.

## References

Spec sections, plan sections, ADRs, lint codes.
```

## Rules

- One decision per ADR. Split a bundle rather than hiding a second choice in a paragraph.
- State the gate and the evidence that closes it.
- Record the cost, not only the benefit.
- Do not silently edit an accepted decision. Supersede it with a new ADR.
- No time estimates; use milestones and gates.
