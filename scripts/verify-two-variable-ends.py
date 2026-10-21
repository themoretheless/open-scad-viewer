#!/usr/bin/env python3
"""Independent STEP topology/dimensions and sampled four-seam G2/radius checks.
Sampling complements native continuous certificates; it proves no global embedding.
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
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
import occt_surface_jets as jets

spec = importlib.util.spec_from_file_location('reference', Path(__file__).with_name('verify-radius-transition-occt.py'))
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)
root = Path(sys.argv[1])
model = json.loads((root / 'model.json').read_text())
source = json.loads((root / 'source.json').read_text())
manifest = json.loads((root / 'manifest.json').read_text())
assert manifest['fourSavedSeamsG2Proven'] and manifest['bothFinalRadiusLawsProven']
assert not manifest['wholeBodyG2'] and not manifest['globalEmbeddingProven']
assert max(manifest['stageCells'] + [manifest['finalCells']]) <= 100000
assert max(manifest['stageExactWork'] + [manifest['finalExactWork']]) <= 1000000
assert manifest['finalCoefficientWork'] <= 2000000
assert [f['surface'] for f in model['faces'][:len(source['faces'])]] == [f['surface'] for f in source['faces']]
reader = STEPControl_Reader()
assert reader.ReadFile(str(root / 'body.step')) == IFSelect_RetDone and reader.TransferRoots()
shape = reader.OneShape()
assert BRepCheck_Analyzer(shape).IsValid()
counts = {name: len(reference.entities(shape, kind)) for name, kind in [('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
assert counts == dict(solids=1, faces=len(model['faces']), edges=len(model['edges']), vertices=len(model['vertices']))
box = Bnd_Box()
BRepBndLib.AddOptimal_s(shape, box, False, False)
assert max(abs(a-b) for a,b in zip(box.Get(), [0.,0.,-0.2,1.,1.,0.4])) < 2e-7
props = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, props, 1e-10, True, False)
assert 0 < props.Mass() < math.pi * .6 / 4
reports = []
for end, edges in enumerate(manifest['contacts']):
    contacts = []
    for side, edge in enumerate(edges):
        uses = [(fi,c) for fi,f in enumerate(model['faces']) for w in [f['outer']]+f['holes'] for c in model['loops'][w]['coedges'] if c['edge']==edge]
        assert len(uses) == 2
        support = manifest['supportFaces'][end][side]
        retained = next(c for f,c in uses if f==support)
        skin, saved = next((f,c) for f,c in uses if f!=support)
        assert skin >= len(source['faces'])
        contacts.append((jets.curve(model['edges'][edge]['curve']), support, retained, skin, saved))
    assert contacts[0][3] == contacts[1][3]
    r0,r1 = manifest['radiiMm'][end]
    pad = manifest['padding'][end]
    law = [r0-pad*(r1-r0), r1+pad*(r1-r0)]
    position = normal = tensor = center_gap = endpoint_error = 0.
    signs = []
    axis = manifest['axes'][end]
    origin = manifest['origins'][end]
    for side,(_,support,_,_,_) in enumerate(contacts):
        surface = jets.surface(model['faces'][support]['surface'])
        u0,u1,v0,v1 = surface.Bounds()
        point,n,_ = jets.jets(surface,(u0+u1)/2,(v0+v1)/2)
        delta = [point[k]-origin[k] for k in range(3)]
        height = jets.dot(delta,axis)
        dot = jets.dot(n,axis) if side==0 else -jets.dot(n,[delta[k]-height*axis[k] for k in range(3)])
        assert dot != 0
        signs.append(math.copysign(1,dot))
    for i in range(65):
        fraction = i/64
        centers = []
        for side,(curve,support,retained,skin,saved) in enumerate(contacts):
            t = curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*fraction
            world = curve.Value(t).Coord()
            data = []
            for face,coedge in [(support,retained),(skin,saved)]:
                pc = jets.curve(coedge['pcurve'])
                q = 1-fraction if coedge['reversed'] else fraction
                uv = pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                point,n,k = jets.jets(jets.surface(model['faces'][face]['surface']),uv.X(),uv.Y())
                position = max(position,math.dist(point,world))
                data.append((n,k))
            (n,k),(n2,k2) = data
            sign = 1 if jets.dot(n,n2)>=0 else -1
            normal = max(normal,math.dist(n,[sign*x for x in n2]))
            tensor = max(tensor,max(abs(k[a][b]-sign*k2[a][b]) for a in range(3) for b in range(3)))
            radius = law[0]+t*(law[1]-law[0])
            centers.append([world[a]+signs[side]*radius*n[a] for a in range(3)])
            if i in [0,64]: endpoint_error=max(endpoint_error,abs(radius-[r0,r1][int(i==64)]))
        center_gap=max(center_gap,math.dist(*centers))
    assert position<1e-7 and normal<1e-5 and tensor<1e-4 and center_gap<1e-7 and endpoint_error<1e-7
    reports.append(dict(end=end,samples=130,maxPositionErrorMm=position,maxNormalChord=normal,maxShapeTensorDifference=tensor,maxBallCenterGapMm=center_gap,maxEndpointRadiusErrorMm=endpoint_error))
protected_reports = []
protected = manifest.get('protectedEdges', [])
if protected:
    assert manifest['protectedSavedSeamsG2Proven']
    assert len(protected) == len(set(protected)) == len(manifest['sourceProtectedEdges']) == 2
    for original, edge in zip(manifest['sourceProtectedEdges'], protected):
        assert model['edges'][edge]['curve'] == source['edges'][original]['curve']
        assert model['topologyIds']['edges'][edge] == source['topologyIds']['edges'][original]
        assert [model['topologyIds']['vertices'][v] for v in model['edges'][edge]['vertices']] == [source['topologyIds']['vertices'][v] for v in source['edges'][original]['vertices']]
        before = [(f,c) for f,face in enumerate(source['faces']) for w in [face['outer']]+face['holes'] for c in source['loops'][w]['coedges'] if c['edge']==original]
        after = [(f,c) for f,face in enumerate(model['faces']) for w in [face['outer']]+face['holes'] for c in model['loops'][w]['coedges'] if c['edge']==edge]
        assert len(before) == len(after) == 2
        assert [(f,c['pcurve'],c['reversed']) for f,c in before] == [(f,c['pcurve'],c['reversed']) for f,c in after]
        curve = jets.curve(model['edges'][edge]['curve'])
        maximum = [0.,0.,0.]
        for i in range(65):
            fraction = i/64
            world = curve.Value(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*fraction).Coord()
            data = []
            for face,c in after:
                pc = jets.curve(c['pcurve'])
                q = 1-fraction if c['reversed'] else fraction
                uv = pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                point,n,k = jets.jets(jets.surface(model['faces'][face]['surface']),uv.X(),uv.Y())
                maximum[0] = max(maximum[0],math.dist(point,world))
                data.append((n,k))
            (n,k),(n2,k2) = data
            sign = 1 if jets.dot(n,n2)>=0 else -1
            maximum[1] = max(maximum[1],math.dist(n,[sign*x for x in n2]))
            maximum[2] = max(maximum[2],max(abs(k[a][b]-sign*k2[a][b]) for a in range(3) for b in range(3)))
        assert all(value <= limit for value,limit in zip(maximum,[1e-7,1e-5,1e-4]))
        protected_reports.append(dict(edge=edge,samples=65,maxPositionErrorMm=maximum[0],maxNormalChord=maximum[1],maxShapeTensorDifference=maximum[2]))
report = dict(protectedSeams=protected_reports,totalSavedG2Seams=4+len(protected),passed=True,topology=counts,bounds=box.Get(),volumeMm3=props.Mass(),ends=reports,wholeBodyG2=False,globalEmbedding=False,sha256={n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in ['model.json','source.json','body.step','manifest.json']})
(root/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
