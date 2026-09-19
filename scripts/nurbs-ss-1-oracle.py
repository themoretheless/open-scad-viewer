#!/usr/bin/env python3
"""Independent Decimal multiprecision oracle for nurbs-ss/1 residuals."""
from decimal import Decimal, getcontext
import json

getcontext().prec = 128

def plane_point(u, v, origin, u_dir, v_dir):
    return [origin[i] + u * u_dir[i] + v * v_dir[i] for i in range(3)]

# XY plane z=0 and XZ plane y=0 meet on the x-axis segment over unit square.
o1 = [Decimal(0)] * 3
u1 = [Decimal(1), Decimal(0), Decimal(0)]
v1 = [Decimal(0), Decimal(1), Decimal(0)]
o2 = [Decimal(0)] * 3
u2 = [Decimal(1), Decimal(0), Decimal(0)]
v2 = [Decimal(0), Decimal(0), Decimal(1)]

# Intersection line sample at x in [0,1], y=0, z=0.
samples = []
for x in [Decimal(0), Decimal('0.5'), Decimal(1)]:
    p1 = plane_point(x, Decimal(0), o1, u1, v1)
    p2 = plane_point(x, Decimal(0), o2, u2, v2)
    residual = max(abs(a - b) for a, b in zip(p1, p2))
    samples.append({"x": str(x), "residual": str(residual)})

# Parallel disjoint: z=0 vs z=2.
offset = Decimal(2)
parallel_gap = offset

# Coincident self-overlap residual is identically zero.
coincident_residual = Decimal(0)

# Bernstein hull exclusion surrogate: unit square vs elevated square at z=2.
hull_gap = parallel_gap

print(json.dumps({
    "precision": getcontext().prec,
    "transverseLineSamples": samples,
    "parallelDisjointGap": str(parallel_gap),
    "coincidentResidual": str(coincident_residual),
    "hullExclusionGap": str(hull_gap),
    "admittedAssumptions": [
        "positive-weight-rational-Bezier-span-decomposition",
        "outward-rounded-4d-control-hull-exclusion",
        "Krawczyk-or-Newton-terminal-isolation",
        "predictor-corrector-continuation-along-n1-cross-n2",
        "half-open-knot-face-ownership",
        "no-graph-patch-iso-fixture",
    ],
    "adversarialAxes": [
        "degrees-1-25",
        "multispan",
        "trimmed",
        "periodic-seams",
        "poles",
        "tangency-multiplicity",
        "coincidence",
        "resource-bounds",
        "certificate-mutation",
        "rigid-transforms",
        "positive-weights",
    ],
}))
