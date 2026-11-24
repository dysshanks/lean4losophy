# Open Questions — v1

Short set. Most questions are one click. Answer boxes are used only where a list is needed.

## Settled (not asked again)

- No account or login.
- No telemetry; offline by default.
- Lean is optional; the app works fully without it.
- Typst is the paper typesetting engine.
- v1 is section 17.3’s recommended first release. Anything outside its **In** list is out for v1.
- The first prototype is the notes shell: GTK4 window, Markdown editor, open and save a notebook folder.
- Solo, part-time.
- Install paths for v1: Flatpak and Nix/AUR.
- Updates: in-app (this conflicts with how Flatpak and Nix/AUR normally update, so Q13 asks which wins).

---

## 1. Prototype and notes

### Q1. What does the notes-shell prototype include?

- [ ] Editing only
- [ ] Editing plus links and backlinks
- [x] Editing plus links, backlinks, and search
- [ ] Other: ______________________________________________

### Q2. Markdown flavor

- [x] CommonMark
- [ ] GitHub-flavored Markdown (GFM)
- [ ] Pandoc Markdown
- [ ] A custom subset
- [ ] Other: ______________________________________________

### Q3. Preview layout

- [ ] Side-by-side
- [ ] Toggle
- [x] Both
- [ ] Other: ______________________________________________

### Q4. Notebook structure

- [ ] Flat
- [ ] Folders
- [x] Both
- [ ] Other: ______________________________________________

### Q5. Navigation

- [ ] Sidebar and a single pane
- [x] Sidebar and split panes
- [ ] Tabs
- [ ] Other: ______________________________________________

### Q6. First-run flow

- [x] Sample notebook, ask Guided or Expert, offer Lean
- [ ] Sample notebook, default to Guided, offer Lean
- [ ] Minimal: create or open a notebook only
- [ ] Other: ______________________________________________

### Q7. Link depth in v1

- [ ] `[[Note]]` links and backlinks only
- [x] Plus aliases
- [x] Plus heading links
- [x] Plus block references
- [ ] Other: ______________________________________________

### Q8. Search depth in v1

- [ ] Plain full-text only
- [x] Plus tag and folder filters
- [x] Plus ranking and snippets
- [ ] Other: ______________________________________________

---

## 2. Data and safety

### Q9. Note identity

- [ ] Path-based
- [ ] Stable IDs
- [x] Hybrid
- [ ] Other: ______________________________________________

### Q10. Where do settings live?

- [ ] Notebook folder
- [ ] App config directory
- [x] Split between both
- [ ] Other: ______________________________________________

### Q11. External edits and multiple instances

- [ ] Single instance; reload changes made outside the app
- [ ] Multiple instances with file locking
- [x] Last writer wins
- [ ] Other: ______________________________________________

### Q12. Where is the on-disk format documented?

- [x] Separate `FORMAT.md`
- [ ] A section in `SPEC.md`
- [ ] Code comments only
- [ ] Not yet
- [ ] Other: ______________________________________________

---

## 3. Install, update, and platform

### Q13. Update model

Flatpak and Nix/AUR normally handle their own updates. You chose in-app updates. Which wins for v1?

- [x] In-app updater, checks on launch
- [ ] In-app updater, manual check only
- [ ] Store-managed for all builds (Flatpak, Nix, AUR)
- [x] Store-managed for Flatpak, in-app for Nix/AUR
- [ ] Other: ______________________________________________

### Q14. Non-technical install path

- [ ] Flathub or the desktop app center
- [ ] Direct `.flatpakref` download
- [x] Both
- [ ] Other: ______________________________________________

### Q15. How Lean is installed

- [ ] Bundled with the app
- [x] Downloaded on demand, with consent
- [ ] Through elan
- [ ] User installs it themselves
- [ ] Other: ______________________________________________

### Q16. Minimum supported system

- [ ] Current Ubuntu LTS
- [ ] Current Fedora
- [x] Any GTK4/Wayland distribution
- [ ] Other: ______________________________________________

---

## 4. v1 product decisions

### Q17. Argument block syntax (open question 13)

- [ ] Custom fenced block
- [ ] YAML front matter
- [x] Markdown list convention
- [ ] Other: ______________________________________________

### Q18. License (open question 13)

- [ ] MIT
- [ ] GPL-3.0
- [x] Apache-2.0
- [ ] Undecided
- [ ] Other: ______________________________________________

### Q19. Name (open question 13)

- [ ] Keep Aporia
- [x] Decide later
- [ ] Other: ______________________________________________

---

## Deferred until the relevant milestone

No answer needed now, but these are still open for v1:

- Paper editor: separate window or same-window mode.
- Default citation style.
- Exercise content and authoring format.
- Lean version policy and project ownership.
- Typst version and template API.
- Accessibility target.
- Localization.
- Import from Obsidian or a Markdown folder (currently outside 17.3’s In list).
