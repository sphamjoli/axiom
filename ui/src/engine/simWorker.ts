// Runs simulations off the main thread, so a long run never blocks the page.
// It accepts one message shape from the page that created it and answers
// with the engine's JSON text, tagged with the request id.
import init, { simulate } from "./wasm/axiom_wasm.js";

export interface SimJob {
  id: number;
  source: string;
  request: string;
}

export interface SimReply {
  id: number;
  json: string;
}

let ready: Promise<unknown> | null = null;

function isJob(data: unknown): data is SimJob {
  if (typeof data !== "object" || data === null) return false;
  const job = data as Record<string, unknown>;
  return Number.isSafeInteger(job.id) && typeof job.source === "string" && typeof job.request === "string";
}

self.onmessage = async (event: MessageEvent<unknown>) => {
  if (!isJob(event.data)) return;
  const { id, source, request } = event.data;
  ready ??= init();
  await ready;
  const reply: SimReply = { id, json: simulate(source, request) };
  self.postMessage(reply);
};
