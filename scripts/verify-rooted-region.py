#!/usr/bin/env python3
"""Independent STEP and sampled seam checks of a root-bound region replacement."""
import importlib.util
import hashlib
import json
import math
import sys
from pathlib import Path
if not __debug__:
    raise RuntimeError("Exact qualification checks require Python assertions")
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
import occt_surface_jets as jets
spec=importlib.util.spec_from_file_location('radius',Path(__file__).with_name('verify-radius-transition-occt.py'))
helpers=importlib.util.module_from_spec(spec);spec.loader.exec_module(helpers)
root=Path(sys.argv[1])
m=json.loads((root/'model.json').read_text());source=json.loads((root/'source.json').read_text());receipt=json.loads((root/'manifest.json').read_text())
expected_roots=receipt.get('requestedCuts',4)
assert 1<=expected_roots<=60 and len(receipt['roots'])==expected_roots and receipt['cells']<=100000
interior_roots=[r for r in receipt['roots'] if not r.get('reusedEndpoint',False)]
endpoint_roots=[r for r in receipt['roots'] if r.get('reusedEndpoint',False)]
assert len({r['vertex'] for r in interior_roots})==len(interior_roots)
for r in endpoint_roots:
    assert len(r['children'])==1 and 0<=r['sourceVertex']<len(source['vertices'])
    assert m['vertices'][r['vertex']]==source['vertices'][r['sourceVertex']]
    assert m['topologyIds']['vertices'][r['vertex']]==source['topologyIds']['vertices'][r['sourceVertex']]
for r in interior_roots:assert len(r['children'])==2
if 'requestedCuts' in receipt:
    groups={}
    for r in receipt['roots']:
        assert 0<=r['sourceEdge']<len(source['edges'])
        groups.setdefault(r['sourceEdge'],[]).append(r)
    for group in groups.values():
        group.sort(key=lambda r:r['parameterBand'][0])
        for a,b in zip(group,group[1:]):
            assert a['parameterBand'][1]<b['parameterBand'][0]
            assert a['children'][-1]==b['children'][0]
if '--automatic' in sys.argv[2:]:
    assert receipt['automaticRegionSelection'] is True
    assert 0 < receipt['topologyVisits'] <= 100000
    assert receipt['selectedOriginalFaces'] and len(set(receipt['selectedOriginalFaces']))==len(receipt['selectedOriginalFaces'])
    assert all(0<=i<len(source['faces']) for i in receipt['selectedOriginalFaces'])
if '--discovered' in sys.argv[2:]:
    assert receipt['automaticRootDiscovery'] is True and receipt['allQueriesComplete'] is True
    assert receipt['sameModelAsExplicitRoots'] is True and receipt['lowWorkPublishesNoModel'] is True
    assert receipt['discoveredRoots']==expected_roots
    queries=receipt['queries']
    assert len(queries)==receipt['queryCount'] and all(q['complete'] for q in queries)
    assert sum(q['retainedRoots'] for q in queries)==expected_roots
    assert sum(q['cells'] for q in queries)==receipt['discoveryCells']<=receipt['cells']
assert receipt['generalRadiusLaw'] is False and receipt['globalEmbedding'] is False
reader=STEPControl_Reader();assert reader.ReadFile(str(root/'body.step'))==IFSelect_RetDone and reader.TransferRoots()
shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
counts={name:len(helpers.entities(shape,kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert counts==dict(solids=1,faces=len(source['faces']),edges=len(source['edges'])+len(interior_roots),vertices=len(source['vertices'])+len(interior_roots))
g=GProp_GProps();BRepGProp.VolumeProperties_s(shape,g,1e-10,True,False)
assert abs(g.Mass()-243/8)<1e-8
vertices=helpers.entities(shape,TopAbs_VERTEX)
for v in source['vertices']:
    assert sum(math.dist(BRep_Tool.Pnt_s(TopoDS.Vertex_s(x)).Coord(),v['point'])<1e-9 for x in vertices)==1
maximum_position=maximum_normal=maximum_tensor=0.;samples=0;seen_edges=set()
for end in receipt['roots']:
    vertex=m['vertices'][end['vertex']]['point'];assert end['errorUpperMm']<=1e-7
    for x,band in zip(vertex,end['positionBounds']):assert band[0]-1e-7<=x<=band[1]+1e-7
    for edge in end['children']:
        assert end['vertex'] in m['edges'][edge]['vertices']
        if edge in seen_edges:continue
        seen_edges.add(edge)
        uses=[(fi,c) for fi,f in enumerate(m['faces']) for w in [f['outer']]+f['holes'] for c in m['loops'][w]['coedges'] if c['edge']==edge]
        assert len(uses)==2
        curve=jets.curve(m['edges'][edge]['curve'])
        for i in range(33):
            t=i/32;point=curve.Value(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*t).Coord();data=[]
            for fi,c in uses:
                pc=jets.curve(c['pcurve']);q=1-t if c['reversed'] else t
                uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                p,n,k=jets.jets(jets.surface(m['faces'][fi]['surface']),uv.X(),uv.Y());data.append((n,k))
                maximum_position=max(maximum_position,math.dist(p,point))
            (n,k),(n2,k2)=data;sign=1 if jets.dot(n,n2)>=0 else -1
            maximum_normal=max(maximum_normal,math.dist(n,[sign*x for x in n2]))
            maximum_tensor=max(maximum_tensor,max(abs(k[a][b]-sign*k2[a][b]) for a in range(3) for b in range(3)))
            samples+=1
assert maximum_position<1e-7 and maximum_normal<1e-5 and maximum_tensor<1e-4
report=dict(passed=True,topology=counts,volumeMm3=g.Mass(),volumeErrorMm3=abs(g.Mass()-243/8),seamSamples=samples,maxPositionErrorMm=maximum_position,maxNormalChord=maximum_normal,maxShapeTensorDifference=maximum_tensor,automaticRegionSelection=receipt.get('automaticRegionSelection',False),automaticRootDiscovery=receipt.get('automaticRootDiscovery',False),generalRadiusLaw=False,globalEmbedding=False,rootCount=expected_roots,reusedEndpointCount=len(endpoint_roots),newRootCount=len(interior_roots),uniqueSeams=len(seen_edges),scope='root-bound split sides in a mapped paraboloid region replacement; sampled checks supplement native proofs',sha256={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in ['source.json','model.json','body.step','manifest.json']})
(root/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
