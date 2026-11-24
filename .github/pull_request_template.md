# Pull request

## Summary

What changes, and why?

## Related

- Issue:
- ADR (required for architecture, format, dependency, network, or unsafe changes):

## Tests

- [ ] `./scripts/lint` passes
- [ ] `cargo test --workspace` passes
- [ ] New or changed behavior has a test
- [ ] Goldens updated only where intended, and the diff was reviewed

## Lint policy

- [ ] No `#[allow(...)]` was added
- [ ] Any `#[expect(...)]` includes an `APORIA-xxxx` code, a reason, and an issue
      or ADR, and is listed below
- [ ] No `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`,
      `dbg!`, or `println!` was added to production code
- [ ] No bare `TODO`, `FIXME`, `HACK`, or `XXX` was added
- [ ] No lint was disabled, no threshold lowered, and no `continue-on-error`
      added

Suppressions and their codes:

## Design

- [ ] The `DESIGN-PLAN.md` checklist was applied to user-visible changes
- [ ] Light and dark mode were checked where relevant
- [ ] Keyboard focus and accessible labels were checked where relevant

## Risk

- [ ] No new network call, or the call is allowlisted and behind consent
- [ ] No new dependency, or its license, size, and maintenance were reviewed
- [ ] No change to the on-disk format, or `FORMAT.md` and a migration were added
- [ ] No new thread or task without an owner and a shutdown path
- [ ] No unbounded growth in a cache, queue, or log

## Notes

Anything a reviewer should know. Do not include time estimates.
