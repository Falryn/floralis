<template>
  <svg
    viewBox="0 0 800 560"
    preserveAspectRatio="xMidYMid slice"
    class="h-full w-full"
    aria-hidden="true"
  >
    <defs>
      <linearGradient id="hero-bg" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0%" stop-color="#fce4f0" />
        <stop offset="52%" stop-color="#e8e0f7" />
        <stop offset="100%" stop-color="#dce8f8" />
      </linearGradient>
      <linearGradient id="ribbon-a" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0%" stop-color="#eb2f96" stop-opacity="0" />
        <stop offset="30%" stop-color="#eb2f96" stop-opacity="0.5" />
        <stop offset="70%" stop-color="#6c5ce7" stop-opacity="0.5" />
        <stop offset="100%" stop-color="#6c5ce7" stop-opacity="0" />
      </linearGradient>
      <radialGradient id="glow" cx="50%" cy="50%" r="50%">
        <stop offset="0%" stop-color="#ffffff" stop-opacity="0.85" />
        <stop offset="100%" stop-color="#ffffff" stop-opacity="0" />
      </radialGradient>
      <g id="flower5">
        <ellipse cx="0" cy="-21" rx="11" ry="20" fill="currentColor" />
        <ellipse cx="0" cy="-21" rx="11" ry="20" fill="currentColor" transform="rotate(72)" />
        <ellipse cx="0" cy="-21" rx="11" ry="20" fill="currentColor" transform="rotate(144)" />
        <ellipse cx="0" cy="-21" rx="11" ry="20" fill="currentColor" transform="rotate(216)" />
        <ellipse cx="0" cy="-21" rx="11" ry="20" fill="currentColor" transform="rotate(288)" />
        <circle r="8.5" fill="#ffd97a" />
      </g>
    </defs>

    <rect width="800" height="560" fill="url(#hero-bg)" />

    <circle cx="620" cy="120" r="220" fill="url(#glow)" />
    <circle cx="140" cy="440" r="200" fill="url(#glow)" opacity="0.7" />

    <path
      d="M -60 400 C 140 290, 300 500, 500 360 S 760 240, 870 300"
      fill="none"
      stroke="url(#ribbon-a)"
      stroke-width="24"
      stroke-linecap="round"
      opacity="0.65"
    />
    <path
      d="M -40 450 C 180 350, 340 540, 560 410 S 790 300, 870 350"
      fill="none"
      stroke="url(#ribbon-a)"
      stroke-width="9"
      stroke-linecap="round"
      opacity="0.5"
    />

    <use href="#flower5" transform="translate(120 130) scale(1.5)" style="color: #f759ab" opacity="0.9" />
    <use href="#flower5" transform="translate(660 400) scale(1.9) rotate(18)" style="color: #a3a3ff" opacity="0.9" />
    <use href="#flower5" transform="translate(700 90) scale(0.9) rotate(-14)" style="color: #ff85c0" opacity="0.85" />
    <use href="#flower5" transform="translate(220 470) scale(1.1) rotate(30)" style="color: #7c6dff" opacity="0.8" />
    <use href="#flower5" transform="translate(420 80) scale(0.7) rotate(45)" style="color: #eb2f96" opacity="0.7" />

    <g fill="#ffffff" opacity="0.9">
      <circle cx="320" cy="180" r="4" />
      <circle cx="540" cy="250" r="3" />
      <circle cx="180" cy="320" r="3.5" />
      <circle cx="620" cy="300" r="2.5" />
      <circle cx="460" cy="460" r="3" />
      <circle cx="80" cy="250" r="2.5" />
    </g>
    <g fill="#eb2f96" opacity="0.35">
      <circle cx="380" cy="120" r="6" />
      <circle cx="720" cy="220" r="5" />
      <circle cx="260" cy="400" r="5" />
    </g>
  </svg>
</template>
