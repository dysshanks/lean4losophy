# UI and UX Rules — the lean4losophy workbench

**Status:** binding for the application UI; current through the M2 workbench slice
**Companions:** `DESIGN-PLAN.md` (visual system), `docs/UI-MAP.md` (implementation map)
**Scope:** the GTK application. Paper and export rules stay in `DESIGN-PLAN.md`.

The direction is a deliberate mix: **VSCode supplies the chrome and the keyboard
model; Obsidian supplies the note canvas and the linking model.** Neither side
is copied wholesale.

## 1. What comes from where

| VSCode supplies | Obsidian supplies |
| --- | --- |
| A persistent activity rail with one job per icon | Notes as the primary object, with folders as a quiet hierarchy |
| Dense, flat chrome built from panels and 1 px borders | A calm editor and preview with a readable measure |
| A command center and a keyboard command palette | Wiki links, backlinks, and a content-first reading surface |
| A compact status bar that reports real state | Local-first, plain-file behaviour with no account surfaces |
| Keyboard-first actions with visible shortcuts | Empty states that teach the next note action |

### Not imported

- No minimap, debug toolbar, extension marketplace, or settings tree. The app is
  not a code editor.
- No graph view, canvas, or plugin browser in v1. They are not required for the
  notes workflow yet.
- No modal navigation for ordinary note switching. Dialogs are for destructive
  actions and short prompts, not for browsing.
- No decorative gradients, large shadows, or branding panels.

## 2. Layout

```text
┌─────────────────────────────────────────────────────────────────────┐
│ header    sidebar toggle · backlinks        command center · actions │
├────┬───────────────┬──────────────────────────┬─────────────────────┤
│ A  │ sidebar       │ editor                   │ preview             │
│ C  │               │                          │                     │
│ T  │ notes/search  │ current note             │ rendered note       │
│ I  │               │                          │                     │
│ V  │               ├──────────────────────────┴─────────────────────┤
│    │               │ backlinks (revealed on demand)                  │
├────┴───────────────┴─────────────────────────────────────────────────┤
│ status: save/search state                              Guided · Markdown│
└─────────────────────────────────────────────────────────────────────┘
```

The activity rail is always present, including on Home. Toggling the sidebar
hides the sidebar, never the rail. The editor and preview keep the same vertical
rhythm; the backlinks panel is a drawer, not a third column.

## 3. Workbench rules

| # | Rule | Test |
| --- | --- | --- |
| W1 | The activity rail is persistent and has one icon per primary surface: Notes, Search, Backlinks, Index, Home. | Rail remains visible when the sidebar is hidden and on Home. |
| W2 | Active rail items use the accent colour plus a tinted surface; inactive items are muted. Only one item is active. | Notes and Search never appear active together. |
| W3 | The sidebar has one surface at a time: notes or search results. Search is not a separate window. | Typing switches the sidebar to results; Escape returns to notes. |
| W4 | The header contains a command center that opens the palette. It is the discoverable counterpart to Ctrl+K. | Clicking the center opens the palette; the palette closes on Escape. |
| W5 | The command palette is keyboard-complete: type to filter, Up/Down to move, Enter to run, Escape to close. Clicking a row works too. | A command can be run without touching the mouse. |
| W6 | The status bar reports persistent truth: current activity, save/search state, and the Guided/Expert mode. Routine autosave is not a toast. | Status text changes after save, rebuild, and mode switch. |
| W7 | Chrome is denser than content. Rail buttons are 32 px, sidebar rows 24–28 px, status 24 px; the editor keeps its 14–15 px text and comfortable line height. | A 10,000-note sidebar stays scannable; prose is not squeezed. |
| W8 | Panels use surface hierarchy, not raised cards: rail darkest, sidebar panel, editor canvas. Borders separate panels. | Chrome has no drop shadows; only the palette may float. |
| W9 | Accent is state, not decoration. It marks selection, active rail items, links, and the single primary action. | No large accent backgrounds, no accent headings. |
| W10 | Workbench chrome uses 6 px radii and 1 px borders; content blocks keep their own rhythm. | Rail, sidebar rows, command center, and palette are consistent. |
| W11 | Every icon-only control has a tooltip and an action; every primary action has a shortcut where possible. | Rail, header, and palette expose the same action names. |
| W12 | Ctrl+K and Ctrl+Shift+P open the palette; Ctrl+F focuses search; Escape closes the palette or clears search. | Shortcuts do not collide; the palette owns Ctrl+K. |
| W13 | The note canvas follows Obsidian: readable measure, visible wiki links, quick backlinks, and no permanent inspector clutter. | Editor and preview stay usable at 760 px width. |
| W14 | Empty states keep one clear CTA. The workbench chrome stays visible so the user does not lose their place. | Home, empty notebook, no search, and no backlinks all retain the rail. |
| W15 | Dark and light themes share the same hierarchy; dark mode uses borders and surface steps instead of heavy shadows. | Contrast checks pass in both themes; no pure-white text. |
| W16 | No feature is added just because VSCode or Obsidian has it. If it does not serve notes, arguments, or search, it is out. | New chrome has a workflow, an empty state, and a keyboard path. |

## 4. Density and type

- UI copy is 11–14 px; status and shortcut hints are 11 px; note body is 14 px
  with 21 px line height; the editor may use 15 px monospace.
- Sidebar section labels are 11 px, weight 700, muted. Do not use all-caps for
  long labels.
- Command palette rows are 32–36 px with the command on the left and its
  shortcut on the right.
- Spacing uses the token scale (4, 8, 12, 16, 24, 32). Chrome paddings are 6–10
  px; content paddings are 18–28 px.
- A control is never taller than the row it sits in unless it is the primary
  action of a dialog.

## 5. Interaction contract

- The palette is filtered by a case-insensitive substring of the command label.
  Fuzzy ranking and recent-command memory are deferred; plain substring is
  predictable and fast.
- The palette closes before it invokes the command, so a command that opens a
  dialog never fights the palette for focus.
- Escape in the search entry clears the search and returns the sidebar to notes.
  Escape in the palette closes only the palette.
- Rail actions that need a notebook say “Open a notebook first.” and stay in
  place; they never show an empty modal.
- Backlinks remain a revealer inside the editor column. They are not a tab and
  do not replace the note list.
- Status messages are transient in content but stable in location. The status
  bar does not move when the message length changes.

## 6. Content rules

- Wiki links are underlined, not colour-only. Backlinks show title, line, and
  context; opening one lands on the source line.
- Editor and preview share the same note and layout toggle. There is no second
  document model for preview.
- The preview is a reading surface: no remote images, fonts, or scripts; blocked
  content is visibly blocked rather than silently fetched.
- Tables follow `DESIGN-PLAN.md`: text left, numbers right with tabular figures.

## 7. Implementation map

| Surface | Widgets | Rule status |
| --- | --- | --- |
| Activity rail | `GtkBox` of flat icon buttons, 46 px wide | W1, W2, W7, W10, W11 |
| Sidebar | panel surface, notes/results stack, filters | W3, W8, W9, W13 |
| Header | command center button, sidebar/backlinks/preview/new/menu actions | W4, W11 |
| Command palette | modal `GtkWindow`, `GtkSearchEntry`, `GtkListBox` | W4, W5, W12 |
| Status bar | left status, right mode | W6, W7, W9 |
| Backlinks | revealer drawer with list and context | W13, W14 |
| Empty states | Home, empty notebook, no results, no backlinks | W14 |

## 8. Review checklist

1. Does the change belong to chrome or content? Apply the density and surface
   rules for that side, not the other side.
2. Is there a keyboard path for every new action? Does it appear in the palette?
3. Does the change keep the rail visible and the note canvas readable at
   760 × 480?
4. Is accent used only for state or the one primary action?
5. Run `./scripts/lint`, then launch with `GDK_BACKEND=wayland` and check stderr
   for CSS parse errors.
6. Update `docs/UI-MAP.md` and this page when a surface or rule changes.
