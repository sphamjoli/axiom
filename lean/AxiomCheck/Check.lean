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

/-- Soundness: if the check accepts a certificate, every claim holds. -/
theorem check_sound (c : Certificate) (h : check c = true) : CertificateHolds c := by
  intro claim hmem
  unfold check at h
  exact checkClaim_sound c claim (List.all_eq_true.mp h claim hmem)

end AxiomCheck.Check
