<script setup lang="ts">
import { computed, ref, useSlots } from 'vue'

interface QuotaMetric {
  label: string
  remaining: number
  resetLabel?: string
  resetAtMs?: number
}

interface UsageProvider {
  id: string
  name: string
  accountLabel?: string
  color: string
  iconUrl?: string
  summary: string
  updatedAt?: string
  metrics: QuotaMetric[]
}

const props = defineProps<{
  providers: UsageProvider[]
  title?: string
  subtitle?: string
  statusLabel?: string
}>()

const isOpen = ref(false)
const slots = useSlots()
let closeTimer: number | undefined

const primaryProviders = computed(() => props.providers.slice(0, 4))
const hasCustomTrigger = computed(() => Boolean(slots.trigger))

function openPanel() {
  if (closeTimer)
    window.clearTimeout(closeTimer)
  isOpen.value = true
}

function scheduleClose() {
  if (closeTimer)
    window.clearTimeout(closeTimer)
  closeTimer = window.setTimeout(() => {
    isOpen.value = false
  }, 120)
}

function togglePanel() {
  if (closeTimer)
    window.clearTimeout(closeTimer)
  isOpen.value = !isOpen.value
}

function metricTone(metric: QuotaMetric) {
  if (metric.remaining <= 20)
    return 'danger'
  if (metric.remaining <= 45)
    return 'warn'
  return 'normal'
}
</script>

<template>
  <div
    class="usage-quota"
    @mouseenter="openPanel"
    @mouseleave="scheduleClose"
    @focusin="openPanel"
    @focusout="scheduleClose"
  >
    <button
      class="usage-trigger"
      type="button"
      :aria-expanded="isOpen"
      aria-label="查看 AI 用量"
      @click="togglePanel"
    >
      <slot name="trigger">
        <img src="/icons/icon-128.png" alt="" class="usage-trigger-icon">
        <span class="usage-trigger-text">
          <span class="usage-trigger-title">iterate</span>
          <span class="usage-trigger-subtitle">AI usage</span>
        </span>
      </slot>
      <template v-if="!hasCustomTrigger">
        <span
          v-for="provider in primaryProviders"
          :key="provider.id"
          class="usage-mini-dot"
        />
      </template>
    </button>

    <Transition name="usage-popover">
      <section
        v-if="isOpen"
        class="usage-panel"
        role="dialog"
        aria-label="AI 用量详情"
        @mouseenter="openPanel"
        @mouseleave="scheduleClose"
      >
        <header class="usage-panel-header">
          <div>
            <p class="usage-eyebrow">
              {{ subtitle || '本机用量' }}
            </p>
            <h3>{{ title || '额度' }}</h3>
          </div>
          <span class="usage-live-pill">{{ statusLabel || '预览' }}</span>
        </header>

        <div class="usage-provider-list">
          <article
            v-for="provider in providers"
            :key="provider.id"
            class="usage-provider"
          >
            <div class="usage-provider-head">
              <span
                class="usage-provider-icon"
              >
                <img
                  v-if="provider.iconUrl"
                  :src="provider.iconUrl"
                  :alt="`${provider.name} icon`"
                >
              </span>
              <div class="usage-provider-copy">
                <strong>{{ provider.name }}</strong>
                <span>{{ provider.summary }}</span>
              </div>
              <span v-if="provider.updatedAt" class="usage-updated">
                {{ provider.updatedAt }}
              </span>
            </div>

            <div class="usage-metrics">
              <div
                v-for="metric in provider.metrics"
                :key="`${provider.id}-${metric.label}`"
                class="usage-metric"
                :data-tone="metricTone(metric)"
              >
                <div class="usage-metric-line">
                  <span>{{ metric.label }}</span>
                  <strong>{{ metric.remaining }}%</strong>
                </div>
                <div class="usage-meter">
                  <span
                    :style="{
                      width: `${metric.remaining}%`,
                    }"
                  />
                </div>
                <small v-if="metric.resetLabel">{{ metric.resetLabel }}</small>
              </div>
            </div>
          </article>
        </div>
      </section>
    </Transition>
  </div>
</template>

<style scoped>
.usage-quota {
  position: relative;
  display: inline-flex;
  isolation: isolate;
  z-index: 1;
}

.usage-trigger {
  display: inline-flex;
  align-items: center;
  min-width: 164px;
  height: 46px;
  gap: 9px;
  padding: 5px 10px 5px 6px;
  border: 1px solid rgba(15, 23, 42, 0.12);
  border-radius: 10px;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(248, 250, 252, 0.92));
  color: #111827;
  cursor: pointer;
  box-shadow:
    0 8px 24px rgba(15, 23, 42, 0.08),
    inset 0 1px 0 rgba(255, 255, 255, 0.86);
  font: inherit;
  line-height: 1;
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}

.usage-trigger:hover,
.usage-trigger:focus-visible,
.usage-trigger[aria-expanded='true'] {
  border-color: rgba(37, 99, 235, 0.34);
  box-shadow:
    0 14px 32px rgba(15, 23, 42, 0.14),
    0 0 0 3px rgba(37, 99, 235, 0.08);
  transform: translateY(-1px);
  outline: none;
}

.usage-trigger-icon {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(15, 23, 42, 0.12);
}

.usage-trigger-text {
  display: grid;
  gap: 1px;
  text-align: left;
}

.usage-trigger-title {
  font-size: 14px;
  font-weight: 700;
  line-height: 1.05;
}

.usage-trigger-subtitle {
  color: #64748b;
  font-size: 10px;
  line-height: 1;
}

.usage-mini-dot {
  width: 7px;
  height: 7px;
  margin-left: -4px;
  border-radius: 999px;
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.9);
}

.usage-panel {
  position: absolute;
  z-index: 10000;
  top: calc(100% + 10px);
  left: 0;
  width: min(392px, calc(100vw - 32px));
  padding: 12px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 8px;
  background: #08090d;
  color: #f8fafc;
  box-shadow:
    0 22px 70px rgba(0, 0, 0, 0.5),
    inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.usage-panel::before {
  position: absolute;
  top: -6px;
  left: 22px;
  width: 11px;
  height: 11px;
  border-top: 1px solid rgba(255, 255, 255, 0.12);
  border-left: 1px solid rgba(255, 255, 255, 0.12);
  background: #08090d;
  content: '';
  transform: rotate(45deg);
}

.usage-panel-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.usage-eyebrow {
  margin: 0 0 2px;
  color: #8b93a7;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.usage-panel-header h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 800;
}

.usage-live-pill {
  padding: 3px 7px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.06);
  color: #a1a1aa;
  font-size: 10px;
  font-weight: 700;
}

.usage-provider-list {
  display: grid;
  gap: 8px;
}

.usage-provider {
  padding: 10px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  background: #111218;
}

.usage-provider-head {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: 9px;
  margin-bottom: 9px;
}

.usage-provider-icon {
  display: inline-grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 5px;
  background: #05060a;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

.usage-provider-icon img {
  width: 19px;
  height: 19px;
  object-fit: contain;
  display: block;
}

.usage-provider-copy {
  display: grid;
  min-width: 0;
}

.usage-provider-copy strong {
  color: #f8fafc;
  font-size: 13px;
  line-height: 1.2;
}

.usage-provider-copy span,
.usage-updated {
  overflow: hidden;
  color: #8b93a7;
  font-size: 11px;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.usage-metrics {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.usage-metric {
  min-width: 0;
}

.usage-metric-line {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
  color: #c4c7d0;
  font-size: 11px;
}

.usage-metric-line strong {
  color: #f8fafc;
  font-size: 11px;
}

.usage-metric[data-tone='warn'] .usage-metric-line strong {
  color: #f8fafc;
}

.usage-metric[data-tone='danger'] .usage-metric-line strong {
  color: #f8fafc;
}

.usage-meter {
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.1);
}

.usage-meter span {
  display: block;
  height: 100%;
  min-width: 4px;
  border-radius: inherit;
  background: linear-gradient(90deg, #f8fafc, #a1a1aa);
}

.usage-metric small {
  display: block;
  margin-top: 3px;
  overflow: hidden;
  color: #747b8c;
  font-size: 10px;
  line-height: 1.2;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.usage-popover-enter-active,
.usage-popover-leave-active {
  transition:
    opacity 0.14s ease,
    transform 0.14s ease;
}

.usage-popover-enter-from,
.usage-popover-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.985);
}

@media (max-width: 520px) {
  .usage-panel {
    width: calc(100vw - 48px);
  }

  .usage-metrics {
    grid-template-columns: 1fr;
  }
}
</style>
