<script setup lang="ts">
import { computed } from "vue";
import Frac from "./Frac.vue";
import StatusBadge from "./StatusBadge.vue";
import type { Fraction, Simulation } from "../engine/types";
import { complement, count, within } from "../format";

// The result of a run in three plain statements: the estimate, the exact
// answer from the maths, and whether the estimate fell inside its band.
const props = defineProps<{ result: Simulation }>();

const subject = computed(() =>
  props.result.chain ? `the chance of ending ${props.result.chain.state} after ${props.result.chain.steps} steps` : "the chance of success per attempt",
);

const exact = computed<{ value: Fraction; name: string } | null>(() => {
  const { chain, retry } = props.result;
  if (chain?.steadyState) return { value: chain.steadyState, name: "the long-run chance from the steady state" };
  if (retry) return { value: retry.success, name: "the success chance written in the model" };
  return null;
});

const inside = computed(() => {
  const estimate = props.result.estimate;
  return estimate && exact.value ? within(estimate.value, exact.value.value, estimate.epsilon) : null;
});
</script>

<template>
  <div class="border-l-[3px] border-dashed border-estimated pl-3 text-sm">
    <ol class="grid gap-1.5">
      <li class="flex flex-wrap items-center gap-2">
        <StatusBadge status="estimated" />
        <span v-if="result.estimate">
          Estimate of {{ subject }}: <Frac :value="result.estimate.value" /> &#177; <Frac :value="result.estimate.epsilon" />, with probability at least
          <Frac :value="complement(result.estimate.delta)" />.
        </span>
        <span v-else>No estimate: {{ count(result.samples) }} runs are fewer than the {{ count(result.requiredSamples) }} it needs.</span>
      </li>
      <li v-if="exact">Exact answer, {{ exact.name }}: <Frac :value="exact.value" />.</li>
      <li v-else>No exact answer to compare with: this chain does not settle.</li>
      <li v-if="inside !== null" :class="inside ? 'text-proven' : 'text-rejected'">
        {{ inside ? "The estimate fell inside its band" : "The estimate fell outside its band, which happens at most" }}
        <template v-if="inside"> of &#177; <Frac :value="result.epsilon" /> around the exact answer.</template>
        <template v-else> <Frac :value="result.delta" /> of the time.</template>
      </li>
    </ol>
    <p class="mt-2 text-xs text-ink-muted">
      An estimate comes from sampling and is not a proof. It is given only when the run has at least the {{ count(result.requiredSamples) }} runs that the Hoeffding bound needs for &#177;
      <Frac :value="result.epsilon" /> at failure probability <Frac :value="result.delta" />.
    </p>
  </div>
</template>
