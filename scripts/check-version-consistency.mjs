import fs from 'node:fs'

function read(path) {
  return fs.readFileSync(path, 'utf8')
}

function requireMatch(label, value, expected) {
  if (value !== expected) {
    console.error(`${label} version mismatch: expected ${expected}, got ${value}`)
    process.exitCode = 1
  }
}

const packageJson = JSON.parse(read('package.json'))
const expected = packageJson.version

const cargoToml = read('Cargo.toml')
const cargoVersion = cargoToml.match(/^version = "([^"]+)"/m)?.[1]

const tauriConfig = JSON.parse(read('tauri.conf.json'))
const readme = read('README.md')
const readmeBadgeVersion = readme.match(/img\.shields\.io\/badge\/version-([0-9.]+)-blue/)?.[1]

requireMatch('Cargo.toml', cargoVersion, expected)
requireMatch('tauri.conf.json', tauriConfig.version, expected)
requireMatch('README badge', readmeBadgeVersion, expected)

if (process.exitCode) {
  process.exit(process.exitCode)
}

console.log(`version consistency check passed: ${expected}`)
