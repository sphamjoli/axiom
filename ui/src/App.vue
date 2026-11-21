<script setup lang="ts">
import { ref, shallowRef, watch, watchEffect } from "vue";
import CertificatePanel from "./components/CertificatePanel.vue";
import ConvergencePlot from "./components/ConvergencePlot.vue";
import Frac from "./components/Frac.vue";
import ModelEditor from "./components/ModelEditor.vue";
import TransitionGraph from "./components/TransitionGraph.vue";
import { LINKS_MODEL } from "./engine/models";
import type { Analysis, Diagnostic, Engine } from "./engine/types";
import { wasmEngine } from "./engine/wasm";

const engine: Engine = wasmEngine;

const source = ref(LINKS_MODEL);
const analysis = shallowRef<Analysis | null>(null);
const diagnostics = ref<Diagnostic[]>([]);

watch(
  source,
  async (text) => {
    const result = await engine.analyze(text);
    if (result.ok) {
      analysis.value = result.analysis;
      diagnostics.value = [];
    } else {
      diagnostics.value = result.diagnostics;
    }
  },
  { immediate: true },
);

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
    <main class="mx-auto grid max-w-6xl gap-4 px-4 py-5 lg:grid-cols-2">
      <ModelEditor v-model="source" :diagnostics="diagnostics" />
      <TransitionGraph v-if="analysis?.chain" :chain="analysis.chain" />
      <section v-if="analysis?.notice" class="panel">
        <h2 class="panel-title">Model {{ analysis.name }}</h2>
        <p class="text-sm text-ink-muted">{{ analysis.notice }}</p>
      </section>
      <ConvergencePlot v-if="analysis?.convergence" :data="analysis.convergence" class="lg:col-span-2" />
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
    </main>
  </div>
</template>
