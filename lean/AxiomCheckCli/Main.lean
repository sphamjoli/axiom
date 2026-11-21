import AxiomCheck

/-!
# Certificate check entry point

Reads the certificate whose path is the single argument, parses it with
`AxiomCheck.Parse.certificate` and decides each claim with
`AxiomCheck.Check.checkMass`. Prints one line per claim, of the form
`claim 0 Mass row 0: accepted` or `claim 0 Mass row 0: rejected`. Exits with
status 0 when every claim is accepted, 1 when some claim is rejected, and 2
when the arguments are wrong or the certificate cannot be read or parsed.
`axiom check` runs it through the Lean interpreter (`lake env lean` in run
mode) from the `lean` directory.
-/

open AxiomCheck

namespace AxiomCheckCli

/-- The line reporting the verdict on claim `index`. -/
def verdictLine (index : Nat) (claim : Claim) (accepted : Bool) : String :=
  match claim with
  | .mass row =>
    s!"claim {index} Mass row {row}: {if accepted then "accepted" else "rejected"}"

/-- Decides every claim, printing one verdict line each, and returns the exit
status: 0 if all are accepted, 1 otherwise. -/
def report (certificate : Certificate) : IO UInt32 := do
  let mut allAccepted := true
  for (claim, index) in certificate.claims.zipIdx do
    let accepted := Check.checkMass certificate claim
    IO.println (verdictLine index claim accepted)
    allAccepted := allAccepted && accepted
  pure (if allAccepted then 0 else 1)

/-- Reads the file at `path` unless it is larger than the certificate size
limit or cannot be read. -/
def readBounded (path : System.FilePath) : IO (Except String String) := do
  try
    let metadata ← path.metadata
    if metadata.byteSize.toNat > Parse.maxCertificateBytes then
      pure (.error "certificate is too large")
    else
      pure (.ok (← IO.FS.readFile path))
  catch _ =>
    pure (.error "certificate cannot be read")

/-- Checks the certificate at `path` and returns the exit status. -/
def run (path : System.FilePath) : IO UInt32 := do
  match ← readBounded path with
  | .error reason =>
    IO.println s!"error: {reason}"
    pure 2
  | .ok text =>
    match Parse.certificate text with
    | .error reason =>
      IO.println s!"error: {reason}"
      pure 2
    | .ok certificate => report certificate

end AxiomCheckCli

/-- Entry point: expects exactly one argument, the certificate path. -/
def main (args : List String) : IO UInt32 :=
  match args with
  | [path] => AxiomCheckCli.run path
  | _ => do
    IO.println "error: expected exactly one argument, the certificate path"
    pure 2
