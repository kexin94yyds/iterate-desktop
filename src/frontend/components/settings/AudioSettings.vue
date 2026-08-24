<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref, watch } from 'vue'

interface AudioAsset {
  id: string
  name: string
  filename: string
}

interface CustomAudioImportResult {
  reference: string
}

const props = defineProps({
  audioNotificationEnabled: {
    type: Boolean,
    required: true,
  },
  audioUrl: {
    type: String,
    default: '',
  },
})

const emit = defineEmits([
  'toggleAudioNotification',
  'updateAudioUrl',
  'testAudio',
  'stopAudio',
  'testAudioError',
])

const presetSounds = ref<AudioAsset[]>([])
const loading = ref(true)
const selectedSoundType = ref<'preset' | 'custom'>('custom')
const selectedPreset = ref('')
const customReference = ref('')
const isImporting = ref(false)

const customConfigured = computed(() => {
  return customReference.value.startsWith('managed-audio:')
})

async function loadAudioAssets() {
  try {
    loading.value = true
    presetSounds.value = await invoke<AudioAsset[]>('get_available_audio_assets')
  }
  catch (error) {
    presetSounds.value = []
    console.error('加载音频资源失败:', error)
  }
  finally {
    loading.value = false
  }
}

function isPresetId(id: string) {
  return presetSounds.value.some(sound => sound.id === id)
}

function initializeState() {
  customReference.value = props.audioUrl.startsWith('managed-audio:') ? props.audioUrl : ''

  if (isPresetId(props.audioUrl)) {
    selectedSoundType.value = 'preset'
    selectedPreset.value = props.audioUrl
    return
  }

  if (customConfigured.value || presetSounds.value.length === 0) {
    selectedSoundType.value = 'custom'
    selectedPreset.value = ''
    return
  }

  selectedSoundType.value = 'preset'
  selectedPreset.value = presetSounds.value[0]?.id ?? ''
}

function stopPreviousAudio() {
  emit('stopAudio')
}

function selectPreset(presetId: string) {
  selectedSoundType.value = 'preset'
  selectedPreset.value = presetId
  emit('updateAudioUrl', presetId)
  stopPreviousAudio()
  emit('testAudio')
}

function selectCustom() {
  if (!customConfigured.value)
    return

  selectedSoundType.value = 'custom'
  emit('updateAudioUrl', customReference.value)
}

async function importCustomAudio() {
  isImporting.value = true
  try {
    const result = await invoke<CustomAudioImportResult | null>('import_custom_audio')
    if (!result)
      return

    customReference.value = result.reference
    selectedSoundType.value = 'custom'
    selectedPreset.value = ''
    emit('updateAudioUrl', result.reference)
    await invoke('test_audio_sound')
  }
  catch (error) {
    console.error('导入自定义提示音失败:', error)
    emit('testAudioError', error)
  }
  finally {
    isImporting.value = false
  }
}

watch(() => props.audioUrl, initializeState)

onMounted(async () => {
  await loadAudioAssets()
  initializeState()
})
</script>

<template>
  <n-space vertical size="large">
    <div class="flex items-center justify-between">
      <div class="flex items-center">
        <div class="w-1.5 h-1.5 bg-warning rounded-full mr-3 flex-shrink-0" />
        <div>
          <div class="text-sm font-medium leading-relaxed">
            音频通知
          </div>
          <div class="text-xs opacity-60">
            启用后在 MCP 工具被触发时播放音频提示
          </div>
        </div>
      </div>
      <n-switch
        :value="audioNotificationEnabled"
        size="small"
        @update:value="$emit('toggleAudioNotification')"
      />
    </div>

    <div v-if="audioNotificationEnabled" class="pt-4 border-t border-gray-200 dark:border-gray-700">
      <div class="flex items-start">
        <div class="w-1.5 h-1.5 bg-warning rounded-full mr-3 mt-2 flex-shrink-0" />
        <div class="flex-1 min-w-0">
          <div class="text-sm font-medium mb-3 leading-relaxed">
            音效选择
          </div>

          <div v-if="loading" class="text-xs opacity-60 mb-4">
            加载中...
          </div>

          <div v-else-if="presetSounds.length > 0" class="mb-4">
            <div class="text-xs opacity-60 mb-2">
              预设音效
            </div>
            <n-space>
              <n-button
                v-for="preset in presetSounds"
                :key="preset.id"
                :type="selectedSoundType === 'preset' && selectedPreset === preset.id ? 'primary' : 'default'"
                size="small"
                @click="selectPreset(preset.id)"
              >
                {{ preset.name }}
              </n-button>
            </n-space>
          </div>

          <div v-else class="mb-4 rounded-lg border border-dashed border-gray-300 dark:border-gray-600 px-3 py-2.5">
            <div class="text-xs font-medium">
              默认保持静音
            </div>
            <div class="text-xs opacity-60 mt-1 leading-relaxed">
              公开版不内置第三方提示音，也不会联网下载音频。你可以从本机导入自己的文件。
            </div>
          </div>

          <div class="mb-3">
            <div class="flex items-center justify-between gap-3 mb-2">
              <div>
                <div class="text-xs opacity-60">
                  自定义提示音
                </div>
                <div class="text-xs opacity-50 mt-1">
                  支持 MP3、WAV、OGG、M4A，最大 10MB
                </div>
              </div>
              <n-space size="small">
                <n-button
                  v-if="customConfigured"
                  :type="selectedSoundType === 'custom' ? 'primary' : 'default'"
                  size="tiny"
                  @click="selectCustom"
                >
                  使用自定义
                </n-button>
                <n-button
                  type="primary"
                  size="tiny"
                  :loading="isImporting"
                  @click="importCustomAudio"
                >
                  <template #icon>
                    <div class="i-carbon-upload text-sm" />
                  </template>
                  {{ customConfigured ? '更换文件' : '选择本地文件' }}
                </n-button>
              </n-space>
            </div>

            <div class="text-xs opacity-60 leading-relaxed">
              文件会复制到 iterate 的应用数据目录；不会保存原始路径，也不会上传或通过 Bridge 返回。
            </div>
          </div>

          <div class="mt-3 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs">
            <span class="opacity-60">当前音效：</span>
            <span v-if="selectedSoundType === 'preset' && selectedPreset">
              {{ presetSounds.find(preset => preset.id === selectedPreset)?.name }}
            </span>
            <span v-else-if="customConfigured">
              本地自定义提示音
            </span>
            <span v-else class="opacity-60">
              未设置（保持静音）
            </span>
          </div>
        </div>
      </div>
    </div>
  </n-space>
</template>
