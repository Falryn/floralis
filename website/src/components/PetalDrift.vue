<script setup lang="ts">
const petals = Array.from({ length: 12 }, (_, i) => ({
  left: `${(i * 8.3 + (i % 3) * 2.4) % 100}%`,
  size: 10 + ((i * 7) % 12),
  duration: 9 + ((i * 13) % 8),
  delay: -((i * 17) % 12),
  hue: i % 3
}))

const hues = ['#ffadd2', '#c7c7ff', '#ffd6e7']
</script>

<template>
  <div class="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden="true">
    <span
      v-for="(p, idx) in petals"
      :key="idx"
      class="animate-petal absolute top-0 block"
      :style="{
        left: p.left,
        width: `${p.size}px`,
        height: `${p.size * 0.85}px`,
        background: `linear-gradient(135deg, ${hues[p.hue]}, #ffffff)`,
        borderRadius: '62% 38% 58% 42% / 55% 48% 52% 45%',
        animationDuration: `${p.duration}s`,
        animationDelay: `${p.delay}s`,
        opacity: 0
      }"
    />
  </div>
</template>
