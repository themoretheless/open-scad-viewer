# Placed annular material wall qualification

This follow-up addresses the specific coordinate rotation and translation recorded in `../placed-limitation.json`. It does not certify arbitrary Euler rotations or whole-body minimum thickness.

Two failures had separate causes:

1. Rational planar cap pcurves were lifted through an independently rounded translated plane. Eligible exact coordinate caps now use the transformed original edge coordinates as their plane-chart coordinates. The original edge curves, vertices, surface control-point set and topology stay unchanged. An exact source-lift guard prevents placement from repairing a preexisting mismatch. General operation charts retain their established UV domains.
2. The ruled blend/wall joined-chart proof tried only two world-axis projections. A bounded family of signed coordinate projections now passes the same full original-chart seam, weighted order and dominance checks. Structural failures are rejected before interval subdivision. Every admitted certificate retains its projection and complete 512-cell report.

The specific placed annular wall now qualifies a 15 mm interval over 36 selected face pairs. Exhausted curved-volume work must still refuse a material interval. Automatic search candidates remain samples; convergence comes from complete selected-union lower bounds and a certified interior material chord.

The final focused placed-wall test passes, including 15 mm convergence and refusal with only one face pair admitted (88.93 s). The complete-cap/source-preservation regression passes. The mismatched-source regression passed separately before widening the rational-boundary degree guard; that guard still excludes the perturbed source. All 46 NURBS Boolean tests pass on the final guard. The bridge suite passed 353 tests / 1 ignored before widening that degree guard.

The complete B-rep suite on the final source passes: 741 tests, 3 ignored, no failures (110.13 s). Final source-chart and joined-projection regressions each pass two tests. The earlier run on the degree-two-only guard is superseded by this full green run. Final bridge source also passes 353 tests / 1 ignored (56.98 s). Final WASM is 9,856,035 bytes, SHA-256 `e8ab780f6f8bc18ebff980e3a036ddd5f3432601b0fa0bc4ea5c172a5e06b8db`.

Seven actual WASM cases, seven typed worker-handler cases and three automatic searches pass. The placed annulus converges on its first sampled candidate to [14.999999999999899, 15.000000000000648] mm; all 36 selected pairs contribute lower-bound evidence. Mouse and keyboard browser runs each pass nine results on this placed model, including oblique rejection, original-face markers, direct/automatic cancellation, localized transport failure and Retry, unchanged document and exact reload. Screenshots were inspected.

The final package passes all 17 part-history service tests. Fresh bracket/flange/enclosure final STEP exports and their import/export cycles pass independent OpenCascade solid validity, bounds and volume checks; maximum bounds error is about 1e-7 mm. Six files pass 16 independent original-face gauges for declared webs/walls, bore and radius-one sections. The `--service` gauge report identifies these exports separately from the prior browser exports.

The distribution verifies 142 artifacts: 7,351,136 asset bytes + 11,912,155 raw WASM bytes = 19,263,291 total bytes. Only measured byte growth was added to limits, preserving preceding margins.

Remaining: automatic whole-wall grouping and complete normal-compatible wall-domain coverage, arbitrary Euler placements and general curved volume qualification. Samples and the declared controlled STEP gauges do not establish whole-body minimum thickness. The full P0–P3 roadmap remains open.
