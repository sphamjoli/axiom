<script setup lang="ts">
import { computed } from "vue";
import type { ChainRun, Transition } from "../engine/types";
import { fractionText } from "../format";

// `reveal` in [0, 1] is how much of each path the replay has drawn.
// `transitions` name the chance behind each change of state, for the marks.
const props = defineProps<{ run: ChainRun; transitions: Transition[]; reveal: number }>();

function chance(from: string, to: string): string {
  const p = props.transitions.find((t) => t.from === from && t.to === to)?.p;
  return p ? fractionText(p) : "?";
}

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
    // A mark at every change of state, saying which roll caused it.
    const changes = drawn.flatMap((s, t) => {
      const before = drawn[t - 1];
      if (t === 0 || before === undefined || before === s) return [];
      const from = props.run.states[before] ?? "";
      const to = props.run.states[s] ?? "";
      return [{ t, x: x(t), y: top + level(s), label: `Step ${t}: the roll landed in the ${from} → ${to} slice, chance ${chance(from, to)}` }];
    });
    return { top, d, changes };
  });
  const height = M.t + rows.length * ROW + M.b;
  return { rows, height, longest, x, level };
});
</script>

<template>
  <figure>
    <figcaption class="mb-2 text-sm text-ink-muted">
      {{ run.paths.length }} sample paths from {{ run.states[0] }}, first {{ view.longest }} steps. Each dot is a roll that changed the state; hover it to see which.
      <span class="font-mono">(top: {{ run.states[0] }}, bottom: {{ run.states[run.states.length - 1] }})</span>
    </figcaption>
    <svg :viewBox="`0 0 ${W} ${view.height}`" class="h-auto w-full" role="img" :aria-label="`${run.paths.length} sample paths of the chain`">
      <g v-for="(row, i) in view.rows" :key="i">
        <line :x1="M.l" :x2="W - M.r" :y1="row.top + ROW - 4" :y2="row.top + ROW - 4" class="stroke-rule" />
        <text :x="M.l - 8" :y="row.top + ROW / 2 + 2" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">path {{ i + 1 }}</text>
        <path :d="row.d" fill="none" stroke-width="1.25" class="stroke-signal" />
        <circle v-for="c in row.changes" :key="c.t" :cx="c.x" :cy="c.y" r="3" class="fill-signal">
          <title>{{ c.label }}</title>
        </circle>
      </g>
      <text :x="M.l" :y="view.height - 6" class="fill-ink-muted font-mono text-[11px]">0</text>
      <text :x="W - M.r" :y="view.height - 6" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">step {{ view.longest }}</text>
    </svg>
  </figure>
</template>
