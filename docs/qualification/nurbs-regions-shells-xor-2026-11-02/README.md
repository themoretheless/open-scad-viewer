# NURBS rational regions, shell forests and decomposed XOR — 2026-11-02

Status: bounded proof-backed extensions in all five requested areas. This does
NOT close universal algebraic NURBS Boolean. General singular/tangent SSI and
closed components inside one original face pair remain unproved.

## Implemented

1. **Rational branches and degree 8.** The automatic proposer now searches
   polynomial and positive rational Bernstein traces through degree 8. Interior
   original-fiber boxes support bounded exact-rational continued-fraction
   reconstruction, including non-dyadic samples. Exact polynomial degree
   reduction avoids rejecting an affine pcurve just because elevated i/5 poles
   are not binary64. Interpolation is ONLY proposal generation: saved 3D/pcurve
   identities against BOTH original carriers still authorize admission.
   A rational free clock uses an exact Bernstein sign proof of N'W - NW',
   replacing the previous equal-weight-only sufficient gate.
2. **A checked boundary-contact sufficient case.** Positive whole-carrier hulls
   may prove separating closed coordinate halfspaces with a ZERO-width band.
   Fresh shell forests and exact original edge/carrier identities establish
   proper bounded materials. Their open interiors are disjoint even when the
   boundaries touch/coincide. Regularized intersection is empty; regularized
   difference is unchanged. This is connected to standard Boolean for those
   operations; it does not grant general tangency/branch topology or union
   face-cancellation authority.
3. **Whole UV arrangements through hole rims.** A DCEL proposal now uses ALL
   original outer/hole rim restrictions and BOTH directions of every complete
   graph arc simultaneously. Exact rational endpoint jets determine cyclic ray
   order; ambiguous tangent rays refuse. Halfedge successors must be a complete
   permutation. Positive cycles and material-left hole ownership are proposed,
   then every face receives a fresh exact profile/lift/coverage proof. Original
   chain cancellation still proves full 2D coverage with multiplicity exactly
   one. Single-path sequential cutting and source-outer-rim-only ownership no
   longer limit these admitted arrangements.
4. **Multiple bodies, cavities, nested islands.** Each original shell has its
   own embedded source atlas and exact signed nominal volume/connectedness.
   Every cross-shell pair must separate throughout the ENTIRE two homotopies
   before exact nominal winding can establish original containment. The full
   containment relation must be laminar, transitive and acyclic, with unique
   immediate containers. Root signs are positive and signs alternate through
   nesting. Positive shells own bodies; immediate negative children are their
   cavities; deeper positive islands are independent bodies. Input body-role
   claims are verified against this forest. Result roles are reconstructed from
   the proof rather than inferred from connectivity alone.
5. **Four-use XOR edges.** Checked material regions partition XOR into A\B and
   B\A. Each part independently passes ordinary-Model assembly, shell nesting
   and global source-atlas embedding. Their open material interiors are disjoint
   by the source truth tables. Topology-only concatenation keeps separately
   owned vertices/edges, so each saved edge has TWO uses, not four. Closure
   contact between parts is permitted. The receipt explicitly stores both parts'
   atlases/forests; it asserts NO single globally embedded boundary atlas for
   their touching join. The returned ordinary Model is a decomposed body set.

`RetainedNurbsBooleanCertificate` is now version **3**. Non-XOR results retain
both result_atlas and result_shells. XOR uses xor_decomposition with its two
separately checked parts; a nonempty XOR's result_atlas/result_shells can be None
because a touching body set has different proof obligations from one embedded
boundary. Verification reconstructs the whole operation from the original
Models; supplied flags, source-role claims, interpolation residuals and body
connectivity are never accepted as proof.

Public additional APIs:

- `certify_nurbs_shell_forest` / `verify_nurbs_shell_forest`.
- `certify_nurbs_boundary_only_boolean` / `verify_nurbs_boundary_only_boolean`.
- Existing `author_retained_nurbs_boolean` and its explicit-partition verifier
  use the new arrangement, material oracle and result author automatically.

## Positive and adversarial checks

- Three-level nesting: positive outer shell, negative cavity, positive island;
  source material in the cavity is VOID. Incorrect cavity role declarations
  and forged containment matrices refuse. Multiple source bodies survive.
- A square annular prism is created directly with SAVED original hole loops,
  not by a Boolean that pre-splits its caps. A slab cuts between its outer and
  hole rims. The full original graph, new regions and result pass reconstruction.
- A genuine rational branch reconstructed from non-dyadic interior values has
  saved weights [3,2,1], exact identities on a plane and a DIFFERENT rational
  strip, and a checked monotone rational clock. A reversed control direction
  fails the sufficient clock test.
- Independent curved solids touching at x=1 have a zero-width separating band,
  empty regularized intersection and unchanged difference. Positive overlap
  and a forged support band do not receive that certificate.
- The independent curved layered example still passes intersection, union,
  both differences and standard dispatch. XOR yields THREE separately owned
  bodies and no four-use saved edge; full certificate reconstruction passes.

## Still open

- Arbitrary algebraic/nonparametrizable branches and endpoints not exactly
  representable in the unchanged original-UV binary64 Model. Endpoint proposals
  still require bounded dyadic values. Interior rational reconstruction is NOT
  an algebraic endpoint representation or a checked approximation pathway.
- General tangencies, sheet coincidences, singularities, nonlinear branch nodes
  and closed components wholly inside one original carrier pair. The new
  support-halfspace proof handles only an explicitly sufficient material case.
- Original physical-graph coverage still requires its admitted common free
  clock/component and injectivity proofs. A DCEL does not eliminate that gate.
- Positive rational proposal search is bounded (degree 8, sample and arithmetic
  limits); some exactly representable curves can still refuse. Exact lift and
  topology proofs are never replaced by a numeric fit.
- XOR closure contact is represented as separately proved bodies. A subsequent
  operation requiring a globally embedded source boundary may still refuse that
  joined body set; no such global atlas is forged.
- Shell nesting uses sufficient joint-homotopy separation; unresolved separation
  returns unproved. Source/result atlas, graph, trim and arithmetic stage budgets
  remain bounded; these are not a universal single global CPU allowance.
- No WASM/dist, browser, STEP, full workspace or performance qualification.

## Reproducibility and external gates

Final native builds use `/private/tmp/osv-nurbs-next-source-20261102`, a frozen
copy of the current Rust workspace plus compile-time fixtures and toolchain.
The source manifest and archive retain all build inputs used by the selected
brep-core dependency closure. `missing-literal-fixtures.txt` records THREE
pre-existing absent literal references in modelgraph-runtime; that crate is not
part of the selected build closure. This is not a full-workspace build claim.
Dependencies/compiler downloads are not vendored. Test executables are copied
and hashed before execution. Core opt-level 0/dependencies 1 with debug assertions
provides functional qualification, not a release benchmark.

The current shared-checkout API-catalog check reports an unrelated unassigned
public module `original_graph_transition` (see api-map.log). That module/catalog
work is outside this change and has not been silently reassigned here.

Intermediate diagnostic logs retain an initial test-fixture issue (orthogonal
CSG had already split the purported hole face) and a corrected test-only call to
an unavailable operations::translate helper. Final tests use direct saved-hole
extrusion and an explicit exact binary test-fixture translation.

## Final native results

**126 tests passed, 0 failed**, from the frozen source and copied executables.

| Group | Passed |
| --- | ---: |
| retained | 6 |
| graph | 4 |
| face-crossings | 7 |
| seam-boolean | 5 |
| graph-box | 5 |
| periodic-bounds | 1 |
| operations | 25 |
| nurbs-ss | 46 |
| boundary | 14 |
| nonintersecting | 6 |
| boundary-only | 2 |
| shell-forest | 3 |
| rational | 2 |

All 4515 frozen inputs match the manifest after testing; there are no
unmanifested files. All 16 complete task-owned live files match the snapshot.
All eight executable hashes were rechecked. The source archive is
11,210,291 bytes. The focused hole-rim development run is not
double-counted in the total.

The current shared checkout also passes the standard-dispatch integration test
(`live-standard.log`: 1 passed, 0 failed); this is additional to the 126 frozen-
source tests. Scoped Rust formatting and diff whitespace checks pass. The API
catalog gate still reports the unrelated `original_graph_transition` module.

A shell-forest certificate concerns the filled original carrier shells and
material ownership. Exact agreement of saved 3D edge curves remains a separate
Boolean obligation: the retained-region checker and boundary-only constructor
perform that check explicitly.
