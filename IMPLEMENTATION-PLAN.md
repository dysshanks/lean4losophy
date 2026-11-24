# Implementation Plan — lean4losophy

**Status:** Draft
**Basis:** `SPEC.md` and the answers in `OPEN-QUESTIONS.md`
**Date:** 2026-11-24
**Build model:** fully agentic; human review at the gates.

---

## 1. Fixed inputs

These are treated as decided and are not reopened here:

- No account, no login, no telemetry. Offline by default.
- Lean is optional; the app is fully usable without it.
- Typst is the paper engine.
- v1 is section 17.3’s **In** list. Anything outside it is post-v1.
- The first prototype is the notes shell.
- Install paths: Flatpak and Nix/AUR.
- License: Apache-2.0.
- Name: lean4losophy (final); app ID `io.github.dysshanks.lean4losophy`.
- Visual and paper rules are in `DESIGN-PLAN.md`.

## 2. Answers and impact

| Question | Answer | Impact on the plan |
| --- | --- | --- |
| Q1 Prototype scope | Editing, links, backlinks, search | Prototype exits after M2, not M1 |
| Q2 Markdown | CommonMark | CommonMark baseline, plus only the extensions v1 requires (tables, footnotes) |
| Q3 Preview | Both side-by-side and toggle | One preview widget, two layouts |
| Q4 Structure | Flat and folders | File tree supports both; no forced hierarchy |
| Q5 Navigation | Sidebar + split panes | `AdwOverlaySplitView` plus `GtkPaned` |
| Q6 First run | Sample notebook, ask Guided/Expert, offer Lean | Onboarding wizard is part of M1 |
| Q7 Links | Aliases, heading links, block references | All three tiers are in M2 |
| Q8 Search | Tag/folder filters, ranking, snippets | Full FTS5 feature set in M2 |
| Q9 Identity | Hybrid | Stable ULID in front matter, path fallback |
| Q10 Settings | Split | Global config in XDG; notebook settings in `.aporia/` |
| Q11 Concurrency | Last writer wins | No merge UI; watcher plus warning |
| Q12 Format docs | Separate `FORMAT.md` | P0 deliverable |
| Q13 Updates | In-app check on launch; Flatpak store-managed; Nix/AUR in-app | Checker/notifier in v1, not a self-replacing binary |
| Q14 Install | Flathub/app center and `.flatpakref` | Both documented and tested |
| Q15 Lean | Downloaded on demand, with consent | Lean manager in M4; no Lean at app build time |
| Q16 Platform | Any GTK4/Wayland distro | Flatpak runtime sets the floor; native builds tested on Ubuntu, Fedora, Arch |
| Q17 Arguments | Markdown list convention | Grammar frozen in `FORMAT.md`, parsed from the AST |
| Q18 License | Apache-2.0 | `LICENSE`, `NOTICE`, SPDX headers, dependency audit |
| Q19 Name | lean4losophy (final) | App ID `io.github.dysshanks.lean4losophy` used in packaging |

**Answer notes:** Q7 and Q8 had multiple boxes checked; they are read as all listed tiers. Q13 had two boxes checked; this plan interprets them as: the app checks for updates on launch and shows a channel-specific action; Flatpak is updated through the store; Nix/AUR are updated by the user with in-app instructions. A true self-replacing binary is out of scope for v1.

## 3. Architecture decision — Gate G0

The spec leaves the UI route open: Lean calls GTK through FFI, or a separate GUI process talks to Lean. This must be settled before the notes shell is written, because the UI code is not portable between the two.

**Recommended path: Plan B**

- GTK4 + libadwaita UI in Rust (`gtk4-rs`).
- Notes, Markdown, links, search, arguments, papers, and settings in Rust.
- Lean is a sidecar: the app manages an optional Lean toolchain and talks LSP to `lake serve`.
- The Lean logic library ships as `.lean` source files, not as part of the app binary.

Why this is recommended for an agent-built project:

- The app must work fully without Lean. Keeping the notebook model out of a Lean binary makes that automatic.
- It removes the Lean-runtime/GObject memory-ownership problem from the critical path.
- Rust has mature crates for Markdown, SQLite, file watching, HTTP, and LSP.
- Flatpak and Nix packaging are well-trodden for GTK/Rust apps.
- The Lean integration remains isolated and can be improved without touching the editor.

**Plan A remains valid:** a Lean application with a thin C FFI shim to GTK. It is closer to the spec’s “core language: Lean 4” and may be preferable if the Lean core is the point of the project. It carries the FFI, event-loop, packaging, and memory-ownership risks the spec already names.

**Spikes, timeboxed:**

1. Plan A spike: open a GTK4 window from Lean, attach one callback, open and save a file. Three days.
2. Plan B spike: same window in `gtk4-rs`, plus a sidecar call to Lean. Three days.
3. Packaging spike: build the winning spike as a Flatpak, a Nix package, and an AUR package.

**Gate G0 passes when:** one path has a window, a working editor, open/save, and a packaged build. Choose on measured time, not preference. Record the result in `docs/adr/0001-ui-architecture.md`.

**The task breakdown below assumes Plan B.** Plan A adds a separate FFI hardening track to M1–M3.

## 4. Scope

### In v1 (from 17.3)

- Markdown notes, links, backlinks, search.
- Argument blocks with an automatic map.
- Lean blocks with diagnostics and skeleton generation.
- Guided and Expert modes.
- Propositional and first-order library.
- Counterexample finder.
- A small exercise set.
- BibTeX.
- Paper projects on Typst with two templates.
- PDF, Markdown, and DOCX export.
- Home screen and sample notebook.
- Lean as an optional download.

### Out of v1

- Cloud sync, accounts, collaboration, mobile, Windows, macOS.
- Telemetry, analytics, crash uploads.
- Mathlib by default, modal/deontic/epistemic modules.
- Live transclusion, blind review, snapshot UI, word targets, index of terms.
- EPUB, PDF/A, LaTeX pipeline, argument-map SVG export.
- Six templates; v1 ships essay and article.
- Premise type tags, manual fallacy annotations, Vim mode.
- Import from Obsidian or a Markdown folder.

### Spec conflicts resolved in P0

- `SPEC.md` no longer describes EPUB, six templates, or a LaTeX pipeline as v1;
  those sections now mark them post-v1 or removed.
- The spec's “core language: Lean 4” line now reads Rust application with a Lean
  sidecar, matching Plan B and ADR-0001. Argument blocks remain usable without
  Lean because the notebook model is Rust.
- The counterexample finder is described as part of the Rust app core, not
  “written in Lean”, so it works with Lean absent.
- DOCX via Pandoc still conflicts with “Pandoc optional” and with Apache-2.0 if
  Pandoc is bundled. M6 decides between host Pandoc, bundled Pandoc with GPL
  notices, and a native exporter.
- “Offline by default” is resolved by an off-by-default consent toggle and one
  allowlisted update check (ADR-0003).

## 5. Repository layout (Plan B)

```text
.
├── Cargo.toml                  # workspace
├── LINT-POLICY.md              # rule codes, enforcement, suppression
├── DESIGN-PLAN.md              # visual system, component rules, paper rules
├── AGENTS.md                   # instructions for coding agents
├── CONTRIBUTING.md             # human workflow
├── crates/
│   ├── lean4losophy/                 # GTK4 app, wiring, windows
│   ├── lean4losophy-core/            # notebook, Markdown, links, search, arguments
│   ├── lean4losophy-lean/            # toolchain manager, LSP client, diagnostics
│   ├── lean4losophy-paper/           # manuscript model, Typst, exports
│   └── lean4losophy-cli/             # optional headless commands and fixtures
├── lean/
│   └── lean4losophy/                 # shipped Lean library
├── format/
│   └── FORMAT.md               # on-disk format spec
├── design/
│   ├── tokens.toml             # single source for app and paper tokens
│   └── theme.css               # GTK4/libadwaita implementation
├── docs/
│   ├── adr/
│   ├── lints/                  # one page per APORIA-xxxx rule
│   └── user/
├── lints/                      # ast-grep / semgrep rules
├── packaging/
│   ├── flatpak/
│   ├── nix/
│   └── aur/
├── tests/
│   ├── fixtures/
│   └── perf/
└── scripts/
```

Plan A would replace `crates/` with `app/` (Lean) plus `ffi/` (C shim), and keep `format/`, `docs/`, `packaging/`, and `tests/`.

## 6. Milestones

| ID | Outcome | Gate |
| --- | --- | --- |
| M0 | Foundations, format spec, architecture and packaging spikes | G0 architecture; G1 packaging |
| M1 | Notes shell: editor, preview, save, settings, onboarding | G2 notes shell |
| M2 | Links, backlinks, search | G3 prototype complete |
| M3 | Argument blocks and map | G4 arguments |
| M4 | Optional Lean, diagnostics, goals, skeleton | G5 Lean |
| M5 | Logic library, counterexamples, exercises, Guided mode | G6 learning |
| M6 | Papers, Typst, citations, exports | G7 papers |
| M7 | Updates, packaging, accessibility, docs, release | G8 v1 |

Staged builds are intentional: `0.1` after M1, `0.2` after M2, `0.3` after M3, `0.4` after M4, `0.5` after M5, `0.6` after M6, `1.0` after M7.

---

## 7. M0 — Foundations and gates

**Goal:** prove the architecture and packaging paths before writing product code.

### Tasks

- [x] Add Apache-2.0 `LICENSE`, `NOTICE`, SPDX headers, and a third-party notices file (`docs/THIRD-PARTY-NOTICES.md`).
- [x] Add dependency license checks (`deny.toml` for `cargo-deny`) and review GTK, WebKitGTK, Typst, Lean, Pandoc, and font licenses.
- [x] Create the workspace, formatter/linter config, and CI for build, test, and lint.
- [x] Write `LINT-POLICY.md` with stable `APORIA-xxxx` codes, rationale, alternatives, and suppression rules.
- [x] Add `docs/lints/` with an index, `APORIA-TEMPLATE.md`, and one page per enabled rule.
- [x] Add `scripts/lint` that runs all linters and enriches messages with the rule code and doc link, plus fixture tests for the custom rules and a design-token check.
- [ ] Make the CI `lint` job a required check. The workflow installs the gate's tools and fails on findings, but marking the job required is a repository-settings change that has not been applied here.
- [x] Add `AGENTS.md` and `CONTRIBUTING.md`: run the lint script, read the rule page, choose an alternative, never suppress without approval.
- [x] Write `FORMAT.md` v0.1: notebook layout, front matter, note IDs, link syntax, block IDs, argument grammar, settings split, cache layout, atomic writes, versioning.
- [x] Write ADRs: UI architecture, Markdown/preview stack, packaging/update model, data format, lint policy.
- [x] Freeze `DESIGN-PLAN.md` and `design/tokens.toml`; generate `theme.css`, `theme-dark.css`, and `paper-tokens.typ` with `scripts/gen-design-tokens`.
- [ ] Spike Plan A: Lean → GTK window, one callback, open/save. Code exists in `spikes/lean-ffi/`; the C half passes `make check-c` (and a compile error it found was fixed), but the Lean half is unbuilt because no toolchain was available.
- [x] Spike Plan B: `gtk4-rs` window, editor, open/save. Crate builds and the window launches under native Wayland; open/save and the sidecar handshake were not exercised (the handshake belongs to M4).
- [ ] Spike Lean: `lake serve` returns diagnostics for one generated block. Client and fixture exist in `spikes/lean-lsp/`; not executed without a toolchain.
- [ ] Spike packaging: minimal app in Flatpak, Nix, and AUR; test portal folder access and network permission. Manifests exist in `packaging/`; no channel has been built.
- [x] Review the spec and record the changes listed in section 4.

### Status

Delivered: license and notice files, workspace, CI, lint policy and pages, lint
script and rule fixtures, `FORMAT.md` v0.1, five ADRs, design tokens and
generated outputs, the Plan B spike, and the spec review.

Open, and the reason G0 and G1 are not recorded as passed:

- Plan A needs a Lean toolchain to build `Main.lean` and `lean_bridge.c`.
- The Lean LSP spike needs `lake serve` with the pinned toolchain.
- The packaging spike needs `flatpak-builder` plus a runtime, `nix`, and a clean
  AUR chroot. None are installed in the development environment.
- The CI required-check setting is a repository setting, not a file change.

`docs/adr/0001-ui-architecture.md` and `docs/adr/0003-packaging-update.md` carry
the evidence and the remaining checklist for G0 and G1.

### Acceptance

- A packaged minimal app opens a notebook, edits one note, and saves atomically. **Not met.**
- Flatpak runs on a clean system; Nix and AUR build from source. **Not met.**
- `FORMAT.md` has enough detail for another developer to write a parser. **Met by review; conformance fixtures are the next step.**
- G0 and G1 are recorded with a decision and a reason. **Open; both ADRs state the remaining evidence.**

---

## 8. M1 — Notes shell: editing

**Goal:** a fast, native notebook editor with no Lean dependency.

### Tasks

- [x] App shell: main window, sidebar, split panes, header bar, empty states.
- [x] Implement the design tokens from `DESIGN-PLAN.md`: color, type, spacing, elevation, and radius.
- [x] Apply the component rules where they exist in the shell: persistent field labels, specific button verbs, separated destructive actions, empty-state CTAs, tinted dark mode. The full visual checklist has not been reviewed against a running window.
- [x] Notebook open/create; directory chooser through the portal `FileDialog`; recent notebooks in the global config.
- [ ] File tree with flat notes and folders; create, rename, delete, and rename-to-move. A list with folder indentation exists; drag-to-move does not.
- [ ] Editor: plain `GtkTextView` with undo/redo in M1. GtkSourceView 5, CommonMark highlighting, line numbers, and find/replace are missing because the library is unavailable in the build environment (ADR-0002).
- [x] Preview: side-by-side and toggle layouts, CommonMark plus tables and footnotes, rendered from the shared comrak block model.
- [x] Preview security: no JavaScript and no remote resources by construction in the native renderer; external links are not actionable yet.
- [ ] Front matter parse/write and tag reading are done; the metadata panel and tag editing are not.
- [x] Autosave with debounce; temp-file, fsync, rename; crash recovery by construction.
- [x] External change detection and last-writer-wins behavior; non-blocking status warning.
- [x] Settings split: global config in XDG, notebook settings in `.aporia/settings.toml`, unknown keys preserved.
- [ ] Sample notes, Guided/Expert choice, and the per-notebook mode are in place. The dedicated first-run wizard step and the Lean offer flow are not; Lean is mentioned as optional text until M4.
- [ ] Home screen has create/open and recent notebooks. Recent notes, quick capture, and the search entry wait for M2.
- [ ] Shortcuts exist for new note, open/create notebook, toggle preview, layout, mode, and home. Focus-search waits for M2.

### Status

Implemented and tested in the core: front matter round-trip with unknown keys,
settings precedence, atomic saves, path-traversal rejection, note listing,
create/rename/delete, and the preview block model (heading, paragraph, list,
code, quote, table, rule, footnotes, HTML dropped). App flows verified by a
smoke launch under native Wayland: the window stays up, loads a recent notebook,
and rewrites global config without error.

Not yet verified: cold-start and typing-latency targets, the visual design
checklist in light and dark mode, crash-during-save recovery (the write path is
atomic but no kill test was run), and the preview's local-image and link paths.
These are the M1 exit items before G2. Per-screen implementation and rule
status are tracked in `docs/UI-MAP.md`.

### Acceptance

- Cold start under 1 s without Lean. **Not measured.**
- Typing latency under 16 ms. **Not measured.**
- A 10,000-note notebook opens and saves without blocking. **Listing measured at 0.03 s** (ignored performance fixture); save behavior under 10,000 notes is unmeasured.
- Killing the app mid-save does not corrupt the note. **Design meets it; no kill test yet.**
- Both preview layouts work and persist the choice. **Implemented; persistence is exercised by the settings tests.**
- Guided/Expert choice is stored per notebook. **Implemented.**
- The `DESIGN-PLAN.md` checklist passes for the shell, forms, tables, and empty states in light and dark mode. **Not reviewed visually.**

---

## 9. M2 — Links, backlinks, search

**Goal:** complete the notes-shell prototype chosen in Q1.

### Tasks

- [x] Parse `[[Note]]`, `[[Note|alias]]`, `[[Note#Heading]]`, and `[[Note#^block]]` from the comrak AST; code spans, fenced blocks, and `![[...]]` never become links.
- [x] Resolve by stable ID, title, alias, then path; ambiguous and broken links are reported with candidates and never resolved silently.
- [x] Create a note from a broken link after confirmation; rename rewrites incoming links that resolved by path and leaves title/alias links alone.
- [x] Block IDs (`^id`) on paragraphs, list items, quotes, tables, and Lean blocks; heading anchors normalized per `FORMAT.md` 5.4 with duplicate suffixes. Both are indexed with lines and masked out of the preview.
- [x] Backlinks pane with source title, line, and context; click opens the source at the line.
- [x] SQLite schema: notes, aliases, tags, headings, blocks, links, FTS5 index; `user_version` check and rebuild on mismatch. `headings.line` was added to the documented cache schema for navigation.
- [x] Incremental indexing on save and external change; full rebuild when the cache is missing or stale, written to `cache.sqlite.tmp` and renamed.
- [x] Search: FTS5 `bm25` with title weight, snippets, tag and folder filters, Enter or row click to open, Ctrl+F / Ctrl+K to focus.
- [x] Index status and rebuild command (menu → Notebook index…), including the broken/ambiguous link list. It is a dialog, not inside a Preferences window.
- [x] Performance fixture with 10,000 generated notes, now covering index build and search.

### Status

Measured on the 10,000-note fixture: full index build 3.39 s and search
11.01 ms, against the 100 ms search target. The core has 8 index integration
tests and 8 unit tests for link parsing, anchors, and block kinds; the earlier
front-matter, notebook, and preview suites still pass.

Indexing happens on the UI thread. Search is debounced and measured fast, but a
full 10k rebuild blocks the window for about 3.4 s; moving indexing to a worker
belongs to the M7 performance pass. Broken links are reported in the index
dialog, not with an inline editor marker. The search list supports Enter and
click; it has no custom arrow-key handler beyond normal GTK focus movement.

### Acceptance

- All four link forms work; aliases and block references survive a restart. **Met** (cache reopen is tested).
- Backlinks are correct after an in-app rename. **Met**; path-resolved links are rewritten and title/alias links keep resolving.
- Search over 10,000 notes returns in under 100 ms. **Met: 11.01 ms measured.**
- Deleting `.aporia/cache.sqlite` loses no data and rebuilds. **Met.**
- The prototype is demoable end to end: create, link, search, reopen. **The window launches with a linked notebook and builds its cache; the click-through itself was not exercised in this environment.**

**Gate G3:** decide whether to continue to arguments or revise the notes model first.

---

## 10. M3 — Arguments and map

**Goal:** structured arguments that work without Lean.

### Tasks

- [ ] Freeze the argument grammar in `FORMAT.md`; add golden parser tests.
- [ ] Model premises, conclusion, stable argument ID, and optional title.
- [ ] Editor actions: insert argument, add/reorder premises, edit conclusion, convert to/from a plain list.
- [ ] Preview card: numbered premises, inference line, `∴` conclusion, status badge.
- [ ] Argument map: read-only auto-layout first; nodes, support/objection edges, pan, zoom.
- [ ] Map accessibility fallback: a keyboard-navigable list view.
- [ ] Status values: unformalized, formalized, checked; no Lean required for the first two.
- [ ] Validation errors for malformed arguments, shown inline and in the map.

### Acceptance

- A user can create, edit, reorder, and delete an argument without seeing Lean.
- The map updates after each edit and survives restart.
- Malformed arguments produce a clear error, not a crash.
- No premise-type tags, manual fallacy annotations, or SVG export in v1.

**Gate G4:** arguments are stable enough to generate Lean from.

---

## 11. M4 — Optional Lean

**Goal:** formal checking for users who install Lean, with no effect on users who do not.

### Tasks

- [ ] Lean manager: detect an existing toolchain; offer a pinned download with size, version, source, checksum, progress, cancel, retry, remove.
- [ ] Support a local Lean archive for offline install.
- [ ] Store toolchains under the app data directory; show disk usage; never install system-wide.
- [ ] Per-notebook Lake project: `lean/lean-toolchain`, `lakefile.lean`, `lean4losophy/` library, version marker.
- [ ] Copy the bundled Lean library into the notebook without overwriting user edits.
- [ ] LSP client: start `lake serve`, initialize, open/change documents, subscribe to diagnostics, request goals.
- [ ] Extract Lean blocks from Markdown into generated files; keep a line/column map back to the note.
- [ ] Debounce 300 ms; cancel stale requests; never block typing.
- [ ] Ctrl+Enter checks the current block; inline diagnostics; goal view at the cursor.
- [ ] Theorem skeleton from an argument: premises as hypotheses, conclusion as goal.
- [ ] `by_valid` experiment: verify `simp`, `omega`, `tauto`, and `aesop` availability; vendor or replace what is missing.
- [ ] `sorry` detection and badge transitions; document that “checked” means valid, not true.
- [ ] Plain-language catalog for the ten most common Lean errors; raw message one click away.
- [ ] Lean sandbox: no network, limited filesystem, CPU and memory limits; trust prompt for notebooks from elsewhere.
- [ ] Graceful behavior when Lean is absent, removed, or crashes.

### Acceptance

- Diagnostics appear within 1 s after a 300 ms pause.
- The UI stays responsive while Lean compiles.
- Removing Lean leaves notes, arguments, search, and papers fully usable.
- A generated skeleton uses the argument’s premises and conclusion.
- Guided mode never shows one of the ten catalogued errors as raw text alone.
- Lean code from an untrusted notebook cannot read the home directory or reach the network.

**Gate G5:** decide the Lean version policy and whether to keep the sandbox design.

---

## 12. M5 — Learning features

**Goal:** the learner path from 17.3, without requiring Lean for basic validity checks.

### Tasks

- [ ] Propositional counterexample finder: truth tables for up to six letters; Valid, Not valid, or Can’t tell; plain-language model; under 2 s.
- [ ] `Lean4losophy.Core` Lean library: propositional and first-order helpers, readable notation, `Argument`, `by_valid`, tests, docs.
- [ ] Exercise engine: three sets (translation, proof step, fallacy); YAML content; hints; grading.
- [ ] Save progress in the notebook; survive restart.
- [ ] Guided mode: hints, simplified Lean view, error explanations; Expert mode hides them.
- [ ] Mode switch at any time; setting remains per notebook.
- [ ] Sample notebook content that teaches one full argument.
- [ ] Tests for the 16.1 acceptance criteria (L1–L5 where applicable).

### Acceptance

- A six-letter propositional argument returns a result and a counterexample in plain words.
- Progress survives restart and moves with the notebook.
- Guided mode is useful with Lean absent; Lean-only exercises are labelled.
- Expert mode has no hints or tutorial overlays.

**Gate G6:** validate the learner flow with at least three people.

---

## 13. M6 — Papers, Typst, citations, exports

**Goal:** a paper project that produces a submission-quality PDF and a DOCX.

### Tasks

- [ ] Paper model: `papers/<name>/paper.md` plus metadata (title, authors, abstract, keywords, language, date).
- [ ] Outline view; section reorder; word count; per-section navigation.
- [ ] Insert a note or section with a link back; no live transclusion in v1.
- [ ] Two Typst templates: student essay and journal article; page size, margins, headings, title block.
- [ ] Apply the paper rules from `DESIGN-PLAN.md`: left-aligned long text, regular body weight, tight heading leading, more space above headings, right-aligned numeric columns, and baseline-aligned mixed sizes.
- [ ] Markdown → Typst converter for the supported subset; footnotes, tables, quotes, math, argument blocks.
- [ ] Logical notation mapping; proper math fonts; `∴` layout.
- [ ] Typst compile worker; live PDF preview; page follows cursor; debounce; never block typing.
- [ ] BibTeX import; CSL style selection; citation links back to note and page.
- [ ] Pre-submission checks: missing citations and unresolved cross-references.
- [ ] Exports: PDF via Typst, Markdown, DOCX.
- [ ] DOCX decision: bundle Pandoc with GPL notices, depend on host Pandoc, or write a native exporter.
- [ ] Provisional paper editor placement: same-window mode; revisit if the preview needs more space.

### Acceptance

- A 20-page paper preview updates in under 2 s after the last edit.
- A 50-page paper with bibliography exports in under 10 s.
- Citations and bibliography are correct in PDF and DOCX.
- Missing citations and broken references are reported before export.
- The paper templates pass the applicable `DESIGN-PLAN.md` rules in the rendered PDF.
- The paper builds from plain files in the notebook.

**Gate G7:** decide the DOCX path and citation default before release.

---

## 14. M7 — Release

**Goal:** installable, updatable, documented v1.0.

### Tasks

- [ ] Update checker on launch: GitHub release feed; disable switch; no identifiers.
- [ ] Channel actions: Flatpak opens the store; Nix and AUR show the exact upgrade command.
- [ ] Flatpak: Flathub submission after the name is fixed; `.flatpakref` for direct install; screenshots and appdata.
- [ ] Nix flake and package; AUR `PKGBUILD`; CI builds for all three.
- [ ] Accessibility pass: keyboard navigation, AT-SPI labels, Orca test, high contrast, map list fallback.
- [ ] Documentation: README, user guide, `FORMAT.md`, ADRs, troubleshooting, privacy statement.
- [ ] Performance verification: 1 s start, 16 ms typing, 100 ms search over 10,000 notes, under 300 MB excluding Lean.
- [ ] Security review: network allowlist, Lean sandbox, WebKit hardening, dependency audit.
- [ ] Verify zero lint errors, every suppression is approved and documented, and every enabled rule has a docs page.
- [ ] Release automation: tag, checksums, release notes, Flatpak bundle, Nix/AUR updates.
- [ ] v1.0 checklist against `SPEC.md` sections 16 and 9.

### Acceptance

- A non-technical user installs from Flathub or a `.flatpakref` without a terminal.
- A power user installs through Nix or AUR and sees the correct update path.
- No network call occurs without consent except the documented update check.
- All v1 features work with Lean absent; Lean features work after the consented download.

**Gate G8:** v1.0.

---

## 15. Format work (`FORMAT.md`)

This is a P0 deliverable and is frozen incrementally. It must cover:

- Notebook layout: `notes/`, `refs/`, `lean/`, `.aporia/`, `papers/`.
- Note front matter: `format`, `id` (ULID), `title`, `aliases`, `tags`, `created`, `updated`; unknown keys preserved.
- Note identity: stable ID first, title/alias/path fallback.
- Links: `[[Note]]`, `[[Note|alias]]`, `[[Note#Heading]]`, `[[Note#^block]]`.
- Block IDs: trailing `^id` on a block.
- Argument grammar (see below).
- Settings: global `config.toml` and notebook `.aporia/settings.toml`.
- Cache: `.aporia/cache.sqlite`, `PRAGMA user_version`, rebuild rules.
- Writes: temp file, fsync, atomic rename; autosave debounce.
- External changes: last writer wins; clean buffer reloads; dirty buffer warns.
- Encoding: UTF-8, LF; invalid UTF-8 rejected with a clear error.
- Versioning: `format: 1`; migrations are explicit and tested.

### Proposed argument grammar

The Markdown AST must contain a list item whose first paragraph begins with `Argument`. The nested ordered list is the body. The item whose text begins with `∴` is the conclusion; the others are premises.

```markdown
- Argument: The mortality argument {#arg-01J8Z9}
  1. All humans are mortal.
  2. Socrates is human.
  3. ∴ Socrates is mortal.
```

A list without the `Argument` marker is an ordinary list. The parser reports duplicate conclusions, missing conclusions, and empty premises. The grammar is frozen in M3 with golden fixtures.

### Preview

- Parse CommonMark plus tables and footnotes.
- Render sanitized HTML; JavaScript off; no remote images or fonts.
- Wiki links become internal navigation targets.
- Lean blocks and argument blocks render as cards with status.
- Both layouts use the same renderer.

## 16. Testing and quality

- Unit tests: parser, link resolution, argument grammar, settings, atomic writes, update comparison.
- Golden tests: Markdown → HTML, argument → Lean skeleton, paper → Typst, exports.
- Property tests: link resolution and rename; fuzz the parser.
- Integration tests: create/open/save, rename, search, external edit, crash recovery.
- UI smoke tests under a headless compositor; manual Orca pass for accessibility.
- Lean tests: mock LSP for UI; real pinned Lean nightly for diagnostics and goals.
- Visual regression tests: light and dark UI snapshots, empty states, forms, tables, and exported paper pages.
- Performance tests: generated 10,000-note notebook; startup, typing, search, paper compile.
- Packaging tests: Flatpak install/run in a VM; Nix build; AUR build in a container.
- CI: build, lint, unit, golden, packaging; nightly performance and Lean integration.

Quality gates per milestone: tests green, acceptance criteria met, docs updated, design checklist passed, no new network calls without review.

### Lint and code-quality gate

Lint findings are errors, not warnings. `scripts/lint` is the single entry point and is required in CI and before review.

It runs:

- `rustfmt --check` and Clippy with project lints denied.
- `cargo-deny` for licenses, advisories, banned crates, and sources.
- `cargo-audit` and secret scanning.
- `ast-grep` custom rules for project-specific patterns.
- `typos`, `markdownlint`, `shellcheck`, and `actionlint`.
- Lean build warnings-as-errors plus a scan of `lean/Lean4losophy` for banned constructs.
- A suppression audit that rejects bare `#[allow]` and undocumented exceptions.

Every rule has a stable `APORIA-xxxx` code and a page in `docs/lints/` explaining what it catches, why it matters, the alternatives, and the exception process. Tool output that cannot emit the code is wrapped by `scripts/lint` so the final message still includes the code and doc link.

The policy and initial rule set are in `LINT-POLICY.md`.

## 17. Packaging, updates, privacy

### Install

- **Flatpak:** Flathub and a direct `.flatpakref`; desktop app center for non-technical users.
- **Nix:** flake with a package and dev shell; `nix run` and profile install.
- **AUR:** source `PKGBUILD`; `lean4` remains optional.
- Minimum floor: the GTK4/libadwaita versions in the chosen Flatpak runtime; native builds tested on current Ubuntu LTS, Fedora, and Arch.

### Updates

- One update checker, started on launch if enabled in onboarding.
- Flatpak: open the software center; never self-replace the Flatpak.
- Nix/AUR: show the exact upgrade command; no silent replacement.
- No self-updating binary in v1; this is a deliberate scope decision from Q13.
- All checks are unauthenticated GETs to a release feed; no identifiers.

### Privacy and security

- No account, telemetry, analytics, or crash upload.
- Network allowlist: update feed and consented Lean download only.
- Preview blocks remote content; external links leave the app.
- Lean runs sandboxed with no network and limited filesystem access; untrusted notebooks require confirmation.
- Logs are local; no note content leaves the machine.
- Flatpak permissions are documented; network permission is disclosed because of the update check and Lean download.

### License compliance

- Apache-2.0 for the app; `NOTICE` and third-party notices.
- Audit GTK, libadwaita, GtkSourceView, WebKitGTK, SQLite, Typst, Lean, Pandoc, and fonts.
- Pandoc is GPL-2.0-or-later. Do not bundle it into the Flatpak without a compliance decision; this is why M6 has an explicit DOCX task.

## 18. Risks

| Risk | Impact | Mitigation | Gate |
| --- | --- | --- | --- |
| Lean + GTK FFI is harder than expected | Critical | Plan B default; timeboxed Plan A spike | G0 |
| Lean app cannot be packaged cleanly | High | Prove Flatpak/Nix/AUR in P0; Lean downloaded at runtime | G1 |
| Lean LSP behavior changes between versions | High | Pin a version; golden tests; isolate the client | G5 |
| v1 scope is too large to land in one pass | High | Staged releases; scope guard; cut list | Every gate |
| CommonMark lacks required tables/footnotes | Medium | Document the extension set; test against fixtures | M1 |
| WebKit preview is heavy or unsafe | Medium | JavaScript off, no remote content, CSP; native renderer fallback | M1 |
| Last-writer-wins loses work | High | Atomic writes, external-change warning, optional local history | M1 |
| Counterexample/`by_valid` tactics are not in Lean core | Medium | Verify in M4; vendor permissive dependencies or implement locally | M4 |
| Lean code from an untrusted notebook executes | Critical | Sandbox, no network, resource limits, trust prompt | M4 |
| Pandoc licensing/size blocks DOCX | Medium | Decide bundle vs host vs native exporter | M6 |
| Flathub submission | Medium | Complete the G1 packaging builds before M7 | M7 |
| Update check conflicts with “offline by default” | Medium | Consent toggle; single allowlisted call; document it | M7 |
| Apache-2.0 incompatible with a dependency | Medium | License audit in P0; replace or relicense before code depends on it | G1 |
| Lint rules are noisy or fight real code | Medium | Tune thresholds and document exceptions; never disable a rule silently | G1 |
| Custom lint rules rot or overfit | Low | Keep the set small, test each rule, review quarterly | M7 |
| Agents bypass lint with `#[allow]` | High | CI gate, suppression audit, required reason and code | G1 |
| App and paper design drift apart | Medium | One token source, generated CSS/Typst, visual regression tests | M6 |

## 19. Initial bootstrap

### Bootstrap A

- [ ] Add `LICENSE` (Apache-2.0), `NOTICE`, SPDX headers, and dependency-license config.
- [ ] Create the workspace and CI skeleton.
- [ ] Draft `LINT-POLICY.md`, `docs/lints/`, and `scripts/lint`; wire the CI lint job.
- [ ] Draft `FORMAT.md` v0.1 with the note, link, argument, settings, and cache sections.
- [ ] Write ADR-0001 (UI architecture) and ADR-0002 (Markdown/preview).
- [ ] Run the Plan B spike: window, editor, open/save.
- [ ] Run the Plan A spike: Lean window, callback, open/save. Timebox three days.

### Bootstrap B

- [ ] Run the Lean LSP spike against one generated block.
- [ ] Build the winning spike as a Flatpak; test portal folder access and network permission.
- [ ] Add a Nix flake and an AUR `PKGBUILD` skeleton; build both.
- [ ] Hold Gate G0 and G1; record the decisions.
- [ ] Update `SPEC.md` with the v1 scope, architecture, update model, privacy rules, license, and `FORMAT.md` references.
- [ ] Start M1 only after G0 and G1 pass.

## 20. Open decisions

These do not block P0 but must be closed by the stated gate:

- Architecture: Plan A or B — G0.
- Flathub submission — after the G1 packaging builds, in M7.
- Default citation style — M6.
- DOCX path: bundled Pandoc, host Pandoc, or native exporter — M6.
- Paper editor placement — M6.
- Lean version policy and update cadence — M5.
- Typst version and template API — M6.
- Accessibility target — M7.
- Localization — post-v1 unless a translator joins.
- Import from Obsidian/Markdown — post-v1.
- Confirm the ambiguous design interpretations in `DESIGN-PLAN.md` section 9 — M1.

## 21. Critical path

`G0 → M1 → M2 → M3 → M4 → M5 → M6 → M7`

The two items most likely to change the critical path are the architecture decision (G0) and Lean integration (M4). Packaging (G1) is the most likely non-code blocker. Paper/DOCX work (M6) is the largest single feature block and the best candidate for a staged `0.6` release before `1.0`.

---

*This plan follows the answers in `OPEN-QUESTIONS.md`. If an answer changes, revise the affected milestone before writing code.*
