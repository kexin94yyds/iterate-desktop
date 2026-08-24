# Contributing to iterate Desktop

Thank you for improving iterate Desktop.

## Scope

This repository accepts changes to the Tauri/Vue desktop application, bundled
MCP server, desktop Bridge, public protocol, cross-platform packaging, tests,
and documentation.

The official iOS app, browser extension, VS Code extension, hosted control
plane, production operations, payment, and marketing systems are maintained
privately and are out of scope here.

## Before opening a pull request

1. Read [BUILDING.md](BUILDING.md), [PRIVACY.md](PRIVACY.md), and
   [SECURITY.md](SECURITY.md).
2. Keep the change focused and add a regression test for changed behavior.
3. Use synthetic paths, device identifiers, conversations, and credentials.
4. Do not add generated bundles, installers, archives, VSIX files, or local
   runtime state.
5. Run:

```bash
pnpm run test:desktop-oss-readiness
pnpm run oss:check
pnpm test
cargo fmt --check
cargo test --locked
git diff --check
```

## Pull requests

Describe the user-visible behavior, security/privacy impact, validation, and
rollback. A green test is evidence, not a substitute for an exact behavior
description.

Changes to authentication, schemas, release workflows, updater trust roots,
or filesystem boundaries require additional security review.

## Security reports

Follow [SECURITY.md](SECURITY.md). Never use a public issue to report a live
credential or exploit.
