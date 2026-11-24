# ADR-0001 — UI architecture: Rust GTK4 with a Lean sidecar

- **Status:** Accepted
- **Date:** 2026-11-24
- **Gate:** G0
- **Related:** ADR-0003, ADR-0005

## Decision record

On 2026-11-24 the project owner directed M1 to start on Plan B. Plan B's shell
builds and launches under native Wayland (see the spike status below), which is
the evidence that matters for the UI choice; Plan A's Lean half remains unbuilt
and is not blocking. Gate G1 (packaging) stays open and is tracked in ADR-0003
and `packaging/README.md`; starting M1 early does not mark it passed.

## Context

Lean 4 has no mature GUI toolkit. `SPEC.md` section 5 names two routes:

- **Plan A:** a Lean 4 application that drives GTK4 through a thin C FFI shim (`@[extern]`).
- **Plan B:** a GTK4 application in Rust that treats Lean as an optional sidecar over LSP.

The routes are not interchangeable. The UI, event loop, data model, and error handling differ, so the choice has to be made before the notes shell (M1) is written. The implementation plan assumes Plan B in its task breakdown and records Plan A as the alternative.

The forces:

- The app must be fully usable without Lean; Lean is an optional, consented download.
- Native Wayland behavior, accessibility, and Flatpak/Nix/AUR packaging are v1 requirements.
- Most code is written by agents with human review at gates; the path must be testable, well documented, and hard to get subtly wrong.
- The Lean logic library and proof checking are the differentiators, but they are not the whole product.
- The project is solo and part-time, so a second language and a process boundary have a real cost.

## Decision

Adopt **Plan B** as the recommended implementation path, pending Gate G0.

- UI: GTK4 + libadwaita in Rust (`gtk4-rs`), GtkSourceView 5 editor, native Wayland backend.
- Notebook, Markdown, links, search, arguments, papers, and settings: Rust crates (`aporia-core`, `aporia-paper`, `aporia-lean` for the client).
- Lean: optional sidecar. The app manages a toolchain, starts `lake serve`, and speaks LSP (`aporia-lean`).
- The Lean logic library ships as `.lean` source under `lean/Aporia/`, not linked into the app binary.

Plan A is not rejected on principle. It stays a valid alternative and becomes the path if G0 shows it is materially faster or less risky. The decision is confirmed or overturned at G0 on measured spike results, not preference.

## Gate G0

G0 is a comparison, not a rubber stamp. Both spikes run before M1.

Required evidence for the winning path:

1. A window opens and renders under Wayland without XWayland.
2. An editor opens, edits, and saves a note inside a notebook folder.
3. Open/save uses atomic writes and folder access through the portal.
4. A packaged build runs: Flatpak first, then Nix and AUR.
5. One Lean round trip works: Plan B returns a diagnostic over LSP; Plan A returns one through the FFI call.
6. Cold start and typing stay within the section 9 targets on the spike.
7. A second developer can repeat the build from the recorded steps.

Selection rule: choose on measured time and risk. Record the spike branch, timings, packaging logs, and the reason in this ADR, then set the status to Accepted or Superseded.

Timeboxes: three days per spike, plus a packaging spike for the winner. A spike that misses its timebox is evidence, not a reason to extend it indefinitely.

## Spike status at M0 (2026-11-24)

G0 is **not passed**. The code exists; the measurements do not. Recorded here so
nobody mistakes a skeleton for a decision:

| Evidence | State |
| --- | --- |
| Plan B window, editor, open/save (`spikes/gtk-shell/`) | Builds and launches under native Wayland; the window stayed up for a 6 s smoke run. Open/save was not exercised. The sidecar handshake is not implemented (it belongs to M4). |
| Plan A Lean → GTK FFI (`spikes/lean-ffi/`) | `ffi.c`, `lean_bridge.c`, `Main.lean`, and a Makefile exist. The C half passes `make check-c` against GTK 4 headers, and that check caught one real bug (a call to a non-existent GTK function) which is fixed. Not built: no Lean toolchain, so `lake build` and the Lean bridge are unverified. |
| Lean diagnostics over `lake serve` (`spikes/lean-lsp/`) | Client, fixture, and runner exist. Not executed: no Lean toolchain. |
| Packaging (`packaging/`) | Manifests exist for Flatpak, Nix, and AUR. None built: no `flatpak-builder`, no `nix`; the AUR `PKGBUILD` parses but was not built. |
| Cold start and typing targets | Not measured. |

What a machine with a display, Lean, `flatpak-builder`, and `nix` must do:

1. `cd spikes/gtk-shell && cargo run` under Wayland; open, edit, and save a note.
2. `cd spikes/lean-ffi && make check-c && make build && make run` with the
   pinned toolchain; confirm the save callback reaches Lean.
3. `cd spikes/lean-lsp && ./run.sh`; paste the diagnostics into its README.
4. Build the Flatpak, Nix package, and AUR package; test portal folder access
   and the network permission as described in `packaging/README.md`.
5. Record timings for cold start and typing, then set the status of this ADR.

Until then this ADR stays **Proposed** and M1 does not start.

## Consequences

Positive:

- The notebook model is not trapped in a Lean binary; "works without Lean" is structural, not a feature flag.
- Removes Lean-runtime/GObject memory ownership from the critical path. `APORIA-1001` (`unsafe`) stays limited to an optional FFI crate.
- Mature Rust crates exist for Markdown, SQLite, file watching, HTTP, and LSP.
- Flatpak and Nix packaging for GTK/Rust applications is well-trodden.
- The Lean integration can change without touching the editor, and the editor can ship before Lean is ready.

Negative:

- Two languages and a process boundary. LSP lifecycle, cancellation, timeouts, and crash recovery must be built and tested.
- Lean ships as source, so the first formal block requires a toolchain download and a Lake build.
- `SPEC.md`'s original "core language: Lean 4" no longer describes the app binary. The spec must say Rust application, Lean library.
- Plan B does not prove the Lean integration. M4 still carries LSP and sandbox risk.
- A Rust UI means the Lean core cannot be called synchronously from a widget; every interaction is asynchronous by construction.

Neutral and follow-ups:

- Update `SPEC.md` sections 5 and 13 when G0 closes.
- If Plan A wins, add an FFI hardening track to M1–M3 and widen the `unsafe` scope in `LINT-POLICY.md`; the FFI crate becomes load-bearing rather than optional.
- If Plan B wins, the `ffi` crate stays absent unless a concrete need appears.
- The crate layout in the implementation plan already reflects Plan B; Plan A replaces `crates/` with `app/` and `ffi/`.

## Alternatives considered

### Plan A — Lean application with C FFI to GTK

Closest to the spec's original framing and keeps one core language. Rejected as the default because it puts FFI, event-loop integration, memory ownership, and packaging on the critical path before any user value ships. It remains viable if the Lean core is the point of the project and the spike is clean.

### Tauri or web UI with a Lean backend

Not carried forward. It adds a webview and a second rendering stack, weakens native Wayland fidelity, and does not remove the process boundary. It also duplicates the Markdown preview problem.

### Lean backend with a native GTK front end in another language

This is Plan B with a different front-end language. Rust is chosen for its GTK bindings, packaging story, and memory safety; the front-end language is not the open question.

## References

- `SPEC.md` sections 5, 9, 12, 13.
- `IMPLEMENTATION-PLAN.md` sections 3, 5, 18, 20.
- ADR-0003 (packaging and updates) for the Flatpak/Nix/AUR proof.
- ADR-0005 (lint policy) for the `unsafe` scope.
