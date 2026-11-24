# Third-Party Notices and License Review

lean4losophy is Apache-2.0. `LICENSE` and `NOTICE` cover the project itself. This page
covers what we depend on, how it is distributed, and why each license is
acceptable. It is reviewed when a dependency is added and at each gate.

The automated gate is `cargo deny check` in `./scripts/lint`, configured by
`deny.toml`. Cargo dependencies outside the allowlist fail the build. This page
covers the non-crate dependencies, which `cargo-deny` cannot see.

## Crates

- Permitted licenses: Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Zlib,
  0BSD, BSL-1.0, Unicode-3.0, CC0-1.0, MPL-2.0.
- GPL, AGPL, LGPL, and SSPL crates are not permitted. An LGPL library may still
  be used when it comes from the runtime or the host system rather than being
  vendored into the app.
- `deny.toml` also denies yanked crates, unknown registries, unknown git
  sources, wildcard requirements, and duplicate major versions.
- The crate list and their resolved versions are in `Cargo.lock`; changes are
  reviewed with the pull request that adds them.

## Runtime and system libraries

These are provided by the Flatpak runtime or by distribution packages. They are
not vendored into the repository, and their notices ship with the runtime.

| Dependency | License | Distribution | Notes |
| --- | --- | --- | --- |
| GTK 4 | LGPL-2.1-or-later | Flatpak runtime `org.gnome.Platform`; distro package | System library, dynamically linked. LGPL compliance relies on the user's ability to replace it, which the runtime and distro packaging preserve. |
| libadwaita | LGPL-2.1-or-later | Flatpak runtime; distro package | Same treatment as GTK. |
| GLib, GObject, Gio, Pango, Cairo, GdkPixbuf | LGPL-2.1-or-later | Flatpak runtime; distro package | Transitive system libraries. |
| GtkSourceView 5 | LGPL-2.1-or-later | Flatpak runtime; distro package | Editor widget, M1. |
| WebKitGTK 6.0 | LGPL-2.1-or-later plus BSD-2-Clause portions | Flatpak runtime; distro package | Preview widget, M1. Never bundled by us. |
| SQLite | Public domain | Bundled through `rusqlite` 0.41 / `libsqlite3-sys` 0.39 (SQLite 3.54.1+) | No notice required; recorded for completeness. The bundled copy is required to be 3.54+ for RUSTSEC-2026-0459. Do not switch to a host SQLite without checking its version. |
| Typst | Apache-2.0 | CLI or embedded library | Compatible. Version pinned in M6. |
| Lean 4 toolchain | Apache-2.0 | Downloaded on demand with consent (ADR-0003) | Not bundled or distributed by us. The download shows source, version, size, and checksum. |
| Lean core library source | Apache-2.0 | Shipped as source under `lean/Lean4losophy/` | First-party files; Lean core itself is a dependency of the toolchain. |
| Pandoc | GPL-2.0-or-later | **Not bundled** | DOCX export uses a host-installed Pandoc only, or a native exporter decided in M6. Bundling it in the Flatpak would impose GPL obligations on the distribution, so it is explicitly out of scope for the bundle. |
| Libertinus Serif, EB Garamond, Source Serif | SIL OFL-1.1 | Bundled with the paper templates in M6 | OFL permits bundling and modification; the font names are reserved. Notices go in this file and the app's about dialog. |
| Adwaita icon theme | LGPL-3.0-or-later or CC-BY-SA-3.0 | Flatpak runtime | Runtime-provided. |

## Rules for adding a dependency

1. Check the license against `deny.toml` and this page.
2. Record why the dependency is needed, its approximate size, and its
   maintenance status in the pull request (`APORIA-6001`).
3. If it is not a Rust crate, add a row here before merging.
4. If it is GPL, AGPL, or SSPL, it cannot be bundled; propose an ADR first.
5. If it links copyleft code, confirm it is dynamically linked from the runtime
   or the host, never vendored.

## Bundled notice obligations

- Apache-2.0 dependencies: keep their `NOTICE` content, if any, alongside the
  app's notices.
- OFL fonts: ship the license text, keep the reserved font names, and do not
  rename a modified font without changing the name.
- LGPL system libraries: do not bundle; the Flatpak runtime's own notices
  satisfy attribution, and the app reads them from the runtime.

This page is part of the M0 license deliverable and is updated whenever the
dependency set changes.
