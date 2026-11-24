# APORIA-xxxx — Short title

**Severity:** error
**Tool:** `clippy::lint_name` / `ast-grep` rule id / `review`
**Scope:** crates, files, or situations the rule covers
**Status:** enabled

## Why

What can go wrong, in user terms. Name the failure the rule prevents: lost notes,
a crash, a privacy leak, an unreadable notebook, a silent format change. One
short paragraph is enough; a rule nobody can justify should not exist.

## What it catches

Concrete patterns. For a syntactic rule, show the shape that triggers it. For a
review rule, list the questions a reviewer asks. State what is out of scope so
the rule does not creep.

## Alternatives

The fix a person or agent should apply, with a short example. At least one
alternative must be valid for every realistic case; if none is, the rule is
wrong and needs a policy change (see `LINT-POLICY.md` section 6).

## Exceptions

- Who may approve an exception.
- What the exception must contain: the code, a reason, and an issue or ADR.
- When it expires and who revisits it.
- Anything that is never exempt.

## Examples

Bad:

```rust
// bad
```

Good:

```rust
// good
```

## Enforcement

- Where the rule runs: `./scripts/lint`, CI, review.
- The fixture that must fail and the fixture that must pass.
- The related codes, if any.
