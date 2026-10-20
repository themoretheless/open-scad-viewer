#!/usr/bin/env python3
"""Independent OCCT samples of stored root-segment pieces; no welding admission."""
import hashlib,json,math,sys
from pathlib import Path
from occt_surface_jets import surface,curve
root=Path(sys.argv[1]);welded=len(sys.argv)>2 and sys.argv[2]=="--welded"
source=json.loads((root/'source.json').read_text());cutter=json.loads((root/'cutter.json').read_text())
def distance(a,b):return math.sqrt(sum((x-y)**2 for x,y in zip(a,b)))
def rectangle(model,face):
    f=model['faces'][face];assert not f['holes'];uses=model['loops'][f['outer']]['coedges'];assert len(uses)==4
    curves=[c['pcurve'] for c in uses]
    assert all(c['degree']==1 and len(c['controlPoints'])==2 and all(w>0 for w in c['weights']) for c in curves)
    points=[p for c in curves for p in c['controlPoints']]
    bounds=[(min(p[k] for p in points),max(p[k] for p in points)) for k in range(2)]
    corners={(x,y) for x in bounds[0] for y in bounds[1]}
    assert len(corners)==4 and {tuple(c['controlPoints'][0]) for c in curves}==corners
    for a,b in zip(curves,curves[1:]+curves[:1]):
        first,last=a['controlPoints'];assert last==b['controlPoints'][0]
        assert (first[0]==last[0] and first[1]!=last[1]) or (first[1]==last[1] and first[0]!=last[0])
    return bounds
reports=[]
for path in sorted(root.glob('face-*.json')):
    data=json.loads(path.read_text());tol=data['modelToleranceMm']
    assert data['firstSurface']==source['faces'][data['face']]['surface']
    assert data['secondSurface']==cutter['faces'][data['cutterFace']]['surface']
    assert data['welded'] is welded and data['wholeTrimMembershipQualified'] is welded
    assert tol==source['toleranceMm']
    limit=tol/(2 if welded else 4)
    if welded:assert data['junctionBandAndWorldErrorUpperMm']<=tol/2
    assert data['proofCells']+data['coverCells']+data['cells']<=100000 and data['evaluations']<=4096
    groups=data['groups'];assert len(groups)==len(data['reversed'])
    if not welded:assert len(groups)==len(data['rootEnclosures'])-1
    if welded:
        flat=[p for group in groups for p in group];assert not any(data['reversed'])
        regions=[rectangle(source,data['face']),rectangle(cutter,data['cutterFace'])]
        for a,b in zip(flat,flat[1:]):
            assert a['curve']['controlPoints'][-1]==b['curve']['controlPoints'][0]
            for side in range(2):assert a['pcurves'][side]['controlPoints'][-1]==b['pcurves'][side]['controlPoints'][0]
        assert flat[0]['curve']['controlPoints'][0]==data['candidateEndpoints'][0]
        assert flat[-1]['curve']['controlPoints'][-1]==data['candidateEndpoints'][1]
        for piece in flat:
            for pc,bounds in zip(piece['pcurves'],regions):
                assert all(w>0 for w in pc['weights'])
                assert all(bounds[k][0]<=p[k]<=bounds[k][1] for p in pc['controlPoints'] for k in range(2))
    surfaces=[surface(data['firstSurface']),surface(data['secondSurface'])]
    maximum=0.;samples=0;pieces=0;ordered=[];native_upper=0.
    for group,reverse in zip(groups,data['reversed']):
        assert group
        for piece in group:
            pieces+=1;assert piece['worldErrorUpperMm']<=limit
            native_upper=max(native_upper,piece['worldErrorUpperMm'])
            world=curve(piece['curve']);pcs=list(map(curve,piece['pcurves']))
            for i in range(33):
                t=i/32;point=world.Value(world.FirstParameter()+(world.LastParameter()-world.FirstParameter())*t)
                for s,pc in zip(surfaces,pcs):
                    uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*t)
                    actual=s.Value(uv.X(),uv.Y());maximum=max(maximum,distance(point.Coord(),actual.Coord()))
                samples+=1
        for piece in reversed(group) if reverse else group:
            world=curve(piece['curve']);a=world.FirstParameter();b=world.LastParameter()
            ordered.append((list(world.Value(b if reverse else a).Coord()),list(world.Value(a if reverse else b).Coord())))
    assert 1<=pieces<=1024 and maximum<=limit
    gap=max((distance(a[1],b[0]) for a,b in zip(ordered,ordered[1:])),default=0.)
    endpoint=max(distance(ordered[0][0],data['candidateEndpoints'][0]),distance(ordered[-1][1],data['candidateEndpoints'][1]))
    assert gap<tol and endpoint<tol
    reports.append(dict(face=data['face'],pieces=pieces,sampleSites=samples,maximumSupportDeviationMm=maximum,
        nativeWorldErrorUpperMm=native_upper,maximumSampledJunctionGapMm=gap,candidateEndpointDeviationMm=endpoint,
        coefficientsSha256=hashlib.sha256(path.read_bytes()).hexdigest()))
assert len(reports)==2
result=dict(passed=True,faces=reports,literalJunctionsChecked=welded,completeUvControlHullsInsideOriginalRectangles=welded,scope='independent support samples; welded mode also checks literal joins and whole positive-rational UV control hulls inside authored rectangles; native root-band proofs remain separate; face partition and solid admission remain pending')
(root/'independent-occt-curves.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
