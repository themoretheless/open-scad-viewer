# Original SS singular charts, quotient trim and material ownership

Base: `830835ed4`. The requested general SS / full trim / closed-body / Boolean
objective remains active. This checkpoint supplies further native mechanisms;
it does not claim an end-to-end general NURBS Boolean or full physical census.
The implementation uses original mathematical geometry; no Plasticity code was
copied. Unrelated changes in the primary checkout are preserved separately.

## Implemented native mechanisms

### Singular SS parameter roots

- All 3x3 minors of every generator triple of the source-equivalent Jacobian.
  A fresh exact Groebner basis represents the complete rank-deficient locus.
  Work/minor exhaustion refuses the stratification. This is parameter rank,
  not intrinsic real dimension or world branch count.
- Exact original root nodes with original rational world images and bounded
  coefficient/work validation.
- Projective blow-up charts about rational parameter roots. Original coordinates
  are `c_j + r` and `c_i + r*t_i`. Exact elimination of `1-z*r` saturates the
  transformed ideal, removing exceptional-only components. Four dominant-axis
  inverse maps cover every punctured original parameter cube; no recognition
  of a cone, plane, graph or prescribed contact template chooses the equations.
- Native uniform root graphs of strict transforms, including source-domain
  containment under the exact blowdown. Regular radial and angular clocks are
  supported. Exact node preimages retain cusp endpoints without replacing them
  with fitted poles or sampled vertices.
- Original root-valued world and paired UV geometry now accepts these charts.
  Cross-chart node ports compare exact original node images. Ordinary affine
  endpoint membership explicitly rejects nonlinear blowdown assumptions.

### Quotient trim transitions

- Relative path crossing parity for root wires in periodic UV quotients.
  Exact native periods and port phases enumerate every relevant deck copy.
  Convex interval homotopies preserve the original crossing count; all source
  queries, subdivisions and copies use shared per-call work allowances.
- Noncontractible physical contours can cross a periodic shore without being
  mislabelled a closed planar Jordan loop. A separately qualified material
  reference remains necessary for an absolute material decision.
- Pole and nonlinear-node UV bridges are explicitly outside this classifier.

### Closed bodies and geometric healing

- `closed_solid::QualifiedClosedSolid` owns immutable legacy Model geometry
  only after fresh continuous boundary embedding, complete shell nesting and
  material orientation. A standalone topology/sampled audit cannot construct it.
- Owned point parity has the required closed embedded material precondition.
- Snap/refit healing publishes repaired geometry only after independent solid
  re-admission. A tolerance repair that passes the repair audit but fails
  continuous geometry returns the unchanged original model.
- Even-odd shell repair rebuilds material bodies/cavities from the proved parent
  forest, corrects material orientation and performs fresh final admission.
  Forest, orientation and final-admission stages have explicit separate budgets.
  Failed repair is atomic and retains the original model.

### Boolean material and volume

- Original surface material seeds enclose the exact rational source point.
  A binary point proposes parity; a certified separation ball connects it to
  the source image. Neither rounding nor sampled agreement assigns material.
- Two-sided face-seed selection implements union/intersection/difference/XOR
  and the correct source-surface reversal, including subtraction from operand B.
  This authorizes one seed, not an entire unpartitioned face.
- Independent retained-domain divergence flux over an admitted material body,
  including cavity orientation and holes. Unclassified cells retain a complete
  `[0,1]` mask; exhaustion returns a complete conservative bound when available,
  never a sum with missing faces/children.
- Adaptive interval integration uses a priority queue and a summation tree.
  Exact polynomial Cox-patch flux handles arbitrary polynomial degrees, equal
  positive weights and non-unit knot clocks with shared exact-work/span budgets.
  General varying-weight surfaces retain rational interval integration.
- Original periodic flux is enabled only after exact native seam identity;
  source periodic flags are preserved.

## Follow-up: native implicit BRep storage

`root_model::RootBrep` now stores original SS edge restrictions, exact shared
root vertices and paired face-local UV projections in the generic native BRep
topology. Paired contour/holes constructors reuse literal world-edge indices
between both original source faces. Source/vertex/projection binding and
combinatorial shell incidence are validated. Rounded legacy coedge trims and
unproved collapsed-pole substitutions are rejected. There is no implicit
conversion to the legacy rational-Curve Model or to `QualifiedClosedSolid`.
This supplies storage, not retained-region embedding/material admission.

## Still open — full scope must not be reduced

1. Complete real-root isolation of the singular locus, irrational/algebraic
   nodes, recursive resolution where a first strict transform remains singular,
   and admission of overdetermined local root systems.
2. Build complete physical branch networks from the entire original source
   census and all singular/periodic/pole strata. Prove ownership, nondegeneracy,
   world embedding and valence; remove overlapping chart duplicates exactly.
3. Extend the root BRep storage to mixed natural rational boundaries and
   implicit intersection edges; split real finite faces. Complete arrangement,
   Jordan/embedding checks, holes, seam homology and collapsed-pole UV bridges
   must be established before publishing a retained trim cell.
4. Extend source/winding/contact/body admission to implicit root geometry. The
   new closed-solid owner currently admits the legacy rational-Curve Model;
   root-valued contours must not be coerced to fitted legacy edges.
5. Propagate the proved material seed through complete connected trim cells,
   select all sectors, sew shared edges and preserve topology lineage.
6. Assemble general union/intersection/difference/XOR output bodies and qualify
   their closed material geometry plus independent volume. A face-seed truth
   table or positive volume alone is not a general Boolean result.
7. Bridge/WASM integration and actual host/Worker qualification for these APIs.
   Installed WASM is not updated by this native-source checkpoint.

## Verification

The qualification directory records the final commands and source hashes.
Regressions include original transverse/closed/periodic roots, mixed-axis source
contours, singular crossing directions, cusp arms, native periodic flux,
noncontractible quotient transitions, immutable material admission, rejected
geometric healing, successful source restoration, nested shell role repair,
rational point seeds, all four Boolean face decisions, cavities and an
independent nonplanar face volume. Finite fixtures exercise generic algorithms;
they do not establish universal convergence or completion of the open items.
