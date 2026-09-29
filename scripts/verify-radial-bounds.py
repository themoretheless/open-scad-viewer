"""Independent exact rational Bernstein evaluation of radius enclosures."""
import hashlib,json,math,sys
from fractions import Fraction as F
from pathlib import Path
source=Path(sys.argv[1]).read_bytes()
cases=json.loads(source)['cases']
queries=0
for case in cases:
    s=case['surface'];p=s['degreeU'];q=s['degreeV']
    origin=list(map(F,case['origin']));lo,hi=map(F,case['bounds'])
    assert 0<=lo<=hi
    def basis(n,t):return [math.comb(n,i)*t**i*(1-t)**(n-i) for i in range(n+1)]
    for iu in range(11):
        for iv in range(11):
            bu,bv=basis(p,F(iu,10)),basis(q,F(iv,10))
            numerator=[F(0)]*3;denominator=F(0)
            for i in range(p+1):
                for j in range(q+1):
                    weight=bu[i]*bv[j]*F(s['weights'][i][j]);denominator+=weight
                    for k in range(3):numerator[k]+=weight*F(s['controlPoints'][i][j][k])
            r2=sum((numerator[k]/denominator-origin[k])**2 for k in range(3))
            assert lo*lo<=r2<=hi*hi,(p,q,iu,iv,case['bounds'],float(r2))
            queries+=1
print(json.dumps({'sourceSha256':hashlib.sha256(source).hexdigest(),'cases':len(cases),'queries':queries,'arithmetic':'exact rational tensor Bernstein evaluation','checks':'passed'},indent=2))
