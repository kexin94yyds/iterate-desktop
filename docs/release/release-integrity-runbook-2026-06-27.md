# Release Integrity Runbook - 2026-06-27

This runbook covers local artifact integrity generation for the public updater channel. It does not publish a GitHub release or handle private key custody.

## Inputs

- Release artifacts built by the platform pipelines.
- `ITERATE_RELEASE_PRIVATE_KEY_B64`: base64 Ed25519 PKCS#8 DER private key, available only in the signing environment.
- `ITERATE_RELEASE_PUBLIC_KEY_B64`: base64 raw 32-byte Ed25519 public key, injected while building the Windows/Linux app so the updater embeds a stable trust root. Runtime env with the same name is only a development/test override.

## Build clients with the public key

Before building Windows/Linux release clients, derive or retrieve the raw public key that matches the release signing private key:

```bash
ITERATE_RELEASE_PRIVATE_KEY_B64=... pnpm run release:sign-assets -- --dry-run --print-public-key target/release/delivery/<asset>
```

Then build the Windows/Linux app with that printed `ITERATE_RELEASE_PUBLIC_KEY_B64=...` in the app build environment. The Rust updater reads this value with `option_env!` and embeds it into the binary; end users do not need to set any runtime environment variable for update verification.

## Generate companions

Dry-run first:

```bash
ITERATE_RELEASE_PRIVATE_KEY_B64=... pnpm run release:sign-assets -- --dry-run --print-public-key target/release/delivery/<asset>
```

Write companion files:

```bash
ITERATE_RELEASE_PRIVATE_KEY_B64=... pnpm run release:sign-assets -- --out-dir target/release/delivery/signed --print-public-key target/release/delivery/<asset>
```

For each asset, upload all three files to the same GitHub release:

- `<asset>`
- `<asset>.sha256`
- `<asset>.sig`

The `.sha256` file contains a named SHA-256 checksum. The `.sig` file contains an Ed25519 signature over the raw asset bytes.

## Updater contract

- macOS update installation verifies trusted release URLs, SHA-256, Apple code signature, Team ID `UM3Z9G5DNH`, bundle ID `com.kexin94yyds.iterate`, and Gatekeeper assessment.
- Windows/Linux update discovery requires a companion `.sig` asset before presenting an installer candidate.
- Windows/Linux update download verifies SHA-256 and Ed25519 signature before returning the manual-install boundary message.
- Windows/Linux automatic replacement remains fail-closed until a platform-specific installer flow is implemented and validated on that platform.

## Local verification

```bash
pnpm run test:release-signing
cargo test ui::updater -- --nocapture
pnpm run test:security-regression
```

## Publish boundary

Do not run release upload, tag mutation, or public publishing from this runbook without an explicit operator step. The repository work here only prepares and verifies local integrity artifacts.
