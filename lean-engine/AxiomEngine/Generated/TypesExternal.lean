module
public import Aeneas
@[expose] public section

/-!
# External types

The model of the opaque Rust type `axiom_core::exact_rational::ExactRational`.
Aeneas leaves it without a body (see `TypesExternal_Template.lean`); proof 3
trusts that the Rust wrapper around `num_rational::BigRational` denotes a
rational number, so it is modelled as Lean's `Rat`.
-/

/-- [axiom_core::exact_rational::ExactRational], modelled as `Rat`. -/
abbrev exact_rational.ExactRational : Type := Rat
