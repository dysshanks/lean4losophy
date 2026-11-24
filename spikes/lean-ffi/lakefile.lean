import Lake
open Lake DSL

/-
  The GTK side is compiled by the Makefile into build/ffi/liblean4losophyffi.a so
  that this lakefile only has to link it. `lake build` alone is not enough;
  run `make build` instead.
-/
package «lean4losophy-ffi-spike» where
  version := v!"0.0.0"
  moreLinkArgs := #[
    "-Lbuild/ffi",
    "-llean4losophyffi",
    "-lgtk-4",
    "-lglib-2.0",
    "-lgobject-2.0"
  ]

@[default_target]
lean_exe lean4losophyFfi where
  root := `Main
