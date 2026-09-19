# Owned direct-document normalization

`directDocumentValidation` starts with `JSON.parse(text)`: the resulting tree
already belongs exclusively to the validator. Defaults and regenerated analytic
sketch points are also newly allocated. Its final JSON stringify/parse clone
therefore copied an entire owned document without adding isolation.

The validator now normalizes numbers in place and returns that owned tree.
This preserves JSON-clone behavior for negative zero and non-finite numbers
in unchecked metadata. Geometry validation still runs first, so invalid
geometry is refused rather than normalized into an apparently valid result.
Defensive copies in history getters and other caller-owned boundaries remain.

The traversal falls back to the original JSON clone past depth 64. This is a
fast-path threshold, not a new admission limit: moderately deep input remains
accepted and excessively deep input still encounters the serializer's refusal.
The helper is private and must not be used for arbitrary caller-owned objects,
getters, functions, cycles or custom prototypes.

## Measurement

Run `node --import tsx benchmarks/direct-validation.mts [new-output-json]`.
Apple M4 Max, macOS arm64, Node v22.23.2; three warmups and nine samples.
The planetary-spinner fixture contains 20 bodies, 20 unique B-reps and
19,852,257 document characters. Fixture construction and equality checks are
outside timing; no builds or tests ran alongside measurements.

| Warm synchronous validation | Median |
| --- | ---: |
| Before | 316.67 ms |
| Owned normalization | 174.57 ms |
| Independent repeat | 176.34 ms |

The reduction is 44.3-44.9%. All runs have identical source and normalized
document SHA-256. This measures parsing, validation, normalization and owned
result creation, not whole-model construction or browser paint. No heap/RSS
savings were measured. The separate first-validation diagnostic includes
reference serialization and is not comparable to the warm-sample interval.

Evidence: `tmp/performance/owned-json-before.json`, `owned-json-after.json`,
`owned-json-after-repeat.json`. Geometry WASM is unchanged at 7,530,983 bytes,
SHA-256 `076ded32f962ce32b043c20e7fd863b8cac15a0986bec2d445ba4e9bca5691d3`.

## Compatibility tests

Tests cover numeric edge cases, special property names, 128 deterministic
mixed JSON trees, independent parse ownership, regenerated analytic samples,
depth fallback, excessive-depth refusal and invalid geometry overflow.
The generated-tree oracle uses Node's `deepStrictEqual`: Vitest's strict type
tester compares `.constructor` by identity, which is inappropriate when that
name is an ordinary JSON property containing another object.

Full regression: 3,228 tests passed and nine failed across 334 files (135.53 s).
All failures are the existing engine-manifest and G0/G1 historical fingerprint
checks. No historical qualification evidence was rewritten. Log:
`tmp/performance/owned-json-full-tests-final.log`.

UI and MCP typechecks, strict standalone test/benchmark typechecking and the
Vite production build passed. Delivery size is 5,783,000 bytes, 343 bytes above
the preceding build and still over the unchanged 5,600,000-byte gate. This is
not a release qualification or a solution to the remaining bundle-size debt.

## Production browser check

Two sequential Chrome 156 runs used the existing `exact-solid-browser.mjs`
runner against the normal production build. Source and geometry hashes for
all three fixtures match `modelgraph-compiler-browser-repeat/report.json`.
Source build, group replacement, cancellation preserving scene/editor,
all three mechanical generators and invalid-input recovery passed; neither
run recorded a page error.

The 20-body spinner took 7,496.5 / 7,557.5 ms; maximum callback gaps were
378.9 / 389.4 ms. The preceding two runs were 7,613.4 / 7,547.4 ms with gaps
390.5 / 247.8 ms. These sparse end-to-end samples do not establish a browser
speedup or responsiveness improvement. Cold B-rep work and other synchronous
phases remain; the 44% claim is limited to warm synchronous validation.

Reports and desktop/mobile captures: `tmp/performance/owned-json-browser/`
and `tmp/performance/owned-json-browser-repeat/`. Screenshots were inspected;
the existing mobile toolbar/editor clipping remains outside this change.
