<script setup lang="ts">
import { computed, onMounted, ref, shallowRef, watch } from "vue";
import AttemptsHistogram from "./AttemptsHistogram.vue";
import PlaybackControls from "./PlaybackControls.vue";
import RunningMeanPlot from "./RunningMeanPlot.vue";
import SamplePaths from "./SamplePaths.vue";
import SimulationVerdict from "./SimulationVerdict.vue";
import { SIM_CAPS, type Analysis, type Engine, type SimulateRequest, type Simulation, type SimulationError } from "../engine/types";
import { complement, count, fractionText } from "../format";
import { createClock } from "../playback";
import { encodeRun, inRange, isSeed, type SharedRun } from "../shareLink";

const props = defineProps<{ engine: Engine; source: string; analysis: Analysis; initialRun: SharedRun | null }>();

// The samples the Hoeffding bound needs for the default 1/100 and 1/1000,
// so a first run gets an estimate.
const DEFAULT_SAMPLES = 38_005;

const seed = ref(props.initialRun?.seed ?? "1");
const samples = ref(props.initialRun?.samples ?? DEFAULT_SAMPLES);
const steps = ref(props.initialRun?.steps ?? 100);
const maxAttempts = ref(props.initialRun?.maxAttempts ?? 100);
const running = ref(false);
const result = shallowRef<Simulation | null>(null);
const failure = shallowRef<SimulationError | null>(null);
const lastRequest = shallowRef<SimulateRequest | null>(null);
const seedChanged = ref(false);
const copied = ref<"seed" | "link" | null>(null);

// A finished run is replayed in 20 frames on the page's playback speed:
// paths draw, the running fraction advances and the bars grow.
const REPLAY_FRAMES = 20;
const replay = createClock();
const reveal = computed(() => (replay.last.value === 0 ? 1 : replay.t.value / replay.last.value));

const kind = computed(() => (props.analysis.chain ? "chain" : "retry"));
const seedValid = computed(() => isSeed(seed.value));
const samplesValid = computed(() => inRange(samples.value, SIM_CAPS.samples));
const stepsValid = computed(() => inRange(steps.value, SIM_CAPS.steps));
const attemptsValid = computed(() => inRange(maxAttempts.value, SIM_CAPS.attempts));
const canRun = computed(
  () => !running.value && seedValid.value && samplesValid.value && (kind.value === "chain" ? stepsValid.value : attemptsValid.value),
);

// What a run does, in plain words, from the model and the settings.
const explanation = computed(() => {
  const n = samplesValid.value ? count(samples.value) : "the";
  const chain = props.analysis.chain;
  if (chain) {
    const [first, second] = [chain.states[0] ?? "", chain.states[1] ?? chain.states[0] ?? ""];
    return `Runs ${n} copies of ${chain.name} from ${first} for ${steps.value} steps, rolling the chances in your model at each step, then counts how many end ${second}.`;
  }
  const success = props.analysis.retry ? fractionText(props.analysis.retry.success) : "";
  return `Runs ${n} requests, each tried up to ${maxAttempts.value} times with a ${success} chance of success per attempt, then counts the attempts each needed.`;
});

const guarantee = computed(() => {
  const sim = result.value;
  if (!sim) return null;
  const confidence = complement(sim.delta);
  return sim.estimate
    ? `With this many runs the count is within ${fractionText(sim.epsilon)} of the true chance at least ${confidence.num} times in ${confidence.den}.`
    : `That guarantee needs at least ${count(sim.requiredSamples)} runs.`;
});

// "Run k of n · step s" while the replay plays.
const progress = computed(() => {
  const sim = result.value;
  if (!sim) return "";
  const runs = Math.max(1, Math.round(reveal.value * sim.samples));
  const stepNow = sim.chain ? ` · step ${Math.round(reveal.value * Math.min(sim.chain.steps, 200))}` : "";
  return `Run ${count(runs)} of ${count(sim.samples)}${stepNow}`;
});

function newSeed(): void {
  const draw = new BigUint64Array(1);
  crypto.getRandomValues(draw);
  seed.value = (draw[0] ?? 0n).toString();
  seedChanged.value = result.value !== null;
}

// A run is for one version of the model; editing the model clears it.
watch(
  () => props.source,
  () => {
    result.value = null;
    failure.value = null;
    lastRequest.value = null;
  },
);

async function simulate(request: SimulateRequest): Promise<void> {
  running.value = true;
  copied.value = null;
  const outcome = await props.engine.simulate(props.source, request);
  running.value = false;
  if (outcome.ok) {
    result.value = outcome.simulation;
    failure.value = null;
    lastRequest.value = request;
    seedChanged.value = false;
    replay.restart(REPLAY_FRAMES);
    const fragment = encodeRun({ model: props.source, ...request });
    if (fragment) history.replaceState(null, "", fragment);
  } else {
    result.value = null;
    failure.value = outcome.error;
  }
}

function run(): void {
  if (!canRun.value) return;
  void simulate({ seed: seed.value, samples: samples.value, steps: steps.value, maxAttempts: maxAttempts.value });
}

// Runs the last request again: same seed, same settings, same result.
function rerun(): void {
  if (lastRequest.value && !running.value) void simulate(lastRequest.value);
}

async function copy(what: "seed" | "link", text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = what;
  } catch {
    copied.value = null;
  }
}

const copyLink = (): Promise<void> => copy("link", window.location.href);

onMounted(() => {
  if (props.initialRun && props.initialRun.model === props.source) run();
});
</script>

<template>
  <section id="simulate" class="panel" aria-labelledby="simulate-title">
    <h2 id="simulate-title" class="panel-title">2. Run it: simulate</h2>
    <p class="mb-4 text-sm text-ink">{{ explanation }}</p>

    <form class="grid gap-3 sm:grid-cols-[minmax(0,1.6fr)_repeat(2,minmax(0,1fr))] sm:items-end" @submit.prevent="run">
      <label class="field">
        <span>Seed <span class="text-xs">· same seed, same run</span></span>
        <span class="flex gap-2">
          <input v-model.trim="seed" class="input min-w-0 flex-1" inputmode="numeric" autocomplete="off" spellcheck="false" maxlength="20" :aria-invalid="!seedValid" aria-describedby="seed-help" />
          <button class="btn shrink-0" type="button" @click="newSeed">New seed</button>
        </span>
        <span v-if="!seedValid" id="seed-help" class="text-xs text-rejected">A whole number from 0 to 18446744073709551615.</span>
        <span v-else-if="seedChanged" id="seed-help" class="text-xs text-ink" aria-live="polite">New seed: the next run will differ from the last one.</span>
      </label>
      <label class="field">
        <span>Runs <span class="text-xs">· more runs, tighter estimate</span></span>
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
      <div class="flex flex-wrap items-center gap-2 sm:col-span-3">
        <button class="btn btn-primary" type="submit" :disabled="!canRun" :aria-busy="running">
          {{ running ? "Running" : samplesValid ? `Run ${count(samples)} runs` : "Run" }}
        </button>
        <template v-if="lastRequest">
          <button class="btn" type="button" :disabled="running" @click="rerun">Replay</button>
          <span class="text-xs text-ink-muted">Same seed, same result.</span>
        </template>
      </div>
    </form>

    <p v-if="failure" class="mt-4 border-l-[3px] border-rejected pl-3 text-sm" role="alert">
      <span class="font-mono text-rejected">{{ failure.code }}</span>
      <span v-if="failure.line !== null"> at line {{ failure.line }}, column {{ failure.column }}</span>: {{ failure.message }}
    </p>

    <div v-if="result" class="sim-result mt-5 grid gap-5">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1 border-b border-rule pb-2 text-sm text-ink-muted">
        <span>{{ count(result.samples) }} runs of {{ result.name }}, seed</span>
        <code class="select-all font-mono text-ink">{{ result.seed }}</code>
        <button class="btn py-0.5 text-xs" type="button" @click="copy('seed', result.seed)">{{ copied === "seed" ? "Seed copied" : "Copy seed" }}</button>
        <button class="btn py-0.5 text-xs" type="button" @click="copyLink">{{ copied === "link" ? "Link copied" : "Copy link to this run" }}</button>
      </div>
      <p class="text-sm text-ink">{{ guarantee }}</p>

      <PlaybackControls :clock="replay">{{ progress }}</PlaybackControls>

      <SimulationVerdict :result="result" />

      <template v-if="result.chain && analysis.chain">
        <SamplePaths :run="result.chain" :transitions="analysis.chain.transitions" :reveal="reveal" />
        <RunningMeanPlot :run="result.chain" :epsilon="result.epsilon" :reveal="reveal" />
      </template>
      <AttemptsHistogram v-if="result.retry" :run="result.retry" :reveal="reveal" />
    </div>
  </section>
</template>
