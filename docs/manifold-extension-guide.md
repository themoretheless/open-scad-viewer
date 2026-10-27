# Extending the manifold library

This guide defines compile-time extension points for new manifold-family
operations. **It is not a runtime plugin loader**: operation modules, routes,
TypeScript wrappers, and worker jobs are compiled into their owning packages.
There is no discovery or loading of third-party native or WebAssembly plugins
at runtime.

Use this guide together with `manifold-developer-guide.md` and the release
checklist in `manifold-definition-of-done.md`.

## Design rule: one native owner

A new operation has exactly one native algorithmic owner. Reuse the lowest
existing crate that already owns its geometry or topology:

- `manifold-core` owns strict manifold predicates, repair, and metrics.
- `mesh-section` owns horizontal section and projection algorithms.
- `polygon-core` owns general solid/mesh geometry, including hull and simplify.
- `manifold-ops` owns admission and postconditions that promise a strictly
  manifold result; it orchestrates lower-level kernels rather than copying them.
- `geometry-bridge` decodes requests, calls native functions, encodes results,
  and manages existing handles. It does not implement geometry.
- TypeScript owns types and integration only. Never add a second hull,
  simplifier, slicer, manifold check, repair, or boolean implementation there.

Do not make `manifold-core` depend on `polygon-core`, `manifold-ops`, the
bridge, WASM, or UI code to host an operation. That would reverse the
established dependency direction.

## Existing transport is sufficient

The generic value-codec path is:

```text
TypeScript callGeometryRust(op, args)
  → request(0, { op, ...args })
  → abi_request(0, ...)
  → geometry(dispatch(value))
  → domain routers, then geometry-bridge dispatch::ROUTES
  → native owner
  → { ok: true, value } or { ok: false, error: { code, message } }
```

The raw four-export manifold ABI remains the appropriate path for the existing
check/repair/metrics/boolean handle API. A new operation does not automatically
need new raw exports: use generic dispatch when a value result is sufficient,
and add handle/slot ABI only when its ownership or performance contract really
requires them. Never create a separate transport per UI consumer.

Unknown or malformed input is an error envelope, not a partial result. Native
`math_core::Error` codes and messages become `{code, message}`; TypeScript
`decodeNurbsResult` raises `GeometryKernelError` (or the NURBS-specific error
where applicable). Preserve these mappings and document operation-specific
error codes before exposing them publicly.

## Operation contracts

The following operations have native foundations but different output
semantics. A public manifold-family extension must not blur them together.

### Hull

Canonical native implementation:
`polygon_core::solid::primitives::hull3` / `hull3_refs`.
The existing handle-based adapter is `CadSolid.hull` → generic `cad` action
`hull` → `geometry-bridge/src/mesh.rs` → `hull3_refs`; use it directly for that
API rather than adding another algorithm.

Contract and current limits:

- the 3D hull consumes the union of input mesh vertex positions, sorts and
  exactly deduplicates them, and builds a convex triangular solid;
- fewer than four unique points, or inputs without sufficient 3D extent,
  produce an empty mesh rather than a lower-dimensional hull;
- at most 20,000 unique input points and 20,000 current hull faces are allowed;
  budget exhaustion is an error;
- degenerate triangles are cleaned and output winding is normalized;
- the tolerance in the current algorithm is scale-dependent; it is not an
  approximation tolerance supplied by the caller and not an error bound.

For a strict-manifold facade, define input policy explicitly: either reject
non-manifold inputs or apply the documented Full admission repair before taking
their vertices. Run a fresh strict post-check before claiming manifold output.
An empty hull is a valid geometric outcome but not a solid with positive
volume. Do not silently turn empty output into an error unless the public
contract explicitly excludes degenerate dimension.

### Simplify

Canonical native implementation:
`polygon_core::solid::primitives::simplify`.

Contract and current limits:

- it merges coplanar triangulation, preserves authoritative boundary rings
  including holes, retriangulates those regions, and stitches the result;
- it is **not** general mesh decimation, remeshing, shape approximation, or an
  optimizer for arbitrary vertex/triangle counts;
- it removes numerically degenerate triangles and uses scale-dependent
  coplanarity and weld tolerances; therefore “no shape approximation” does not
  mean bit-for-bit preservation of every input coordinate or triangle;
- groups whose boundary loops cannot be obtained are retained as pieces rather
  than being simplified; callers must not assume every coplanar group was
  collapsed;
- the current native result does not preserve UV attributes. A public wrapper
  must either reject/drop UVs explicitly or define a tested attribute policy;
- do not infer improved manifoldness from fewer triangles.

If the operation promises strict-manifold output, post-check in `manifold-ops`
and return the output report/residual policy with the mesh. If post-check fails,
return an error rather than an unlabelled non-manifold mesh. If the application
only needs best-effort geometry cleanup, name that weaker contract separately;
do not reuse the strict operation name.

### Slice

Canonical native algorithms live in `mesh-section`, with compatibility
adapters in `polygon-core/src/solid/section.rs`.

There are already two related, non-interchangeable contracts:

1. `sectionPolygonMesh` → `mesh_section` → `MeshSectionIndex::section` returns
   horizontal section contours plus `candidateTriangles` and per-contour
   `sourceTriangles` diagnostics.
2. `CadSolid.slice` → `cad` action `slice` → `mesh_section::slice` returns
   planar `Rings` stored as a `CrossSection` after closed-boundary traversal
   and planar union.

Choose the required existing contract; do not add a third slicing algorithm.
A slice returns a **2D profile**, not a manifold 3D mesh. Strict-manifold input
may be an admission requirement for a solid-profile API, but the output must
not be described as a strict-manifold solid or sent through a mesh-producing
post-check.

Current `mesh_section::slice` uses strict edge crossing and therefore does not
provide a general coplanar-face intersection operation. It requires finite `z`,
uses a fixed absolute `1e-8` XY weld epsilon, rejects open or ambiguous boundary
traversal, and enforces a boundary-length budget. Document whether empty
profiles and tangential contacts are accepted before adding a public wrapper.
The contour diagnostic path and planar-union path have distinct semantics;
tests must target the selected one.

## Extension sequence

### 1. Stabilize the native contract first

Specify operand kinds, units, output kind, empty-output behavior, tolerances,
attribute policy, resource budgets, deterministic ordering, and error codes.
State whether the operation guarantees strict-manifold mesh output or returns
another geometric type.

Put the algorithm in its canonical native owner. Where a strict solid
postcondition is required, add a small `manifold-ops` facade around it:

```text
validate inputs
  → reject or explicitly Full-repair operands
  → call the existing native operation
  → strict output check for mesh-producing strict operations
  → return mesh + report, or a typed error with residual diagnostics
```

Do not automatically apply aggressive repair to every new operation. Admission
policy changes geometry and must be visible in the API and result report.

### 2. Add a thin bridge adapter

For generic value results, register a stable operation name in the appropriate
existing family router (usually `dispatch/mesh.rs` for direct mesh operations)
and decode/encode through `field`, `encode`, and `json!`. Alternatively, an
operation on existing `CadSolid`/`CrossSection` handles belongs in the current
`cad`/`mesh` action adapter.

Do not modify `abi_request` operation 0, add another response envelope, or move
algorithmic policy into the bridge. Extend raw manifold ABI only when necessary,
without renumbering existing functions or array slots.

### 3. Add one typed TypeScript wrapper

Call the bridge through `callGeometryRust`; reuse `PolygonMesh` and existing
typed-array normalization. Return owned arrays or immutable profile data and
explicit reports. Keep default tolerances in one shared public definition.

Handle-based APIs must preserve `delete()`/lifetime rules. Generic value calls
still release request allocations and response buffers through `kernel.ts`;
wrappers must not bypass that cleanup or retain WASM linear-memory views.

### 4. Extend the existing worker only when integration needs it

A pure library wrapper does not require a worker job. For main-thread
integration, extend the existing path rather than creating a new worker:

1. add the typed variant to `MainSolidJob` and its result to
   `MainSolidResults` in `mainSolidProtocol.ts`;
2. register the kind in exhaustive `MAIN_SOLID_JOB_KINDS`;
3. execute it in `mainSolidWorkerRuntime.ts`;
4. capture request-specific limits/counts in `mainSolidExpectation.ts`;
5. validate the result against that expectation in `mainSolidResult.ts`;
6. transfer only freshly created response buffers in
   `mainSolidWorkerTransport.ts`; never detach scene-owned request buffers;
7. report progress and honor the existing worker lease/cancellation protocol
   where the operation is long-running.

Test through the real packaged-WASM worker, not only direct TypeScript calls.
A generic value-codec operation is still synchronous inside native Rust:
checking cancellation before/after a call is not cooperative cancellation.
Long operations need native budget/cancellation support and safe cleanup before
being advertised as cancellable.

### 5. Add UI only when requested

The UI consumes the worker/public wrapper and must not recompute geometry.
Where applicable, connect command state, Undo/Redo, progress, cancellation,
error presentation, and non-manifold residual diagnostics. A profile result
needs profile visualization/export, not solid-manifold indicators.

## Qualification checklist for an extension

- Native unit tests cover empty, degenerate, tangential, budget, error, and
  representative valid inputs.
- Property/fuzz tests exercise invariants appropriate to the operation:
  containment/extrema for hull, idempotence where guaranteed for simplify, and
  profile/boundary invariants for slice. Do not claim properties the current
  algorithm does not establish.
- Determinism and f32/f64 narrowing are checked on native and packaged WASM.
- Budget exhaustion, cancellation, and all error paths release handles and
  buffers and never publish partial geometry.
- Public ABI/artifact exports and smoke tests are updated only when the chosen
  path requires them; generic dispatch is exercised through the artifact too.
- Worker expectation, result validation, transfer ownership, and real-worker
  behavior are covered when a worker job exists.
- A focused rbench case measures the native owner and repeated end-to-end path;
  compare baseline/current on the same machine and document noise.
- User guide, error catalog, tolerance policy, developer guide, changelog, and
  migration notes reflect actual behavior and limitations.
- Finish every applicable item in `manifold-definition-of-done.md` before
  calling the operation production-ready.
