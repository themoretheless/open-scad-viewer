# Manifold tolerance and welding policy

The manifold API has two independent tolerances. They solve different
problems and must not be used as substitutes for each other.

| Setting | Owner | Units | Controls |
| --- | --- | --- | --- |
| `epsilon` / `weld_epsilon` / `weldEpsilonMm` | `manifold-core` repair and manifold-ops pre/post repair | Absolute mesh units (millimetres in the application) | Whether distinct vertex records can share identity during repair. |
| `relative_tolerance` / `relativeTolerance` | `polygon-core` BSP through `manifold-ops` | Fraction of the longest combined input-bound side | Geometric classification, contact/intersection predicates, and numeric acceptance during a boolean. |

Defaults are deliberately conservative: weld epsilon `0.0`, relative boolean
tolerance `1e-9`.

## Start from valid, measured input

Before choosing a tolerance:

1. Validate buffer lengths, finite coordinates, and index ranges.
2. Establish the mesh unit and a meaningful local coordinate scale.
3. Run `check` and inspect defect locations.
4. When geometric validity is not already known, run self-intersection
   analysis separately.
5. Measure the noise/seam discrepancy and the smallest intended feature.
   Do not choose epsilon only because a larger value makes an error disappear.

A tolerance cannot repair malformed buffers, infer missing geometry, or make
an intersecting triangle soup a valid solid.

## Weld epsilon: exact by default

### Use `epsilon = 0` when

- duplicated vertices come from the same deterministic tessellation/export
  path and should have bitwise-identical coordinates;
- small features, thin walls, or close-but-distinct vertices must be
  preserved;
- establishing a correctness baseline before any approximate repair;
- running reproducibility, native/WASM, or differential tests;
- no measured coordinate noise justifies approximation.

Core exact welding compares the bit patterns of all three coordinates. It
keeps the first representative and does not average positions. Consequently,
positive and negative zero are distinct to this particular exact-weld path.
External input still must be finite.

### Use a positive epsilon only for measured approximate seams

A positive epsilon is appropriate when independent geometry stages produce
slightly different coordinates for vertices that are intended to be
identical, and the discrepancy is known to be bounded.

The implementation:

- hashes positions into grid cells of side epsilon;
- compares a point only with retained representatives in the same cell;
- merges when Euclidean distance is `<= epsilon`;
- keeps the first representative's original coordinates rather than averaging
  or snapping to the grid.

Therefore this is deliberately **not** a promise that every pair within
epsilon will merge. Points across a cell boundary can remain separate. It is
also not a nearest-neighbor clustering or general mesh-simplification API.

Core repair clamps a negative epsilon to zero. The raw ABI rejects a
non-finite repair epsilon. Boolean options reject non-finite weld epsilon and
values outside `0..=1`; use finite, nonnegative values everywhere for a
consistent contract.

There is no universal nonzero epsilon recommendation. Derive it from measured
noise and intended feature size, start with the smallest plausible value, and
validate every increase.

## What welding can change

Welding changes indexed identity, not just a diagnostic report. It can:

- reduce vertex count and remap every triangle corner;
- collapse triangles to repeated corners, which repair then removes;
- close intended seams or remove small features;
- merge shells and change component count, genus, surface area, or volume;
- create or expose non-manifold edges and disconnected vertex fans;
- change which contacts and intersections a subsequent boolean sees.

Full repair can split some resulting non-manifold structures and fill
supported simple boundaries, but it cannot restore geometric detail already
collapsed by welding. It also cannot remove self-intersections.

After any positive-epsilon repair, compare more than `is_manifold()`:

- residual defect lists;
- triangle/vertex/component counts;
- signed volume and surface area;
- genus where meaningful;
- minimum intended feature dimensions and wall thickness;
- downstream boolean volume, topology, and error behavior.

## Weld inside boolean operations

`manifold-ops` does not unconditionally weld every operand. It first checks
each input and applies Full pre-repair with `weld_epsilon` only when strict
manifold admission fails. After the BSP operation it permits one Conservative
seam-repair pass, also using that epsilon.

Consequences:

- setting a positive weld epsilon has no pre-repair effect on an operand that
  already passes strict `check`;
- it can affect both operands when either needs repair;
- it can also affect the output seam pass;
- `operand_repairs` records whether pre-repair occurred;
- a positive weld is part of the geometric operation, not a harmless I/O
  normalization.

Booleans require closed, consistently outward-oriented solid inputs after
admission. Core repair unifies winding but does not promise positive signed
volume; correct inward shells before invoking the BSP operation.

## Relative boolean tolerance

The BSP normalizes both operands around their combined bounds. If `L` is the
longest side of the combined input bounding box, the reported absolute
classification tolerance is:

```text
toleranceMm = relativeTolerance × L
```

For example, with `L = 100 mm` and the default `1e-9`, the absolute tolerance
is `1e-7 mm`. Changing the scale or the bounds of either operand changes this
absolute value even when the relative option stays constant.

The accepted option range is `1e-12..=1e-5`; the default is `1e-9`.

### Keep or lower relative tolerance when

- operands contain small but valid triangles or close contact features;
- classification must remain strict relative to model size;
- a larger tolerance would erase intended separation or make an input
  triangle smaller than the boolean tolerance.

Lowering tolerance can expose floating-point noise, increase work, or turn
near-contact cases into failures. It is not automatically more robust.

### Increase relative tolerance only with evidence

A larger relative tolerance can be appropriate when geometric predicates
need a wider, measured contact classification at the current model scale.
It can also:

- classify close features as touching or overlapping;
- reject triangles that are small relative to the normalized tolerance;
- change intersections, output topology, volume, and work/fragment counts;
- hide a scale or input-generation problem behind a successful-looking
  operation.

Do not increase it to bypass `UnrepairableInput`, non-simple boundaries, or
self-intersections. Those require topology/geometry repair first.

## f32 and f64 inputs

TypeScript f32 positions are widened exactly from their already-quantized
float32 values. F64 positions do not make an f32 round trip. Exact welding
therefore sees different coordinates when f32 quantization changed them.

Do not expect f32 and f64 paths to produce identical work counters or every
same triangle splitting decision at tolerance boundaries. Require equivalent
valid topology and metric agreement within the environment-matrix tolerance;
use f64 for the highest-precision input path.

## Scaling models

Prefer a sensible local origin and model scale over an extreme tolerance.
When applying a uniform scale factor `s` to geometry:

- multiply absolute weld epsilon by `|s|` to preserve its geometric meaning;
- relative tolerance is dimensionless, but normalization, source precision,
  and feature size still require requalification;
- record the scale conversion at the import/export boundary instead of
  applying unrelated ad-hoc tolerances at each operation.

Very large translations can lose local precision before a boolean starts.
Centering geometry near a local origin is often more appropriate than
increasing tolerance.

## Required change procedure

When a case fails, change one thing at a time:

1. Preserve the original operands, options, and failing error code.
2. Establish the `epsilon = 0`, `relativeTolerance = 1e-9` baseline.
3. Fix malformed or self-intersecting input before tolerance tuning.
4. If measured seam noise is the issue, try the smallest justified weld
   epsilon and inspect its geometric/topological effects.
5. If measured BSP classification scale is the issue, adjust relative
   tolerance within range, independently of weld.
6. Re-run check, metrics, self-intersection validation, and the boolean.
7. Compare native/WASM and f32/f64 behavior and run differential cases.
8. Record the chosen values and their geometric justification with the case.

A green `isManifold` flag alone is not sufficient evidence that a tolerance
change preserved the intended model.
