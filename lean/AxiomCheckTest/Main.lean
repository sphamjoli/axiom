import AxiomCheck

/-!
# Checker tests

Runs the strict JSON reader and the certificate reader against the shared
fixtures in `fixtures/certificates/` and against reader-specific cases. Every
rejection fixture must be rejected, except the one whose fault is arithmetic:
the reader accepts it structurally and the claim check
rejects it. Exits with a non-zero status if any case fails.
-/

open AxiomCheck

namespace AxiomCheckTest

/-- The shared certificate fixtures, relative to the `lean` directory. -/
def fixtureDirectory : System.FilePath := ".." / "fixtures" / "certificates"

/-- Rejection fixtures whose fault is checked by the claim checker, not the
reader. -/
def arithmeticFixtures : List String := ["does-not-sum-to-one.json"]

/-- The outcome of one test case: its name and, on failure, why. -/
structure Outcome where
  /-- What was tested. -/
  name : String
  /-- Why the case failed, or `none` if it passed. -/
  failure : Option String

/-- A case that passes when `ok` holds. -/
def expect (name : String) (ok : Bool) (why : String) : Outcome :=
  ⟨name, if ok then none else some why⟩

/-- Whether reading `text` as JSON succeeds. -/
def jsonAccepts (text : String) : Bool :=
  match Json.parse text with
  | .ok _ => true
  | .error _ => false

/-- The number of members of an object, or `none` for other values. -/
def memberCount : Json.Value → Option Nat
  | .obj members => some members.length
  | _ => none

/-- Cases for the strict JSON reader, including every input on which Lean's
built-in parser differs from the certificate format. -/
def readerCases : List Outcome :=
  [ expect "keeps repeated keys"
      (match Json.parse "{\"a\":1,\"a\":2}" with
       | .ok value => memberCount value == some 2
       | .error _ => false)
      "repeated keys were merged"
  , expect "rejects an exponent" (!jsonAccepts "1e0") "accepted 1e0"
  , expect "rejects a fraction" (!jsonAccepts "1.0") "accepted 1.0"
  , expect "rejects a sign" (!jsonAccepts "-0") "accepted -0"
  , expect "rejects a leading zero" (!jsonAccepts "01") "accepted 01"
  , expect "rejects a byte order mark" (!jsonAccepts "\uFEFF{}") "accepted a byte order mark"
  , expect "rejects literals" (!jsonAccepts "true" && !jsonAccepts "null") "accepted a literal"
  , expect "rejects a trailing comma" (!jsonAccepts "[1,]") "accepted [1,]"
  , expect "rejects trailing data" (!jsonAccepts "{} {}") "accepted trailing data"
  , expect "rejects deep nesting" (!jsonAccepts (String.mk (List.replicate 9 '[') ++ String.mk (List.replicate 9 ']')))
      "accepted nine levels"
  , expect "accepts the deepest allowed nesting" (jsonAccepts (String.mk (List.replicate 8 '[') ++ String.mk (List.replicate 8 ']')))
      "rejected eight levels"
  , expect "rejects an unterminated string" (!jsonAccepts "\"abc") "accepted an unterminated string"
  , expect "rejects a control character" (!jsonAccepts "\"a\nb\"") "accepted a raw newline in a string"
  , expect "rejects a lone surrogate" (!jsonAccepts "\"\\uD800\"") "accepted a lone surrogate"
  , expect "decodes escapes"
      (match Json.parse "\"\\u0031\\n\\uD83D\\uDE00\"" with
       | .ok (.str text) => text == "1\n" ++ String.singleton (Char.ofNat 0x1F600)
       | _ => false)
      "escapes were not decoded"
  , expect "accepts an object with the most members allowed"
      (jsonAccepts ("{" ++ ",".intercalate ((List.range 8).map fun i => s!"\"k{i}\":0") ++ "}"))
      "rejected 8 members"
  , expect "rejects an object with too many members"
      (!jsonAccepts ("{" ++ ",".intercalate ((List.range 9).map fun i => s!"\"k{i}\":0") ++ "}"))
      "accepted 9 members"
  , expect "rejects thousands of members without exhausting the stack"
      (!jsonAccepts ("{" ++ ",".intercalate ((List.range 5000).map fun i => s!"\"k{i}\":0") ++ "}"))
      "accepted 5000 members"
  , expect "accepts an array with the most items allowed"
      (jsonAccepts ("[" ++ ",".intercalate (List.replicate 64 "0") ++ "]"))
      "rejected 64 items"
  , expect "rejects an array with too many items"
      (!jsonAccepts ("[" ++ ",".intercalate (List.replicate 65 "0") ++ "]"))
      "accepted 65 items"
  , expect "rejects an integer with 21 digits" (!jsonAccepts (String.mk (List.replicate 21 '1')))
      "accepted 21 digits"
  ]

/-- Whether reading `text` as a certificate succeeds. -/
def certificateAccepts (text : String) : Bool :=
  match Parse.certificate text with
  | .ok _ => true
  | .error _ => false

/-- Cases for the shared golden file. -/
def goldenCases : IO (List Outcome) := do
  let text ← IO.FS.readFile (fixtureDirectory / "links-mass.json")
  pure
    [ expect "reads the golden file as the link model"
        (match Parse.certificate text with
         | .ok certificate =>
           certificate.rows == [[⟨9, 10⟩, ⟨1, 10⟩], [⟨3, 10⟩, ⟨7, 10⟩]] &&
           certificate.claims == [.mass 0, .mass 1]
         | .error _ => false)
        "golden file was not read as the link model" ]

/-- One case per rejection fixture. -/
def rejectionCases : IO (List Outcome) := do
  let entries ← (fixtureDirectory / "reject").readDir
  let names : List String := (entries.map (·.fileName)).toList.mergeSort (· ≤ ·)
  names.mapM fun (name : String) => do
    let text ← IO.FS.readFile (fixtureDirectory / "reject" / name)
    let accepted := certificateAccepts text
    if arithmeticFixtures.contains name then
      pure (expect s!"reader accepts {name} structurally" accepted "rejected an arithmetic fixture")
    else
      pure (expect s!"rejects {name}" (!accepted) "accepted a rejection fixture")

/-- One case per acceptance fixture. -/
def acceptanceCases : IO (List Outcome) := do
  let entries ← (fixtureDirectory / "accept").readDir
  let names : List String := (entries.map (·.fileName)).toList.mergeSort (· ≤ ·)
  names.mapM fun (name : String) => do
    let text ← IO.FS.readFile (fixtureDirectory / "accept" / name)
    pure (expect s!"accepts {name}" (certificateAccepts text) "rejected an acceptance fixture")

/-- Whether `text` parses and every claim checks. -/
def checks (text : String) : Option Bool :=
  match Parse.certificate text with
  | .ok certificate => some (Check.check certificate)
  | .error _ => none

/-- The golden certificate with row 0 replaced by `row`. -/
def goldenWithRow0 (row : List Fraction) : Certificate :=
  ⟨[row, [⟨3, 10⟩, ⟨7, 10⟩]], [.mass 0, .mass 1]⟩

/-- Cases for the claim checker. -/
def checkCases : IO (List Outcome) := do
  let golden ← IO.FS.readFile (fixtureDirectory / "links-mass.json")
  let arithmetic ← IO.FS.readFile (fixtureDirectory / "reject" / "does-not-sum-to-one.json")
  pure
    [ expect "checks the golden file" (checks golden == some true) "golden file did not check"
    , expect "parses but rejects does-not-sum-to-one.json" (checks arithmetic == some false)
        "arithmetic fixture was not parsed and rejected"
    , expect "rejects a row summing to 999/1000"
        (!Check.check (goldenWithRow0 [⟨899, 1000⟩, ⟨1, 10⟩])) "accepted 999/1000"
    , expect "rejects a row summing to 1001/1000"
        (!Check.check (goldenWithRow0 [⟨901, 1000⟩, ⟨1, 10⟩])) "accepted 1001/1000"
    , expect "rejects an entry above one"
        (!Check.check (goldenWithRow0 [⟨3, 2⟩, ⟨0, 1⟩])) "accepted 3/2"
    , expect "rejects a claim on a missing row"
        (!Check.check ⟨[[⟨1, 1⟩]], [.mass 1]⟩) "accepted a missing row"
    , expect "accepts the unmutated golden model"
        (Check.check (goldenWithRow0 [⟨9, 10⟩, ⟨1, 10⟩])) "rejected the golden model" ]

/-- Cases for size and shape limits that are impractical as fixture files. -/
def limitCases : List Outcome :=
  let half := "{\"num\":\"1\",\"den\":\"2\"}"
  let certificateWith (rows : String) :=
    "{\"format\":\"axiom-certificate\",\"version\":1,\"model\":{\"rows\":" ++ rows ++
      "},\"claims\":[{\"kind\":\"Mass\",\"row\":0}]}"
  let uniform (states : Nat) :=
    let entry := "{\"num\":\"1\",\"den\":\"" ++ toString states ++ "\"}"
    let row := "[" ++ ",".intercalate (List.replicate states entry) ++ "]"
    certificateWith ("[" ++ ",".intercalate (List.replicate states row) ++ "]")
  let digits (n : Nat) := String.mk ('1' :: List.replicate (n - 1) '0')
  [ expect "accepts the largest allowed model" (certificateAccepts (uniform 16)) "rejected 16 states"
  , expect "rejects one state more than the limit" (!certificateAccepts (uniform 17)) "accepted 17 states"
  , expect "rejects oversized text" (!certificateAccepts (String.mk (List.replicate (1024 * 1024 + 1) ' ')))
      "accepted more than 1 MiB"
  , expect "accepts 64-digit fractions"
      (certificateAccepts (certificateWith
        ("[[{\"num\":\"" ++ String.mk (List.replicate 63 '9') ++ "\",\"den\":\"" ++ digits 64 ++
          "\"},{\"num\":\"1\",\"den\":\"" ++ digits 64 ++ "\"}],[" ++ half ++ "," ++ half ++ "]]")))
      "rejected 64 digits"
  , expect "rejects 65-digit fractions"
      (!certificateAccepts (certificateWith
        ("[[{\"num\":\"1\",\"den\":\"" ++ digits 65 ++ "\"},{\"num\":\"1\",\"den\":\"1\"}],[" ++ half ++
          "," ++ half ++ "]]")))
      "accepted 65 digits"
  , expect "rejects an entry above one"
      (!certificateAccepts (certificateWith ("[[{\"num\":\"3\",\"den\":\"2\"}," ++ half ++ "],[" ++ half ++ "," ++ half ++ "]]")))
      "accepted 3/2"
  ]

end AxiomCheckTest

open AxiomCheckTest in
/-- Runs every case, prints failures, and returns 1 if any failed. -/
def main : IO UInt32 := do
  let outcomes :=
    readerCases ++ (← goldenCases) ++ (← checkCases) ++ (← acceptanceCases) ++ (← rejectionCases) ++ limitCases
  let failures := outcomes.filterMap fun outcome => outcome.failure.map (outcome.name, ·)
  for (name, why) in failures do
    IO.eprintln s!"FAIL {name}: {why}"
  IO.println s!"{outcomes.length - failures.length} of {outcomes.length} checker tests passed"
  pure (if failures.isEmpty then 0 else 1)
