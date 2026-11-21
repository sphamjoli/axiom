import type { Fraction } from "./engine/types";
import { complement, fractionText } from "./format";

// The model text the probability controls write. The engine stays the
// authority: text written here is parsed and checked by crates/axiom-wasm
// like any typed model.

function row(states: [string, string], weights: [Fraction, Fraction]): string {
  return states
    .map((s, i) => [s, weights[i]] as const)
    .filter((entry): entry is readonly [string, Fraction] => entry[1] !== undefined && entry[1].num !== "0")
    .map(([s, p]) => `${s} ${fractionText(p)}`)
    .join(", ");
}

// A two-state chain that leaves the first state with probability a and the
// second with probability b. Zero entries are left out, as the model allows.
export function twoStateText(name: string, states: [string, string], a: Fraction, b: Fraction): string {
  const [first, second] = states;
  return `chain ${name} {
  states ${first}, ${second}
  from ${first}: ${row(states, [complement(a), a])}
  from ${second}: ${row(states, [b, complement(b)])}
}
`;
}

export function retryText(name: string, success: Fraction): string {
  return `retry ${name} {
  success ${fractionText(success)}
}
`;
}
