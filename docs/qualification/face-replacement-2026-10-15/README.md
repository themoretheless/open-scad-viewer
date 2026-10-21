# Retained sector surface replacement

`face_replacement::replace` replaces the selected face's support surface and
its original-chart pcurves while preserving every existing owned 3D boundary
curve, vertex, edge identity and shell orientation. It accepts an entire face
or a sector produced by `face_cut::partition`. Each actual directed boundary
use requires exact coefficient identity or continuous interval lift agreement.
The complete trim region, orientation, chart containment and unique ownership
are checked before mutation. Work remains bounded by shared cells, 10,000 trim
pairs and 1,000,000 exact work. Periodic replacement charts refuse.

Source and replacement face identities are recorded with Modify lineage;
unaffected sectors keep their geometry and identities. Failure returns no
model. The result is explicitly a candidate requiring new global admission.
Closed topology is not promoted to a radius, G1/G2, thickness or solid proof.

## Verified controls

Two new tests cover a surface whose center depth changes by 0.1 mm, preserved
owned topology, incorrect boundary lift, missing pcurves and insufficient work.
A second control partitions a planar face and replaces one sector with a
nonplanar bicubic surface while retaining the other sector and all shared edges.
Its design polynomial vanishes on the diagonal and original outer boundaries.
Both positive controls pass continuous exact-aware closure. The focused
face_replacement, face_cut, face_window, face_crossings, edge_partition,
pcurve_geometric_g2 and watertight suites passed all 35 tests.

## Independent STEP measurement

Run `face_replacement_acceptance` with an output directory, then
`scripts/verify-cad-step-occt.py <directory> --no-preview` in the OCP environment.
The current independent run reports one valid solid, 12 unique edges and 8
unique vertices. Expected volume is the design integral
`512 - 64 * 0.4 / 9 = 509.15555555555557 mm3`; OpenCascade measured
509.15555555555545 mm3, an error of 1.1368683772161603e-13 mm3.
Outer bound error is 1e-7 mm. The generated manifest deliberately records
`nativeProven:false`: native global solid admission was not run for this control.
Generated artifacts are under `output/qualification/face-replacement-2026-10-15/`.

## Outstanding objective

Automatic contact-curve construction, material-side selection/removal, fillet
end transitions and unequal-radius complex corners still need implementation.
This mechanism does not generate a rolling-ball or G2 blend. Whole-result
radius, tangency, thickness and self-intersection qualification, WASM/worker/UI,
bracket/flange/enclosure/history acceptance, P0 and remaining P2/P3 stay open.
