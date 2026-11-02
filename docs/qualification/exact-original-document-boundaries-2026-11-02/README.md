# Original document boundary identities for the sewn roof

The full seven-stage CAD goal remains active. This step binds the sewn roof's natural boundaries to ORIGINAL document edge and vertex TopoIds with exact parent intervals. It does not append the remaining source faces or close a solid. Prior receipts remain unchanged.

## Naming and complete geometry equality

`Roof.bind_original_boundaries` takes explicit original source-face indices and checks their actual surfaces against the retained-region sources. It maps original canonical UV-line intervals to actual natural source uses, including original coedge reversal. It never searches by Cartesian distance or welds near vertices.

Original 3D curve definitions must be nonperiodic polynomial Beziers with constant positive weight gauge. The immutable original curve is restricted in exact rational arithmetic; canonical orientation is applied and COMPLETE polynomial equality is checked after exact degree elevation where necessary. Native degree-nine source-line representations therefore match original degree-eight curves through proof rather than fitting or endpoint sampling. Equality applies to the same normalized parameter clock, not merely to the geometric point set.

Each unique bound roof edge records the original source edge index, actual TopoId, rational parent range and direction. Endpoint records retain original source vertex IDs at original parameters 0/1. New cut endpoints retain their parent edge ID and exact rational parameter rather than adopting a nearby old vertex. Shared crease uses from the two source faces must resolve to the SAME original identity and interval.

`OriginalBoundaryMap.is_current` checks the relevant source face/loop snapshots and IDs, full original edge definitions including endpoint indices, mapped original vertex values/IDs, and actual roof root ownership/ranges. This is geometry/naming-scope validation, not a completed application document-session/revision protocol.

## Evidence

Eleven B-rep integration tests pass. The new case proves all original boundary identities, both normalized original-crease intervals [0,1/4] and [3/4,1], derived parent-edge endpoint metadata, and invalidation on source changes. A 2^-100 interior curve perturbation is rejected even though it is far below ordinary modelling tolerance. Existing source trim, split, field and sewing tests remain green.

The final expanded control records eight unique original edge links: all SIX open outer roof edges and the two retained original crease intervals. Every binding is current. Topology remains 22 faces, 74 edges, 53 exact vertices and 142 face-use checks. Complete roof field upper remains 9.867331764661289/mm under the unchanged 10/mm cap. Shared charged generated coefficients: 1109243. Shell.closed=false and bodies.len=0.

Reproduce with `CARGO_TARGET_DIR=/tmp/open-scad-trim-g2-20261101 cargo test --offline --manifest-path crates/Cargo.toml --config profile.dev.package.brep-core.opt-level=2 --config profile.dev.package.brep-core.debug=0 -p brep-core --test exact_graph_strip`, or the same flags and `run -p brep-core --example expanded_contact_runout -- --exact-source-brep-split`.

## Remaining original-face import and solid qualification

Use these named bindings to append the other original side/floor faces, preserving their edge/vertex identities and verifying actual UV/3D lifts. The present binder requires explicit source faces, no source holes/authored coedge trims, linear polynomial original UV uses and polynomial Bezier original world curves. Multi-span/rational original edges and more general source arrangements need extension; they are not silently accepted. Complete closure, orientation, global embedding/intersections, radius, thickness, volume, new-entity persisted naming, coordinate Model/WASM/worker/UI, STEP/history/part acceptance, P0 and P2-P3 remain open. Hashes cover listed files in a shared dirty worktree, not an isolated delivery revision.
