# Security Policy

## Supported versions

Security fixes are applied to the latest public desktop release and the
current default branch. Older releases may be asked to upgrade before a fix is
backported. Pre-release builds are supported on a best-effort basis.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability or include a working
secret, private conversation, valid QR code, or exploit against a real user.

Use GitHub's private vulnerability reporting for this repository. Include:

- affected commit or release;
- platform and architecture;
- minimal reproduction using synthetic data;
- impact and required privileges;
- suggested remediation, if known.

If private vulnerability reporting is not yet enabled, contact the repository
owner privately through GitHub and wait for a secure reporting channel before
sending sensitive details.

## High-priority areas

- Bridge authentication, authorization scopes, WebSocket origin checks, and
  replay protection;
- MCP tool boundaries and remote-input capability enforcement;
- local file path validation, symlink escape, and secret redaction;
- updater signatures, checksums, code signing, and release provenance;
- CI token permissions, untrusted pull requests, and dependency supply chain;
- APNs, Web Push, relay, browser, and other optional credential handling.

Web Push subscriptions are restricted to HTTPS 443 endpoints operated by the
supported browser push services (FCM, Mozilla Push, Apple Web Push, and WNS).
The sender does not follow redirects, rejects IP literals and unknown hosts,
and bounds subscription count and field lengths so paired clients cannot turn
the desktop process into a generic network egress proxy.

## Current dependency audit exception

The locked macOS and Windows desktop dependency graphs have no known Rust
security advisory after the August 2026 dependency remediation. The Linux
target still resolves `glib 0.18.5` through Tauri's GTK3/WebKitGTK stack and is
therefore reported by GHSA-wrw7-89jp-8q8g (also RUSTSEC-2024-0429). The fixed
`glib 0.20` line cannot be substituted independently without a compatible
upstream Tauri/WebKitGTK migration. The affected `VariantStrIter` API is not
used directly by iterate, but Linux releases must continue to track this
upstream issue and treat it as an explicit exception rather than suppressing
the advisory globally.

Other RustSec entries currently reported for the locked graph are
unmaintained-package notices in the target-specific GTK3 and build-time macro
chains. They are maintenance risk, not evidence that those packages are
exploitable in iterate. Every release should rescan both lock files and must
not introduce a new high or critical advisory.

## Disclosure

Please allow maintainers time to reproduce, patch, and prepare an advisory
before public disclosure. The project will credit reporters who request credit
and will not include private exploit data or credentials in an advisory.
