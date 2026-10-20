#!/usr/bin/env python3
"""Independent saved carrier/G2 samples and STEP check for mixed owned roots.
Samples supplement continuous native admission; they do not prove embedding.
"""
import hashlib, importlib.util, json, math, sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
import occt_surface_jets as jets

root = Path(sys.argv[1])
model = json.loads((root/'model.json').read_text())
source = json.loads((root/'source.json').read_text())
meta = json.loads((root/'manifest.json').read_text())
assert meta['allActualChildG2Proven'] and not meta['wholeBodyG2'] and not meta['globalEmbedding']
assert meta['cells'] <= 100000 and meta['exactWork'] <= 1000000 and meta['coefficientWork'] <= 2000000
assert model['faces'] == source['faces']
assert model['vertices'][:len(source['vertices'])] == source['vertices']
assert model['topologyIds']['vertices'][:len(source['vertices'])] == source['topologyIds']['vertices']
edge = meta['originalEdge']; chain = meta['children']; endpoint = meta['endpointVertex']
assert len(chain) == 2 and endpoint in source['edges'][edge]['vertices']
assert endpoint in model['edges'][chain[0]]['vertices']
assert model['edges'][chain[0]]['vertices'][1] == model['edges'][chain[1]]['vertices'][0] == meta['interiorVertex']
assert len(model['vertices']) == len(source['vertices'])+1
assert len(model['edges']) == len(source['edges'])+1
for i,e in enumerate(source['edges']):
    if i != edge: assert model['edges'][i] == e

def uses(m,e):
    return [(fi,c) for fi,f in enumerate(m['faces']) for w in [f['outer']]+f['holes']
            for c in m['loops'][w]['coedges'] if c['edge']==e]
faces = {fi for fi,_ in uses(source,edge)}
assert len(faces) == 2
original = jets.curve(source['edges'][edge]['curve'])
carrier = position = normal = tensor = 0.; samples = 0
for child in chain:
    curve = jets.curve(model['edges'][child]['curve']); contacts = uses(model,child)
    assert len(contacts)==2 and {fi for fi,_ in contacts}==faces
    for j in range(65):
        q=j/64; t=curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*q
        world=curve.Value(t).Coord(); carrier=max(carrier,math.dist(world,original.Value(t).Coord()))
        data=[]
        for fi,c in contacts:
            pc=jets.curve(c['pcurve']); fraction=1-q if c['reversed'] else q
            uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*fraction)
            p,n,k=jets.jets(jets.surface(model['faces'][fi]['surface']),uv.X(),uv.Y())
            position=max(position,math.dist(p,world)); data.append((n,k))
        (n,k),(m,l)=data; sign=1 if jets.dot(n,m)>=0 else -1
        normal=max(normal,math.dist(n,[sign*x for x in m]))
        tensor=max(tensor,max(abs(k[a][b]-sign*l[a][b]) for a in range(3) for b in range(3)))
        samples+=1
assert carrier <= source['toleranceMm'] and position <= source['toleranceMm']
assert normal <= 1e-5 and tensor <= 1e-4
spec=importlib.util.spec_from_file_location('reference',Path(__file__).with_name('verify-radius-transition-occt.py'))
reference=importlib.util.module_from_spec(spec);spec.loader.exec_module(reference)
volumes=[]; counts=[]
for name, expected in [('source.step',source),('body.step',model)]:
    reader=STEPControl_Reader(); assert reader.ReadFile(str(root/name))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
    count={n:len(reference.entities(shape,kind)) for n,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    assert count==dict(solids=1,faces=len(expected['faces']),edges=len(expected['edges']),vertices=len(expected['vertices']))
    props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False)
    volumes.append(props.Mass());counts.append(count)
assert abs(volumes[0]-volumes[1])<1e-9
result=dict(controls=1,samples=samples,carrierErrorMm=carrier,positionErrorMm=position,normalError=normal,tensorError=tensor,
            volumeDifferenceMm3=abs(volumes[0]-volumes[1]),stepCounts=counts,globalEmbeddingProven=False,
            hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.suffix in ('.json','.step') and p.name!='independent-check.json'})
(root/'independent-check.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
