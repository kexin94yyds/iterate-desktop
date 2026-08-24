import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { createHash, generateKeyPairSync, verify } from 'node:crypto'
import { mkdir, mkdtemp, readFile, stat, writeFile } from 'node:fs/promises'
import os from 'node:os'
import path from 'node:path'
import process from 'node:process'
import { promisify } from 'node:util'
import test from 'node:test'

const execFileAsync = promisify(execFile)
const scriptPath = path.resolve('scripts/release-sign-assets.mjs')
const spkiPrefix = Buffer.from('302a300506032b6570032100', 'hex')

async function tempDir() {
  return mkdtemp(path.join(os.tmpdir(), 'iterate-release-signing-'))
}

function releaseKeyPair() {
  const { privateKey, publicKey } = generateKeyPairSync('ed25519')
  const privateDer = privateKey.export({ format: 'der', type: 'pkcs8' })
  const publicDer = publicKey.export({ format: 'der', type: 'spki' })
  return {
    privateKey,
    publicKey,
    privateKeyB64: privateDer.toString('base64'),
    rawPublicKeyB64: publicDer.subarray(spkiPrefix.length).toString('base64'),
  }
}

async function exists(filePath) {
  try {
    await stat(filePath)
    return true
  }
  catch {
    return false
  }
}

test('release signer writes SHA-256 and detached Ed25519 signature companions', async () => {
  const dir = await tempDir()
  const outDir = path.join(dir, 'companions')
  const assetPath = path.join(dir, 'iterate-linux-x86_64.AppImage')
  const payload = Buffer.from('fake release payload\n')
  await writeFile(assetPath, payload)

  const keys = releaseKeyPair()
  const { stdout } = await execFileAsync(process.execPath, [
    scriptPath,
    '--out-dir',
    outDir,
    '--print-public-key',
    assetPath,
  ], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      ITERATE_RELEASE_PRIVATE_KEY_B64: keys.privateKeyB64,
    },
  })

  const expectedSha = createHash('sha256').update(payload).digest('hex')
  const shaText = await readFile(path.join(outDir, 'iterate-linux-x86_64.AppImage.sha256'), 'utf8')
  const sigText = await readFile(path.join(outDir, 'iterate-linux-x86_64.AppImage.sig'), 'utf8')
  const signature = Buffer.from(sigText.trim().replace(/^ed25519:/, ''), 'base64')

  assert.ok(stdout.includes(`ITERATE_RELEASE_PUBLIC_KEY_B64=${keys.rawPublicKeyB64}`))
  assert.equal(shaText, `${expectedSha}  iterate-linux-x86_64.AppImage\n`)
  assert.equal(verify(null, payload, keys.publicKey, signature), true)
})

test('release signer dry-run does not write companion files', async () => {
  const dir = await tempDir()
  const outDir = path.join(dir, 'companions')
  await mkdir(outDir)
  const assetPath = path.join(dir, 'iterate-windows-x86_64.msi')
  await writeFile(assetPath, 'fake windows payload\n')

  const keys = releaseKeyPair()
  const { stdout } = await execFileAsync(process.execPath, [
    scriptPath,
    '--dry-run',
    '--out-dir',
    outDir,
    assetPath,
  ], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      ITERATE_RELEASE_PRIVATE_KEY_B64: keys.privateKeyB64,
    },
  })

  assert.match(stdout, /dry-run:/)
  assert.equal(await exists(path.join(outDir, 'iterate-windows-x86_64.msi.sha256')), false)
  assert.equal(await exists(path.join(outDir, 'iterate-windows-x86_64.msi.sig')), false)
})
