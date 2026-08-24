# Third-party notices

iterate Desktop depends on third-party open-source software. Each dependency
remains subject to its own license.

## Upstream project

- `imhuso/cunzhi` — MIT License. The original notice is preserved in
  [LICENSE](LICENSE).

## Core frameworks

- Tauri — Apache-2.0 or MIT
- Vue — MIT
- Tokio — MIT
- Axum — MIT
- Naive UI — MIT
- Mermaid — MIT
- markdown-it — MIT

This summary is not a substitute for a generated dependency inventory. A
publishable release must generate an SBOM and complete license report from the
locked Cargo and pnpm dependency graphs.

## Embedded audio assets

The private source history contains notification sounds obtained from Pixabay
and Mixkit. Their platform licenses allow use inside a larger work but restrict
redistribution of the original files as standalone source assets. Those files
are explicitly excluded from the public desktop source snapshot and are not
licensed under this repository's MIT license.

The public desktop build starts without a bundled notification sound. Users may
import their own local sound at runtime; user-provided files are local data and
are never part of the source distribution.
