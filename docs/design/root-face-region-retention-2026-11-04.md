# Root face UV support and retained topology

`QualifiedRootFaceRegion` classifies a complete finite UV query box against the authored outer-minus-holes domain: nonzero outer winding, excluding the union of all nonzero hole windings. It evaluates every loop, records their windings and returns OutsideOuter, ExcludedByHoles with all local hole indices, or InsideSupport. Boundary/unknown queries yield no classification. The segment budget is shared across loops; geometry allowances are explicitly per loop. Up to 64 hole loops are admitted.

This is membership in an explicit UV support, independently of Jordan embedding or physical solid material orientation. It must not be used to infer whole-face Boolean material from a single sample.

`RootFaceRetention::retain` performs actual candidate topology selection for explicit face indices. It removes unused vertices, world edges and loops, retains surfaces and root curves, updates all topology indices and pcurve world-edge links, and returns stable old-to-new maps for vertices, edges, loops, faces, shells and bodies. Selection order does not reorder the source topology. Repeated or invalid face IDs are rejected. Empty selection produces an empty model.

Partly retained shells become open. A body is retained only when every face of its outer and inner shells remains. This prevents surface trimming from implicitly creating a valid-solid claim. Existing root vertex owners and all retained geometry definitions stay unchanged. Replay checks geometry, topology and all lineage maps.

Remaining: complete face arrangements and Jordan/nesting proofs, global retained material classification, unrestricted singular/tangent/periodic/pole SS branches, shell orientation/cavities/self-intersections/healing, arbitrary intersecting Boolean and bridge/WASM qualification. Explicit face selection is not an automatic Boolean classifier.

Regressions cover hole exclusion and remaining support on both children of a mixed SS chord split, shared segment-budget refusal, retention of one child with its hole, preserved UV closure, stable maps, empty selection, invalid indices and removal of incomplete bodies.

Validation: 9 tests passed across root_uv_closure and root_model. Extended UV tests took 86.42 seconds. Architecture audit passed: 54 pinned oversized files, zero layer violations. Logs: docs/qualification/root-face-region-retention-2026-11-04.
