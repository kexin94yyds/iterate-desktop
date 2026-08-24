#!/usr/bin/env node
import { execFileSync } from 'node:child_process'
import process from 'node:process'

const args = process.argv.slice(2)
const json = args.includes('--json')
const tasksMdIndex = args.indexOf('--tasks-md')
const tasksMdPath = tasksMdIndex >= 0 ? args[tasksMdIndex + 1] : ''
const explicitFiles = args.filter((arg, index) => {
  if (arg.startsWith('--'))
    return false
  if (tasksMdIndex >= 0 && index === tasksMdIndex + 1)
    return false
  return true
})

function gitFiles(commandArgs) {
  try {
    return execFileSync('git', commandArgs, { encoding: 'utf8' })
      .split(/\r?\n/)
      .map(line => line.trim())
      .filter(Boolean)
  }
  catch {
    return []
  }
}

function unique(values) {
  return [...new Set(values)]
}

function changedFiles() {
  if (explicitFiles.length > 0)
    return unique(explicitFiles)

  const staged = gitFiles(['diff', '--name-only', '--cached'])
  const unstaged = gitFiles(['diff', '--name-only'])
  const untracked = gitFiles(['ls-files', '--others', '--exclude-standard'])
  const files = unique([...staged, ...unstaged, ...untracked])

  if (files.length > 0)
    return files

  return gitFiles(['diff-tree', '--no-commit-id', '--name-only', '-r', 'HEAD'])
}

const rules = [
  {
    platform: 'macos',
    reason: 'macOS app packaging, signing, notarization, native APIs, or updater behavior may be affected.',
    patterns: [
      /^src\/rust\/native_speech\/.*mac/i,
      /^src\/rust\/.*mac/i,
      /^scripts\/(prepare-macos-delivery|notarize-macos-app)\.sh$/,
      /^Entitlements\.plist$/,
      /^\.github\/workflows\/macos-sign-notarize\.yml$/,
      /^tauri\.conf\.json$/,
    ],
  },
  {
    platform: 'windows',
    reason: 'Windows binary, package layout, helper scripts, or cross-platform Tauri build may be affected.',
    patterns: [
      /^release-package\/windows\//,
      /^scripts\/.*\.ps1$/,
      /^\.github\/workflows\/windows-package\.yml$/,
      /^src\/rust\/.*windows/i,
      /^tauri\.conf\.json$/,
    ],
  },
  {
    platform: 'ios',
    reason: 'iOS bridge, simulator validation, APNS, or mobile companion behavior may be affected.',
    patterns: [
      /^ios-bridge-dev\//,
      /^scripts\/verify-ios-/,
      /^src\/rust\/.*ios/i,
      /^src\/rust\/.*apns/i,
      /^mobile\.html$/,
    ],
  },
  {
    platform: 'web',
    reason: 'Frontend, browser bridge, mobile web, or static deployment behavior may be affected.',
    patterns: [
      /^src\/frontend\//,
      /^browser-extension\//,
      /^mobile\.html$/,
      /^src\/rust\/bridge\/bridge_test\.html$/,
      /^package\.json$/,
      /^vite\.config\./,
    ],
  },
  {
    platform: 'shared-core',
    reason: 'Shared Rust, protocol, MCP, room, config, or conversation logic may affect every platform.',
    patterns: [
      /^src\/rust\//,
      /^src\/bin\//,
      /^mcp-server\//,
      /^Cargo\.toml$/,
      /^Cargo\.lock$/,
      /^scripts\/codex-room/,
    ],
  },
  {
    platform: 'release',
    reason: 'Release scripts, workflows, docs, or package metadata changed; run package/release readiness loops.',
    patterns: [
      /^\.github\/workflows\//,
      /^scripts\/.*(release|readiness|delivery|notarize|stability|sync-version).*/,
      /^release-package\//,
      /^docs\/.*(安装|release|delivery|发布).*/i,
      /^package\.json$/,
      /^tauri\.conf\.json$/,
    ],
  },
  {
    platform: 'database',
    reason: 'Database schema or fulfillment behavior may require migration dry-run and rollback review.',
    patterns: [
      /^migrations\//,
      /^server\//,
      /^docs\/.*migration.*/i,
      /^docs\/.*fulfillment.*/i,
    ],
  },
]

const loopByPlatform = {
  macos: ['macOS install package loop', 'auto-update loop'],
  windows: ['Windows install package loop', 'cross-platform sync loop'],
  ios: ['iOS bridge/simulator loop', 'cross-platform sync loop'],
  web: ['frontend deployment loop', 'UI visual regression loop'],
  'shared-core': ['cross-platform sync loop', 'AI bug-fix loop', 'regression-test loop'],
  release: ['GitHub Release integrity loop', 'macOS install package loop', 'Windows install package loop'],
  database: ['backend deployment loop', 'database migration safety loop'],
}

const files = changedFiles()
const impacts = rules
  .map((rule) => {
    const matches = files.filter(file => rule.patterns.some(pattern => pattern.test(file)))
    return {
      platform: rule.platform,
      impacted: matches.length > 0,
      reason: rule.reason,
      files: matches,
      loops: loopByPlatform[rule.platform] || [],
    }
  })
  .filter(item => item.impacted)

const recommendedLoops = unique(impacts.flatMap(item => item.loops))
const result = {
  checked_files: files,
  impacted_platforms: impacts.map(({ platform, reason, files, loops }) => ({
    platform,
    reason,
    files,
    loops,
  })),
  recommended_loops: recommendedLoops,
}

function markdownTasks() {
  const lines = [
    '# Cross-Platform Impact Tasks',
    '',
    `Generated: ${new Date().toISOString()}`,
    '',
    '## Changed Files',
    '',
    ...files.map(file => `- ${file}`),
    '',
    '## Impacted Platforms',
    '',
  ]

  if (impacts.length === 0) {
    lines.push('- No platform impact detected by current rules.')
  }
  else {
    for (const impact of impacts) {
      lines.push(`### ${impact.platform}`)
      lines.push('')
      lines.push(impact.reason)
      lines.push('')
      lines.push('Files:')
      for (const file of impact.files)
        lines.push(`- ${file}`)
      lines.push('')
      lines.push('Tasks:')
      for (const loop of impact.loops)
        lines.push(`- [ ] Run ${loop}`)
      lines.push('')
    }
  }

  lines.push('## Recommended Loop Checklist')
  lines.push('')
  for (const loop of recommendedLoops)
    lines.push(`- [ ] ${loop}`)
  lines.push('')
  return `${lines.join('\n')}\n`
}

if (tasksMdPath) {
  const { writeFileSync } = await import('node:fs')
  writeFileSync(tasksMdPath, markdownTasks())
}

if (json) {
  console.log(JSON.stringify(result, null, 2))
}
else {
  console.log('iterate cross-platform impact')
  console.log(`files=${files.length}`)
  for (const file of files)
    console.log(`  - ${file}`)
  console.log('')
  console.log(`impacted_platforms=${impacts.length}`)
  for (const impact of impacts) {
    console.log(`\n[${impact.platform}] ${impact.reason}`)
    for (const file of impact.files)
      console.log(`  - ${file}`)
    console.log(`  loops: ${impact.loops.join(', ')}`)
  }
  console.log('')
  console.log(`recommended_loops=${recommendedLoops.join(', ') || '<none>'}`)
}
