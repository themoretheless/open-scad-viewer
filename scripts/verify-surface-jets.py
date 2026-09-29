"""Independent 80-digit tensor-product NURBS jet evaluation of native fixtures."""
from decimal import Decimal as D, getcontext
import json, sys, math, hashlib
from pathlib import Path
getcontext().prec=80
path=Path(sys.argv[1])
cases=json.loads(path.read_text(),parse_float=lambda x:D.from_float(float(x)),parse_int=D)

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

def jets(s,boundary,t):
    p=s['controlPoints'];w=s['weights'];nu,nv=len(p),len(p[0]);pu,pv=int(s['degreeU']),int(s['degreeV'])
    ku,kv=s['knotsU'],s['knotsV'];au,zu,av,zv=ku[pu],ku[nu],kv[pv],kv[nv]
    cross_u=boundary[0]=='u';at_max=boundary.endswith('Max')
    u,v=((zu if at_max else au),av+t*(zv-av)) if cross_u else (au+t*(zu-au),(zv if at_max else av))
    bu=basis(ku,pu,nu,u);bv=basis(kv,pv,nv,v)
    def h(du,dv):
        return [sum((bu[du][i]*bv[dv][j]*w[i][j]*(p[i][j][a] if a<3 else 1) for i in range(nu) for j in range(nv)),D(0)) for a in range(4)]
    h0,hu,hv,huu,huv,hvv=(h(*ij) for ij in [(0,0),(1,0),(0,1),(2,0),(1,1),(0,2)])
    c=[x/h0[3] for x in h0[:3]]
    cu=[(hu[a]-c[a]*hu[3])/h0[3] for a in range(3)]
    cv=[(hv[a]-c[a]*hv[3])/h0[3] for a in range(3)]
    cuu=[(huu[a]-2*cu[a]*hu[3]-c[a]*huu[3])/h0[3] for a in range(3)]
    cvv=[(hvv[a]-2*cv[a]*hv[3]-c[a]*hvv[3])/h0[3] for a in range(3)]
    cuv=[(huv[a]-cu[a]*hv[3]-cv[a]*hu[3]-c[a]*huv[3])/h0[3] for a in range(3)]
    cross=(zu-au if cross_u else zv-av)*(-1 if at_max else 1);along=zv-av if cross_u else zu-au
    return [c,[x*cross for x in (cu if cross_u else cv)],[x*cross*cross for x in (cuu if cross_u else cvv)],[x*cross*along for x in cuv]]

maximum_ratio=D(0);maximum_error=D(0);samples=0
for case in cases:
    result=case['result'];report=result['report'];scale=case['scale'];order=int(case['order'])
    for i in range(33):
        t=D(i)/32;a=jets(case['reference'],case['referenceBoundary'],t);b=jets(result['surface'],case['editedBoundary'],1-t if case.get('reverse') else t)
        for field,key,factor in [(0,'positionUpper',D(1)),(1,'firstDerivativeUpper',-scale)]+([(2,'secondDerivativeUpper',scale*scale),(3,'mixedDerivativeUpper',scale if case.get('reverse') else -scale)] if order==2 else []):
            error=sum(((x-factor*y)**2 for x,y in zip(b[field],a[field])),D(0)).sqrt()
            bound=report['errorBounds'][key]
            assert error<=bound+D('1e-60'),(case['referenceBoundary'],case['editedBoundary'],order,scale,t,key,error,bound)
            if bound:maximum_ratio=max(maximum_ratio,error/bound)
            maximum_error=max(maximum_error,error)
        samples+=1
report={'cases':len(cases),'samples':samples,'precisionDigits':80,'maxMeasuredJetResidual':float(maximum_error),'maxFractionOfReportedBound':float(maximum_ratio),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'oracle':'Independent Decimal Cox-de Boor rational first, second and mixed derivatives'}
path.with_name('surface-jets-independent-report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
