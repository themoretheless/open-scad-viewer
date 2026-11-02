# Trimmed-face principal-curvature inspection

The preceding 14-face candidate repaired one material witness while increasing
local curvature. This staged inspector addresses a declared maximum magnitude
of principal curvature on the entire authored trimmed face. It does not
construct a radius law, repair material, or admit a solid.

## Algorithm

Fresh original-support shape tensors come from `surface_shape_operator`.
A floating Jacobi frame is only a proposed coordinate transformation. Bounds
use interval Q-transpose A Q, and a Gershgorin lower bound for Q-transpose Q
proves the proposed frame invertible. Orthogonality and floating eigenvalues
are never trusted. Rayleigh quotients give rejection lower bounds; the
transformed symmetric row norm divided by the Gram lower bound gives a
conservative spectral upper bound. No rank-two or Frobenius-norm shortcut
substitutes for the principal magnitude.

Hints may reject only after fresh Inside classification and a certified lower
bound above the declared limit. Hints cannot admit. Positive admission needs
a complete interval cover of the trimmed parameter domain, with Outside
cells omitted only by whole-region classification. Closed UV loops and their
containment in the covered natural support are required, preventing a
vacuous pass for out-of-domain loops. Non-C2 internal representations,
exhausted budgets, missing regularity and unresolved arithmetic remain
unresolved. The geometry and source model stay immutable.

## File access recovery

On resumption the former Cargo process was absent. Existing manifests and source
files were readable again; 15 explicit reads succeeded (see
`filesystem-recovery.json`). The earlier stall was in file opening before
compilation, not in a numerical CAD test. The exact external OS cause has not
been established; no permission reset, checkout replacement or reboot was used.
The public module export is now confirmed by native compilation.

## Continuous extrusion reduction

An axis is reduced only for degree-one, two-layer supports whose rational
weights match along that axis and whose Cartesian displacements are exactly
equal as binary rationals. The world curvature tensor is then translation
invariant. Shape derivatives are still evaluated on the original 2D support;
only the interval query is collapsed along the proven invariant axis. The
cover retains the entire original trimmed domain. A near extrusion is not
eligible. This avoids needless axial subdivisions on the rational cylinder
without relaxing its curvature bound or increasing its work budget.

## Scope

Native regression and saved-candidate results are recorded in `status.json`.
A complete cover is needed for `Within`; an Inside witness with a certified
lower bound above the declared maximum suffices for `Exceeds`. Any unresolved
coverage stays `Unresolved`. The declared magnitude limit is not a varying
fillet radius law. Material containment, full-body self-intersections, end
transitions, STEP/history acceptance, WASM and UI remain separate open gates.
The rejected 14-face construction has not been repaired by this diagnostic.

## Observed result, 2026-10-30

Seven native integration tests passed, including both rational extrusion axes,
a near-extrusion negative control, out-of-support loops, a hint inside a hole,
and zero/external domain budgets. Two native spectral arithmetic tests passed.
The public diagnostic example built successfully. A local compilation blocker
in atlas hashing was corrected by explicit two-digit byte hex formatting.

On face 13 of the saved 14-face model, the automatic run (no supplied UV hint)
found an Inside witness after one cover cell. For the explicitly declared
10/mm threshold, the certified magnitude interval was
[214.96570207578517, 214.96570208561997]/mm. A fresh independent OCCT STEP run
measured 214.96570207896414/mm at the same point, inside that interval.
Both the hinted and automatic reports say `Exceeds`. This is successful defect
detection, not successful geometry construction or a full-face maximum.
