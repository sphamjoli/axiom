import Batteries.Data.Rat.Lemmas

/-!
# Rational helpers

The absolute value, natural powers and sums of rationals that the semantics
and the Hoeffding check share. They use Batteries' `Rat` and avoid Mathlib.
They live in the `AxiomCheck.Semantics` namespace beside the definitions that
use them.
-/

namespace AxiomCheck.Semantics

/-- The absolute value of a rational. -/
def ratAbs (x : Rat) : Rat := if x < 0 then -x else x

/-- `x` raised to the natural power `k`. -/
def ratPow (x : Rat) : Nat → Rat
  | 0 => 1
  | k + 1 => ratPow x k * x

/-- Multiplies `acc` by `x` `k` times, in a tail-recursive loop so that
evaluating a large power does not exhaust the interpreter's stack. -/
def ratPowLoop (x : Rat) : Rat → Nat → Rat
  | acc, 0 => acc
  | acc, k + 1 => ratPowLoop x (acc * x) k

/-- Starting from `x ^ j`, the loop computes `x ^ (j + k)`. -/
theorem ratPowLoop_eq (x : Rat) (j k : Nat) :
    ratPowLoop x (ratPow x j) k = ratPow x (j + k) := by
  induction k generalizing j with
  | zero => rfl
  | succ k ih =>
    show ratPowLoop x (ratPow x (j + 1)) k = _
    rw [ih, Nat.add_right_comm, Nat.add_assoc]

/-- `x` raised to the natural power `k`, by the loop. -/
def ratPowFast (x : Rat) (k : Nat) : Rat := ratPowLoop x 1 k

/-- Compiled code evaluates `ratPow` with the loop. -/
@[csimp] theorem ratPow_eq_fast : @ratPow = @ratPowFast := by
  funext x k
  have h := ratPowLoop_eq x 0 k
  rw [Nat.zero_add] at h
  exact h.symm

/-- The sum of a list of rationals. -/
def ratSum (xs : List Rat) : Rat := xs.foldr (· + ·) 0

/-- The sum of `f i` over every state `i` of an `n`-state model. -/
def sumFin {n : Nat} (f : Fin n → Rat) : Rat := ratSum ((List.finRange n).map f)

end AxiomCheck.Semantics
