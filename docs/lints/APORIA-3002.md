# APORIA-3002 — No network call without the consent gate

**Severity:** error
**Tool:** `ast-grep` rule `net-outside-client`, `cargo-deny` source allowlist, review
**Scope:** all crates; only the allowlisted client module may open a socket, and only behind the consent gate.

## Why

The app is offline by default. The allowlist is narrow: one update feed check and the consented Lean toolchain download. Anything else breaks the promise and the Flatpak permission story. The realistic failures are quiet: a citation resolver that queries a DOI service leaks the bibliography of an unpublished paper; a preview that fetches an image tells a third party when a note was opened; a Lean block that runs `#eval` with network access turns opening a file into remote code contact.

## Alternatives

- Serve preview assets locally and block remote images, fonts, and scripts (APORIA-3003).
- Add the call to the allowlisted client in the net module, behind an explicit setting.
- Show the consent screen with the endpoint, the data sent, and the size before the first request.
- Respect an offline flag; fail with a typed error instead of retrying silently.
- For Lean, run with no network and no host filesystem beyond the notebook.

## Exceptions

- Owner approval plus a privacy review, an ADR, and an entry in the privacy statement and `FORMAT.md` if it affects stored state.
- The exception names the endpoint, the request fields, and why a local alternative is impossible.
- Never for note content, search queries, citations, Lean block execution, or background polling.
- Re-reviewed at every milestone; an unused exception is deleted.

## Examples

Bad:

```rust
// link preview: sends the note title to a search API
let hits = reqwest::get(format!("https://api.example/search?q={title}")).await?;
```

Good:

```rust
// the only network paths: both allowlisted and consent-gated
if settings.check_for_updates && consent.updates_allowed() {
    net::update::check(&settings.update_feed).await?;
}
if consent.lean_download_allowed() {
    net::lean::download(&toolchain, &progress).await?;
}
```
