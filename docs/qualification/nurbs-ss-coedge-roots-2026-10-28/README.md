# Original SS boundary events bound to actual B-rep coedges

`ss_face_membership::qualify_coedge_roots` binds the original graph/simple-root/branch proof to every actual boundary coedge of both selected immutable models. Each face must exactly retain its corresponding original support definition and native parameterization, complete persistent naming and one authored shell use. The shared existing face preparer revalidates model definitions/topology, simple outer/hole regions and whole-clock actual 3D-edge/pcurve/support agreement. Stale source geometry and mismatched saved world edges cannot reuse roots.

Every actual loop and coedge is visited, including those with zero roots. Source addresses retain model side, shell and face-use index/identity, face identity/reversal, outer/hole role, loop identity, coedge ordinal and actual edge identity/reversal/degeneracy. A coedge ordinal is scoped to its immutable source loop; no persistent standalone coedge ID is invented. Original root ordinals and the private native SS edge/vertex bindings remain available through each private coedge record.

Original pcurve root clocks are transported to the actual independent 3D-edge knot domain with the actual coedge reversal applied once. The source root position box is expanded only by the model's already-verified whole-clock edge/lift tolerance to contain the actual saved 3D-edge position at that event clock. This is a query position bound, not a new vertex or a tolerance increase. Positive rational pcurve reparameterization and the corresponding independently represented world curve are checked together.

Each event is freshly classified against the other actual retained face. Original normalized-domain ownership justifies clipping an outward native UV overestimate to that original domain, without enlarging its trims. Roots excluded by an actual hole remain explicit Outside events. Unresolved membership fails rather than being treated as Inside.

An additional exact closed-boundary witness is available when both original native domains are identical and the actual pcurve definition literally matches one audited other-face coedge. The original normalized correspondence is then the identity, so the true event lies on that actual other boundary. This can prove closed-set membership when a generic interval winding query remains in a boundary band. Nearby contours, different native frames or merely similar curves cannot use the witness. The optional counterpart index records the proof actually used, not an exhaustive catalogue of every possible equivalent boundary representation.

Full literal coedge metadata windows with zero periodic lifts are admitted. Partial, directed or lifted windows require their own source-clock ownership and are explicitly refused; they are never silently replaced by the full stored curve. The underlying query pcurve remains one complete clamped degree1..8 positive-rational Bézier. General multispan windows, unresolved endpoint/multiple roots and coincident support regions remain outside this constructor.

## Shared work and replay

The operation ledger includes the original source replay, every root construction, the complete immutable root replay performed by branch binding, branch-binding operations, event parameter/position transport, UV clipping and exact counterpart attempts. Successful immutable replay reproduces the original deterministic operation counts; its known work is reserved before invoking the binder. Exhaustion returns no partial owner. Face interval audit/lift/membership cells and trim pairs use one separate shared face ledger. Ordinary bounded model/curve definition validation and domain preparation retain their existing input-validation limits; they are not claimed as interval operations. A shared coedge ceiling prevents silently omitting any later boundary.

The private nonserializable result borrows the original source and actual models. They cannot change while it lives. `reverify` reconstructs the entire actual boundary census and compares source definitions, exact addresses, input queries, event bounds, membership, original bindings and work counts. A public event copy or fabricated address cannot grant authority.

## Remaining construction work

Complete boundary-event coverage is not complete retained SS-region coverage. A closed SS loop can have zero boundary events and still lie inside both faces. This owner does not pair arbitrary coincident events, choose cut representatives, partition actual edges, build new face loops, assign material sides, sew or authorize a Solid. Those stages and the other four full-plan directions remain open. No new WASM/editor/STEP integration is claimed by this native extension.

## Verification

The final focused run passed **12/12** tests; the complete B-rep library passed **1905 tests, zero failed, five explicitly ignored**. The initial nine tests also passed before adding rational-clock and independent-frame regressions. [matrix.json](matrix.json) records exact source/log hashes and ignored-test reasons. Core NURBS sources are unchanged from their preceding 3060-test qualification; this extension reran the complete B-rep consumer.

Tests cover complete zero-event boundaries around a still-existing closed SS loop, actual hole crossings, retained Outside events, independent edge knot clocks, reversal applied once, actual rational pcurve/world-curve reparameterization, independent other-face native domains, literal common trim membership, sub-tolerance contours without false coincidence, stale edited supports, mismatched actual world curves, full/partial metadata windows, exact shared operation exhaustion and source/report tampering. Identical numerical pcurves in different native frames cannot claim boundary coincidence; explicitly tighter UV classification resolves their four distinct events without merging them. Input model snapshots remain unchanged.

Rustfmt, diff whitespace and API mapping pass. The architecture gate still reports 71 existing violations; no limit was increased. This native addition does not resolve the existing distribution-size gate or expose the new source-owned operation in WASM/UI/STEP.

```sh
CARGO_BUILD_JOBS=3 CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_TARGET_DIR=/private/tmp/osv-nurbs-plan-target-20261028 cargo test --manifest-path crates/Cargo.toml -p brep-core --lib --offline --locked --no-fail-fast
node scripts/map-nurbs-api.mjs --check
```
