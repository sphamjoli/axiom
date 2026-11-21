/-!
# Certificates

The checker's view of a version 1, 2 or 3 certificate. Values of these types are produced only
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
  /-- The model has two states with `a + b ≠ 0` and `|λ ^ steps (start - d*)|`
  is at most `tolerance`, so every iterate from step `steps` on is within
  `tolerance` of the stationary probability. Introduced in version 3. -/
  | convergenceBound (start : Fraction) (steps : Nat) (tolerance : Fraction)
  /-- The model is a retry loop and `probability` is its probability of
  eventual success; `expectedAttempts`, when present, is `1 / p` with
  `p > 0`, and `failureAfter`, when present, is a count `k` with the
  probability `(1 - p) ^ k` that the first `k` attempts fail. Introduced in
  version 3. -/
  | termination (probability : Fraction) (expectedAttempts : Option Fraction)
      (failureAfter : Option (Nat × Fraction))
  deriving DecidableEq, Repr

/-- The model a certificate embeds. -/
inductive Model where
  /-- A chain, given by its transition matrix, one row per current state. -/
  | chain (rows : List (List Fraction))
  /-- A retry loop whose attempts each succeed with probability `success`.
  Introduced in version 3. -/
  | retry (success : Fraction)
  deriving DecidableEq, Repr

/-- A certificate whose structure is valid: a model that is either a square
matrix of 1 to 16 rows of canonical fractions no greater than one or a retry
loop, and a non-empty list of distinct claims that refer to the right kind of
model: `mass` claims to existing rows of a chain, `fixedPoint`,
`contraction` and `convergenceBound` claims to a two-state chain, and
`termination` claims to a retry loop. -/
structure Certificate where
  /-- The embedded model. -/
  model : Model
  /-- The claims, in the order they appear. -/
  claims : List Claim
  deriving Repr

/-- The transition matrix of a chain certificate, or no rows for a retry
model. -/
def Certificate.rows (c : Certificate) : List (List Fraction) :=
  match c.model with
  | .chain rows => rows
  | .retry _ => []

end AxiomCheck
