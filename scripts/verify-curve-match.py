"""Independent rational endpoint evaluation and angular error at 80 digits."""
from decimal import Decimal as D,getcontext
from pathlib import Path
import sys,json,hashlib
getcontext().prec=80
path=Path(sys.argv[1]);cases=json.loads(path.read_text(),parse_float=lambda x:D.from_float(float(x)))
PI=D('3.141592653589793238462643383279502884197169399375105820974944592307816406286208998628')
def basis(knots,p,n,u):
    p,n=int(p),int(n)
    b=[D(knots[i]<=u<knots[i+1]) for i in range(len(knots)-1)]
    if u==knots[n]:
        b=[D(0)]*len(b);span=n-1
        while knots[span]==u:span-=1
        b[span]=D(1)
    d=[D(0)]*len(b);dd=d[:]
    for order in range(1,p+1):
        nb=[];nd=[];ndd=[]
        for i in range(len(b)-1):
            left,right=knots[i+order]-knots[i],knots[i+order+1]-knots[i+1]
            nb.append(((u-knots[i])*b[i]/left if left else D(0))+((knots[i+order+1]-u)*b[i+1]/right if right else D(0)))
            nd.append((order*b[i]/left if left else D(0))-(order*b[i+1]/right if right else D(0)))
            ndd.append((order*d[i]/left if left else D(0))-(order*d[i+1]/right if right else D(0)))
        b,d,dd=nb,nd,ndd
    return b[:n],d[:n],dd[:n]


def endpoint(curve,end):
    points=curve['controlPoints'];weights=curve['weights'];knots=curve['knots'];p=curve['degree'];n=len(points)
    u=knots[p] if end=='start' else knots[n]
    b,d,_=basis(knots,p,n,u)
    w=sum((x*y for x,y in zip(b,weights)),D(0));dw=sum((x*y for x,y in zip(d,weights)),D(0))
    h=[sum((b[i]*weights[i]*points[i][a] for i in range(n)),D(0)) for a in range(len(points[0]))]
    dh=[sum((d[i]*weights[i]*points[i][a] for i in range(n)),D(0)) for a in range(len(points[0]))]
    return [x/w for x in h],[(dx*w-x*dw)/(w*w) for x,dx in zip(h,dh)]
maximum=D(0);fraction=D(0)
for case in cases:
    result=case['result'];report=result['report']
    pa,a=endpoint(case['reference'],case['referenceEnd']);pb,b=endpoint(result['curve'],case['editedEnd'])
    assert max(abs(x-y) for x,y in zip(pa,pb))<D('1e-60')
    a=[x*(-1 if case['referenceEnd']=='start' else 1) for x in a]
    b=[x*(-1 if case['editedEnd']=='end' else 1) for x in b]
    dot=sum((x*y for x,y in zip(a,b)),D(0));assert dot>0
    wedge=sum(((a[i]*b[j]-a[j]*b[i])**2 for i in range(len(a)) for j in range(i+1,len(a))),D(0)).sqrt()
    sine=wedge/(sum((x*x for x in a),D(0))*sum((x*x for x in b),D(0))).sqrt()
    assert sine<=report['sineAngleUpper']+D('1e-60')
    assert sine<D('0.0001')
    term=sine;angle=sine
    for n in range(20):
        term*=D((2*n+1)**2)/D(2*(n+1)*(2*n+3))*sine*sine;angle+=term
    angle=angle*180/PI
    assert angle<=report['angleDegreesUpper']+D('1e-60')
    assert report['angleDegreesUpper']<=report['maxAngleDegrees']
    maximum=max(maximum,angle);fraction=max(fraction,angle/report['angleDegreesUpper'])
report={'cases':len(cases),'precisionDigits':80,'maxMeasuredAngleDegrees':float(maximum),'maxFractionOfBound':float(fraction),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}
path.with_name('curve-match-independent-report.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
