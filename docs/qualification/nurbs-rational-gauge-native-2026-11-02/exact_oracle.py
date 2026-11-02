"""Independent exact rational-polynomial gauge checks; no Rust imports."""
from fractions import Fraction as F
from math import ldexp
import json

def mul(a,b):
    c=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b): c[i+j]+=x*y
    return c

def bernstein2(values):
    a,b,c=map(F,values)
    return [a,2*(b-a),a-2*b+c]

def residual(weights,changed):
    w=bernstein2(weights); v=bernstein2(changed)
    failures=[]
    for coords in ([0,1,2],[0,1,0]):
        n=bernstein2([F(x)*F(y) for x,y in zip(weights,coords)])
        m=bernstein2([F(x)*F(y) for x,y in zip(changed,coords)])
        failures.extend(x-y for x,y in zip(mul(n,v),mul(m,w)))
    return failures

weights=[.1,.3,.9]
checks={}
for exponent in [-30,0,1,30]:
    checks[f'complete_polynomial_identity_{exponent}']=all(x==0 for x in residual(weights,[ldexp(w,exponent) for w in weights]))
checks['rounded_max_division_detected']=any(x!=0 for x in residual(weights,[w/max(weights) for w in weights]))
# Exact tensor denominator/numerator at (1/2,1/2).
def center(rows):
    basis=[F(1,4),F(1,2),F(1,4)]
    denominator=sum(b*(a+c)/2 for b,(a,c) in zip(basis,rows))
    numerator=sum(b*c/2 for b,(a,c) in zip(basis,rows))
    return numerator/denominator
original=[[F(1),F(1)],[F(2),F(1)],[F(1),F(1)]]
checks['tensor_global_factor_preserves_center']=center(original)==center([[x/2 for x in row] for row in original])==F(2,5)
checks['tensor_row_gauge_changes_center']=center([[x/max(row) for x in row] for row in original])!=center(original)
assert all(checks.values()),checks
print(json.dumps({'checks':checks,'passed':len(checks),'failed':0,'scope':'independent coefficient identities and tensor counterexample; not a Rust execution or whole-workspace proof'},indent=2))
