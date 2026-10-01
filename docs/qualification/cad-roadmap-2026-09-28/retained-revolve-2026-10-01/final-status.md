# Retained revolution acceptance — 2026-10-01

Solid Revolve now accepts retained line/circular-arc profiles, including holes and disconnected regions, through the existing native rational revolution. Exact mode is selected automatically; faceted substitution is unavailable. Source control curves and identities remain unchanged. Both axes, radial sides, axis offsets, tilted plane, full and +/-90-degree turns are checked against the polygon route. Crossing-axis input is refused without mutation.

Fixed STEP periodic-carrier recognition: candidate patch counts/degrees must also have matching controls/weights along all grid boundaries and cyclic closure. Partial turns keep their original exact patches, with the existing topology/export certification still required. The holed quarter-turn native regression and all 30 STEP module tests passed.

Packaged geometry WASM: 9,559,365 bytes, SHA256 a6e7378416ec7f3eab0993cb0cbcd81b03d82ca00b84a5991c9f3ba8c421d781. Full frontend CAD regression: 793 passed in 66 files, no skips/failures. New retained test is included in test:cad-roadmap. Vue typecheck and Vite build passed. Distribution gate passed after explicitly measured feature-budget changes: 7,158,076 asset bytes, 11,615,485 raw WASM, 137 artifacts.

OpenCascade independently passed rational torus, holed full/quarter revolutions and disconnected region revolution (two solids): exact hashed STEPs, expected bounds/volumes and reports in occt/. Mouse and keyboard Chromium scenarios passed preview, cancellation, invalid quantity, Apply without recomputation and Undo/Redo. Keyboard additionally injected a worker failure and retried (606 Tab presses, 6 downloads); initial upload is automated. Browser-generated STEP independently passed OCCT. The preview and independent geometry images were visually inspected.

Scope: retained analytic line/circular-arc profiles admitted by planar_trim, regular independent new-body revolutions. General NURBS profile preparation, broader Boolean combinations, larger control-part operations and the full P0–P3 roadmap remain open. Source history in status.md retains first failures and corrections; this final status supersedes its pending entries.
