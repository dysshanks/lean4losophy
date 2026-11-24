# ADR-0002 — Markdown dialect and preview stack

- **Status:** Accepted
- **Date:** 2026-11-24
- **Gate:** G2 (notes shell)
- **Related:** ADR-0001, ADR-0004

## Context

Notes are plain Markdown files that must also be readable in any editor. The open questions settle the dialect as CommonMark (Q2) and the preview as both side-by-side and toggle (Q3). The v1 feature set needs tables and footnotes, wiki links, argument blocks, and Lean blocks rendered as cards with status.

The preview is the largest untrusted-content surface in the app. A note can come from anywhere; it must not be able to execute code, load remote resources, exfiltrate note content, or reach the network. The preview also sits in the typing path, so a slow parse or render breaks the 16 ms keystroke target.

## Decision

### Dialect

- **CommonMark** (0.31.2) is the baseline.
- Exactly two extensions are enabled: **tables** and **footnotes**.
- GFM task lists, strikethrough, autolink literals, and tagfilter stay off.
- Pandoc Markdown is not supported as an input dialect.
- No custom syntax is added for arguments, links, or blocks. Wiki links and block IDs are defined in `FORMAT.md`; argument blocks use the documented Markdown list convention.
- The extension set is frozen in `FORMAT.md` and covered by golden fixtures. A new extension is a format change under `APORIA-5002`.

### Parser

- One parser: `comrak`, compiled with only the table and footnote extensions. It is CommonMark-compliant, produces a single AST, and has no network or runtime dependency.
- The AST is the shared input for the preview, link and backlink extraction, search indexing, and the Markdown-to-Typst paper converter. A second parser is not introduced for any of these paths.
- Raw HTML is not passed through. The renderer escapes it; anything that survives is still sanitized.
- `pulldown-cmark` is the fallback if `comrak` fails a conformance or performance fixture. The choice is recorded here; the AST boundary stays the same.

### Sanitization

- Render to HTML, then sanitize with `ammonia` and an explicit allowlist.
- Allowed: headings, paragraphs, emphasis, strong, lists, block quotes, code and pre, tables, footnotes, horizontal rules, links, and local images.
- Removed: `script`, `style`, `iframe`, `object`, `embed`, `form`, `input`, `svg`, event-handler attributes, `javascript:` and `data:text/html` URLs, and inline `style` attributes.
- Syntax-highlight classes are allowed from a generated list; no arbitrary class or id attributes.
- Wiki links become internal navigation targets (`lean4losophy://note/...` or an equivalent app-internal scheme), not file URLs.
- Lean and argument blocks render as cards. Lean source is displayed as text; it is never executed by the preview.

### Preview widget

- WebKitGTK 6.0 through the `webkit2gtk` crate (webkit2gtk-rs), in a `GtkBox`/`GtkPaned`.
- One renderer serves both layouts: side-by-side and toggle differ only in container and visibility, never in parsing or sanitization.
- JavaScript is disabled at the WebKit settings level, and the content security policy denies it as a second layer.
- Content security policy: `default-src 'none'; img-src file: data:; style-src 'self' 'unsafe-inline'; font-src 'self'`. No `connect-src`, no remote hosts.
- No remote images, fonts, stylesheets, or media. Images must resolve under the notebook root; anything else renders as a blocked placeholder with the URL shown as text.
- External links are intercepted and opened in the host browser through the portal. The preview never navigates away from local content.
- Local navigation, in-page anchors, and backlinks stay inside the app.
- Parse and render run on a worker; the UI receives a finished document. The debounce is short (about 150 ms) and stale renders are cancelled. The UI thread never parses or lays out note content synchronously (`APORIA-2001`).

### Fallback

WebKitGTK is revisited at G2 if any of these holds:

- it misses the cold-start or memory targets in `SPEC.md` section 9;
- it cannot be packaged or updated safely in the chosen Flatpak runtime;
- an unpatched critical advisory makes it unsuitable for untrusted content.

The fallback is a native renderer built on GTK text and drawing widgets with the same AST and the same sanitization rules. The fallback is recorded in this ADR before it is implemented, so the security and layout rules do not fork.

## M1 implementation status (2026-11-24)

M1 ships the **documented native fallback**, not WebKitGTK. The build
environment has GTK4 and libadwaita but no WebKitGTK 6.0 or GtkSourceView 5
development packages, and the app must build and run.

What is in place:

- One parser: comrak with only tables and footnotes enabled, in
  `lean4losophy_core::preview`. It produces an owned block model shared by the
  renderer and future exporters.
- The preview is a GTK widget tree built from those blocks. It performs no I/O,
  cannot load a remote image or font, and drops raw HTML at parse time. The
  security promises (no JavaScript, no remote content) hold by construction
  because there is no web engine.
- The editor is a `GtkTextView` with line wrap, monospace text, and undo/redo.

What is missing relative to the sections above, and must be closed before G2:

- Syntax highlighting, line numbers, find/replace, and the GtkSourceView
  integration.
- Local images, clickable links, and external links opening in the browser.
  Inline link text is rendered, but the target is not yet actionable.
- The WebKit path itself: CSP, settings, and the JavaScript-off test.

This is a substitution, not a reversal. WebKitGTK remains the intended preview
for the shipped app, and the block model is the seam that keeps the swap from
being a rewrite. Revisit at Gate G2; if the native renderer is kept for v1, this
ADR must be updated rather than left describing a stack the app does not use.

## Consequences

Positive:

- Notes stay portable and readable outside the app.
- One AST keeps preview, links, search, and paper export consistent.
- A small, frozen dialect is testable and does not grow by accident.
- Sanitization and a no-network policy make untrusted notes safe to open.

Negative:

- Tables and footnotes are extensions, so a strict CommonMark reader elsewhere may not render them identically.
- WebKitGTK is a large dependency and a recurring security surface; it must be updated with the runtime.
- A sanitizer allowlist needs maintenance when new rendering features arrive.
- CommonMark has no native wiki-link or block-reference syntax; those are app conventions and will look like plain text in other editors.

Neutral and follow-ups:

- Preview fixtures cover tables, footnotes, nested lists, raw HTML, `javascript:` links, remote images, and malformed input.
- The same fixtures feed the paper converter so the two renderers do not drift.
- The CSP and WebKit settings are tested, not just documented.

## Alternatives considered

### GitHub-flavored Markdown

Adds task lists, strikethrough, autolinks, and tagfilter that v1 does not need, and still lacks footnotes. Rejected as a larger surface with no user benefit.

### Pandoc Markdown

Rich, but ties the app to Pandoc, which is optional and GPL-2.0-or-later. It also makes the on-disk dialect depend on an external tool. Rejected.

### A custom Markdown subset

Would simplify the parser but break the plain-file promise and make notes unreadable elsewhere. Rejected.

### Native GTK renderer first

Avoids WebKit but costs more layout work up front and risks a weaker reading experience. Kept as the documented fallback rather than the default.

## References

- `SPEC.md` sections 4.1, 4.6, 9, 14.
- `IMPLEMENTATION-PLAN.md` section 15 (Preview) and section 18 (risks).
- `FORMAT.md` for the link, block, and argument conventions.
- `APORIA-2001`, `APORIA-3002`, `APORIA-3003`.
