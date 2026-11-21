import AxiomCheck.Estimate
import AxiomCheck.Json

/-!
# Hoeffding check tests

Evaluates `expLower` and `checkHoeffding` on the shared cases in
`fixtures/estimate/cases.json`, which the engine's `axiom-sim` tests read
too, so the checker and the engine must agree on every number. Each `exp`
case gives the exact rounded-down bound; each `required` case gives the least
accepted sample count, which must be accepted while the count below it is
rejected; each `hoeffding` case gives a verdict. Numbers are decimal strings
because the strict reader accepts only non-negative integers. Exits with a
non-zero status if any case fails or the file cannot be read.
-/

open AxiomCheck
open AxiomCheck.Estimate

namespace AxiomCheckEstimateTest

/-- The shared estimate cases, relative to the `lean` directory. -/
def casesFile : System.FilePath := ".." / "fixtures" / "estimate" / "cases.json"

/-- The member `key` of an object. -/
def member (value : Json.Value) (key : String) : Except String Json.Value :=
  match value with
  | .obj members =>
    match members.find? (·.1 == key) with
    | some (_, found) => pure found
    | none => throw s!"missing member {key}"
  | _ => throw s!"expected an object around {key}"

/-- The string member `key` of an object. -/
def text (value : Json.Value) (key : String) : Except String String := do
  match ← member value key with
  | .str s => pure s
  | _ => throw s!"member {key} is not a string"

/-- The array member `key` of an object. -/
def items (value : Json.Value) (key : String) : Except String (List Json.Value) := do
  match ← member value key with
  | .arr list => pure list
  | _ => throw s!"member {key} is not an array"

/-- The natural number written as the string member `key`. -/
def natural (value : Json.Value) (key : String) : Except String Nat := do
  match (← text value key).toNat? with
  | some n => pure n
  | none => throw s!"member {key} is not a natural number"

/-- The fraction `{"num", "den"}` in member `key`, with a non-zero
denominator. -/
def fraction (value : Json.Value) (key : String) : Except String Rat := do
  let object ← member value key
  let some num := (← text object "num").toInt? | throw s!"{key}.num is not an integer"
  let some den := (← text object "den").toNat? | throw s!"{key}.den is not a natural number"
  if den == 0 then throw s!"{key}.den is zero"
  pure ((num : Rat) / (den : Rat))

/-- One named case and whether it passed. -/
abbrev Case := String × Bool

/-- An `exp` case: the bound at `x` equals the exact value. -/
def expCase (value : Json.Value) : Except String Case := do
  let x ← fraction value "x"
  pure (s!"exp lower bound at {x}", expLower x == (← fraction value "bound"))

/-- A `required` case: the least accepted count is accepted, and the count
below it rejected. -/
def requiredCases (value : Json.Value) : Except String (List Case) := do
  let ε ← fraction value "epsilon"
  let δ ← fraction value "delta"
  let n ← natural value "samples"
  pure
    [ (s!"n = {n} meets eps {ε}, delta {δ}", checkHoeffding n ε δ)
    , (s!"n = {n - 1} is too small for eps {ε}, delta {δ}", n > 0 && !checkHoeffding (n - 1) ε δ) ]

/-- A `hoeffding` case: the verdict matches. -/
def hoeffdingCase (value : Json.Value) : Except String Case := do
  let n ← natural value "n"
  let ε ← fraction value "epsilon"
  let δ ← fraction value "delta"
  let accept ← match ← text value "verdict" with
    | "accept" => pure true
    | "reject" => pure false
    | _ => throw "verdict is neither accept nor reject"
  pure (s!"verdict for n = {n}, eps {ε}, delta {δ}", checkHoeffding n ε δ == accept)

/-- Every case in the shared file. -/
def cases (root : Json.Value) : Except String (List Case) := do
  let exp ← (← items root "exp").mapM expCase
  let required ← (← items root "required").mapM requiredCases
  let hoeffding ← (← items root "hoeffding").mapM hoeffdingCase
  if exp.isEmpty || required.isEmpty || hoeffding.isEmpty then
    throw "a case list is empty"
  pure (exp ++ required.flatten ++ hoeffding)

end AxiomCheckEstimateTest

open AxiomCheckEstimateTest in
/-- Runs every case and reports the failures. -/
def main : IO UInt32 := do
  let source ← IO.FS.readFile casesFile
  let parsed := match Json.parse source with
    | .ok root => cases root
    | .error e => .error s!"cases.json is not valid at {e.position}: {e.reason}"
  match parsed with
  | .error why =>
    IO.eprintln s!"FAIL reading the shared cases: {why}"
    pure 1
  | .ok all =>
    let failures := all.filter fun (_, passed) => !passed
    for (name, _) in failures do
      IO.eprintln s!"FAIL {name}"
    IO.println s!"{all.length - failures.length} of {all.length} estimate tests passed"
    pure (if failures.isEmpty then 0 else 1)
