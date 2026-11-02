# Local component-count performance review

`manifold-core::count_components` counts union-find representatives directly instead of allocating a second hash table for unique labels. Every component has exactly one representative whose label is its own index. The component-label algorithm and its consumers are unchanged.

An independent face-adjacency traversal checks the count on 192 generated/order-reversed soups, plus vertex-only contacts and degenerate faces. All 34 manifold-core tests and 7 manifold-ops tests passed. The architecture audit passed with its existing 52 size pins and zero layer violations.

[review.json](review.json) retains three paired local comparisons against the failed CI run's base `4d065b3b60f969f72ea7645911ab832eb1fbefe5`. Both sides use the pinned nightly and the thorough profile: 100 observations per case, five cases, three processes per revision, in alternating order. Every pair and the aggregate passed the unchanged 25% regression threshold. The aggregate check/icosphere-5120 measurement was 955414 ns/op for the base and 878195 ns/op for the current binary, an observed local reduction of 8.08%.

[raw-runs.tar.gz](raw-runs.tar.gz) contains all six original run documents, including all 3000 observations. Their uncompressed hashes and measured binary hashes are in the review. Compressed test and architecture logs are adjacent.

This is a local macOS measurement with uncontrolled background load. It does not resolve or supersede the earlier failed hosted measurements, prove a universal speedup, attest an exact final source bundle, or qualify a WASM/STEP/UI runtime. Hosted CI, Linux numerical reconciliation and current runtime artifact qualification remain pending.
