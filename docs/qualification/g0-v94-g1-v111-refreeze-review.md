# G0 v94 / G1 v111 delivery amendment

Preserve OwnRust79, G0v93, G1v110, their review/status, generator and the finite
runtime-v7 frontend receipt byte for byte. Append new source identities only.
All 4740 clean G1 work units remain incomplete; no prior run is imported.

The v6 distribution caps predate the current v7 feature set: a controlled build
of the pre-refactor input already exceeded them. The current artifact is exactly
91521ffbc32570cddc99b4555dcdd3f55fdaa902a806fccc215bf7afefab129f. Its measured
geometry chunk is 5360574 bytes and asset total is 10066845 bytes. The reviewed
v7 caps are 5370000 and 10080000 bytes, retaining margins of 9426 and 13155 bytes.
The selector binds these caps to that exact SHA; unknown artifacts are refused.
V6 keeps its original 4680000 / 9090000-byte policy. No geometry tolerances,
proof work, memory, stream, decompression or raw-WASM caps are raised.

Private Rust impl-module identifiers use snake_case. Standard multiplication
operators delegate to the existing named Quat/DualQuat API, retaining public
compatibility and arithmetic order. Targeted Clippy expectations document that
compatibility API alongside the implemented trait. Compute lint repairs preserve
slice grouping, comparison semantics and arithmetic order; benchmark-only lint
repairs add no performance claim. The optimized geometry WASM rebuild is
byte-identical to the already tested v7 artifact.

The owner registry now names current paths and explicitly records remaining
TypeScript numerical migration work. Zero unreviewed numerical files is a census
review result, not a claim that all algorithms have moved to Rust. The earlier
owner census is retained; the current census is dated 2026-10-31.
