# Mapped retained carrier qualification

The native control is a closed planar tetrahedron with one original edge split at a qualified root: four faces, seven edges and five vertices. Its affine carrier maps cover the original curve; each retained half remains incomplete. The ordered two-fragment chain proves complete coverage on both incident faces.

Native source-family tests: 131 passed. Real WASM worker tests: two passed, including STEP preparation (5 vertices, 7 edges, 4 faces; endpoint error at most 1e-7 mm), archive restoration, metadata Undo/Redo and fresh geometry restoration after reload. Vue type checking and diff checking passed.

These worker results use intermediate WASM SHA-256 `9d9ebf775a57fbf2cfdcd0c2aad87db8212436409e932b1eb33a1a9cca41927b`. It includes the individual-fragment coverage correction, but predates the primary mapped-chain contact extension. Admission already succeeds through another native contact proof. The native controlled regression establishes the chain extension; the worker test establishes transport and restoration, not a before/after admission change. Latest package qualification is pending.

Independent OCCT 8.0.1 accepts the exported control as one valid solid. Volume is 0.16666666666666663 mm³ (absolute error 2.78e-17 mm³); axis extents differ from the unit tetrahedron by approximately 1e-7 mm, within the independent 1e-6 mm acceptance tolerance. The STEP file and hash-bound report are retained here. Reproduce with `scripts/verify-mapped-root-step.py`.

This does not qualify general NURBS fillets, geometric editing history or the three required production parts.

Full real-worker regression suite on the intermediate package: 26/26 passed in 90.70 seconds. This includes mapped root restoration/history/STEP, reference regular seam checks, singular endpoint diagnostics, cancellation and retained-domain wall checks. It preserves explicit unresolved results; it does not establish whole-wall clearance or complete UI acceptance.

## Final package

WASM SHA-256 `d3c14ce4318b38b245e51006387c1acfc9b1295bce5653edf285afad6e8b0caf`, 12,107,173 bytes, includes the mapped-chain contact extension. After packaging completed, the full real-worker suite passed 26/26 in 90.98 seconds. Independent OCCT repeated STEP acceptance successfully. Type checking, package audit and distribution verification passed: 152 artifacts, 8,397,731 asset bytes plus 15,403,565 raw WASM bytes. Existing budgets were retained.

An earlier concurrent worker/build run overlapped publication of generated package files and failed; it is excluded from this qualification. Final evidence comes from runs after packaging completion.
