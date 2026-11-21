<script setup lang="ts">
import { computed } from "vue";
import Frac from "./Frac.vue";
import type { RetryRun } from "../engine/types";
import { count, fractionText } from "../format";

// `reveal` in [0, 1] grows the bars from the axis as the replay runs.
const props = defineProps<{ run: RetryRun; reveal: number }>();

const W = 640;
const H = 240;
const M = { l: 56, r: 12, t: 22, b: 30 };

const view = computed(() => {
  const buckets = props.run.histogram;
  const most = Math.max(1, ...buckets.map((b) => b.count));
  const slots = Math.max(1, buckets.length);
  const slot = (W - M.l - M.r) / slots;
  // Bucket a covers [a - 1/2, a + 1/2], so a mark at a mean of attempts
  // lines up with the bars.
  const x = (attempts: number): number => M.l + slot * (attempts - 0.5);
  const y = (c: number): number => H - M.b - ((H - M.t - M.b) * c) / most;
  const bars = buckets.map((b) => ({ ...b, x: x(b.attempts) - slot / 2 + 1, w: Math.max(1, slot - 2), y: y(b.count) }));
  const inside = (v: number | null): v is number => v !== null && v <= slots + 0.5;
  return { bars, most, x, y, slots, inside };
});
</script>

<template>
  <figure>
    <figcaption class="mb-2 text-sm text-ink-muted">Runs by the attempt on which they succeeded</figcaption>
    <svg :viewBox="`0 0 ${W} ${H}`" class="h-auto w-full" role="img" aria-label="Histogram of attempts until success">
      <line :x1="M.l" :x2="M.l" :y1="M.t" :y2="H - M.b" class="stroke-rule" />
      <line :x1="M.l" :x2="W - M.r" :y1="H - M.b" :y2="H - M.b" class="stroke-rule" />
      <g :transform="`translate(0 ${H - M.b}) scale(1 ${reveal}) translate(0 ${-(H - M.b)})`"><rect v-for="b in view.bars" :key="b.attempts" :x="b.x" :width="b.w" :y="b.y" :height="H - M.b - b.y" class="fill-signal/70">
        <title>{{ b.attempts }} attempts: {{ count(b.count) }} runs</title>
      </rect></g>
      <g v-if="view.inside(run.expectedApprox) && run.expectedAttempts">
        <line :x1="view.x(run.expectedApprox)" :x2="view.x(run.expectedApprox)" :y1="M.t - 6" :y2="H - M.b" stroke-dasharray="4 3" stroke-width="1.5" class="stroke-ink" />
        <text :x="view.x(run.expectedApprox) + 6" :y="M.t + 4" class="fill-ink font-mono text-[11px]">exact {{ fractionText(run.expectedAttempts) }}</text>
      </g>
      <g v-if="view.inside(run.meanApprox)">
        <line :x1="view.x(run.meanApprox ?? 0)" :x2="view.x(run.meanApprox ?? 0)" :y1="M.t + 12" :y2="H - M.b" stroke-width="1.5" class="stroke-estimated" />
        <text :x="view.x(run.meanApprox ?? 0) + 6" :y="M.t + 22" class="fill-estimated font-mono text-[11px]">sample mean</text>
      </g>
      <text :x="M.l - 8" :y="view.y(view.most) + 4" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">{{ count(view.most) }}</text>
      <text :x="M.l - 8" :y="H - M.b + 4" text-anchor="end" class="fill-ink-muted font-mono text-[11px]">0</text>
      <text :x="view.x(1)" :y="H - 10" text-anchor="middle" class="fill-ink-muted font-mono text-[11px]">1</text>
      <text :x="view.x(view.slots)" :y="H - 10" text-anchor="middle" class="fill-ink-muted font-mono text-[11px]">{{ view.slots }}</text>
    </svg>
    <dl class="mt-2 grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
      <dt class="text-ink-muted">Sample mean attempts</dt>
      <dd><Frac v-if="run.meanAttempts" :value="run.meanAttempts" /><span v-else>None: no run succeeded</span></dd>
      <dt class="text-ink-muted">Exact expected attempts</dt>
      <dd><Frac v-if="run.expectedAttempts" :value="run.expectedAttempts" /><span v-else>None: an attempt never succeeds</span></dd>
      <dt class="text-ink-muted">Runs that succeeded</dt>
      <dd><Frac :value="run.successFraction" /></dd>
      <template v-if="run.beyond > 0">
        <dt class="text-ink-muted">After attempt {{ view.slots }}</dt>
        <dd class="font-mono">{{ count(run.beyond) }} runs, not drawn</dd>
      </template>
      <template v-if="run.failures > 0">
        <dt class="text-ink-muted">No success in {{ run.maxAttempts }} attempts</dt>
        <dd class="font-mono">{{ count(run.failures) }} runs</dd>
      </template>
    </dl>
  </figure>
</template>
