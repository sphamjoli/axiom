import AxiomCheck.Certificate

/-!
# Verdict lines

The line the checker prints for each claim. An accepted claim carries its
certainty: `estimated` for an `Estimate` claim, `proven` for every other
kind. `axiom check` reads these lines and requires the word to match the
claim kind, so the label cannot be changed without the mismatch being
reported.
-/

namespace AxiomCheck.Verdict

/-- The certainty of an accepted claim: `estimated` for an `Estimate` claim,
whose check is about its sample count only, and `proven` for every other
kind. -/
def certainty : Claim → String
  | .estimate .. => "estimated"
  | _ => "proven"

/-- The line reporting the verdict on claim `index`: an accepted claim
carries its certainty, as in `claim 5 Estimate: accepted (estimated)`. -/
def verdictLine (index : Nat) (claim : Claim) (accepted : Bool) : String :=
  let verdict := if accepted then s!"accepted ({certainty claim})" else "rejected"
  match claim with
  | .mass row => s!"claim {index} Mass row {row}: {verdict}"
  | .fixedPoint _ => s!"claim {index} FixedPoint: {verdict}"
  | .contraction _ => s!"claim {index} Contraction: {verdict}"
  | .convergenceBound .. => s!"claim {index} ConvergenceBound: {verdict}"
  | .termination .. => s!"claim {index} Termination: {verdict}"
  | .parametricContraction => s!"claim {index} ParametricContraction: {verdict}"
  | .estimate .. => s!"claim {index} Estimate: {verdict}"

end AxiomCheck.Verdict
