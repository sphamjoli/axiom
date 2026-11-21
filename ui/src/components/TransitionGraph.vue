<script setup lang="ts">
import { computed } from "vue";
import type { Chain } from "../engine/types";

const props = defineProps<{ chain: Chain }>();

const W = 420;
const H = 240;
const R = 30;

const nodes = computed(() => {
  const n = props.chain.states.length;
  return props.chain.states.map((name, i) => {
    const angle = Math.PI + (2 * Math.PI * i) / n;
    const rx = n === 2 ? 120 : 130;
    return { name, x: W / 2 + rx * Math.cos(angle), y: H / 2 + 20 + 60 * Math.sin(angle) };
  });
});

const label = (num: string, den: string): string => (den === "1" ? num : `${num}/${den}`);

const edges = computed(() =>
  props.chain.transitions
    .filter((t) => t.p.num !== "0")
    .flatMap((t) => {
      const a = nodes.value.find((n) => n.name === t.from);
      const b = nodes.value.find((n) => n.name === t.to);
      if (a === undefined || b === undefined) {
        return [];
      }
      const text = label(t.p.num, t.p.den);
      if (a === b) {
        const d = `M ${a.x - 14} ${a.y - R + 4} C ${a.x - 40} ${a.y - 95}, ${a.x + 40} ${a.y - 95}, ${a.x + 14} ${a.y - R + 4}`;
        return [{ key: `${t.from}-${t.to}`, d, text, lx: a.x, ly: a.y - 82 }];
      }
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const len = Math.hypot(dx, dy);
      const ux = dx / len;
      const uy = dy / len;
      // Bend each direction to its own side so opposite edges never overlap.
      const nx = uy;
      const ny = -ux;
      const bend = 36;
      const sx = a.x + ux * R + nx * 8;
      const sy = a.y + uy * R + ny * 8;
      const ex = b.x - ux * (R + 4) + nx * 8;
      const ey = b.y - uy * (R + 4) + ny * 8;
      const cx = (a.x + b.x) / 2 + nx * bend;
      const cy = (a.y + b.y) / 2 + ny * bend;
      const d = `M ${sx} ${sy} Q ${cx} ${cy} ${ex} ${ey}`;
      return [{ key: `${t.from}-${t.to}`, d, text, lx: cx, ly: cy + (ny > 0 ? 14 : -6) }];
    }),
);
</script>

<template>
  <section class="panel">
    <h2 class="panel-title">Transition graph</h2>
    <svg :viewBox="`0 0 ${W} ${H}`" class="h-auto w-full" role="img" :aria-label="`Transition graph of chain ${chain.name}`">
      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M 0 0 L 10 5 L 0 10 z" class="fill-ink-muted" />
        </marker>
      </defs>
      <g v-for="e in edges" :key="e.key">
        <path :d="e.d" fill="none" stroke-width="1.75" marker-end="url(#arrow)" class="stroke-ink-muted" />
        <text :x="e.lx" :y="e.ly" text-anchor="middle" class="fill-ink font-mono text-[13px]">{{ e.text }}</text>
      </g>
      <g v-for="n in nodes" :key="n.name">
        <circle :cx="n.x" :cy="n.y" :r="R" stroke-width="2" class="fill-paper stroke-ink" />
        <text :x="n.x" :y="n.y + 5" text-anchor="middle" class="fill-ink font-mono text-sm font-medium">{{ n.name }}</text>
      </g>
    </svg>
  </section>
</template>
