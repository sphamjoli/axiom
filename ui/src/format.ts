import type { Fraction, Range } from "./engine/types";

// Exact fractions in the page: parsing, arithmetic and text. Every exact
// value is a pair of decimal strings; floats come only from `approx`, for
// plotting.

// Groups the digits of a count in threes with a no-break space: 38 005.
export function count(value: number): string {
  return Math.trunc(value).toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");
}

// A count with its noun, singular or plural: "1 run", "38 005 runs".
export function counted(value: number, singular: string, plural = `${singular}s`): string {
  return `${count(value)} ${Math.trunc(value) === 1 ? singular : plural}`;
}

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

// An exact fraction as text, "1/4", or "3" when the denominator is 1.
export function fractionText(value: Fraction): string {
  return value.den === "1" ? value.num : `${value.num}/${value.den}`;
}

// 1 - value, exactly, in lowest terms.
export function complement(value: Fraction): Fraction {
  return reduce(BigInt(value.den) - BigInt(value.num), BigInt(value.den));
}

// Whether |a - b| <= epsilon, decided exactly over the integers.
export function within(a: Fraction, b: Fraction, epsilon: Fraction): boolean {
  const gap = BigInt(a.num) * BigInt(b.den) - BigInt(b.num) * BigInt(a.den);
  const size = gap < 0n ? -gap : gap;
  return size * BigInt(epsilon.den) <= BigInt(epsilon.num) * BigInt(a.den) * BigInt(b.den);
}

// The sign of a - b, decided exactly over the integers (denominators are
// positive).
export function compare(a: Fraction, b: Fraction): number {
  const gap = BigInt(a.num) * BigInt(b.den) - BigInt(b.num) * BigInt(a.den);
  return gap === 0n ? 0 : gap < 0n ? -1 : 1;
}

// Whether `value` lies in `range`, open ends excluded.
export function inInterval(value: Fraction, range: Range): boolean {
  const lo = compare(value, range.lo);
  const hi = compare(value, range.hi);
  return (range.loClosed ? lo >= 0 : lo > 0) && (range.hiClosed ? hi <= 0 : hi < 0);
}

// A signed fraction with a true minus sign, for labels: "−1", "1/4".
export function signedText(value: Fraction): string {
  const text = fractionText(value);
  return text.startsWith("-") ? `−${text.slice(1)}` : text;
}

// An interval as written in a model: "]0, 1[" excludes both ends.
export function rangeText(range: Range): string {
  return `${range.loClosed ? "[" : "]"}${signedText(range.lo)}, ${signedText(range.hi)}${range.hiClosed ? "]" : "["}`;
}

// A fraction's value as a float, for plotting only.
export function approx(value: Fraction): number {
  return Number(value.num) / Number(value.den);
}
