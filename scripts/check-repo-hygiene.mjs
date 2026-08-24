import { execFileSync } from 'node:child_process'

const blockedExact = new Set([
  '.cunzhi-memory/checkpoints.jsonl',
  '.cunzhi-memory/checkpoint_links.jsonl',
  '.cunzhi-memory/tasks.json',
])

const blockedPrefixes = [
  '.cunzhi-memory/app-workflow-runs/',
  '.cunzhi-memory/auto-transport-runs/',
  '.cunzhi-memory/runtime-logs/',
  '.cunzhi-memory/codex-room/',
  '.cunzhi-memory/cpu-guard/',
  '.cunzhi-memory/mcp-response-channel-smoke-runs/',
  '.cunzhi-memory/public-stability-runs/',
  '.cunzhi-memory/release-parity-runs/',
]

function gitLsFiles() {
  const output = execFileSync('git', ['ls-files', '-z'], { encoding: 'utf8' })
  return output.split('\0').filter(Boolean)
}

const tracked = gitLsFiles()
const violations = tracked.filter((path) => {
  const isRootMemoryFile = path.startsWith('.cunzhi-memory/')
    && !path.slice('.cunzhi-memory/'.length).includes('/')
  const isUnpromotedRootJson = isRootMemoryFile
    && path.endsWith('.json')
    && path !== '.cunzhi-memory/metadata.json'
  const isRootJsonl = isRootMemoryFile && path.endsWith('.jsonl')

  return blockedExact.has(path)
    || blockedPrefixes.some(prefix => path.startsWith(prefix))
    || isUnpromotedRootJson
    || isRootJsonl
})

if (violations.length > 0) {
  console.error('Tracked volatile cunzhi memory files must be removed from the index with git rm --cached:')
  for (const path of violations)
    console.error(`- ${path}`)
  process.exit(1)
}

console.log('repo hygiene check passed')
