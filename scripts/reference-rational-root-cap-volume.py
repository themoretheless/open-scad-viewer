#!/usr/bin/env python3
"""Actual-coefficient rational control quadrature; not an interval certificate."""
import hashlib
import json
import sys
from decimal import Decimal as D, localcontext
from pathlib import Path
root=Path(sys.argv[1])
s=json.loads((root/'cutter.json').read_text())['faces'][0]['surface']
assert s['degreeU']==s['degreeV']==2 and s['knotsU']==s['knotsV']==[0,0,0,1,1,1]
cp=s['controlPoints'];weights=s['weights']
assert all(p[:2]==[-1+i,-1+j] for i,row in enumerate(cp) for j,p in enumerate(row))
assert weights[0][0]>1 and all(weights[i][j]==1 for i in range(3) for j in range(3) if (i,j)!=(0,0))
source=json.loads((root/'source.json').read_text())
assert {tuple(v['point']) for v in source['vertices']}=={(x,y,z) for x in [-.5,.5] for y in [-.5,.5] for z in [-.05,.2]}
with localcontext() as ctx:
    ctx.prec=55
    exact=lambda x:D.from_float(float(x))
    z=[[exact(p[2]) for p in row] for row in cp]
    delta=exact(weights[0][0])-1;top=exact(.2);bottom=exact(-.05)
    def site(x,v):
        # Original weighted coordinate equation X(u,v)=x solved algebraically.
        t=(1-x)/(1+(1+delta*(1-v)**2*(1-x*x)).sqrt())
        u=1-t;a=[t*t,2*u*t,u*u];b=[(1-v)**2,2*v*(1-v),v*v]
        r=delta*a[0]*b[0];w=1+r
        nx=2*u-1-r;ny=2*v-1-r
        h=(sum(a[i]*b[j]*z[i][j] for i in range(3) for j in range(3))+r*z[0][0])/w
        ru=-2*delta*t*(1-v)**2;rv=-2*delta*t*t*(1-v)
        xu=(2*w-ru*(w+nx))/(w*w);xv=-rv*(w+nx)/(w*w)
        yu=-ru*(w+ny)/(w*w);yv=(2*w-rv*(w+ny))/(w*w)
        assert xu>0
        return h,ny/w,yv-yu*xv/xu
    def crossing(x,lo,hi):
        flo=site(x,lo)[0]-top;fhi=site(x,hi)[0]-top
        assert flo*fhi<0
        for _ in range(120):
            mid=(lo+hi)/2;fm=site(x,mid)[0]-top
            if fm==0:return mid
            if (fm>0)==(flo>0):lo=mid;flo=fm
            else:hi=mid
        return (lo+hi)/2
    def simpson(f,a,b,n):
        step=(b-a)/n;total=f(a)+f(b)
        for i in range(1,n):total+=(4 if i%2 else 2)*f(a+step*i)
        return total*step/3
    def removed_slice(x,n):
        lo=crossing(x,D(0),D('.5'));hi=crossing(x,D('.5'),D(1))
        assert D('-.5')<site(x,lo)[1]<site(x,hi)[1]<D('.5')
        def integrand(v):
            h,y,dy=site(x,v);assert h>bottom and dy>0
            return (top-h)*dy
        return simpson(integrand,lo,hi,n)
    values=[]
    for n in [64,128,256]:
        removed=simpson(lambda x:removed_slice(x,n),D('-.5'),D('.5'),n)
        values.append((n,top-bottom-removed))
    differences=[abs(values[i][1]-values[i-1][1]) for i in range(1,len(values))]
    assert differences[-1]<D('1e-11')
    result=dict(expectedChangedVolumeMm3=str(values[-1][1]),quadrature=[dict(intervals=n,volumeMm3=str(v)) for n,v in values],
                successiveDifferenceMm3=list(map(str,differences)),
                cutterSha256=hashlib.sha256((root/'cutter.json').read_bytes()).hexdigest(),
                scope='55-digit actual original rational control coefficients; algebraic X inverse, bracketed height roots, tensor Simpson convergence; reference only, not interval/global volume certification')
(root/'expected-rational-cap-volume.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
