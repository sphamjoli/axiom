import AxiomCheck.Certificate
import AxiomCheck.Json
import AxiomCheck.Parse
import AxiomCheck.Rational
import AxiomCheck.Estimate
import AxiomCheck.Semantics
import AxiomCheck.Check
import AxiomCheck.Verdict

/-!
# AxiomCheck

The independent certificate checker. `AxiomCheck.Parse.certificate` reads a
version 1, 2, 3 or 4 certificate with a strict JSON reader and enforces the
format's structural rules, `AxiomCheck.Semantics` gives each claim its
meaning, and `AxiomCheck.Check.check` decides the claims with a proof of
soundness. `AxiomCheck.Estimate` decides the Hoeffding bound behind
`Estimate` claims; it is Mathlib-free, so the checker imports it.

`AxiomCheck.Theorems` and `AxiomCheck.Theorems.Estimate` prove the
semantics' dynamics, convergence, parametric, termination and Hoeffding
results for all parameters. They use Mathlib, so they are separate library
roots that this module, and therefore the checker entry point, does not
import.
-/
