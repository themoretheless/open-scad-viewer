#!/usr/bin/env python3
"""Independent Decimal oracle for bounded rational Bézier projection fixtures."""
import json
import sys
from decimal import Decimal, getcontext

getcontext().prec = 80

def bernstein(n, i, t):
    from math import comb
    power = lambda value, exponent: Decimal(1) if exponent == 0 else value**exponent
    return Decimal(comb(n, i)) * power(t, i) * power(Decimal(1)-t, n-i)

def evaluate(curve, t):
    degree = curve["degree"]
    weights = [Decimal(str(value)) for value in curve["weights"]]
    controls = [[Decimal(str(value)) for value in point] for point in curve["controlPoints"]]
    basis = [bernstein(degree, i, t) for i in range(degree+1)]
    denominator = sum((basis[i]*weights[i] for i in range(degree+1)), Decimal(0))
    return [sum((basis[i]*weights[i]*controls[i][axis] for i in range(degree+1)), Decimal(0))/denominator
            for axis in range(len(controls[0]))]

def main():
    request = json.load(sys.stdin)
    curve, query = request["curve"], [Decimal(str(value)) for value in request["point"]]
    samples = 4096
    best = min(range(samples+1), key=lambda i: sum((a-b)**2 for a,b in zip(
        evaluate(curve, Decimal(i)/samples), query)))
    lo, hi = Decimal(max(0,best-1))/samples, Decimal(min(samples,best+1))/samples
    ratio = (Decimal(5).sqrt()-1)/2
    for _ in range(220):
        c, d = hi-ratio*(hi-lo), lo+ratio*(hi-lo)
        fc = sum((a-b)**2 for a,b in zip(evaluate(curve,c),query))
        fd = sum((a-b)**2 for a,b in zip(evaluate(curve,d),query))
        if fc < fd: hi=d
        else: lo=c
    parameter=(lo+hi)/2
    point=evaluate(curve,parameter)
    distance=sum((a-b)**2 for a,b in zip(point,query)).sqrt()
    json.dump({"precision":getcontext().prec,"parameter":str(parameter),
               "point":[str(value) for value in point],"distance":str(distance)},sys.stdout)

if __name__ == "__main__":
    main()
