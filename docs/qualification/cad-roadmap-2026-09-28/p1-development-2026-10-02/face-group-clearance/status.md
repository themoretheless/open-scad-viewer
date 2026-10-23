# Complete selected face unions

`distance_between_face_sets` measures the minimum distance between two unions
of original trimmed faces. It validates full source models and nonempty,
unique, in-range selectors. Every selected pair contributes a lower bound,
including pairs pruned or left unrefined by the shared work budget. Witnesses
retain original source face indices and parameters, rather than subset slots.

The complete-shell API delegates to the same implementation. Initial bounds
combine Cartesian, spatial radial and coordinate-axis radial separations.
All three optional radius bounds remain conservative and fall back on numeric
or unsupported-chart failures. Surface subdivision and trim admission still
supply the upper witness; topology membership alone cannot supply one.

Canonical native results at 1e-5 mm tolerance:

| Source union | Target union | Pairs covered | Interval mm |
| --- | --- | --- | --- |
| All three blend charts | All six inner-wall faces | 18 | 13.749999999999917 … 13.750000017677785 |
| All three blend charts | All six bottom faces | 18 | 4.749999999999997 … 4.750005575035684 |
| All six retained outer-wall faces | All six inner-wall faces | 36 | 14.999999999999902 … 15.00000150253369 |

One pair is refined in each case; the complete initial pair bounds prove
coverage of all other pairs. `native.json` retains source geometry, selectors,
work counts and original-face witnesses. Reproduce with `face-group-clearance`.

Eight shell-distance regressions pass, including unsorted source selectors,
empty/duplicate/out-of-range rejection, geometry and domain work exhaustion,
source preservation, overlapping groups and existing whole-shell cases.
The full suite before this extension passed 725 tests with three ignored;
these focused checks qualify the extension, not a new full-suite count.

This is selected-union clearance, not a general material-thickness definition.
Automatic opposing-region detection, material chord admission, integration
through WASM/UI and qualification of other poses and geometries remain open.
Endpoint G1 and the complete P0-P3 roadmap also remain open.
