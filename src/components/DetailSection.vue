<script setup lang="ts">
defineProps<{
  label: string;
}>();
</script>

<template>
  <div>
    <p class="text-text-sub text-xs mb-2 font-medium uppercase tracking-wide">{{ label }}</p>
    <slot />
  </div>
</template>
