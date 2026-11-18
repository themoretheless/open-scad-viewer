"""Independent exact finite-chart and full original-basis projective oracle."""
from fractions import Fraction as F
from itertools import permutations
from pathlib import Path
import copy
import hashlib
import json
import math
import sys


def rational(x):
    # Authored inputs and stored controls are binary64, not exact decimal data.
    return F(float(x))


def determinant(m):
    total=F(0)
    for p in permutations(range(4)):
        inversions=sum(p[i]>p[j] for i in range(4) for j in range(i+1,4))
        term=F((-1)**inversions)
        for i in range(4):term*=m[i][p[i]]
        total+=term
    return total


def homogeneous(g,kind):
    points=g['controlPoints'];weights=g['weights']
    if kind=='surface':
        if len(points)!=len(weights) or any(len(a)!=len(b) for a,b in zip(points,weights)):return None
        points=[p for row in points for p in row];weights=[w for row in weights for w in row]
    if len(points)!=len(weights) or not points:return None
    if any(len(p)!=3 for p in points):return None
    return [[rational(x)*rational(w) for x in p]+[rational(w)] for p,w in zip(points,weights)]


def verify(r):
    try:
        kind=r['kind'];a=r['source'];b=r['output']
        if kind not in ['curve','surface']:return False
        basis=['degree','knots','periodic'] if kind=='curve' else ['degreeU','degreeV','knotsU','knotsV','periodicU','periodicV']
        if any(a[k]!=b[k] for k in basis):return False
        if kind=='surface' and [len(x) for x in a['controlPoints']]!=[len(x) for x in b['controlPoints']]:return False
        matrix=[[rational(x) for x in row] for row in r['matrix']]
        if len(matrix)!=4 or any(len(row)!=4 for row in matrix) or determinant(matrix)==0:return False
        source=homogeneous(a,kind);actual=homogeneous(b,kind)
        if source is None or actual is None or len(source)!=len(actual):return False
        expected=[[sum(m*h for m,h in zip(row,p)) for row in matrix] for p in source]
        den=[p[3] for p in expected]
        if not (all(x>0 for x in den) or all(x<0 for x in den)):return False
        if any(p[3]<=0 for p in actual):return False
        scale=actual[0][3]/expected[0][3]
        return scale!=0 and all(out==[scale*x for x in original] for out,original in zip(actual,expected))
    except (KeyError,TypeError,ValueError,ZeroDivisionError,OverflowError):return False


def rows(path):
    decoder=json.JSONDecoder()
    return sorted([decoder.raw_decode(line.split('exact-projective-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-projective-oracle=' in line],key=lambda r:json.dumps(r,sort_keys=True))


if __name__=='__main__':
    a=rows(sys.argv[1]);assert len(a)==12 and all(verify(r) for r in a)
    curve=next(r for r in a if r['kind']=='curve')
    surface=next(r for r in a if r['kind']=='surface')
    controls=[]
    wrong=copy.deepcopy(curve);wrong['output']['controlPoints'][0][0]=math.nextafter(wrong['output']['controlPoints'][0][0],math.inf);controls.append(wrong)
    wrong=copy.deepcopy(curve);wrong['output']['knots'][0]-=1;controls.append(wrong)
    wrong=copy.deepcopy(curve);wrong['matrix'][0][3]+=1;controls.append(wrong)
    wrong=copy.deepcopy(curve);wrong['matrix'][3]=[0.,0.,0.,0.];controls.append(wrong)
    wrong=copy.deepcopy(curve);wrong['source']['weights'][0]*=2;controls.append(wrong)
    wrong=copy.deepcopy(curve);wrong['output']['weights'][0]*=2;controls.append(wrong)
    wrong=copy.deepcopy(surface);wrong['output']['weights'][1]=[x*2 for x in wrong['output']['weights'][1]];controls.append(wrong)
    wrong=copy.deepcopy(surface);wrong['output']['periodicV']=not wrong['output']['periodicV'];controls.append(wrong)
    assert all(not verify(r) for r in controls)
    if len(sys.argv)>2:assert a==rows(sys.argv[2])
    print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':len(controls),'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),'scope':'Exact permutation determinant, common homogeneous coefficient identity for actual serialized output over unchanged full basis and strictly signed whole-domain denominator. No sampled identity, evaluator-bit, topology, WASM or application claim.'},indent=2))
