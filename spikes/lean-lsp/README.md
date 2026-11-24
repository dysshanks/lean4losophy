# Lean LSP spike

M0 spike: prove that a per-notebook Lake project can return diagnostics for a
generated block over `lake serve`.

**Status:** not executed. The development environment for this change had no
Lean toolchain (`lean` and `lake` are absent), so the client and the fixture are
written but unproven. Do not record this spike as passed until `run.sh` has been
run against the pinned toolchain and its output is pasted below.

## What it does

1. Starts `lake serve` in this directory (the JSON-RPC/LSP server built into
   Lake).
2. Sends `initialize`, then `initialized`, then `textDocument/didOpen` for
   `Aporia/Spike.lean`, which contains one intentional type error.
3. Waits for `textDocument/publishDiagnostics` for that file.
4. Prints the diagnostics and exits nonzero if none arrive within the timeout.

The file also contains a valid theorem and a `sorry`, so a real toolchain should
return one error and one warning. That distinction matters for M4: only errors
mean "not checked".

## Run

```bash
./run.sh          # 120 s timeout
LEAN_TIMEOUT=300 ./run.sh
```

Requires `lake` on `PATH`; the toolchain is pinned in `lean-toolchain` and
fetched by `elan` on first use.

## Expected shape of the result

```text
initialize: ok
Aporia/Spike.lean: 2 diagnostics
  (4, 24) error: type mismatch ...
  (9, 8) warning: declaration uses 'sorry'
spike: diagnostics received
```

Record the actual output and the toolchain version in this file when it runs.

## Why this spike is separate

The app's Lean client (`crates/aporia-lean`, M4) depends on three things this
spike checks without building the app: the `lake serve` lifecycle, the exact
diagnostic payload shape for a generated file, and whether a pinned toolchain
needs a network fetch on first run. A failure here is cheap; a failure in M4 is
not.
