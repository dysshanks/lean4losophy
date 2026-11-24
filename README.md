# Aporia (working title)

A local-first notebook for philosophical thinking. Notes are plain Markdown.
Arguments can be written as structured premises and conclusions, and can
optionally be formalized and machine-checked in Lean 4.

Aporia is offline by default: no account, no telemetry, and no cloud
dependency. A notebook is a folder of plain files that works with Git and any
text editor.

## Status

Milestone M1, the notes shell, is in place. `cargo run -p aporia` opens a
GTK4/libadwaita notebook: create or open a folder, list and switch notes, edit
with debounced autosave, preview Markdown side by side or toggled, rename and
delete notes, and keep Guided/Expert and preview settings with the notebook.

Two M1 substitutions are recorded in ADR-0002 because the build environment has
no GtkSourceView 5 or WebKitGTK 6.0: the editor is a plain `GtkTextView`, and the
preview is a native GTK renderer over the shared comrak block model. Line
numbers, find/replace, the metadata panel, local images, and the first-run
wizard step are still partial.

Gates G0 and G1 are not fully closed. G0 is accepted for Plan B (ADR-0001) since
the shell builds and launches; Plan A's Lean half, the `lake serve` probe, and
the Flatpak/Nix/AUR builds were not executed here, so G1 remains an M7 gate.

`aporia-core` holds the notebook model, front matter with unknown-key
preservation, settings with documented precedence, atomic writes, note listing,
and the preview block model. It is covered by unit, integration, and an ignored
10,000-note performance fixture.

## Documentation

| Document | What it covers |
| --- | --- |
| [SPEC.md](SPEC.md) | Product spec: goals, users, features, architecture, risks |
| [IMPLEMENTATION-PLAN.md](IMPLEMENTATION-PLAN.md) | Milestones, gates, repository layout, quality gates |
| [DESIGN-PLAN.md](DESIGN-PLAN.md) | Visual system, component rules, paper and typography rules |
| [LINT-POLICY.md](LINT-POLICY.md) | `APORIA-xxxx` rule codes, alternatives, suppression policy |
| [OPEN-QUESTIONS.md](OPEN-QUESTIONS.md) | Settled decisions and the remaining product questions |
| [format/FORMAT.md](format/FORMAT.md) | Normative on-disk notebook format: notes, links, arguments, settings, cache, writes |
| [docs/THIRD-PARTY-NOTICES.md](docs/THIRD-PARTY-NOTICES.md) | Dependency license review and bundling rules |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development workflow, lint gate, ADR requirement |
| [AGENTS.md](AGENTS.md) | Short instructions for coding agents |

Design tokens live in `design/tokens.toml`; `design/theme.css`,
`design/theme-dark.css`, `design/paper-tokens.typ`, and `design/tokens.json`
are generated from it and must not be edited by hand.

## Repository layout

```text
.
├── Cargo.toml            # Rust workspace
├── deny.toml             # cargo-deny: licenses, advisories, bans, sources
├── crates/
│   ├── aporia/           # GTK4/libadwaita application (M1 notes shell)
│   └── aporia-core/      # notebook model, Markdown, links, search, arguments
├── design/
│   ├── tokens.toml       # single source for app and paper tokens
│   ├── theme.css         # generated: light GTK4/libadwaita theme
│   ├── theme-dark.css    # generated: dark GTK4/libadwaita theme
│   └── paper-tokens.typ  # generated: Typst variables for papers
├── docs/
│   ├── adr/              # architecture decision records
│   └── lints/            # one page per APORIA-xxxx rule
├── format/FORMAT.md      # normative notebook format
├── lints/                # ast-grep rules and their bad/good fixtures
├── packaging/
│   ├── flatpak/          # manifest, desktop entry, AppStream metadata
│   ├── nix/              # flake and package
│   └── aur/              # PKGBUILD
├── scripts/
│   ├── lint              # single lint entry point, used by CI
│   ├── lean-lint         # banned-construct scan for lean/Aporia
│   └── gen-design-tokens # generates design outputs; --check in CI
├── spikes/               # M0 architecture and packaging spikes
└── .github/workflows/    # CI: lint, test, build
```

Planned crates are `aporia` (GTK4 app), `aporia-lean` (toolchain and LSP
client), `aporia-paper` (manuscripts and Typst), and `aporia-cli` (headless
commands). They are added by their milestones.

## Prerequisites

- Linux with a recent stable Rust toolchain. `rust-toolchain.toml` pins the
  `stable` channel and requests `rustfmt` and `clippy`; install it with
  [rustup](https://rustup.rs/).
- GTK4 and libadwaita development packages to build the app (`libgtk-4-dev` and
  `libadwaita-1-dev` on Debian/Ubuntu, `gtk4-devel` and `libadwaita-devel` on
  Fedora).
- Git, for the repository and for notebook history.
- Optional, for the full lint gate: `cargo-deny`, `cargo-audit`, `gitleaks`,
  `typos`, `markdownlint` or `rumdl`, `shellcheck`, `actionlint`, `ast-grep`,
  and Python 3.11+ for the design-token check. `scripts/lint` reports missing
  tools and skips them; CI installs them so the gate is complete there.
- Optional, for Lean features only: a Lean 4 toolchain. The app must remain
  fully usable without Lean, so Lean is never required to build or test the
  Rust workspace.

## Build, test, and lint

```bash
# Regenerate design outputs after editing design/tokens.toml
python3 scripts/gen-design-tokens

# Build the workspace
cargo build --workspace

# Run the app (opens the home screen; no notebook is created for you)
cargo run -p aporia

# Run the tests
cargo test --workspace

# Run the 10,000-note performance fixture
cargo test -p aporia-core --test performance -- --ignored

# Check formatting and run Clippy with warnings denied
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run the full lint gate (formatting, Clippy, optional tools, custom checks)
./scripts/lint

# Scan the Lean library when it exists
./scripts/lean-lint
```

`scripts/lint` is the single entry point used by CI and by review. Findings are
errors: fix them with an alternative from the matching
`docs/lints/APORIA-xxxx.md` page. Do not add `#[allow(...)]`; use
`#[expect(..., reason = "APORIA-xxxx: ...")]` only with approval.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE). Third-party
dependencies keep their own licenses; dependency licenses are checked in the
lint gate.

Contributions are accepted under the same license. See
[CONTRIBUTING.md](CONTRIBUTING.md).
