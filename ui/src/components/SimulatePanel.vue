<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import AttemptsHistogram from "./AttemptsHistogram.vue";
import Frac from "./Frac.vue";
import RunningMeanPlot from "./RunningMeanPlot.vue";
import SamplePaths from "./SamplePaths.vue";
import StatusBadge from "./StatusBadge.vue";
import { SIM_CAPS, type Engine, type Simulation, type SimulationError } from "../engine/types";
import { complement, count } from "../format";
import { createClock } from "../playback";

const props = defineProps<{ engine: Engine; source: string; kind: "chain" | "retry" }>();

const U64_MAX = 18446744073709551615n;

// The samples the Hoeffding bound needs for the default 1/100 and 1/1000,
// so a first run gets an estimate.
const DEFAULT_SAMPLES = 38_005;

const seed = ref("1");
const samples = ref(DEFAULT_SAMPLES);
const steps = ref(100);
const maxAttempts = ref(100);
const running = ref(false);
const result = shallowRef<Simulation | null>(null);
const failure = shallowRef<SimulationError | null>(null);
const copied = ref(false);

// A finished run is replayed in 20 frames on the page's playback speed:
// paths draw, the running fraction advances and the bars grow.
const REPLAY_FRAMES = 20;
const replay = createClock();
const reveal = computed(() => (replay.last.value === 0 ? 1 : replay.t.value / replay.last.value));

function newSeed(): void {
  const draw = new BigUint64Array(1);
  crypto.getRandomValues(draw);
  seed.value = (draw[0] ?? 0n).toString();
}

const inRange = (value: number, max: number): boolean => Number.isInteger(value) && value >= 1 && value <= max;

const seedValid = computed(() => /^\d{1,20}$/.test(seed.value) && BigInt(seed.value) <= U64_MAX);
const samplesValid = computed(() => inRange(samples.value, SIM_CAPS.samples));
const stepsValid = computed(() => inRange(steps.value, SIM_CAPS.steps));
const attemptsValid = computed(() => inRange(maxAttempts.value, SIM_CAPS.attempts));
const canRun = computed(
  () => !running.value && seedValid.value && samplesValid.value && (props.kind === "chain" ? stepsValid.value : attemptsValid.value),
);

// A run is for one version of the model; editing the model clears it.
watch(
  () => props.source,
  () => {
    result.value = null;
    failure.value = null;
  },
);

async function run(): Promise<void> {
  if (!canRun.value) return;
  running.value = true;
  copied.value = false;
  const outcome = await props.engine.simulate(props.source, {
    seed: seed.value,
    samples: samples.value,
    steps: steps.value,
    maxAttempts: maxAttempts.value,
  });
  running.value = false;
  if (outcome.ok) {
    result.value = outcome.simulation;
    failure.value = null;
    replay.restart(REPLAY_FRAMES);
  } else {
    result.value = null;
    failure.value = outcome.error;
  }
}

async function copySeed(value: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(value);
    copied.value = true;
  } catch {
    copied.value = false;
  }
}

const estimateSubject = computed(() => {
  const sim = result.value;
  if (sim?.chain) return `P(${sim.chain.state} after ${sim.chain.steps} steps)`;
  return "Success per attempt";
});
</script>

<template>
  <section id="simulate" class="panel" aria-labelledby="simulate-title">
    <div class="mb-3 flex flex-wrap items-baseline justify-between gap-2">
      <h2 id="simulate-title" class="panel-title mb-0">2. Run it: simulate</h2>
      <p class="text-sm text-ink-muted">Seeded sampling in the browser. The same seed draws the same picture.</p>
    </div>

    <form class="grid gap-3 sm:grid-cols-[minmax(0,1.6fr)_repeat(2,minmax(0,1fr))] sm:items-end" @submit.prevent="run">
      <label class="field">
        <span>Seed <span class="text-xs">· same seed, same run</span></span>
        <span class="flex gap-2">
          <input v-model.trim="seed" class="input min-w-0 flex-1" inputmode="numeric" autocomplete="off" spellcheck="false" maxlength="20" :aria-invalid="!seedValid" aria-describedby="seed-help" />
          <button class="btn shrink-0" type="button" @click="newSeed">New seed</button>
        </span>
        <span v-if="!seedValid" id="seed-help" class="text-xs text-rejected">A whole number from 0 to 18446744073709551615.</span>
      </label>
      <label class="field">
        <span>Samples <span class="text-xs">· more samples, tighter estimate</span></span>
        <input v-model.number="samples" class="input" type="number" min="1" :max="SIM_CAPS.samples" step="1" :aria-invalid="!samplesValid" />
        <span v-if="!samplesValid" class="text-xs text-rejected">From 1 to {{ count(SIM_CAPS.samples) }}.</span>
      </label>
      <label v-if="kind === 'chain'" class="field">
        <span>Steps per run</span>
        <input v-model.number="steps" class="input" type="number" min="1" :max="SIM_CAPS.steps" step="1" :aria-invalid="!stepsValid" />
        <span v-if="!stepsValid" class="text-xs text-rejected">From 1 to {{ count(SIM_CAPS.steps) }}.</span>
      </label>
      <label v-else class="field">
        <span>Attempts per run, at most</span>
        <input v-model.number="maxAttempts" class="input" type="number" min="1" :max="SIM_CAPS.attempts" step="1" :aria-invalid="!attemptsValid" />
        <span v-if="!attemptsValid" class="text-xs text-rejected">From 1 to {{ count(SIM_CAPS.attempts) }}.</span>
      </label>
      <div class="sm:col-span-3">
        <button class="btn btn-primary" type="submit" :disabled="!canRun" :aria-busy="running">
          {{ running ? "Running" : samplesValid ? `Run ${count(samples)} samples` : "Run" }}
        </button>
      </div>
    </form>

    <p v-if="failure" class="mt-4 border-l-[3px] border-rejected pl-3 text-sm" role="alert">
      <span class="font-mono text-rejected">{{ failure.code }}</span>
      <span v-if="failure.line !== null"> at line {{ failure.line }}, column {{ failure.column }}</span>: {{ failure.message }}
    </p>

    <div v-if="result" class="sim-result mt-5 grid gap-5" aria-live="polite">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-rule pb-2 text-sm text-ink-muted">
        <span>{{ count(result.samples) }} samples of {{ result.name }}, seed</span>
        <code class="select-all font-mono text-ink">{{ result.seed }}</code>
        <button class="btn py-0.5 text-xs" type="button" @click="copySeed(result.seed)">{{ copied ? "Seed copied" : "Copy seed" }}</button>
      </div>

      <div class="border-l-[3px] border-dashed border-estimated pl-3">
        <div class="flex flex-wrap items-center gap-2">
          <StatusBadge status="estimated" />
          <p v-if="result.estimate" class="text-sm">
            {{ estimateSubject }} = <Frac :value="result.estimate.value" /> &#177; <Frac :value="result.estimate.epsilon" />, with probability at least
            <Frac :value="complement(result.estimate.delta)" />
          </p>
          <p v-else class="text-sm">No estimate: {{ count(result.samples) }} samples are fewer than the {{ count(result.requiredSamples) }} it needs.</p>
        </div>
        <p class="mt-1 text-xs text-ink-muted">
          An estimate comes from sampling and is not a proof. It is given only when the run has at least the {{ count(result.requiredSamples) }} samples that the Hoeffding bound needs for &#177;
          <Frac :value="result.epsilon" /> at failure probability <Frac :value="result.delta" />.
        </p>
        <p v-if="result.retry && result.estimate" class="mt-1 text-xs text-ink-muted">
          Estimated from first attempts only. The exact success per attempt is <Frac :value="result.retry.success" />.
        </p>
      </div>

      <template v-if="result.chain">
        <SamplePaths :run="result.chain" :reveal="reveal" />
        <RunningMeanPlot :run="result.chain" :epsilon="result.epsilon" :reveal="reveal" />
      </template>
      <AttemptsHistogram v-if="result.retry" :run="result.retry" :reveal="reveal" />
    </div>
  </section>
</template>
