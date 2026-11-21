<script setup lang="ts">
import { computed } from "vue";
import Frac from "./Frac.vue";
import type { Convergence } from "../engine/types";

const props = defineProps<{ data: Convergence }>();

const W = 880;
const H = 280;
const M = { l: 56, r: 16, t: 20, b: 44 };

const ratio = (f: { num: string; den: string }): number => Number(f.num) / Number(f.den);

const view = computed(() => {
  const pts = props.data.series;
  const tMax = pts.length > 0 ? (pts[pts.length - 1]?.t ?? 1) : 1;
  const values = pts.map((p) => p.approx);
  const lo = Math.min(0, ...values);
  const hi = Math.max(1, ...values);
  const x = (t: number): number => M.l + ((W - M.l - M.r) * t) / Math.max(tMax, 1);
  const y = (v: number): number => H - M.b - ((H - M.t - M.b) * (v - lo)) / (hi - lo);
  const fixed = ratio(props.data.fixedPoint);
  const eps = ratio(props.data.epsilon);
  // The band is far thinner than a pixel at this scale, so draw it at least 4px tall.
  const bandTop = Math.min(y(fixed + eps), y(fixed) - 2);
  const bandBottom = Math.max(y(fixed - eps), y(fixed) + 2);
  const line = pts.map((p, i) => `${i === 0 ? "M" : "L"} ${x(p.t)} ${y(p.approx)}`).join(" ");
  const xTicks = Array.from({ length: Math.floor(tMax / 5) + 1 }, (_, i) => i * 5);
  return { x, y, line, fixed, bandTop, bandBottom, xTicks, tMax };
});
</script>

<template>
  <section class="panel">
    <div class="mb-3 flex flex-wrap items-baseline justify-between gap-2">
      <h2 class="panel-title mb-0">Convergence of P({{ data.state }})</h2>
      <p class="text-sm text-ink-muted">
        d<sub>t+1</sub> = <Frac :value="data.a" /> d<sub>t</sub> + <Frac :value="data.b" />, d<sub>0</sub> = <Frac :value="data.initial" />
      </p>
    </div>
    <div class="overflow-x-auto"><svg :viewBox="`0 0 ${W} ${H}`" class="h-auto w-full min-w-[600px]" role="img" aria-label="Convergence plot">
      <line :x1="M.l" :x2="M.l" :y1="M.t" :y2="H - M.b" class="stroke-rule" />
      <line :x1="M.l" :x2="W - M.r" :y1="H - M.b" :y2="H - M.b" class="stroke-rule" />
      <g v-for="t in view.xTicks" :key="t">
        <text :x="view.x(t)" :y="H - M.b + 18" text-anchor="middle" class="fill-ink-muted font-mono text-[11px]">{{ t }}</text>
      </g>
      <text :x="M.l - 8" :y="view.y(1) + 4" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">1</text>
      <text :x="M.l - 8" :y="view.y(0) + 4" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">0</text>
      <rect :x="M.l" :width="W - M.l - M.r" :y="view.bandTop" :height="view.bandBottom - view.bandTop" class="fill-signal/15" />
      <line :x1="M.l" :x2="W - M.r" :y1="view.y(view.fixed)" :y2="view.y(view.fixed)" stroke-dasharray="4 4" class="stroke-ink-muted" />
      <text :x="M.l - 8" :y="view.y(view.fixed) + 4" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">{{ data.fixedPoint.num }}/{{ data.fixedPoint.den }}</text>
      <text :x="W - M.r" :y="view.bandTop - 5" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">band {{ data.fixedPoint.num }}/{{ data.fixedPoint.den }} &#177; {{ data.epsilon.num }}/{{ data.epsilon.den }}</text>
      <text :x="(M.l + W - M.r) / 2" :y="H - 6" text-anchor="middle" class="fill-ink-muted text-[12px]">step t</text>
      <text :x="14" :y="(M.t + H - M.b) / 2" text-anchor="middle" :transform="`rotate(-90 14 ${(M.t + H - M.b) / 2})`" class="fill-ink-muted text-[12px]">P({{ data.state }}) at step t</text>
      <g v-if="data.horizon !== null">
        <line :x1="view.x(data.horizon)" :x2="view.x(data.horizon)" :y1="M.t" :y2="H - M.b" stroke-dasharray="3 3" class="stroke-ink-muted" />
        <text :x="view.x(data.horizon) + 6" :y="M.t + 12" class="fill-ink font-mono text-[12px]" :text-anchor="view.x(data.horizon) > W * 0.6 ? 'end' : 'start'" :dx="view.x(data.horizon) > W * 0.6 ? -12 : 0">t = {{ data.horizon }}, within 1/1000 from here</text>
      </g>
      <path :d="view.line" fill="none" stroke-width="2.25" class="stroke-signal" />
      <circle v-for="p in data.series" :key="p.t" :cx="view.x(p.t)" :cy="view.y(p.approx)" r="2.5" class="fill-signal">
        <title>t = {{ p.t }}: {{ p.value.num }}/{{ p.value.den }}</title>
      </circle>
    </svg></div>
    <p v-if="data.horizon === null" class="mt-2 text-sm text-ink-muted">
      Not yet within <Frac :value="data.epsilon" /> of <Frac :value="data.fixedPoint" /> after {{ view.tMax }} steps.
    </p>
    <p v-else class="mt-2 text-sm text-ink-muted">
      Within <Frac :value="data.epsilon" /> of <Frac :value="data.fixedPoint" /> for every t &ge; {{ data.horizon }} (shaded band, widened for visibility).
    </p>
  </section>
</template>
