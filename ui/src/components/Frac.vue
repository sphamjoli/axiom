<script setup lang="ts">
import type { Fraction } from "../engine/types";

defineProps<{ value: Fraction }>();
</script>

<template>
  <span v-if="value.den === '1'" class="font-mono tabular-nums">{{ value.num }}</span>
  <span v-else class="font-mono tabular-nums">{{ value.num }}/{{ value.den }}</span>
</template>
