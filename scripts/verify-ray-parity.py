"""Independent analytic ray counts for boxes, a sphere and a through-hole."""
from fractions import Fraction as F
from pathlib import Path
import json,hashlib,sys
source=Path(sys.argv[1]).read_bytes();data=json.loads(source);checked=0;unresolved=0;odd=0
for row in data['queries']:
    p=list(map(F,row['point']));d=list(map(F,row['direction']));kind=row['model']
    if kind==1:
        a=sum(x*x for x in d);b=2*sum(x*y for x,y in zip(p,d));c=sum(x*x for x in p)-4;disc=b*b-4*a*c
        assert c!=0 and disc!=0
        inside=c<0
        count=0 if disc<0 else 1 if inside else 2 if b<0 else 0
    else:
        if kind==0:
            planes=[[F(0),F(1)]]*3
            def material(q):return all(0<x<1 for x in q)
        else:
            planes=[[F(0),F(4),F(6),F(10)],[F(0),F(4),F(6),F(10)],[F(0),F(1)]]
            def material(q):return 0<q[0]<10 and 0<q[1]<10 and 0<q[2]<1 and not(4<=q[0]<=6 and 4<=q[1]<=6)
        inside=material(p);events=sorted(set((x-p[k])/d[k] for k in range(3) for x in planes[k] if (x-p[k])/d[k]>0));count=0;prior=inside
        for i,t in enumerate(events):
            after=(t+events[i+1])/2 if i+1<len(events) else t+1
            state=material([p[k]+after*d[k] for k in range(3)])
            count+=state!=prior;prior=state
    if row['parity'] is None:
        assert row['unresolved']>0;unresolved+=1;continue
    assert row['unresolved']==0
    assert row['parity']==inside,(kind,list(map(float,p)),row['parity'],inside)
    assert len(row['crossings'])==count,(kind,list(map(float,p)),len(row['crossings']),count)
    checked+=1;odd+=inside
assert checked==len(data['queries']),(checked,unresolved)
print(json.dumps({'sourceSha256':hashlib.sha256(source).hexdigest(),'queries':len(data['queries']),'classified':checked,'unresolved':unresolved,'odd':odd,'even':checked-odd,'maxCells':max(q['cells'] for q in data['queries']),'arithmetic':'exact rational analytic parity and crossing counts','checks':'passed'},indent=2))
