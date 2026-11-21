import { SIM_CAPS } from "./engine/types";

// A run as a URL fragment, so a link replays the identical run and nothing
// is sent to a server. The fragment is untrusted input: it is size-limited,
// decoded defensively and parsed into a named type here; the model text in
// it is only ever handed to the engine and shown as plain text.

export interface SharedRun {
  model: string;
  seed: string;
  samples: number;
  steps: number;
  maxAttempts: number;
}

const PREFIX = "#run=";
// The engine refuses models over 64 KiB; base64 grows text by a third.
const MAX_MODEL_BYTES = 65_536;
const MAX_FRAGMENT_CHARS = 90_000;

function toBase64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(text: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]*$/.test(text)) return null;
  try {
    const binary = atob(text.replace(/-/g, "+").replace(/_/g, "/"));
    return Uint8Array.from(binary, (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
}

// Whether `value` is a whole number from 1 to `max`.
export const inRange = (value: unknown, max: number): value is number => Number.isInteger(value) && (value as number) >= 1 && (value as number) <= max;

// Whether `text` is a decimal seed that fits in 64 bits.
export const isSeed = (text: string): boolean => /^\d{1,20}$/.test(text) && BigInt(text) <= 18446744073709551615n;

function parse(data: unknown): SharedRun | null {
  if (typeof data !== "object" || data === null) return null;
  const run = data as Record<string, unknown>;
  const { model, seed, samples, steps, maxAttempts } = run;
  if (typeof model !== "string" || new TextEncoder().encode(model).length > MAX_MODEL_BYTES) return null;
  if (typeof seed !== "string" || !isSeed(seed)) return null;
  if (!inRange(samples, SIM_CAPS.samples) || !inRange(steps, SIM_CAPS.steps) || !inRange(maxAttempts, SIM_CAPS.attempts)) return null;
  return { model, seed, samples, steps, maxAttempts };
}

export function encodeRun(run: SharedRun): string | null {
  const fragment = PREFIX + toBase64Url(new TextEncoder().encode(JSON.stringify(run)));
  return fragment.length <= MAX_FRAGMENT_CHARS ? fragment : null;
}

// The run in `hash`, or null when it is absent, too long or malformed.
export function decodeRun(hash: string): SharedRun | null {
  if (!hash.startsWith(PREFIX) || hash.length > MAX_FRAGMENT_CHARS) return null;
  const bytes = fromBase64Url(hash.slice(PREFIX.length));
  if (!bytes) return null;
  try {
    return parse(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)) as unknown);
  } catch {
    return null;
  }
}
