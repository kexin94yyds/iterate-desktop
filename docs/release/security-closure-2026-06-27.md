# Security Closure - 2026-06-27

## Closed Findings

- Browser WebSocket authentication: challenge-response HMAC is required before command handlers are reachable.
- Browser WebSocket origin policy: browser page origins and `null` are rejected during handshake; extension and native local clients remain supported.
- Pro Bridge browser endpoints: pending/context/sent/result require the browser capability token; the extension popup can configure the token without logging token material.
- Tauri updater release source: download URLs are restricted to the public release repository and require SHA-256 companion assets.
- macOS updater install path: update payloads must pass SHA-256, code signing, expected Team ID, expected bundle ID, and Gatekeeper checks.
- Windows/Linux updater boundary: update discovery requires detached signature assets and download verifies Ed25519 signatures; automatic replacement remains disabled.
- Mermaid renderer: Tauri renderer uses strict Mermaid security and disables flowchart HTML labels.

## Remaining Boundaries

- Real release publishing is not performed here; it needs an explicit operator step and release credentials.
- Real Windows/Linux auto-install is not implemented; current behavior verifies integrity and then instructs manual install.
- Browser extension token entry depends on the user copying the app-generated Browser WebSocket token from settings into the popup; no GUI automation was performed.

## Regression Entry Points

```bash
pnpm run test:browser-ws-auth
pnpm run test:pro-bridge
pnpm run test:mermaid-security
pnpm run test:release-signing
cargo test browser::websocket -- --nocapture
cargo test ui::updater -- --nocapture
pnpm run test:security-regression
```

## Release Checklist

- Upload each release asset with `<asset>.sha256` and, for Windows/Linux, `<asset>.sig`.
- Ensure the `ITERATE_RELEASE_PUBLIC_KEY_B64` GitHub secret matches the private key used by `scripts/release-sign-assets.mjs`; Windows/Linux workflows fail before build when it is missing.
- Run `pnpm run test:security-regression` before publishing.
- Treat any request to push, upload, publish, or use real key material as a separate high-risk approval step.
