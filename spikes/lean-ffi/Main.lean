/-
  Plan A spike: Lean drives a GTK4 window through the C shim in ffi.c.

  The spike environment had no Lean toolchain, so this module was not built.
  The declarations below follow Lean's `@[extern]` ABI; confirm them against
  the pinned toolchain before trusting the result.

  Ownership:
    - `@& String` arguments are borrowed by C for the duration of the call.
    - `USize` handles are opaque pointers to the C-side `AporiaApp`.
    - Every call returns an `IO` result; C wraps failures with
      `lean_io_result_mk_error`.
-/

@[extern "aporia_gtk_init"]
opaque gtkInit : IO Unit

@[extern "aporia_window_new"]
opaque windowNew : @& String → IO USize

@[extern "aporia_window_set_text"]
opaque windowSetText : USize → @& String → IO Unit

@[extern "aporia_window_get_text"]
opaque windowGetText : USize → IO String

@[extern "aporia_window_run"]
opaque windowRun : USize → IO UInt32

@[extern "aporia_window_free"]
opaque windowFree : USize → IO Unit

/-- Called from C after a successful save. This is the GTK → Lean callback
    that the Plan A spike exists to prove. -/
@[export aporia_on_save]
def onSave (text : String) : IO Unit := do
  IO.println s!"[lean] save requested ({text.length} characters)"

def main : IO Unit := do
  gtkInit
  let window ← windowNew "Aporia — Lean FFI spike"
  windowSetText window
    "# Aporia\n\nThis text is set from Lean through the C shim.\n\nOpen and save a file to exercise the GTK side.\n"
  let status ← windowRun window
  windowFree window
  if status != 0 then
    IO.eprintln s!"GTK exited with status {status}"
