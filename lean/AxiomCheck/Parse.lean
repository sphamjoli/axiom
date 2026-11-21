import AxiomCheck.Certificate
import AxiomCheck.Json

/-!
# Reading certificates

Turns certificate text into a `Certificate`, enforcing the structural rules of
the version 1 format: size, JSON syntax and fields, `format`, `version`, matrix
shape, fractions, then claims. It accepts the same certificates as the
engine's reader, except that it leaves the sum of each row to the claim check;
the two readers may report a different first error for the same invalid
certificate.
Whether each row sums to one is a claim, so it is checked by the checker, not
here. Error messages report positions and counts, never certificate text.
-/

namespace AxiomCheck.Parse

open AxiomCheck.Json

/-- The largest certificate, in UTF-8 bytes. -/
def maxCertificateBytes : Nat := 1024 * 1024

/-- The largest number of states a model may have. -/
def maxStates : Nat := 16

/-- The longest numerator or denominator, in decimal digits. -/
def maxFractionDigits : Nat := 64

/-- The largest row index the format can express. -/
def maxRowIndex : Nat := 2 ^ 64 - 1

/-- The members of an object, provided its keys are exactly `expected`, each
once, in any order. -/
def exactMembers (expected : List String) :
    Value → Except String (List (String × Value))
  | .obj members =>
    let keys := members.map Prod.fst
    if keys.length > expected.length then .error "unexpected or missing field"
    else if keys.eraseDups.length != keys.length then .error "repeated field"
    else if keys.all expected.contains && expected.all keys.contains then .ok members
    else .error "unexpected or missing field"
  | _ => .error "expected an object"

/-- The value of field `key`, which `exactMembers` has shown is present. -/
def field (members : List (String × Value)) (key : String) : Except String Value :=
  match members.lookup key with
  | some value => .ok value
  | none => .error "missing field"

/-- A decimal string with no sign, no leading zero except `0` itself, and at
most `maxFractionDigits` digits, read as a number. -/
def canonicalNat (text : String) : Option Nat :=
  let digits := text.toList
  if digits.isEmpty || digits.length > maxFractionDigits then none
  else if !digits.all Char.isDigit then none
  else if digits.length > 1 && digits.head? == some '0' then none
  else some (digits.foldl (fun total d => total * 10 + (d.toNat - '0'.toNat)) 0)

/-- A fraction object: canonical digits, positive denominator, lowest terms,
and no greater than one. -/
def fraction (value : Value) : Except String Fraction := do
  let members ← exactMembers ["num", "den"] value
  let (.str numText) ← field members "num" | .error "num must be a string"
  let (.str denText) ← field members "den" | .error "den must be a string"
  let (some num, some den) := (canonicalNat numText, canonicalNat denText)
    | .error "fraction is not in canonical form"
  if den == 0 || Nat.gcd num den != 1 then .error "fraction is not in canonical form"
  else if num > den then .error "entry is above one"
  else pure ⟨num, den⟩

/-- A list of values, as an array. -/
def array : Value → Except String (List Value)
  | .arr items => .ok items
  | _ => .error "expected an array"

/-- The transition matrix: 1 to `maxStates` rows, each with one entry per
row, checked before any fraction is read. -/
def matrix (value : Value) : Except String (List (List Fraction)) := do
  let rows ← (← array value).mapM array
  if rows.isEmpty then .error "model has no rows"
  else if rows.length > maxStates then .error "model has too many states"
  else if rows.any (·.length != rows.length) then .error "matrix is not square"
  else rows.mapM (·.mapM fraction)

/-- A single claim, whose row must exist. -/
def claim (states : Nat) (value : Value) : Except String Claim := do
  let members ← exactMembers ["kind", "row"] value
  let (.str kind) ← field members "kind" | .error "kind must be a string"
  if kind != "Mass" then .error "unknown claim kind"
  let (.nat row) ← field members "row" | .error "row must be an integer"
  if row > maxRowIndex || row ≥ states then .error "claim refers to a missing row"
  else pure (.mass row)

/-- The claims: non-empty and distinct. -/
def claims (states : Nat) (value : Value) : Except String (List Claim) := do
  let parsed ← (← array value).mapM (claim states)
  if parsed.isEmpty then .error "certificate makes no claims"
  else if parsed.eraseDups.length != parsed.length then .error "a claim is repeated"
  else pure parsed

/-- Reads and structurally validates a version 1 certificate. -/
def certificate (text : String) : Except String Certificate := do
  if text.utf8ByteSize > maxCertificateBytes then .error "certificate is too large"
  let root ← match Json.parse text with
    | .ok root => pure root
    | .error e => .error s!"invalid JSON at character {e.position}: {e.reason}"
  let members ← exactMembers ["format", "version", "model", "claims"] root
  let (.str format) ← field members "format" | .error "format must be a string"
  if format != "axiom-certificate" then .error "certificate format is not axiom-certificate"
  let (.nat version) ← field members "version" | .error "version must be an integer"
  if version != 1 then .error s!"unsupported certificate version"
  let model ← exactMembers ["rows"] (← field members "model")
  let rows ← matrix (← field model "rows")
  let claimList ← claims rows.length (← field members "claims")
  pure ⟨rows, claimList⟩

end AxiomCheck.Parse
