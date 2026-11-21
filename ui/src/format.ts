import type { Fraction } from "./engine/types";

// Groups the digits of a count in threes with a no-break space: 38 005.
export function count(value: number): string {
  return Math.trunc(value).toString().replace(/\B(?=(\d{3})+(?!\d))/g, " ");
}

// An exact fraction as text, "1/4", or "3" when the denominator is 1.
export function fractionText(value: Fraction): string {
  return value.den === "1" ? value.num : `${value.num}/${value.den}`;
}

// 1 - value, exactly, for a fraction num/den with 0 <= num <= den.
export function complement(value: Fraction): Fraction {
  return { num: (BigInt(value.den) - BigInt(value.num)).toString(), den: value.den };
}

// A fraction's value as a float, for plotting only.
export function approx(value: Fraction): number {
  return Number(value.num) / Number(value.den);
}
