<script setup lang="ts">
import type { ClaimStatus } from "../engine/types";

defineProps<{ status: ClaimStatus }>();

// Proven is a solid filled tag; Estimated is a dashed outline with no fill and
// a different glyph, so the two differ in shape as well as colour. Unchecked, a claim the engine
// computed but the checker has not verified, is a dotted neutral outline.
const styles: Record<ClaimStatus, { label: string; glyph: string; cls: string }> = {
  proven: { label: "Proven", glyph: "∎", cls: "border-proven bg-proven text-sheet" },
  estimated: { label: "Estimated", glyph: "≈", cls: "border-dashed border-estimated text-estimated" },
  rejected: { label: "Rejected", glyph: "✗", cls: "border-rejected text-rejected" },
  unchecked: { label: "Unchecked", glyph: "?", cls: "border-dotted border-unchecked text-unchecked" },
};
</script>

<template>
  <span :class="['inline-flex shrink-0 items-center gap-1.5 rounded border-[1.5px] px-2 py-0.5 font-mono text-xs font-medium', styles[status].cls]">
    <span aria-hidden="true">{{ styles[status].glyph }}</span>{{ styles[status].label }}
  </span>
</template>
