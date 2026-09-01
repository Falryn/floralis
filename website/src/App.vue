<script setup lang="ts">
import NavBar from './components/NavBar.vue'
import HeroSection from './components/HeroSection.vue'
import FeaturesSection from './components/FeaturesSection.vue'
import GallerySection from './components/GallerySection.vue'
import DownloadSection from './components/DownloadSection.vue'
import DonateSection from './components/DonateSection.vue'
import FooterBar from './components/FooterBar.vue'
</script>

<template>
  <div class="min-h-screen">
    <NavBar />
    <main>
      <HeroSection />
      <FeaturesSection />
      <GallerySection />
      <DownloadSection />
      <DonateSection />
    </main>
    <FooterBar />
  </div>
</template>
