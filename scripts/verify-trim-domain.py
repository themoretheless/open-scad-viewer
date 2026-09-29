"""Independent UV winding check using exact rational Bernstein ray roots.

The native classifier proves a homotopy using outward curve hulls. This checker
instead isolates ray roots with Bernstein sign variation and exact Fraction
subdivision. The fixtures are clamped rational Bezier loops with no query on
an endpoint ray; unsupported/ambiguous fixtures fail explicitly.
"""
from fractions import Fraction as Q
from pathlib import Path
import json,sys,hashlib
path=Path(sys.argv[1]);cases=json.loads(path.read_text(),parse_float=lambda x:Q.from_float(float(x)))
def q(x):return x if isinstance(x,Q) else Q(x)
def split(net):
    rows=[net]
    while len(rows[-1])>1:
        old=rows[-1];rows.append([[(a+b)/2 for a,b in zip(old[i],old[i+1])] for i in range(len(old)-1)])
    return [row[0] for row in rows],[row[-1] for row in reversed(rows)]
def sign(x):return (x>0)-(x<0)
def variations(values):
    signs=[sign(x) for x in values if x]
    return sum(a!=b for a,b in zip(signs,signs[1:]))
def midpoint_value(values):
    while len(values)>1:values=[(a+b)/2 for a,b in zip(values,values[1:])]
    return values[0]
def exact_crossing(values):
    # An odd root crosses; an even root only touches the horizontal ray.
    for order in range(1,len(values)):
        values=[b-a for a,b in zip(values,values[1:])]
        value=midpoint_value(values)
        if value:return sign(value) if order%2 else 0
    raise AssertionError('Fixture curve lies on the ray')
def winding(curve,point):
    p=curve['degree'];cp=curve['controlPoints'];knots=curve['knots']
    assert len(cp)==p+1 and len(set(knots[:p+1]))==len(set(knots[p+1:]))==1
    net=[[q(w)*(q(v[0])-point[0]),q(w)*(q(v[1])-point[1])] for v,w in zip(cp,curve['weights'])]
    assert net[0][1] and net[-1][1], 'Fixture query shares endpoint ray'
    stack=[(net,0)];total=0
    while stack:
        net,depth=stack.pop();assert depth<256,'Unresolved exact ray root'
        ys=[v[1] for v in net];variation=variations(ys)
        if not variation:continue
        xs=[v[0] for v in net]
        if max(xs)<0:continue
        if variation==1 and min(xs)>0:
            total+=sign(next(y for y in reversed(ys) if y));continue
        left,right=split(net)
        if left[-1][1]==0:
            assert left[-1][0]!=0,'Fixture query is on curve'
            if left[-1][0]>0:total+=exact_crossing(ys)
        stack.extend([(left,depth+1),(right,depth+1)])
    return total
checked=0;max_native_cells=0;counts={'inside':0,'outside':0}
for case in cases:
    for query in case['queries']:
        point=list(map(q,query['point']))
        expected=sum(winding(c,point) for loop in case['loops'] for c in loop)
        report=query['report'];location='inside' if expected else 'outside'
        assert report['location']==location and report['winding']==expected,(case['name'],point,expected,report)
        checked+=1;counts[location]+=1;max_native_cells=max(max_native_cells,report['cells'])
print(json.dumps({'sourceSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'arithmetic':'exact rational Bernstein ray roots','cases':len(cases),'queries':checked,'locations':counts,'maxNativeCells':max_native_cells,'checks':'passed'},indent=2))
