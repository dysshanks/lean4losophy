import Lake
open Lake DSL

/-
  Minimal Lake project for the M0 LSP spike. `lake serve` runs the LSP server
  against this package, so diagnostics come from a real build configuration
  rather than a bare file.
-/
package «lean4losophy-lsp-spike» where
  version := v!"0.0.0"

@[default_target]
lean_lib Lean4losophy
