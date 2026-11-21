import AxiomCheck.Check
import Mathlib.Algebra.Order.Field.Basic
import Mathlib.Algebra.BigOperators.Fin
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.FieldSimp
import Mathlib.Tactic.Ring

/-!
# Theorems

Proofs about the semantics, stated once for all parameters. The checker does
not depend on this module; it uses Mathlib, which `Semantics` and `Check`
avoid. The theorems live in the `AxiomCheck.Semantics` namespace beside the
definitions they are about.

* Dynamics: a step of a chain whose rows sum to one preserves total mass; the
  state-one component of a two-state chain's step is the affine update; the
  stationary probability is a fixed point; the error contracts by `λ ^ t`.
* Convergence: a certified bound `|λ ^ n (d₀ - d*)| ≤ ε` with `|λ| ≤ 1`
  implies `|d_t - d*| ≤ ε` for every `t ≥ n`, and perturbations decay as
  `|λ| ^ t`.
* Two-state parameters: for `0 < a + b < 2`, `d*` is the unique fixed point
  and the update is a strict contraction.
* Retry termination: the closed-form success probability is the least fixed
  point, and the expected attempts and failure probabilities have their
  closed forms.
* Full soundness: if the checker accepts a certificate, every claim holds in
  its full meaning, including `ConvergenceBoundHolds` and `TerminationHolds`.
-/

namespace AxiomCheck.Semantics

/-! ## Rational helpers -/

/-- `ratAbs` is the absolute value. -/
theorem ratAbs_eq_abs (x : ℚ) : ratAbs x = |x| := by
  unfold ratAbs
  split
  · rename_i h
    exact (abs_of_neg h).symm
  · rename_i h
    exact (abs_of_nonneg (not_lt.mp h)).symm

/-- `ratPow` is the natural power. -/
theorem ratPow_eq_pow (x : ℚ) (k : ℕ) : ratPow x k = x ^ k := by
  induction k with
  | zero => rfl
  | succ k ih => simp [ratPow, ih, pow_succ]

/-- `ratSum` is the list sum. -/
theorem ratSum_eq_sum (xs : List ℚ) : ratSum xs = xs.sum := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp [ratSum, List.foldr_cons] at ih ⊢; rw [ih]

/-- `sumFin` is the sum over all states. -/
theorem sumFin_eq_sum {n : ℕ} (f : Fin n → ℚ) : sumFin f = ∑ i, f i := by
  rw [sumFin, ratSum_eq_sum, Fin.sum_univ_def]

/-! ## Dynamics -/

/-- A step of a chain whose rows each sum to one preserves the total mass of
any vector, in particular of any distribution. -/
theorem Chain.step_mass {n : ℕ} (m : Chain n) (hrows : ∀ i, sumFin (m.rows i) = 1)
    (d : Fin n → ℚ) : sumFin (m.step d) = sumFin d := by
  simp only [Chain.step, sumFin_eq_sum] at hrows ⊢
  rw [Finset.sum_comm]
  refine Finset.sum_congr rfl fun i _ => ?_
  rw [← Finset.mul_sum, hrows i, mul_one]

/-- A step of a well-formed chain preserves total mass. -/
theorem Chain.step_mass_of_wellFormed {n : ℕ} (m : Chain n) (hm : m.WellFormed)
    (d : Fin n → ℚ) : sumFin (m.step d) = sumFin d :=
  Chain.step_mass m (fun i => (hm i).2) d

/-- For a distribution `d` over two states, the state-one component of a step
of the two-state chain is the affine update `(1 - a - b) d₁ + a`. -/
theorem TwoState.step_one (m : TwoState) (d : Fin 2 → ℚ) (hd : d 0 + d 1 = 1) :
    m.chain.step d 1 = m.update (d 1) := by
  have h0 : d 0 = 1 - d 1 := by linarith
  simp only [Chain.step, sumFin_eq_sum, Fin.sum_univ_two, TwoState.chain, TwoState.entry,
    TwoState.update, TwoState.lam, TwoState.offset, h0]
  simp
  ring

/-- The stationary probability `a / (a + b)` is a fixed point of the update
when `a + b ≠ 0`. -/
theorem TwoState.fixedPoint_holds (m : TwoState) (h : m.a + m.b ≠ 0) :
    FixedPointHolds m m.fixedPoint := by
  unfold FixedPointHolds TwoState.update TwoState.lam TwoState.offset TwoState.fixedPoint
  field_simp
  ring

/-- One update scales the distance to the stationary probability by `λ`. -/
theorem TwoState.update_sub_fixedPoint (m : TwoState) (h : m.a + m.b ≠ 0) (d : ℚ) :
    m.update d - m.fixedPoint = m.lam * (d - m.fixedPoint) := by
  have hfp := m.fixedPoint_holds h
  unfold FixedPointHolds TwoState.update at hfp
  unfold TwoState.update
  linarith

/-- The error identity `d_t - d* = λ ^ t (d₀ - d*)`, when `a + b ≠ 0`. -/
theorem TwoState.iterate_sub_fixedPoint (m : TwoState) (h : m.a + m.b ≠ 0) (d : ℚ) (t : ℕ) :
    m.iterate d t - m.fixedPoint = m.lam ^ t * (d - m.fixedPoint) := by
  induction t with
  | zero => simp [TwoState.iterate]
  | succ t ih =>
    rw [TwoState.iterate, m.update_sub_fixedPoint h, ih, pow_succ]
    ring

/-! ## Convergence -/

/-- The key convergence theorem: when `a + b ≠ 0` and `|λ| ≤ 1`, the certified
bound `|λ ^ n (d₀ - d*)| ≤ ε` implies `|d_t - d*| ≤ ε` for every `t ≥ n`. -/
theorem convergenceBound_holds (m : TwoState) (start : ℚ) (steps : ℕ) (tolerance : ℚ)
    (hab : m.a + m.b ≠ 0) (hlam : ratAbs m.lam ≤ 1)
    (hcert : ConvergenceBoundCertified m start steps tolerance) :
    ConvergenceBoundHolds m start steps tolerance := by
  intro t ht
  unfold ConvergenceBoundCertified at hcert
  rw [ratAbs_eq_abs] at hlam ⊢
  rw [ratAbs_eq_abs, ratPow_eq_pow, abs_mul, abs_pow] at hcert
  rw [m.iterate_sub_fixedPoint hab, abs_mul, abs_pow]
  have hpow : |m.lam| ^ t ≤ |m.lam| ^ steps :=
    pow_le_pow_of_le_one (abs_nonneg _) hlam ht
  calc |m.lam| ^ t * |start - m.fixedPoint|
      ≤ |m.lam| ^ steps * |start - m.fixedPoint| :=
        mul_le_mul_of_nonneg_right hpow (abs_nonneg _)
    _ ≤ tolerance := hcert

/-- Stability: a perturbation `r` of the starting point is carried to
`λ ^ t r` after `t` updates. -/
theorem TwoState.iterate_perturbation (m : TwoState) (d r : ℚ) (t : ℕ) :
    m.iterate (d + r) t - m.iterate d t = m.lam ^ t * r := by
  induction t with
  | zero => simp [TwoState.iterate]
  | succ t ih =>
    have e : m.iterate (d + r) (t + 1) - m.iterate d (t + 1) =
        m.lam * (m.iterate (d + r) t - m.iterate d t) := by
      simp only [TwoState.iterate, TwoState.update]
      ring
    rw [e, ih, pow_succ]
    ring

/-- Stability corollary: a perturbation `r` decays as `|λ| ^ t |r|`. -/
theorem TwoState.perturbation_decay (m : TwoState) (d r : ℚ) (t : ℕ) :
    ratAbs (m.iterate (d + r) t - m.iterate d t) = ratAbs m.lam ^ t * ratAbs r := by
  rw [m.iterate_perturbation, ratAbs_eq_abs, ratAbs_eq_abs, ratAbs_eq_abs, abs_mul, abs_pow]

/-! ## Two-state parameters -/

/-- For `a, b ∈ [0, 1]` with `0 < a + b < 2`, a value is a fixed point of the
update exactly when it is `a / (a + b)`. -/
theorem TwoState.fixedPoint_unique (m : TwoState) (_ha : 0 ≤ m.a ∧ m.a ≤ 1)
    (_hb : 0 ≤ m.b ∧ m.b ≤ 1) (hpos : 0 < m.a + m.b) (_hlt : m.a + m.b < 2) (v : ℚ) :
    FixedPointHolds m v ↔ v = m.fixedPoint := by
  have hne : m.a + m.b ≠ 0 := ne_of_gt hpos
  constructor
  · intro hv
    unfold FixedPointHolds TwoState.update TwoState.lam TwoState.offset at hv
    unfold TwoState.fixedPoint
    rw [eq_div_iff hne]
    linarith
  · rintro rfl
    exact m.fixedPoint_holds hne

/-- For `a, b ∈ [0, 1]` with `0 < a + b < 2`, the update is a strict
contraction: `|1 - a - b| < 1`. -/
theorem TwoState.lam_contraction (m : TwoState) (_ha : 0 ≤ m.a ∧ m.a ≤ 1)
    (_hb : 0 ≤ m.b ∧ m.b ≤ 1) (hpos : 0 < m.a + m.b) (hlt : m.a + m.b < 2) :
    ratAbs m.lam < 1 := by
  rw [ratAbs_eq_abs, abs_lt]
  unfold TwoState.lam
  constructor <;> linarith

/-- The parametric two-state theorem: for `a, b ∈ [0, 1]` with
`0 < a + b < 2`, `a / (a + b)` is the unique fixed point and the update is a
strict contraction with factor `λ`. -/
theorem TwoState.parametric (m : TwoState) (ha : 0 ≤ m.a ∧ m.a ≤ 1)
    (hb : 0 ≤ m.b ∧ m.b ≤ 1) (hpos : 0 < m.a + m.b) (hlt : m.a + m.b < 2) :
    (∀ v, FixedPointHolds m v ↔ v = m.fixedPoint) ∧ ContractionHolds m m.lam :=
  ⟨m.fixedPoint_unique ha hb hpos hlt, m.lam_contraction ha hb hpos hlt, rfl⟩

/-! ## Retry termination -/

/-- When `0 < p ≤ 1`, success is certain: one is the least fixed point. -/
theorem Retry.successProbability_of_pos (m : Retry) (hp : 0 < m.p) (_hp1 : m.p ≤ 1) :
    m.IsLeastFixedPoint m.successProbability := by
  have hne : m.p ≠ 0 := ne_of_gt hp
  simp only [Retry.successProbability, hne, if_false]
  refine ⟨⟨zero_le_one, le_refl 1⟩, by simp [Retry.map], ?_⟩
  intro y _ _ hy
  unfold Retry.map at hy
  have h : m.p * (1 - y) = 0 := by linarith
  rcases mul_eq_zero.mp h with h | h
  · exact absurd h hne
  · linarith

/-- When `p = 0`, the attempts never succeed: zero is the least fixed point. -/
theorem Retry.successProbability_of_zero (m : Retry) (hp : m.p = 0) :
    m.IsLeastFixedPoint m.successProbability := by
  simp only [Retry.successProbability, hp, if_true]
  refine ⟨⟨le_refl 0, zero_le_one⟩, by simp [Retry.map, hp], ?_⟩
  intro y hy _ _
  exact hy

/-- For every `p ∈ [0, 1]`, the closed-form success probability satisfies the
`Termination` claim. -/
theorem Retry.termination_holds (m : Retry) (hp : 0 ≤ m.p) (hp1 : m.p ≤ 1) :
    TerminationHolds m m.successProbability := by
  rcases hp.lt_or_eq with h | h
  · exact m.successProbability_of_pos h hp1
  · exact m.successProbability_of_zero h.symm

/-- When `p > 0`, the expected number of attempts `1 / p` solves
`E = 1 + (1 - p) E`. -/
theorem Retry.expectedAttempts_equation (m : Retry) (hp : 0 < m.p) :
    m.expectedAttempts = 1 + (1 - m.p) * m.expectedAttempts := by
  have hne : m.p ≠ 0 := ne_of_gt hp
  unfold Retry.expectedAttempts
  field_simp

/-- The probability that each of the first `k` attempts fails is
`(1 - p) ^ k`. -/
theorem Retry.failureAfter_eq (m : Retry) (k : ℕ) :
    m.failureAfter k = ratPow (1 - m.p) k := by
  rw [ratPow_eq_pow]
  induction k with
  | zero => simp [Retry.failureAfter, Retry.successWithin]
  | succ k ih =>
    unfold Retry.failureAfter at ih ⊢
    rw [Retry.successWithin, pow_succ, ← ih]
    unfold Retry.map
    ring

/-! ## Full soundness of the checker -/

/-- The full meaning of a claim: for `convergenceBound`, every iterate from
step `steps` on is within `tolerance` of the stationary probability; for
`termination`, `probability` is the least fixed point of the retry equation,
the expected attempts solve `E = 1 + (1 - p) E` with `p > 0`, and the failure
probability is that of the first `k` attempts all failing. The other kinds
mean what `ClaimHolds` says. -/
def ClaimMeans (c : Certificate) : Claim → Prop
  | .convergenceBound start steps tolerance =>
    ∃ m, twoState c = some m ∧
      ConvergenceBoundHolds m (Fraction.toRat start) steps (Fraction.toRat tolerance)
  | .termination probability expected failure =>
    ∃ m, retryModel c = some m ∧ TerminationHolds m (Fraction.toRat probability) ∧
      (∀ e, expected = some e → 0 < m.p ∧ Fraction.toRat e = m.expectedAttempts ∧
        m.expectedAttempts = 1 + (1 - m.p) * m.expectedAttempts) ∧
      (∀ k q, failure = some (k, q) → Fraction.toRat q = m.failureAfter k)
  | claim => ClaimHolds c claim

/-- The certified facts of a claim imply its full meaning. -/
theorem claimHolds_means (c : Certificate) (claim : Claim) (h : ClaimHolds c claim) :
    ClaimMeans c claim := by
  cases claim with
  | mass r => exact h
  | fixedPoint v => exact h
  | contraction f => exact h
  | convergenceBound start steps tolerance =>
    obtain ⟨m, hm, hab, hlam, hcert⟩ := h
    exact ⟨m, hm, convergenceBound_holds m _ steps _ hab hlam hcert⟩
  | termination probability expected failure =>
    obtain ⟨m, hm, ⟨hp0, hp1⟩, hprob, hexp, hfail⟩ := h
    refine ⟨m, hm, ?_, ?_, ?_⟩
    · rw [hprob]
      exact m.termination_holds hp0 hp1
    · intro e he
      have hsome : expected.map Fraction.toRat = some (Fraction.toRat e) := by rw [he]; rfl
      obtain ⟨hne, heq⟩ := hexp _ hsome
      have hpos : 0 < m.p := lt_of_le_of_ne hp0 (Ne.symm hne)
      exact ⟨hpos, heq, m.expectedAttempts_equation hpos⟩
    · intro k q hf
      have hsome : failureToRat failure = some (k, Fraction.toRat q) := by
        rw [hf]; rfl
      rw [hfail k _ hsome, m.failureAfter_eq]

/-- Full soundness: if the checker accepts a certificate, every claim holds in
its full meaning. -/
theorem check_sound_full (c : Certificate) (h : Check.check c = true) :
    ∀ claim ∈ c.claims, ClaimMeans c claim :=
  fun claim hmem => claimHolds_means c claim (Check.check_sound c h claim hmem)

end AxiomCheck.Semantics
