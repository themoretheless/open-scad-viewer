# Independent original UV trim clocks

Native qualification, 2026-11-02.

Original-boundary binding and imported-face sewing now read explicit trimmed polynomial UV lines using exact rational arithmetic. Pcurve clocks may differ from world curve clocks. World intervals must still be complete; periodic lifts and partial world intervals remain refused by this constructor. Polynomial surfaces and existing graph-transition construction are required. This does not claim arbitrary rational NURBS trimming or general fillet completion.

Legacy model UV sampling now respects full-world-edge UV trim intervals, including directed intervals. Partial-world diagnostic windows retain their existing full-topology contour semantics. Sampling does not grant continuous or exact admission.

## Evidence

31 native tests passed in one terminal-successful cargo run: exact_graph_strip 17, continuous_validation 6, frozen_graph_volume 4, validation_sampler 4. See native-tests.log. git diff --check passed.

New tests demonstrate:
- Original imported/support faces with independent UV clocks [2,6] trimmed to [3,4] sew into the supported 9-face/25-edge exact closed shell. Source geometry and identities remain unchanged; source/transition field and G2 checks remain active.
- Six negative cases cover both source-binding and imported-face paths: changed UV endpoint, partial world interval and periodic lift are refused.
- The existing exact volume recipe with 27 edges and nondyadic root cuts accepts the trimmed UV representation, proves global embedding and G2/curvature <=10 mm^-1, exports/restores, and preserves exact source, artifact and volume.
- Generic sampled validation handles independently directed UV clocks and rejects out-of-domain clocks. Generic interval validation still returns BREP_GEOMETRY_UNRESOLVED for this new fixture; the test explicitly preserves that refusal. No generic continuous-validation completion is claimed.

## Open scope

Partial 3D-root ownership across neighboring source faces, arbitrary rational NURBS/periodic charts, general mixed-radius/5+ nodes, radius-law and unrestricted thickness proof remain unfinished. This change is native-only; packaged WASM has not been rebuilt with these edits. STEP/part/history acceptance and the broader P0/P2/P3 goal remain open.
