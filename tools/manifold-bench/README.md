# Manifold comparison benches

This tree is **not** the product.

`manifold-3d` lives only here, in its own lockfile, so comparison timings can
still be taken without putting the foreign kernel back into
`open-scad-viewer` dependencies or `src/`.

The four fixture IDs match the product CPU suite. Geometry is built through
the Manifold API directly, not through the product OpenSCAD parser.

See the [benchmarking guide](../../docs/manifold-benchmarking.md) for native
rbench commands, median normalization, CI regression policy, and the limits of
the external CPU and memory sidecars.

## Run

```sh
cd tools/manifold-bench
npm install
npm run bench:cpu -- --out tmp/cpu-baseline.json
npm run bench:memory -- tmp/memory.json
```

`--quick` shortens the CPU pass. Do not import this package from `src/`.

## Native kernel regression gate

```sh
npm run test:rbench
npm run bench:kernel:check -- --baseline rbench-baseline.macos-aarch64.json --max-regression 0.25
```

The runner executes the release `manifold-core` rbench suite and compares wall-clock medians normalized to nanoseconds per operation. The default budget is 25%; exceeding it exits nonzero and still writes `tmp/rbench-comparison.json`. Missing cases, changed contracts, incomplete samples, and OS/architecture mismatches are errors. A stored baseline is meaningful only for repeated measurements on the same machine. Record a replacement only after reviewing an intentional performance change:

```sh
npm run bench:kernel:record -- --baseline rbench-baseline.macos-aarch64.json
```

CI instead builds and measures the base revision and current revision on the same macOS runner, then applies the same 25% gate. `compare --baseline BASE/run.json --current CURRENT/run.json` compares existing reports without rerunning either revision.

## Differential qualification

```sh
npm ci
npm run test:differential
```

The differential runner reads the shared `differential-corpus.json`, evaluates every union/intersection/difference in native Rust and `manifold-3d`, and compares volume, surface area, and whole-mesh genus with a relative tolerance of `1e-7`. Every mismatch is a test failure with the case, operation, metric, both values, and tolerance; there is no snapshot-update mode. The Rust example is a sidecar adapter only and is not linked into the product kernel.

## Native/WASM and precision matrix

From the repository root, after `npm run build:geometry`:

```sh
node --import tsx tools/manifold-bench/environment-matrix.mts
```

The runner evaluates checks, metrics, full repair, and booleans on the shared `environment-corpus.json` in native Rust and the packaged WASM, using both f64 and f32 inputs. Native/WASM results at equal precision use a strict `1e-9` relative tolerance; f64/f32 comparisons use `2e-5`. Triangle indices are canonically rotated to start at the smallest vertex index without sorting vertices or changing winding. Geometric, topological, and structural mismatches fail the run. Only the diagnostic `kernel.work` counter may differ between precisions without failing; its values and deltas remain visible in `diagnosticDifferences`. The JSON report is written to `tmp/environment-matrix.json`, and CI uploads it for each Node version.
