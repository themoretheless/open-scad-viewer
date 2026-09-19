# Sparse face overlay indexing experiment

## Finding

`buildFaceTriangleIndex` rejects IDs above `4 * triangleCount + 1024` to avoid
allocating an offsets table based on a potentially enormous uint32 ID. The
renderer caches that null result, but subsequent face selections then scan the
entire face-ID array. Dense IDs already have a cached CSR index and are not the
target of this experiment.

A compact `Map<faceId, row>` plus CSR prototype removes repeated scans while
preserving ascending triangle order. It was tested in production code, then
removed because its synchronous construction delays the first hover. The
prototype is retained only in the benchmark, not shipped in the renderer.

## Reproduction

Run `node --import tsx benchmarks/sparse-face-overlay.mts` on an otherwise idle
machine. The report includes Node/CPU identity, production and benchmark source
hashes, raw samples and index storage counts. Each fixture has sparse uint32 IDs,
two triangles per face, and 100 varying face selections per measured sample.
There are three warmups and nine alternating scan/indexed measurements. Exact
filled-triangle and boundary-line parity is asserted outside timing. Seven
separate construction samples include allocation/GC. This is a synthetic CPU
experiment, not a rendered-frame or representative imported-model measurement.

## Results

Two consecutive local runs, milliseconds (repeat in parentheses):

| Triangles | Build compact index | Scan, 100 selections | Indexed, 100 selections |
| --- | ---: | ---: | ---: |
| 2,000 | 0.239 (0.226) | 0.260 (2.042) | 0.131 (0.242) |
| 100,000 | 5.193 (5.400) | 5.372 (5.451) | 0.178 (0.187) |
| 500,000 | 35.992 (32.762) | 25.815 (25.645) | 0.194 (0.195) |

The small fixture is unstable and supports no reliable speedup claim. Larger
fixtures show a repeatable improvement for repeated selections, but construction
only amortizes after approximately 100-103 selections at 100k triangles and
129-141 selections at 500k. These are CPU break-even estimates, not observed user
interaction counts. At 500k, retained typed storage is 3,000,004 bytes plus a
250,000-entry JavaScript Map; the report deliberately does not label typed-array
bytes as total memory.

## Decision

Do not build this sparse index synchronously on first hover. The 33-36 ms setup
cost exceeds a 60 Hz frame interval before overlay creation/rendering, whereas a
single direct scan in this fixture costs about 0.26 ms. Keep current production
behavior while evaluating worker-side preparation, bounded per-face caching or
a lower-allocation index. Any replacement must measure first-selection latency,
publication cost, retained memory and repeated selections on actual sparse-ID
models. Dense-ID behavior and the overlay triangle cap must remain unchanged.

The temporary production prototype passed 18 overlay/grouping tests and UI type
checking; after rejecting it, production and existing test files match their
original bytes. The retained benchmark passed exact output checks in both runs.
