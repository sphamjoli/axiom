<script setup lang="ts">
import { computed, ref } from "vue";
import StatusBadge from "./StatusBadge.vue";
import type { Certificate, ClaimStatus } from "../engine/types";

const props = defineProps<{ certificate: Certificate }>();

const COMMAND = "axiom check certificate.json";
const copied = ref(false);

async function copyCommand(): Promise<void> {
  try {
    await navigator.clipboard.writeText(COMMAND);
    copied.value = true;
  } catch {
    copied.value = false;
  }
}

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
  <section id="certificate" class="panel">
    <h2 class="panel-title">3. Certificate (version {{ certificate.version }})</h2>
    <p class="mb-3 text-sm text-ink"><StatusBadge status="unchecked" class="mr-1.5 align-middle" /> means the engine computed the claim; it becomes Proven only when the Lean checker accepts it.</p>
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
    <div class="mt-3 flex flex-wrap items-center gap-2">
      <a class="btn" :href="href" download="certificate.json">Save certificate</a>
      <code class="select-all rounded-md border border-rule bg-paper px-2.5 py-1 font-mono text-sm">{{ COMMAND }}</code>
      <button class="btn" type="button" @click="copyCommand">{{ copied ? "Command copied" : "Copy command" }}</button>
    </div>
  </section>
</template>
