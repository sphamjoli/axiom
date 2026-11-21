<script setup lang="ts">
import { SPEEDS, speedMs, type Clock } from "../playback";

defineProps<{ clock: Clock }>();
</script>

<template>
  <div class="flex flex-wrap items-center gap-2 text-sm">
    <button v-if="clock.playing.value" class="btn" type="button" @click="clock.pause()">Pause</button>
    <button v-else class="btn" type="button" @click="clock.play()">{{ clock.done.value ? "Play again" : "Play" }}</button>
    <button class="btn" type="button" :disabled="clock.done.value" @click="clock.step()">Step</button>
    <label class="flex items-center gap-1.5 text-ink-muted">
      Speed
      <select v-model.number="speedMs" class="input py-1">
        <option v-for="s in SPEEDS" :key="s.ms" :value="s.ms">{{ s.label }}, {{ s.ms }} ms a step</option>
      </select>
    </label>
    <span class="font-mono text-ink"><slot /></span>
  </div>
</template>
