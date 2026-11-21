// Shapes returned by the analysis engine. They mirror what crates/axiom-wasm
// serialises, so every exact quantity is a fraction of decimal integer
// strings (arbitrary precision, never a float).

export interface Fraction {
  num: string;
  den: string;
}

export interface Transition {
  from: string;
  to: string;
  p: Fraction;
}

export interface Chain {
  name: string;
  states: string[];
  transitions: Transition[];
}

export interface SeriesPoint {
  t: number;
  value: Fraction;
  // Floating approximation for plotting only; never shown as a label.
  approx: number;
}

// The affine update d' = a d + b of the probability of the tracked state.
export interface Convergence {
  state: string;
  a: Fraction;
  b: Fraction;
  initial: Fraction;
  fixedPoint: Fraction;
  epsilon: Fraction;
  // First step from which d_t stays within epsilon of the fixed point, or
  // null when the series stops before reaching it.
  horizon: number | null;
  series: SeriesPoint[];
}

export interface Retry {
  success: Fraction;
  successProbability: Fraction;
  // Null when an attempt never succeeds.
  expectedAttempts: Fraction | null;
}

// "unchecked" marks a claim the engine computed but the Lean checker has not
// verified; the browser never runs the checker, so it reports only these.
export type ClaimStatus = "proven" | "estimated" | "rejected" | "unchecked";

export interface Claim {
  id: string;
  statement: string;
  status: ClaimStatus;
  detail: string;
}

export interface Certificate {
  version: 1 | 2;
  claims: Claim[];
  // The certificate file exactly as `axiom analyse` writes it.
  json: string;
}

export interface Diagnostic {
  line: number;
  column: number;
  code: string;
  message: string;
}

// A chain yields chain, certificate and, for a settling two-state chain,
// convergence; a retry yields retry. Absent parts are null.
export interface Analysis {
  name: string;
  chain: Chain | null;
  convergence: Convergence | null;
  retry: Retry | null;
  certificate: Certificate | null;
  // Set when the model is valid but the browser does not analyse it.
  notice: string | null;
}

export type AnalyzeResult =
  | { ok: true; analysis: Analysis }
  | { ok: false; diagnostics: Diagnostic[] };

export interface Engine {
  analyze(source: string): Promise<AnalyzeResult>;
  simulate(source: string, request: SimulateRequest): Promise<SimulateResult>;
}

// The caps crates/axiom-wasm enforces on a simulation request. The page
// bounds its inputs by them; the engine refuses anything past them.
export const SIM_CAPS = {
  samples: 200_000,
  steps: 1000,
  attempts: 1000,
} as const;

// A simulation request. The seed is a decimal string because a JavaScript
// number cannot hold every 64-bit seed.
export interface SimulateRequest {
  seed: string;
  samples: number;
  steps: number;
  maxAttempts: number;
}

// A Hoeffding estimate: with probability at least 1 - delta over the
// sampling, the true value lies within epsilon of value. Never a proof.
export interface Estimate {
  value: Fraction;
  approx: number;
  samples: number;
  epsilon: Fraction;
  delta: Fraction;
  certainty: "estimated";
}

export interface RunningPoint {
  samples: number;
  approx: number;
}

export interface ChainRun {
  states: string[];
  // The state whose fraction is tracked, the second one.
  state: string;
  steps: number;
  // Up to five paths of state indices, at most 201 entries each.
  paths: number[][];
  // The fraction of the first `samples` runs that end in `state`.
  running: RunningPoint[];
  fraction: Fraction;
  // Null when the chain has no steady state it settles to.
  steadyState: Fraction | null;
  steadyApprox: number | null;
}

export interface Bucket {
  attempts: number;
  count: number;
}

export interface RetryRun {
  maxAttempts: number;
  success: Fraction;
  histogram: Bucket[];
  // Runs that succeeded after the last bucket.
  beyond: number;
  // Runs that had not succeeded within maxAttempts.
  failures: number;
  successFraction: Fraction;
  meanAttempts: Fraction | null;
  meanApprox: number | null;
  expectedAttempts: Fraction | null;
  expectedApprox: number | null;
}

export interface Simulation {
  name: string;
  seed: string;
  samples: number;
  // The samples the Hoeffding bound needs for epsilon and delta.
  requiredSamples: number;
  epsilon: Fraction;
  delta: Fraction;
  // Null when samples < requiredSamples.
  estimate: Estimate | null;
  chain: ChainRun | null;
  retry: RetryRun | null;
}

export interface SimulationError {
  code: string;
  message: string;
  line: number | null;
  column: number | null;
}

export type SimulateResult =
  | { ok: true; simulation: Simulation }
  | { ok: false; error: SimulationError };
