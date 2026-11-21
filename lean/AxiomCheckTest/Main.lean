import AxiomCheck

/-!
# Checker tests

Runs the strict JSON reader and the certificate reader against the shared
fixtures in `fixtures/certificates/` and against reader-specific cases. Every
rejection fixture must be rejected for the reason `reject-reasons.tsv` gives:
the reader's error, or `check` when its fault is arithmetic and the claim
check rejects it. Exits with a non-zero status if any case fails.
-/

open AxiomCheck

namespace AxiomCheckTest

/-- The shared certificate fixtures, relative to the `lean` directory. -/
def fixtureDirectory : System.FilePath := ".." / "fixtures" / "certificates"

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

/-- Whether `text` parses and every claim checks. -/
def checks (text : String) : Option Bool :=
  match Parse.certificate text with
  | .ok certificate => some (Check.check certificate)
  | .error _ => none

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
        "golden file was not read as the link model"
    , expect "reads the version 2 golden file as the link model with its two-state claims"
        (match Parse.certificate (← IO.FS.readFile (fixtureDirectory / "links-fixed-point.json")) with
         | .ok certificate =>
           certificate.rows == [[⟨9, 10⟩, ⟨1, 10⟩], [⟨3, 10⟩, ⟨7, 10⟩]] &&
           certificate.claims ==
             [.mass 0, .mass 1, .fixedPoint ⟨1, 4⟩, .contraction ⟨false, 3, 5⟩]
         | .error _ => false)
        "version 2 golden file was not read as the link model" ]

/-- The checker's verdict on certificate text: the reader's error message,
`check` when the reader accepts it and some claim fails, or `accepted`. -/
def verdict (text : String) : String :=
  match Parse.certificate text with
  | .ok certificate => if Check.check certificate then "accepted" else "check"
  | .error reason => reason

/-- One case per rejection fixture, comparing the checker's verdict with the
reason in `reject-reasons.tsv`, plus one case that the table names exactly the
fixtures present. -/
def rejectionCases : IO (List Outcome) := do
  let table ← IO.FS.readFile (fixtureDirectory / "reject-reasons.tsv")
  let expected := (table.splitOn "
").filterMap fun line =>
    match line.splitOn "	" with
    | [name, _engine, reason] => some (name, reason)
    | _ => none
  let entries ← (fixtureDirectory / "reject").readDir
  let names : List String := (entries.map (·.fileName)).toList.mergeSort (· ≤ ·)
  let cases ← names.mapM fun (name : String) => do
    let found := verdict (← IO.FS.readFile (fixtureDirectory / "reject" / name))
    pure (expect s!"rejects {name} for its rule" (expected.lookup name == some found)
      s!"verdict was {found}")
  pure (expect "the reason table names exactly the rejection fixtures"
    (expected.map Prod.fst == names) "reason table and fixtures differ" :: cases)

/-- One case per acceptance fixture. -/
def acceptanceCases : IO (List Outcome) := do
  let entries ← (fixtureDirectory / "accept").readDir
  let names : List String := (entries.map (·.fileName)).toList.mergeSort (· ≤ ·)
  names.mapM fun (name : String) => do
    let text ← IO.FS.readFile (fixtureDirectory / "accept" / name)
    pure (expect s!"accepts and checks {name}" (checks text == some true)
      "rejected an acceptance fixture")


/-- The golden certificate with row 0 replaced by `row`. -/
def goldenWithRow0 (row : List Fraction) : Certificate :=
  ⟨.chain [row, [⟨3, 10⟩, ⟨7, 10⟩]], [.mass 0, .mass 1]⟩

/-- The link model's rows. -/
def linkRows : List (List Fraction) := [[⟨9, 10⟩, ⟨1, 10⟩], [⟨3, 10⟩, ⟨7, 10⟩]]


/-- Checker cases for two-state claims that the fixtures do not cover. -/
def twoStateCases : IO (List Outcome) := do
  let golden ← IO.FS.readFile (fixtureDirectory / "links-fixed-point.json")
  pure
    [ expect "checks the version 2 golden file" (checks golden == some true)
        "version 2 golden file did not check"
    , expect "rejects factor -3/5" (!Check.check ⟨.chain linkRows, [.contraction ⟨true, 3, 5⟩]⟩)
        "accepted -3/5"
    , expect "rejects a two-state claim about a one-state model"
        (!Check.check ⟨.chain [[⟨1, 1⟩]], [.fixedPoint ⟨1, 1⟩]⟩) "accepted a one-state model" ]

/-- Checker cases for `Mass` claims at the boundaries of a row sum. -/
def checkCases : IO (List Outcome) := do
  let golden ← IO.FS.readFile (fixtureDirectory / "links-mass.json")
  pure
    [ expect "checks the version 1 golden file" (checks golden == some true)
        "golden file did not check"
    , expect "rejects a row summing to 999/1000"
        (!Check.check (goldenWithRow0 [⟨899, 1000⟩, ⟨1, 10⟩])) "accepted 999/1000"
    , expect "rejects a row summing to 1001/1000"
        (!Check.check (goldenWithRow0 [⟨901, 1000⟩, ⟨1, 10⟩])) "accepted 1001/1000"
    , expect "rejects an entry above one"
        (!Check.check (goldenWithRow0 [⟨3, 2⟩, ⟨0, 1⟩])) "accepted 3/2"
    , expect "rejects a claim on a missing row"
        (!Check.check ⟨.chain [[⟨1, 1⟩]], [.mass 1]⟩) "accepted a missing row" ]

/-- Theorem smoke tests: evaluation cases for the semantics the theorems are
about, the link model
`a = 1/10`, `b = 3/10` and a retry loop with `p = 3/10`. -/
def semanticsCases : List Outcome :=
  let link : Semantics.TwoState := ⟨1 / 10, 3 / 10⟩
  let retry : Semantics.Retry := ⟨3 / 10⟩
  [ expect "link factor is 3/5" (link.lam == 3 / 5) s!"factor {link.lam}"
  , expect "link steady state is 1/4" (link.fixedPoint == 1 / 4) s!"steady state {link.fixedPoint}"
  , expect "certifies the bound 1/1000 after 13 steps from 1"
      (decide (Semantics.ConvergenceBoundCertified link 1 13 (1 / 1000))) "did not certify 13 steps"
  , expect "does not certify the bound 1/1000 after 12 steps from 1"
      (!decide (Semantics.ConvergenceBoundCertified link 1 12 (1 / 1000))) "certified 12 steps"
  , expect "retry expects 10/3 attempts" (retry.expectedAttempts == 10 / 3)
      s!"expected {retry.expectedAttempts}"
  , expect "retry fails 20 times with probability (7/10)^20"
      (retry.failureAfter 20 == 79792266297612001 / 100000000000000000000 &&
        Semantics.ratPow (7 / 10) 20 == 79792266297612001 / 100000000000000000000)
      s!"failure probability {retry.failureAfter 20}"
  ]

/-- Cases for the version 3 claims beyond the shared fixtures: both golden
files, the alternation of the periodic chain, and the cost of the largest step
count. -/
def versionThreeCases : IO (List Outcome) := do
  let links ← IO.FS.readFile (fixtureDirectory / "links-convergence.json")
  let retry ← IO.FS.readFile (fixtureDirectory / "retry-termination.json")
  let periodic : Semantics.TwoState := ⟨1, 1⟩
  pure
    [ expect "reads and checks the convergence golden file"
        (match Parse.certificate links with
         | .ok c => c.claims.getLast? == some (.convergenceBound ⟨1, 1⟩ 13 ⟨1, 1000⟩) && Check.check c
         | .error _ => false)
        "convergence golden file did not check"
    , expect "reads and checks the termination golden file"
        (match Parse.certificate retry with
         | .ok c => c.model == .retry ⟨3, 10⟩ &&
           c.claims == [.termination ⟨1, 1⟩ (some ⟨10, 3⟩)
             (some (20, ⟨79792266297612001, 100000000000000000000⟩))] && Check.check c
         | .error _ => false)
        "termination golden file did not check"
    , expect "the periodic chain alternates"
        ((List.range 6).map (periodic.iterate 1) == [1, 0, 1, 0, 1, 0])
        "periodic chain did not alternate"
    , expect "checks the largest step count in time"
        (!Check.check ⟨.chain [[⟨999999, 1000000⟩, ⟨1, 1000000⟩], [⟨1, 1000000⟩, ⟨999999, 1000000⟩]],
          [.convergenceBound ⟨1, 1⟩ 1000 ⟨1, 10 ^ 60⟩]⟩)
        "accepted a bound the slow chain does not reach" ]

/-- Cases for the version 4 golden files: the parametric links model with
`a` and `b` in `]0, 1[`, and the links model with its steady-state estimate,
whose verdict line says it is estimated, not proven. -/
def versionFourCases : IO (List Outcome) := do
  let param ← IO.FS.readFile (fixtureDirectory / "links-param.json")
  let estimate ← IO.FS.readFile (fixtureDirectory / "links-estimate.json")
  let openUnit : Interval := ⟨⟨0, 1⟩, ⟨1, 1⟩, true, true⟩
  pure
    [ expect "reads and checks the parametric golden file"
        (match Parse.certificate param with
         | .ok c => c.model == .parametric openUnit openUnit &&
           c.claims == [.parametricContraction] && Check.check c
         | .error _ => false)
        "parametric golden file did not check"
    , expect "reads and checks the estimate golden file"
        (match Parse.certificate estimate with
         | .ok c => c.claims.getLast? ==
             some (.estimate .steadyState ⟨9583, 38005⟩ 38005 ⟨1, 100⟩ ⟨1, 1000⟩ 42) &&
           Check.check c
         | .error _ => false)
        "estimate golden file did not check"
    , expect "labels an accepted estimate as estimated and other claims as proven"
        (Verdict.verdictLine 5 (.estimate .success ⟨1, 2⟩ 1 ⟨1, 2⟩ ⟨1, 2⟩ 0) true ==
            "claim 5 Estimate: accepted (estimated)" &&
          Verdict.verdictLine 0 (.mass 0) true == "claim 0 Mass row 0: accepted (proven)" &&
          Verdict.verdictLine 1 .parametricContraction false ==
            "claim 1 ParametricContraction: rejected")
        "verdict lines are mislabelled" ]

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
    readerCases ++ (← goldenCases) ++ (← checkCases) ++ (← twoStateCases) ++ (← versionThreeCases) ++ (← versionFourCases) ++ (← acceptanceCases) ++ (← rejectionCases) ++ semanticsCases ++ limitCases
  let failures := outcomes.filterMap fun outcome => outcome.failure.map (outcome.name, ·)
  for (name, why) in failures do
    IO.eprintln s!"FAIL {name}: {why}"
  IO.println s!"{outcomes.length - failures.length} of {outcomes.length} checker tests passed"
  pure (if failures.isEmpty then 0 else 1)
