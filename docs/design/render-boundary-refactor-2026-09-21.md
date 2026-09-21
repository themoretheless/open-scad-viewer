# Render Boundary Refactor: 2026-09-21

## Decision

Keep the current detached `CadMeshBuffer -> RenderMesh` contract as the
compatibility oracle, and introduce any fused `export -> render` path behind a
private Rust adapter first. Do not widen the WASM ABI until the fused path has
the same output signature and a measured host-visible win.

## Evidence

The current retained-handle measurements are:

| Fixture | native render | host render | host export |
|---|---:|---:|---:|
| sphere-128 | 2.353 ms | 6.563 ms | 3.091 ms |
| three-spheres-128 | approximately 7.059 ms | 19.965 ms | 9.792 ms |
| cylinder-128 | 0.097 ms | 0.160 ms | 0.041 ms |

The native value includes only `mesh_render::render` over a detached snapshot.
The host render value includes export preparation, ABI response allocation,
linear-memory views, owned typed-array copies, and render. The values are not
additive. They identify the boundary as the next target, while preserving the
current normal/property-vertex algorithm as the parity-controlled kernel.

## Invariants

- Authored triangle order, face ids, crease classification and merge records
  remain byte-identical.
- Near-zero position normalization from the export snapshot remains intact.
- Rust-owned buffers never escape their lease; host arrays remain transferable
  and independent of WASM memory growth.
- A failed fused operation publishes no partial render result.
- The existing detached path remains available as a fallback and oracle.
- No persistent global scratch is introduced; reusable storage is session- or
  operation-owned and bounded by the existing mesh budget.

## Staged Implementation

1. Factor snapshot preparation and render assembly behind private typed Rust
   inputs without changing the current ABI.
2. Add a native A/B harness that compares detached and fused output signatures
   on cube, sphere, cylinder, degenerate and transformed fixtures.
3. Add a host-boundary benchmark that measures export, fused render, response
   copy and total publication separately.
4. Retain the fused path only if sphere and three-sphere cases improve while
   cylinder and all parity tests remain stable.
5. Expose a new ABI operation only after the private path is proven; otherwise
   keep the existing contract and treat the result as a rejected candidate.

## Explicit Non-Goals

This refactor does not migrate `GeometryScene` into the runtime owner, change
face-selection semantics, or introduce adaptive LOD. Those are separate
boundaries with different ownership and camera contracts.
