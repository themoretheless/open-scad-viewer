# Exact affine Solid binding across STEP face and chart changes

The coefficient owner now proves a bijection between every expected original
profile/time patch and every actual face in its owned material shell. Native
face order is only the first search proposal. Every failed coefficient comparison
consumes the same exact-work allowance; no actual face may be matched twice.
Face count, shell coverage and the final used-face set exclude omissions and
duplicates. Material shell IDs/order still follow the native body contract.

For each candidate face the owner checks the eight signed tensor-square
symmetries: optional exchange of U/V, reflection of U, and reflection of V.
Degrees, normalized clamped knots, full control arrays and positive weights
must match **exactly** after the corresponding index permutation. All affine
images are recomputed from the owned source coefficients with the existing
error-free arithmetic checks. No pole, weight, knot or source law is refitted,
rounded or snapped to obtain a match. More general parameter gauges remain
unsupported and refuse.

The actual full rectangular domain is then independently checked from its own
coedges, pcurves and world edges. The actual boundary inspector still visits
every edge and vertex link and proves exact correspondence, regularity and Gk.
The original and actual global volume atlases and original-boundary differential
owner retain their independent same-call obligations.

The determinant sign of the signed parameter permutation is negative exactly
when the number of axis exchanges/reflections is odd. The native binding report
privately retains this proved gauge for each actual face. Its actual face-use
sense must satisfy

```
actual_reversed XOR chart_reversed = source_expected_reversed
```

where the source expectation comes from the owned world volume Jacobian and
original material winding. Comparing the bare sense bit after STEP changed
parameters would reject physically identical boundaries or admit the wrong
orientation. This adjustment uses coefficient-proved gauges, not guessed normals.
The supplied model's geometry and topology remain unchanged by the auditor.

Focused regressions cover face reordering, refusal to reuse one actual face,
all seven nonidentity square gauges with valid UV loop/coedge sense changes,
short work allowances, and sub-tolerance coefficient/weight/domain mutations.
Seven raw native JSON/STEP fixtures cover RMF, authored and guided laws, holes,
reversed material orientation, a nonplanar filled carrier and profiles, and
40 nonbinary time intervals. Export/import is followed by a fresh audit of
the actual imported model's source and retained G2, global volume, topology,
orientation and error bound.

Full frozen native qualification is a new successor; the earlier 3178-test
checkpoint does not qualify these changed coefficients/ownership checks.
This matrix is native STEP, not an independent external STEP-engine check.
Physical end caps, pole quotients, mixed C0/Gk policies and public delivery
qualification remain separate unfinished work. Sharp joins stay C0.
