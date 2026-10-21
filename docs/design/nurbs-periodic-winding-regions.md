# Original periodic winding chains to regular UV regions

`brep_core::profile_certificate::regularize_periodic_winding` connects the
original periodic chart proof to exact nonzero-fill region regularization.
The returned immutable `PeriodicWindingRegions` owns original curves,
partitions, chart/anchor/proof limits, the admitted weighted graph, cycle
provenance and material-left region profiles. It does not author 3D faces.

## Complete source and integer-flow admission

The constructor first recomputes `assemble_weighted_periodic_chart` from the
actual original curves and partitions. Caller-authored graphs, hull flags or
winding approvals are not accepted. Original partition/region correspondence,
periodic winding calibration, seam connectors and exact reversed-encoding
cancellation must succeed before graph access.

Before regularization, every original materialized edge ID must occur exactly
once among retained dictionary contributions and cancelled groups. Signs must
be +/-1; signed contribution sums must equal dictionary coefficients, or zero
for cancelled groups. Every dictionary use is checked, and original cycle use
counts must equal each positive integer edge coefficient and total expanded
edge count. Existing 512-edge bounds apply before allocation/iteration.

A repeated closed dictionary word reduces to its primitive word with integer
multiplicity. Cyclic rotations canonicalize by exact lexicographic edge-ID
order; equal cyclic words then merge. Re-expanding the compressed words by
integer coefficients must reproduce the complete dictionary-flow census.
Geometry is copied from the original dictionary; no refitting, knot reflection
or sampled coincidence test supplies authority.

The compressed original wires and integer coefficients go through
`regularize_nonzero_winding`. It removes redundant material-internal borders,
assigns holes/islands and independently checks each retained original NURBS
profile. `cycle_edges()` maps each region input wire to original dictionary
edges. `original_cycles()` identifies every original graph cycle exactly once;
a single repeated walk can contribute multiplicity greater than one.
Original cancellations and [loop,piece] provenance remain in `assembly()`.

`reverify_source()` re-runs the bounded original chart admission, compares the
actual dictionary, cycles, connector roles and all contribution/cancellation
provenance, reconstructs cycle compression and source bindings, then replays
the region proofs. This original chart stage performs proof search;
`regions().verify()` remains the separate search-free UV-region replay.

## Limits and remaining work

This path uses exact original periodic winding transfer. General approximate
source-to-candidate isotopy needs its own combined admission; an approximate
partition is not silently treated as exact. Unique normalized cycles must fit
the region owner's 32-wire/128-source-span/256-proof-piece limits and degree
1–8 requirements. Intersecting or touching distinct normalized wires may still
be unresolved. Actual supporting-surface correspondence, lifted 3D boundary
curves, owned face topology and seam sewing remain separate. NURBS-0173 stays
planned and the catalog remains **195/200**.

## Verification completed 2026-10-16

**39 Rust checks passed**: 30 profile certificate/region/replay unit checks,
4 new periodic-region integration checks and 5 region regularization checks.
Both crates compile for WASM. Tests cover tripled original rational boundaries,
exact opposite cancellation, negative filled winding, duplicated seam-crossing
contours with two preserved chart components, immutable original source
ownership, independent rectangle-membership observations, primitive repeated
walks and rotated cyclic words, full provenance/flow census, invalid dictionary
references, stale source/partition/graph/cycle claims and exhausted budgets.
[Evidence](../qualification/nurbs-periodic-winding-regions-2026-10-15/native-tests.json).

## Tolerance-qualified weighted candidates, 2026-10-16

`nurbs_core::periodic_trim::assemble_approximated_weighted_chart` retains the
same complete source/candidate isotopy, periodic-image separation, anchor-image
avoidance and original winding transfer as the existing approximate path.
Only the final graph decomposition uses balanced weighted flow; the original
public unweighted path still requires its unique angular ownership checks.
No graph is published after incomplete source topology, calibration or flow.

`brep_core::profile_certificate::regularize_approximated_periodic_winding`
recomputes that admission from original curves/partitions, verifies the same
complete integer provenance/cycle census and constructs exact nonzero-filled
candidate UV profiles. The immutable `ApproximatePeriodicWindingRegions` owns
the original source context. `candidate_regions()` names the moved candidate
explicitly: source topology follows the separately verified continuous isotopy,
not pointwise original/candidate boundary equality. No new geometric fitting is
performed during cycle compression or region regularization.

Source replay re-runs isotopy, image separation, anchor avoidance, original
winding query and candidate graph admission. It checks the actual original
anchor integer, both topology completion flags, dictionary/cycle provenance and
region bindings, then replays the exact candidate region proofs. Forging both
stored anchor/query integers cannot replace the fresh original source query.
Search budgets remain independently explicit; replay performs bounded search.

**136 focused Rust checks passed**: 89 core periodic checks, 36 profile owner
unit/replay checks and 11 region integration checks. Both crates compile for
WASM. New cases exercise non-binary64 seam parameters, both contour orientations,
independent candidate winding queries, stale source translation, insufficient
isotopy work, forged original anchor integers and incomplete topology metadata.

Coincident original source groups still require admissible coherent isotopy;
they are not silently accepted when the ordinary embedding proof is unproved.
General contact regularization and owned supporting-surface faces also remain.
NURBS-0173 is still planned; catalog **195/200**.
[Evidence](../qualification/nurbs-periodic-weighted-approx-2026-10-16/native-tests.json).

## Coherent coincident source proof, 2026-10-16

`nurbs_core::periodic_trim::verify_coherent_anchor_winding` now provides a
separate `CoherentAnchorTransfer`. It proves copy-coherent weighted homotopies
without declaring coincident source loops to be an ordinary embedded contour.
Original inputs are grouped only when source basis/control values agree and
positive weights are exactly proportional, and every corresponding candidate
piece has the same parameter function, source domain, translation and tile.
Independent source/candidate global weight scales are permitted. Rational weight
ratios use exact products, not floating division or a geometric tolerance.
Preview hull flags do not establish group membership or approval.

Every original source occurs once in a group, whose first member is the stored
representative. Existing continuous isotopy, periodic-image separation and
anchor avoidance are recomputed for those distinct representative functions.
Because all copies have identical source and candidate position functions,
their full straight-line homotopies coincide for every parameter and time.
Multiplicity therefore remains fixed. The quotient's ordinary `AnchorTransfer`
is retained as a quotient proof, not relabelled as an embedding of all copies.

Original anchor winding is queried on all original sources. Nonuniform group
multiplicities cannot be recovered by scaling one aggregate quotient winding:
a tested nested pair has quotient winding 0 and original winding 1. Quotient
and original classification queries share one max_cells allowance. For inputs
without duplicate groups, the actual original query is reused without spending
or fabricating a second query. An unproved quotient or exhausted remainder
publishes no original winding approval.

**93 core periodic checks passed**, including four new coherent-copy cases;
both crates compile for WASM. New cases cover independent exact global source
and candidate weight scales, ignored preview flags, nonuniform multiplicities,
changed candidate homotopies, failed quotient topology, count mismatch and
exhausted shared query work.

This completes the copy-coherence proof prefix. Its integration into weighted
chart/region construction still needs an explicit typed choice between ordinary
embedding and quotient/coherent evidence; no `AnchorTransfer` flags are forged.
Opposite-orientation/reparameterized aliases, general contacts and owned
supporting-surface faces remain separate. NURBS-0173 stays planned; **195/200**.
[Evidence](../qualification/nurbs-coherent-anchor-2026-10-16/native-tests.json).

## Coherent weighted charts and candidate regions, 2026-10-16

`nurbs_core::periodic_trim::assemble_coherent_weighted_chart` now consumes the
copy-coherence proof directly. `CoherentWeightedChartAssembly` retains the
actual `CoherentAnchorTransfer`; an ordinary full-copy embedding receipt is
never fabricated or substituted. All original candidate pieces still undergo
exact tile/translation checks and actual chart hull containment before graph
publication. Winding calibration uses the full original query, not the
quotient aggregate. Representative-query cells are subtracted before the
backend charges the actual original query and raw-candidate classification,
so all three stages share the caller's classification allowance.

Dictionary merging now admits exactly proportional positive weights when
basis/control values agree. Geometry is retained from an actual original
candidate member. Reverse cancellation uses exactly proportional reversed
weights with the existing exact domain/knot-reflection checks. This matching is
necessary after positive alias merging: cancellation must not depend on which
weight-scaled representative arrived first. Four source-order permutations
independently verify one residual curve and winding 1 for two positive aliases
and one reverse. This dictionary identity is not a proof of reversed whole-source
homotopy coherence; that remains a separate source-admission obligation.

`brep_core::profile_certificate::regularize_coherent_periodic_winding` connects
this genuine coherent source proof to the complete dictionary/cycle census and
exact nonzero candidate-region profiles. The immutable
`CoherentPeriodicWindingRegions` owns all original sources, partitions, proof
limits, group/representative data, weighted graph, cycle provenance and region
certificates. `reverify_source()` recomputes actual original grouping/winding,
checks source graph/provenance and region bindings, and replays the region
proofs. Quotient embedding remains explicitly distinct from the weighted
original source topology.

Tests cover tripled seam-crossing source/candidate functions with independent
global weight scales, two preserved chart components and exact winding 3.
Nonuniform nested multiplicities have quotient winding 0 but original winding
1; the output correctly retains a filled outer instead of a false quotient
hole. Changed copy groups, original winding integers, region multiplicity and
cycle provenance are rejected during replay.

This closes same-parameter coherent-copy chart/UV-region integration. General
reversed/reparameterized source families, inter-wire contact regularization and
owned supporting-surface faces remain open. NURBS-0173 stays planned; catalog
**195/200**.

Verification for this integration: **155 focused Rust checks passed** (95 core,
47 profile owner unit/replay checks and 13 region integration checks), plus WASM
compilation. [Evidence](../qualification/nurbs-coherent-weighted-chart-2026-10-16/native-tests.json).

## Exact affine and reverse source families, 2026-10-16

Coherent grouping now compares normalized rational parameter functions.
Expanded knot coordinates are related by exact sums of eight original binary64
products; control values and proportional positive weights are checked as before.
Both increasing affine changes and reversed orientation are supported without
materializing an affine slope, offset or reflected knot in binary64.

Every corresponding source piece domain must match in normalized original-source
coordinates. Reverse families pair pieces in opposite order and reverse their
normalized local candidate functions. Equal translations/tiles retain the same
lift. These identities imply H_copy(s,tau)=H_representative(phi(s),tau) for the
complete straight-line source/candidate homotopy, where phi is a fixed affine
bijection of source domains. No sampled parameter correspondence supplies proof.
`source_orientations` records each original +/- orientation relative to its
representative and is independently checked during B-rep source replay.

Weighted dictionaries use the same exact normalized-function identity for
positive merging and reverse cancellation. The stored curve still comes from
an actual candidate member; original parameter domains are retained in the
source context. Winding is invariant under increasing reparameterization and
changes sign under reversal. Original winding is still queried on all sources,
not inferred from the quotient aggregate.

**119 focused checks passed**: 98 core periodic integration checks, one exact
normalized-identity unit check, 15 B-rep region integration checks and 5 source
replay checks. Both crates compile for WASM. New cases cover exact negative-domain
reversals of non-binary64 seam cuts, complete cancellation to an empty UV region,
shifted/scaled parameter domains through region construction, signed copy
coherence, rejected non-affine perturbations and forged orientation metadata.
Independent de Boor positions verify reflected functions on parameter ranges
around 1e-200; ordinary products underflow there, while exact comparison still
rejects a one-ULP knot change.

This qualifies affine/reverse families satisfying the complete original-function
and piece-correspondence identities. General nonlinear reparameterization,
inter-wire contacts and owned supporting-surface faces remain separate.
NURBS-0173 stays planned; **195/200**.
[Evidence](../qualification/nurbs-coherent-affine-reverse-2026-10-16/native-tests.json).

## Owned supporting-surface correspondence, 2026-10-16

`bind_periodic_regions_to_surface` constructs an opaque `SurfacePeriodicRegions`
from the original coherent UV sources and an owned surface snapshot. Construction
and `reverify_source` rebuild the source topology and continuous world-error
proof, validate the actual active chart, prove each wrapped C0 seam, and reject
retained generated connectors on nonwrapped axes. A periodic flag is insufficient.
Clamped boundary weights may differ by an exactly proved common positive scale;
one-ULP relative changes are not accepted as identical boundary functions.

World admission selects a UV subdivision target from outward surface derivative
bounds, then independently compares the final outward world error to the caller's
unchanged world tolerance. This target does not modify the candidate geometry.
An insufficient shared deviation budget remains an explicit refusal, including
valid geometry whose correspondence cannot be established within that budget.
The object does not assert surface regularity, injectivity, lifted 3D edges,
sewn topology or owned-face admission. NURBS-0173 remains planned.

## Owned faces on the original supporting surface, 2026-10-16

`assemble_periodic_surface_faces(&SurfacePeriodicRegions, PeriodicFaceLimits)`
now constructs an `OwnedPeriodicFaces` model from every retained candidate
region. Each face keeps the actual original Surface coefficients and parameter
domain. Original source winding, partitions, calibrated seams and world-space
correspondence are replayed before construction. The model contains sheet
shells and keeps original source ownership; it does not manufacture a Body.

Injectivity/rank is proved on the convex UV hull of each retained region using
original knot-cell derivatives. `surface_injectivity::certify_on_refined` builds
complete covers and tests one common Jacobian hull and fixed projection. It
never combines independent local successes into a global claim. Every cover
level shares the rectangle allowance; the configurable depth is0..6. No rounded
surface trim is used as an original reference. A coarse failed cover can refine
without changing geometry or tolerance.

Actual pcurves are lifted to3D and continuously checked against the unchanged
support. Multi-span polynomial pcurves retain their original piecewise paths.
Nonlinear/rational Bezier pcurves compose on one original support knot cell;
rational piece joins include candidate knot/weight/endpoint rounding in the
subsequent original-source check. Affine supports prefer the original pcurve
degree instead of an inflated tensor-composition degree. General nonlinear
pcurves crossing multiple support cells can still be refused.

Only original generated connector provenance permits seam-edge pairing.
Canonical UV endpoints use the exact calibrated seam representatives. Both
face uses must have opposite traversal, and the one owned world curve is
checked on both actual pcurves/supports. Every generated connector needs exactly
one mate. Quotient vertex links must form a connected cycle or boundary path;
coincident endpoints cannot join disconnected fans. A fresh full-model
edge/support audit is required. Unresolved stages return an error, never a
partially qualified model. `verify_model` rebuilds the complete original-source
ownership before accepting saved coefficients/topology; `reverify` also checks
the retained model and error receipt.

The reported boundary error is the outward sum of original UV-to-world source
error and saved edge/support tolerance. Face-local injectivity does not assert
global cross-face spatial embedding; `global_spatial_embedding_certified` is
false. Nonlinear generated connector aliases, ambiguous seam mates, insufficient
projection proofs and exhausted allowances remain explicit refusals.

Tests cover a closed polynomial tube (two faces, four whole spline edges, three
vertices and one shared seam), a rational cylinder with curved world edges, and
a rational torus with unequal U/V domains and two wrapped axes (four faces,
nine edges, six vertices and a four-spoke cyclic corner fan). Independent
cylinder/torus equations check actual edge samples. Changed world poles or
pcurves, collapsed support, partial work and insufficient refinement cannot
inherit qualification.

Original/candidate UV deviation now uses common-parameter rational Bernstein
cross products, including the actual stored translations and weights. Restricted
original homogeneous controls remain outward intervals; neither a rounded trim
nor a sampled residual is proof. The rational-cylinder duplicate-source example
needs six cells per source, with UV error upper3.9413e-15, instead of exhausting
100000 independent-box cells. Tests with genuine nonzero saved UV deviation
still refuse an insufficient world tolerance.

This is progress on NURBS-0173, which stays planned. General nonlinear source
family correspondence and inter-wire contact regularization remain; the native
200 target is still198/200. Evidence is in
`docs/qualification/nurbs-periodic-owned-faces-2026-10-16/native-tests.json`.
