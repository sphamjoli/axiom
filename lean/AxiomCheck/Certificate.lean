/-!
# Certificates

The checker's view of a version 1 or version 2 certificate. Values of these types are produced only
by `AxiomCheck.Parse.certificate`, which enforces every rule of the format
except the arithmetic ones that the checker itself verifies.
-/

namespace AxiomCheck

/-- A non-negative fraction in lowest terms with a positive denominator, as
written in a certificate. -/
structure Fraction where
  /-- The numerator. -/
  num : Nat
  /-- The denominator, which is positive. -/
  den : Nat
  deriving DecidableEq, Repr

/-- A fraction in lowest terms with a positive denominator that may be
negative, as written for a `Contraction` factor. In the certificate a
negative value's numerator is `-` followed by the canonical digits of its
magnitude; `-0` is not allowed, so `negative` implies `num > 0` and every
value has exactly one encoding. -/
structure SignedFraction where
  /-- Whether the value is below zero. -/
  negative : Bool
  /-- The numerator's magnitude. -/
  num : Nat
  /-- The denominator, which is positive. -/
  den : Nat
  deriving DecidableEq, Repr

/-- A claim about the embedded model. -/
inductive Claim where
  /-- Every entry of row `row` is a probability and the entries sum to
  exactly one. -/
  | mass (row : Nat)
  /-- The model has two states and `value` is a fixed point of the update of
  the probability of the second state. Introduced in version 2. -/
  | fixedPoint (value : Fraction)
  /-- The model has two states, `factor` is the factor `λ = 1 - a - b` of
  that update, and `|λ| < 1`. Introduced in version 2. -/
  | contraction (factor : SignedFraction)
  deriving DecidableEq, Repr

/-- A certificate whose structure is valid: a square matrix of 1 to 16 rows
of canonical fractions no greater than one, and a non-empty list of distinct
claims that refer to existing rows, with `fixedPoint` and `contraction`
claims only in a version 2 certificate of a two-state model. -/
structure Certificate where
  /-- The transition matrix, one row per current state. -/
  rows : List (List Fraction)
  /-- The claims, in the order they appear. -/
  claims : List Claim
  deriving Repr

end AxiomCheck
