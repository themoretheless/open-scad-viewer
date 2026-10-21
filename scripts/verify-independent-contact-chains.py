#!/usr/bin/env python3
"""Check saved unequal contact windows using OCCT original geometry and STEP.
Sampling supplements native continuous certificates, not global embedding proof.
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
root=Path(sys.argv[1]);source=json.loads((root/'source.json').read_text());model=json.loads((root/'model.json').read_text());meta=json.loads((root/'manifest.json').read_text())
assert meta['allActualSeamsG2Proven'] and not meta['wholeBodyG2'] and not meta['globalEmbedding']
assert meta['originalSavedGeometryChangedByProofWindows'] is False
assert meta['operationCells']<=100000 and meta['combinedCells']<=100000 and meta['exactWork']<=1000000 and meta['coefficientWork']<=2000000
assert list(map(len,meta['chains']))==[2,1] and len(meta['pieces'])==2
assert model['faces']==source['faces']
assert model['vertices'][:len(source['vertices'])]==source['vertices']
assert model['topologyIds']['vertices'][:len(source['vertices'])]==source['topologyIds']['vertices']
for i,e in enumerate(source['edges']):
    if i!=meta['sourceContacts'][0]:assert model['edges'][i]==e

def uses(edge):
    return [(fi,c) for fi,f in enumerate(model['faces']) for w in [f['outer']]+f['holes'] for c in model['loops'][w]['coedges'] if c['edge']==edge]

coverage={};carrier=position=normal=tensor=center=drift=0.;samples=0
for piece in meta['pieces']:
    curves=[jets.curve(model['edges'][e]['curve']) for e in piece['edges']]
    for side,(edge,curve) in enumerate(zip(piece['edges'],curves)):
        assert [curve.FirstParameter(),curve.LastParameter()]==piece['domains'][side]
        coverage.setdefault((side,edge),[]).append(piece['normalizedWindows'][side])
    piece_max=0.
    for j in range(65):
        q=j/64;balls=[];radii=[]
        params=[]
        for side,curve in enumerate(curves):
            a,b=piece['normalizedWindows'][side];fraction=a+(b-a)*q
            params.append(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*fraction)
        radius=meta['lawRadii'][0]+(params[0]-meta['lawDomain'][0])/(meta['lawDomain'][1]-meta['lawDomain'][0])*(meta['lawRadii'][1]-meta['lawRadii'][0])
        for side,(edge,curve,t) in enumerate(zip(piece['edges'],curves,params)):
            a,b=piece['normalizedWindows'][side];fraction=a+(b-a)*q
            world=curve.Value(t).Coord();original=jets.curve(source['edges'][meta['sourceContacts'][side]]['curve'])
            carrier=max(carrier,math.dist(world,original.Value(t).Coord()))
            contacts=uses(edge);assert len(contacts)==2 and sum(fi==meta['skin'] for fi,_ in contacts)==1
            data=[]
            for fi,c in contacts:
                pc=jets.curve(c['pcurve']);f=1-fraction if c['reversed'] else fraction
                uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*f)
                p,n,k=jets.jets(jets.surface(model['faces'][fi]['surface']),uv.X(),uv.Y())
                position=max(position,math.dist(p,world));data.append((n,k))
                if fi!=meta['skin']:balls.append([world[k]+meta['sides'][side]*radius*n[k] for k in range(3)])
            (n,k),(m,l)=data;sign=1 if jets.dot(n,m)>=0 else -1
            normal=max(normal,math.dist(n,[sign*x for x in m]));tensor=max(tensor,max(abs(k[a][b]-sign*l[a][b]) for a in range(3) for b in range(3)))
            radii.append(meta['lawRadii'][0]+(t-meta['lawDomain'][0])/(meta['lawDomain'][1]-meta['lawDomain'][0])*(meta['lawRadii'][1]-meta['lawRadii'][0]))
        piece_max=max(piece_max,math.dist(*balls));drift=max(drift,abs(radii[0]-radii[1]));samples+=1
    assert piece_max<=piece['maximumCenterGapMm'][1]+1e-13
    center=max(center,piece_max)
for side,chain in enumerate(meta['chains']):
    for edge in chain:
        windows=sorted(coverage[(side,edge)])
        assert windows[0][0]==0 and windows[-1][1]==1
        assert all(a[1]==b[0] for a,b in zip(windows,windows[1:]))
assert carrier<=source['toleranceMm'] and position<=source['toleranceMm'] and center<=source['toleranceMm']
assert normal<=1e-5 and tensor<=1e-4
spec=importlib.util.spec_from_file_location('reference',Path(__file__).with_name('verify-radius-transition-occt.py'));reference=importlib.util.module_from_spec(spec);spec.loader.exec_module(reference)
volumes=[];counts=[]
for name,m in [('source.step',source),('body.step',model)]:
    reader=STEPControl_Reader();assert reader.ReadFile(str(root/name))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
    count={n:len(reference.entities(shape,kind)) for n,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    assert count==dict(solids=1,faces=len(m['faces']),edges=len(m['edges']),vertices=len(m['vertices']));counts.append(count)
    props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False);volumes.append(props.Mass())
assert abs(volumes[0]-volumes[1])<1e-9
result=dict(samples=samples,carrierErrorMm=carrier,positionErrorMm=position,normalError=normal,tensorError=tensor,centerGapMm=center,radiusClockDriftMm=drift,volumeDifferenceMm3=abs(volumes[0]-volumes[1]),stepCounts=counts,globalEmbeddingProven=False,
            hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir() if p.suffix in ('.json','.step') and p.name!='independent-check.json'})
(root/'independent-check.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
