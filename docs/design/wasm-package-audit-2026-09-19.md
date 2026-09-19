# WASM package audit: convergence candidate rejected

The geometry kernel accounts for roughly half of delivery bytes. Before
changing Rust optimization profiles, we checked two low-level possibilities:
removing metadata and repeating the existing Binaryen size passes.

The current geometry/language modules have no `name`, `producers`,
`target_features`, `sourceMappingURL` or `dylink.0` custom sections. Build scripts
already request Cargo symbol stripping. No metadata removal was applied.

## Isolated experiment

Binaryen `wasm-opt version 116 (version_116)` was run against a separate output:

```sh
wasm-opt -Oz --all-features --converge \
  src/generated/geometry-kernels/kernel_bg.wasm \
  -o tmp/performance/geometry-converged.wasm

npm run audit:wasm-package -- \
  src/generated/geometry-kernels/kernel_bg.wasm \
  tmp/performance/geometry-converged.wasm \
  --out tmp/performance/geometry-converged-package.json
```

The input is already optimized by the production `-Oz` pass. Convergence applies
further standard passes until size stops decreasing; no unsafe floating-point
or trap assumptions were added. The process was still running after 3m37s
(observed via `ps`), before completing. An exact total build-time benchmark was
not taken.

| Representation | Current | Converged | Saved |
| --- | ---: | ---: | ---: |
| Raw WASM bytes | 7,530,983 | 7,525,629 | 5,354 |
| Brotli bytes | 2,238,337 | 2,235,650 | 2,687 |
| Base85 characters | 2,797,939 | 2,794,579 | 3,360 |

Compression uses the production quality 11 / window 24, four-byte original
length header, and existing Base85 encoder. The computed baseline literal hash
matches `src/generated/geometry-kernels/bytes.ts` exactly. Both packages unpack
to their own original WASM bytes, have no imports, and expose the same export
names/kinds. This is not a proof of equivalent ABI types or geometry behavior.

Current WASM SHA-256:
`076ded32f962ce32b043c20e7fd863b8cac15a0986bec2d445ba4e9bca5691d3`.
Candidate SHA-256:
`289e74791ad1c6b77ef54e55ec0972fc044a6b98ad369a983129332c60c55efd`.

## Decision

Do not adopt this candidate. Saving 3,360 delivered literal bytes (about 0.12%
of the kernel package) does not justify adding minutes to every geometry build
and qualifying a changed executable. No production WASM, packed literal,
optimization profile or delivery budget was modified. The existing delivery
size remains 5,782,657 bytes, still above the 5,600,000-byte aggregate budget.

The candidate remains an experiment under `tmp/performance`; full geometry
tests and runtime-speed measurements were deliberately not run for a variant
rejected at the delivery-size stage. It must not be promoted on the strength of
this packaging audit alone.

## Reusable checks

`scripts/audit-wasm-package.mjs` accepts one to four modules and records raw,
Brotli and Base85 sizes/hashes, exact unpacking, export names/kinds, and deltas
against the first module. It enforces the production raw/compressed limits and
refuses modules with external imports. It reports zlib/Brotli/Node versions and
does not claim runtime equivalence from matching exports.

`npm run test:wasm-package` covers deterministic roundtrips for Buffer and
Uint8Array inputs, malformed modules, the raw-size bound and external-import
refusal. Three new tests pass; combined with the existing bundle-audit tests,
five pass. `git diff --check` passes. Raw experiment evidence is in
`tmp/performance/geometry-converged-package.json`.
