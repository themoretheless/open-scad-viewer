# Central circular-blend chart: projection investigation

Tested both orientations of the current exported quarter-rim prototype.
The central constant-radius chart (face 5) remains unresolved by the existing
global projection-contraction proof.

An additional fixed linear projection was selected from control-net secants:
the U endpoint difference at the middle V control and the V endpoint difference
at the middle U control. Admission used a single interval Jacobian hull over
the complete chart, not sampled Jacobians or independent local certificates.

Two implementations were tried: projection of existing derivative intervals,
and projection of the homogeneous control net before derivative enclosure.
The latter used a constant affine denominator and was tested at 4×4 and 16×16
subdivision. Neither produced a contraction bound below one for face 5 in
either orientation. Existing injectivity tests passed during the experiment,
including folded-chart rejection and the sphere shared-budget regression.

The unsuccessful extra projection and subdivision branch were removed. The
retained diagnostics and production algorithm remain unchanged. This result
does not establish a self-intersection. It rules out treating this particular
generic projection fallback as a completed proof.

Next investigation: a structure-aware criterion that certifies the exact
rational angular and meridian factors, positive radius, and monotonic angular
coordinates. Such a criterion must verify the supplied control net rather
than trust constructor metadata, handle intended boundary poles separately,
and reject perturbed or folded surfaces. Pairwise contact qualification is
still required independently.
