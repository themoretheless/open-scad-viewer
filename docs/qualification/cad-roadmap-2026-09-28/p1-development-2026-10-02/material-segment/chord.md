# Original boundary endpoint admission — 2026-10-02

`material_chord::inspect` qualifies a material chord between two certified
transverse intersections of an authored finite parametric line with original
trimmed B-rep faces. Endpoints are original face UV enclosures and line root
parameter enclosures; rounded evaluated points are not treated as exact endpoints.

## Preconditions and proof

1. The boundary embedding, nesting ownership and material orientation must pass
   the existing volume validity audit.
2. The authored origin must be proven outside material.
3. Complete finite line coverage must have no unresolved roots or trims and
   exactly two isolated, strictly interior, nonoverlapping crossings.
4. Strict interval Krawczyk inclusion supplies transverse roots: the projected
   surface Jacobian is invertible. On an embedded closed material boundary,
   these crossings change material membership. Starting outside, the first
   enters material and the second leaves it. Complete coverage excludes an
   additional hidden boundary or cavity between them.
5. Original surface rectangle enclosures bound the endpoints. Their outward
   enclosure distance bounds the chord length.

Reports retain both crossing face indices, UV and parameter intervals, endpoint
enclosures, validity evidence and stage work. Unproven reports have no chord
length or endpoint enclosure result.

## Native evidence

- Cuboid chord in both directions encloses 10 mm with width below 1e-6 mm.
- A cuboid cavity yields four crossings and refuses a single material chord.
- Inside seed and endpoint on the finite line boundary remain unqualified.
- Canonical annular wall chord at y=2,z=3 encloses
  `sqrt(400-4)-sqrt(25-4)` mm with width below 1e-5 mm.
- A full line through the annular hole yields four crossings and is refused.
- The focused native material tests passed: 13 tests, zero failures.
- Both bridge dispatch tests passed, including source/configuration preservation,
  nullable unproven outputs, work exhaustion and invalid budgets.
- The broader bridge run passed 350 tests, ignored one existing gate and failed
  one old tessellation expectation: sphere detail 32 produces 25,924 triangles
  and correctly refuses the 20,000-triangle limit. The test now explicitly
  requires that refusal while retaining topology assertions for admitted levels.
  Its focused rerun passed. A complete rerun after this test-only correction
  has not been performed.

## API

`cad_material_segment` and `cad_material_chord` accept explicit original `model`,
`origin`, `direction`, `toleranceUv` and stage `limits`. The shared validity
configuration uses the existing solid-distance budget contract. Results bind
the original model, coefficients, tolerance and limits.

WASM and worker/UI qualification are pending. Normal alignment, automatic
opposing region selection and global minimum wall thickness remain open.
The generic chord result is not a minimum thickness certificate.
