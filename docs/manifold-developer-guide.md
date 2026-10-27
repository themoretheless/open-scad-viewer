# Manifold library developer guide

This guide defines ownership and dependency rules for the manifold library.
Its purpose is to keep one canonical native implementation instead of parallel
Rust and TypeScript topology logic.

For API semantics and user-facing limitations, see
`docs/manifold-user-guide.md`. For compile-time operation extension points and
the hull/simplify/slice contracts, see `docs/manifold-extension-guide.md`.
Every new operation must satisfy `docs/manifold-definition-of-done.md` before
being called production-ready.

## Dependency direction

The manifold-specific dependency chain is:

```text
mesh-topology
    ↑
manifold-core
    ↑
polygon-core
    ↑
manifold-ops
    ↑
geometry-bridge ← also depends directly on manifold-core and polygon-core
    ↑
geometry-wasm
    ↑
TypeScript manifoldAnalysis.ts
```

Arrows point from a lower-level owner to its consumer. Equivalently, in
`A → B` dependency notation:

```text
manifold-ops → polygon-core → manifold-core → mesh-topology
geometry-wasm → geometry-bridge → manifold-ops
TypeScript → packaged geometry-wasm
```

This is a focused graph, not a complete list of polygon or bridge
dependencies. The essential rules are:

- `manifold-core` must not depend on `polygon-core`, `manifold-ops`, bridge,
  WASM, or UI code.
- `polygon-core` may use core manifold predicates but must not depend on
  `manifold-ops`.
- `manifold-ops` orchestrates the guarantee around polygon kernels; moving
  that facade into core would create a dependency cycle.
- Native crates below the bridge know nothing about raw WASM pointers,
  handles, JavaScript typed arrays, workers, or Vue.
- The TypeScript layer calls the canonical kernels; it must not become a
  second implementation of manifold checks, repair, metrics, or booleans.

## Ownership by layer

| Layer | Owns | Does not own |
| --- | --- | --- |
| `mesh-topology` | Borrowed mesh views, packed indexed edge incidence, boundaries, general inspection, exact welding and display-edge primitives. | Full vertex-link manifold verdicts, manifold repair policy, BSP booleans. |
| `manifold-core` | Indexed transport validation, full manifold reports, vertex links, Conservative/Full repair, repair residuals, mesh/component metrics. | BSP classification, boolean orchestration, raw ABI, worker scheduling. |
| `polygon-core` | General mesh and solid representations, geometric predicates, BSP boolean kernels, classification and kernel work budgets. | The public guaranteed-manifold facade or a duplicate vertex-link implementation. |
| `manifold-ops` | Operand admission, necessary Full pre-repair, boolean options, post-check and conservative seam repair, `BooleanOutcome`/`BooleanError`. | A second BSP implementation or low-level edge/fan algorithms. |
| `geometry-bridge` | Raw input validation, f32/f64 conversion, result storage and handle lifetimes, ABI error mapping, array slots. | New manifold mathematics or policy that belongs in core/ops. |
| `geometry-wasm` | Thin `extern "C"` exports and unsafe-boundary forwarding. | Validation, result policy, topology algorithms. |
| `manifoldAnalysis.ts` | Typed calls, input uploads, output copies, handle/buffer cleanup, public TypeScript types and defaults. | Independent topology, repair, metric or boolean calculations. |

`src/services/meshTopology.ts` also retains application-facing validation and
crease-angle conversion around native semantic-edge extraction. That display
edge contract is distinct from the manifold API in
`src/services/geometry/manifoldAnalysis.ts`.

## Where to add a change

### A new diagnostic or metric

1. Define and compute it in `manifold-core`.
2. Add focused native unit tests plus property/invariant coverage where the
   value has a mathematical invariant.
3. If public through WASM, append result storage/slots in
   `geometry-bridge/src/abi/manifold.rs` and the corresponding owned result
   representation; do not renumber existing slots.
4. Forward only when a new exported function is required; existing array
   access already goes through `abi_array_field`.
5. Expose an immutable typed field in `manifoldAnalysis.ts` and copy the data
   out of linear memory before freeing the handle.
6. Add native/WASM, f32/f64, real-worker, and integration coverage.

Do not compute the same value independently in TypeScript to "save" an ABI
change. That creates two sources of truth.

### A repair operation

Put topology changes and residual computation in `manifold-core/src/repair.rs`
or a focused submodule. Preserve deterministic output ordering and make the
mode contract explicit:

- Conservative must not silently perform aggressive topology reconstruction.
- Full may split shells/fans and fill only supported simple loops.
- Every pass returns the repaired buffers and a fresh residual report.
- Unsupported geometry remains reported; it must not be dropped merely to
  make `is_manifold()` true.

Add corpus fixtures for defective STL/OFF inputs when the change addresses a
real-world defect. Include idempotence/fixpoint properties where applicable.
If an algorithm becomes cooperative or cancellable, its partial state must be
owned and dropped without publishing a partial mesh.

### A boolean-kernel improvement

Geometric intersection, classification, BSP partitioning, and kernel budgets
belong in `polygon-core`. Admission and the manifold guarantee stay in
`manifold-ops`:

```text
input check → Full pre-repair if needed → polygon boolean
            → output check → conservative seam repair if needed
            → success only when strictly manifold
```

A kernel error or budget exhaustion is not an empty successful result. A
non-manifold kernel output is a kernel defect, not something the caller should
silently accept. Run the differential suite against `manifold-3d` and the
rbench regression gate for kernel changes.

### A new manifold-family operation

Place reusable geometry in the lowest existing crate that can own it without
an upward dependency. Place manifold admission/output guarantees in
`manifold-ops`. Add a thin bridge adapter only after the native contract is
stable. Do not add a special transport implementation per consumer.

A new operation is not complete after its native function compiles: its
contract, error behavior, determinism, ABI lifetime, TypeScript wrapper,
worker path, tests, benchmark, and documentation all need explicit owners.

### ABI and TypeScript changes

The raw ABI uses caller-owned input buffers and owned result handles. Keep
these invariants:

- validate transport lengths, formats, finite coordinates, and index ranges
  before topology code;
- never retain a pointer into linear memory across a call that can grow it;
- copy returned arrays before `abi_array_free`;
- free every input allocation and result handle on every exit path;
- preserve existing operation tags and array-slot meanings;
- map native failures to stable error codes instead of traps or partial
  results.

`geometry-wasm/src/lib.rs` should remain a forwarding layer. TypeScript
interfaces belong in `manifoldAnalysis.ts`; worker-specific request/response
orchestration belongs in the existing worker modules, not in native core.

## Testing and qualification map

From the repository root:

```sh
# Required library, ABI, fixed-seed property and TypeScript integration gate.
npm run test:manifold

# Static ownership/size audit and required-check/no-skip policy.
npm run test:architecture
npm run audit:architecture

# After npm run build:geometry: native/WASM and f64/f32 comparison.
node --import tsx tools/manifold-bench/environment-matrix.mts

# Differential qualification and native microbenchmark comparator tests.
cd tools/manifold-bench
npm ci
npm run test:differential
npm run test:rbench
```

CI additionally runs random reproducible property/fuzz seeds, transport
fuzzing, complete native test partitioning, real packaged-WASM worker tests,
and same-machine base/current performance comparisons. Do not add a required
test as skipped, focused, todo, ignored, or allowed-to-fail.

## Review checklist

Before accepting a manifold change, answer:

1. Is the algorithm in the lowest correct owner, with no upward dependency?
2. Is there exactly one canonical implementation across native and TS?
3. Are input validation, output manifoldness, and residual/error behavior
   explicit?
4. Are ordering and precision behavior deterministic and covered on native
   and WASM?
5. Are all owned buffers/handles released on success, error, and cancellation?
6. Did the required tests run without exclusions?
7. If performance- or contract-sensitive, are benchmark and compatibility
   evidence attached?
8. Does `docs/manifold-changelog.md` record the crate, ABI, TypeScript, and
   migration impact with the correct version boundary?
