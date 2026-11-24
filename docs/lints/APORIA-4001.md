# APORIA-4001 — No `sorry` or `admit` in `Aporia.Core`

**Severity:** error
**Tool:** `scripts/lean-lint` banned-token scan, `lake build`, review
**Scope:** `lean/Aporia`, the shipped library; user notebooks are exempt.

## Why

`sorry` closes a goal without a proof. In a note that is honest: the UI shows "formalized (has sorry)" and the paper appendix prints the status. In the shipped library it is a lie with a green check attached. A `by_valid` tactic that succeeds through `sorry` tells a learner an invalid argument is valid, and a paper that cites `Aporia.Core` claims a result the kernel never checked.

## Alternatives

- Prove the goal.
- Restate the theorem with the missing fact as a hypothesis, so the assumption is visible in the type.
- Move unfinished experiments to a scratch file outside `lean/Aporia`.
- Mark the argument as unformalized or formalized-with-`sorry` in the UI instead of shipping it.

## Exceptions

- User notebooks are out of scope for all `4xxx` rules; `sorry` there is a visible status, not a library defect.
- A temporary `sorry` in `Aporia.Core` needs the Lean owner, an ADR, and an issue; it fails the release gate.
- `admit` is never approved; it is `sorry` with a different name.
- The scan covers generated files too; generate proofs instead of suppressing the scan.

## Examples

Bad:

```lean
theorem modus_ponens (h₁ : P) (h₂ : P → Q) : Q := by
  sorry
```

Good:

```lean
theorem modus_ponens (h₁ : P) (h₂ : P → Q) : Q := h₂ h₁
```
