# Building iterate Desktop

This repository builds the iterate desktop application and its bundled MCP
server. The official iOS, browser-extension, VS Code-extension, and hosted
control-plane sources are not part of this repository.

## Prerequisites

- Node.js 24 or a compatible current LTS release
- pnpm 10.12.1, as pinned by `packageManager`
- The stable Rust toolchain
- Platform prerequisites required by Tauri 2

Follow the official [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
for macOS, Windows, or Linux before building the native application.

## Install dependencies

```bash
pnpm install --frozen-lockfile
```

Do not add signing keys, APNs keys, tokens, cookies, or production `.env`
files to the repository. Development without optional service credentials
must remain supported.

## Development

```bash
pnpm tauri:dev
```

The frontend alone can be run with:

```bash
pnpm dev
```

## Build

```bash
pnpm build
cargo build --locked --bin iterate --bin mcp-server
```

Official signed artifacts are produced by protected release workflows. A local
build is not an official iterate distribution unless its source receipt,
signature, and artifact digest can be verified.

## Test

```bash
pnpm run test:desktop-oss-readiness
pnpm run oss:check
pnpm test
cargo fmt --check
cargo test --locked
git diff --check
```

Some release checks require platform tools such as `codesign`, `xcrun`, or
PowerShell. They must fail closed when required credentials or verification
tools are absent.

## Local runtime data

Use `ITERATE_CONFIG_DIR` to isolate development and test state. Never point a
test at a real user's active configuration directory.

```bash
ITERATE_CONFIG_DIR="$(mktemp -d)" cargo test --locked
```

## Open-source snapshot

The public source snapshot is allowlist-driven:

```bash
pnpm run oss:check
pnpm run oss:export -- /absolute/path/to/new-empty-destination
```

The exporter never overwrites an existing destination and refuses to export a
dirty source tree unless `--allow-dirty` is explicitly used for a local
preview. A publishable snapshot must have `dirty: false` in
`SOURCE_RECEIPT.json`.
