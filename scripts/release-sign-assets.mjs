#!/usr/bin/env node
import { createHash, createPrivateKey, createPublicKey, sign } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import process from 'node:process'

const DEFAULT_PRIVATE_KEY_ENV = 'ITERATE_RELEASE_PRIVATE_KEY_B64'
const ED25519_SPKI_PREFIX_HEX = '302a300506032b6570032100'

function usage() {
  return [
    'Usage: node scripts/release-sign-assets.mjs [options] <asset...>',
    '',
    'Options:',
    '  --out-dir <dir>          Write companion files to this directory; defaults beside each asset.',
    '  --dry-run                Print planned companion files without writing them.',
    '  --print-public-key       Print the raw Ed25519 public key for ITERATE_RELEASE_PUBLIC_KEY_B64.',
    `  --private-key-env <name>  Private key env var; default ${DEFAULT_PRIVATE_KEY_ENV}.`,
    '  --help                   Show this help.',
    '',
    'Private key format: base64 Ed25519 PKCS#8 DER.',
  ].join('\n')
}

function parseArgs(argv) {
  const options = {
    dryRun: false,
    outDir: '',
    printPublicKey: false,
    privateKeyEnv: DEFAULT_PRIVATE_KEY_ENV,
    assets: [],
  }

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--dry-run') {
      options.dryRun = true
    }
    else if (arg === '--print-public-key') {
      options.printPublicKey = true
    }
    else if (arg === '--out-dir') {
      index += 1
      if (!argv[index]) throw new Error('--out-dir requires a value')
      options.outDir = argv[index]
    }
    else if (arg === '--private-key-env') {
      index += 1
      if (!argv[index]) throw new Error('--private-key-env requires a value')
      options.privateKeyEnv = argv[index]
    }
    else if (arg === '--help' || arg === '-h') {
      options.help = true
    }
    else if (arg.startsWith('--')) {
      throw new Error(`unknown option: ${arg}`)
    }
    else {
      options.assets.push(arg)
    }
  }

  return options
}

function loadReleasePrivateKey(envName) {
  const encoded = process.env[envName]
  if (!encoded || !encoded.trim()) {
    throw new Error(`${envName} is required and must contain a base64 Ed25519 PKCS#8 private key`)
  }
  return createPrivateKey({
    key: Buffer.from(encoded.trim(), 'base64'),
    format: 'der',
    type: 'pkcs8',
  })
}

function rawEd25519PublicKeyBase64(privateKey) {
  const publicKey = createPublicKey(privateKey)
  const spki = publicKey.export({ format: 'der', type: 'spki' })
  const prefix = Buffer.from(ED25519_SPKI_PREFIX_HEX, 'hex')
  if (spki.length !== prefix.length + 32 || !spki.subarray(0, prefix.length).equals(prefix)) {
    throw new Error('failed to export raw Ed25519 public key from SPKI material')
  }
  return spki.subarray(prefix.length).toString('base64')
}

function companionPath(assetPath, outDir, suffix) {
  const basename = path.basename(assetPath)
  const directory = outDir ? path.resolve(outDir) : path.dirname(path.resolve(assetPath))
  return path.join(directory, `${basename}${suffix}`)
}

async function signAsset(assetPath, privateKey, options) {
  const payload = await readFile(assetPath)
  const basename = path.basename(assetPath)
  const digest = createHash('sha256').update(payload).digest('hex')
  const signature = sign(null, payload, privateKey).toString('base64')
  const sha256Path = companionPath(assetPath, options.outDir, '.sha256')
  const signaturePath = companionPath(assetPath, options.outDir, '.sig')
  const sha256Body = `${digest}  ${basename}\n`
  const signatureBody = `ed25519:${signature}\n`

  if (!options.dryRun) {
    await mkdir(path.dirname(sha256Path), { recursive: true })
    await writeFile(sha256Path, sha256Body, { mode: 0o644 })
    await writeFile(signaturePath, signatureBody, { mode: 0o644 })
  }

  return {
    asset: path.resolve(assetPath),
    sha256Path,
    signaturePath,
    sha256: digest,
    wrote: !options.dryRun,
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2))
  if (options.help) {
    console.log(usage())
    return
  }
  if (options.assets.length === 0) {
    throw new Error('at least one asset path is required')
  }

  const privateKey = loadReleasePrivateKey(options.privateKeyEnv)
  if (options.printPublicKey) {
    console.log(`ITERATE_RELEASE_PUBLIC_KEY_B64=${rawEd25519PublicKeyBase64(privateKey)}`)
  }

  for (const asset of options.assets) {
    const result = await signAsset(asset, privateKey, options)
    const mode = result.wrote ? 'wrote' : 'dry-run'
    console.log(`${mode}: ${result.asset}`)
    console.log(`  sha256: ${result.sha256Path}`)
    console.log(`  sig:    ${result.signaturePath}`)
    console.log(`  digest: ${result.sha256}`)
  }
}

main().catch((error) => {
  console.error(`release-sign-assets: ${error.message}`)
  process.exit(1)
})
