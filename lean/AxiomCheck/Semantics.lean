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

/-- Multiplies `acc` by `x` `k` times, in a tail-recursive loop so that
evaluating a large power does not exhaust the interpreter's stack. -/
def ratPowLoop (x : Rat) : Rat → Nat → Rat
  | acc, 0 => acc
  | acc, k + 1 => ratPowLoop x (acc * x) k

/-- Starting from `x ^ j`, the loop computes `x ^ (j + k)`. -/
theorem ratPowLoop_eq (x : Rat) (j k : Nat) :
    ratPowLoop x (ratPow x j) k = ratPow x (j + k) := by
  induction k generalizing j with
  | zero => rfl
  | succ k ih =>
    show ratPowLoop x (ratPow x (j + 1)) k = _
    rw [ih, Nat.add_right_comm, Nat.add_assoc]

/-- `x` raised to the natural power `k`, by the loop. -/
def ratPowFast (x : Rat) (k : Nat) : Rat := ratPowLoop x 1 k

/-- Compiled code evaluates `ratPow` with the loop. -/
@[csimp] theorem ratPow_eq_fast : @ratPow = @ratPowFast := by
  funext x k
  have h := ratPowLoop_eq x 0 k
  rw [Nat.zero_add] at h
  exact h.symm

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

/-- The probability of success within the first `k` attempts: the retry map
iterated `k` times from zero. -/
def Retry.successWithin (m : Retry) : Nat → Rat
  | 0 => 0
  | k + 1 => m.map (m.successWithin k)

/-- The probability that each of the first `k` attempts fails. -/
def Retry.failureAfter (m : Retry) (k : Nat) : Rat := 1 - m.successWithin k

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

/-- The certified error bound is decidable by exact arithmetic. -/
instance (m : TwoState) (start : Rat) (steps : Nat) (tolerance : Rat) :
    Decidable (ConvergenceBoundCertified m start steps tolerance) :=
  inferInstanceAs (Decidable (_ ≤ _))

/-- `ConvergenceBound`: from `start`, every iterate from step `steps` onward
is within `tolerance` of the stationary probability. -/
def ConvergenceBoundHolds (m : TwoState) (start : Rat) (steps : Nat)
    (tolerance : Rat) : Prop :=
  ∀ t, steps ≤ t → ratAbs (m.iterate start t - m.fixedPoint) ≤ tolerance

/-- The decidable facts a `Termination` claim is checked against: `p` is a
probability, `probability` is the closed-form success probability, the
expected attempts, when given, are `1 / p` with `p ≠ 0`, and the failure
probability after `k` attempts, when given, is `(1 - p) ^ k`. -/
def TerminationCertified (m : Retry) (probability : Rat) (expected : Option Rat)
    (failure : Option (Nat × Rat)) : Prop :=
  (0 ≤ m.p ∧ m.p ≤ 1) ∧ probability = m.successProbability ∧
    (∀ e, expected = some e → m.p ≠ 0 ∧ e = m.expectedAttempts) ∧
    (∀ k q, failure = some (k, q) → q = ratPow (1 - m.p) k)

/-- `Termination`: `probability` is the least fixed point of the retry loop's
equation, that is, the probability that it eventually succeeds. -/
def TerminationHolds (m : Retry) (probability : Rat) : Prop :=
  m.IsLeastFixedPoint probability

/-! ## Certificates -/

/-- The rational value of a certificate fraction. -/
def Fraction.toRat (f : Fraction) : Rat := (f.num : Rat) / (f.den : Rat)

/-- The rational value of a signed certificate fraction. -/
def SignedFraction.toRat (f : SignedFraction) : Rat :=
  if f.negative then -((f.num : Rat) / (f.den : Rat)) else (f.num : Rat) / (f.den : Rat)

/-- The two-state chain of a certificate whose model has exactly two rows of
two entries. With states ordered as in the certificate, `a = rows[0][1]` is
the probability of leaving the first state and `b = rows[1][0]` that of
leaving the second, so the probability `d` of the second state evolves by
`d' = (1 - a - b) d + a`, the convention the engine uses. The diagonal entries
are not read; that each row sums to one is the business of `Mass` claims. -/
def twoState (c : Certificate) : Option TwoState :=
  match c.rows with
  | [[_, a], [b, _]] => some ⟨Fraction.toRat a, Fraction.toRat b⟩
  | _ => none

/-- The retry loop of a certificate whose model is a retry model. -/
def retryModel (c : Certificate) : Option Retry :=
  match c.model with
  | .retry success => some ⟨Fraction.toRat success⟩
  | .chain _ => none

/-- The rational value of an optional `failure_after` pair. -/
def failureToRat (failure : Option (Nat × Fraction)) : Option (Nat × Rat) :=
  failure.map fun (k, q) => (k, Fraction.toRat q)

/-- The rational entries of row `r` of a certificate, if it exists. -/
def rowValues (c : Certificate) (r : Nat) : Option (List Rat) :=
  (c.rows[r]?).map (·.map Fraction.toRat)

/-- Every row of the certificate's matrix is a probability distribution. -/
def RowsHold (c : Certificate) : Prop :=
  ∀ row ∈ c.rows, MassHolds (row.map Fraction.toRat)

/-- What a claim in a certificate asserts about its embedded model, as
decidable facts. Every two-state claim also requires that each row is a
probability distribution, so it is never accepted about a matrix that is not
stochastic, and `fixedPoint` requires `a + b ≠ 0`, so the fixed point is
unique: the stationary probability. For `convergenceBound` and `termination` these are the
certified facts; `AxiomCheck.Theorems` proves that they imply the claims'
full meaning, `ConvergenceBoundHolds` and `TerminationHolds`. -/
def ClaimHolds (c : Certificate) : Claim → Prop
  | .mass r => ∃ row, rowValues c r = some row ∧ MassHolds row
  | .fixedPoint v =>
    ∃ m, twoState c = some m ∧ RowsHold c ∧ m.a + m.b ≠ 0 ∧ FixedPointHolds m (Fraction.toRat v)
  | .contraction f =>
    ∃ m, twoState c = some m ∧ RowsHold c ∧ ContractionHolds m (SignedFraction.toRat f)
  | .convergenceBound start steps tolerance =>
    ∃ m, twoState c = some m ∧ RowsHold c ∧ m.a + m.b ≠ 0 ∧ ratAbs m.lam ≤ 1 ∧
      ConvergenceBoundCertified m (Fraction.toRat start) steps (Fraction.toRat tolerance)
  | .termination probability expected failure =>
    ∃ m, retryModel c = some m ∧ TerminationCertified m (Fraction.toRat probability)
      (expected.map Fraction.toRat) (failureToRat failure)

/-- Every claim of the certificate holds. -/
def CertificateHolds (c : Certificate) : Prop :=
  ∀ claim ∈ c.claims, ClaimHolds c claim

end AxiomCheck.Semantics
