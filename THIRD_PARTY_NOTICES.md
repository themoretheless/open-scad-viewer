# Third-party notices

## Own Rust geometry kernel

The application uses the repository's Rust CAD implementation in
`crates/polygon-core` through `crates/geometry-bridge`. It contains no Manifold,
OpenCascade, CGAL or other external CAD kernel. The repository license is MIT.
The core geometry libraries use optional `wgpu` in
`crates/gpu-compute` for native Metal/Vulkan/DX12 compute (feature `gpu`) and
optional `cudarc` for the CUDA driver API (feature `cuda`). WASM builds
leave both features off. The document adapter additionally uses the static SVG
libraries listed below. A separate compression bootstrap uses the Brotli crates
listed below; they are not geometry kernels. Document values, binary transport,
direct WASM bindings and runtime DEFLATE decoding are repository-owned.
The UI, MCP integration and build
tooling still use the packages listed below.
Historical Manifold qualification fixtures and manifests describe old releases;
they are not dependencies or attestations of the current runtime.

## Static SVG parsing, text and rendering

- Libraries: `usvg` and `resvg` 0.47.0, with the exact transitive versions in `crates/Cargo.lock`
- Source: <https://github.com/linebender/resvg>
- Local source and compatibility patches: `crates/vendor/usvg`,
  `crates/vendor/resvg`; provenance and changes in `crates/vendor/README.md`
- License: MIT OR Apache-2.0; dependencies carry their own notices
- Complete shipped notices: [public/third-party/svg.txt](public/third-party/svg.txt)

The browser/Node WASM adapter resolves SVG structure and text with `usvg`,
expands vector strokes and clips using the planar kernel, and uses `resvg`
only for explicitly selected rendered silhouettes. System font lookup and font
memory mapping are disabled; external file and URL resolvers are disabled.
These libraries parse/render SVG documents and do not replace the CAD kernel.

### Noto Sans

- Asset: `crates/geometry-bridge/assets/NotoSans-Regular.ttf`
- Source: <https://github.com/notofonts/noto-fonts/blob/main/hinted/ttf/NotoSans/NotoSans-Regular.ttf>
- Copyright 2018 The Noto Project Authors
- License: SIL Open Font License 1.1
- Complete shipped notice: [public/third-party/noto-sans-OFL.txt](public/third-party/noto-sans-OFL.txt)

The unmodified font is embedded for deterministic SVG text across browser,
worker and Node environments. Users may supply additional fonts explicitly.

## Synchronous Brotli compression bootstrap

- Crates: `brotli-decompressor` 5.0.3, `alloc-no-stdlib` 2.0.4 and `alloc-stdlib` 0.2.4
- Source: <https://github.com/dropbox/rust-brotli-decompressor> and <https://github.com/dropbox/rust-alloc-no-stdlib>
- Distributed license: BSD-3-Clause; `brotli-decompressor` package metadata additionally lists MIT.
- Complete shipped notice: [public/third-party/brotli.txt](public/third-party/brotli.txt)

`crates/wasm-brotli` uses the default safe decoder, without its `unsafe` or
`ffi-api` features. This small standalone WASM module decompresses generated
geometry code synchronously. It has no geometry dependencies or external
imports. Its compressed bytes are separate from the geometry artifact; the
ordinary repository-owned DEFLATE decoder loads this bootstrap. The complete
license notice is copied into the browser distribution by Vite's public assets.

## Linear truss numerical solver

- Crate: `nalgebra` 0.35.0, pinned in `crates/Cargo.lock`
- Source: <https://github.com/dimforge/nalgebra>
- License: Apache-2.0; transitive dependencies carry their own licenses
- Complete shipped notices: [public/third-party/truss.txt](public/third-party/truss.txt)

Used by the bounded axial-bar solver in `crates/mechanics-core`. Default
features are disabled; only `std` is enabled. The geometry WASM exposes this
module through the strict `truss_solve` and `truss_solve_wrenches` operations
and a TypeScript adapter.
It is not yet connected to workbench controls. It does not replace the CAD
kernel or certify material properties or structural safety. The notice bundle
contains the pinned normal wasm32 dependencies' complete license files and is
copied into the browser distribution by Vite's public assets.

## wgpu

- Crate: `wgpu`
- Version: `30`
- Source: <https://github.com/gfx-rs/wgpu>
- License: MIT OR Apache-2.0

Used only by `crates/gpu-compute` when the optional `gpu` feature is enabled
on native hosts (Metal on macOS, Vulkan on Linux/Windows, DX12 on Windows).
Browser WebGPU does not link this crate.

## cudarc

- Crate: `cudarc` (with its `libloading` dependency)
- Version: `0.19`
- Source: <https://github.com/coreylowman/cudarc>
- License: MIT OR Apache-2.0

Used only by `crates/gpu-compute` when the optional `cuda` feature is enabled
on native hosts. Only the CUDA driver-API bindings are compiled
(`driver`, `dynamic-loading`); the NVIDIA driver library is loaded at run time
and nothing from the CUDA toolkit is linked or redistributed. The kernels are
repository-owned CUDA C (`crates/sdf-core/src/sdf_grid.cu`) committed as PTX.
Browser and WASM builds do not link this crate.

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
