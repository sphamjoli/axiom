<script setup lang="ts">
import { computed } from "vue";
import StatusBadge from "./StatusBadge.vue";
import type { Certificate, ClaimStatus } from "../engine/types";

const props = defineProps<{ certificate: Certificate }>();

const href = computed(() => `data:application/json;charset=utf-8,${encodeURIComponent(props.certificate.json)}`);

// The left gutter repeats the verdict as a rule: solid for a kernel-checked
// proof, dashed for an estimate, so an estimate never reads as a proof.
const gutter: Record<ClaimStatus, string> = {
  proven: "border-l-[3px] border-solid border-proven",
  estimated: "border-l-[3px] border-dashed border-estimated",
  rejected: "border-l-[3px] border-dotted border-rejected",
  unchecked: "border-l-[3px] border-double border-unchecked",
};
</script>

<template>
  <section class="panel">
    <h2 class="panel-title">Certificate (version {{ certificate.version }})</h2>
    <ul class="space-y-2 font-mono text-sm">
      <li v-for="c in certificate.claims" :key="c.id" :class="['py-1.5 pl-3', gutter[c.status]]">
        <div class="flex flex-wrap items-start justify-between gap-2">
          <p :class="c.status === 'rejected' ? 'text-ink-muted line-through' : 'text-ink'">
            <span v-if="c.status === 'proven'" class="text-ink-muted select-none">&#8866; </span>{{ c.statement }}
          </p>
          <StatusBadge :status="c.status" />
        </div>
        <p class="mt-1 font-sans text-xs text-ink-muted">{{ c.detail }}</p>
      </li>
    </ul>
    <p class="mt-3 text-sm text-ink-muted">
      The browser does not run the Lean checker. Save the certificate and run <code class="font-mono">axiom check</code> on it to check these claims.
    </p>
    <a class="btn mt-3 inline-block" :href="href" download="certificate.json">Save certificate</a>
  </section>
</template>
