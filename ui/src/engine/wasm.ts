import init, { analyse } from "./wasm/axiom_wasm.js";
import type { AnalyzeResult, Engine } from "./types";

// Loads the WebAssembly module once, on first use.
let ready: Promise<unknown> | null = null;

export const wasmEngine: Engine = {
  async analyze(source) {
    ready ??= init();
    await ready;
    return JSON.parse(analyse(source)) as AnalyzeResult;
  },
};
