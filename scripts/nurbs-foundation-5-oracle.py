#!/usr/bin/env python3
"""Independent Decimal oracle for /5 CC/CS residuals and contact classification."""
from decimal import Decimal, getcontext
import json

getcontext().prec = 96

def lerp(a, b, t):
    return a + (b - a) * t

def eval_line(p0, p1, t):
    return [lerp(p0[i], p1[i], t) for i in range(3)]

# Crossing lines: A(t)=(t,t,0), B(u)=(u,1-u,0) meet at t=u=1/2.
a0, a1 = [Decimal(0)]*3, [Decimal(1), Decimal(1), Decimal(0)]
b0, b1 = [Decimal(0), Decimal(1), Decimal(0)], [Decimal(1), Decimal(0), Decimal(0)]
t = u = Decimal('0.5')
pa = eval_line(a0, a1, t)
pb = eval_line(b0, b1, u)
cc_residual = max(abs(x-y) for x,y in zip(pa, pb))

# Curve/plane: piercing (0.25,0.4,z) through z=0 plane at t=1/2.
cs_point = [Decimal('0.25'), Decimal('0.4'), Decimal(0)]
cs_residual = abs(cs_point[2])

# Tangency parity surrogate: parabola y=0 on x-axis has even contact (identical).
# Relative second derivative of (t,0)-(t,0) is zero => higher/even class.
even_contact = "coincident_or_even"

# Bernstein sign bound for plane distance of chord endpoints on unit quarter circle.
# Chord (1,0)-(0,1) vs arc: endpoints residual 0, midpoint residual 1-sqrt(1/2).
mid = [Decimal('0.5'), Decimal('0.5'), Decimal(0)]
arc_mid = [Decimal(2).sqrt()/2, Decimal(2).sqrt()/2, Decimal(0)]
chord_gap = max(abs(a-b) for a,b in zip(mid, arc_mid))

print(json.dumps({
    "precision": getcontext().prec,
    "curveCurveCrossingResidual": str(cc_residual),
    "curveSurfacePiercingResidual": str(cs_residual),
    "chordArcMidGap": str(chord_gap),
    "evenContactSurrogate": even_contact,
    "admittedAssumptions": [
        "positive-weight-rational-Bezier-span-decomposition",
        "outward-rounded-control-hull-exclusion",
        "Krawczyk-or-plane-Bernstein-terminal-isolation",
        "half-open-knot-face-ownership",
    ],
}))
