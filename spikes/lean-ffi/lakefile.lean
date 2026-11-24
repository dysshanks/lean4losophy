import Lake
open Lake DSL

/-
  The GTK side is compiled by the Makefile into build/ffi/libaporiffi.a so
  that this lakefile only has to link it. `lake build` alone is not enough;
  run `make build` instead.
-/
package «aporia-ffi-spike» where
  version := v!"0.0.0"
  moreLinkArgs := #[
    "-Lbuild/ffi",
    "-laporiffi",
    "-lgtk-4",
    "-lglib-2.0",
    "-lgobject-2.0"
  ]

@[default_target]
lean_exe aporiaFfi where
  root := `Main
