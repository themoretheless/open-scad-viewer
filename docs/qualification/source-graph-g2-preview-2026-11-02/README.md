# Source graph G2 preview transport

Added `brep_source_graph_g2_preview` to the common native/WASM B-rep router. Original graph supports and the width curve construct the exact convex G2 candidate in Rust. Only its rounded display surface crosses this preview boundary. A single coefficient budget covers construction, display conversion, whole-field curvature and full-patch radius inspection. The radius inspection always covers [0,1] x [0,1], not the previously qualified narrow central band.

The versioned report exposes exact source contact-jet status, display storage error, the fixed 10 mm^-1 curvature limit, actual radius decision/witness, work usage and explicit false solid/embedding/application admission flags. Unsupported request fields refuse, preventing callers from supplying admission flags. TypeScript validates the report against requested radius and work budget before exposing it. Forged body claims, omitted geometry, partial radius domains and cancellation are tested.

`sourceGraphG2Preview` is registered in the shared MainSolid worker job/result/expectation protocol. It returns a display-only candidate, never a mutated CAD document. The worker client can cancel and replace the execution realm. This is backend and worker integration; no new UI command, edge-selection workflow, closed-body artifact transport or STEP/history completion is claimed.

Native bridge tests pass (2); transport and existing worker-client/runout regression tests pass (10). Real packaged-WASM evidence and separate-worker evidence are archived after the rebuild.

## Final packaged evidence

All 17 selected tests pass: 5 native Runout/router regressions, 10 transport and shared-client tests, and 2 real separate-worker tests. The standalone verifier additionally proves direct WASM and actual handler replies agree, invalid zero-work requests fail, and original inputs remain unchanged. The worker test cancels the first realm and accepts a verified reply from its replacement. No browser UI interaction or new-operation-specific late-reply injection is claimed.

Packaged WASM SHA-256: `1a70b8d7c3c94767c70df5c99b4a5fcc136161151df4a12cbf8a67cbe40ce99c`, 16334881 bytes. Full-patch radius decision remains `exceeds`; all admission flags remain false.
