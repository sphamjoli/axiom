import AxiomCheck.Certificate
import AxiomCheck.Json
import AxiomCheck.Parse
import AxiomCheck.Semantics

/-!
# AxiomCheck

The independent certificate checker. `AxiomCheck.Parse.certificate` reads a
version 1 certificate with a strict JSON reader and enforces the format's
structural rules. The formal semantics and the claim checks are still to be
written.
-/
