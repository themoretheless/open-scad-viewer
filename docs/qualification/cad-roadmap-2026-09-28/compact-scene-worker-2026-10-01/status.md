# Compact scene requests — 2026-10-01

The existing sceneEdit API accepts either a full DirectDocument or compact document text. The worker parses compact text through the authoritative validator with its instance cache before applying the operation, then validates the result. The UI sends compact text for group metadata edits and detach when linked instances are present. Gizmo requests retain their full-document type and behavior.

Real postMessage qualification passed for exact full/compact parity of group move, group creation and detach. The compact snapshot omits linked mesh/B-rep caches. Corrupt source indices are rejected before a metadata edit; geometry validation is not bypassed.

Typecheck, production build and distribution checks passed (137 artifacts, 7,155,156 asset bytes). The 1000-instance browser run is active and explicitly records whether metadata/detach requests use compact text. Correctness, cancellation and performance qualification are not yet complete for this implementation. P0–P3 remains open.

Initial metadata/detach browser qualification passed on 1000 linked instances, including group target change and Esc cancellation, exact JSON snapshots, detached geometry, source independence and Undo/Redo. Recorded snapshot lengths are 149960–150003 characters. This records actual postMessage request form, not a claim about wire byte size or overall speed. 52 worker/runtime/transport tests passed.

The UI has now extended compact requests to numeric transforms whenever linked instances are present. Real-worker parity additionally passed for transforming a linked instance. Typecheck, build and artifact budgets passed (7,155,148 asset bytes). A separate final browser run includes edits of both the source with all linked instances and the source after detach; that run remains active.

Final CPU browser run passed group controls, Esc/active-group cancellation, independent geometry and source independence, Undo/Redo and exact final source/link/placement export. All eight relevant metadata, detach and numeric-transform postMessage requests were compact, with lengths 149960–168565 characters.

One sample with automation and durable-save latency: group create 1747.46 ms, group move 1852.49 ms, detach 3249.87 ms, detached-source edit 6542.81 ms, source edit with all links 4466.47 ms. These remain too slow for the large-scene target. The finite uncontrolled samples do not establish an overall speed improvement. Returning expanded results, history copies/validation and display scans remain open performance paths.

Final client, cancellation, real-worker and transport regression passed: 47 tests in four files. This complements the earlier 52 worker/runtime/transport tests. No goal completion claim.
