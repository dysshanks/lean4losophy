# APORIA-5001 — Markdown and docs must pass the docs linter

**Severity:** error
**Tool:** `markdownlint` or `rumdl`, link check, review
**Scope:** all tracked Markdown: `docs/`, `format/FORMAT.md`, `README.md`, templates, and PR text that becomes a file.

## Why

Docs are how a learner gets unstuck and how another developer implements the format. A broken link in `docs/lints/` sends an agent to a missing page instead of an alternative. A malformed table in `FORMAT.md` makes the argument grammar ambiguous, and two parsers then disagree about which list item is the conclusion. A stray heading level breaks the outline the user guide is built from.

The linter catches structural faults cheaply: heading order, unclosed fences, duplicate headings, missing fence languages, and links that point nowhere.

## Alternatives

- Fix the structure: one H1 per page, sequential heading levels, a language on every fence.
- Point links at real files with relative paths, for example `[APORIA-1002](APORIA-1002.md)`.
- Keep tables rectangular; move long prose out of cells.
- Run the docs linter locally through `./scripts/lint` before proposing the change.
- If a rule is wrong for a page, change the rule and its docs in a separate reviewed change.

## Exceptions

- Generated Markdown (for example a vendored `NOTICE` or an exported fixture) may be excluded by path in the linter config, with owner approval and a reason.
- No per-line suppression; fix the page.
- An exception is listed in CI output and re-checked at each milestone.

## Examples

Bad:

```markdown
# Links
### Where to go
See [the rule](APORIA-9999.md) and [the format](format/FORMAT.md).
```

Good:

```markdown
# Links

## Where to go

See [APORIA-1002](APORIA-1002.md) and [FORMAT.md](../../format/FORMAT.md).
```
