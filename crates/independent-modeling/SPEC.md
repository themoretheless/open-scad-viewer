# Independent modeling library

Implement geometry in Rust using only the standard library. Do not depend on
the host repository's CAD kernels or on any proprietary geometry runtime.
This workspace is intentionally separate from the surrounding Cargo workspace.

## Behavioral requirements

- Finite Cartesian coordinates, radians, and consistent caller-selected length
  units; invalid and unsupported inputs return typed errors.
- Rational B-spline curves and tensor-product surfaces retain their definitions.
  Evaluation and knot insertion must preserve rational weights.
- Lines, polylines, conics, ruled/lofted/extruded/revolved surfaces.
- Explicitly faceted solids and tessellation with caller-selected resolution.
  Sampling does not certify approximation error or solid validity.
- Mesh construction, affine transforms, incidence queries, area, signed volume,
  ray intersection and OBJ/STL export.
- Boolean operations on finite axis-aligned boxes, including disconnected
  results and empty results. General curved-body booleans remain unimplemented.
- Solid editing must name its supported domain; no faceted result may be
  described as an exact curved B-rep.
- A second version may add an operation/document layer using architectural
  observations, while preserving a frozen first-version source manifest.

## Provenance boundary

The clean package is authored from this behavioral specification, mathematical
definitions and independent tests. No target binary, bytecode, pseudocode,
target API declaration, source path or existing CAD implementation is an input
to that implementation step. Its API is independently designed.

The author has prior exposure to reverse-engineering findings in this chat.
This is therefore a documented independent implementation, not proof of formal
personnel-separated clean-room development. Hashes attest only to source identity
and chronology, not to legal status or mathematical correctness.

Mathematical references (formulas, not source-code imports):

- C.-K. Shene, [de Boor evaluation](https://pages.mtu.edu/~shene/COURSES/cs3621/NOTES/spline/B-spline/de-Boor.html).
- C.-K. Shene, [single knot insertion](https://pages.mtu.edu/~shene/COURSES/cs3621/NOTES/spline/B-spline/single-insertion.html).

## Unimplemented branches

General trimmed B-rep sewing and booleans; analytic variable-radius fillets,
general shell/offset with self-intersection cleanup; constrained surface solving;
general curve/surface intersection completeness; STEP/IGES; certified geometric
error bounds. These are requirements for future development, not existing APIs.
