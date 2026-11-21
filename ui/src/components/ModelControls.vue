<script setup lang="ts">
import { computed } from "vue";
import Frac from "./Frac.vue";
import ProbabilityControl from "./ProbabilityControl.vue";
import type { Analysis, Fraction, Point } from "../engine/types";
import { retryText, twoStateText } from "../modelText";

const props = defineProps<{ analysis: Analysis | null }>();
const emit = defineEmits<{ source: [text: string]; point: [point: Point] }>();

// The controls are a second view of the model: they read the engine's
// analysis and write model text back, which the engine analyses again.
// A parametric chain keeps its text: the controls pick a point of its box,
// and the engine analyses that instance.
const twoState = computed(() => {
  if (props.analysis?.parametric) return null;
  const chain = props.analysis?.chain;
  if (!chain || chain.states.length !== 2) return null;
  const [first, second] = chain.states as [string, string];
  const p = (from: string, to: string): Fraction | undefined => chain.transitions.find((t) => t.from === from && t.to === to)?.p;
  const zero: Fraction = { num: "0", den: "1" };
  return { name: chain.name, states: [first, second] as [string, string], a: p(first, second) ?? zero, b: p(second, first) ?? zero };
});

function setChain(a: Fraction, b: Fraction): void {
  const chain = twoState.value;
  if (chain) emit("source", twoStateText(chain.name, chain.states, a, b));
}

function setRetry(success: Fraction): void {
  if (props.analysis) emit("source", retryText(props.analysis.name, success));
}
</script>

<template>
  <section id="controls" class="panel">
    <template v-if="analysis?.parametric">
      <h2 class="panel-title">Pick a point in the range</h2>
      <p class="mb-3 text-sm text-ink-muted">The model text keeps {{ analysis.parametric.a.name }} and {{ analysis.parametric.b.name }} symbolic. The values here only choose which instance the graph, plot and simulation show.</p>
      <div class="grid gap-4">
        <ProbabilityControl
          id="point-a"
          :label="`Failure probability ${analysis.parametric.a.name} (${analysis.parametric.states[0]} → ${analysis.parametric.states[1]})`"
          :value="analysis.parametric.point.a"
          :range="analysis.parametric.a.range"
          @change="(a) => analysis?.parametric && emit('point', { a, b: analysis.parametric.point.b })"
        />
        <ProbabilityControl
          id="point-b"
          :label="`Repair probability ${analysis.parametric.b.name} (${analysis.parametric.states[1]} → ${analysis.parametric.states[0]})`"
          :value="analysis.parametric.point.b"
          :range="analysis.parametric.b.range"
          @change="(b) => analysis?.parametric && emit('point', { a: analysis.parametric.point.a, b })"
        />
        <p v-if="analysis.convergence" class="font-mono text-sm text-ink" aria-live="polite">
          At this point: steady state <Frac :value="analysis.convergence.fixedPoint" /> · λ <Frac :value="analysis.convergence.a" /> ·
          <template v-if="analysis.convergence.horizon !== null">within 1/1000 from step {{ analysis.convergence.horizon }}</template>
          <template v-else>not within 1/1000 in the steps shown</template>
        </p>
      </div>
    </template>
    <template v-else>
      <h2 class="panel-title">Probabilities</h2>
      <div v-if="twoState" class="grid gap-4">
        <ProbabilityControl
          id="leave-first"
          :label="`Failure probability (${twoState.states[0]} → ${twoState.states[1]})`"
          :value="twoState.a"
          @change="(a) => twoState && setChain(a, twoState.b)"
        />
        <ProbabilityControl
          id="leave-second"
          :label="`Repair probability (${twoState.states[1]} → ${twoState.states[0]})`"
          :value="twoState.b"
          @change="(b) => twoState && setChain(twoState.a, b)"
        />
        <p class="font-mono text-sm text-ink" aria-live="polite">
          <template v-if="analysis?.convergence">
            Steady state <Frac :value="analysis.convergence.fixedPoint" /> · λ <Frac :value="analysis.convergence.a" /> ·
            <template v-if="analysis.convergence.horizon !== null">within 1/1000 from step {{ analysis.convergence.horizon }}</template>
            <template v-else>not within 1/1000 in the steps shown</template>
          </template>
          <template v-else>Never settles: the chain has no steady state it converges to.</template>
        </p>
      </div>
      <div v-else-if="analysis?.retry" class="grid gap-4">
        <ProbabilityControl id="success" label="Success probability per attempt" :value="analysis.retry.success" @change="setRetry" />
        <p class="font-mono text-sm text-ink" aria-live="polite">
          Expected attempts
          <Frac v-if="analysis.retry.expectedAttempts" :value="analysis.retry.expectedAttempts" />
          <span v-else>none: an attempt never succeeds</span>
        </p>
      </div>
      <p v-else class="text-sm text-ink-muted">Sliders cover two-state chains and retries without parameters. This model has another shape, so edit the text to change it.</p>
    </template>
  </section>
</template>
