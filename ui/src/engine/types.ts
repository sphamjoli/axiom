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
}
