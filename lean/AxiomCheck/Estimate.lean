import AxiomCheck.Semantics

/-!
# Estimate

The decision behind estimated claims. For `n` independent samples in
`[0, 1]`, Hoeffding's inequality bounds the chance that the sample mean
misses the true mean by `ε` or more by `2 exp(-2 n ε²)`. An estimate with
failure probability `δ` is justified when `2 exp(-2 n ε²) ≤ δ`, that is
when `exp x ≥ 2 / δ` for `x = 2 n ε²`.

`checkHoeffding` decides this over the rationals with `expLower`, a rational
*lower* bound on `exp x`, equivalently an upper bound on `exp (-x)`. The
direction is what makes the decision sound: a lower bound can only make the
check reject a sample count that is in fact enough, never accept one that is
not. An upper bound in its place would accept false estimates.
`AxiomCheck.Theorems.Estimate` proves that acceptance implies the real
inequality.

The bound clamps `x` to `[0, 256]`, halves it `s` times to `y = x / 2 ^ s`,
takes the partial sum `Σ_{k < 16} y ^ k / k!`, and squares back `s` times,
rounding down to 128 binary places after every step so the numbers stay
small. The cost is therefore bounded whatever the inputs. Estimates are
Estimated, never Proven: the guarantee is about the sampling, not the model.
The engine's `axiom_sim::exp_lower_bound` runs the same steps.
-/

namespace AxiomCheck.Estimate

open AxiomCheck.Semantics

/-- The number of series terms in the lower bound for `exp`. -/
def seriesTerms : Nat := 16

/-- The number of binary places kept when rounding down. -/
def precisionBits : Nat := 128

/-- The largest argument the bound looks at; larger ones are clamped. -/
def argumentCap : Rat := 256

/-- The most halvings the argument reduction performs. -/
def halvingFuel : Nat := 16

/-- The factorial, computed without Mathlib. -/
def fact : Nat → Nat
  | 0 => 1
  | k + 1 => (k + 1) * fact k

/-- The partial sum `Σ_{k < K} y ^ k / k!` of the exponential series. -/
def expSeries (y : Rat) : Nat → Rat
  | 0 => 0
  | k + 1 => expSeries y k + ratPow y k / (fact k : Rat)

/-- `max 0 (⌊z · 2 ^ P⌋ / 2 ^ P)`: a value at most `z` when `z ≥ 0`, with
`P = precisionBits` binary places. -/
def roundDown (z : Rat) : Rat :=
  let scale : Rat := ((2 ^ precisionBits : Nat) : Rat)
  let r : Rat := ((z * scale).floor : Rat) / scale
  if r < 0 then 0 else r

/-- Squares `z` and rounds down, `s` times. -/
def squareDown : Nat → Rat → Rat
  | 0, z => z
  | s + 1, z => squareDown s (roundDown (z * z))

/-- The halvings that bring `x` to at most one, up to `fuel`. The choice only
affects accuracy, never soundness. -/
def halvings : Nat → Rat → Nat
  | 0, _ => 0
  | fuel + 1, x => if x ≤ 1 then 0 else halvings fuel (x / 2) + 1

/-- `x` clamped to `[0, argumentCap]`. -/
def clampArg (x : Rat) : Rat :=
  if x < 0 then 0 else if argumentCap < x then argumentCap else x

/-- A rational lower bound on `exp x`, proved in
`AxiomCheck.Estimate.expLower_le_exp`. -/
def expLower (x : Rat) : Rat :=
  let c := clampArg x
  let s := halvings halvingFuel c
  squareDown s (roundDown (expSeries (c / ratPow 2 s) seriesTerms))

/-- Decides `δ ≥ 2 exp (-2 n ε²)` soundly: accepts only when `δ > 0` and
`2 ≤ δ · expLower (2 n ε²)`. Because `expLower` bounds `exp` from *below*,
acceptance implies the real inequality (`checkHoeffding_sound`); the check
may reject a count that is just barely enough, never accept one that is
not. -/
def checkHoeffding (n : Nat) (ε δ : Rat) : Bool :=
  decide (0 < δ) && decide (2 ≤ δ * expLower (2 * (n : Rat) * (ε * ε)))

end AxiomCheck.Estimate
