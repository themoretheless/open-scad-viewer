"""Independent Fraction signed-volume check for the two planar cap fixtures.
Checks the exported triangulation, not arbitrary NURBS volume certification.
"""
import copy
import json
import sys
from fractions import Fraction as F


def volume(mesh):
    p = list(map(F, mesh['positions']))
    ids = mesh['indices']
    assert len(ids) % 3 == 0 and len(p) % 3 == 0
    total = F(0)
    for i in range(0, len(ids), 3):
        a,b,c = [p[3*j:3*j+3] for j in ids[i:i+3]]
        total += a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0])
    return total/6


def verify(case):
    base,area = {'bracket': (6500,325), 'enclosure': (7152,264)}[case['name']]
    request=case['request']; assert request['amount']==1
    assert case['response']['ok']
    original=request['body']; result=case['response']['value']
    assert original['id']==result['id']
    tolerance=F(1,100000)
    assert abs(volume(original['mesh'])-base)<tolerance
    assert abs(volume(result['mesh'])-(base+area))<tolerance
    before=[tuple(v['point']) for v in original['brep']['vertices']]
    after=[tuple(v['point']) for v in result['brep']['vertices']]
    assert max(p[2] for p in before)==20 and max(p[2] for p in after)==21
    assert all(p in after for p in before if p[2]<20)
    return {'name':case['name'],'before':float(volume(original['mesh'])),'after':float(volume(result['mesh']))}


all_cases=json.load(open(sys.argv[1]))['cases']
cases=[c for c in all_cases if c['name'] in {'bracket','enclosure'}]
assert {c['name'] for c in cases}=={'bracket','enclosure'}
results=[verify(c) for c in cases]
for case in cases:
    bad=copy.deepcopy(case)
    bad['response']['value']['mesh']['positions']=[v*2 for v in bad['response']['value']['mesh']['positions']]
    try: verify(bad)
    except AssertionError: pass
    else: raise AssertionError('Scaled geometry accepted')
print(json.dumps({'cases':results,'rejectedMutations':len(cases)}))
