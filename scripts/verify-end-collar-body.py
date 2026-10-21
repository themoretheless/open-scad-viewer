#!/usr/bin/env python3
"""Independent changed-body STEP, meridian-volume and saved-contact samples."""
import json,math,sys,hashlib
from fractions import Fraction as F
from pathlib import Path
from occt_surface_jets import surface,curve,jets,dot,norm
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
root=Path(sys.argv[1]);meta=json.loads((root/'manifest.json').read_text());model=json.loads((root/'model.json').read_text());src=json.loads((root/'source.json').read_text())
assert meta['closedBodyTopologyQualified'] and meta['savedSkinG2Qualified'] and meta['savedChartInjective'] and meta['rootIdsPreserved']
assert meta['cells']<=100000 and meta['exactWork']<=1000000
assert meta['retainedSurfaceDefinitionsUnchanged']
ends=meta.get('ends',[meta])
for work in meta.get('stageWork',[]):
    assert work['cells']<=100000 and work['exactWork']<=1000000 and work['coefficientWork']<=2000000
assert meta.get('coefficientWork',0)<=2000000
assert len(model['faces'])==len(src['faces'])+len(ends)
assert all(face['surface']==model['faces'][i]['surface'] for i,face in enumerate(src['faces']))


def power(p):
    n=len(p)-1
    return [F(math.comb(n,k))*sum((F(-1) if (k-i)%2 else F(1))*math.comb(k,i)*F(p[i]) for i in range(k+1)) for k in range(n+1)]
def mul(a,b):
    out=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b):out[i+j]+=x*y
    return out

def entities(s,k):
    out=[];e=TopExp_Explorer(s,k)
    while e.More():
        v=e.Current()
        if not any(v.IsSame(old) for old in out):out.append(v)
        e.Next()
    return out

def read(name):
    r=STEPControl_Reader();assert r.ReadFile(str(root/name))==IFSelect_RetDone and r.TransferRoots()
    s=r.OneShape();assert not s.IsNull() and BRepCheck_Analyzer(s).IsValid()
    assert len(entities(s,TopAbs_SOLID))==1
    g=GProp_GProps();BRepGProp.VolumeProperties_s(s,g,1e-10,True,False)
    return s,g.Mass()
shape,volume=read('body.step');original,source_volume=read('source.step')
topology={n:len(entities(shape,k)) for n,k in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert topology==dict(solids=1,faces=meta['faces'],edges=meta['edges'],vertices=meta['vertices'])
box=Bnd_Box();BRepBndLib.AddOptimal_s(shape,box,False,False);bounds=list(box.Get())
outer_radius=max(math.hypot(*p[:2]) for i in [0,3,6] for p in src['faces'][i]['surface']['controlPoints'][0])
z_bounds=[min(v['point'][2] for v in src['vertices']),max(v['point'][2] for v in src['vertices'])]
assert max(abs(a-b) for a,b in zip(bounds,[0.,0.,z_bounds[0],outer_radius,outer_radius,z_bounds[1]]))<2e-7,bounds

# Independent exact polynomial meridian integrals for all three circular bands.
# Admit only canonical quarter-angle tensor products with constant V weights.
source_integral=F(0)
source_axial_control_deviation=0.
for index in [0,3,6]:
    s=src['faces'][index]['surface'];assert s['degreeU']==2 and s['knotsU']==[0,0,0,1,1,1]
    rows=s['controlPoints'];radii=[p[0] for p in rows[0]];heights=[p[2] for p in rows[0]]
    assert all(rows[0][j][:2]==[r,0] and rows[1][j][:2]==[r,r] and rows[2][j][:2]==[0,r] for j,r in enumerate(radii))
    source_axial_control_deviation=max(source_axial_control_deviation,max(abs(p[2]-heights[j]) for row in rows for j,p in enumerate(row)))
    assert source_axial_control_deviation<1e-15
    assert all(len(set(row))==1 for row in s['weights'])
    assert [row[0] for row in s['weights']]==[1.,math.sqrt(0.5),1.]
    rp=power(radii);hp=power(heights);dh=[i*hp[i] for i in range(1,len(hp))]
    poly=mul(mul(rp,rp),dh);source_integral+=sum(v/(i+1) for i,v in enumerate(poly))
assert abs(source_volume-math.pi/4*float(source_integral))<1e-9
source_radius=ends[0]['sourceRadiusMm']
assert all(p[0]==source_radius for p in src['faces'][0]['surface']['controlPoints'][0])
removed_volume=0.
for end in ends:
    skin=model['faces'][end['skinFace']]['surface'];assert skin['degreeV']==5 and len(skin['controlPoints'][0])==6
    assert all(len(set(row))==1 for row in skin['weights'])
    row=skin['controlPoints'][0];scale=math.hypot(*row[-1][:2]);r=[math.hypot(*p[:2])/scale for p in row];h=[p[2] for p in row]
    rp=power(r);hp=power(h);dh=[i*hp[i] for i in range(1,len(hp))];polynomial=mul(mul(rp,rp),dh)
    integral=sum(v/(i+1) for i,v in enumerate(polynomial));dz=h[-1]-h[0]
    removed_volume+=math.pi/4*end['sourceRadiusMm']**2*(abs(dz)-abs(float(integral)))
expected=source_volume-removed_volume;error=abs(volume-expected)
assert error<1e-8,(volume,expected,error)
maximum=[0.,0.,0.];samples=0
groups=[(end['skinFace'],end['contactEdges']) for end in ends]
protected=meta.get('protectedEdges',[])
if protected:
    assert meta['protectedSavedSeamsG2Proven'] and meta['centralRadiusLawRequalified']
    assert not meta['wholeBodyG2'] and not meta['generalCornerClaim']
    assert len(protected)==len(set(protected))==len(meta['sourceProtectedEdges'])==2
    for before,after in zip(meta['sourceProtectedEdges'],protected):
        assert src['topologyIds']['edges'][before]==model['topologyIds']['edges'][after]
        assert src['edges'][before]['curve']==model['edges'][after]['curve']
        assert [src['topologyIds']['vertices'][v] for v in src['edges'][before]['vertices']]==[model['topologyIds']['vertices'][v] for v in model['edges'][after]['vertices']]
        assert [src['vertices'][v]['point'] for v in src['edges'][before]['vertices']]==[model['vertices'][v]['point'] for v in model['edges'][after]['vertices']]
    groups.append((None,protected))
for skin_id,edge_ids in groups:
    for edge in edge_ids:
        uses=[]
        for f,face in enumerate(model['faces']):
            for w in [face['outer']]+face['holes']:
                uses.extend((f,c) for c in model['loops'][w]['coedges'] if c['edge']==edge)
        assert len(uses)==2 and (skin_id is None or sum(f==skin_id for f,c in uses)==1)
        converted=[(surface(model['faces'][f]['surface']),curve(c['pcurve']),c['reversed']) for f,c in uses]
        for i in range(129):
            t=i/128;values=[]
            for s,p,reverse in converted:
                lo=p.FirstParameter();hi=p.LastParameter();q=p.Value(lo+(hi-lo)*(1-t if reverse else t));values.append(jets(s,q.X(),q.Y()))
            a,na,ta=values[0];b,nb,tb=values[1];sign=1 if dot(na,nb)>=0 else -1
            e=[norm([x-y for x,y in zip(a,b)]),norm([x-sign*y for x,y in zip(na,nb)]),math.sqrt(sum((ta[k][j]-sign*tb[k][j])**2 for k in range(3) for j in range(3)))]
            maximum=[max(x,y) for x,y in zip(maximum,e)];samples+=1
assert maximum[0]<1e-7 and maximum[1]<1e-5 and maximum[2]<1e-4,maximum
axial_error=radial_error=0.
for end in ends:
    edge_ids=end['contactEdges']
    cap_curve=curve(model['edges'][edge_ids[0]]['curve']);wall_curve=curve(model['edges'][edge_ids[1]]['curve'])
    cap=cap_curve.Value(cap_curve.FirstParameter());wall=wall_curve.Value(wall_curve.FirstParameter())
    axial_error=max(axial_error,abs(end.get('axialSign',1)*(wall.Z()-cap.Z())-end['axialSetbackMm']))
    radial_error=max(radial_error,abs(end['sourceRadiusMm']-math.hypot(cap.X(),cap.Y())-end['radialSetbackMm']))
assert axial_error<1e-7 and radial_error<1e-7
report=dict(protectedSavedSeams=len(protected),totalSavedG2Seams=sum(len(edges) for _,edges in groups),passed=True,topology=topology,boundsMm=bounds,sourceVolumeMm3=source_volume,sourceAxialControlDeviationMm=source_axial_control_deviation,volumeMm3=volume,expectedVolumeMm3=expected,volumeErrorMm3=error,
    contactSamples=samples,maximumPositionErrorMm=maximum[0],maximumNormalError=maximum[1],maximumWorldCurvatureTensorError=maximum[2],
    axialSetbackErrorMm=axial_error,radialSetbackErrorMm=radial_error,stepSha256=hashlib.sha256((root/'body.step').read_bytes()).hexdigest(),
    scope=f'closed quarter-body with {len(ends)} saved plane/cylinder G2 ends; sharp angular walls and general corners remain outside scope')
(root/'independent-occt-body.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
