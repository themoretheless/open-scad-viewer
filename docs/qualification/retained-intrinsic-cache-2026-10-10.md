# Shared retained intrinsic and endpoint derivative hulls

The immutable retained-profile workspace now serves whole-rectangle intrinsic
checks as well as endpoint pairs. Both use the same complete derivative hull
cache, with exact profile/UV keys. Input validation, continuous-chart checks
and zero-budget refusal still happen before lookup. No final decision is cached.
BRep wall coverage now routes retained intrinsic checks through this workspace
and accounts for its cache hits.

Focused native controls pass: 25 tests, zero failures. New checks compare
warm intrinsic decisions with fresh computations and require lower span work,
retain zero-budget and invalid-domain refusals, and ensure a rational
half-cylinder's real normal diameter is never excluded by a warm cache.
Log: `/tmp/retained-intrinsic-cache-controls-20261010.log`.

Source manifest:
`mapped-retained-carrier-2026-10-08/retained-intrinsic-cache-native-inputs-2026-10-10.json`.
Full NURBS regression and matched complete-carrier wall coverage are launched.
The wall request is unchanged: 10,000 cells, 1,000,000 spans and 100,000 trim
cells. Work savings and coverage equivalence remain unproven until completion.

This source generation is newer than the endpoint-only retained cache currently
being optimized into WASM and the separate live partial-rational wall binary.
Those processes use previously compiled immutable binaries; their results must
not be assigned to this new intrinsic-cache implementation.

Full NURBS regression completed with exit zero: 848 passed, zero failures,
two ignored, 29.50 seconds. Log:
`/tmp/retained-intrinsic-cache-nurbs-all-20261010.log`. The BRep matched coverage
run is still separate and must be checked before claiming work reduction.

Reproducible compile-only WASM build launched separately for this generation:
`/tmp/retained-intrinsic-cache-wasm-compile-20261010.log`. All 1,811 manifest
inputs match the current source at launch. This does not publish an artifact.

## Matched wall and raw ABI completed

The matched complete-carrier wall test completed with exit zero: one passed,
253.21 seconds under concurrent load. Span work decreased from 23,890 to
23,653 (237 saved, 0.992%). Intrinsic work decreased 1,802 -> 1,651 and retained
pair work 1,390 -> 1,304. Every pending product, order, reason and exclusion
counter remains exactly equal; 4,081 regions are still unresolved. The strict
coverage/work-reduction gate passed:
`mapped-retained-carrier-2026-10-08/retained-intrinsic-cache-regression-2026-10-10.json`.
Runtime is not a controlled benchmark and no whole-wall certificate follows.

Reproducible WASM compilation completed with exit zero in 2m14s. Raw SHA256:
`a8edc0b661a4be05eeaccbfa4a4e3570913c49d06f67fd50903058002b55685c`,
14,765,245 bytes. All six direct ABI cases passed, with zero-budget refusal,
invalid-domain refusal and recovery. Report:
`mapped-retained-carrier-2026-10-08/retained-intrinsic-cache-unpacked-abi-2026-10-10.json`.

Full production packaging is launched after the endpoint-cache publisher
completed. Log `/tmp/retained-intrinsic-cache-wasm-build-20261010.log`.
Full BRep regression log `/tmp/retained-intrinsic-cache-brep-all-20261010.log`.
A separate matched partial-carrier maximum-work run is launched with unchanged
100,000 cells / 1,000,000 spans / 1,000,000 trim cells; log
`/tmp/retained-intrinsic-cache-partial-wall-max-20261010.log`.
Its results must be compared with the endpoint-cache partial run after completion.

## Maximum-work partial rational carrier: terminal

The matched 100,000-cell / 1,000,000-span / 1,000,000-trim-cell run terminated successfully: one test passed, 1594.90 seconds under concurrent workload. Log `/tmp/retained-intrinsic-cache-partial-wall-max-20261010.log`.

Against the endpoint-cache baseline, total native span work decreased from 795,987 to 788,883 (7,104 fewer, approximately 0.8925%). Intrinsic spans decreased from 395,624 to 392,341; retained-pair spans from 15,298 to 11,477. Both process exactly 100,000 cells. The exact-coverage comparator passed: all 46,287 pending products, domain endpoints, order, refusal reasons and exclusion counters are identical. Exact containment also passed with no changed products.

Reports: `mapped-retained-carrier-2026-10-08/retained-intrinsic-cache-partial-regression-2026-10-10.json` and `retained-intrinsic-cache-partial-pending-2026-10-10.json`. This establishes reduced certificate work for unchanged coverage; whole-wall qualification remains false. It predates the width-Gram and section-data-cache generations.
