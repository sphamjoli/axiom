<script setup lang="ts">
import { computed } from "vue";
import Frac from "./Frac.vue";
import type { ChainRun, Fraction } from "../engine/types";
import { approx, count, fractionText } from "../format";

// `reveal` in [0, 1] is how far through the samples the replay has got.
const props = defineProps<{ run: ChainRun; epsilon: Fraction; reveal: number }>();

const W = 640;
const H = 240;
const M = { l: 56, r: 12, t: 16, b: 30 };

// The vertical range centres on the exact steady state, wide enough for the
// band and for every point after the first tenth of the run, so the band is
// drawn at its true width rather than as a hairline.
const view = computed(() => {
  const pts = props.run.running;
  const n = pts[pts.length - 1]?.samples ?? 1;
  const eps = approx(props.epsilon);
  const centre = props.run.steadyApprox ?? pts[pts.length - 1]?.approx ?? 0.5;
  const settled = pts.filter((p) => p.samples >= n / 10).map((p) => Math.abs(p.approx - centre));
  const half = Math.min(0.5, Math.max(3 * eps, ...settled) * 1.15);
  const lo = centre - half;
  const hi = centre + half;
  const x = (k: number): number => M.l + ((W - M.l - M.r) * k) / n;
  const y = (v: number): number => H - M.b - ((H - M.t - M.b) * (Math.min(hi, Math.max(lo, v)) - lo)) / (hi - lo);
  const drawn = pts.slice(0, Math.max(1, Math.ceil(props.reveal * pts.length)));
  const line = drawn.map((p, i) => `${i === 0 ? "M" : "L"} ${x(p.samples)} ${y(p.approx)}`).join(" ");
  return { n, x, y, line, centre, eps };
});
</script>

<template>
  <figure>
    <figcaption class="mb-2 text-sm text-ink-muted">
      Fraction of runs in {{ run.state }} after {{ run.steps }} steps, over the first k samples
    </figcaption>
    <svg :viewBox="`0 0 ${W} ${H}`" class="h-auto w-full" role="img" :aria-label="`Running fraction of runs in ${run.state}`">
      <line :x1="M.l" :x2="M.l" :y1="M.t" :y2="H - M.b" class="stroke-rule" />
      <line :x1="M.l" :x2="W - M.r" :y1="H - M.b" :y2="H - M.b" class="stroke-rule" />
      <g v-if="run.steadyState">
        <rect :x="M.l" :width="W - M.l - M.r" :y="view.y(view.centre + view.eps)" :height="view.y(view.centre - view.eps) - view.y(view.centre + view.eps)" class="fill-estimated/15" />
        <line :x1="M.l" :x2="W - M.r" :y1="view.y(view.centre)" :y2="view.y(view.centre)" stroke-dasharray="4 4" class="stroke-ink-muted" />
        <text :x="M.l - 8" :y="view.y(view.centre) + 4" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">{{ fractionText(run.steadyState) }}</text>
        <text :x="W - M.r" :y="view.y(view.centre + view.eps) - 5" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">{{ fractionText(run.steadyState) }} &#177; {{ fractionText(epsilon) }}</text>
      </g>
      <path :d="view.line" fill="none" stroke-width="1.75" class="stroke-signal" />
      <text :x="M.l" :y="H - 10" class="fill-ink-muted font-mono text-[11px]">0</text>
      <text :x="W - M.r" :y="H - 10" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">k = {{ count(view.n) }}</text>
    </svg>
    <p class="mt-1 text-sm text-ink-muted">
      After all {{ count(view.n) }} samples: <Frac :value="run.fraction" />.
      <template v-if="run.steadyState">The exact steady state is <Frac :value="run.steadyState" />; the shaded band is &#177; <Frac :value="epsilon" />.</template>
      <template v-else>This chain does not settle to a steady state, so there is no exact value to compare with.</template>
    </p>
  </figure>
</template>
