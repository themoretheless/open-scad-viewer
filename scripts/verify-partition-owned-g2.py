#!/usr/bin/env python3
"""Independent STEP and sampled original-support checks for split G2 contacts.

Sampling supplements the native continuous certificates; it does not certify
root isolation, general fillets, or global absence of self-intersections.
"""
import hashlib
import importlib.util
import json
import math
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
import occt_surface_jets as jets

spec = importlib.util.spec_from_file_location('radius', Path(__file__).with_name('verify-radius-transition-occt.py'))
radius = importlib.util.module_from_spec(spec)
spec.loader.exec_module(radius)
root = Path(sys.argv[1])
m = json.loads((root/'model.json').read_text())
s = json.loads((root/'source.json').read_text())
manifest = json.loads((root/'manifest.json').read_text())
cuts=len(manifest['vertices'])
assert cuts in [1,2] and len(manifest['childEdges']) == cuts+1
assert m['vertices'][:len(s['vertices'])] == s['vertices']
for kind in ['vertices', 'faces', 'loops', 'shells', 'bodies']:
    assert m['topologyIds'][kind][:len(s['topologyIds'][kind])] == s['topologyIds'][kind]
assert [f['surface'] for f in m['faces']] == [f['surface'] for f in s['faces']]
r = STEPControl_Reader()
assert r.ReadFile(str(root/'body.step')) == IFSelect_RetDone and r.TransferRoots()
shape = r.OneShape()
assert BRepCheck_Analyzer(shape).IsValid()
topology = {name: len(radius.entities(shape,kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert topology == dict(solids=1, faces=11, edges=21+cuts, vertices=12+cuts)
g = GProp_GProps()
BRepGProp.VolumeProperties_s(shape,g,1e-10,True,False)
expected_volume = 0.011440973246027764
assert abs(g.Mass()-expected_volume) < 1e-9
vertices = radius.entities(shape,TopAbs_VERTEX)
for vertex in m['vertices']:
    point = vertex['point']
    matches = [v for v in vertices if math.dist(BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)).Coord(),point) < 1e-9]
    assert len(matches) == 1
    if vertex in [m['vertices'][i] for i in manifest['vertices']]:
        v = matches[0]
        edges = [e for e in radius.entities(shape,TopAbs_EDGE) if any(v.IsSame(x) for x in radius.entities(e,TopAbs_VERTEX))]
        faces = [f for f in radius.entities(shape,TopAbs_FACE) if any(v.IsSame(x) for x in radius.entities(f,TopAbs_VERTEX))]
        assert len(edges) == len(faces) == 2
max_position = max_normal = max_tensor = 0.
samples = 0
for edge_id in manifest['childEdges']:
    edge = m['edges'][edge_id]
    world = jets.curve(edge['curve'])
    uses = [(fi,c) for fi,f in enumerate(m['faces']) for li in [f['outer']]+f['holes'] for c in m['loops'][li]['coedges'] if c['edge'] == edge_id]
    assert len(uses) == 2 and sorted(fi for fi,_ in uses) == sorted(manifest['faces'])
    for i in range(33):
        t = i/32
        point = world.Value(world.FirstParameter()+(world.LastParameter()-world.FirstParameter())*t).Coord()
        data=[]
        for fi,c in uses:
            pc=jets.curve(c['pcurve'])
            q=1-t if c['reversed'] else t
            uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
            p,n,k=jets.jets(jets.surface(m['faces'][fi]['surface']),uv.X(),uv.Y())
            max_position=max(max_position,math.dist(point,p))
            data.append((n,k))
        n0,k0=data[0];n1,k1=data[1]
        sign=1 if jets.dot(n0,n1)>=0 else -1
        max_normal=max(max_normal,math.dist(n0,[sign*x for x in n1]))
        max_tensor=max(max_tensor,max(abs(k0[a][b]-sign*k1[a][b]) for a in range(3) for b in range(3)))
        samples+=1
assert max_position < 1e-9 and max_normal < 1e-9 and max_tensor < 1e-8
if manifest['rootsIsolated']:
    assert cuts==1
    p=m['vertices'][manifest['vertices'][0]]['point']
    residual=abs(p[0]-manifest['cutterGraphCoefficient']*p[1]*p[2]-manifest['cutterX'])
    assert residual < 1e-7
    assert manifest['rootPositionErrorUpperMm'] <= 1e-7
    assert manifest['cells']<=100000 and manifest['coefficientWork']<=2000000
report=dict(passed=True,topology=topology,volumeMm3=g.Mass(),volumeErrorMm3=abs(g.Mass()-expected_volume),samples=samples,maxPositionErrorMm=max_position,maxNormalChord=max_normal,maxShapeTensorDifference=max_tensor,nativeRootsIsolated=manifest['rootsIsolated'],wholeBodyG2=False,scope='partitions of one owned circular G2 contact; sampled independent checks',sha256={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in ['source.json','model.json','body.step','manifest.json']})
(root/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
