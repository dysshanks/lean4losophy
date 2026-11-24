# APORIA-4002 — No `axiom` without an ADR

**Severity:** error
**Tool:** `scripts/lean-lint` banned-token scan, review
**Scope:** `lean/Lean4losophy`; user notebooks are exempt, but their axioms are not imported into the library.

## Why

An axiom is trust added on top of the kernel. The logic library is where that trust is easiest to hide: a modal module that postulates the S5 reduction axiom, or a `by_valid` helper that assumes a classical principle, makes the checker accept arguments it did not derive. A learner sees a green badge; a paper appendix shows a checked proof whose foundation is an unstated assumption.

Axioms are sometimes the right tool. They are never an implementation detail.

## Alternatives

- Prove the statement from existing definitions.
- Make the assumption a typeclass parameter so it appears in the theorem's signature and in the goal view.
- Use a shallow embedding with an explicit semantics and prove soundness for the fragment you need.
- If the axiom is unavoidable, write the ADR first, including the model that justifies it.

## Exceptions

- The Lean owner plus a second reviewer approve, with an ADR that names the axiom, the intended model, why it cannot be proved, and the trust impact.
- The axiom is documented in the library README and shown in the UI when an argument depends on it.
- Reviewed at each milestone; an axiom with no user-facing justification is removed.
- No exception for axioms that make an invalid argument pass `by_valid`.

## Examples

Bad:

```lean
-- no ADR, no model, no warning in the UI
axiom s5_refl : □P → P
```

Good:

```lean
-- ADR-0004: Kripke semantics; reflexivity is a typeclass assumption,
-- so every theorem that uses it shows it in the statement.
class Reflexive (World : Type) (r : World → World → Prop) : Prop where
  refl : ∀ w, r w w
```
