import AxiomCheck.Certificate
import AxiomCheck.Json

/-!
# Reading certificates

Turns certificate text into a `Certificate`, enforcing the structural rules of
the version 1, 2, 3 and 4 formats: size, JSON syntax and fields, `format`,
`version`, model, fractions, then claims. Version 2 adds the `FixedPoint` and
`Contraction` claim kinds, which are only allowed in a certificate whose model
has exactly two states. Version 3 adds the `ConvergenceBound` claim kind for
two-state chains, the retry model `{"retry": {"success": p}}` as an
alternative to `{"rows": ...}`, and the `Termination` claim kind for retry
models; step and attempt counts are capped at `maxCount`. Version 4 adds the
parametric model `{"parametric": {"a": interval, "b": interval}}`, where an
interval is `{"lo": f, "hi": f, "lo_end": e, "hi_end": e}` with probability
ends and each `e` the string `open` or `closed` (the JSON reader has no
booleans), and the `ParametricContraction` claim kind for it.

The reader leaves the arithmetic of claims to the claim check: the sum of each
row, and whether a fixed point, contraction factor, convergence bound or
termination claim holds. A certificate is valid when this reader accepts it
and every claim checks; the engine's reader does both at once. The shared
fixtures in `fixtures/certificates/`, each with the rule it breaks, test that
the two agree on every case they cover; agreement is not proved. Error
messages report positions and counts, never certificate text.
-/

namespace AxiomCheck.Parse

open AxiomCheck.Json

/-- The largest certificate, in UTF-8 bytes. -/
def maxCertificateBytes : Nat := 1024 * 1024

/-- The largest number of states a model may have. -/
def maxStates : Nat := 16

/-- The longest numerator or denominator, in decimal digits. -/
def maxFractionDigits : Nat := 64

/-- The largest step count of a `ConvergenceBound` claim and attempt count of
a `Termination` claim's `failure_after`. It bounds the size of the powers the
checker computes. -/
def maxCount : Nat := 1000

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

/-- The members of an object, provided its keys include every key of
`required`, each once, and otherwise only keys of `optional`. -/
def someMembers (required optional : List String) :
    Value → Except String (List (String × Value))
  | .obj members =>
    let keys := members.map Prod.fst
    if keys.eraseDups.length != keys.length then .error "repeated field"
    else if keys.all (fun k => required.contains k || optional.contains k) &&
        required.all keys.contains then .ok members
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

/-- A non-negative fraction with no upper bound: canonical digits, positive
denominator and lowest terms. -/
def unboundedFraction (value : Value) : Except String Fraction := do
  let members ← exactMembers ["num", "den"] value
  let (.str numText) ← field members "num" | .error "num must be a string"
  let (.str denText) ← field members "den" | .error "den must be a string"
  let (some num, some den) := (canonicalNat numText, canonicalNat denText)
    | .error "fraction is not in canonical form"
  if den == 0 || Nat.gcd num den != 1 then .error "fraction is not in canonical form"
  else pure ⟨num, den⟩

/-- A step or attempt count: an integer no greater than `maxCount`. -/
def count : Value → Except String Nat
  | .nat n => if n > maxCount then .error "count is above the limit" else .ok n
  | _ => .error "count must be an integer"

/-- A `Contraction` factor: like `fraction`, except that the numerator may be
`-` followed by canonical non-zero digits, and there is no upper bound. `-0`,
`+`, a repeated sign and a signed denominator are rejected. -/
def signedFraction (value : Value) : Except String SignedFraction := do
  let members ← exactMembers ["num", "den"] value
  let (.str numText) ← field members "num" | .error "num must be a string"
  let (.str denText) ← field members "den" | .error "den must be a string"
  let (negative, magnitude) := match numText.toList with
    | '-' :: rest => (true, String.mk rest)
    | _ => (false, numText)
  let (some num, some den) := (canonicalNat magnitude, canonicalNat denText)
    | .error "fraction is not in canonical form"
  if den == 0 || Nat.gcd num den != 1 || (negative && num == 0) then
    .error "fraction is not in canonical form"
  else pure ⟨negative, num, den⟩

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

/-- The `kind` of a claim object. The object's full set of fields is checked
afterwards, once the kind says which fields it must have. -/
def claimKind : Value → Except String String
  | .obj members =>
    match members.lookup "kind" with
    | some (.str kind) => .ok kind
    | some _ => .error "kind must be a string"
    | none => .error "unexpected or missing field"
  | _ => .error "expected an object"

/-- Rejects a claim kind that needs version `needed` in an older certificate,
and a two-state claim about a model that is not a chain of exactly two
states. -/
def requireTwoState (needed version : Nat) (model : Model) : Except String Unit :=
  if version < needed then .error s!"claim kind needs certificate version {needed}"
  else match model with
    | .retry _ | .parametric _ _ => .error "claim kind needs a chain model"
    | .chain rows => if rows.length != 2 then .error "claim kind needs a two-state model" else pure ()

/-- The `failure_after` object of a `Termination` claim. -/
def failureAfter (value : Value) : Except String (Nat × Fraction) := do
  let members ← exactMembers ["attempts", "probability"] value
  pure (← count (← field members "attempts"), ← fraction (← field members "probability"))

/-- A single claim of a certificate of version `version` with model `model`.
A `Mass` claim's row must exist in a chain; `FixedPoint` and `Contraction`
claims need version 2 and a two-state chain; `ConvergenceBound` needs version
3 and a two-state chain; `Termination` needs version 3 and a retry model. -/
def claim (version : Nat) (model : Model) (value : Value) : Except String Claim := do
  match ← claimKind value with
  | "Mass" =>
    let .chain rows := model | .error "claim kind needs a chain model"
    let members ← exactMembers ["kind", "row"] value
    let (.nat row) ← field members "row" | .error "row must be an integer"
    if row ≥ rows.length then .error "claim refers to a missing row"
    else pure (.mass row)
  | "FixedPoint" =>
    requireTwoState 2 version model
    let members ← exactMembers ["kind", "value"] value
    pure (.fixedPoint (← fraction (← field members "value")))
  | "Contraction" =>
    requireTwoState 2 version model
    let members ← exactMembers ["kind", "factor"] value
    pure (.contraction (← signedFraction (← field members "factor")))
  | "ConvergenceBound" =>
    requireTwoState 3 version model
    let members ← exactMembers ["kind", "start", "steps", "tolerance"] value
    let start ← fraction (← field members "start")
    let steps ← count (← field members "steps")
    let tolerance ← unboundedFraction (← field members "tolerance")
    if tolerance.num == 0 then .error "tolerance must be positive"
    else pure (.convergenceBound start steps tolerance)
  | "Termination" =>
    if version < 3 then .error "claim kind needs certificate version 3"
    let .retry _ := model | .error "claim kind needs a retry model"
    let members ← someMembers ["kind", "probability"] ["expected_attempts", "failure_after"] value
    let probability ← fraction (← field members "probability")
    let expected ← (members.lookup "expected_attempts").mapM unboundedFraction
    let failure ← (members.lookup "failure_after").mapM failureAfter
    pure (.termination probability expected failure)
  | "ParametricContraction" =>
    if version < 4 then .error "claim kind needs certificate version 4"
    let .parametric _ _ := model | .error "claim kind needs a parametric model"
    let _ ← exactMembers ["kind"] value
    pure .parametricContraction
  | _ => .error "unknown claim kind"

/-- An interval end: `open` excludes the end value, `closed` includes it. -/
def intervalEnd : Value → Except String Bool
  | .str "open" => .ok true
  | .str "closed" => .ok false
  | _ => .error "interval end must be open or closed"

/-- An interval of a parametric model: two probability ends, each open or
closed, containing at least one value. -/
def interval (value : Value) : Except String Interval := do
  let members ← exactMembers ["lo", "hi", "lo_end", "hi_end"] value
  let lo ← fraction (← field members "lo")
  let hi ← fraction (← field members "hi")
  let loOpen ← intervalEnd (← field members "lo_end")
  let hiOpen ← intervalEnd (← field members "hi_end")
  let below := lo.num * hi.den < hi.num * lo.den
  let equal := lo.num * hi.den == hi.num * lo.den
  if below || (equal && !loOpen && !hiOpen) then pure ⟨lo, hi, loOpen, hiOpen⟩
  else .error "interval is empty"

/-- The model of a certificate of version `version`: an object with exactly
one of `rows`, a transition matrix, `retry`, a retry loop that needs
version 3, and `parametric`, a parametric two-state chain that needs
version 4. -/
def model (version : Nat) (value : Value) : Except String Model := do
  match value with
  | .obj [("rows", rows)] => pure (.chain (← matrix rows))
  | .obj [("retry", retry)] =>
    if version < 3 then .error "retry model needs certificate version 3"
    let members ← exactMembers ["success"] retry
    pure (.retry (← fraction (← field members "success")))
  | .obj [("parametric", parametric)] =>
    if version < 4 then .error "parametric model needs certificate version 4"
    let members ← exactMembers ["a", "b"] parametric
    pure (.parametric (← interval (← field members "a")) (← interval (← field members "b")))
  | .obj _ => .error "model must have exactly one of rows, retry and parametric"
  | _ => .error "expected an object"

/-- The claims: non-empty and distinct. -/
def claims (version : Nat) (model : Model) (value : Value) :
    Except String (List Claim) := do
  let parsed ← (← array value).mapM (claim version model)
  if parsed.isEmpty then .error "certificate makes no claims"
  else if parsed.eraseDups.length != parsed.length then .error "a claim is repeated"
  else pure parsed

/-- Reads and structurally validates a version 1, 2, 3 or 4 certificate. -/
def certificate (text : String) : Except String Certificate := do
  if text.utf8ByteSize > maxCertificateBytes then .error "certificate is too large"
  let root ← match Json.parse text with
    | .ok root => pure root
    | .error e => .error s!"invalid JSON at character {e.position}: {e.reason}"
  let members ← exactMembers ["format", "version", "model", "claims"] root
  let (.str format) ← field members "format" | .error "format must be a string"
  if format != "axiom-certificate" then .error "certificate format is not axiom-certificate"
  let (.nat version) ← field members "version" | .error "version must be an integer"
  if version < 1 || version > 4 then .error s!"unsupported certificate version"
  let parsedModel ← model version (← field members "model")
  let claimList ← claims version parsedModel (← field members "claims")
  pure ⟨parsedModel, claimList⟩

end AxiomCheck.Parse
