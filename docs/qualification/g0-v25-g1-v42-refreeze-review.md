# G0 v25 / G1 v42 no-claim re-freeze

This append-only amendment preserves G0 v24, G1 v41, and all earlier artifacts byte-for-byte. V41 cannot admit further evidence because its frozen bindings do not match the bytes a clean checkout produces.

The drift is reviewed:

- **Machine-specific kernel bytes.** rustc embeds source paths in panic-location strings, and registry crates live under `$CARGO_HOME`, so the packed geometry kernel (and every G0 fingerprint and own-Rust evidence bound to it) carried the recording machine's absolute checkout and cargo-home paths. A CI runner rebuilding the same sources at the same pinned toolchain produced different kernel bytes, and the engine-manifest, G0 fingerprint and own-Rust evidence checks failed on every checkout but the recording one. The build scripts (`build-geometry-kernels.mjs`, `build-language-kernel.mjs`, `build-photogrammetry.mjs`, `build-wasm-brotli.mjs`) now pass `--remap-path-prefix` for cargo home (`/cargo`) and the checkout root (`/src`) through `scripts/reproducible-cargo.mjs`, extending `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS` when those are set instead of being silently overridden by them. Two checkouts at different paths with different cargo homes rebuild byte-identical kernels, and no packed kernel contains a home, runner or cargo-home path. Only embedded path strings change; the language, photogrammetry, wasm-brotli and harfbuzz packs are unchanged.
- **`package.json`.** `8207512` added the `qual:shader-library` script after the v24 freeze.
- **`crates/Cargo.lock` and compute crates.** `253d7700` added the `tensor-core`, `compute-cuda` and `compute-mlx` workspace members with their lockfile entries and extended `compute-core`. None of these is in the geometry kernel dependency graph (`cargo tree -p geometry-wasm --target wasm32-unknown-unknown`).
- **Renderer.** This change set also carries renderer fixes (texture-resource lifecycle, environment-map orientation and sRGB decoding, shadow-caster depth bias, bind-group completeness and frame validation scopes) in raster-core WGSL, the generated shader sources and renderer services. None of these is part of the own-Rust geometry kernel source bundle.

The own-Rust geometry kernel sources, `THIRD_PARTY_NOTICES.md` and `package-lock.json` are byte-identical to the own-rust-cad-v16 evidence, so kernel semantics are unchanged and the own-Rust oracle stays at v3.

V42 recomputes every artifact and canonical bundle digest from current bytes, re-binds the exact GitHub Actions workflow, evidence-producing harness, fragment selectors (now selecting v42), and the unchanged hosted-runner identity freeze (`g1-github-actions-v34.json`, retained as the active environment freeze). Own-Rust evidence advances to `own-rust-cad-v17.json`, binding the path-independent kernel bytes and the current Rust lockfile.

The matrix remains 4740 planned work units. V42 starts with zero completed runs and zero completed units, imports no prior result, makes no qualification claim, and authorizes no production cutover.
