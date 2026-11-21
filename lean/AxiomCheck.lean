import AxiomCheck.Certificate
import AxiomCheck.Json
import AxiomCheck.Parse
import AxiomCheck.Semantics
import AxiomCheck.Check

/-!
# AxiomCheck

The independent certificate checker. `AxiomCheck.Parse.certificate` reads a
version 1 or 2 certificate with a strict JSON reader and enforces the format's
structural rules, `AxiomCheck.Semantics` gives each claim its meaning, and
`AxiomCheck.Check.check` decides the claims with a proof of soundness.

`AxiomCheck.Theorems` proves the semantics' dynamics, convergence and
termination results for all parameters. It uses Mathlib, so it is a separate
library root that this module, and therefore the checker entry point, does
not import.
-/
