import type { Fraction } from "./engine/types";

// Exact fractions for the model controls, and the model text they write.
// The engine stays the authority: text written here is parsed and checked
// by crates/axiom-wasm like any typed model.

function gcd(a: bigint, b: bigint): bigint {
  let x = a < 0n ? -a : a;
  let y = b < 0n ? -b : b;
  while (y !== 0n) [x, y] = [y, x % y];
  return x;
}

export function reduce(num: bigint, den: bigint): Fraction {
  const g = gcd(num, den) || 1n;
  return { num: (num / g).toString(), den: (den / g).toString() };
}

// Parses "n/d" or "n" with at most 9 digits each, as a probability in [0, 1].
export function parseProbability(text: string): Fraction | null {
  const match = /^\s*(\d{1,9})\s*(?:\/\s*(\d{1,9}))?\s*$/.exec(text);
  if (!match) return null;
  const num = BigInt(match[1] ?? "0");
  const den = BigInt(match[2] ?? "1");
  if (den === 0n || num > den) return null;
  return reduce(num, den);
}

// The nearest hundredth, for a slider position.
export function fromHundredths(position: number): Fraction {
  return reduce(BigInt(Math.round(position * 100)), 100n);
}

export function complementOf(p: Fraction): Fraction {
  return reduce(BigInt(p.den) - BigInt(p.num), BigInt(p.den));
}

export function text(p: Fraction): string {
  return p.den === "1" ? p.num : `${p.num}/${p.den}`;
}

function row(states: [string, string], weights: [Fraction, Fraction]): string {
  return states
    .map((s, i) => [s, weights[i]] as const)
    .filter(([, p]) => p !== undefined && p.num !== "0")
    .map(([s, p]) => `${s} ${p ? text(p) : "0"}`)
    .join(", ");
}

// A two-state chain that leaves the first state with probability a and the
// second with probability b. Zero entries are left out, as the model allows.
export function twoStateText(name: string, states: [string, string], a: Fraction, b: Fraction): string {
  const [first, second] = states;
  return `chain ${name} {
  states ${first}, ${second}
  from ${first}: ${row(states, [complementOf(a), a])}
  from ${second}: ${row(states, [b, complementOf(b)])}
}
`;
}

export function retryText(name: string, success: Fraction): string {
  return `retry ${name} {
  success ${text(success)}
}
`;
}
