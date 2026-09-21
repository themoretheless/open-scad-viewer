# Geometry System Design Audit: 2026-09-21

## Current Shape

The geometry stack has four useful ownership layers:

1. The TypeScript facade (`CadSolid`, `CrossSection`) owns user-facing
   composition and compatibility defaults.
2. The Rust kernel owns retained geometry handles, validation, budgets,
   topology, exact/NURBS semantics, and operation results.
3. The ABI returns opaque response or mesh handles. JavaScript creates typed
   views, copies owned arrays, and then releases the Rust handle.
4. Analysis is split into explicit operations: display rendering, BVH build,
   semantic edges, inspection, and combined analysis.

This split is preferable to passing a mutable mesh graph through every host
operation: ownership and refusal rules stay native, while the host receives
stable snapshots that can be transferred or cached.

## Invariants

- Rust handles never cross worker/realm boundaries.
- Unknown wire variants refuse explicitly; they do not become empty geometry.
- Failed operations do not publish partial geometry or mutate retained inputs.
- Display output preserves authored triangle order, crease classification,
  face ids, merge records, and deterministic byte parity.
- Combined analysis must match the individually requested mesh/BVH/edge calls.
- WASM artifacts are rebuilt from source and checked by the runtime manifest.

## Main Costs

- `mesh_render::render` is the dominant warm solid-analysis phase for dense
  curved meshes. The current CSR adjacency and property-vertex chain are the
  right data structures, but their temporary storage is recreated per call.
- The JS/WASM boundary necessarily copies response arrays into host-owned
  typed arrays. This is intentional ownership, not zero-copy transport.
- Disposable NURBS process timings include startup, validation, transfer, and
  shutdown; they must not be used as native-kernel latency.
- `CadSolid` compatibility methods still expose a broad legacy facade, which
  makes adding operations less discoverable than the typed Rust operation
  families.

## Next Refactor Boundaries

1. Keep `geometry-bridge` as the ABI adapter and move reusable mesh analysis
   kernels into small Rust modules with typed input/output structs.
2. Introduce an explicit per-session analysis scratch owner for adjacency and
   temporary vectors. It must be leased to one operation, cleared on release,
   and bounded by the same mesh budget; no global mutable scratch.
3. Keep `RenderMesh` as the canonical display snapshot. Other consumers should
   request views of it instead of independently rebuilding normals or merges.
4. Separate compatibility facade methods from the typed operation dispatcher;
   new features should enter through typed request/response schemas first.
5. Add native microbench coverage for render, not only JS/WASM aggregate
   analysis, with the existing byte-parity reference as the oracle.

## Rewrite-from-Zero Shape

If rebuilt from zero, the stable core would be:

- `geometry-types`: bounded scalar/vector/mesh/topology value types;
- `geometry-kernel`: pure operation planning and native geometry execution;
- `geometry-session`: retained handles, leases, budgets, cancellation, and
  publication transactions;
- `geometry-analysis`: render/BVH/edges/mass-property products over one owned
  mesh snapshot;
- `geometry-abi`: a deliberately small C/WASM boundary with versioned typed
  envelopes;
- host adapters for TypeScript, CLI, and worker transport.

The current repository is moving in this direction. The highest-value next
step is scratch reuse plus a native render benchmark, not a wholesale rewrite:
both can be validated against the existing ABI and parity suite.
