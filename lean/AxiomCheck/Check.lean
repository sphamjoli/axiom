import AxiomCheck.Semantics

/-!
# Claim checks

Decides each claim of a parsed certificate by evaluation over exact
rationals, and proves that a successful check implies the claim's meaning.
Every check is a decidable equality or inequality of rationals, so each
soundness proof reflects the `decide` that computed it.
-/

namespace AxiomCheck.Check

open AxiomCheck.Semantics

/-- Whether a list of rationals is a row of probabilities summing to one. -/
def checkRow (row : List Rat) : Bool :=
  row.all (fun x => decide (0 ≤ x ∧ x ≤ 1)) && decide (ratSum row = 1)

/-- A successful row check implies `MassHolds`. -/
theorem checkRow_sound (row : List Rat) (h : checkRow row = true) : MassHolds row := by
  unfold checkRow at h
  simp only [Bool.and_eq_true, List.all_eq_true, decide_eq_true_eq] at h
  exact ⟨h.1, h.2⟩

/-- Whether `value` is unchanged by the update `d' = λ d + c` of the
two-state chain `m`. -/
def checkFixedPoint (m : TwoState) (value : Rat) : Bool :=
  decide (m.update value = value)

/-- A successful fixed point check implies `FixedPointHolds`. -/
theorem checkFixedPoint_sound (m : TwoState) (value : Rat)
    (h : checkFixedPoint m value = true) : FixedPointHolds m value := by
  unfold FixedPointHolds
  exact of_decide_eq_true h

/-- Whether `factor` is the factor `λ = 1 - a - b` of the two-state chain `m`
and `|λ| < 1`. -/
def checkContraction (m : TwoState) (factor : Rat) : Bool :=
  decide (ratAbs m.lam < 1 ∧ factor = m.lam)

/-- A successful contraction check implies `ContractionHolds`. -/
theorem checkContraction_sound (m : TwoState) (factor : Rat)
    (h : checkContraction m factor = true) : ContractionHolds m factor := by
  unfold ContractionHolds
  exact of_decide_eq_true h

/-- Whether `a + b ≠ 0`, `|λ| ≤ 1` and the bound
`|λ ^ steps (start - d*)| ≤ tolerance` hold for the two-state chain `m`. -/
def checkConvergenceBound (m : TwoState) (start : Rat) (steps : Nat) (tolerance : Rat) : Bool :=
  decide (m.a + m.b ≠ 0 ∧ ratAbs m.lam ≤ 1 ∧ ConvergenceBoundCertified m start steps tolerance)

/-- A successful convergence bound check implies the certified facts. -/
theorem checkConvergenceBound_sound (m : TwoState) (start : Rat) (steps : Nat) (tolerance : Rat)
    (h : checkConvergenceBound m start steps tolerance = true) :
    m.a + m.b ≠ 0 ∧ ratAbs m.lam ≤ 1 ∧ ConvergenceBoundCertified m start steps tolerance :=
  of_decide_eq_true h

/-- Whether the expected attempts, if given, are `1 / p` with `p ≠ 0`. -/
def checkExpected (m : Retry) : Option Rat → Bool
  | none => true
  | some e => decide (m.p ≠ 0 ∧ e = m.expectedAttempts)

/-- Whether the failure probability, if given, is `(1 - p) ^ k`. -/
def checkFailure (m : Retry) : Option (Nat × Rat) → Bool
  | none => true
  | some (k, q) => decide (q = ratPow (1 - m.p) k)

/-- Whether a `Termination` claim's facts hold for the retry loop `m`. -/
def checkTermination (m : Retry) (probability : Rat) (expected : Option Rat)
    (failure : Option (Nat × Rat)) : Bool :=
  decide (0 ≤ m.p ∧ m.p ≤ 1) && decide (probability = m.successProbability) &&
    checkExpected m expected && checkFailure m failure

/-- A successful termination check implies `TerminationCertified`. -/
theorem checkTermination_sound (m : Retry) (probability : Rat) (expected : Option Rat)
    (failure : Option (Nat × Rat))
    (h : checkTermination m probability expected failure = true) :
    TerminationCertified m probability expected failure := by
  unfold checkTermination at h
  simp only [Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨⟨⟨hp, hprob⟩, he⟩, hf⟩ := h
  refine ⟨hp, hprob, ?_, ?_⟩
  · intro e hsome
    subst hsome
    exact of_decide_eq_true he
  · intro k q hsome
    subst hsome
    exact of_decide_eq_true hf

/-- Checks one claim of a certificate against its embedded model. A
`fixedPoint` or `contraction` claim fails unless the model has two states. -/
def checkClaim (c : Certificate) : Claim → Bool
  | .mass r =>
    match rowValues c r with
    | some row => checkRow row
    | none => false
  | .fixedPoint v =>
    match twoState c with
    | some m => checkFixedPoint m (Fraction.toRat v)
    | none => false
  | .contraction f =>
    match twoState c with
    | some m => checkContraction m (SignedFraction.toRat f)
    | none => false
  | .convergenceBound start steps tolerance =>
    match twoState c with
    | some m => checkConvergenceBound m (Fraction.toRat start) steps (Fraction.toRat tolerance)
    | none => false
  | .termination probability expected failure =>
    match retryModel c with
    | some m => checkTermination m (Fraction.toRat probability) (expected.map Fraction.toRat)
        (failureToRat failure)
    | none => false

/-- Checks every claim of a certificate. -/
def check (c : Certificate) : Bool :=
  c.claims.all (checkClaim c)

/-- A successful claim check implies the claim holds. -/
theorem checkClaim_sound (c : Certificate) (claim : Claim) (h : checkClaim c claim = true) :
    ClaimHolds c claim := by
  cases claim with
  | mass r =>
    simp only [checkClaim] at h
    simp only [ClaimHolds]
    cases hrow : rowValues c r with
    | some row =>
      rw [hrow] at h
      exact ⟨row, rfl, checkRow_sound row h⟩
    | none =>
      rw [hrow] at h
      exact absurd h Bool.false_ne_true
  | fixedPoint v =>
    simp only [checkClaim] at h
    simp only [ClaimHolds]
    cases hm : twoState c with
    | some m =>
      rw [hm] at h
      exact ⟨m, rfl, checkFixedPoint_sound m (Fraction.toRat v) h⟩
    | none =>
      rw [hm] at h
      exact absurd h Bool.false_ne_true
  | contraction f =>
    simp only [checkClaim] at h
    simp only [ClaimHolds]
    cases hm : twoState c with
    | some m =>
      rw [hm] at h
      exact ⟨m, rfl, checkContraction_sound m (SignedFraction.toRat f) h⟩
    | none =>
      rw [hm] at h
      exact absurd h Bool.false_ne_true
  | convergenceBound start steps tolerance =>
    simp only [checkClaim] at h
    simp only [ClaimHolds]
    cases hm : twoState c with
    | some m =>
      rw [hm] at h
      exact ⟨m, rfl, checkConvergenceBound_sound m _ steps _ h⟩
    | none =>
      rw [hm] at h
      exact absurd h Bool.false_ne_true
  | termination probability expected failure =>
    simp only [checkClaim] at h
    simp only [ClaimHolds]
    cases hm : retryModel c with
    | some m =>
      rw [hm] at h
      exact ⟨m, rfl, checkTermination_sound m _ _ _ h⟩
    | none =>
      rw [hm] at h
      exact absurd h Bool.false_ne_true

/-- Soundness: if the check accepts a certificate, every claim holds. -/
theorem check_sound (c : Certificate) (h : check c = true) : CertificateHolds c := by
  intro claim hmem
  unfold check at h
  exact checkClaim_sound c claim (List.all_eq_true.mp h claim hmem)

end AxiomCheck.Check
