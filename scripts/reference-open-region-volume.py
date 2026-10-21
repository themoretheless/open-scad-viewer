#!/usr/bin/env python3
"""Independent control-family volume quadrature from actual authored coefficients.

Convergence evidence supplements a future STEP measurement; this is neither
an interval integration certificate nor a general B-rep volume algorithm.
"""
import hashlib,json,sys
from decimal import Decimal as D,localcontext
from pathlib import Path
root=Path(sys.argv[1]);source=json.loads((root/'source.json').read_text());cutter=json.loads((root/'cutter.json').read_text())
s=cutter['faces'][0]['surface'];cp=s['controlPoints']
assert s['degreeU']==s['degreeV']==2 and s['knotsU']==s['knotsV']==[0,0,0,1,1,1]
assert all(w==1 for row in s['weights'] for w in row)
assert all(p[:2]==[-1+i,-1+j] for i,row in enumerate(cp) for j,p in enumerate(row))
points=[v['point'] for v in source['vertices']]
low=[min(p[k] for p in points) for k in range(3)];high=[max(p[k] for p in points) for k in range(3)]
assert low==[-.5,-.5,-.05] and high==[.5,.5,.2]
assert len(points)==8 and {tuple(p) for p in points}=={(x,y,z) for x in [low[0],high[0]] for y in [low[1],high[1]] for z in [low[2],high[2]]}
assert len(source['faces'])==6
owned=set()
with localcontext() as ctx:
    ctx.prec=65;exact=lambda x:D.from_float(float(x))
    for face in source['faces']:
        f=face['surface'];assert f['degreeU']==f['degreeV']==1 and f['knotsU']==f['knotsV']==[0,0,1,1]
        face_cp=f['controlPoints'];assert len(face_cp)==2 and all(len(row)==2 for row in face_cp)
        assert f['weights']==[[1,1],[1,1]] and not face['holes']
        assert all(exact(face_cp[0][0][k])+exact(face_cp[1][1][k])==exact(face_cp[1][0][k])+exact(face_cp[0][1][k]) for k in range(3))
        wire=source['loops'][face['outer']]['coedges'];assert len(wire)==4
        curves=[c['pcurve'] for c in wire]
        assert all(c['degree']==1 and len(c['controlPoints'])==2 and c['weights']==[1,1] for c in curves)
        uv=[c['controlPoints'][0] for c in curves]
        for a,b in zip(curves,curves[1:]+curves[:1]):
            assert a['controlPoints'][-1]==b['controlPoints'][0]
            first,last=a['controlPoints'];assert (first[0]==last[0])!=(first[1]==last[1])
        ranges=[(min(p[k] for p in uv),max(p[k] for p in uv)) for k in range(2)]
        assert {tuple(p) for p in uv}=={(x,y) for x in ranges[0] for y in ranges[1]}
        mapped=[]
        for u,v in uv:
            assert 0<=u<=1 and 0<=v<=1;u=exact(u);v=exact(v)
            point=[sum(exact(face_cp[i][j][k])*(u if i else 1-u)*(v if j else 1-v) for i in range(2) for j in range(2)) for k in range(3)]
            matches=[p for p in points if max(abs(exact(p[k])-point[k]) for k in range(3))<D('1e-25')]
            assert len(matches)==1;mapped.append(matches[0])
        fixed=[(k,v) for k in range(3) for v in [low[k],high[k]] if all(p[k]==v for p in mapped)]
        assert len(fixed)==1 and fixed[0] not in owned;owned.add(fixed[0])
        k,_=fixed[0];free=[i for i in range(3) if i!=k]
        assert {tuple(p[i] for i in free) for p in mapped}=={(a,b) for a in [low[free[0]],high[free[0]]] for b in [low[free[1]],high[free[1]]]}

with localcontext() as ctx:
    ctx.prec=65;exact=lambda x:D.from_float(float(x))
    z=[[exact(p[2]) for p in row] for row in cp]
    symmetric=z[0]==z[2] and all(row[0]==row[2] for row in z)
    def removed_section(x):
        u=(x+1)/2;b=[(1-u)**2,2*u*(1-u),u*u]
        q=[sum(b[i]*z[i][j] for i in range(3)) for j in range(3)]
        # Original Bernstein height is a*v^2+b*v+q0; its roots at the
        # source top bound the removed section. No mirror symmetry is needed.
        a=q[0]-2*q[1]+q[2];b=2*(q[1]-q[0]);c=q[0]-exact(high[2])
        discriminant=b*b-4*a*c
        assert a>0 and discriminant>0
        width=discriminant.sqrt()/a
        first=(-b-discriminant.sqrt())/(2*a);last=first+width
        assert (exact(low[1])+1)/2<first<last<(exact(high[1])+1)/2
        vertex=-b/(2*a);minimum=q[0]-b*b/(4*a)
        assert first<vertex<last and minimum>exact(low[2])
        # Integral of a*(v-first)*(last-v), with physical dy=2*dv.
        return 2*a*width*width*width/6
    def integrate(n):
        lo=exact(low[0]);hi=exact(high[0]);step=(hi-lo)/n
        total=removed_section(lo)+removed_section(hi)
        for i in range(1,n):total+=(4 if i%2 else 2)*removed_section(lo+step*i)
        return total*step/3
    values=[integrate(n) for n in [256,512,1024]]
    source_volume=(exact(high[0])-exact(low[0]))*(exact(high[1])-exact(low[1]))*(exact(high[2])-exact(low[2]))
    assert abs(values[2]-values[1])<D('1e-12')
    report=dict(status='independent-reference-only',sourceVolumeMm3=str(source_volume),
        expectedChangedVolumeMm3=str(source_volume-values[2]),removedVolumeMm3=str(values[2]),
        simpson256To512DeltaMm3=str(abs(values[1]-values[0])),simpson512To1024DeltaMm3=str(abs(values[2]-values[1])),
        sourceSha256=hashlib.sha256((root/'source.json').read_bytes()).hexdigest(),
        cutterSha256=hashlib.sha256((root/'cutter.json').read_bytes()).hexdigest(),
        symmetric=symmetric,scope='actual quadratic-in-Y control surface; high precision quadrature with convergence evidence, not an interval integral certificate',
        changedStepVerified=False,requiredStepVolumeToleranceMm3=1e-9)
(root/'expected-changed-volume.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
