"""Independent exact oracle: expand Bernstein polynomials into the power basis."""
import json
import sys
from fractions import Fraction as F
from math import comb


def polynomial(control, axis):
    n = len(control)-1
    result = [F(0)]*(n+1)
    for i, row in enumerate(control):
        value = F(row[3]) * (F(row[axis]) if axis < 3 else 1)
        for j in range(n-i+1):
            result[i+j] += value*comb(n,i)*comb(n-i,j)*(-1)**j
    return result


def multiply(a,b):
    out = [F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):
            out[i+j] += x*y
    return out


def verify(c):
    a,b = c['a'],c['b']
    assert all(len(p)==4 and F(p[3])>0 for p in a+b)
    wa,wb = polynomial(a,3),polynomial(b,3)
    equal = all(multiply(polynomial(a,k),wb)==multiply(polynomial(b,k),wa) for k in range(3))
    assert c['outcome']==('equal' if equal else 'different')
    return equal


cases=json.load(open(sys.argv[1]))['cases']
assert len(cases)==146
results=[verify(c) for c in cases]
# Reject a forged decision and a changed source control, independently.
import copy
for mutation in ['decision','control']:
    c=copy.deepcopy(cases[0])
    if mutation=='decision': c['outcome']='different'
    else: c['b'][1][0]+=0.125
    try: verify(c)
    except AssertionError: pass
    else: raise AssertionError('mutation accepted: '+mutation)
print(json.dumps({'cases':len(cases),'equal':sum(results),'different':len(cases)-sum(results),'rejectedMutations':2}))
