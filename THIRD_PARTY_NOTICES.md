# Third-party notices

## Own Rust geometry kernel

The application uses the repository's Rust CAD implementation in
`crates/polygon-kernel` through `crates/geometry-bridge`. It contains no Manifold,
OpenCascade, CGAL or other external CAD kernel. The repository license is MIT.
Rust serialization and WebAssembly bindings still use serde/serde_json and
wasm-bindgen, recorded in `crates/Cargo.lock`; host transport uses fflate.
This is not a claim that the UI, transport or build tooling has zero dependencies.
Historical Manifold qualification fixtures and manifests describe old releases;
they are not dependencies or attestations of the current runtime.

## Model Context Protocol TypeScript SDK

- Package: `@modelcontextprotocol/server`
- Version: `2.0.0`
- Project: <https://modelcontextprotocol.io>
- Source: <https://github.com/modelcontextprotocol/typescript-sdk>
- Declared package license: MIT
- Distributed license text: <https://unpkg.com/@modelcontextprotocol/server@2.0.0/LICENSE>

The distributed `LICENSE` records the project's transition from MIT to
Apache-2.0: new and relicensed code and specification contributions are
Apache-2.0, remaining legacy contributions are MIT, and documentation other
than specifications is CC-BY-4.0. Release packaging must retain that complete
license file rather than relying only on the package metadata's MIT field.

## DuckDB Node API

- Package: `@duckdb/node-api`
- Version: `1.5.5-r.2`
- Source: <https://github.com/duckdb/duckdb-node-neo>
- License: MIT
- License text: <https://unpkg.com/@duckdb/node-api@1.5.5-r.2/LICENSE>

The installed package carries its `LICENSE` file. The API loads DuckDB native
bindings selected for the host platform; release packaging must retain the
license files supplied with the applicable DuckDB packages.

## tsx

- Package: `tsx`
- Version: `4.23.1`
- Project: <https://tsx.hirok.io>
- Source: <https://github.com/privatenumber/tsx>
- License: MIT
- License text: <https://unpkg.com/tsx@4.23.1/LICENSE>

The installed package carries its `LICENSE` file, which must remain with any
redistributed runtime copy.

## Zod

- Package: `zod`
- Version: `4.4.3`
- Project: <https://zod.dev>
- Source: <https://github.com/colinhacks/zod>
- License: MIT
- License text: <https://unpkg.com/zod@4.4.3/LICENSE>

The installed package carries its `LICENSE` file, which must remain with any
redistributed runtime copy.

## HarfBuzz.js

- Package: `harfbuzzjs`
- Version: `0.10.3`
- Source: <https://github.com/harfbuzz/harfbuzzjs/tree/0.10.3>
- License: MIT for the project, with Apache-licensed Zephyr libc and Emscripten
  implementation files as identified by the distributed license
- License text: <https://unpkg.com/harfbuzzjs@0.10.3/LICENSE>

The independent `text()` implementation uses the packaged HarfBuzz WebAssembly
module for OpenType shaping and glyph outlines. Release packaging must retain
the complete distributed `LICENSE` file alongside the JavaScript and WebAssembly
artifacts.

No official OpenSCAD executable, WebAssembly module, parser or runtime code is
included in this application.
