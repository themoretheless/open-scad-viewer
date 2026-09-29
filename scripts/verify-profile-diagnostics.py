"""Verify returned segment locations against the represented input using Decimal."""
from decimal import Decimal as D,getcontext
from pathlib import Path
import json,sys,hashlib
getcontext().prec=80
p=Path(sys.argv[1]);cases=json.loads(p.read_text(),parse_float=lambda x:D.from_float(float(x)))
def cross(a,b,c):return (b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])
for case in cases:
    r=case['result'];assert not r['accepted'];d=r['segmentDefect'];assert d['kind'] in ['intersection','overlap']
    segments=d['segments'];assert len(segments)==2
    points=case['points'][:-1]
    for s in segments:
        i=s['index'];assert s['a']==points[i] and s['b']==points[(i+1)%len(points)]
    a,b=segments[0]['a'],segments[0]['b'];c,e=segments[1]['a'],segments[1]['b']
    assert all(max(a[k],b[k])>=min(c[k],e[k]) and max(c[k],e[k])>=min(a[k],b[k]) for k in range(2))
    assert cross(a,b,c)*cross(a,b,e)<=0 and cross(c,e,a)*cross(c,e,b)<=0
    if d['kind']=='overlap':
        assert cross(a,b,c)==cross(a,b,e)==0
        assert any(min(max(a[k],b[k]),max(c[k],e[k]))>max(min(a[k],b[k]),min(c[k],e[k])) for k in range(2))
report={'cases':len(cases),'precisionDigits':80,'locatedSegmentsVerified':True,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}
p.with_name('profile-diagnostics-independent-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
