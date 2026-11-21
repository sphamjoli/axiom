<script setup lang="ts">
import { onMounted, ref, shallowRef } from "vue";
import type { AnalyzeResult, Engine } from "../engine/types";
import { fractionText } from "../format";
import { TEMPLATES, type Template } from "../models/templates";

const props = defineProps<{ engine: Engine }>();
const emit = defineEmits<{ load: [source: string]; close: [] }>();

// What the engine derives from each template, in plain words. It is computed
// by the real engine when the guide opens, so it cannot drift from it.
const derived = shallowRef<Record<string, string>>({});
const copied = ref<string | null>(null);

function describe(result: AnalyzeResult): string {
  if (!result.ok) {
    const d = result.diagnostics[0];
    return d ? `Rejected: ${d.code} at line ${d.line}, column ${d.column}: ${d.message}.` : "Rejected.";
  }
  const a = result.analysis;
  if (a.notice) return a.notice;
  if (a.convergence) {
    const c = a.convergence;
    const horizon = c.horizon === null ? "It is not within 1/1000 in the steps shown." : `It is within 1/1000 from step ${c.horizon}.`;
    return `Settles at P(${c.state}) = ${fractionText(c.fixedPoint)}. Each step shrinks the distance by λ = ${fractionText(c.a)}. ${horizon}`;
  }
  if (a.retry) {
    const r = a.retry;
    const attempts = r.expectedAttempts ? `${fractionText(r.expectedAttempts)} attempts are expected` : "an attempt never succeeds";
    return `It succeeds eventually with probability ${fractionText(r.successProbability)}, and ${attempts}.`;
  }
  if (a.chain) {
    return a.chain.states.length === 2
      ? "It never settles: the chance of each state keeps changing."
      : `${a.chain.states.length} states: every row adds up to 1, and no steady state is analysed.`;
  }
  return "";
}

onMounted(async () => {
  const entries = await Promise.all(TEMPLATES.map(async (t) => [t.id, describe(await props.engine.analyze(t.source))] as const));
  derived.value = Object.fromEntries(entries);
});

async function copy(t: Template): Promise<void> {
  try {
    await navigator.clipboard.writeText(t.source);
    copied.value = t.id;
  } catch {
    copied.value = null;
  }
}
</script>

<template>
  <section id="guide" class="panel min-w-0 lg:col-span-2" aria-labelledby="guide-title">
    <div class="mb-3 flex flex-wrap items-baseline justify-between gap-2">
      <h2 id="guide-title" class="panel-title mb-0">How to write a model</h2>
      <button class="btn" type="button" @click="emit('close')">Close the guide</button>
    </div>

    <div class="grid grid-cols-[minmax(0,1fr)] gap-x-8 gap-y-3 text-sm lg:grid-cols-2">
      <div>
        <h3 class="font-medium text-ink">States</h3>
        <p class="text-ink-muted">A link is either up or down.</p>
        <pre class="code">states up, down</pre>
      </div>
      <div>
        <h3 class="font-medium text-ink">A row of chances</h3>
        <p class="text-ink-muted">
          A link that is up stays up 9 times in 10 and fails 1 time in 10. Each row must add up to exactly 1. Chances are exact fractions, never decimals. A state left out of a row has chance 0.
        </p>
        <pre class="code">from up: up 9/10, down 1/10</pre>
      </div>
      <div>
        <h3 class="font-medium text-ink">A retry</h3>
        <p class="text-ink-muted">A request succeeds 3 times in 10; otherwise it is tried again.</p>
        <pre class="code">retry request { success 3/10 }</pre>
      </div>
      <div>
        <h3 class="font-medium text-ink">Parameters</h3>
        <p class="text-ink-muted">
          a is any failure chance strictly between 0 and 1. A bracket facing the number includes it, one facing away excludes it: <code class="font-mono">[0, 1]</code> includes both ends,
          <code class="font-mono">]0, 1[</code> neither, and <code class="font-mono">[0, 1[</code> or <code class="font-mono">]0, 1]</code> one. Weights may add multiples of parameters, such as
          <code class="font-mono">1 - a</code> or <code class="font-mono">3/10 + 1/2*b</code>.
        </p>
        <pre class="code">param a in ]0, 1[</pre>
      </div>
      <div>
        <h3 class="font-medium text-ink">Comments</h3>
        <p class="text-ink-muted">Everything after <code class="font-mono">#</code> on a line is ignored.</p>
        <pre class="code"># Alice's links</pre>
      </div>
      <div>
        <h3 class="font-medium text-ink">Limits and what is rejected</h3>
        <p class="text-ink-muted">
          At most 16 states, 8 parameters and 64 digits in a number. Decimals such as <code class="font-mono">0.1</code>, products of parameters such as
          <code class="font-mono">a*b</code>, and loops are rejected with an error that gives its line and column.
        </p>
      </div>
    </div>

    <h3 class="mt-6 mb-2 font-display text-base font-semibold text-ink">Templates</h3>
    <ul class="grid grid-cols-[minmax(0,1fr)] gap-4 lg:grid-cols-2">
      <li v-for="t in TEMPLATES" :key="t.id" class="min-w-0 rounded-md border border-rule p-3">
        <h4 class="font-medium text-ink">{{ t.title }}</h4>
        <p class="text-sm text-ink-muted">{{ t.description }}</p>
        <dl class="mt-2 grid gap-1 text-sm">
          <template v-for="b in t.breakdown" :key="b.line">
            <dt class="text-ink">{{ b.plain }}</dt>
            <dd class="font-mono text-xs text-ink-muted">{{ b.line }}</dd>
          </template>
        </dl>
        <pre class="code mt-2">{{ t.source }}</pre>
        <p class="mt-2 text-sm text-ink">{{ derived[t.id] ?? "Asking the engine" }}</p>
        <p v-if="t.note" class="mt-1 text-sm text-ink-muted">{{ t.note }}</p>
        <div class="mt-2 flex flex-wrap items-center gap-2">
          <button class="btn" type="button" @click="copy(t)">Copy</button>
          <button class="btn" type="button" @click="emit('load', t.source)">Load into editor</button>
          <span class="fade text-xs text-proven" :style="{ opacity: copied === t.id ? 1 : 0 }" aria-live="polite">{{ copied === t.id ? "Copied" : "" }}</span>
        </div>
      </li>
    </ul>
  </section>
</template>
