# Mixed-pair original-root face construction

`RootBrep::face_contours` constructs a candidate face from an outer original-root wire and optional hole wires. `RootFaceContour` supplies explicit surface slots per edge, so different SS pairs and source-slot positions may participate. All selected surfaces must equal the authored face surface. Every native UV joint is checked before returning topology. Global selectors, original world graphs, exact endpoint ports and paired pcurves are retained; no fitted curve or tolerance weld is introduced.

The UV joint authority now also joins a triple port to an ordinary AB port when the sealed triple witness refers to that same global selector and its original A or B role. This is an original-coordinate identity, not a world-space overlap test.

The constructor produces one open shell and no solid body. It accepts candidate hole loops but does not assert hole nesting, orientation, Jordan embedding or retained material. Those require the next arrangement/material stages. General singular, tangent and periodic/pole SS completeness, complete finite trim, healing and arbitrary intersecting Boolean remain open.

Regression geometry: a circle/bowl SS arc on AB and a straight AC chord bound a real planar lens. Both endpoints are distinct algebraic triple roots. The test checks actual mixed graph storage, two shared global vertices, both pcurve calls, exact UV closure replay and rejection of an incorrect face support.

Validation: 12 tests passed across root_uv_closure, root_model and root_brep_subdivision. Architecture audit passed: 54 pinned oversized files, no layer violations. Logs are under docs/qualification/mixed-ss-face-contours-2026-11-04.
