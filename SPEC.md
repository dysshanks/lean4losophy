# Philosophy Notebook — Spec Sheet

**Working title:** Aporia (placeholder)
**Target:** Linux desktop, Wayland-first (X11 via XWayland as fallback only)
**Core languages:** Rust (application) and Lean 4 (logic library and proof checking; see Decisions)
**Version:** 0.1 draft

## Decisions

These are settled inputs. The linked documents carry the detail; anything not listed here stays open until its gate.

- **v1 scope:** section 17.3's **In** list. Everything else is post-v1.
- **No account, no login, no telemetry.** Offline by default.
- **License:** Apache-2.0.
- **Name:** deferred; "Aporia" is a working title until the name is fixed before Flathub submission.
- **Architecture:** Plan B (Rust GTK4 UI with Lean as an optional sidecar over LSP) is recommended; Plan A (Lean with a C FFI to GTK) is the alternative. Decided at Gate G0; see section 5 and `docs/adr/0001-ui-architecture.md`.
- **Data format:** `FORMAT.md` is the source of truth for the on-disk format; section 7 is a summary.
- **Project sources:** `DESIGN-PLAN.md` defines the visual and paper rules; `LINT-POLICY.md` defines the `APORIA-xxxx` lint gate. Both are binding.

## 1. Summary

A local-first notebook for philosophical thinking. Notes are written in Markdown; arguments can be written as structured premises and conclusions and, optionally, formalized and checked in Lean 4. The goal is to make rigorous reasoning a normal part of note-taking, not a separate tool.

## 2. Goals and Non-Goals

### Goals

- Fast, distraction-free note writing with Markdown.
- Structured arguments (premise → conclusion) with optional Lean 4 formalization and machine-checked validity.
- Linked notes (backlinks, concept pages, citations).
- Native Wayland behavior: correct scaling, IME, clipboard, portals.
- Fully offline, plain-file storage the user owns.

### Non-goals (v1)

- Cloud sync or multi-user collaboration.
- Proving philosophical truth. Lean checks logical validity relative to the formalization, not whether the premises are correct.
- Mobile or Windows/macOS builds.

## 3. Target Users

The app is built for three groups: people who are learning logic and analytical philosophy, people who already know it, and people who pursue philosophy as a hobby. Users outside these groups can use it, but design decisions are judged against these three.

- **Learners** (students and self-learners in logic and analytical philosophy). Need guidance, not assumed Lean knowledge: step-by-step translation of English arguments into symbols, plain-language explanations of Lean errors, a glossary of core terms (validity vs soundness, modus ponens, quantifier scope), and worked example arguments they can edit.
- **Experienced users** (people who already know logic and analytical philosophy: advanced students, teachers, researchers). Need speed and rigor: full access to Lean, custom notation and definitions, modal and deontic libraries, keyboard-first editing, and paper export, without hints in the way.
- **Hobbyists** (people who read and think about philosophy for enjoyment, with or without formal training). Need a low barrier: the app works as a plain note-taking and argument-mapping tool with Lean fully optional, readable defaults, and an easy path from a casual note to a structured argument when they want one.

**Design consequence:** one app with a Guided mode (hints, explanations, exercises, simplified Lean view) and an Expert mode (no hints, full Lean access, compact layout). The user can switch at any time, and the choice is per notebook. Example content and the bundled library focus on topics in analytical philosophy such as epistemology, metaphysics, philosophy of language, and philosophy of mind.

## 4. Functional Requirements

### 4.1 Notes

- Markdown editor with live preview, headings, footnotes, quotes, and tables.
- Wiki-style links `[[Concept]]` and automatic backlinks.
- Tags and a per-note metadata header (author, source, date, topic).
- Full-text search across all notes.

### 4.2 Argument Blocks

- A block type for arguments: numbered premises, inference markers, and conclusion.
- Premise type tags (assumption, empirical claim, definition, contested) are
  post-v1.
- Argument map view: read-only automatic layout in v1, with nodes for claims and
  edges for support and objection. Interactive editing of the map is post-v1.
- Manual fallacy and objection annotations are post-v1; the exercise set covers
  fallacies instead.

### 4.3 Lean 4 Integration

- Embedded lean code blocks inside notes, compiled as part of a per-notebook Lake project.
- Inline diagnostics (errors, goals, sorry warnings) shown beside the block.
- Argument blocks can be converted to a Lean theorem skeleton: premises become hypotheses, conclusion becomes the goal.
- Status badge per argument: unformalized, formalized (has sorry), checked.
- Bundled logic library (see section 6).

### 4.4 Sources and Citations

- BibTeX/CSL-JSON import.
- Cite with `@key`; quotes link back to source and page.
- Bibliography generated on export.

### 4.5 Concept Glossary

- Each concept gets a page with definitions by different authors, side by side.
- Optional Lean definition for a concept, reusable across arguments.

### 4.6 Export

- Markdown (default), PDF, and DOCX. Section 14.6 is authoritative for the v1 export set.
- Argument maps export as SVG (post-v1).

## 5. Architecture

Important constraint: Lean 4 has no mature GUI toolkit. The design keeps Lean where it is strong (logic, checking, proof checking) and uses a standard toolkit for the window.

**Decision pending Gate G0.** Plan B is recommended: a Rust GTK4 + libadwaita application, with Lean as an optional sidecar over LSP. Plan A — a Lean application that calls GTK through a thin C FFI shim — remains the alternative. `docs/adr/0001-ui-architecture.md` records the criteria; G0 chooses on measured spike results. The table describes Plan B; the Plan A differences follow it.

| Layer | Choice | Notes |
| --- | --- | --- |
| Application and notebook core | Rust library (`aporia-core`) | Parser for note files, argument model, link index, search, settings |
| Logic library | Lean 4 library (`Aporia.Core`), shipped as source | Propositional and first-order helpers, theorem skeleton generator |
| Proof checking | Lean 4 sidecar (`lake serve`, LSP) | Optional; one long-lived process per notebook, started by the app when Lean is installed |
| UI | GTK4 + libadwaita in Rust (`gtk4-rs`), native Wayland backend | No Lean runtime in the UI process |
| Editor widget | GtkSourceView 5 | Markdown highlighting, Lean highlighting via a custom language definition |
| Index and search | SQLite (FTS5) | Rebuildable cache, never the source of truth |
| Argument map | Custom drawing area (Cairo/GSK) | Pan, zoom, drag nodes |

**Plan A differs in:** the application core and UI are Lean; GTK is reached through a thin C FFI shim (`@[extern]`); the notebook model lives in the Lean binary; and the Lean library is linked rather than shipped as source. It adds an FFI hardening track to M1–M3 and the memory-ownership risk named in section 12. It may be preferable if the Lean core is the point of the project.

### Process model

1. App starts, opens the notebook folder.
2. Core library loads and indexes notes.
3. If Lean is installed, the sidecar starts in the background; diagnostics stream into the UI asynchronously so typing never blocks.
4. Without Lean, step 3 is skipped and every non-Lean feature stays available.

## 6. Lean 4 Library Scope

- Propositional and first-order logic helpers with readable notation. This is
  the v1 library scope.
- Modal logic via a shallow embedding (possible-worlds semantics): necessity,
  possibility, and common axiom systems (K, T, S4, S5) as typeclasses. Post-v1;
  the v1 **In** list in 17.3 is propositional and first-order only.
- Deontic and epistemic operators as optional modules. Post-v1.
- Argument structure: `structure Argument` with premises, conclusion, and metadata, plus a tactic wrapper such as `by_valid` that tries `simp`, `omega`, `tauto`, and `aesop`.
- Dependency: Lean 4 core only at first; Mathlib optional because of its size and build time.

## 7. Data Model and Storage

The on-disk format is specified in `FORMAT.md`; this section is a summary. A format change requires a version and a migration note (ADR-0004).

- A notebook is a folder: `notes/*.md`, `refs/library.bib`, `papers/`, `lean/` (Lake project), `.aporia/` (cache and notebook settings).
- Notes are plain Markdown with YAML front matter; Lean blocks use `lean` code fences.
- Note identity is hybrid: a stable ULID in front matter, with title/alias/path as fallback. Unknown front-matter keys are preserved.
- Settings are split: global preferences live in the XDG config directory (`config.toml`); notebook preferences live in `.aporia/settings.toml` and travel with the folder.
- External edits and multiple instances use last-writer-wins, with a non-blocking warning when the buffer is dirty. There is no merge UI in v1.
- Everything is diffable and works with Git. The SQLite cache can be deleted at any time.
- Autosave with atomic writes (write temp file, fsync, then rename).

## 8. Wayland and Linux Requirements

- Native Wayland through GTK4; no hard X11 dependency. Test with `GDK_BACKEND=wayland` as the default.
- Correct fractional scaling and HiDPI rendering.
- Input methods through text-input-v3 (important for non-Latin philosophy terms, Greek, and Sanskrit).
- Clipboard through the Wayland data device; drag and drop for importing files.
- File open/save through xdg-desktop-portal so it works sandboxed.
- No global shortcuts or window positioning assumptions (Wayland forbids them).
- Dark/light follows the desktop color-scheme setting.
- Distribution: Flatpak (primary, Flathub and `.flatpakref`), plus a Nix flake and AUR package. Tested on GNOME and KDE Plasma, and on Sway/Hyprland for tiling compositors.
- Updates: one checker on launch when enabled at first run. Flatpak is store-managed; Nix and AUR show the exact upgrade command. The app never replaces its own binary. See ADR-0003.

## 9. Non-Functional Requirements

| Area | Target |
| --- | --- |
| Cold start (without Lean server) | under 1 s |
| Typing latency | under 16 ms per keystroke |
| Notebook size | 10,000 notes searchable in under 100 ms |
| Lean diagnostics | asynchronous, debounced 300 ms after last edit |
| Memory | under 300 MB excluding the Lean server |
| Privacy | no account, telemetry, analytics, or crash upload; offline by default |
| Network | allowlist: update feed and consented Lean download only; no other outbound calls |
| Accessibility | full keyboard navigation, screen reader labels through AT-SPI |

## 10. Keyboard-First UX

- Command palette (Ctrl+K) for notes, concepts, and actions.
- Ctrl+Enter checks the current Lean block.
- Ctrl+Shift+A inserts an argument block.
- Vim keybinding mode only if GtkSourceView provides it at no extra cost;
  otherwise post-v1.

## 11. Milestones

The milestone list, gates, and acceptance criteria live in
`IMPLEMENTATION-PLAN.md` section 6. That plan supersedes the earlier five-step
list in this document: M0 foundations and spikes, M1 notes shell, M2 links and
search, M3 arguments, M4 optional Lean, M5 learning features, M6 papers, M7
release. Product code starts only after Gate G0 and G1 pass.

## 12. Risks

- **GUI through Lean FFI** is the biggest risk if Plan A is chosen: memory
  ownership between Lean's runtime and GTK needs careful handling. Plan B
  removes it by keeping the UI in Rust. Mitigation for Plan A: keep the FFI
  surface tiny and prototype first (the M0 spike does this).
- **Lean build times** can make feedback slow. Mitigation: avoid Mathlib by default and cache the Lake build.
- **Formalization is hard for beginners.** Mitigation: Lean is optional; the app is useful as a plain notebook first.
- **False confidence:** a green check can be mistaken for a sound argument. The UI must state clearly that it means valid, not true.

## 13. Open Questions

**Resolved:**

- **Mathlib:** not in v1. The library uses Lean core; an opt-in Mathlib module is post-v1.
- **Argument block syntax:** a plain Markdown list convention, not a custom fenced syntax. The grammar is frozen in `FORMAT.md`.
- **License:** Apache-2.0.

**Open:**

- **Architecture:** Plan A (Lean with a C FFI to GTK) or Plan B (Rust GTK4 UI with a Lean sidecar). Plan B is recommended; decided at Gate G0. See `docs/adr/0001-ui-architecture.md`.
- **Name and app ID:** deferred until before Flathub submission; "Aporia" is a working title.

## 14. Paper Composer and Professional Typesetting (added)

The notebook must be able to turn notes and arguments into a complete, publication-quality paper, not just exports of single notes. This section extends goals 4.6 (Export) and milestones in section 11.

### 14.1 Paper Projects

- A paper is a first-class object: a manuscript file (`papers/<name>/paper.md`) plus metadata (title, authors, affiliation, abstract, keywords, date, language).
- Structure: title block, abstract, numbered sections and subsections, footnotes, appendices, bibliography.
- Transclusion is post-v1. In v1 an inserted note keeps a link back to its
  source; there is no live `![[...]]` syntax.
- Outline view: reorder and navigate sections in v1. Per-section word targets are
  post-v1.
- Draft snapshots with a diff UI are post-v1; Git already covers history for
  users who want it.

### 14.2 Typesetting Engine

- Primary engine: Typst (single binary, fast incremental compiles, good typography, scriptable templates). LaTeX via Pandoc is not a v1 pipeline; `.tex` source export for journals that require it is post-v1.
- Live PDF preview beside the editor, rendered in-app (poppler or pdfium widget), updated on a short debounce; page position follows the cursor.
- Typographic quality: proper kerning and ligatures, hyphenation (English, Dutch, German, Greek), widow and orphan control, left-aligned text with a ragged right edge, optical margins where the engine supports them. Visual rules are in `DESIGN-PLAN.md`.
- Fonts: bundle open fonts suited to scholarly work (e.g. Libertinus, EB Garamond, Source Serif); the user can pick any installed font, with full Unicode and polytonic Greek support.

### 14.3 Templates

- Two built-in templates in v1: student essay and journal article. Thesis chapter, conference paper, book review, and lecture handout are post-v1.
- Template settings: page size (A4 default, US Letter), margins, line spacing, heading styles, running headers, page numbers, title page, table of contents.
- Users can add their own templates as plain Typst files in the notebook; templates are versioned with the paper.
- Blind-review mode is post-v1.

### 14.4 Citations and Cross-References

- Citation styles through CSL: Chicago (notes-bibliography and author-date), APA 7, MLA, Harvard, and Oxford footnote style, plus any CSL file the user adds.
- Footnotes or endnotes (user's choice), with citation links pointing back to the note and page of the source.
- Automatic numbering and cross-references for sections, arguments (P1, P2, ∴ C), figures, and tables.
- An index of terms and a glossary built from concept pages are post-v1.

### 14.5 Formal Logic in Papers

- Argument blocks render as clean numbered premise-and-conclusion layouts, with the inference line and the ∴ symbol.
- Logical and modal notation (∀, ∃, →, ¬, □, ◇, and so on) is typeset with proper math fonts; the same Lean notation maps to Typst math automatically.
- Optional appendix that includes the checked Lean source for each formalized argument, with a status note (checked, contains sorry). The paper never claims truth from a green check, only validity.
- Embedding argument maps as SVG in a paper is post-v1.

### 14.6 Export and Submission

- v1 exports: PDF (default, via Typst), Markdown, and DOCX (via Pandoc when it is installed). EPUB is cut from v1. PDF/A, HTML, and `.tex` source are post-v1; LaTeX is not a second pipeline.
- Pre-submission checks: missing citations, unresolved cross-references, broken links, word and page limits, unreferenced bibliography entries, unformalized arguments flagged as warnings.
- Reproducible builds: the same notebook and template always produce the same PDF; compile settings are saved in the paper's metadata.

### 14.7 Requirements

| Area | Target |
| --- | --- |
| Preview update (20-page paper) | under 2 s after last edit |
| Full export | under 10 s for a 50-page paper with bibliography |
| Dependencies | Typst bundled; Pandoc detected if installed and never bundled; the DOCX path (host Pandoc, bundled Pandoc with GPL notices, or a native exporter) is decided in M6; LaTeX is never required |
| Output quality | Embedded fonts; tagged PDF for accessibility where the engine supports it. PDF/A is post-v1. |

### 14.8 Milestones Added

Paper work is milestone M6 in `IMPLEMENTATION-PLAN.md`. Its v1 slice is: paper
projects, two templates (student essay, journal article), outline navigation,
Markdown to Typst conversion, live PDF preview, BibTeX with a chosen CSL style,
missing-citation and broken-reference checks, and PDF, Markdown, and DOCX
export. Transclusion, blind-review mode, PDF/A, `.tex` export, extra templates,
and embedded argument maps are post-v1.

### 14.9 Risks and Open Questions

- Typst is young; if a journal requires `.tex`, the Pandoc/LaTeX path needs equal testing. Decision after milestone 6.
- Matching a specific journal's house style may need custom templates; ship a few and document how to write more.
- Live preview must never block typing; compilation runs in a separate process.
- Should the paper editor be a separate window from the notebook, or a mode in the same window?

## 15. The Three Users: How They Think and Work

The three groups want different things from the same app. These personas are reasoned from the target groups in section 3, not from interviews; section 17.4 says how to check them.

|  | Learner | Experienced user | Hobbyist |
| --- | --- | --- | --- |
| Typical person | Student in a logic or philosophy course, or self-taught | Grad student, teacher, researcher, advanced undergraduate | Reads books, podcasts, and essays for enjoyment |
| Why they open the app | Understand how arguments map to logic; finish coursework | Reconstruct and test arguments; write papers; keep reading notes | Remember and connect ideas; sometimes test one argument |
| What they do | Translate sentences, check validity, do exercises, write short essays | Annotate sources, formalize in Lean, write and export papers | Capture quotes and thoughts, link notes, lay out an argument now and then |
| When | Bursts: homework evenings, before exams, during lectures | Daily deep-work blocks; writing sprints before deadlines | Irregular: evenings and weekends, often weeks apart |
| How they work | Copy patterns, make many errors, need fast feedback | Keyboard-first, want full control, plain files, Git | Low commitment; drop tools that need setup |
| Main worry | "Am I doing this right?" | "Don't slow me down, and don't overstate what the checker proves." | "Is this worth the effort?" |
| What makes them quit | Cryptic Lean errors | Hand-holding, lock-in, slow preview | Install hassle, feeling like homework |

## 16. User Stories and Acceptance Criteria

Each story is followed by the criteria that must hold for it to count as done.

### 16.1 Learner stories

**L1. Translate an argument.** As a learner, I want to turn an English argument into symbols step by step, so that I see how sentences map to logic.

- I can mark a sentence and assign it a letter or predicate.
- The symbolic form appears beside the English, and the link between them stays.
- I can finish the translation without writing Lean syntax.

**L2. Check validity.** As a learner, I want to see whether my argument is valid and why, so that I learn from mistakes.

- A propositional argument with up to 6 letters returns Valid, Not valid, or Can't tell in under 2 s.
- For an invalid argument, a counterexample (which letters are true or false) is shown in plain words.
- The result says "valid", never "true", with a one-line explanation of the difference.

**L3. Understand errors.** As a learner, I want Lean errors explained in plain language, so that I am not stuck on jargon.

- The 10 most common Lean errors each show a plain explanation and a suggested fix.
- The raw Lean message is one click away.
- In Guided mode, none of those 10 errors appears as a raw message alone.

**L4. Practice.** As a learner, I want exercises with feedback, so that I know whether I understood.

- Built-in sets cover translating, completing a proof step, and spotting a fallacy.
- Each answer is marked correct or incorrect; hints reveal one step at a time.
- Progress is saved in the notebook and survives a restart.

**L5. Hand in homework.** As a learner, I want to export my work as a PDF, so that I can submit it.

- I pick the essay or problem-set template and get a PDF with my name and numbered arguments.
- Lean code appears only if I choose to include it.
- A 5-page export finishes in under 10 s.

### 16.2 Experienced-user stories

**E1. Reconstruct a paper's argument.** As an experienced user, I want to turn a passage I am reading into a premise, so that I can rebuild the author's argument as I read.

- One shortcut turns a selected quote into a premise that keeps its source and page.
- Premises reorder by drag or keyboard, and numbering updates.
- The argument block shows the numbered premises and the conclusion C, and the numbering updates when premises are reordered.

**E2. Formalize in Lean without leaving the app.** As an experienced user, I want full Lean 4 access, so that I can formalize exactly what I mean.

- Diagnostics appear within 1 s after I stop typing for 300 ms.
- A goal view shows the proof state at the cursor.
- I can import from my own Lake project and define custom notation.
- Ctrl+Enter checks the current block.

**E3. Choose a modal system.** As an experienced user, I want to pick the modal logic an argument is checked in, so that I can test it under different assumptions.

- Per argument I pick K, T, S4, or S5.
- Changing the system re-checks the argument and updates its status.

**E4. Write and cite a paper.** As an experienced user, I want citations and a typeset draft, so that I can submit or circulate it.

- BibTeX imports, and a chosen CSL style formats citations and the bibliography.
- The preview of a 20-page paper updates in under 2 s.
- Exporting `.tex` source is post-v1; v1 exports PDF, Markdown, and DOCX.

**E5. Own my files.** As an experienced user, I want plain files, so that I am never locked in.

- A notebook is a plain folder that opens in any editor and works with Git.
- Deleting the cache folder loses nothing.
- The app never writes outside the notebook folder.

**E6. Trace a concept.** As an experienced user, I want to see every place a concept is used, so that I can check consistency.

- A concept page lists every note and argument that mentions it.
- Backlinks update on save, and search over 10,000 notes returns in under 100 ms.

**E7. Stay out of my way.** As an experienced user, I want no hints or tutorials, so that nothing slows me down.

- Expert mode turns off hints, tutorials, and the simplified Lean view.
- The setting is remembered per notebook.

### 16.3 Hobbyist stories

**H1. Capture fast.** As a hobbyist, I want to jot down a thought or quote immediately, so that I do not lose it.

- The app opens to a ready new note in under 1 s.
- A new note takes two actions or fewer (shortcut, then type).
- Pasting a quote with its source takes one action.
- Autosave means I never press save.

**H2. Connect ideas.** As a hobbyist, I want my notes to link to each other, so that I can see how ideas relate.

- Typing `[[` suggests existing notes.
- Backlinks show beside every note.

**H3. Lay out an argument without Lean.** As a hobbyist, I want to write down an argument's premises and conclusion, so that I can see whether it holds together, without learning a proof language.

- I can create an argument block and never see Lean.
- Status reads "not formalized" in a neutral style, never as an error or warning.
- The argument map shows it.

**H4. Come back after weeks.** As a hobbyist, I want to pick up where I left off, so that a gap does not feel like falling behind.

- The home screen shows recent notes and a search box.
- Nothing needs updating, migrating, or catching up; there are no reminders or streaks.

**H5. Share an essay.** As a hobbyist, I want a good-looking PDF in one click, so that I can share what I wrote.

- One click exports a PDF with the default template and no settings.
- The result looks professional on the first export.

**H6. Install without hassle.** As a hobbyist, I want to start using the app right away, so that I do not give up before I begin.

- Installing is one Flatpak command.
- The app is fully usable without the Lean toolchain; Lean is offered when I first create a formal block, with the download size shown.
- First launch includes a sample notebook that I can delete.

## 17. Feature Audit

Every feature in sections 4 to 14 was checked against the stories above. The spec is too wide for a first release: about a third of the features serve one rare use, or duplicate something Git, Pandoc, or GTK already gives us.

### 17.1 Verdict per feature

**Use:** Core = the group relies on it, Useful = helps, Rare = occasionally, No = not needed.

| Feature | Learner | Experienced | Hobbyist | Verdict |
| --- | --- | --- | --- | --- |
| Markdown editor and preview | Core | Core | Core | Keep |
| Links, backlinks, full-text search | Useful | Core | Core | Keep |
| Command palette | Rare | Core | Useful | Keep |
| Tags and metadata header | Rare | Useful | Useful | Simplify: tags only, metadata optional |
| Argument blocks | Core | Core | Useful | Keep |
| Premise type tags (assumption, empirical, definition, contested) | Rare | Useful | Rare | Defer |
| Argument map view | Useful | Useful | Core | Simplify: auto-layout, read-only first |
| Manual fallacy annotations | Useful | Rare | Rare | Merge into exercises |
| Lean blocks with inline diagnostics | Useful | Core | Rare | Keep |
| Argument to Lean skeleton, status badge | Core | Core | Rare | Keep |
| Guided mode, plain-language errors | Core | No | Useful | Keep (promote to a named feature) |
| Modal logic library (K, T, S4, S5) | Rare | Core | No | Keep as optional module |
| Deontic and epistemic modules | No | Rare | No | Defer |
| Mathlib support | No | Rare | No | Defer |
| BibTeX and citations | Useful | Core | Rare | Keep |
| Concept glossary with definitions side by side by author | Useful | Rare | Rare | Simplify: plain concept pages |
| Paper projects with outline | Useful | Core | Rare | Keep |
| Live transclusion | No | Useful | No | Simplify: insert with a link back; live later |
| Typst typesetting with live preview | Useful | Core | Useful | Keep |
| LaTeX as a full second pipeline | No | Rare | No | Cut: export `.tex` only |
| Six built-in templates | Useful | Useful | Rare | Simplify: two (essay, article) |
| Blind-review mode | No | Rare | No | Defer |
| Draft snapshot UI with diffs | No | Rare | No | Defer: Git already covers it |
| Word targets, index of terms | Rare | Rare | No | Defer |
| Pre-submission checks | Rare | Useful | No | Simplify: missing citations and references |
| Export: PDF, Markdown | Core | Core | Core | Keep |
| Export: DOCX via Pandoc | Useful | Useful | Rare | Keep |
| Export: PDF/A, EPUB, argument map SVG | No | Rare | Rare | Defer (EPUB: cut) |
| Vim keybindings | No | Rare | No | Keep only if GtkSourceView provides it at no cost |

### 17.2 Missing functionality

The stories need things the spec does not yet describe.

| Gap | Needed by | Why |
| --- | --- | --- |
| English-to-symbols translation workflow | Learner | The first hurdle in logic; story L1 |
| Truth table and counterexample finder (propositional logic, implemented in the Rust app core so it works without Lean) | Learner | Learners need to see why an argument fails; story L2 |
| Exercises with feedback and saved progress | Learner | Practice is the reason they come; story L4 |
| Lean as an optional, on-demand download; app works without it | Hobbyist, Learner | The toolchain is a large download and would stop most hobbyists at install; story H6 |
| Home screen with recent notes, quick capture, sample notebook | Hobbyist | Low-commitment use needs a fast start; stories H1, H4 |
| BibTeX import (v1); import from a Markdown folder or Obsidian vault (post-v1) | Experienced, Hobbyist | People already have notes; no import means no switch |

### 17.3 Recommended first release

**In:** Markdown notes with links and search; argument blocks with an automatic map; Lean blocks with diagnostics and skeleton generation; Guided and Expert modes; propositional and first-order library; counterexample finder; a small exercise set; BibTeX; paper projects on Typst with two templates; PDF, Markdown, and DOCX export; home screen and sample notebook; Lean as an optional download.

**After:** modal library, transclusion, blind review, snapshot UI, extra templates, deontic and epistemic modules, PDF/A.

**Milestone order:** the notes shell lands first (M1), so a build that works fully without Lean exists early. The architecture and packaging spikes come before it in M0, and the Lean connection follows at M4. `IMPLEMENTATION-PLAN.md` section 6 is authoritative.

### 17.4 How to check these assumptions

- Interview five people from each group for 30 minutes about how they take notes and where they get stuck.
- Run three usability sessions on first launch: a learner should reach a checked argument in under 10 minutes without reading docs, and a hobbyist should write a first note in under 30 seconds.
- Revisit the cut list after those sessions; features marked Defer move up if two or more people ask for them unprompted.
