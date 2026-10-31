# Exact-artifact runtime v7 delivery, 2026-10-31

The distribution verifier now selects a reviewed budget by the exact geometry
WASM SHA-256. This is a versioned delivery-policy correction, not compression or
geometry simplification. Unknown hashes fail rather than borrowing a larger cap.

| Artifact | Geometry chunk cap | Total asset cap |
| --- | ---: | ---: |
| v6 `2ff06f662379…` | 4680000 | 9090000 |
| v7 `91521ffbc325…` | 5370000 | 10080000 |

V7 measures 5360574 geometry bytes and 10066845 asset bytes: margins are 9426 and
13155 bytes. Its geometry WASM remains exactly 15900297 bytes. The raw-WASM,
streaming, decompression, native proof/work, memory and geometric tolerance limits
are unchanged. The historical v6 policy and all predecessor records remain intact.

The controlled pre-architecture input already produced 5354665 embedded-source
bytes; the current generated source is 5360625 bytes (+5960, about 0.11%). The
old delivery cap therefore predates the feature set. See the preserved controlled
build evidence in `../architecture-recovery-2026-10-31/` for its exact scope.

Additional delivery checks repaired private Rust module identifier style and
legacy native Clippy findings. Named quaternion multiplication APIs are preserved;
standard Mul operators delegate to them, with narrow compatibility expectations.
Reduction grouping and arithmetic order remain unchanged. The rebuilt optimized
geometry WASM is byte-identical, not merely numerically close.

The owner census records current paths and explicitly marks the remaining
TypeScript numerical migration work. A reviewed census does not mean all
algorithms are implemented in Rust. The previous dated census is preserved.

The full 6540-test frontend receipt remains applicable to the unchanged app source
and exact WASM; the new distribution index is also byte-identical. The changed
fingerprint/manifest tests and Node delivery-policy tests are re-executed.
OwnRust80 / G0v94 / G1v111 bind current sources with zero completed G1 clean work.
No full native-workspace, cross-host, performance or hosted CI claim is inferred.

Final measured checks and evidence hashes are recorded in `receipt.json`.
