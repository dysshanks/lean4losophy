# ADR-0003 — Packaging and update model

- **Status:** Accepted
- **Date:** 2026-11-24
- **Gate:** G1 (packaging), G8 (v1 release)
- **Related:** ADR-0001, ADR-0005

## Context

v1 ships through Flatpak and Nix/AUR. Those channels normally own their own updates, but Q13 also asked for an in-app update check. Both answers were selected, so the plan interprets them as: check on launch, let Flatpak stay store-managed, and guide Nix/AUR users without replacing anything.

Two promises constrain the design:

- "Offline by default" and "no telemetry" are product promises, not preferences.
- A self-replacing binary is a security and packaging liability: it bypasses signatures, the sandbox, and the package manager.

## Decision

### Channels

- **Flatpak** is the primary channel: Flathub for discovery, plus a direct `.flatpakref` for users without Flathub.
- **Nix:** a flake with a package and a dev shell; `nix run` and profile install.
- **AUR:** a source `PKGBUILD`; `lean4` stays optional.
- The Flatpak runtime sets the minimum GTK4/libadwaita floor. Native builds are tested on current Ubuntu LTS, Fedora, and Arch.

### Update check

- One checker, started on launch **only when the user enabled it** during onboarding or later in settings.
- The onboarding toggle is off by default, so "offline by default" is literal. Enabling it is one click and is remembered globally.
- The check is an unauthenticated GET to the project's release feed (GitHub releases). It sends no identifiers, no note content, no machine ID, and no usage data.
- A static `User-Agent` identifies the app and version, as the feed requires; it is not unique per user or machine.
- Failure, timeout, and offline are normal and silent. The app never retries in a loop and never blocks startup.
- The result is cached for a minimum interval (24 hours) and can be refreshed by a manual "Check for updates" action.
- The check reveals the user's IP address to the feed host. This is disclosed in the privacy statement and the Flathub appdata.

### Channel actions

- **Flatpak:** store-managed. The app opens the desktop software center at the app page and never invokes `flatpak`, `ostree`, or a package manager itself.
- **Nix and AUR:** guided. The app shows the exact command for the detected channel (`nix profile upgrade ...`, or the AUR helper command for the package) with a copy button. It does not run the command, does not use `sudo`, and does not write outside its own data directory.
- **No self-replacing binary in v1.** No downloaded executable, no delta patcher, no writing into the Flatpak runtime, no `flatpak override`. This is a deliberate scope decision from Q13.

### Privacy and network rules

- No account, login, telemetry, analytics, or crash upload.
- The outbound allowlist has exactly two flows:
  1. the update feed, when enabled;
  2. the consented Lean toolchain download, with a pinned version, source, size, and checksum.
- Everything else is local. The preview blocks remote content; external links leave the app; Lean runs sandboxed with no network.
- Logs are local and contain no note content.
- The Flatpak manifest requests network access because of the two allowed flows. The permission is disclosed in the appdata, the README, and the privacy statement, even when the user disables the check: Flatpak permissions are static and cannot be dropped at runtime.
- A network call outside the allowlist is a lint failure (`APORIA-2003`, `APORIA-3002`) and a design-review failure (`APORIA-6004`).

### Naming

The app name and app ID are deferred. Flathub submission waits for the name; internal builds use a temporary ID that is not published. The final ID is recorded here before M7.

## Spike status at M0 (2026-11-24)

G1 is **not passed**. `packaging/` contains a Flatpak manifest with a desktop
entry and AppStream metadata, a Nix flake and package, and an AUR `PKGBUILD`.
None has been built: the development environment has no `flatpak-builder` and no
`nix`, and the AUR package was only parsed with `makepkg --printsrcinfo`. The
Flatpak manifest is also not yet installable because there is no app binary
until M1 and `cargo-sources.json` is not generated. The six-step G1 checklist in
`packaging/README.md` is the evidence required before this ADR records Accepted.

## Consequences

Positive:

- Updates follow the channel's trust model: signatures, sandbox, and package-manager policy stay intact.
- The app never needs write access to its own installation.
- The network surface is small enough to audit and test.
- Flatpak users get one-click updates from the software center; Nix/AUR users get the correct command without guessing.

Negative:

- The app cannot hotfix itself. A bad release waits for the store or for the user to upgrade.
- Flatpak users depend on Flathub review and release cadence.
- Nix/AUR guidance is copy-and-paste; users still run the command.
- The network permission is present even when unused, which is a disclosure burden and a small trust cost.
- A release feed is an external dependency; rate limits and feed changes must be handled gracefully.

Neutral and follow-ups:

- Version comparison handles prereleases, build metadata, and downgrades, with unit tests.
- Packaging tests cover Flatpak install/run, Nix build, and AUR build in a container.
- The privacy statement is part of the release checklist, not an afterthought.

## Alternatives considered

### Store-managed for every channel

Cleanest trust model, but Nix/AUR users get no signal at all, and Q13 explicitly asked for an in-app path. Rejected for v1, kept as the simplest fallback if the checker proves noisy.

### In-app updater that replaces the binary

Rejected. It bypasses package signatures and the sandbox, needs write access to the install prefix, and creates a second update mechanism to secure.

### Manual check only

Less intrusive, but users who never open settings never learn about a security release. Rejected; the launch check with an off-by-default toggle is the compromise.

### No update check

Contradicts Q13 and leaves security updates invisible. Rejected.

## References

- `SPEC.md` sections 8, 9, 12.
- `IMPLEMENTATION-PLAN.md` sections 17 (Packaging, updates, privacy) and 20 (open decisions).
- `OPEN-QUESTIONS.md` Q13, Q14, Q15, Q16.
- `APORIA-2003`, `APORIA-3001`, `APORIA-3002`, `APORIA-6004`.
