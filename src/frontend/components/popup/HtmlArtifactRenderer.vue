<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = withDefaults(defineProps<{
  content: string
  currentTheme?: string
  title?: string
  showChrome?: boolean
}>(), {
  currentTheme: 'dark',
  title: 'HTML Artifact',
  showChrome: true,
})

const frameRef = ref<HTMLIFrameElement | null>(null)
const frameHeight = ref(520)
const frameId = `html-artifact-${Math.random().toString(36).slice(2)}`

const MIN_FRAME_HEIGHT = 360
const MAX_FRAME_HEIGHT = 1200

const normalizedContent = computed(() => props.content.replace(/\\n/g, '\n'))

const srcDoc = computed(() => buildArtifactDocument(
  normalizedContent.value,
  props.currentTheme,
  frameId,
))

watch(srcDoc, () => {
  frameHeight.value = 520
}, { immediate: true })

function clampFrameHeight(height: number) {
  return Math.min(MAX_FRAME_HEIGHT, Math.max(MIN_FRAME_HEIGHT, height))
}

function handleFrameMessage(event: MessageEvent) {
  if (!frameRef.value?.contentWindow || event.source !== frameRef.value.contentWindow)
    return

  const data = event.data as {
    source?: string
    frameId?: string
    type?: string
    height?: number
  } | null

  if (
    !data
    || data.source !== 'iterate-html-artifact'
    || data.frameId !== frameId
    || data.type !== 'resize'
    || typeof data.height !== 'number'
  ) {
    return
  }

  frameHeight.value = clampFrameHeight(Math.ceil(data.height))
}

onMounted(() => {
  window.addEventListener('message', handleFrameMessage)
})

onBeforeUnmount(() => {
  window.removeEventListener('message', handleFrameMessage)
})

function buildArtifactDocument(content: string, theme: string, id: string) {
  const isFullDocument = /<!doctype\s+html/i.test(content) || /<html[\s>]/i.test(content)
  const headAdditions = [
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; img-src data: blob:; media-src data: blob:; style-src \'unsafe-inline\'; script-src \'unsafe-inline\' blob:; font-src data:; connect-src \'none\'; frame-src \'none\'; object-src \'none\'; base-uri \'none\'; form-action \'none\'">',
  ].join('')
  const bridgeScript = createBridgeScript(id)

  if (isFullDocument) {
    const withHead = /<head[\s>]/i.test(content)
      ? content.replace(/<head([^>]*)>/i, `<head$1>${headAdditions}`)
      : content.replace(/<html([^>]*)>/i, `<html$1><head>${headAdditions}</head>`)

    if (/<\/body>/i.test(withHead))
      return withHead.replace(/<\/body>/i, `${bridgeScript}</body>`)

    return `${withHead}${bridgeScript}`
  }

  return `<!doctype html>
<html>
  <head>
    ${headAdditions}
    ${createBaseStyle(theme)}
  </head>
  <body>
    ${content}
    ${bridgeScript}
  </body>
</html>`
}

function createBaseStyle(theme: string) {
  const isLight = theme === 'light'
  const background = isLight ? '#f7f7f2' : '#101114'
  const foreground = isLight ? '#202124' : '#f4f4ef'
  const muted = isLight ? '#5b6169' : '#a3a7ae'
  const panel = isLight ? '#ffffff' : '#17191d'
  const border = isLight ? '#d9ddd4' : '#30343a'

  return `<style>
    :root {
      color-scheme: ${isLight ? 'light' : 'dark'};
      --artifact-bg: ${background};
      --artifact-fg: ${foreground};
      --artifact-muted: ${muted};
      --artifact-panel: ${panel};
      --artifact-border: ${border};
      --artifact-green: #35c481;
      --artifact-orange: #f97316;
      --artifact-blue: #5ba7f7;
      --artifact-red: #ef4444;
    }

    * {
      box-sizing: border-box;
    }

    html,
    body {
      margin: 0;
      min-height: 100%;
      background: var(--artifact-bg);
      color: var(--artifact-fg);
      font-family: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      font-size: 14px;
      line-height: 1.55;
    }

    body {
      padding: 20px;
    }

    h1,
    h2,
    h3,
    p {
      margin-top: 0;
    }

    h1 {
      font-size: 25px;
      line-height: 1.1;
    }

    h2 {
      font-size: 17px;
      line-height: 1.2;
    }

    p {
      color: var(--artifact-muted);
    }

    button,
    input,
    select,
    textarea {
      font: inherit;
    }

    code,
    pre {
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    }
  </style>`
}

function createBridgeScript(id: string) {
  return `<script>
    (() => {
      const frameId = ${JSON.stringify(id)};
      let frame = 0;
      let delayedMeasure = 0;
      let resizeObserver = null;

      function measure() {
        const height = Math.max(
          document.body ? document.body.scrollHeight : 0,
          document.documentElement ? document.documentElement.scrollHeight : 0
        );
        parent.postMessage({
          source: 'iterate-html-artifact',
          frameId,
          type: 'resize',
          height
        }, '*');
      }

      function scheduleMeasure() {
        cancelAnimationFrame(frame);
        frame = requestAnimationFrame(measure);
      }

      function cleanup() {
        cancelAnimationFrame(frame);
        clearTimeout(delayedMeasure);
        resizeObserver?.disconnect();
        window.removeEventListener('load', scheduleMeasure);
        window.removeEventListener('resize', scheduleMeasure);
        window.removeEventListener('pagehide', cleanup);
        document.removeEventListener('input', scheduleMeasure);
        document.removeEventListener('click', scheduleMeasure);
      }

      window.addEventListener('load', scheduleMeasure);
      window.addEventListener('resize', scheduleMeasure);
      window.addEventListener('pagehide', cleanup);
      document.addEventListener('input', scheduleMeasure);
      document.addEventListener('click', scheduleMeasure);

      if ('ResizeObserver' in window) {
        resizeObserver = new ResizeObserver(scheduleMeasure);
        resizeObserver.observe(document.body || document.documentElement);
      }

      scheduleMeasure();
      delayedMeasure = setTimeout(scheduleMeasure, 120);
    })();
  <\/script>`
}
</script>

<template>
  <section
    class="html-artifact-renderer"
    :class="currentTheme === 'light' ? 'html-artifact-renderer--light' : 'html-artifact-renderer--dark'"
  >
    <div v-if="showChrome" class="html-artifact-renderer__bar">
      <div class="html-artifact-renderer__title">
        <div class="i-carbon-application-web w-3.5 h-3.5" />
        <span>{{ title }}</span>
      </div>
      <div class="html-artifact-renderer__meta">
        <span>HTML</span>
        <span>Sandboxed</span>
      </div>
    </div>
    <iframe
      :key="srcDoc"
      ref="frameRef"
      class="html-artifact-renderer__frame"
      :srcdoc="srcDoc"
      :style="{ height: `${frameHeight}px` }"
      title="HTML Artifact preview"
      sandbox="allow-scripts"
    />
  </section>
</template>

<style scoped>
.html-artifact-renderer {
  overflow: hidden;
  border: 1px solid;
  border-radius: 8px;
}

.html-artifact-renderer--dark {
  background: #101114;
  border-color: rgba(255, 255, 255, 0.12);
}

.html-artifact-renderer--light {
  background: #f7f7f2;
  border-color: rgba(31, 35, 40, 0.14);
}

.html-artifact-renderer__bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 34px;
  padding: 0 10px;
  border-bottom: 1px solid;
  font-size: 12px;
}

.html-artifact-renderer--dark .html-artifact-renderer__bar {
  border-bottom-color: rgba(255, 255, 255, 0.1);
  color: #e5e7eb;
}

.html-artifact-renderer--light .html-artifact-renderer__bar {
  border-bottom-color: rgba(31, 35, 40, 0.12);
  color: #202124;
}

.html-artifact-renderer__title,
.html-artifact-renderer__meta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.html-artifact-renderer__title {
  font-weight: 600;
}

.html-artifact-renderer__meta {
  color: #8b949e;
  font-size: 11px;
}

.html-artifact-renderer__meta span {
  padding: 2px 6px;
  border: 1px solid currentColor;
  border-radius: 999px;
}

.html-artifact-renderer__frame {
  display: block;
  width: 100%;
  min-height: 360px;
  border: 0;
  background: transparent;
  transition: height 160ms ease;
}
</style>
