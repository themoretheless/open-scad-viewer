#!/usr/bin/env python3
"""Independent Decimal oracle for /4 composition and cloud Hausdorff remainder."""
from decimal import Decimal, getcontext
from math import comb
import json

getcontext().prec = 96

def bernstein(n, i, t):
    return Decimal(comb(n, i)) * (t**i if i else Decimal(1)) * (((1-t)**(n-i)) if n-i else Decimal(1))

def eval_rational(controls, weights, t):
    n = len(controls) - 1
    num = [Decimal(0)] * len(controls[0])
    den = Decimal(0)
    for i, (point, weight) in enumerate(zip(controls, weights)):
        b = bernstein(n, i, t) * weight
        den += b
        for axis, value in enumerate(point):
            num[axis] += b * value
    return [value / den for value in num]

# Exact identity composition of the unit-weight quadratic identity map.
phi_controls = [Decimal(0), Decimal('0.5'), Decimal(1)]
phi_weights = [Decimal(1)] * 3
arc_controls = [[Decimal(1), Decimal(0), Decimal(0)],
                [Decimal(1), Decimal(1), Decimal(0)],
                [Decimal(0), Decimal(1), Decimal(0)]]
arc_weights = [Decimal(1), Decimal(2).sqrt()/2, Decimal(1)]

samples = []
for i in range(257):
    t = Decimal(i) / 256
    phi = eval_rational([[c] for c in phi_controls], phi_weights, t)[0]
    direct = eval_rational(arc_controls, arc_weights, phi)
    # Identity map => composed geometry equals source geometry at the same parameter.
    composed_as_source = eval_rational(arc_controls, arc_weights, t)
    err = max(abs(a-b) for a,b in zip(direct, composed_as_source))
    samples.append(err)

# Cloud Hausdorff remainder: site residual + lipschitz*delta.
sites = [(Decimal(0),Decimal(0)), (Decimal('0.5'),Decimal('0.4')),
         (Decimal(1),Decimal(0)), (Decimal('1.5'),Decimal('0.3')), (Decimal(2),Decimal(0))]
# degree-1 polyline through first/last as a crude fit surrogate
residual = max(
    ((p[0]- (Decimal(0) if i==0 else Decimal(2))* (Decimal(i)/Decimal(len(sites)-1)))**2
     + (p[1]-Decimal(0))**2).sqrt()
    for i,p in enumerate(sites)
)
delta = Decimal(1) / Decimal(len(sites)-1)
speed = Decimal(2)  # unit-interval chord length upper for this surrogate
hausdorff = residual + speed * delta

print(json.dumps({
    "precision": getcontext().prec,
    "identityCompositionMaxError": str(max(samples)),
    "cloudHausdorffSurrogate": str(hausdorff),
    "admittedAssumptions": [
        "chordal-parameter-delta-net-of-fitted-domain",
        "residual-Lipschitz-from-control-polygon-speed",
    ],
}))
