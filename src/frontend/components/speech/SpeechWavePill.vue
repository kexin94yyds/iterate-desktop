<script setup lang="ts">
defineProps<{
  active: boolean
  tone: 'idle' | 'active' | 'success' | 'error'
  holding?: boolean
}>()
</script>

<template>
  <span
    class="speech-wave-pill"
    :class="[`speech-wave-pill--${tone}`, { 'speech-wave-pill--holding': holding }]"
    aria-hidden="true"
  >
    <span class="speech-wave-pill__wave" :class="{ active, idle: !active }">
      <span />
      <span />
      <span />
      <span />
      <span />
    </span>
  </span>
</template>

<style scoped>
.speech-wave-pill {
  position: relative;
  box-sizing: border-box;
  display: inline-flex;
  width: 58px;
  height: 34px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 18px;
  background: rgba(0, 0, 0, 0.72);
  backdrop-filter: blur(18px);
  -webkit-backdrop-filter: blur(18px);
}

.speech-wave-pill--active {
  background: rgba(0, 0, 0, 0.78);
}

.speech-wave-pill--success {
  background: rgba(0, 0, 0, 0.72);
}

.speech-wave-pill--error {
  background: rgba(52, 10, 10, 0.82);
}

.speech-wave-pill--holding::after {
  position: absolute;
  inset: 1px;
  border: 2px solid rgba(255, 255, 255, 0.88);
  border-left-color: transparent;
  border-radius: inherit;
  content: '';
  animation: speech-hold-progress 5s linear forwards;
}

.speech-wave-pill__wave {
  display: inline-flex;
  width: 42px;
  height: 24px;
  flex-shrink: 0;
  align-items: center;
  justify-content: center;
  gap: 4px;
  pointer-events: none;
  transition: transform 140ms ease, opacity 140ms ease;
}

.speech-wave-pill__wave.idle {
  opacity: 0.5;
}

.speech-wave-pill__wave.active {
  opacity: 1;
}

.speech-wave-pill__wave span {
  width: 3px;
  border-radius: 999px;
  background: linear-gradient(180deg, #e0e0e0, #808080);
  transform-origin: center;
  animation: speech-wave-breathe 1.05s ease-in-out infinite;
}

.speech-wave-pill__wave.idle span {
  animation: none;
  opacity: 0.5;
  transform: scaleY(0.5);
}

.speech-wave-pill__wave span:nth-child(1) { height: 7px; animation-delay: -0.18s; }
.speech-wave-pill__wave span:nth-child(2) { height: 12px; animation-delay: -0.42s; }
.speech-wave-pill__wave span:nth-child(3) { height: 16px; animation-delay: -0.08s; }
.speech-wave-pill__wave span:nth-child(4) { height: 12px; animation-delay: -0.3s; }
.speech-wave-pill__wave span:nth-child(5) { height: 7px; animation-delay: -0.14s; }

@keyframes speech-wave-breathe {
  0%,
  100% {
    opacity: 0.42;
    transform: scaleY(0.58);
  }

  50% {
    opacity: 1;
    transform: scaleY(1.06);
  }
}

@keyframes speech-hold-progress {
  from { clip-path: inset(0 100% 0 0 round 18px); }
  to { clip-path: inset(0 0 0 0 round 18px); }
}

@media (prefers-reduced-motion: reduce) {
  .speech-wave-pill__wave span {
    animation: none !important;
  }

  .speech-wave-pill--holding::after {
    animation-timing-function: steps(5, end);
  }
}
</style>
