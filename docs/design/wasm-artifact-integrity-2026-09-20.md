# Verified WASM Compilation

## Scope

This addresses the artifact-loading portion of the
[runtime manifest binding gap](runtime-manifest-binding-2026-09-20.md), not
qualification admission. Geometry and language builds now generate a small
identity module from their actual optimized bytes. Historical qualification
records and archived manifests are unchanged.

Both asynchronous and synchronous embedded compilation check exact length and
SHA-256 before creating a module. Inputs and expected identity are snapshotted
before asynchronous work so caller mutation cannot change compiled bytes after
verification. Artifact size remains bounded to 16 MiB. Verified modules receive
an internal weak-map association, not a forgeable public property.

Optional host compilers receive the expected build identity. A returned module
must have been produced by `compileWasmArtifact` for that identity; arbitrary
precompiled modules are refused with `WASM_ARTIFACT_MISMATCH`. This protects
the host contract against accidental wrong-version modules, not against
malicious JavaScript capable of replacing native APIs in the same realm.

For bound network artifacts, the browser loader reads at most the declared
size, verifies it, then compiles it. It no longer publishes a streaming-compiled
geometry/language module before byte verification. The existing two-second
deadline covers fetching, hashing and compilation; failure returns null so
the kernel uses its verified embedded copy. Stalled bodies are cancelled, and
late responses are discarded. Unbound streaming clients such as the separate
photogrammetry loader retain their existing contract; they are not newly attested.

Verification occurs once per kernel initialization, never per geometry request.
WebCrypto is used for async hashing where available. The existing synchronous
JS SHA-256 is used for sync callers and hosts without WebCrypto. This is a
correctness change with a measurable cold-start cost, not a throughput win.

## Rejected Rust Experiment

The already-locked `sha2` 0.11.0 was tried in the standalone Brotli bootstrap,
with a bounded owned input and a 32-byte digest export. The digest matched Node
SHA-256 at padding boundaries and through 16 MiB. This did not justify retaining
it: first verified compilation in fresh Node processes took 124.44 ms versus
106.98 ms for the JS control. A bootstrap-local sha2 `opt-level=3` variant took
125.32 ms versus 108.12 ms for its interleaved JS control. Neither was a win.

The Rust changes, profile override, added dependency edge and distribution
notices were removed, and the original bootstrap rebuilt. No new Rust dependency
ships in this change. Experimental patch:
`/private/tmp/osv-sha256-rejected-rust.patch`; reports:
`/private/tmp/osv-artifact-verification-bench-{b1,b2,o3}.json`.

## Reproduction And Compatibility

`node --import tsx benchmarks/wasm-artifact-verification.mts` runs nine fresh
Node processes per mode, alternating forward/reverse mode order. Timing covers
first compilation plus verification when selected. File IO, module imports,
identity setup, decompression, instantiation, browser and network time are
excluded. No local build or test runs during measurement.

Node 22.23.2, macOS arm64, 7,716,482-byte geometry artifact. Median milliseconds
in two separate campaigns (nine fresh processes per mode in each):

| Mode | First campaign | Final campaign |
|---|---:|---:|
| Raw async compilation | 6.051 | 6.070 |
| Verified async (WebCrypto) | 9.754 | 9.764 |
| Raw synchronous compilation | 5.070 | 5.016 |
| Verified synchronous (JS digest) | 110.279 | 108.006 |

Thus the selected async integrity check costs about 3.7 ms; sync verification
costs about 103-105 ms on this workload. This does not measure complete startup
and does not establish a browser speedup. Raw reports:
`/private/tmp/osv-artifact-verification-bench-{a,final}.json`.

Host-backed benchmarks and the runtime audit now use the verified compilation
API. A historical raw WASM cannot silently replace the kernel of the current
build: it must be measured from its matching checkout/generated identity. A
mismatched `GCODE_WASM_PATH` or surface-group artifact path now fails explicitly,
rather than falling back and mislabelling measurements. Historical reports
retain their original pre-verification methodology and artifact hashes.

`node scripts/check-wasm-artifact-browser.mjs` uses a temporary Vite server and
real Chromium to check the actual geometry artifact, corrupted/truncated/
oversized responses, a stalled body and an unverified host module. Set
`CHROMIUM_EXECUTABLE` when the locally installed browser is used. This is a
browser smoke test, not qualification or a latency benchmark.

## Remaining Work

The runtime audit still reports the previously established mismatch between
active qualified v1 manifests and the now-verified current artifact. This change
does not permit unqualified execution, rewrite admission policy or grant a
qualified record. Runtime descriptors and explicit development/qualified
admission still need the coordinated correction described in the earlier report.
The synchronous digest cost is a measured candidate for avoiding sync startup
in worker composition roots, not grounds to remove artifact verification.

## Verification

The complete final Vitest run passes 3,383 tests and fails the same nine
historical qualification-binding checks (353 files, 122.43 seconds). The focused
integrity, streaming and runtime-audit run passes 24 tests, including real
kernel refusal and recovery. Vue, MCP and standalone changed-script/test
typechecks pass. Production build and `verify-dist` pass: 92 artifacts,
5,963,149 asset bytes plus 9,744,344 raw WASM bytes, 15,707,493 total.

Chromium 156.0.8063.3 accepts the actual kernel and refuses each damaged,
truncated, oversized and unverified-module case. The stalled response returns
null at about 2,002 ms. Its source-module Vite harness is a real browser smoke,
not a production-bundle end-to-end rendering test. Report:
`/private/tmp/osv-artifact-browser-final.json`.

Geometry WASM is unchanged at SHA256
`bb97e78ae87b5fa690fc474e76ac413b704b737cec3e319e4b548e0d92ac47d2`.
The language artifact was rebuilt from current source: its size stays 1,341,106
bytes, with 40 differing bytes versus the previously tracked artifact. The
intervening `modelgraph-runtime` source difference is formatting-only, shifting
line positions. Its new SHA256 is
`541a186889fc102a8f26486c614a4a7c2ea6d9261fbaf72cf4cd795e186109c4`.
The runtime-manifest audit still exits 1 for the known qualified-v1 attribution
defect; verified loading does not falsely turn that audit green.

The updated G-code boundary benchmark succeeds with the current artifact
(80k-move warm median 187.538 ms in a single control run), while selecting the
retained older artifact exits with `WASM_ARTIFACT_MISMATCH` before sampling.
This is a functional benchmark-harness check, not a new before/after speed claim.
