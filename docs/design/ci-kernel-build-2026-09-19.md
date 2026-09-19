# Avoid repeated kernel packaging in CI

The check job explicitly runs `npm run build:geometry`, then used to run
`npm run check`. The latter expands into typecheck, test and build commands;
their `pretypecheck`, `pretest` and `prebuild` hooks each invoke the full kernel
build. A successful check job therefore requested four kernel builds.
The native-smoke job similarly built explicitly and again through `npm test`.

Cargo's incremental cache does not avoid the subsequent work:
`build-geometry-kernels.mjs` and `build-language-kernel.mjs` unconditionally run
Binaryen and Brotli quality 11. Binaryen operates on the Cargo artifact in place,
so repeated invocation should not be assumed to be a source-identity-neutral
no-op either. This change does not redesign artifact caching or claim to resolve
the existing historical fingerprint failures.

The workflow now invokes the local installed CLI entrypoints after its explicit
kernel build: Vue typecheck, MCP TypeScript check, the complete Vitest suite,
Vite build, and `verify-dist`. Native smoke invokes the same two Vitest files
directly. Node/OS matrices, Rust tests, the official OpenSCAD job and all public
npm lifecycle hooks remain unchanged. No validation gate is removed or relaxed.

Local verification on the existing generated artifacts:

- Both direct typecheck commands passed.
- Direct native smoke passed all 38 tests in two files. Log:
  `tmp/performance/ci-prebuilt-smoke.log`.
- The workflow parsed successfully with a YAML parser; inspected command order
  preserves one explicit kernel build before each consuming job.
- The same direct Vite entrypoint was used for the production packaging check;
  `verify-dist` still reports the outstanding total-byte budget failure.

The full test suite was not rerun for this workflow-only change. Previous full
regression results and their nine fingerprint failures remain applicable only
to that recorded run, not proof of a green CI run here. The active GitHub run
35467138467 uses the preceding committed workflow. No wall-clock CI saving is
claimed before the updated workflow executes; four-to-one/two-to-one describes
removed command invocations on successful paths, not a measured speedup ratio.

## Keep Cargo artifacts immutable

The optimizer now writes to an isolated temporary file, validates and returns
its bytes, and removes the temporary directory on success or failure. Geometry
and language build scripts publish those returned bytes to their generated
directories. Flags are unchanged. Cargo's own artifact is no longer modified.
Optimizer failures throw rather than terminating the host process internally.

`npm run test:wasm-optimize` uses the real pinned Binaryen package: repeated
optimization preserves the original input, returns identical bytes, and executes
an exported function returning 42. Invalid input is preserved and failure is
catchable. Both tests passed. These small-module checks are not a full clean
geometry/language build or cross-platform reproducibility proof.

An existing cache produced by the old in-place optimizer may already contain
optimized Cargo outputs. This patch does not repair that state. Fresh compilation
and two complete packaging runs must establish kernel fingerprints before any
historical qualification evidence is updated. No generated artifacts or frozen
fingerprints were changed in this follow-up.

## Clean geometry verification

A fresh offline release compilation used
`--target-dir tmp/performance/clean-kernel-target`, leaving the existing Cargo
cache and generated artifacts untouched. Command profile: locked,
`profile.release.strip="symbols"`, `wasm32-unknown-unknown`,
`crates/geometry-wasm/Cargo.toml`. It completed successfully in 56.38 seconds.

Two consecutive calls to the new optimizer consumed the same fresh input:

- Input: 8,546,691 bytes; SHA-256
  `049200006f9e917126a5da9183f8ccfff227e1e12c6e7a7e8a7c2d0b6930f52d`.
- Each output: 7,670,726 bytes; SHA-256
  `f37b48906375cad0155d2696bb529505b528217798048f8440011ef493bd0a8c`.
- Byte equality of the input was asserted after each call. Output hashes match
  one another and the existing generated geometry WASM.
- Local optimizer wall times were 297.1 and 300.6 seconds. These demonstrate the
  material repeated work, not a controlled CI speedup comparison.

Evidence: `tmp/performance/clean-kernel-build.log`,
`tmp/performance/clean-kernel-repeat.log` and
`tmp/performance/clean-kernel-repeat.json`. No production artifact was replaced.
The language-kernel follow-up is recorded below.

WASM-consuming CI jobs now use `rust-wasm-immutable-v2-...` cache keys without a
fallback to the old potentially modified artifacts. The Rust-only job retains
its existing key. The check matrix runs the optimizer regression tests before
building kernels. YAML parsing and cache-key checks passed locally.

## Current CI evidence

Node 22 job 105961446609 in run 35467138467 completed with 3,220 passed,
19 failed and five skipped tests. Its downloaded log is
`tmp/performance/ci-node22-failure.log`. Beyond historical fingerprint/evidence
failures, it reports provider readiness exceeding 250 ms, a 5-second test
timeout and downstream worker/HTTP assertions. Do not describe all CI failures
as the nine historical local failures. Investigate cold readiness and test
resource contention separately; no timeout or qualification assertion was
relaxed here. The official OpenSCAD MCP job passed; the overall run was still
in progress when inspected.

## Language kernel follow-up

Fresh language crate compilation into the isolated target directory completed
in 15.04 seconds. The two optimizer calls preserved the input and produced the
same output (27.5 and 27.3 seconds; not a performance comparison):

- Raw: 1,497,147 bytes,
  `377ce87735c95ae748f2303dff7ee05487ecebcebf440c10331751693efffded`.
- Optimized: 1,341,106 bytes,
  `e8960fc3180fd9203d4e0e1b9137349b90ad8301d9f831c9f492074f3dfa6483`.
- Existing generated language kernel differs:
  `357302c023784bb574554e0167010a4370d5c11056e02fd9ef4fb311bc3f58bd`.

Unlike geometry, byte identity with the generated language artifact is **not**
established. Do not attribute the difference to a specific cause without checking
the original build's source/toolchain/optimizer history. The generated artifact
was not replaced, and its frozen evidence was not refreshed.

`benchmarks/language-artifact-parity.mts` accepts two WASM paths and compares their
real MGV1 ABI responses, freeing request/response allocations. The 14 checked-in
ModelGraph examples exercise operations 1 and 4; five OpenSCAD sources under two
profiles exercise parsing/evaluation, assertions and syntax refusals (10 and 11).
All 48 cases matched, with successful results required for every operation and
refusal coverage required overall. Each case records input/output hashes.
This is bounded behavioral evidence, not equivalence of all language operations
or full candidate-kernel qualification.

Evidence: `tmp/performance/clean-language-build.log`,
`tmp/performance/clean-language-repeat.json`, and
`tmp/performance/clean-language-parity.json`. Reproduce parity with:

```sh
node --import tsx benchmarks/language-artifact-parity.mts \
  src/generated/language-kernel/kernel_bg.wasm \
  tmp/performance/clean-language-optimized-0.wasm
```

CI run 35467138467 was later cancelled after a newer upstream merge started run
35468484552 at `d0a7050ba0524286e2d1248d36d78f02949bb7f9`. Results from that newer
commit must not be attributed to this local change set.

### Chained-pass experiment

Running the same optimizer on the first optimized language output, rather than
the raw Cargo input, produced 1,340,693 bytes (413 fewer), SHA-256
`1521a6aec81f7ab990d332a2ff96a6610d950ab41bfe5e228987aa4a2dd266e3`.
This differs from both the single-pass output and the existing generated
artifact. Evidence: `tmp/performance/language-second-pass.json`.

Thus the hypothesis that the generated artifact is exactly a second pass over
this fresh build is rejected. The narrower finding is confirmed: the pinned
Binaryen recipe is not idempotent for this kernel, so optimizing Cargo's cached
artifact in place makes build invocation count affect output identity. Do not
infer the generated artifact's source/toolchain history from this experiment.
The experiment used only scratch artifacts and preserved its input.

## Content-keyed optimizer cache

`wasm-opt-cache.mjs` now caches validated optimized bytes beside the input in
`.osv-wasm-opt-cache`. The key includes raw input SHA-256, the complete pinned
Binaryen executable SHA-256 (which embeds its WASM), flags, Node version,
platform, architecture and a cache format version. It does not use timestamps.
The optimizer consumes a private snapshot of the hashed input, so a concurrent
change to the Cargo file cannot silently alter the cached computation.

Entries are single JSON envelopes published by a same-directory atomic rename.
Hits require matching key/output SHA-256 and successful WASM validation.
Malformed, missing, oversized or mismatched entries are misses; failures of the
optimizer are not cached. Cache write failures warn and return the successfully
optimized bytes. The cache is local derived data, not an authenticity boundary
against an attacker who can rewrite both content and metadata. Each entry is
limited to 32 MiB; total cache size is not capped or automatically pruned.
Remove this derived cache directory to force independent optimization.

A full language kernel miss plus two hits measured 14,521.1 ms, 10.5 ms and
9.2 ms, including input/tool hashing and cache validation. All outputs exactly
matched the independently verified single-pass artifact and the Cargo input
remained unchanged. Evidence: `tmp/performance/language-optimizer-cache.json`
and `.log`. This is an optimizer-stage improvement, not an end-to-end build
speedup: Cargo, Brotli packing and generated-file publication still run.
The earlier 27-second optimizer runs are not a controlled baseline for this
measurement; the same-run miss is the relevant comparison.

Eight Node tests cover independent real Binaryen runs plus a hit, unchanged input,
failure propagation, invalidation, damaged/mismatched entries (including invalid
WASM with a matching content hash), and unwritable cache storage. CI's existing
`test:wasm-optimize` command includes these tests. Additional cases verify an
oversized entry is recomputed, and two real worker threads forced through
simultaneous cache misses publish one intact entry without scratch leftovers.
The concurrency barrier accepts both waking a waiter and an already-open gate,
so scheduler ordering cannot turn a correct publication into a spurious failure.
All eight tests passed locally. Native-smoke now runs the same command on macOS
and Windows; those updated jobs have not yet executed. Geometry-scale cache
timings and Windows rename behavior remain unverified.
