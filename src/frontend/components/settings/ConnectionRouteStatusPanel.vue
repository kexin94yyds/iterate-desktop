<script setup lang="ts">
import type { ConnectionRouteView } from './useConnectionRouteStatus'

defineProps<{
  routeView: ConnectionRouteView
  compact?: boolean
}>()

function routeLanes(view: ConnectionRouteView) {
  return [
    view.localBridge,
    view.tailscale,
    view.publicRoute,
  ]
}
</script>

<template>
  <div class="connection-route-status-panel">
    <n-alert
      v-if="routeView.showPublicMaintenanceHint"
      type="info"
      :bordered="false"
      class="mb-3"
    >
      {{ routeView.maintenanceHint }}
    </n-alert>

    <div
      class="grid gap-2"
      :class="compact ? '' : 'p-3 bg-black-100 rounded-lg'"
    >
      <div
        v-if="!compact"
        class="text-xs font-medium opacity-70 mb-1"
      >
        连接通道
      </div>

      <div
        v-for="lane in routeLanes(routeView)"
        :key="lane.title"
        class="flex items-start gap-2"
      >
        <div
          class="w-1.5 h-1.5 rounded-full flex-shrink-0 mt-1.5"
          :class="lane.dotClass"
        />
        <div class="min-w-0 flex-1">
          <div class="text-sm font-medium leading-relaxed">
            {{ lane.title }}
          </div>
          <div class="text-xs opacity-60 leading-relaxed">
            {{ lane.detail }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
