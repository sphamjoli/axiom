import AxiomCheck.Estimate
import AxiomCheck.Theorems
import Mathlib.Data.Complex.Exponential
import Mathlib.Data.Rat.Floor

/-!
# Soundness of the Hoeffding check

`checkHoeffding n ε δ = true` implies `2 * Real.exp (-2 * n * ε ^ 2) ≤ δ`
over the reals. The proof shows that every step of `expLower` stays below
`Real.exp`: the series partial sum is below `exp y` for `y ≥ 0`, rounding
down keeps a value below any bound it was already below, squaring preserves
`0 ≤ z ≤ exp y ↦ z ^ 2 ≤ exp (2 y)`, and clamping only lowers the argument.
This module uses Mathlib, so it is a separate library root that the checker
does not import.
-/

namespace AxiomCheck.Estimate

open AxiomCheck.Semantics

/-- `fact` is the factorial. -/
theorem fact_eq_factorial (k : ℕ) : fact k = k.factorial := by
  induction k with
  | zero => rfl
  | succ k ih => simp [fact, Nat.factorial_succ, ih]

/-- `expSeries` is the partial sum of the exponential series. -/
theorem expSeries_cast (y : ℚ) (K : ℕ) :
    ((expSeries y K : ℚ) : ℝ) = ∑ i ∈ Finset.range K, (y : ℝ) ^ i / (i.factorial : ℝ) := by
  induction K with
  | zero => simp [expSeries]
  | succ K ih =>
    rw [expSeries, Finset.sum_range_succ, ← ih, ratPow_eq_pow, fact_eq_factorial]
    push_cast
    ring

/-- For `y ≥ 0` the partial sum is a lower bound on `exp y`. -/
theorem expSeries_le_exp {y : ℚ} (hy : 0 ≤ y) (K : ℕ) :
    ((expSeries y K : ℚ) : ℝ) ≤ Real.exp y := by
  rw [expSeries_cast]
  exact Real.sum_le_exp_of_nonneg (by exact_mod_cast hy) K

/-- Rounding down keeps a value below any non-negative bound it was below,
and the result is non-negative. -/
theorem roundDown_le {z : ℚ} {E : ℝ} (hz : (z : ℝ) ≤ E) (hE : 0 ≤ E) :
    ((roundDown z : ℚ) : ℝ) ≤ E ∧ 0 ≤ ((roundDown z : ℚ) : ℝ) := by
  unfold roundDown
  simp only
  split
  · simp [hE]
  · rename_i hneg
    have hs : (0 : ℚ) < ((2 ^ precisionBits : ℕ) : ℚ) := by positivity
    have hf : (((z * ((2 ^ precisionBits : ℕ) : ℚ)).floor : ℤ) : ℚ) ≤
        z * ((2 ^ precisionBits : ℕ) : ℚ) := Rat.le_floor.mp le_rfl
    have hr : (((z * ((2 ^ precisionBits : ℕ) : ℚ)).floor : ℤ) : ℚ) /
        ((2 ^ precisionBits : ℕ) : ℚ) ≤ z := (div_le_iff₀ hs).mpr hf
    have hr' : ((((((z * ((2 ^ precisionBits : ℕ) : ℚ)).floor : ℤ) : ℚ) /
        ((2 ^ precisionBits : ℕ) : ℚ)) : ℚ) : ℝ) ≤ (z : ℝ) := by exact_mod_cast hr
    refine ⟨le_trans hr' hz, ?_⟩
    exact_mod_cast not_lt.mp hneg

/-- Squaring and rounding down `s` times turns a lower bound on `exp y` into
a lower bound on `exp (2 ^ s * y)`. -/
theorem squareDown_le (s : ℕ) :
    ∀ (z : ℚ) (y : ℝ), 0 ≤ (z : ℝ) → (z : ℝ) ≤ Real.exp y →
      ((squareDown s z : ℚ) : ℝ) ≤ Real.exp (2 ^ s * y) := by
  induction s with
  | zero =>
    intro z y _ hz
    simpa [squareDown] using hz
  | succ s ih =>
    intro z y hz0 hz
    have hsq : ((z * z : ℚ) : ℝ) ≤ Real.exp (2 * y) := by
      push_cast
      rw [show 2 * y = y + y by ring, Real.exp_add]
      exact mul_le_mul hz hz hz0 (Real.exp_pos y).le
    obtain ⟨hle, hnn⟩ := roundDown_le hsq (Real.exp_pos _).le
    have := ih (roundDown (z * z)) (2 * y) hnn hle
    rw [squareDown]
    calc ((squareDown s (roundDown (z * z)) : ℚ) : ℝ) ≤ Real.exp (2 ^ s * (2 * y)) := this
      _ = Real.exp (2 ^ (s + 1) * y) := by rw [pow_succ]; ring_nf

/-- The clamped argument is non-negative. -/
theorem clampArg_nonneg (x : ℚ) : 0 ≤ clampArg x := by
  unfold clampArg argumentCap
  split_ifs with h1 h2 <;> linarith

/-- For `x ≥ 0` the clamped argument is at most `x`. -/
theorem clampArg_le {x : ℚ} (hx : 0 ≤ x) : clampArg x ≤ x := by
  unfold clampArg
  split_ifs with h1 h2 <;> linarith

/-- `expLower x` is a lower bound on `exp x` for every `x ≥ 0`. -/
theorem expLower_le_exp {x : ℚ} (hx : 0 ≤ x) : ((expLower x : ℚ) : ℝ) ≤ Real.exp x := by
  unfold expLower
  simp only
  set c := clampArg x with hcdef
  set s := halvings halvingFuel c
  have hc0 : 0 ≤ c := clampArg_nonneg x
  have hcx : c ≤ x := clampArg_le hx
  have hpow : ratPow 2 s = 2 ^ s := ratPow_eq_pow 2 s
  have hpos : (0 : ℚ) < 2 ^ s := by positivity
  have hy : 0 ≤ c / ratPow 2 s := by rw [hpow]; exact div_nonneg hc0 hpos.le
  obtain ⟨hle, hnn⟩ := roundDown_le (expSeries_le_exp hy seriesTerms) (Real.exp_pos _).le
  have hsq := squareDown_le s _ _ hnn hle
  have hback : (2 : ℝ) ^ s * ((c / ratPow 2 s : ℚ) : ℝ) = (c : ℝ) := by
    rw [hpow]
    push_cast
    field_simp
  rw [hback] at hsq
  exact le_trans hsq (Real.exp_le_exp.mpr (by exact_mod_cast hcx))

/-- Soundness of `checkHoeffding`: when it accepts, Hoeffding's failure
bound `2 exp (-2 n ε²)` is at most `δ`, so `n` samples in `[0, 1]` estimate
their mean within `ε` with probability at least `1 - δ`. The direction
matters: the check uses a lower bound on `exp (2 n ε²)`, so it never accepts
a sample count that is too small. -/
theorem checkHoeffding_sound (n : ℕ) (ε δ : ℚ) (h : checkHoeffding n ε δ = true) :
    2 * Real.exp (-2 * (n : ℝ) * (ε : ℝ) ^ 2) ≤ (δ : ℝ) := by
  simp only [checkHoeffding, Bool.and_eq_true, decide_eq_true_eq] at h
  obtain ⟨hδ, hbound⟩ := h
  have hx : (0 : ℚ) ≤ 2 * (n : ℚ) * (ε * ε) := mul_nonneg (by positivity) (mul_self_nonneg ε)
  have hL := expLower_le_exp hx
  have hδr : (0 : ℝ) ≤ (δ : ℝ) := by exact_mod_cast hδ.le
  have h2 : (2 : ℝ) ≤ (δ : ℝ) * Real.exp ((2 * (n : ℚ) * (ε * ε) : ℚ) : ℝ) :=
    calc (2 : ℝ) ≤ (δ : ℝ) * ((expLower (2 * (n : ℚ) * (ε * ε)) : ℚ) : ℝ) := by
          exact_mod_cast hbound
      _ ≤ _ := mul_le_mul_of_nonneg_left hL hδr
  have hneg : -2 * (n : ℝ) * (ε : ℝ) ^ 2 = -(((2 * (n : ℚ) * (ε * ε) : ℚ)) : ℝ) := by
    push_cast
    ring
  rw [hneg, Real.exp_neg, ← div_eq_mul_inv, div_le_iff₀ (Real.exp_pos _)]
  exact h2

end AxiomCheck.Estimate
