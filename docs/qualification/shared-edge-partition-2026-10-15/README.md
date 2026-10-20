# Atomic partition of an owned B-rep edge

`edge_partition::partition` splits one nonperiodic edge and updates every one or
two incident face-local uses in one candidate. Each use supplies its independent
natural pcurve cut parameter. The two children share one vertex evaluated from
the original 3D curve. Inputs are immutable, and no candidate is returned after
an invalid address, mismatching lift, incomplete work or missing identity.

Continuous whole-child checks compare both authored children with the original
curve representation and all resulting pcurves with their original support
surfaces. The reference carrier copies the complete original poles, weights and
knots; it does not substitute a rounded restriction as evidence. Rounded endpoint
authoring is included in these checks. No exact mathematical identity is claimed.

Untouched edge identities are preserved. The two new edges carry explicit
parent-to-children `Split` lineage, and the output must retain complete naming.
The result is a geometric/topological candidate requiring fresh global solid
qualification; closure alone is insufficient.

## Automatic planar roots

`partition_at_plane` runs the native certified curve/plane root query afresh on
the original curve. It requires complete coverage and an isolated transverse
crossing, retains the original root parameter interval and positional enclosure,
and bounds the single authored vertex's error against that enclosure. Native
root isolation and whole-child checks share one work budget. Caller-supplied
root reports do not authorize this operation.

The currently admitted cutting planes are the finite axis-aligned rectangular
charts supported by the native root prover. Natural pcurve cuts proposed from
original traversal must independently pass the whole-child checks.

## Remaining requirements

General surface/surface root and branch selection, nonaffine edge/pcurve mapping
authorship, periodic source curves/pcurves, tangencies, retained-region selection,
fillet end caps, unequal-radius converging corners, global embedding/thickness
qualification, WASM and UI/history admission remain separate work. This module
must not be presented as complete arbitrary B-rep trimming or solid filleting.

## Evidence, 2026-10-15

- Seven `edge_partition` integration tests passed: shared incidence/lineage,
  rational arc with a nonunit original domain, invalid/duplicate/missing pcurve
  cuts and exhausted budget, twenty sequential partitions, a quintic transition
  on an affinely deformed rational patch network, and two automatic plane-root
  cases. The eight existing/new original-boundary correspondence tests also
  passed in the final joint invocation (15 distinct tests total).
- The first midpoint-only representative caused four exact lift mismatches in
  each control cuboid despite tolerance-qualified closure. Root representative
  selection now prefers a simpler binary fraction inside the certified band;
  all root-position and original-domain proofs remain mandatory. A regression
  checks exact lift identity for the two control cuts, without extrapolating
  that exact result to every root or representation.
- `owned_edge_partition_audit` freshly certified the two partitioned 10×8×12 mm
  cuboids at cutting plane x=1 mm and x=5 mm: exact lifts/joins, valid trims,
  embedded boundary and global volume admission all passed. The certified vertex
  error upper bounds were 1.4210854715202016e-14 mm and approximately 5.44e-162 mm.
  These are bounds for the chosen representatives, not exact root coordinates.
- STEP v5 exports independently passed OpenCascade validity, one-solid count,
  bounding boxes and the design volume 960 mm³. Both measured volumes were
  960 mm³; the maximum reported bounding-box discrepancy was 1e-7 mm.

Local generated evidence is in
`output/qualification/shared-edge-partition-2026-10-15/` (not source code).
Reproduce exports with the `owned_edge_partition_audit` native example, then
run `scripts/verify-cad-step-occt.py <output-directory> --no-preview` using the
qualified OCP environment. These fixtures do not replace the bracket/flange/
enclosure, Undo/Redo, reload, general SS-root or WASM/UI acceptance requirements.

### Follow-up: certified curved supports

The subsequent `certified-curve-surface-roots-2026-10-15` qualification adds
`partition_at_surface` for internally certified transverse roots on full
nonperiodic NURBS support domains. It includes a curved-support partition,
fresh global admission and independent STEP checks. General trimmed-face/SS
branch classification and the remaining fillet/UI requirements stay open.
