import AxiomCheck

/-!
# Certificate check entry point

Reads the certificate whose path is the single argument, parses it with
`AxiomCheck.Parse.certificate` and decides each claim with
`AxiomCheck.Check.checkClaim`. Prints one line per claim, such as
`claim 0 Mass row 0: accepted` or `claim 2 FixedPoint: rejected`. Exits with
status 0 when every claim is accepted, 1 when some claim is rejected, and 2
when the arguments are wrong or the certificate cannot be read or parsed.
`axiom check` runs it through the Lean interpreter (`lake env lean` in run
mode) from the `lean` directory.
-/

open AxiomCheck

namespace AxiomCheckCli

/-- The line reporting the verdict on claim `index`. -/
def verdictLine (index : Nat) (claim : Claim) (accepted : Bool) : String :=
  let verdict := if accepted then "accepted" else "rejected"
  match claim with
  | .mass row => s!"claim {index} Mass row {row}: {verdict}"
  | .fixedPoint _ => s!"claim {index} FixedPoint: {verdict}"
  | .contraction _ => s!"claim {index} Contraction: {verdict}"
  | .convergenceBound .. => s!"claim {index} ConvergenceBound: {verdict}"
  | .termination .. => s!"claim {index} Termination: {verdict}"

/-- Decides every claim, printing one verdict line each, and returns the exit
status: 0 if all are accepted, 1 otherwise. -/
def report (certificate : Certificate) : IO UInt32 := do
  let mut allAccepted := true
  for (claim, index) in certificate.claims.zipIdx do
    let accepted := Check.checkClaim certificate claim
    IO.println (verdictLine index claim accepted)
    allAccepted := allAccepted && accepted
  pure (if allAccepted then 0 else 1)

/-- Reads at most `remaining` more bytes from `handle` onto `acc`, stopping at
the end of the file. Each read returns at least one byte unless the file has
ended, so `fuel` reads always suffice when `fuel` exceeds `remaining`. -/
def readUpTo (handle : IO.FS.Handle) : Nat → Nat → ByteArray → IO ByteArray
  | 0, _, acc => pure acc
  | fuel + 1, remaining, acc => do
    if remaining == 0 then return acc
    let chunk ← handle.read (USize.ofNat (min remaining 65536))
    if chunk.isEmpty then return acc
    readUpTo handle fuel (remaining - chunk.size) (acc ++ chunk)

/-- Reads the file at `path` unless it is not a regular file, is larger than
the certificate size limit, is not UTF-8, or cannot be read. The read goes
through a handle capped at one byte past the limit, so a device or a file that
grows after its size is checked cannot make the checker read without bound. -/
def readBounded (path : System.FilePath) : IO (Except String String) := do
  try
    let metadata ← path.metadata
    if metadata.type != .file then
      return .error "certificate is not a regular file"
    let limit := Parse.maxCertificateBytes + 1
    let bytes ← IO.FS.withFile path .read fun handle =>
      readUpTo handle (limit + 1) limit ByteArray.empty
    if bytes.size > Parse.maxCertificateBytes then
      return .error "certificate is too large"
    match String.fromUTF8? bytes with
    | some text => pure (.ok text)
    | none => pure (.error "certificate is not UTF-8")
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
