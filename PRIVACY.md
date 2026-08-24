# Privacy

iterate Desktop is local-first software. Open sourcing the desktop code does
not publish users' conversations, configuration, credentials, device records,
or logs.

## Public source versus private data

This repository contains source code, tests, configuration schemas, example
values, and public verification material. It must not contain:

- real conversations, prompts, Memory, or knowledge-base contents;
- API tokens, cookies, passwords, private keys, APNs keys, or signing keys;
- real device registrations, QR payloads, paired-device tokens, or production
  logs;
- private iOS, browser-extension, VS Code-extension, or hosted-service source.

## Local storage

Desktop configuration and Bridge state are resolved through OS-standard
configuration directories. On common platforms these map to locations such
as macOS Application Support, Windows AppData, and the Linux XDG configuration
directory. Credentials that require secure storage should use Keychain,
Credential Manager, or Secret Service rather than plain repository files.

`ITERATE_CONFIG_DIR` can redirect configuration and Bridge state to an
isolated directory for development and tests.

Some optional features can read local `.cunzhi` or `.cunzhi-knowledge` data at
the user's request. Those directories are runtime inputs, not repository
content, and must never be copied into an open-source snapshot.

The public desktop source includes no bundled notification sounds. If a user
chooses a local custom sound, iterate validates it and copies it into the
OS-standard application-data directory under a fixed managed filename. The
original filename and path are not retained in configuration, returned through
Bridge configuration APIs, logged, uploaded, or added to a source export.
Custom notification sounds are never downloaded from a URL. A missing or
invalid managed sound falls back to silence without interrupting task flow.

## Network connections

The desktop application can make network connections when the corresponding
feature is enabled or invoked, including:

- checking the public release repository for updates;
- connecting to a user-configured public Bridge, Cloudflare route, or relay;
- sending APNs or Web Push notifications when the user has configured the
  required provider credentials;
- using optional integrations such as Telegram or an official closed-source
  client.

Public endpoints, bundle identifiers, and release public keys are not secrets.
Authentication tokens and private keys are secrets and must be injected at
runtime or by protected CI environments.

## Telemetry and crash reporting

The open-source desktop base does not require a telemetry or crash-reporting
credential to build or run. Any future telemetry integration must be disabled
by default without explicit configuration, documented here, bounded to the
minimum necessary fields, and accompanied by a user-visible control.

## Diagnostics

Logs and diagnostic bundles can contain project paths, request identifiers,
and operational metadata. Diagnostic export must redact secrets and should be
initiated by the user. Do not attach raw private logs, valid QR codes, tokens,
or private conversations to public issues.

## Official services

The official iOS app, extensions, and hosted services are separate products
and are not made open source by this repository's MIT license. Their data
handling must be documented by the privacy notice shipped with those
products. The desktop README must distinguish local functionality from
features that require an official service.
