import init, { analyse } from "./wasm/axiom_wasm.js";
import type { SimJob, SimReply } from "./simWorker";
import type { AnalyzeResult, Engine, SimulateResult } from "./types";

// Loads the WebAssembly module once, on first use.
let ready: Promise<unknown> | null = null;

// Simulations run in one dedicated worker, created on first use. Each job
// carries an id, and a reply settles only the job with that id.
let worker: Worker | null = null;
let nextId = 0;
const waiting = new Map<number, (result: SimulateResult) => void>();

const WORKER_FAILED: SimulateResult = {
  ok: false,
  error: { code: "worker-failed", message: "The simulation worker stopped. Reload the page to try again.", line: null, column: null },
};

function simWorker(): Worker {
  if (worker) return worker;
  const created = new Worker(new URL("./simWorker.ts", import.meta.url), { type: "module" });
  created.onmessage = (event: MessageEvent<SimReply>) => {
    const settle = waiting.get(event.data.id);
    if (!settle) return;
    waiting.delete(event.data.id);
    settle(JSON.parse(event.data.json) as SimulateResult);
  };
  created.onerror = () => {
    for (const settle of waiting.values()) settle(WORKER_FAILED);
    waiting.clear();
    created.terminate();
    worker = null;
  };
  worker = created;
  return created;
}

export const wasmEngine: Engine = {
  async analyze(source) {
    ready ??= init();
    await ready;
    return JSON.parse(analyse(source)) as AnalyzeResult;
  },
  simulate(source, request) {
    nextId += 1;
    const job: SimJob = { id: nextId, source, request: JSON.stringify(request) };
    return new Promise((resolve) => {
      waiting.set(job.id, resolve);
      simWorker().postMessage(job);
    });
  },
};
