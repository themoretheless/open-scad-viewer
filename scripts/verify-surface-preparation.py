"""Independent 80-digit evaluation of basis preparation fixtures."""
from decimal import Decimal as D,getcontext
import json,sys,hashlib
from pathlib import Path
getcontext().prec=80
path=Path(sys.argv[1])
cases=json.loads(path.read_text(),parse_float=lambda x:D.from_float(float(x)))
def basis(knots,degree,count,t):
    degree,count=int(degree),int(count)
    values=[D(knots[i]<=t<knots[i+1]) for i in range(len(knots)-1)]
    if t==knots[count]:
        values=[D(0)]*(len(knots)-1)
        span=count-1
        while knots[span]==t:
            span-=1
        values[span]=D(1)
    for p in range(1,int(degree)+1):
        result=[]
        for i in range(len(values)-1):
            a,b=knots[i+p]-knots[i],knots[i+p+1]-knots[i+1]
            result.append(((t-knots[i])/a*values[i] if a else D(0))
                          +((knots[i+p+1]-t)/b*values[i+1] if b else D(0)))
        values=result
    return values[:int(count)]

def evaluate(surface,u,v):
    points,weights=surface['controlPoints'],surface['weights']
    bu=basis(surface['knotsU'],surface['degreeU'],len(points),u)
    bv=basis(surface['knotsV'],surface['degreeV'],len(points[0]),v)
    h=[D(0)]*4
    for i,row in enumerate(points):
        for j,p in enumerate(row):
            w=bu[i]*bv[j]*weights[i][j]
            for k in range(3):
                h[k]+=w*p[k]
            h[3]+=w
    assert h[3]>0
    return [x/h[3] for x in h[:3]]


maximum=D(0);fraction=D(0);samples=0
for case in cases:
    source,target=case['source'],case['target']
    ku,kv=source['knotsU'],source['knotsV']
    au,zu=ku[source['degreeU']],ku[len(source['controlPoints'])]
    av,zv=kv[source['degreeV']],kv[len(source['controlPoints'][0])]
    for i in range(33):
        for j in range(33):
            u=D(au)+(D(zu)-D(au))*D(i)/32;t=D(j)/32
            v=D(av)+(D(zv)-D(av))*(1-t if case['reverse'] else t)
            tu,tv=target['knotsU'],target['knotsV']
            ta,tz=tu[target['degreeU']],tu[len(target['controlPoints'])]
            va,vz=tv[target['degreeV']],tv[len(target['controlPoints'][0])]
            a,b=evaluate(source,u,v),evaluate(target,D(ta)+(D(tz)-D(ta))*D(i)/32,D(va)+(D(vz)-D(va))*t)
            error=sum(((x-y)**2 for x,y in zip(a,b)),D(0)).sqrt()
            assert error<=case['bound']+D('1e-60'),(error,case['bound'])
            maximum=max(maximum,error);fraction=max(fraction,error/case['bound'] if case['bound'] else D(0));samples+=1
report={'cases':len(cases),'samples':samples,'precisionDigits':80,'maxMeasuredDeviation':float(maximum),'maxFractionOfBound':float(fraction),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
path.with_name('preparation-independent-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
