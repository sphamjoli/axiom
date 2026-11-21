import { computed, ref } from "vue";

// One playback clock for the page. The convergence plot and the transition
// graph read the same step t; the simulation replay uses the same speed and
// motion policy. It never loops: it plays to the last step and stops.

export const SPEEDS = [
  { label: "Slow", ms: 500 },
  { label: "Normal", ms: 400 },
  { label: "Fast", ms: 300 },
] as const;

const reducedQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
export const reducedMotion = ref(reducedQuery.matches);
reducedQuery.addEventListener("change", (event) => {
  reducedMotion.value = event.matches;
});

export const speedMs = ref<number>(SPEEDS[1].ms);

// A clock over steps 0..last. `t` is the step shown.
export function createClock() {
  const t = ref(0);
  const last = ref(0);
  const playing = ref(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function stop(): void {
    clearTimeout(timer);
    playing.value = false;
  }

  function tick(): void {
    if (t.value >= last.value) {
      stop();
      return;
    }
    t.value += 1;
    timer = setTimeout(tick, speedMs.value);
  }

  // Plays from the current step to the end; with reduced motion it shows the
  // end at once.
  function play(): void {
    stop();
    if (reducedMotion.value) {
      t.value = last.value;
      return;
    }
    if (t.value >= last.value) t.value = 0;
    playing.value = true;
    timer = setTimeout(tick, speedMs.value);
  }

  function step(): void {
    stop();
    t.value = Math.min(last.value, t.value + 1);
  }

  // Sets the range and plays it from step 0.
  function restart(steps: number): void {
    stop();
    last.value = Math.max(0, steps);
    t.value = 0;
    play();
  }

  // Sets the range and shows its end without playing.
  function settle(steps: number): void {
    stop();
    last.value = Math.max(0, steps);
    t.value = last.value;
  }

  const done = computed(() => t.value >= last.value);
  return { t, last, playing, done, play, pause: stop, step, restart, settle };
}

export type Clock = ReturnType<typeof createClock>;

// The clock the convergence plot and the transition graph share.
export const modelClock = createClock();
