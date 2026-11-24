# APORIA-3005 — No secrets in the repository

**Severity:** error
**Tool:** `gitleaks detect`, CI secret scanning, review
**Scope:** every tracked file and every commit, including fixtures, docs, packaging, and Nix expressions.

## Why

A committed credential is a credential someone else can use. For this project the damage is concrete: a Flatpak or release signing key lets an attacker publish an update users will install; an AUR or CI token lets them alter a package; an API key copied into a test fixture is still a live key. Removing the file does not remove the secret from Git history or from forks.

## Alternatives

- Read secrets from the environment or a CI secret store at build or release time.
- Keep local secrets in an untracked file and document the variable name in `.env.example`.
- Use synthetic placeholders in fixtures: `test-token-not-real`, never a copied value.
- Prefer short-lived, scoped tokens over long-lived keys.
- If a secret is found: revoke and rotate first, then remove it, then purge history if the repository is public.

## Exceptions

- None for a real credential.
- A committed test credential needs owner approval and must be provably non-functional, clearly labelled, and unable to reach any service.
- Rotation is required even when the leak was brief; the reviewer records the incident in the PR.
- A detected secret blocks the merge until rotation is confirmed.

## Examples

Bad:

```rust
const FLATPAK_TOKEN: &str = "example-live-token-0123456789";
let client = Client::new("https://release.example", FLATPAK_TOKEN);
```

Good:

```rust
let token = std::env::var("APORIA_RELEASE_TOKEN")
    .map_err(|_| ReleaseError::MissingToken)?;
let client = Client::new("https://release.example", &token);
```
