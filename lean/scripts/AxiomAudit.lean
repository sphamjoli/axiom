import AxiomCheck
import Lean

/-!
# Axiom audit

Kernel-level audit run by `scripts/checkLeanAxioms.sh`. Every constant declared
in a module of the `AxiomCheck` library must depend only on `propext`,
`Classical.choice`, and `Quot.sound`; any other axiom, including `sorryAx` and
`Lean.ofReduceBool`, is reported as an error. The audit also fails if no
`AxiomCheck` module is imported, so renaming the library cannot silently turn
it into a no-op.
-/

open Lean Elab Command

/-- The axioms a checker declaration may depend on. -/
def allowedAxioms : List Name := [``propext, ``Classical.choice, ``Quot.sound]

/-- Reports every `AxiomCheck` declaration that depends on a disallowed axiom. -/
elab "#audit_axioms" : command => do
  let env ← getEnv
  unless env.header.moduleNames.any (·.getRoot == `AxiomCheck) do
    logError "no AxiomCheck module is imported, so nothing would be audited"
  let mut audited := 0
  for (name, _) in env.constants.map₁.toList do
    let some moduleIdx := env.getModuleIdxFor? name | continue
    let some moduleName := env.header.moduleNames[moduleIdx.toNat]? | continue
    unless moduleName.getRoot == `AxiomCheck do continue
    audited := audited + 1
    let disallowed := (← collectAxioms name).filter (· ∉ allowedAxioms)
    unless disallowed.isEmpty do
      logError m!"{name} depends on disallowed axioms: {disallowed.toList}"
  logInfo m!"Audited {audited} AxiomCheck declarations."

#audit_axioms
