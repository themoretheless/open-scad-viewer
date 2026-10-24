# Fresh controlled part acceptance

Published WASM: aec4f2618d7356e5ec305e12aa931cdf264a90e2f7f4a4d5cd36cfd4482ccdc3.

Three parts were constructed afresh through production services, translated
by (3,-2,1) mm, rounded at radius 1 mm and exported before/after. Bracket and
flange use profile extrusion; enclosure uses box Boolean difference. Selected
fillet capabilities are simple prism, annular circular rim and layered prism.
Two commits per part survive two Undo/Redo steps and exact JSON restore with
B-rep topology identities. Initial document normalization preserves B-rep.

OpenCascade independently imports all six STEP files as valid solids, with
bounds and volumes matching analytical expectations. Maximum bounds error is
about 1e-7 mm; maximum volume error about 1.65e-6 mm³. See manifest.json and
occt-report.json for precise values and hashes. Preview was visually inspected.

This is fresh native controlled-class acceptance, not a new browser mouse or
keyboard chain. JSON restore is not browser reload. General NURBS fillets,
wall/seam qualification, crash recovery and full P0 remain open. The newer
point-cache candidate is not covered by this published-package run.

Independent STEP face/line intersections also passed all 24 declared gauges:
bracket webs and rounded corner; flange bore, off-axis bore, axial thickness
and outer rim section; enclosure walls, cavity, bottom, open top, wall height
and rounded corner. These are specified sections, not whole-wall coverage.

STEP reimport preserves all topology identity sets and exact vertex/edge
bindings. The strict whole representation binding check remains failed:
STEP flips some planar charts/face uses and expands flange periodic support
charts. Face-to-loop identities remain preserved. Full chart/orientation
equivalence is unqualified; see identity-report.json and chart-differences.json.

## Oriented STEP incidence

`oriented-incidence-report.json` passes for all three modified parts. Exact vertex/edge geometry bindings, body-to-shell bindings, and oriented face-to-loop-to-edge boundaries survive STEP normalization, modulo cyclic boundary start and shell orientation. Deliberate shell-orientation and coedge-direction mutations are rejected for each part. This does not certify surface interiors or pcurve/surface correspondence; the strict representation report remains failed and chart equivalence remains open.

## Algebraic chart and incidence qualification

`chart-map-incidence-report.json` supersedes the earlier unqualified chart assessment for these three fixtures only. All 63 faces have exact coefficient agreement under identity, U reflection or owned quadratic Bezier-span restriction. Matching positive rational bases bound trim UV deviation at every parameter by at most 4.1633363423443376e-17 (tolerance 1e-14). Exact edge/vertex bindings and oriented incidence are required in the same gate. Surface and trim mutations are rejected separately on each part. Strict raw-representation equality remains false; general chart matching, full seam tangency, whole-wall checks and browser reload remain open.
