import AxiomCheck.Certificate
import Batteries.Data.Rat.Lemmas

/-!
# Semantics

The meaning of models and claims, over exact rationals. A chain is a square
matrix of transition probabilities; a two-state chain evolves by the affine
update `d' = λ d + c`; a retry model succeeds with probability `p` on each
attempt. Each claim kind is given a meaning as a proposition, and certificate
fractions are interpreted as rationals so that a parsed certificate's `Mass`
claims have a precise meaning.

The definitions use Batteries' `Rat` and avoid Mathlib, so the checker's
trusted base stays small.
-/

namespace AxiomCheck.Semantics

/-! ## Rational helpers -/

/-- The absolute value of a rational. -/
def ratAbs (x : Rat) : Rat := if x < 0 then -x else x

/-- `x` raised to the natural power `k`. -/
def ratPow (x : Rat) : Nat → Rat
  | 0 => 1
  | k + 1 => ratPow x k * x

/-- The sum of a list of rationals. -/
def ratSum (xs : List Rat) : Rat := xs.foldr (· + ·) 0

/-- The sum of `f i` over every state `i` of an `n`-state model. -/
def sumFin {n : Nat} (f : Fin n → Rat) : Rat := ratSum ((List.finRange n).map f)

/-! ## Chains -/

/-- A Markov chain on `n` states, given by its transition matrix: `rows i j` is
the probability of moving from state `i` to state `j`. -/
structure Chain (n : Nat) where
  /-- The transition matrix, one row per current state. -/
  rows : Fin n → Fin n → Rat

/-- A row of probabilities: every entry lies in `[0, 1]` and the entries sum
to exactly one. -/
def IsDistribution {n : Nat} (row : Fin n → Rat) : Prop :=
  (∀ j, 0 ≤ row j ∧ row j ≤ 1) ∧ sumFin row = 1

/-- A chain is well formed when every row is a probability distribution. -/
def Chain.WellFormed {n : Nat} (m : Chain n) : Prop :=
  ∀ i, IsDistribution (m.rows i)

/-- One step of the chain applied to a distribution over states:
`d' j = Σ i, d i * rows i j`. -/
def Chain.step {n : Nat} (m : Chain n) (d : Fin n → Rat) : Fin n → Rat :=
  fun j => sumFin fun i => d i * m.rows i j

/-! ## Two-state chains -/

/-- A two-state chain that leaves state 0 with probability `a` and leaves
state 1 with probability `b`. Its matrix is `[[1 - a, a], [b, 1 - b]]`. -/
structure TwoState where
  /-- The probability of moving from state 0 to state 1. -/
  a : Rat
  /-- The probability of moving from state 1 to state 0. -/
  b : Rat

/-- The entry in row `i`, column `j` of a two-state chain's matrix. -/
def TwoState.entry (m : TwoState) (i j : Fin 2) : Rat :=
  if i.val = 0 then (if j.val = 0 then 1 - m.a else m.a)
  else (if j.val = 0 then m.b else 1 - m.b)

/-- The transition matrix of a two-state chain, used only in proofs and
specifications. -/
noncomputable def TwoState.chain (m : TwoState) : Chain 2 := ⟨m.entry⟩

/-- The contraction factor `λ = 1 - a - b` of the affine update. -/
def TwoState.lam (m : TwoState) : Rat := 1 - m.a - m.b

/-- The constant `c = a` of the affine update. -/
def TwoState.offset (m : TwoState) : Rat := m.a

/-- The affine update `d' = λ d + c` of the probability `d` of being in
state 1. -/
def TwoState.update (m : TwoState) (d : Rat) : Rat := m.lam * d + m.offset

/-- The probability of being in state 1 after `t` updates from `d`. -/
def TwoState.iterate (m : TwoState) (d : Rat) : Nat → Rat
  | 0 => d
  | t + 1 => m.update (m.iterate d t)

/-- The stationary probability `d* = a / (a + b)` of being in state 1. -/
def TwoState.fixedPoint (m : TwoState) : Rat := m.a / (m.a + m.b)

/-! ## Retry models -/

/-- A retry loop whose attempts each succeed independently with probability
`p`. -/
structure Retry where
  /-- The probability that one attempt succeeds. -/
  p : Rat

/-- The one-step map `x ↦ p + (1 - p) x` whose least fixed point on `[0, 1]`
is the probability of eventual success. -/
def Retry.map (m : Retry) (x : Rat) : Rat := m.p + (1 - m.p) * x

/-- `x` is the least fixed point of the retry map among the probabilities:
`x ∈ [0, 1]`, `x = p + (1 - p) x`, and `x ≤ y` for every such `y`. -/
def Retry.IsLeastFixedPoint (m : Retry) (x : Rat) : Prop :=
  (0 ≤ x ∧ x ≤ 1) ∧ m.map x = x ∧
    ∀ y, 0 ≤ y → y ≤ 1 → m.map y = y → x ≤ y

/-- The closed form of the success probability: zero when attempts never
succeed, one otherwise. -/
def Retry.successProbability (m : Retry) : Rat := if m.p = 0 then 0 else 1

/-- The expected number of attempts, `1 / p`, meaningful when `p > 0`. -/
def Retry.expectedAttempts (m : Retry) : Rat := 1 / m.p

/-! ## Claims -/

/-- `Mass`: every entry of the row is a probability and the entries sum to
exactly one. -/
def MassHolds (row : List Rat) : Prop :=
  (∀ x ∈ row, 0 ≤ x ∧ x ≤ 1) ∧ ratSum row = 1

/-- `FixedPoint`: `value` is unchanged by the two-state update. -/
def FixedPointHolds (m : TwoState) (value : Rat) : Prop :=
  m.update value = value

/-- `Contraction`: the update is a strict contraction, `|λ| < 1`, and `factor`
is its factor `λ`. -/
def ContractionHolds (m : TwoState) (factor : Rat) : Prop :=
  ratAbs m.lam < 1 ∧ factor = m.lam

/-- The certified error bound after `steps` updates from `start`:
`|λ ^ steps (start - d*)| ≤ tolerance`. -/
def ConvergenceBoundCertified (m : TwoState) (start : Rat) (steps : Nat)
    (tolerance : Rat) : Prop :=
  ratAbs (ratPow m.lam steps * (start - m.fixedPoint)) ≤ tolerance

/-- `ConvergenceBound`: from `start`, every iterate from step `steps` onward
is within `tolerance` of the stationary probability. -/
def ConvergenceBoundHolds (m : TwoState) (start : Rat) (steps : Nat)
    (tolerance : Rat) : Prop :=
  ∀ t, steps ≤ t → ratAbs (m.iterate start t - m.fixedPoint) ≤ tolerance

/-- `Termination`: `probability` is the least fixed point of the retry loop's
equation, that is, the probability that it eventually succeeds. -/
def TerminationHolds (m : Retry) (probability : Rat) : Prop :=
  m.IsLeastFixedPoint probability

/-! ## Certificates -/

/-- The rational value of a certificate fraction. -/
def Fraction.toRat (f : Fraction) : Rat := (f.num : Rat) / (f.den : Rat)

/-- The rational entries of row `r` of a certificate, if it exists. -/
def rowValues (c : Certificate) (r : Nat) : Option (List Rat) :=
  (c.rows[r]?).map (·.map Fraction.toRat)

/-- What a claim in a certificate asserts about its embedded model. -/
def ClaimHolds (c : Certificate) : Claim → Prop
  | .mass r => ∃ row, rowValues c r = some row ∧ MassHolds row

/-- Every claim of the certificate holds. -/
def CertificateHolds (c : Certificate) : Prop :=
  ∀ claim ∈ c.claims, ClaimHolds c claim

end AxiomCheck.Semantics
