"""Independent Cox-de Boor / Fraction proof of actual Rust conic outputs."""
import ast
import copy
import hashlib
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

source = Path('docs/qualification/nurbs-exact-knot-insertion-2026-11-02/exact_oracle.py')
tree = ast.parse(source.read_text())
tree.body = [n for n in tree.body if isinstance(n, (ast.Import, ast.ImportFrom, ast.FunctionDef))]
scope = {}
exec(compile(tree, str(source), 'exec'), scope)
add, mul, channels = (scope[k] for k in ('add', 'mul', 'channels'))

def scale(p, s):
    return [x*s for x in p]

def dot(a, b):
    return sum(x*y for x, y in zip(a, b))

def value(p, t):
    return sum(c*t**i for i, c in enumerate(p))

def derivative(p):
    return [i*x for i, x in enumerate(p)][1:]

def verify(row):
    c = row['curve']
    if c['degree'] != 2 or c['knots'] != [0.,0.,0.,.25,.25,.5,.5,.75,.75,1.,1.,1.]:
        return False
    if len(c['weights']) != 9 or any(w <= 0 for w in c['weights']):
        return False
    origin, u, v = (list(map(F, row[k])) for k in ('center', 'axisU', 'axisV'))
    aa, ab, bb = dot(u,u), dot(u,v), dot(v,v)
    det = aa*bb-ab*ab
    if det <= 0:
        return False
    normal = [u[i]*v[j]-u[j]*v[i] for i,j in [(1,2),(2,0),(0,1)]]
    cardinal = [(1,0),(0,1),(-1,0),(0,-1),(1,0)]
    for index in range(4):
        lo, hi = F(index,4), F(index+1,4)
        *h, w = channels(c,lo,hi)
        t = [add(h[i],scale(w,-origin[i])) for i in range(3)]
        plane = [F(0)]
        at, bt = [F(0)], [F(0)]
        for i in range(3):
            plane = add(plane,scale(t[i],normal[i]))
            at, bt = add(at,scale(t[i],u[i])), add(bt,scale(t[i],v[i]))
        if any(plane):
            return False
        x, y, d = add(scale(at,bb),scale(bt,-ab)), add(scale(bt,aa),scale(at,-ab)), scale(w,det)
        if any(add(add(mul(x,x),mul(y,y)),scale(mul(d,d),-1))):
            return False
        for endpoint, target in [(lo,cardinal[index]),(hi,cardinal[index+1])]:
            if value(d,endpoint) <= 0:
                return False
            if (value(x,endpoint),value(y,endpoint)) != tuple(z*value(d,endpoint) for z in target):
                return False
        # Convert the angular polynomial from global power t to local s in [0,1]
        # and then to Bernstein coefficients; strict positivity proves winding.
        angular = add(mul(x,derivative(y)),scale(mul(y,derivative(x)),-1))
        local = [F(0)]*len(angular)
        for j,a in enumerate(angular):
            for k in range(j+1):
                local[k] += a*math.comb(j,k)*lo**(j-k)*(hi-lo)**k
        degree = len(local)-1
        bernstein = [sum(local[k]*F(math.comb(i,k),math.comb(degree,k)) for k in range(i+1)) for i in range(degree+1)]
        if any(x <= 0 for x in bernstein):
            return False
    return True

def rows(path):
    decoder = json.JSONDecoder()
    result = [decoder.raw_decode(line.split('exact-conic-oracle=',1)[1])[0]
              for line in Path(path).read_text().splitlines() if 'exact-conic-oracle=' in line]
    return sorted(result,key=lambda r: json.dumps(r,sort_keys=True))

if __name__ == '__main__':
    a,b = rows(sys.argv[1]),rows(sys.argv[2])
    assert a == b and len(a) == 4
    assert all(verify(r) for r in a)
    r = next(r for r in a if r['center'] == [0.,0.,0.] and r['axisV'] == [0.,1.,0.])
    wrong = copy.deepcopy(r)
    wrong['curve']['controlPoints'][1][0] = math.nextafter(1.,math.inf)
    assert not verify(wrong)
    wrong = copy.deepcopy(r); wrong['curve']['weights'][1] = math.sqrt(.5)
    assert not verify(wrong)
    wrong = copy.deepcopy(r); wrong['curve']['knots'][3:5] = [.2,.2]
    assert not verify(wrong)
    wrong = copy.deepcopy(r); wrong['axisU'][0] = 2.
    assert not verify(wrong)
    scaled = copy.deepcopy(r); scaled['curve']['weights'] = [2*w for w in scaled['curve']['weights']]
    assert verify(scaled)
    print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':4,'globalGaugeControl':True,
        'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),
        'basisOracleSha256':hashlib.sha256(source.read_bytes()).hexdigest(),
        'scope':'Exact Fraction Cox-de Boor polynomial identity on every actual knot cell, plane, cardinal endpoints, positive angular numerator. Native geometry only; no sampling, evaluator-bit or application claim.'},indent=2))
