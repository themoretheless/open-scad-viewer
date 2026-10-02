# Source-bound placed STEP specimens

Twelve exports cover the four admitted source rim arcs in the original frame,
an axis permutation plus translation, and rotation around two axes plus
translation. The partial-annular-placed-step native example reproduces them.
Matrices and selected edges are recorded in manifest.json.

verify-cad-partial-placed-occt.py independently imports each native STEP with
cadquery-ocp 8.0.1.0.0. Placed bounds are compared with the matching base result
transformed by OCCT; volume is compared with the independent removed-material
integral. All 12 have one valid solid, maximum bounds discrepancy 1.43e-14 mm
and maximum volume discrepancy 6.89e-8 mm³, below the unchanged 1e-6 thresholds.

A native test separately checks all four admitted arcs in both placed frames:
body identity, exactly 24 uniquely parented support faces, complete coverage
by original face identities and unchanged source model. This does not prove
all unchanged vertex/edge identities under arbitrary floating-point rotations.

General geometry, a complete boundary intersection proof, UI command dispatch,
preview/cancel behavior and user document history acceptance remain open.
