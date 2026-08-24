<script setup lang="ts">
import { computed } from 'vue'

interface Props {
  theme: string
}

const props = defineProps<Props>()

// 显示点击后会切换到的主题图标，这样更符合用户直觉
const iconClass = computed(() => {
  return props.theme === 'light' ? 'i-carbon-moon' : 'i-carbon-sun'
})
</script>

<template>
  <div
    class="inline-flex items-center justify-center w-4 h-4 transition-transform duration-300 hover:rotate-180"
    :class="[iconClass]"
  />
</template>
