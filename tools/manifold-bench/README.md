# Manifold comparison benches

This tree is **not** the product.

`manifold-3d` lives only here, in its own lockfile, so comparison timings can
still be taken without putting the foreign kernel back into
`open-scad-viewer` dependencies or `src/`.

The four fixture IDs match the product CPU suite. Geometry is built through
the Manifold API directly, not through the product OpenSCAD parser.

## Run

```sh
cd tools/manifold-bench
npm install
npm run bench:cpu -- --out tmp/cpu-baseline.json
npm run bench:memory -- tmp/memory.json
```

`--quick` shortens the CPU pass. Do not import this package from `src/`.

## Differential qualification

```sh
npm ci
npm run test:differential
```

The differential runner reads the shared `differential-corpus.json`, evaluates every union/intersection/difference in native Rust and `manifold-3d`, and compares volume, surface area, and whole-mesh genus with a relative tolerance of `1e-7`. Every mismatch is a test failure with the case, operation, metric, both values, and tolerance; there is no snapshot-update mode. The Rust example is a sidecar adapter only and is not linked into the product kernel.
