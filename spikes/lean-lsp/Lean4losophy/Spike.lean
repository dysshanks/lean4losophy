/-
  Spike fixture: one valid theorem, one `sorry`, and one deliberate type error.

  The LSP spike checks that `lake serve` reports the error as an error and the
  `sorry` as a warning. M4 relies on that difference: only the absence of
  errors sets an argument's status to `checked`.
-/

theorem valid (p q : Prop) (hp : p) (hpq : p → q) : q := hpq hp

theorem unfinished (p : Prop) : p := by
  sorry

theorem broken : 1 = 2 := by
  rfl
