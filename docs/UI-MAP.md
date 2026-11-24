# UI Map — what exists, what is missing

**Status:** living document, current through the M2 workbench slice
**Companions:** `DESIGN-PLAN.md` (visual rules), `docs/UI-RULES.md` (workbench rules), `IMPLEMENTATION-PLAN.md` (milestones), `docs/adr/0001-ui-architecture.md`

This page maps the GTK application screen by screen and checks the
`DESIGN-PLAN.md` rules against the widgets that actually exist. It exists so a
gap is a written decision rather than an omission. Update it with any UI change.

## 1. Screens and states

| Area | Widgets | States implemented | Missing or partial |
| --- | --- | --- | --- |
| Home | `AdwStatusPage`, Create notebook / Open notebook pills, recent notebooks | No notebook; recent list | Recent notes, quick capture |
| Empty notebook | `AdwStatusPage` with **Create first note** and **Open notebook…** | Notebook with zero notes | — |
| Shell | activity rail, `AdwOverlaySplitView`, command center in `AdwHeaderBar`, bottom status bar | Sidebar shown/hidden; Notes/Search active state; mode in status | Status bar has no persistent index state |
| Sidebar notes | `GtkListBox`, folder indentation, new-note button | Flat and foldered notes, selection | Drag to move; per-note metadata; sort control |
| Search | `GtkSearchEntry`, visible Tag/Folder fields, results `GtkListBox` with snippets | Empty, results, no-match CTA, filters | Debounce only; no saved searches |
| Editor | `GtkTextView` (monospace, wrap, undo) | Clean, dirty, saving, read-only for `format > 1`, external-change warning | GtkSourceView, line numbers, find/replace, CommonMark highlighting |
| Preview | Native widget renderer over comrak blocks | Headings, paragraphs, lists, code, quotes, tables, rules, footnotes, underlined wiki links | Local images; clickable non-wiki links; syntax highlighting |
| Backlinks | `GtkRevealer` + `GtkListBox` | Closed, list with title/line/context, empty CTA, unavailable | Inline broken-link markers |
| Dialogs | New/Rename prompt; delete confirm; create-from-broken-link confirm | Labeled field; separated destructive action; specific cancel verbs | Validation beyond empty input |
| Index | Window with counts, problem list, **Rebuild index**, **Close** | Current cache, rebuild result | Not a Preferences page; no progress during rebuild |
| About | `GtkAlertDialog` | Version and privacy line | Credits and licenses |
| Onboarding | — | — | Sample notebook choice and Guided/Expert step (M1 gap) |
| Metadata/tags | — | Front matter is read and preserved | Panel to view or edit title, aliases, tags |
| Settings | — | Settings files and precedence | UI showing scope and values |
| Command palette | Modal `GtkWindow`, `GtkSearchEntry`, filtered `GtkListBox`, shortcuts | Opens from Ctrl+K, Ctrl+Shift+P, or the command center; filters, arrow keys, Enter, Escape | Fuzzy ranking and recent-command memory |

## 2. Rule status (`DESIGN-PLAN.md` section 2)

| # | Rule | Status | Evidence or gap |
| --- | --- | --- | --- |
| 1 | Left-align long text | Met | Editor and preview labels use `xalign 0`; no justified text |
| 2 | Label fills about half the button | Unverified | GTK sizes buttons to content; no width measurement |
| 3 | Regular body weight | Met | `.lean4losophy-body` is weight 400; headings 650 |
| 4 | Blue-tinted greys | Met | Tokens use hue 220; generator checks contrast |
| 5 | Deletes far from save | Met | Delete dialog uses danger styling and a separator; cancel is “Keep note” |
| 6 | Layered small shadows | Partial | Tokens define three levels, but no app surface uses elevation yet |
| 7 | Tight heading leading | Met | h1 32/36, h2 24/28, body 14/21 |
| 8 | Tinted text on colour | Partial | `on_accent`/`on_danger` exist; no coloured callout surfaces yet |
| 9 | Baseline-aligned mixed sizes | Unverified | No widget forces baseline alignment; GTK defaults vary |
| 10 | Visible field labels | Met with exception | New/Rename fields and search filters have labels; the search box itself relies on its icon and tooltip (see section 3) |
| 11 | One hue per semantic colour | Met | Token file defines a single hue per semantic colour |
| 12 | Side padding ≥ top padding | Met | Preview 28/18, dialogs 20/16, index 18/18, editor 24/16 |
| 13 | Button labels say what they do | Met | Create note, Rename note, Delete note, Rebuild index, Clear search, Find notes to link |
| 14 | More space above headings | Met | Preview margins top 18, bottom 6 |
| 15 | Toned-down dark mode | Met | Dark text is tinted off-white, not `#FFFFFF`; accents desaturated |
| 16 | Right-align numeric table columns | Met | Columns whose non-empty body cells parse as numbers are right-aligned with tabular figures |
| 17 | Stacked text gap = smaller line height | Partial | Sidebar result and backlink rows now use the 8 px spacing token, not the caption’s 16 px line height |
| 18 | CTAs in empty states | Met | Home, empty notebook, no search results, and no backlinks each have one clear action |

## 3. Documented exceptions and decisions

- **Search is not a form field.** It has no persistent label; the magnifier icon,
  placeholder, and tooltip (“Search notes”) carry the meaning. A visible label
  would take space from the results it labels. Confirm this exception or add a
  label above the entry.
- **Ctrl+K belongs to the palette.** Ctrl+F focuses note search; Ctrl+K and
  Ctrl+Shift+P open the command palette; Escape in the search entry returns to
  the notes list. See `docs/UI-RULES.md` rule W12.
- **Numeric column detection** is deliberately simple: a column is numeric when
  every non-empty body cell parses as a number after removing commas and a
  trailing `%`. Headers are ignored. Mixed columns stay left-aligned.
- **Window size** is restored from `config.toml` on launch and saved on close.
  The maximized size is saved like any other; the unmaximized size is not
  remembered separately.
- **Empty-model states:** “no notebook”, “notebook with no notes”, “no search
  results”, and “no backlinks” are distinct states with distinct CTAs. A note
  that fails to parse opens read-only rather than showing a separate error page.

## 4. Gaps, in priority order

### P1 — next UI slice

1. Metadata panel: title, aliases, tags, created/updated, read-only ID. This is
   the largest missing surface and is already promised by the format.
2. Onboarding: a first-run page that offers the sample notebook and the
   Guided/Expert choice, instead of relying on menu toggles.
3. Settings window: global and notebook settings with their scope shown
   (`FORMAT.md` section 11.3).
4. Find/replace inside the note, once GtkSourceView is available or implemented
   on `GtkTextView`.
5. Accessible names and roles for icon-only header buttons; verified keyboard
   focus order; an Orca pass.
6. Broken-link markers in the editor or a per-note problems strip.

### P2 — later

- Command palette fuzzy ranking and recent-command memory.
- Visual-regression snapshots for light/dark, forms, tables, empty states.
- Responsive pane sizing and a remembered split position.
- Recent notes on Home; drag-to-move notes in the sidebar.
- Link hover/focus states beyond the persistent underline.
- About dialog credits and third-party notices.

## 5. How to check a UI change

1. Find the affected row in section 1 and the rule in section 2.
2. Update the status and the evidence, or record a new exception in section 3.
3. Run `./scripts/lint` (contrast is checked by the token generator).
4. Launch with `GDK_BACKEND=wayland` and check stderr for CSS parse errors.
5. Exercise the command palette with keyboard only: Ctrl+K, type, Up/Down,
   Enter, Escape. Check that Ctrl+F still focuses search.
6. If the change touches an empty state, exercise it with a notebook that has no
   notes, a search with no matches, and a note with no backlinks.
