<script setup lang="ts">
import StatusBadge from "./StatusBadge.vue";
import type { Certificate, ClaimStatus } from "../engine/types";

defineProps<{ certificate: Certificate }>();

// The left gutter repeats the verdict as a rule: solid for a kernel-checked
// proof, dashed for an estimate, so an estimate never reads as a proof.
const gutter: Record<ClaimStatus, string> = {
  proven: "border-l-[3px] border-solid border-proven",
  estimated: "border-l-[3px] border-dashed border-estimated",
  rejected: "border-l-[3px] border-dotted border-rejected",
};
</script>

<template>
  <section class="panel">
    <h2 class="panel-title">Certificate (version {{ certificate.version }})</h2>
    <ul class="space-y-2 font-mono text-sm">
      <li v-for="c in certificate.claims" :key="c.id" :class="['py-1.5 pl-3', gutter[c.status]]">
        <div class="flex flex-wrap items-start justify-between gap-2">
          <p :class="c.status === 'rejected' ? 'text-ink-muted line-through' : 'text-ink'">
            <span class="text-ink-muted select-none">&#8866; </span>{{ c.statement }}
          </p>
          <StatusBadge :status="c.status" />
        </div>
        <p class="mt-1 font-sans text-xs text-ink-muted">{{ c.detail }}</p>
      </li>
    </ul>
  </section>
</template>
