import type { MaybeRefOrGetter } from 'vue'
import QRCode from 'qrcode'
import { ref, toValue, watch } from 'vue'

interface LocalQrCodeOptions {
  width?: number
  margin?: number
}

export function useLocalQrCode(source: MaybeRefOrGetter<string>, options: LocalQrCodeOptions = {}) {
  const dataUrl = ref('')
  const error = ref<Error | null>(null)
  const isGenerating = ref(false)
  let generation = 0

  watch(
    () => toValue(source).trim(),
    async (value) => {
      const currentGeneration = ++generation
      error.value = null

      if (!value) {
        dataUrl.value = ''
        isGenerating.value = false
        return
      }

      isGenerating.value = true
      try {
        const nextDataUrl = await QRCode.toDataURL(value, {
          errorCorrectionLevel: 'M',
          width: options.width ?? 320,
          margin: options.margin ?? 1,
          color: {
            dark: '#000000',
            light: '#ffffff',
          },
        })

        if (currentGeneration === generation)
          dataUrl.value = nextDataUrl
      }
      catch (err) {
        if (currentGeneration === generation) {
          dataUrl.value = ''
          error.value = err instanceof Error ? err : new Error(String(err))
        }
      }
      finally {
        if (currentGeneration === generation)
          isGenerating.value = false
      }
    },
    { immediate: true },
  )

  return {
    dataUrl,
    error,
    isGenerating,
  }
}
