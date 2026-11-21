/-!
# Certificates

The checker's view of a version 1 certificate, as defined in
`docs/adr/0001-certificate-format.md`. Values of these types are produced only
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

/-- A claim about the embedded model. -/
inductive Claim where
  /-- Every entry of row `row` is a probability and the entries sum to
  exactly one. -/
  | mass (row : Nat)
  deriving DecidableEq, Repr

/-- A certificate whose structure is valid: a square matrix of 1 to 16 rows
of canonical fractions no greater than one, and a non-empty list of distinct
claims that refer to existing rows. -/
structure Certificate where
  /-- The transition matrix, one row per current state. -/
  rows : List (List Fraction)
  /-- The claims, in the order they appear. -/
  claims : List Claim
  deriving Repr

end AxiomCheck
