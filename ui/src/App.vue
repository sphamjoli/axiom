<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef, watch, watchEffect } from "vue";
import CertificatePanel from "./components/CertificatePanel.vue";
import ConvergencePlot from "./components/ConvergencePlot.vue";
import Frac from "./components/Frac.vue";
import ModelControls from "./components/ModelControls.vue";
import ModelEditor from "./components/ModelEditor.vue";
import SimulatePanel from "./components/SimulatePanel.vue";
import TransitionGraph from "./components/TransitionGraph.vue";
import { LINKS_MODEL } from "./engine/models";
import type { Analysis, Diagnostic, Engine, Fraction } from "./engine/types";
import { complement } from "./format";
import { PRESETS } from "./modelText";
import { modelClock as clock } from "./playback";
import { wasmEngine } from "./engine/wasm";

const engine: Engine = wasmEngine;

const source = ref(LINKS_MODEL);
const analysis = shallowRef<Analysis | null>(null);
const diagnostics = ref<Diagnostic[]>([]);

// Analysis runs once on load, then 150 ms after the last keystroke, so typing
// does not queue an analysis per character. Only the newest run may update
// the page, so a slow earlier run cannot overwrite a later result.
const ANALYSIS_DELAY_MS = 150;
let pending: ReturnType<typeof setTimeout> | undefined;
let latestRun = 0;

async function runAnalysis(text: string): Promise<void> {
  latestRun += 1;
  const run = latestRun;
  const result = await engine.analyze(text);
  if (run !== latestRun) return;
  if (result.ok) {
    analysis.value = result.analysis;
    // Every new model plays its convergence once from t = 0, then waits.
    const series = result.analysis.convergence?.series;
    if (series) clock.restart(series.length - 1);
    diagnostics.value = [];
  } else {
    diagnostics.value = result.diagnostics;
  }
}

watch(source, (text) => {
  clearTimeout(pending);
  pending = setTimeout(() => void runAnalysis(text), ANALYSIS_DELAY_MS);
});
onBeforeUnmount(() => clearTimeout(pending));
void runAnalysis(source.value);

// The probability of each state at the clock's step: the series tracks the
// second state, so the first holds the rest.
const mass = computed<Fraction[] | null>(() => {
  const convergence = analysis.value?.convergence;
  if (!convergence) return null;
  const point = convergence.series.find((p) => p.t === clock.t.value) ?? convergence.series[0];
  return point ? [complement(point.value), point.value] : null;
});

function load(text: string): void {
  source.value = text;
}

const dark = ref(window.matchMedia("(prefers-color-scheme: dark)").matches);
watchEffect(() => document.documentElement.classList.toggle("dark", dark.value));
</script>

<template>
  <div class="min-h-screen bg-paper font-sans text-ink">
    <header class="border-b border-rule bg-sheet">
      <div class="mx-auto flex max-w-6xl items-center justify-between gap-3 px-4 py-3">
        <h1 class="font-display text-xl font-semibold">axiom</h1>
        <button class="btn" type="button" :aria-pressed="dark" @click="dark = !dark">Use {{ dark ? "light" : "dark" }} theme</button>
      </div>
    </header>
    <div class="mx-auto max-w-6xl px-4 pt-5">
      <p class="text-base text-ink">Write a model, watch it run, and get a certificate the Lean checker can verify.</p>
      <ol class="mt-3 grid gap-2 text-sm sm:grid-cols-3">
        <li><a class="step" href="#model-source"><span class="step-n">1</span><span>Edit the model, or move the probabilities</span></a></li>
        <li><a class="step" href="#convergence"><span class="step-n">2</span><span>Run it: play the convergence and simulate</span></a></li>
        <li><a class="step" href="#certificate"><span class="step-n">3</span><span>Save the certificate and run <code class="font-mono">axiom check</code> on it</span></a></li>
      </ol>
      <div class="mt-4 flex flex-wrap items-center gap-2 text-sm" role="group" aria-label="Example models">
        <span class="text-ink-muted">Load an example:</span>
        <button v-for="p in PRESETS" :key="p.label" class="btn" type="button" :aria-pressed="source === p.source" @click="load(p.source)">{{ p.label }}</button>
      </div>
    </div>
    <main class="mx-auto grid max-w-6xl gap-4 px-4 py-5 lg:grid-cols-2">
      <ModelEditor v-model="source" :diagnostics="diagnostics" />
      <ModelControls :analysis="analysis" @source="load" />
      <TransitionGraph v-if="analysis?.chain" :chain="analysis.chain" :mass="mass" :step="clock.t.value" />
      <section v-if="analysis?.notice" class="panel">
        <h2 class="panel-title">Model {{ analysis.name }}</h2>
        <p class="text-sm text-ink-muted">{{ analysis.notice }}</p>
      </section>
      <ConvergencePlot v-if="analysis?.convergence" id="convergence" :data="analysis.convergence" />
      <CertificatePanel v-if="analysis?.certificate" :certificate="analysis.certificate" />
      <section v-if="analysis?.retry" class="panel">
        <h2 class="panel-title">Retry until success</h2>
        <dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
          <dt class="text-ink-muted">Success per attempt</dt>
          <dd><Frac :value="analysis.retry.success" /></dd>
          <dt class="text-ink-muted">Probability of eventual success</dt>
          <dd><Frac :value="analysis.retry.successProbability" /></dd>
          <dt class="text-ink-muted">Expected attempts</dt>
          <dd>
            <Frac v-if="analysis.retry.expectedAttempts" :value="analysis.retry.expectedAttempts" />
            <span v-else>None: an attempt never succeeds</span>
          </dd>
        </dl>
      </section>
      <SimulatePanel
        v-if="analysis?.chain || analysis?.retry"
        :engine="engine"
        :source="source"
        :kind="analysis.chain ? 'chain' : 'retry'"
        class="lg:col-span-2"
      />
    </main>
  </div>
</template>
