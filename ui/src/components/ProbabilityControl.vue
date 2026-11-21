<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Fraction } from "../engine/types";
import { fromHundredths, parseProbability, text } from "../modelText";
import { approx } from "../format";

const props = defineProps<{ id: string; label: string; value: Fraction }>();
const emit = defineEmits<{ change: [value: Fraction] }>();

// The slider snaps to hundredths; the text field takes any exact fraction.
const typed = ref(text(props.value));
watch(
  () => props.value,
  (value) => {
    typed.value = text(value);
  },
);
const valid = computed(() => parseProbability(typed.value) !== null);

function slide(event: Event): void {
  emit("change", fromHundredths(Number((event.target as HTMLInputElement).value)));
}

function commit(): void {
  const parsed = parseProbability(typed.value);
  if (parsed) emit("change", parsed);
}
</script>

<template>
  <div class="grid gap-1">
    <label :for="`${id}-range`" class="text-sm text-ink-muted">{{ label }}</label>
    <div class="flex items-center gap-3">
      <input
        :id="`${id}-range`"
        class="range min-w-0 flex-1"
        type="range"
        min="0"
        max="1"
        step="0.01"
        :value="approx(value)"
        :aria-valuetext="text(value)"
        @input="slide"
      />
      <input
        v-model.trim="typed"
        class="input w-24 shrink-0"
        :aria-label="`${label}, exact fraction`"
        :aria-invalid="!valid"
        inputmode="text"
        spellcheck="false"
        maxlength="19"
        @change="commit"
        @keydown.enter.prevent="commit"
      />
    </div>
    <span v-if="!valid" class="text-xs text-rejected">A fraction n/d from 0 to 1, such as 1/10.</span>
  </div>
</template>
