# Photogrammetry transport optimization — 2026-09-09

The WASM adapter now serializes borrowed geometry directly into the existing MGV1 response. It does not first allocate a Value array for every vertex, color and triangle. An exact preflight includes envelope, keys, metadata, arrays and numbers; only then is a single exact-capacity output buffer allocated. Each numeric triple is appended as one 32-byte record. Small arbitrary metadata still uses the shared value-codec encoder.

Sparse output borrows Point fields directly instead of collecting separate temporary position/color arrays. The stored diagnostic report is moved once and borrowed for later responses. Dense/compact response field order, float/integer tags, nonfinite-as-null behavior, byte/item/depth caps, errors, and session state publication order are preserved. The JS API and shared codec are unchanged; there are no new dependencies and the module has zero WASM imports.

| Fixture | Vertices / triangles | Serialization ms, before → after | Allocation count | Allocated MB | Fresh process peak RSS MB |
|---|---:|---:|---:|---:|---:|
| shell12 | 17,158 / 10,782 | 2.381 → 0.301 | 45,154 → 17 | 9.97 → 1.44 | 12.22 → 6.09 |
| monstree6 | 19,422 / 18,210 | 3.010 → 0.367 | 57,110 → 17 | 11.50 → 1.83 | 13.58 → 7.00 |
| large | 125,000 / 250,000 | 27.438 → 3.183 | 500,059 → 17 | 97.56 → 16.00 | 167.76 → 24.77 |

Timing scope is serialization alone; it must not be described as an 8× reconstruction improvement. The two real PLY fixtures come from the prior photogrammetry qualification run. The large case is a deterministic synthetic transport stress fixture with 125,000 vertices and 250,000 triangles, not a newly reconstructed high-resolution scan. The encoded bytes are respectively 1.44, 1.83 and 16.00 MB.

Measurements use release opt-level=s, LTO, the same process binary and unchanged inputs. Every path runs in a fresh process. Iteration 0 separately records Rust allocation traffic/live bytes; iteration 1 warms the path; reported medians cover iterations 2–7 with tracking disabled. The allocation wrapper remains installed in this test binary, so ordinary production pipeline benchmarks are the final cross-check. Other agents were working concurrently: these times are indicative. RSS includes process/harness, fixture loading and serialization; it is not browser heap or end-to-end WASM peak memory. Allocated MB counts output encoding allocations only and excludes the already-resident input.

Verification:

- 15 Rust tests passed; one explicit comparative benchmark is ignored in ordinary test runs.
- Byte-for-byte equality and decoded-value equality against the old generic encoder cover missing cameras, point order, Unicode metadata, positive/negative zero, subnormal/large floats, null nonfinite coordinates, u32/u64 values, empty geometry and diagnostics.
- Boundary tests preserve exactly 32 MiB / 4,000,000 items / depth 128, count the complete envelope, reject overflow, and reject geometry whose item count is oversized while its bytes still fit.
- Real shell12 and Monstree plus the large transport fixture produced identical binary files and identical results through the unchanged JavaScript decoder.
- The final release WASM build succeeds and has zero imports. Root-owned full application/browser verification remains a separate integration step.

Changed production files are recorded with SHA-256 in `changed-files.json` (four files, all within crates/photogrammetry-wasm/src). Raw records and provenance are in `transport-comparison.json`; JS decoder validation is in `js-equivalence.jsonl`; Rust/build logs are adjacent.

Reproduce in this isolated tree:

```sh
cargo test --offline --release --manifest-path crates/Cargo.toml -p photogrammetry-wasm
python3 reports/benchmark.py
```

The benchmark can also be run directly from the release test executable with `--ignored --exact response::tests::benchmark_surface_transport --test-threads=1 --nocapture`. Set PHOTO_TRANSPORT_MODE to baseline, direct or verify and PHOTO_TRANSPORT_FIXTURE to an ASCII PLY with XYZ/RGB vertices and triangle faces. Omit the fixture for the synthetic stress case. The fixture reader is a test-only reader for trusted qualification outputs, not a new product import API.
