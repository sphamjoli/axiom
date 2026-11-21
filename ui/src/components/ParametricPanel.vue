<script setup lang="ts">
import StatusBadge from "./StatusBadge.vue";
import type { Parametric } from "../engine/types";
import { rangeText } from "../format";

defineProps<{ parametric: Parametric }>();
const emit = defineEmits<{ show: [] }>();
</script>

<template>
  <section id="parametric" class="panel" aria-labelledby="parametric-title">
    <div class="mb-2 flex flex-wrap items-start justify-between gap-2">
      <h2 id="parametric-title" class="panel-title mb-0">Parametric chain</h2>
      <StatusBadge status="unchecked" />
    </div>
    <p class="text-base text-ink">
      For every failure probability {{ parametric.a.name }} in <span class="font-mono">{{ rangeText(parametric.a.range) }}</span> and repair probability
      {{ parametric.b.name }} in <span class="font-mono">{{ rangeText(parametric.b.range) }}</span>, the chain settles to
      <span class="font-mono">P({{ parametric.states[1] }}) = {{ parametric.steadyState }}</span>, and the gap shrinks by
      <span class="font-mono">|{{ parametric.factor }}| &lt; 1</span> each step.
    </p>
    <p class="mt-2 text-sm text-ink">
      This holds for every value in the range, not just the ones tried; the Lean checker proves it from the certificate.
    </p>
    <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
      <dt class="text-ink-muted">{{ parametric.a.name }} + {{ parametric.b.name }} ranges over</dt>
      <dd class="font-mono">{{ rangeText(parametric.sum) }}</dd>
      <dt class="text-ink-muted">λ = {{ parametric.factor }} ranges over</dt>
      <dd class="font-mono">{{ rangeText(parametric.lambda) }}</dd>
    </dl>
    <p class="mt-3 text-sm text-ink-muted">
      <StatusBadge status="unchecked" class="mr-1.5 align-middle" /> means the engine computed the claim; it becomes Proven only when the Lean checker accepts it.
    </p>
    <button class="btn mt-3" type="button" @click="emit('show')">Show the instance at the chosen point</button>
  </section>
</template>
