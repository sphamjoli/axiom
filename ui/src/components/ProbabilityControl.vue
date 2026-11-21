<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Fraction, Range } from "../engine/types";
import { approx, fractionText as text, fromHundredths, inInterval, parseProbability, rangeText } from "../format";

// `range`, when given, limits the value to an interval, open ends excluded.
const props = defineProps<{ id: string; label: string; value: Fraction; range?: Range }>();
const emit = defineEmits<{ change: [value: Fraction] }>();

// The slider snaps to hundredths; the text field takes any exact fraction.
const typed = ref(text(props.value));
watch(
  () => props.value,
  (value) => {
    typed.value = text(value);
  },
);

const allowed = (value: Fraction): boolean => !props.range || inInterval(value, props.range);

// The hundredths inside the range: the first and last the slider may take.
const steps = computed(() => {
  const range = props.range;
  if (!range) return { min: 0, max: 100 };
  const hundredths = (value: Fraction, up: boolean): number => {
    const scaled = BigInt(value.num) * 100n;
    const den = BigInt(value.den);
    const floor = scaled / den;
    return Number(up && floor * den < scaled ? floor + 1n : floor);
  };
  let min = hundredths(range.lo, true);
  let max = hundredths(range.hi, false);
  if (!allowed(fromHundredths(min / 100))) min += 1;
  if (!allowed(fromHundredths(max / 100))) max -= 1;
  return { min, max };
});

const parsed = computed(() => parseProbability(typed.value));
const valid = computed(() => parsed.value !== null && allowed(parsed.value));
const hint = computed(() => (props.range ? `A fraction n/d in ${rangeText(props.range)}, such as 1/10.` : "A fraction n/d from 0 to 1, such as 1/10."));

function slide(event: Event): void {
  emit("change", fromHundredths(Number((event.target as HTMLInputElement).value) / 100));
}

function commit(): void {
  if (parsed.value && valid.value) emit("change", parsed.value);
}
</script>

<template>
  <div class="grid gap-1">
    <label :for="`${id}-range`" class="text-sm text-ink-muted">{{ label }}</label>
    <div class="flex items-center gap-3">
      <input
        v-if="steps.min <= steps.max"
        :id="`${id}-range`"
        class="range min-w-0 flex-1"
        type="range"
        :min="steps.min"
        :max="steps.max"
        step="1"
        :value="Math.round(approx(value) * 100)"
        :aria-valuetext="text(value)"
        @input="slide"
      />
      <input
        :id="steps.min <= steps.max ? undefined : `${id}-range`"
        v-model.trim="typed"
        class="input w-24 shrink-0"
        :aria-label="`${label}, exact fraction`"
        :aria-invalid="!valid"
        :aria-describedby="valid ? undefined : `${id}-hint`"
        inputmode="text"
        spellcheck="false"
        maxlength="19"
        @change="commit"
        @keydown.enter.prevent="commit"
      />
    </div>
    <span v-if="!valid" :id="`${id}-hint`" class="text-xs text-rejected">{{ hint }}</span>
  </div>
</template>
