# Design Plan

**Status:** Draft
**Basis:** the rules supplied for this project, `SPEC.md`, and `IMPLEMENTATION-PLAN.md`
**Scope:** the GTK4 application and the Typst paper templates/exports

## 1. Principles

- Text first: the interface exists to make reading and writing comfortable.
- Calm surfaces: few hues, low chroma, small elevation, no decorative noise.
- Explicit actions: labels say what happens, and destructive actions are never next to safe ones.
- One system: the app and the generated paper share typography, spacing, and color rules where the medium allows it.
- Accessibility is part of the rule set, not a later pass.

## 2. Rules and where they apply

| # | Rule | Interpretation | App | Paper |
| --- | --- | --- | --- | --- |
| 1 | Left-align long text | Body copy, notes, and paper prose are left-aligned with a ragged right edge; no justification | Yes | Yes |
| 2 | Text fills half the button | Button width is about twice the label width; the label occupies roughly half the button; no wide buttons with tiny labels | Yes | No |
| 3 | Keep body text regular | Body text is weight 400; use weight for headings and emphasis, not for paragraphs | Yes | Yes |
| 4 | Greys have a slight blue tint | Neutral palette uses one blue hue at low chroma, not pure grey | Yes | Partial |
| 5 | Keep deletes far from save | Destructive actions are spatially separated from save/primary actions and styled as destructive | Yes | No |
| 6 | Layered smaller shadows instead of big shadows | Elevation uses two or three small, low-opacity shadows; no single large blur | Yes | No |
| 7 | Tight headline line height | Headings use 1.1–1.2 line height; body uses about 1.45–1.5 | Yes | Yes |
| 8 | Tint text on a coloured background | Text on a coloured surface is a low-chroma tint of that hue, not pure black or white | Yes | Yes |
| 9 | Mixed text sizes align on the baseline | Different sizes in one row or stack share a baseline, not a center or top edge | Yes | Yes |
| 10 | Not only placeholders; add labels in fields | Every field has a persistent visible label; placeholders are examples only | Yes | No |
| 11 | One hue per colour | Each semantic colour is built from one hue; states vary lightness and chroma, not hue | Yes | Yes |
| 12 | More padding on the sides than the top | Containers use horizontal padding greater than or equal to top padding | Yes | Partial |
| 13 | Button labels say what they do | Use “Save note”, “Export PDF”, “Delete notebook”; never “Submit” or “OK” alone | Yes | No |
| 14 | Headings get more space above than below | Heading margin-top is clearly larger than margin-bottom | Yes | Yes |
| 15 | Toned-down colours in dark mode | Dark mode lowers saturation and avoids bright accents and pure white text | Yes | No |
| 16 | Right-align numbers in tables | Numeric columns are right-aligned with tabular figures; text columns stay left | Yes | Yes |
| 17 | Stacked text is spaced by the smaller text’s height | The vertical gap between stacked text blocks equals the line height of the smaller text | Yes | Yes |
| 18 | Add a CTA to empty states | Every empty state has one clear next action, not just an explanation | Yes | No |

## 3. Design tokens

Tokens are defined once in `design/tokens.toml` and generated into GTK CSS and Typst variables. No component hard-codes a colour, size, or shadow.

### 3.1 Colour

- Neutral hue: 220 (blue).
- Neutral chroma: low, roughly 4–8%; never a pure grey.
- One hue per semantic colour:
  - accent: one blue hue;
  - success: one green hue;
  - warning: one amber hue;
  - danger: one red hue.
- States use lightness and chroma only; no hue-shifting gradients.
- Text on a coloured surface uses a tint of that surface’s hue, with contrast checked.
- Dark mode:
  - surfaces are darker and less saturated;
  - accents are desaturated and slightly lighter than their light-mode equivalents;
  - body text is a tinted off-white, not `#FFFFFF`;
  - avoid pure black backgrounds.

Starting values, to be tuned visually:

| Token | Light | Dark |
| --- | --- | --- |
| `bg` | blue-tinted near-white | blue-tinted near-black |
| `surface` | white with a blue tint | elevated blue-tinted dark |
| `text` | near-black with a blue tint | off-white with a blue tint |
| `muted` | blue-tinted mid-grey | blue-tinted light-grey |
| `accent` | one blue | desaturated blue |
| `danger` | one red | desaturated red |

All combinations must meet WCAG 2.2 AA contrast for their size.

### 3.2 Typography

- UI font: the system UI font; do not bundle a second UI font.
- Body: regular 400.
- Heading weights may be 600–700; never bold body paragraphs.
- Line height:
  - body: 1.45–1.5;
  - headings: 1.1–1.2;
  - captions: 1.35–1.4.
- Type scale (starting point):
  - caption 12/16
  - body 14/21
  - body large 16/24
  - h3 20/24
  - h2 24/28
  - h1 32/36
  - display 40/44
- Mixed sizes on one line align on the baseline.
- Long text keeps a readable measure; target 60–75 characters per line in the editor and paper.

### 3.3 Spacing

- Base scale: 4, 8, 12, 16, 24, 32, 48.
- Container padding: horizontal ≥ top. Example: 20 px sides, 16 px top and bottom.
- Heading spacing: top is about 1.5–2× the bottom.
- Stacked text: gap = line height of the smaller adjacent text.
- Avoid one-off spacing values; add a token if a new value is genuinely needed.

### 3.4 Elevation

Three levels, each built from small shadows:

- Level 1: `0 1px 2px` plus `0 1px 1px`, low opacity.
- Level 2: `0 1px 2px` plus `0 4px 8px`, low opacity.
- Level 3: `0 1px 2px` plus `0 6px 12px` plus `0 12px 24px`, all low opacity.

No single large blur. Shadows are darker in light mode and nearly absent in dark mode; dark mode relies on surface lightness instead.

### 3.5 Radius

- Controls: 8 px.
- Cards and popovers: 12 px.
- Pills and badges: full radius.
- Nested controls use a smaller radius than their container.

## 4. Layout

- Long text is left-aligned. Centered text is reserved for short labels, empty-state titles, and dialogs.
- Page and panel padding: horizontal padding is larger than top padding.
- Content columns keep a readable measure; the editor and paper preview do not stretch prose across the full window.
- Mixed-size rows align on the baseline: icon plus label, title plus metadata, number plus unit.
- Stacked blocks use the smaller text’s line height as the gap, not a fixed spacing token.
- Headings sit closer to the text they introduce than to the text above them.
- The sidebar and main pane share the same vertical rhythm; no pane uses a different base spacing.

## 5. Components

### Buttons

- Label the action, not the mechanism: “Save note”, “Export PDF”, “Delete notebook”.
- Avoid “Submit”, “OK”, “Yes”, and “Continue” when a specific verb is available.
- Width is about twice the label width; the label should occupy roughly half the button. Short labels get short buttons.
- One primary action per view or dialog. Secondary actions are quieter; destructive actions are separate.
- Destructive actions:
  - never adjacent to save/primary;
  - use the danger colour;
  - require confirmation for irreversible actions;
  - may move to an overflow menu when space is tight.
- Disabled buttons explain why through a tooltip or nearby helper text.

### Forms and fields

- Every field has a visible label above it.
- Placeholders are examples, never the only label.
- Required fields are marked; optional fields are marked only if most fields are required.
- Helper text sits below the field; error text replaces or follows it without shifting the layout.
- Labels and values align on the baseline.
- Validation messages say what to fix, not just “invalid”.

### Tables

- Text columns are left-aligned.
- Numeric columns are right-aligned with tabular figures.
- Units are in the header, not repeated in every cell.
- Header labels align with their column’s content.
- Row height is driven by the text line height; mixed sizes align on the baseline.
- No zebra stripes unless they are needed for wide tables; use spacing and rules first.

### Headings and body

- Body text is regular weight and left-aligned.
- Headings use tight line height and more space above than below.
- Heading levels differ by size and weight, not by colour alone.
- Do not use all-caps for long headings; reserve it for short labels.
- Links are distinguishable by more than colour: underline on hover/focus, or a persistent underline in body text.

### Cards, dialogs, and popovers

- Horizontal padding is greater than or equal to top padding.
- Elevation uses the layered small shadows from section 3.4.
- A dialog has one primary action; destructive actions are separated and styled as danger.
- Close/cancel is never the most prominent action.
- Scrollable content keeps the header and action row stable.

### Badges, banners, and callouts

- Text on a coloured background is a tint of that hue, not pure white or black.
- Each semantic colour uses one hue; do not mix a blue background with a purple border.
- Contrast is checked for the actual text size.
- Icons match the text colour or use the same hue.

### Empty states

- Every empty state has one clear CTA.
- The CTA is specific: “Create your first note”, “Import a notebook”, “Add an argument”.
- The explanation is one or two lines; the CTA is the visual focus.
- No empty state is a dead end.

### Dark mode

- Surfaces are dark and low-chroma, not pure black.
- Accents are desaturated compared with light mode.
- Body text is a tinted off-white; avoid `#FFFFFF`.
- Shadows are reduced; use surface lightness and subtle borders instead.
- Images and paper previews keep their own colours; do not invert them.

## 6. Paper generation

The paper templates use the same design language where print allows it. The Typst templates and exports must follow these rules:

- **Body:** left-aligned, ragged right, regular weight, 10.5–11.5 pt, line height 1.35–1.45. This overrides the earlier “justified text” note in `SPEC.md`.
- **Headings:** tight leading (1.1–1.2), space above about 1.5–2× the space below.
- **Measure:** 60–75 characters per line where the template allows.
- **Tables:** numeric columns right-aligned with tabular figures; text columns left; units in the header.
- **Mixed sizes:** captions, table cells, and inline notes align on the baseline.
- **Stacked text:** caption/footnote gaps equal the smaller text’s line height.
- **Callouts and shaded boxes:** one hue per colour; text is a tint of the background hue; contrast checked.
- **Accents:** one hue per semantic colour; no multi-hue gradients.
- **Greys:** a slight blue tint is acceptable for rules, captions, and table lines; body text stays near-black for print contrast.
- **Margins:** side padding greater than top padding applies to boxes, table cells, and callouts, not necessarily to page margins, which follow the template.
- **Not applicable:** buttons, form labels, empty-state CTAs, destructive-action spacing, dark mode, and shadows.

Typst starting points:

- Body font: Libertinus Serif, EB Garamond, or Source Serif.
- Headings: same family, heavier weight; no decorative display face.
- Numeric tables: `align(right)` plus tabular figures.
- Callout text: `mix` the background hue with white/black at low chroma.
- Page size: A4 default; US Letter as a template option.

## 7. Implementation

### Single source of tokens

`design/tokens.toml` is the source of truth for:

- colour hue, chroma, and lightness;
- type scale and line heights;
- spacing scale;
- radius;
- shadow layers;
- paper-specific values (body size, measure, margins).

`scripts/gen-design-tokens` generates:

- `design/theme.css` for GTK4/libadwaita;
- `design/paper-tokens.typ` for the Typst templates;
- a JSON file for tests and documentation.

No component may hard-code a token value. A new value is added to the token file first.

### GTK4/libadwaita

- Use `AdwStyleManager` for light/dark and high-contrast changes.
- Use CSS custom properties or generated classes; avoid inline style strings in Rust.
- Use the system UI font and libadwaita controls where possible.
- Add accessible labels, roles, and keyboard focus to every custom widget.
- The argument map and paper preview provide a keyboard-accessible alternative or companion view.

### Typst

- Templates import the generated token file.
- Template functions set body alignment, heading spacing, table alignment, and callout colours.
- The two v1 templates are student essay and journal article.
- Exports use the same resolved manuscript and tokens; PDF and DOCX should not diverge in spacing rules where the format allows.

### Tests

- Visual snapshots for light and dark mode: buttons, forms, tables, empty states, dialogs, callouts.
- PDF page snapshots for both templates: body alignment, heading spacing, table alignment, callout tint.
- Contrast checks for every text/background token pair.
- A design review checklist in the PR template.

## 8. Design review checklist

- [ ] Long text is left-aligned.
- [ ] Body text is regular weight.
- [ ] Button labels are specific verbs.
- [ ] Button width is proportional to the label; no tiny label in a wide button.
- [ ] Destructive actions are separated from save/primary actions.
- [ ] Shadows are layered and small.
- [ ] Headings have tight leading and more space above than below.
- [ ] Text on coloured surfaces is tinted, not pure black/white.
- [ ] Mixed text sizes align on the baseline.
- [ ] Every field has a visible label.
- [ ] Each colour uses one hue.
- [ ] Horizontal padding is greater than or equal to top padding.
- [ ] Dark mode is toned down and not pure black/white.
- [ ] Numeric table columns are right-aligned with tabular figures.
- [ ] Stacked text uses the smaller line height as the gap.
- [ ] Empty states have a CTA.
- [ ] Paper templates follow the applicable rules.
- [ ] Contrast and keyboard focus pass.

## 9. Interpretations to confirm

The ambiguous rules are written down here so they can be corrected:

- “Text fills half the button” is implemented as button width ≈ 2× label width. If it means height or something else, change the token.
- “One hue per colour” is implemented as one hue per semantic colour, with states varying lightness and chroma.
- “Stacked text spaced by the smallest text’s height” is implemented as gap = the smaller text’s line height.
- “More padding on the sides than the top” applies to containers, cards, dialogs, and table cells, not to page margins.
- Paper body is left-aligned, which replaces the “justified text” line in the earlier spec.
- “Greys have a slight blue tint” applies to the app and to paper rules/captions; paper body text stays near-black for print contrast.
