# mesh-query

Triangle BVH construction, ray picking, scene broadphase, closest points,
oriented signed distance and sampled mesh deviation. Runtime dependencies are
`osv-math` and `mesh-topology`; there is no polygon, JSON or WASM dependency.

```rust
use mesh_query::{MeshView, closest_point};
let positions = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
let indices = [0, 1, 2];
let mesh = MeshView::new(&positions, &indices);
let (point, distance) = closest_point(&mesh, [0.25, 0.25, 2.]);
assert_eq!(point, [0.25, 0.25, 0.]);
assert_eq!(distance, 2.);
```

- `bvh`: float32 vertex records with explicit stride, u32 triangle indices,
  deterministic median splits and packed node/triangle buffers. Invalid and
  degenerate triangles are omitted. Cooperative builds preserve the same bytes.
- `bvh_query`: borrows caller-owned buffers. Queries retain unnormalised ray
  parameters, affine transforms, inverse-transpose normals and triangle exclusions.
  Malformed cyclic trees return `QueryError::InvalidTree`.
- `scene_bvh`: binary64 object bounds, stable distance/id order.
- `proximity`: binary64 mesh buffers. Nearest point and signed distance require
  buffers validated once by the caller; out-of-range indices may panic. Empty nearest-point queries return
  the query point and infinite distance. Signed distance requires a closed,
  consistently oriented, non-self-intersecting boundary for solid semantics.
  Deviation samples vertices and triangle centroids in both directions, with an
  8,000,000 triangle-test budget; it is not a Hausdorff error certificate.

Cooperative futures deliberately yield without waking an executor; the owner
must repoll after returning to its event loop. Dropping a future cancels it.

## Examples and measurements

```sh
cargo run -p mesh-query --example picking
cargo bench -p mesh-query --bench picking
cargo bench -p mesh-query --bench picking -- ../../examples/skadis-box/hook-box.stl
```

Run from the `crates` directory. The benchmark reads binary or ASCII STL, builds
one BVH and casts 10,000 deterministic vertical rays across the XY bounds. It
reports build time, hit count and per-query p50/p95/p99 including timer overhead.
The default input is a generated 32,768-triangle height field. Timings depend on
the machine and hit/miss distribution; they are measurements, not guarantees.

## API contract for 0.1

- `build_mesh_bvh` and `raycast` share the same immutable vertex/index buffers
  and stride (at least three float32 components). Keep the buffers alive and
  rebuild the tree after changing geometry. Triangle IDs are source indices.
- `Query` rays need not be normalized. `Hit::t` uses ray parameter units; it is
  a distance only for a unit direction. Bounds are inclusive. `local_from_world`
  uses a column-major affine inverse. Exclusions use original triangle IDs.
- Packed `MeshBvh` buffers are public for existing host adapters. Their layout is
  an in-memory contract; persisted buffers should carry the producing version.
- `closest_point` uses binary64 geometry and a linear triangle scan, not the
  float32 picking BVH. Validate `MeshView` before using indexed proximity calls.
- `QueryError` implements `std::error::Error`; malformed trees produce an error.
  Invalid rays or empty trees produce no hit. A miss is a successful query.

## Release dependencies

The package has an MIT license and versioned registry dependencies. Release
`osv-math`, then `mesh-topology`, then `mesh-query`; both mesh crates require
`osv-math` 0.1. Tests use
`serde_json` only as a development dependency for the frozen picking corpus.
Neither library needs nightly Rust or a viewer build. Publication and registry
verification are separate from local source and consumer checks.

### Verification snapshot (2026-09-30)

Stable Rust 1.98.1 passed 28 unit tests and two documentation examples across
query/topology, including the frozen 256-case picking corpus. Clippy with
`--all-targets --no-deps -- -D warnings` passed for both packages. The CPU `osv-math` range-loop warnings were subsequently resolved; strict
Clippy now also passes with dependencies.

Both `.crate` archives pass Cargo package verification with local registry
patches for unpublished dependencies. An external consumer runs picking and
nearest-point calls from the unpacked archives, with only `osv-math` patched
to local source. These are local package checks, not registry publication proof.
A live crates.io package check reported no `osv-math` package; registry-only
verification remains blocked until that dependency is published.

One local run on `examples/skadis-box/hook-box.stl` (648 triangles) measured
0.367 ms construction and ray p50/p95/p99 of 750/1500/2083 ns, with 9,779 hits
out of 10,000 rays. The generated 32,768-triangle field measured 12.824 ms
construction and 584/750/875 ns. Both are single-run measurements including
clock overhead, not comparative performance claims.

## Repeated distance queries

`distance_bvh::distance_triangles(MeshView)` validates and prepares binary64
triangle records. `DistanceBvh::build` builds a median-split tree for repeated
`distance` and `signed_distance` queries. The sign uses ray parity and requires
a closed boundary. Empty trees return infinite distance.

`flatten_distance_bvh` provides the shared GPU/CUDA lattice layout: ten float32
words per node, with child IDs and triangle windows stored as integer bits,
and nine float32 coordinates per triangle. Picking retains its separate float32
buffer contract.

## Reconstruction admission

`proximity::valid_source` uses native exact welding and checks nonempty input,
the caller's triangle ceiling and triangle degeneracy. It returns owned neutral
buffers; it does not require closedness or certify full manifoldness. The
application adapter owns compatibility error codes and mesh conversion.
