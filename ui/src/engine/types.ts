// Shapes returned by the analysis engine. They mirror what the future WASM
// crate will serialise, so every exact quantity is a fraction of decimal
// integer strings (arbitrary precision, never a float).

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
  horizon: number;
  series: SeriesPoint[];
}

export interface Retry {
  success: Fraction;
  successProbability: Fraction;
  expectedAttempts: Fraction;
}

export type ClaimStatus = "proven" | "estimated" | "rejected";

export interface Claim {
  id: string;
  statement: string;
  status: ClaimStatus;
  detail: string;
}

export interface Certificate {
  version: 1;
  claims: Claim[];
}

export interface Diagnostic {
  line: number;
  message: string;
}

export interface Analysis {
  chain: Chain;
  convergence: Convergence;
  retry: Retry;
  certificate: Certificate;
}

export type AnalyzeResult =
  | { ok: true; analysis: Analysis }
  | { ok: false; diagnostics: Diagnostic[] };

export interface Engine {
  analyze(source: string): Promise<AnalyzeResult>;
}
