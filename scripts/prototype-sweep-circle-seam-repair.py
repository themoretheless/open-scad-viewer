"""Bounded offline prototype; does not modify production geometry or certificates."""
import json,sys,math
from fractions import Fraction as F
from pathlib import Path

def canonical_repair(poles,quantum):
    if len(poles)!=9 or poles[0]!=poles[-1]:return None
    center=[(F(a)+F(b))/2 for a,b in zip(poles[0],poles[4])]
    a=[F(x)-c for x,c in zip(poles[0],center)]
    b=[F(x)-c for x,c in zip(poles[2],center)]
    # Quantize shared generators, never individual poles. Integer sums below
    # binary64's exact mantissa limit preserve all canonical affine identities.
    generators=[[round(x/F(quantum)) for x in vector] for vector in [center,a,b]]
    signs=[(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1),(0,-1),(1,-1),(1,0)]
    result=[]
    for x,y in signs:
        integers=[generators[0][k]+x*generators[1][k]+y*generators[2][k] for k in range(3)]
        if any(abs(n)>=2**52 for n in integers):return None
        result.append([float(F(n)*F(quantum)) for n in integers])
    squared=max(sum((F(x)-F(y))**2 for x,y in zip(old,new)) for old,new in zip(poles,result))
    upper=math.nextafter(math.sqrt(float(squared)),math.inf)
    # Verify the reported floating upper bound against the exact squared error.
    while F(upper)**2<squared:upper=math.nextafter(upper,math.inf)
    return result,upper

def self_test():
    poles=[[1.,0.,0.],[1.,1.,0.],[0.,1.,0.],[-1.,1.,0.],[-1.,0.,0.],[-1.,-1.,0.],[0.,-1.,0.],[1.,-1.,0.],[1.,0.,0.]]
    transformed=[[10.+.31*x+.27*y,4.+.19*x-.41*y,3.+.11*x+.17*y] for x,y,_ in poles]
    repaired,upper=canonical_repair(transformed,2**-40)
    assert upper<1e-10
    for j in [0,2,4,6]:
        left=repaired[(j-1)%8];right=repaired[(j+1)%8];shared=repaired[j]
        assert all(F(left[k])+F(right[k])==2*F(shared[k]) for k in range(3))
    assert repaired[0]==repaired[-1]
    assert canonical_repair(transformed,2**-60) is None
    print('Shared generators preserve four exact tangent identities; displacement < 1e-10; mantissa exhaustion refuses')

if __name__=='__main__':self_test()
