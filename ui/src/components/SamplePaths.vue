<script setup lang="ts">
import { computed } from "vue";
import type { ChainRun } from "../engine/types";

// `reveal` in [0, 1] is how much of each path the replay has drawn.
const props = defineProps<{ run: ChainRun; reveal: number }>();

// One thin strip per path: small multiples share the step axis, and each
// state sits on its own level, the first state at the top.
const W = 640;
const ROW = 34;
const M = { l: 64, r: 8, t: 6, b: 22 };

const view = computed(() => {
  const states = props.run.states.length;
  const longest = Math.max(1, ...props.run.paths.map((p) => p.length - 1));
  const x = (t: number): number => M.l + ((W - M.l - M.r) * t) / longest;
  const level = (s: number): number => (states > 1 ? (ROW - 12) * (s / (states - 1)) : 0) + 4;
  const rows = props.run.paths.map((path, i) => {
    const top = M.t + i * ROW;
    const drawn = path.slice(0, Math.max(1, Math.ceil(props.reveal * (path.length - 1)) + 1));
    const d = drawn
      .map((s, t) => {
        const y = top + level(s);
        return t === 0 ? `M ${x(0)} ${y}` : `H ${x(t)} V ${y}`;
      })
      .join(" ");
    return { top, d };
  });
  const height = M.t + rows.length * ROW + M.b;
  return { rows, height, longest, x, level };
});
</script>

<template>
  <figure>
    <figcaption class="mb-2 text-sm text-ink-muted">
      {{ run.paths.length }} sample paths from {{ run.states[0] }}, first {{ view.longest }} steps
      <span class="font-mono">(top: {{ run.states[0] }}, bottom: {{ run.states[run.states.length - 1] }})</span>
    </figcaption>
    <svg :viewBox="`0 0 ${W} ${view.height}`" class="h-auto w-full" role="img" :aria-label="`${run.paths.length} sample paths of the chain`">
      <g v-for="(row, i) in view.rows" :key="i">
        <line :x1="M.l" :x2="W - M.r" :y1="row.top + ROW - 4" :y2="row.top + ROW - 4" class="stroke-rule" />
        <text :x="M.l - 8" :y="row.top + ROW / 2 + 2" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">path {{ i + 1 }}</text>
        <path :d="row.d" fill="none" stroke-width="1.25" class="stroke-signal" />
      </g>
      <text :x="M.l" :y="view.height - 6" class="fill-ink-muted font-mono text-[11px]">0</text>
      <text :x="W - M.r" :y="view.height - 6" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">step {{ view.longest }}</text>
    </svg>
  </figure>
</template>
