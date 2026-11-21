<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, shallowRef, watch, watchEffect } from "vue";
import CertificatePanel from "./components/CertificatePanel.vue";
import ConvergencePlot from "./components/ConvergencePlot.vue";
import Frac from "./components/Frac.vue";
import ModelControls from "./components/ModelControls.vue";
import ModelEditor from "./components/ModelEditor.vue";
import ParametricPanel from "./components/ParametricPanel.vue";
import SimulatePanel from "./components/SimulatePanel.vue";
import TransitionGraph from "./components/TransitionGraph.vue";
import type { AnalyzeAtResult, Analysis, Diagnostic, Engine, Fraction, Point } from "./engine/types";
import { complement, fractionText } from "./format";
import ModelGuide from "./components/ModelGuide.vue";
import { TEMPLATES } from "./models/templates";
import { modelClock as clock } from "./playback";
import { decodeRun } from "./shareLink";
import { wasmEngine } from "./engine/wasm";

const engine: Engine = wasmEngine;

// A shared run in the URL fragment brings its model; the Simulate panel
// replays it. The model text goes only to the engine and the editor.
const sharedRun = decodeRun(window.location.hash);
const source = ref(sharedRun?.model ?? TEMPLATES[0]?.source ?? "");
const analysis = shallowRef<Analysis | null>(null);
// The point of a parametric chain's box the page shows, or null to let the
// engine pick the midpoint. A shared link may bring one; the engine checks
// it lies in the box before using it.
const chosen = shallowRef<Point | null>(sharedRun?.point ?? null);
const diagnostics = ref<Diagnostic[]>([]);
// True while the editor holds a model with errors: the results on the page
// are then from the last valid model, so they are dimmed and cannot be used.
const stale = ref(false);

// Analysis runs once on load, then 150 ms after the last keystroke, so typing
// does not queue an analysis per character. Only the newest run may update
// the page, so a slow earlier run cannot overwrite a later result.
const ANALYSIS_DELAY_MS = 150;
let pending: ReturnType<typeof setTimeout> | undefined;
let latestRun = 0;

async function runAnalysis(text: string): Promise<void> {
  latestRun += 1;
  const run = latestRun;
  const at = chosen.value;
  let result: AnalyzeAtResult = at ? await engine.analyzeAt(text, at) : await engine.analyze(text);
  // A point that does not fit the model, or a model without a box, falls
  // back to the plain analysis, which shows the midpoint.
  if (!result.ok && "error" in result) result = await engine.analyze(text);
  if (run !== latestRun) return;
  if (result.ok) {
    analysis.value = result.analysis;
    const point = result.analysis.parametric?.point;
    chosen.value = point ? { a: point.a, b: point.b } : null;
    // Every new model plays its convergence once from t = 0, then waits.
    const series = result.analysis.convergence?.series;
    if (series) clock.restart(series.length - 1);
    diagnostics.value = [];
    stale.value = false;
  } else if ("diagnostics" in result) {
    diagnostics.value = result.diagnostics;
    stale.value = analysis.value !== null;
  }
}

watch(source, (text) => {
  clearTimeout(pending);
  pending = setTimeout(() => void runAnalysis(text), ANALYSIS_DELAY_MS);
});
onBeforeUnmount(() => clearTimeout(pending));

// Picking a point re-analyses that instance at once; the text is unchanged.
function pick(point: Point): void {
  chosen.value = point;
  clearTimeout(pending);
  void runAnalysis(source.value);
}

const instanceLabel = computed(() => {
  const parametric = analysis.value?.parametric;
  if (!parametric) return null;
  const { a, b, point } = parametric;
  return `Showing ${a.name} = ${fractionText(point.a)}, ${b.name} = ${fractionText(point.b)}`;
});
void runAnalysis(source.value);

// The probability of each state at the clock's step: the series tracks the
// second state, so the first holds the rest.
const mass = computed<Fraction[] | null>(() => {
  const convergence = analysis.value?.convergence;
  if (!convergence) return null;
  const point = convergence.series.find((p) => p.t === clock.t.value) ?? convergence.series[0];
  return point ? [complement(point.value), point.value] : null;
});

const PRESETS = TEMPLATES.filter((t) => t.preset !== null);
const guideOpen = ref(false);

function load(text: string): void {
  source.value = text;
}

// Brings an element into view: a short glide that shows where the page
// went, or an instant jump when reduced motion is asked for.
function reveal(id: string): void {
  const element = document.getElementById(id);
  if (!element) return;
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  element.scrollIntoView({ behavior: reduced ? "auto" : "smooth", block: "start" });
}

// Loading from the guide also brings the editor into view.
function loadFromGuide(text: string): void {
  load(text);
  reveal("model-source");
  document.getElementById("model-source")?.focus({ preventScroll: true });
}

// Opening the guide brings it into view.
watch(guideOpen, async (open) => {
  if (!open) return;
  await nextTick();
  reveal("guide");
});

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
        <button v-for="p in PRESETS" :key="p.id" class="btn" type="button" :aria-pressed="source === p.source" @click="load(p.source)">{{ p.preset }}</button>
        <button class="btn btn-primary" type="button" :aria-expanded="guideOpen" aria-controls="guide" @click="guideOpen = !guideOpen">
          {{ guideOpen ? "Hide how to write a model" : "How to write a model" }}
        </button>
      </div>
    </div>
    <main class="mx-auto grid grid-cols-[minmax(0,1fr)] max-w-6xl gap-4 px-4 py-5 lg:grid-cols-2">
      <ModelGuide v-if="guideOpen" :engine="engine" @load="loadFromGuide" @close="guideOpen = false" />
      <ModelEditor v-model="source" :diagnostics="diagnostics" />
      <p v-if="stale" class="panel text-sm text-rejected lg:col-span-2" role="status">
        The model has an error, so the results below are from the last valid model and cannot be used. Fix the error to update them.
      </p>
      <ParametricPanel v-if="analysis?.parametric" :class="{ 'is-stale': stale }" :inert="stale" :parametric="analysis.parametric" @show="reveal('instance')" />
      <ModelControls :class="{ 'is-stale': stale }" :inert="stale" :analysis="analysis" @source="load" @point="pick" />
      <p v-if="instanceLabel" id="instance" class="panel py-3 font-mono text-sm text-ink lg:col-span-2" :class="{ 'is-stale': stale }" role="status">
        {{ instanceLabel }}<span class="font-sans text-ink-muted">: the graph, the convergence plot and the simulation below are this instance.</span>
      </p>
      <TransitionGraph :class="{ 'is-stale': stale }" :inert="stale" v-if="analysis?.chain" :chain="analysis.chain" :mass="mass" :step="clock.t.value" />
      <section v-if="analysis?.notice" class="panel" :class="{ 'is-stale': stale }" :inert="stale">
        <h2 class="panel-title">Model {{ analysis.name }}</h2>
        <p class="text-sm text-ink-muted">{{ analysis.notice }}</p>
      </section>
      <ConvergencePlot :class="{ 'is-stale': stale }" :inert="stale" v-if="analysis?.convergence" id="convergence" :data="analysis.convergence" />
      <CertificatePanel :class="{ 'is-stale': stale }" :inert="stale" v-if="analysis?.certificate" :certificate="analysis.certificate" />
      <section v-if="analysis?.retry" class="panel" :class="{ 'is-stale': stale }" :inert="stale">
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
        :class="{ 'is-stale': stale }"
        :inert="stale"
        :engine="engine"
        :source="source"
        :analysis="analysis"
        :initial-run="sharedRun"
        class="lg:col-span-2"
      />
    </main>
  </div>
</template>
