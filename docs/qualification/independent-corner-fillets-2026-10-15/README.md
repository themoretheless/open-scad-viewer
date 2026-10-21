# Independent longitudinal corner fillets

## Implemented scope

`analytic_features::independent_cuboid_corner_fillets` accepts an audited,
axis-aligned cuboid and four optional bottom/top radius pairs. Corner order:
(-X,-Y), (+X,-Y), (+X,+Y), (-X,+Y). `None` retains a sharp corner.

The native author builds rational circular end profiles, trims the intervening
straight support boundaries, and constructs a closed ruled B-rep with owned
edges and source topology lineage. Radii interpolate linearly along +Z.
The public result must pass the existing solid audit and naming checks.

Adjacent setbacks must leave more than the model tolerance on each shared
support edge, at both ends. Because their sum is affine in height, endpoint
checks cover the whole height. Invalid, nonpositive and overlapping selected
radii are refused. No tolerance or proof budget is increased.

## Qualification

Native regressions cover closure, eight independent end arcs (including
0.1 mm), source vertex preservation, overlap at either end, nonfinite radii,
and the audited public entry point. Test execution status is recorded below.

## Remaining scope

This is a longitudinal prism construction. It does not implement automatic
trimming of arbitrary B-rep faces, general surface-intersection branch selection,
periodic/pole trimming, three or more converging unequal-radius fillets, or G2
corner transitions. Numerical samples in regressions are not a continuous
certificate for those unsupported cases.

WASM, worker/UI, independent STEP roundtrip and history qualification for this
new API are pending. Existing runtime artifacts are not replaced by this work.

### Native results, 2026-10-15

Passed 11 distinct native tests: all seven `imprint_pipeline::tests`, the new
public independent-radius entry point, the previous variable-radius cuboid
fillet, the audited multi-edge fillet, and the previous single cuboid fillet.
The three new tests are included in that count. No full-suite, WASM, browser,
STEP or general-corner qualification is claimed.

The initial shared-cache build reported missing APIs that were present in the
current dependency sources. A fresh target directory compiled successfully and
was used for the results above. No source APIs were removed to bypass errors.
