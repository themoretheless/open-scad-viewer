# Language WASM size profile control

Before integrating PR #21's workspace-wide `s` to `z` change, build both profiles
from the same current language sources in an isolated checkout. Main's generated
artifacts and Cargo profile were not changed. The snapshot starts at f8bf39ab
with the reviewed unused-module deletions; those do not affect the language
kernel. Existing per-package overrides are retained.

## Method

Run `CARGO_PROFILE_RELEASE_OPT_LEVEL=s node scripts/build-language-kernel.mjs`,
then `node --import tsx benchmarks/openscad-frontend.mts`. Repeat with `z`, run
the benchmark twice, rebuild `s` and rerun the benchmark. Order: s, z, z, s.
No local build/test overlaps the measurements. Binaryen and Brotli use the
existing build script unchanged. Node 22.23.2, macOS arm64.

The benchmark checks full wire AST parity outside timed regions and reports
21 alternating TS/Rust samples after five warmups. It measures full ABI calls,
not parser-only CPU time or geometry execution.

## Results

| Artifact | s bytes | z bytes |
| --- | ---: | ---: |
| Optimized raw language WASM | 1,341,106 | 1,133,177 |
| Generated packed bytes.ts | 436,948 | 388,533 |

Raw size decreases 15.5%, packed module size 11.1%. Packed module bytes include
base85 and the TS wrapper; this is not an HTTP compressed-transfer measurement.

| Call nodes | s median ms | z median ms | z repeat ms | s repeat ms |
| ---: | ---: | ---: | ---: | ---: |
| 2 | 0.09192 | 0.10304 | 0.09821 | 0.08737 |
| 200 | 4.14363 | 4.32796 | 4.25621 | 4.15179 |
| 1000 | 20.76638 | 21.50100 | 21.14288 | 20.57817 |

Cold initialization: s 15.14/15.49 ms; z 14.77/14.59 ms. There are only two
process-cold observations per profile; do not claim a reliable startup gain.
The large full-AST cases show a modest throughput penalty rather than a CPU
optimization. Both still substantially trail the TS frontend on these fixtures.

## Integration boundary

This is a delivery-versus-execution tradeoff, not a blanket rejection of `z`.
Before selecting a product profile, measure the product-used ModelGraph path,
geometry build workloads and browser startup/network costs. Do not overwrite
newer generated photogrammetry bytes with the branch's older artifact. Keep
measured package overrides for polygon, photo and the decompressor unless
separate controls justify changing them.

Local JSON reports `/private/tmp/osv-language-profile-{s,z,z-repeat,s-repeat}.json`
contain samples and compiler/host/packed-payload fingerprints. Isolated workspace:
`/private/tmp/osv-cleanup-check.1thRbw`. These are exploratory measurements, not
qualification evidence or a production profile change.
