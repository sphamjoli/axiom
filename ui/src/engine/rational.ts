import type { Fraction } from "./types";

// Minimal exact rational arithmetic for the mock engine.
export interface Q {
  n: bigint;
  d: bigint;
}

const abs = (x: bigint): bigint => (x < 0n ? -x : x);

function gcd(a: bigint, b: bigint): bigint {
  let x = abs(a);
  let y = abs(b);
  while (y !== 0n) {
    [x, y] = [y, x % y];
  }
  return x;
}

export function q(n: bigint, d: bigint = 1n): Q {
  if (d === 0n) {
    throw new RangeError("zero denominator");
  }
  const sign = d < 0n ? -1n : 1n;
  const g = gcd(n, d) || 1n;
  return { n: (sign * n) / g, d: (sign * d) / g };
}

export const add = (x: Q, y: Q): Q => q(x.n * y.d + y.n * x.d, x.d * y.d);
export const sub = (x: Q, y: Q): Q => q(x.n * y.d - y.n * x.d, x.d * y.d);
export const mul = (x: Q, y: Q): Q => q(x.n * y.n, x.d * y.d);
export const div = (x: Q, y: Q): Q => q(x.n * y.d, x.d * y.n);
export const absQ = (x: Q): Q => q(abs(x.n), x.d);
export const le = (x: Q, y: Q): boolean => x.n * y.d <= y.n * x.d;
export const eq = (x: Q, y: Q): boolean => x.n === y.n && x.d === y.d;

export const toFraction = (x: Q): Fraction => ({ num: x.n.toString(), den: x.d.toString() });
export const toNumber = (x: Q): number => Number(x.n) / Number(x.d);

export function parseQ(text: string): Q | undefined {
  const m = /^(\d+)(?:\/(\d+))?$/.exec(text.trim());
  if (!m || m[1] === undefined) {
    return undefined;
  }
  const d = BigInt(m[2] ?? "1");
  return d === 0n ? undefined : q(BigInt(m[1]), d);
}
