# Manifold error catalog

This catalog covers failures from manifold transport validation, repair
options, boolean admission, the polygon BSP kernel, and the TypeScript WASM
boundary. Messages add context; branch on structured variants or stable
`code` values, not on message text.

For API semantics, see `docs/manifold-user-guide.md`.

## Results that are not errors

- `check` returns a `ManifoldReport`. `is_manifold() == false` is a diagnosis,
  not an exception.
- `repair` and `repair_with_mode` return a `RepairOutcome`. Inspect
  `report.residual` or `is_manifold()`; an unsuccessful repair does not throw.
- In TypeScript, the corresponding flags are `isManifold` and
  `isManifoldWithBoundary`. A repair call can return normally while strict
  manifoldness remains false.
- `metrics` can measure open or defective meshes. `watertight == false` and
  `genus == None/null` are metric results, not transport failures.

## Rust structural validation

Call `validate_indexed_mesh` for untrusted native input before `check`,
`repair`, `metrics`, or `boolean_manifold`. It returns these
`IndexedMeshError` variants:

| Variant | Cause | Recommended action |
| --- | --- | --- |
| `PositionsLength { len }` | The position buffer length is not divisible by 3. | Rebuild flat xyz triples; do not pad silently. |
| `IndicesLength { len }` | The index buffer length is not divisible by 3. | Triangulate polygons and rebuild complete triangle corners. |
| `NonFiniteCoordinate { index }` | A coordinate is NaN or infinite. | Find the producer, remove non-finite math, and regenerate the mesh. |
| `IndexOutOfRange { index, vertex_count }` | An index references a missing vertex. | Fix index remapping, buffer truncation, or vertex/index pairing. |

The WASM boundary performs this validation automatically and maps it to
`GEOMETRY_INVALID_INPUT`.

## Rust boolean failures

`boolean_manifold` and `boolean_manifold_with_options` return
`Result<BooleanOutcome, BooleanError>`:

| Variant | Meaning | Recommended action |
| --- | --- | --- |
| `UnrepairableInput { operand, residual }` | Operand 0 or 1 remains non-manifold after Full pre-repair. | Inspect the residual edges/vertices/triangles for that operand. Fix non-simple boundaries, split or rebuild bad shells, and run geometric self-intersection analysis where needed. |
| `Kernel(math_core::Error)` | The BSP kernel rejected input/options, exceeded a budget, hit a numeric failure, or rejected its own result. | Branch on `error.code` using the kernel table below. Preserve the message and stage for diagnosis. |
| `NonManifoldOutput { residual }` | The polygon call returned, but the manifold-ops post-check and conservative seam repair could not obtain strict manifold output. | Treat as a kernel/integration defect. Preserve both operands, options, residual, and a reproducible seed/case; do not return the mesh as a successful result. |

Example dispatch:

```rust
use manifold_ops::BooleanError;

match result {
    Ok(out) => assert!(out.output_report.is_manifold()),
    Err(BooleanError::UnrepairableInput { operand, residual }) => {
        eprintln!("operand {operand}: {residual:?}");
    }
    Err(BooleanError::Kernel(error)) => {
        eprintln!("kernel {}: {}", error.code, error.message);
    }
    Err(BooleanError::NonManifoldOutput { residual }) => {
        eprintln!("kernel output defect: {residual:?}");
    }
}
```

## TypeScript and raw ABI errors

The synchronous wrappers throw `GeometryKernelError` with `code` and
`message`:

```ts
import {GeometryKernelError} from './services/geometry/kernel'

try {
  const out = booleanManifoldInKernel(ap, ai, bp, bi, 'union')
  // Consume out.positions/out.indices only here.
} catch (error) {
  if (error instanceof GeometryKernelError) {
    console.error(error.code, error.message)
  } else {
    throw error
  }
}
```

### Bridge-level codes

| Code | Manifold causes | Recommended action |
| --- | --- | --- |
| `GEOMETRY_INVALID_INPUT` | Invalid position/index triples; non-finite coordinate; out-of-range index; unknown raw vertex format; null nonempty raw buffer; non-finite repair epsilon; invalid raw repair mode or boolean operation; boolean weld/tolerance outside their ranges; an operand remains unrepairable after Full repair. | For transport fields, correct the buffer and regenerate it. For options, use finite values and documented ranges. For an unrepairable operand, use the operand number and native residual/report to repair topology; do not merely increase tolerance. |
| `GEOMETRY_RESOURCE_LIMIT` | Input allocation or encoded response exceeds the geometry transport limit. | Reduce mesh size or split the workflow into bounded parts. Keep position/index buffers paired. Do not retry the same payload in a tight loop. |
| `GEOMETRY_KERNEL` | The manifold-ops post-check rejected the polygon result and conservative seam repair failed (`NonManifoldOutput`). | Treat as a kernel/integration defect and capture a minimal reproduction. Do not downgrade this to a warning or consume partial output. |

Raw pointer lifetime violations are unsafe-programming errors, not recoverable
mesh errors. Use the provided TypeScript wrappers instead of calling raw ABI
from application code.

## Polygon BSP codes passed through `BooleanError::Kernel`

These codes reach TypeScript unchanged because the bridge preserves
`BooleanError::Kernel` rather than replacing its code.

| Code | Cause | Recommended action |
| --- | --- | --- |
| `POLYGON_INVALID_INPUT` | General polygon mesh validation or indexed-point access failed: malformed buffers, unsupported resource shape, or another polygon precondition. | Validate the native mesh first, inspect the message, and correct the producer. If core validation passes but this persists, reduce to a minimal operand pair for a kernel issue. |
| `POLYGON_BOOLEAN_INVALID_SOLID` | The operand is not a valid BSP solid: intersecting/overlapping/unstitched triangles at the active tolerance, orientation/winding defects, or another solid-validation failure. | Run `check` and geometric self-intersection analysis. Stitch/weld only appropriate seams, rebuild overlapping shells, and correct winding. Full repair alone does not remove geometric intersections. |
| `POLYGON_BOOLEAN_INVALID_OPTIONS` | Polygon tolerance or work/fragment/output limits are invalid. The manifold facade validates its exposed tolerance and uses valid default budgets, so this should not occur through normal facade options. | With direct polygon use, correct all option ranges. Through `manifold-ops`, treat this as a defaults/integration regression and report it rather than masking it with arbitrary tolerances. |
| `POLYGON_BOOLEAN_RESOURCE_LIMIT` | Work, fragment, or output-triangle budget exhausted during input validation, clipping, BSP construction, stitching, or result validation. | Reduce triangle count and unnecessary fragmentation, simplify inputs before the boolean, or split a large modeling operation into bounded stages. The manifold facade does not expose unlimited budgets. |
| `POLYGON_BOOLEAN_NUMERIC_ERROR` | Bounds cannot be normalized, a plane/predicate becomes numerically invalid, or world-coordinate precision cannot represent the result. | Remove non-finite/degenerate source geometry, normalize extreme coordinates near a local origin, avoid collapsed solids, and review scale/tolerance. Persistent valid-input cases need a minimal numeric reproduction. |
| `POLYGON_BOOLEAN_NON_MANIFOLD_RESULT` | The BSP kernel's own result validation rejected topology or geometric intersections at the active tolerance. | Check input validity and tolerance first. If operands are valid, preserve them and report a kernel defect. This is distinct from `GEOMETRY_KERNEL`, which comes from the later manifold-ops post-check. |

## Choosing a recovery strategy

1. **Malformed transport:** fix buffers and indexing. Repair cannot make an
   invalid memory layout meaningful.
2. **Topological defects:** inspect `ManifoldReport`, choose Conservative or
   Full repair deliberately, then inspect the residual.
3. **Geometric intersections:** run intersection analysis and rebuild the
   offending shells; increasing weld epsilon is not a general solution.
4. **Resource limit:** reduce or partition work. Retrying identical input does
   not increase the budget.
5. **Numeric failure:** inspect scale, finite values, degeneracy, and local
   coordinate origin before adjusting tolerance.
6. **Kernel/output defect:** preserve a minimal reproduction and report it.
   Never substitute an unchecked mesh for the failed guaranteed-manifold
   result.
