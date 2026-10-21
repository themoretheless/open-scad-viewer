# Rational NURBS source atlases and mesh rounding

The new Rust source certificate proves a whole rational-surface-to-PL homotopy
without a global radial centre, product recognition, or a prescribed polynomial
deformation. It accepts compatible full rectangular positive-weight NURBS
atlases, including nonconstant rational weights, multiple knot spans and degrees
1–8 in each tensor direction. The checked freeform torus has genus one:
128 chart triangles, 64 vertex stars, 7290 disjoint patch pairs and 1030 link
boundary pairs. The rational box has six independently curved rational faces.

The proof checks original rational seams, projected vertex disks, Jordan link
boundaries and all nonincident swept chart pairs. The certificate binds original
source data, exact rational endpoint coordinates, connectivity and depth.
Standalone source certification and verification are available through Rust,
the native checker, the WASM bridge and TypeScript transport. The source
certificate describes the exact rational endpoint; it makes no claim about an
unchecked subsequent display conversion. Source authority is the original
oriented face images, as in the previous enclosure certificates, rather than a
claim about unrelated stored 3D edge metadata.

Enclosure certificate version 4 additionally binds this atlas to the actual
oriented nominal source chain. Cases covered by the existing v3 recipes retain
v3 output; v1/v2/v3 verification retains the previous recipes. Two proved source
atlases do not prove topology of a Minkowski sum. The existing separate radial
and prismatic sum proofs still govern that stronger claim.

Rounding now handles coplanar disjoint hulls with exact in-plane separating
axes, removes common translation before hull bounds, subdivides complete closed
time intervals and checks exact area squared when a valid triangle rotates
beyond its original normal. Real collapse and crossing still refuse. The former
critical f32 example (tube radii 2/1, height 2, kernel 1.9×0.5×0.25) now proves its
rounding homotopy, while correctly leaving curved-sum topology unproved.
Its nominal exact volume is 41.7375 mm³.

See [proof-scope.md](proof-scope.md) for the mathematical argument and its
hypotheses. This is an exact sufficient Rust checker, not a proof-assistant
formalization or a complete arbitrary-NURBS decision procedure.

## Exact private candidate

- Optimized WASM SHA-256:
  `70d2945dbde77b1fc223eda4a1e1e9c84558a0cc5c5c4e60f95bb410a65d9e48`
- WASM size: 12,294,925 bytes.
- Frozen source and packaged runtime:
  `/private/tmp/osv-rational-atlas-source-20261019`
- Standalone Rust checker:
  `/private/tmp/osv-rational-atlas-checker-20261019`
- Checker SHA-256:
  `de11083ee3b44717e7cdb229788223c93fe99e941c2f18e84d025c7ff3953181`

[build-receipt.json](build-receipt.json) binds the raw input to Binaryen's cache
key and to the optimized artifact. [frozen-native-source-sha256.json](frozen-native-source-sha256.json)
and [owned-source-sha256.json](owned-source-sha256.json) record the exact source;
[frozen-source-drift.json](frozen-source-drift.json) is empty. Geometry and
verification remain in Rust; TypeScript only transports data and runs fixtures.

## Validation records

| Check | Evidence |
| --- | --- |
| 47 native geometry tests, all passed | `final-compatible-native-tests.log` |
| 12 runtime certificates and 35 rejected forgeries | `final-wasm-results.json`, `final-wasm-tests.log` |
| Same 12 certificates and 35 forgeries checked in separate Rust executable | `final-standalone-results.json`, `final-standalone-tests.log`, `legacy-v3-results.json` |
| Application calls, original document preserved, proof JSON round trip | `final-host-results.json`, `final-host-tests.log` |
| 17 candidate application/protocol tests | `final-host-candidate-tests.log` |
| 5 focused UI component tests on the installed kernel | `ui-current-tests.log` |
| Type checking | `types.log` |

The complete freeform box and torus exercise source certification independently
of a filled Minkowski decomposition. Application sum integration additionally
uses a single curved rational cap with nonuniform rational weights on all faces;
this isolates the new source proof from unnecessary metric-cover complexity.
The signed-zero canonicalization of runtime JSON fixtures happens before either
checker receives an input; no source is modified after certification.

Only these feature checks qualify this private artifact. Node 26.12.0 was used
for the runtime scripts. Node 20, real-browser, provider-wide, Sweep and STEP
release gates are not inferred from another artifact. This task does not replace
shared generated assets, `dist`, or immutable runtime evidence versions. Unrelated
native changes already present in the shared checkout are frozen in this source
ledger; their broader delivery is outside this qualification.

## Remaining scope

General curvilinear/holed UV trims require a compatible embedded parameter
triangulation. Singular chart poles require additional local charts. Arbitrary
nonlinear clocks on the same curved seam require verified parameter transport;
pointwise seam identity is currently required. Some otherwise embedded stars
still fail the sufficient projection conditions or finite work/subdivision
budgets. Genuine f32 collapse cannot be certified as topology preserving.

The final results are summarized in [manifest.json](manifest.json); delivery
status is recorded in [delivery-status.json](delivery-status.json).

The final artifact replays certificates produced by the earlier 030c candidate
and the archived df2 v3 candidate. Verification recomputes their whole geometry;
these are test inputs, not transferred runtime gates. The source-box and both
sum producer entry points are also exercised again and their canonical
certificates compared. A frozen v3 polynomial-atlas rounder prevents new display
predicates from changing old serialized proof choices; see
[legacy-recipe-check.json](legacy-recipe-check.json).

The `diagnostics/` directory preserves rejected/aborted intermediate runs,
including the fixed metric-bound regression. These are not the final validation
records listed above.
