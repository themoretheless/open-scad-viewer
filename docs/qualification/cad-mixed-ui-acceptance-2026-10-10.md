# Mixed UI acceptance, 2026-10-10

All three declared specimens completed 20 mixed operations in the actual browser, 20 Undo, 20 Redo and structural document equality after reload. All 21 B-rep states per specimen match the service histories. The actual UI-exported STEP files were independently measured by OpenCascade exact line intersections: bracket 3 gauges, flange 2, enclosure 3.

- [Bracket evidence](cad-mixed-bracket-ui-2026-10-10/qualification.json)
- [Flange evidence](cad-mixed-flange-ui-2026-10-10/qualification.json)
- [Enclosure evidence](cad-mixed-enclosure-ui-2026-10-10/qualification.json)

The enclosure run was repeated without HMR after adding the explicit additional-edge selector. Seven ready previews were canceled with document equality. The selector permits additive selection without clearing existing edges and rejects invalid indices; duplicate additions preserve selection cardinality. English/Russian regressions passed. The full direct-modeler UI suite passed 323 tests in 126.65 seconds; Vue typecheck passed.

During the flange run WebGPU became unavailable, CPU fallback continued, and every Undo/Redo and reload preserved the exported document. This demonstrates this observed recovery only; it does not establish the cause of earlier browser-process crashes or full GPU/recovery parity.

## Remaining scope

These section gauges are not whole-wall thickness certificates. A second actual browser STEP import/export cycle now passed for all three specimens: six independent valid solids and 16 section gauges. All six topology identity sets are preserved. Vertex and edge bindings match exactly after resolving array indices by ID. Face UV orientation, loop order and shell face orientation are normalized by STEP; their equivalence is qualified below for these three specimens. Mouse picking and SVG keyboard focus on dense/occluded edges still need qualification; the enclosure used the explicit selector. General NURBS fillets, mixed-radius corners, full tangency/thickness/self-intersection coverage, the complete P0 matrix and P2/P3 remain open.

## Actual STEP roundtrip

[Six-file OCCT report](cad-mixed-ui-step-roundtrip-2026-10-10/occt-report.json), [16 dimension gauges](cad-mixed-ui-step-roundtrip-2026-10-10/dimension-report.json), [native orientation-aware identity proof](cad-mixed-ui-step-roundtrip-2026-10-10/native-oriented-status.json). Validity, one solid, bounds and service-reference volume passed for each file. The section gauges derive from declared dimensions; they do not prove whole-wall thickness.

## Completed planar support equivalence qualification

The native orientation binding test now passes all three actual UI roundtrip specimens (1.00 seconds). Bracket and enclosure match exact rational geometry/incidence after canonical face senses. The four expanded flange supports are proven coplanar by exact orientation predicates; their oriented 3D edge cycles and loop identities match. Both documents independently pass whole pcurve-to-edge coupling qualification before the planar trimmed region is compared. [Native report](cad-mixed-ui-step-roundtrip-2026-10-10/native-oriented-status.json). This supersedes the earlier flange refusal; general curved reparameterizations and whole-wall thickness remain open.

The native rerun with four negative controls passed (0.95 seconds): warped support, shifted parallel plane, altered oriented edge cycle, and altered pcurve all refuse.

## Sequential multitab recovery

[Actual browser evidence](cad-multitab-ui-2026-10-10/qualification.json): a 1 mm translation on the controlled flange/AP242 document triggered a conflict in a second tab. Loading the saved version, Undo to local, Redo to shared and reload passed four whole-document equality checks. Undo/Redo writes triggered the reciprocal conflict. Simultaneous commit races, browser crashes and corrupt storage remain unqualified.
