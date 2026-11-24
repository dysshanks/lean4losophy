# Lint Rule Pages

One page per `APORIA-xxxx` code. The policy, enforcement, and full initial rule table are in [`LINT-POLICY.md`](../../LINT-POLICY.md).

## Page requirements

Use [`APORIA-TEMPLATE.md`](APORIA-TEMPLATE.md) for a new page. It follows the
template in `LINT-POLICY.md` section 9, and every page covers:

- what the rule catches;
- why it matters in user terms;
- concrete alternatives;
- who can approve an exception and what it must contain;
- bad and good examples.

## Initial pages landed in P0

All of these exist, so the corresponding rules may be enabled. Each Rust or
ast-grep rule also has a bad/good fixture pair in `lints/fixtures/`; the
fixtures are run by `./scripts/lint`.

| Code | Rule |
| --- | --- |
| APORIA-1001 | No `unsafe` outside the FFI crate |
| APORIA-1002 | No `unwrap` or `expect` in production |
| APORIA-1003 | No `panic!`, `todo!`, or `unimplemented!` |
| APORIA-1007 | No `process::exit` outside `main` |
| APORIA-1008 | No `println!` or `dbg!` for logging |
| APORIA-1009 | No `#[allow(...)]` |
| APORIA-2001 | No blocking work on the UI thread |
| APORIA-2005 | No bare `TODO`, `FIXME`, `HACK`, or `XXX` |
| APORIA-2009 | No `String` for filesystem paths |
| APORIA-3001 | No telemetry or analytics |
| APORIA-3002 | No network without the consent gate |
| APORIA-3005 | No secrets in the repository |
| APORIA-4001 | No `sorry` or `admit` in `Lean4losophy.Core` |
| APORIA-4002 | No `axiom` without an ADR |
| APORIA-4003 | No `native_decide` in the core |
| APORIA-5001 | Docs must pass the docs linter |
| APORIA-5002 | Format changes need a version and migration |
