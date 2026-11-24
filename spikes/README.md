# Spikes

M0 exists to prove the risky paths before product code. These directories are
not part of the workspace and are not held to the production lint gate; they are
evidence for Gates G0 (architecture) and G1 (packaging).

## Inventory

| Spike | Directory | Question | State |
| --- | --- | --- | --- |
| Plan B shell | `gtk-shell/` | Can a GTK4 window in Rust open, edit, and save a note? | Builds and launches under native Wayland (`GDK_BACKEND=wayland`); the window stayed up until a 6 s timeout. Open/save was not exercised. |
| Plan A FFI | `lean-ffi/` | Can Lean drive GTK through a small C shim, with one callback back into Lean? | C half passes `make check-c`; it found and fixed a bad GTK call. Lean half unbuilt: no toolchain. |
| Lean diagnostics | `lean-lsp/` | Does `lake serve` return diagnostics for a generated block? | Client and fixture written. Not executed: no toolchain. |
| Packaging | `../packaging/` | Do Flatpak, Nix, and AUR builds work, with portal file access and no stray network calls? | Manifests written. No channel built: no `flatpak-builder`, no `nix`. |

The detailed evidence checklist is in `docs/adr/0001-ui-architecture.md` (G0) and
`packaging/README.md` (G1). Neither gate is passed. The status table above is
the source of truth for what was actually executed.

## Rules for spike code

- Spike code may take shortcuts, but each shortcut is named in a comment with
  the `APORIA-xxxx` code it would trip in production, so the shortcut does not
  quietly move into `crates/`.
- A spike result is only a result when it is written down with the command, the
  output, and the version of the toolchain or runtime used.
- Spike code is deleted when its milestone ships the real implementation, unless
  it is still the cheapest way to reproduce a bug.

## Plan B build

```bash
cd spikes/gtk-shell
cargo build
# with a display:
cargo run
```

The crate is deliberately standalone: an empty `[workspace]` table keeps it out
of the root workspace, and its dependencies never reach the app.

## Plan A build

```bash
cd spikes/lean-ffi
make check-c   # GTK-only half; works without Lean
make build     # needs lake, gcc, GTK 4 headers
make run
```
