module
public import Aeneas
public import AxiomEngine.Generated.Types
@[expose] public section
open Aeneas Aeneas.Std Result

/-!
# External functions

Models of the opaque methods of `axiom_core::exact_rational::ExactRational`,
which Aeneas leaves without bodies (see `FunsExternal_Template.lean`). Each is
a definition, not an axiom: proof 3 trusts that the Rust method, a one-line
delegation to `num_rational`, computes the operation on `Rat` given here. The
Rust division panics on a zero divisor, so its model fails there.
-/

/-- [axiom_core::exact_rational::ExactRational::one]: the rational one. -/
def exact_rational.ExactRational.one : Result exact_rational.ExactRational :=
  ok 1

/-- [axiom_core::exact_rational::ExactRational::add]: the sum. -/
def exact_rational.ExactRational.add (x y : exact_rational.ExactRational) :
    Result exact_rational.ExactRational :=
  ok (x + y)

/-- [axiom_core::exact_rational::ExactRational::sub]: the difference. -/
def exact_rational.ExactRational.sub (x y : exact_rational.ExactRational) :
    Result exact_rational.ExactRational :=
  ok (x - y)

/-- [axiom_core::exact_rational::ExactRational::mul]: the product. -/
def exact_rational.ExactRational.mul (x y : exact_rational.ExactRational) :
    Result exact_rational.ExactRational :=
  ok (x * y)

/-- [axiom_core::exact_rational::ExactRational::div]: the quotient, which
panics on a zero divisor. -/
def exact_rational.ExactRational.div (x y : exact_rational.ExactRational) :
    Result exact_rational.ExactRational :=
  if y = 0 then fail .panic else ok (x / y)

/-- [axiom_core::exact_rational::ExactRational::is_zero]: whether the value is
zero. -/
def exact_rational.ExactRational.is_zero (x : exact_rational.ExactRational) :
    Result Bool :=
  ok (decide (x = 0))
