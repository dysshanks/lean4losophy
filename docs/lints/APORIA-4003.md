# APORIA-4003 — No `native_decide` in the core

**Severity:** error
**Tool:** `scripts/lean-lint` banned-token scan, `lake build`, review
**Scope:** `lean/Lean4losophy`, the shipped library; tests and user notebooks are exempt.

## Why

`native_decide` compiles the proposition to native code, runs it, and trusts the result. It bypasses the kernel, so the proof depends on the compiler, the runtime, and the machine. That is a poor fit for a library whose value is that the kernel checked the argument.

The likely use here is the counterexample finder and the truth-table helpers, where it is fast and tempting. If a code-generation bug flips the answer, a learner is told a valid argument is invalid, or the reverse. The result is also not reproducible from the paper appendix, which prints Lean source a reader may re-check on another toolchain.

## Alternatives

- Give a kernel proof, even if it is longer.
- Use `decide` where the proposition has a `Decidable` instance; it produces a proof term the kernel checks.
- Enumerate cases explicitly for small propositional formulas; a six-letter truth table is 64 cases.
- Put experimental evaluation in a test module with `#guard` or `#eval` (APORIA-4005), not in the library.

## Exceptions

- The Lean owner may approve `native_decide` in a test or tooling module, with a reason and an issue, when a kernel proof is impractical.
- No exception for `Lean4losophy.Core`, for any lemma used by `by_valid`, or for anything cited in a paper appendix.
- If performance forces it, the ADR must include an independent kernel check of the same statement.

## Examples

Bad:

```lean
theorem tautology (φ : Formula) : Tautology φ := by
  native_decide
```

Good:

```lean
theorem tautology (φ : Formula) : Tautology φ := by
  decide
```
