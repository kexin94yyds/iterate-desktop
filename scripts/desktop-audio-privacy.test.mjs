import assert from 'node:assert/strict'
import { readFile, readdir } from 'node:fs/promises'
import path from 'node:path'
import test from 'node:test'

const root = new URL('../', import.meta.url)

async function source(relativePath) {
  return readFile(new URL(relativePath, root), 'utf8')
}

test('custom audio uses a managed local import instead of paths or URLs', async () => {
  const [frontend, backend] = await Promise.all([
    source('src/frontend/components/settings/AudioSettings.vue'),
    source('src/rust/ui/audio.rs'),
  ])

  assert.match(frontend, /invoke<CustomAudioImportResult \| null>\('import_custom_audio'\)/)
  assert.match(frontend, /不会保存原始路径/)
  assert.doesNotMatch(frontend, /音效文件路径或URL|https:\/\/example\.com\/notification/)
  assert.doesNotMatch(backend, /reqwest::get\(|play_audio_from_url/)
  assert.match(backend, /store_managed_custom_audio/)
})

test('Bridge cannot write or expose the configured audio source', async () => {
  const bridge = await source('src/rust/bridge/ws.rs')

  assert.match(bridge, /values\.len\(\) != 1 \|\| !values\.contains_key\("notification_enabled"\)/)
  assert.match(bridge, /\| "custom_url"/)
})

test('every bundled audio file is explicitly excluded from the public source snapshot', async () => {
  const manifest = JSON.parse(await source('open-source-manifest.json'))
  const resourceDirectory = new URL('src/rust/assets/resources/', root)
  const entries = await readdir(resourceDirectory)
  const audioExtensions = new Set(['.aac', '.flac', '.m4a', '.mp3', '.ogg', '.wav'])
  const audioFiles = entries
    .filter(entry => audioExtensions.has(path.extname(entry).toLowerCase()))
    .map(entry => `src/rust/assets/resources/${entry}`)
    .sort()
  const excludedAudioFiles = manifest.excludeFiles
    .filter((file) => {
      return file.startsWith('src/rust/assets/resources/')
        && audioExtensions.has(path.extname(file).toLowerCase())
    })
    .sort()

  assert.equal(excludedAudioFiles.length, 10)
  assert.deepEqual(
    audioFiles.filter(file => !manifest.excludeFiles.includes(file)),
    [],
  )
})
