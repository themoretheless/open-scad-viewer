"""Exact Fraction verification of the opposite-side sufficient criterion.
This proves separation off the common edge for positive rational bases;
refusal cases mean that the criterion is unproven, not that a defect exists.
"""
import json
import sys
from fractions import Fraction as F

def sub(a,b):return [x-y for x,y in zip(a,b)]
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def cross(a,b):return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
def bezier(knots,degree):return len(knots)==2*(degree+1) and knots==[knots[0]]*(degree+1)+[knots[-1]]*(degree+1) and knots[0]<knots[-1]

cases=json.load(open(sys.argv[1]))['cases'];assert len(cases)==72
certified=refused=0
for c in cases:
    edge=c['edge'];assert edge['degree']==2 and bezier(edge['knots'],2) and not edge['periodic']
    ep=[list(map(F,p)) for p in edge['controlPoints']];assert len(ep)==3 and all(len(p)==3 for p in ep)
    normal=cross(sub(ep[1],ep[0]),sub(ep[2],ep[0]));assert any(normal)
    signs=[]
    for s in [c['a'],c['b']]:
        assert not s['periodicU'] and not s['periodicV'] and s['degreeU']==1 and s['degreeV']==2
        assert bezier(s['knotsU'],1) and bezier(s['knotsV'],2)
        assert len(s['controlPoints'])==len(s['weights'])==2
        assert all(len(row)==3 for row in s['controlPoints']+s['weights'])
        assert all(F(w)>0 for row in s['weights'] for w in row)
        assert s['controlPoints'][0]==edge['controlPoints'] and s['weights'][0]==edge['weights']
        pc=c['pcurve'];assert pc['degree']==1 and bezier(pc['knots'],1) and all(F(w)>0 for w in pc['weights'])
        assert pc['controlPoints']==[[s['knotsU'][0],s['knotsV'][0]],[s['knotsU'][0],s['knotsV'][-1]]]
        assert all(dot(normal,sub(p,ep[0]))==0 for p in ep)
        side={dot(normal,sub(list(map(F,p)),ep[0])) for p in s['controlPoints'][1]}
        signs.append({(x>0)-(x<0) for x in side})
    proof=signs in [[{1},{-1}],[{-1},{1}]]
    assert c['certified']==proof and proof!=(c['fold'])
    if proof:certified+=1
    else:refused+=1
print(json.dumps({'cases':len(cases),'exactCertificates':certified,'conservativeRefusals':refused}))
