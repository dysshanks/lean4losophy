# Packaging

Three channels, one app: Flatpak (primary), Nix, and AUR. The policy and the
update model are in `docs/adr/0003-packaging-update.md`.

## Status at M0 — Gate G1 is open

The manifests here are **skeletons**, not proven builds. What exists and what is
missing:

| Channel | Files | State |
| --- | --- | --- |
| Flatpak | `flatpak/io.github.dysshanks.lean4losophy.yml`, desktop entry, AppStream metadata | Manifest targets the `lean4losophy` binary. Not buildable until `cargo-sources.json` is generated; `flatpak-builder` is not installed here. |
| Nix | `nix/flake.nix`, `nix/package.nix` | Builds the workspace with GTK4 and libadwaita inputs and installs the binary, desktop entry, and metainfo. Not executed: `nix` is not installed in the development environment. |
| AUR | `aur/PKGBUILD` | Installs the M1 binary, desktop entry, metainfo, and licenses. `makepkg --printsrcinfo` parses it; it was not built from source here. |

G1 requires more than files. It passes only when, on a clean system:

1. the Flatpak builds from the manifest and runs;
2. opening a notebook folder works through the portal, with no broad home
   access;
3. the network permission behaves as disclosed: no request without the update
   toggle, and the Lean download works with consent;
4. the Nix package builds from source, `nix run` starts the app;
5. the AUR package builds in a clean chroot;
6. all three are recorded with commands and output in the gate note.

None of the six has been executed yet. Do not mark G1 passed before they are.

## Flatpak

```bash
# One-time: generate the vendored crate sources from the lockfile.
python3 flatpak-cargo-generator.py Cargo.lock \
  -o packaging/flatpak/cargo-sources.json

# Build and run against the runtime.
flatpak-builder --user --install --force-clean build-dir \
  packaging/flatpak/io.github.dysshanks.lean4losophy.yml
flatpak run io.github.dysshanks.lean4losophy
```

Portal check: open a notebook from a folder outside the sandbox (Documents,
a USB drive). The file chooser must appear through the portal and the app must
read and write only the chosen folder. Any request for `--filesystem=home` is a
failure of the portal path.

Network check: run with the update toggle off and confirm no outbound
connection (for example with a network namespace or a capture). Then enable the
toggle and confirm only the release feed is contacted.

## Nix

```bash
nix build .#packages.x86_64-linux.default
nix run .#packages.x86_64-linux.default
nix develop
```

The flake pins `nixpkgs-unstable`; the app's minimum GTK floor is the Flatpak
runtime's GTK, which is newer than the oldest distro supported.

## AUR

```bash
cd packaging/aur
makepkg --printsrcinfo > .SRCINFO   # regenerate after any PKGBUILD change
makepkg -si                          # build and install
```

`lean4` is an optional dependency; the app must install and run without it.

## Naming

The product is **lean4losophy** and the app ID is
`io.github.dysshanks.lean4losophy`. Flathub submission uses that ID; the
desktop entry, metainfo, and Flatpak manifest are named after it.
