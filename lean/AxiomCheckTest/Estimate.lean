import AxiomCheck.Estimate

/-!
# Hoeffding check tests

Evaluates `checkHoeffding` and `expLower` on cases with known answers. The
headline case is the engine's: an estimate within `1/100` at confidence
`0.999` needs `⌈ln 2000 / (2 · 10⁻⁴)⌉ = 38005` samples, the count
`axiom_sim::required_samples` computes, so 38005 must be accepted and 38004
rejected. Exits with a non-zero status if any case fails.
-/

open AxiomCheck.Estimate

namespace AxiomCheckEstimateTest

/-- One named case and whether it passed. -/
def cases : List (String × Bool) :=
  [ ("exp lower bound at zero is one", expLower 0 == 1)
  , ("exp lower bound at one is within 1e-9 below e",
      (2718281828 : Rat) / 1000000000 < expLower 1 && expLower 1 < (2718281829 : Rat) / 1000000000)
  , ("exp lower bound at ten is below exp 10",
      (220264 : Rat) / 10 < expLower 10 && expLower 10 < (2202647 : Rat) / 100)
  , ("negative arguments are clamped to zero", expLower (-5) == 1)
  , ("n = 38005 meets eps 1/100, delta 1/1000", checkHoeffding 38005 (1 / 100) (1 / 1000))
  , ("n = 38004 is too small for eps 1/100, delta 1/1000",
      !checkHoeffding 38004 (1 / 100) (1 / 1000))
  , ("a generous count is accepted", checkHoeffding 1000000 (1 / 100) (1 / 1000))
  , ("one sample is rejected", !checkHoeffding 1 (1 / 100) (1 / 1000))
  , ("delta zero is rejected", !checkHoeffding 1000000000 (1 / 2) 0)
  , ("negative delta is rejected", !checkHoeffding 1000000000 (1 / 2) (-1))
  , ("n = 26771 meets eps 1/50, delta 1e-9", checkHoeffding 26771 (1 / 50) (1 / 1000000000))
  , ("n = 26770 is too small for eps 1/50, delta 1e-9",
      !checkHoeffding 26770 (1 / 50) (1 / 1000000000))
  , ("a huge count stays cheap", checkHoeffding 18446744073709551615 (1 / 2) (1 / 1000)) ]

end AxiomCheckEstimateTest

open AxiomCheckEstimateTest in
/-- Runs every case and reports the failures. -/
def main : IO UInt32 := do
  let failures := cases.filter fun (_, passed) => !passed
  for (name, _) in failures do
    IO.eprintln s!"FAIL {name}"
  IO.println s!"{cases.length - failures.length} of {cases.length} estimate tests passed"
  pure (if failures.isEmpty then 0 else 1)
