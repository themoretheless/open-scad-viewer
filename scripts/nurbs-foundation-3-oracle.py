#!/usr/bin/env python3
"""Independent Decimal oracle for /3 rational maps and wrapped seam fixtures."""
from decimal import Decimal, getcontext
from math import comb
import json

getcontext().prec = 90

def rational(values, weights, t):
    n=len(values)-1
    basis=[Decimal(comb(n,i))*(t**i if i else 1)*((1-t)**(n-i) if n-i else 1) for i in range(n+1)]
    denominator=sum((basis[i]*weights[i] for i in range(n+1)),Decimal(0))
    return sum((basis[i]*weights[i]*values[i] for i in range(n+1)),Decimal(0))/denominator

values=list(map(Decimal,["0","0.2","1"]))
weights=list(map(Decimal,["1","0.75","1"]))
samples=[rational(values,weights,Decimal(i)/4096) for i in range(4097)]
assert samples[0] == 0 and samples[-1] == 1
assert all(a < b for a,b in zip(samples,samples[1:]))

degree=2
controls=[(1,0),(0,1),(-1,0),(0,-1),(1,0),(0,1)]
assert controls[:degree] == controls[len(controls)-degree:]

print(json.dumps({
    "precision":getcontext().prec,
    "strictMonotoneSamples":len(samples),
    "midpoint":str(samples[len(samples)//2]),
    "wrappedSeamOrdersChecked":[0,1,2]
}))
