import AxiomCheck.Semantics

/-!
# Claim checks

Decides each claim of a parsed certificate by evaluation over exact
rationals, and proves that a successful check implies the claim's meaning.
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

/-- Checks one claim of a certificate against its embedded model. -/
def checkMass (c : Certificate) : Claim → Bool
  | .mass r =>
    match rowValues c r with
    | some row => checkRow row
    | none => false

/-- Checks every claim of a certificate. -/
def check (c : Certificate) : Bool :=
  c.claims.all (checkMass c)

/-- A successful claim check implies the claim holds. -/
theorem checkMass_sound (c : Certificate) (claim : Claim) (h : checkMass c claim = true) :
    ClaimHolds c claim := by
  cases claim with
  | mass r =>
    simp only [checkMass] at h
    simp only [ClaimHolds]
    cases hrow : rowValues c r with
    | some row =>
      rw [hrow] at h
      exact ⟨row, rfl, checkRow_sound row h⟩
    | none =>
      rw [hrow] at h
      exact absurd h Bool.false_ne_true

/-- Soundness: if the check accepts a certificate, every claim holds. -/
theorem check_sound (c : Certificate) (h : check c = true) : CertificateHolds c := by
  intro claim hmem
  unfold check at h
  exact checkMass_sound c claim (List.all_eq_true.mp h claim hmem)

end AxiomCheck.Check
