<script setup lang="ts">
import type { Diagnostic } from "../engine/types";

const model = defineModel<string>({ required: true });
defineProps<{ diagnostics: Diagnostic[] }>();
</script>

<template>
  <section class="panel flex flex-col">
    <label for="model-source" class="panel-title">Model source</label>
    <textarea
      id="model-source"
      v-model="model"
      spellcheck="false"
      rows="8"
      class="w-full flex-1 resize-y rounded border border-rule bg-paper p-3 font-mono text-sm leading-6 text-ink"
    />
    <ul v-if="diagnostics.length > 0" class="mt-3 space-y-1 text-sm text-rejected" role="alert">
      <li v-for="(d, i) in diagnostics" :key="i">Line {{ d.line }}: {{ d.message }}</li>
    </ul>
    <p v-else class="mt-3 text-sm text-ink-muted">Edit the chain and the results below recompute.</p>
  </section>
</template>
