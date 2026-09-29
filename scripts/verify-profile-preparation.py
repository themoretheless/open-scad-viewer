"""Independent Decimal checks of native cycle assembly and explicit connectors."""
from decimal import Decimal as D, getcontext
from collections import Counter
from pathlib import Path
import json,sys,hashlib
getcontext().prec=80
path=Path(sys.argv[1]);cases=json.loads(path.read_text(),parse_float=lambda s:D.from_float(float(s)))
def edge(a,b):return tuple(sorted((tuple(a),tuple(b))))
def cross(a,b,c):return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
max_gap=D(0)
for case in cases:
    r=case['result'];assert r['accepted'];p=r['points'];assert len(p)>=3
    expected=Counter(edge(a,b) for chain in case['chains'] for a,b in zip(chain,chain[1:]))
    for connector in r['connectors']:
        a,b=connector['a'],connector['b'];gap=sum(((x-y)**2 for x,y in zip(a,b)),D(0)).sqrt()
        assert gap<=case['tolerance'];max_gap=max(max_gap,gap);expected[edge(a,b)]+=1
    actual=Counter(edge(a,b) for a,b in zip(p,p[1:]+p[:1]));assert actual==expected
    assert all(a!=b for a,b in actual)
    area=sum((a[0]*b[1]-a[1]*b[0] for a,b in zip(p,p[1:]+p[:1])),D(0))/2
    assert area!=0
    for i,a in enumerate(p):
        b=p[(i+1)%len(p)]
        for j in range(i+2,len(p)):
            if i==0 and j==len(p)-1:continue
            c,d=p[j],p[(j+1)%len(p)]
            if any(max(a[k],b[k])<min(c[k],d[k]) or max(c[k],d[k])<min(a[k],b[k]) for k in range(2)):continue
            assert not(cross(a,b,c)*cross(a,b,d)<=0 and cross(c,d,a)*cross(c,d,b)<=0)
report={'cases':len(cases),'precisionDigits':80,'maxConnectorLength':float(max_gap),'inputSegmentsPreserved':True,'simpleClosedContours':True,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
path.with_name('profile-independent-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
