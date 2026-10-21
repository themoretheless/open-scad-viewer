#!/usr/bin/env python3
"""Independent STEP, original-carrier and saved radius/G2 chain samples.
Samples supplement native continuous checks; whole-body embedding stays open.
"""
import hashlib,importlib.util,json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
import occt_surface_jets as jets
spec=importlib.util.spec_from_file_location('reference',Path(__file__).with_name('verify-radius-transition-occt.py'))
reference=importlib.util.module_from_spec(spec);spec.loader.exec_module(reference)
root=Path(sys.argv[1]);model=json.loads((root/'model.json').read_text());source=json.loads((root/'source.json').read_text());meta=json.loads((root/'manifest.json').read_text())
assert meta['combinedAdmission'] and meta['wholeSavedChainRadiusProven'] and meta['allSavedFragmentsG2Proven']
assert meta['combinedCells']<=100000 and meta['combinedExactWork']<=1000000
assert meta['operationCells']<=100000 and meta['numericCutsAreRootCertificates'] is False
assert meta['operationCells']==meta['partitionCells']+meta['originalCarrierCells']+meta['combinedCells']
assert not meta['wholeBodyG2'] and not meta['globalEmbedding']
assert meta['cells']<=100000 and meta['exactWork']<=1000000
assert meta['g2Cells']<=100000 and meta['g2ExactWork']<=1000000 and meta['g2CoefficientWork']<=2000000
assert len(meta['pieces'])==3 and all(len(c)==3 for c in meta['chains'])
assert model['faces']==source['faces']
for i,e in enumerate(source['edges']):
    if i not in meta['sourceContacts']:assert model['edges'][i]==e
for vertex in source['vertices']:assert sum(v['point']==vertex['point'] for v in model['vertices'])==1
shapes=[];volumes=[]
for name in ['source.step','body.step']:
    reader=STEPControl_Reader();assert reader.ReadFile(str(root/name))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid();shapes.append(shape)
    p=GProp_GProps();BRepGProp.VolumeProperties_s(shape,p,1e-10,True,False);volumes.append(p.Mass())
counts={n:len(reference.entities(shapes[1],kind)) for n,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert counts==dict(solids=1,faces=len(source['faces']),edges=len(source['edges'])+4,vertices=len(source['vertices'])+4)
assert abs(volumes[1]-volumes[0])<1e-9
box=Bnd_Box();BRepBndLib.AddOptimal_s(shapes[1],box,False,False)
assert max(abs(a-b) for a,b in zip(box.Get(),[0,0,-.2,1,1,.4]))<2e-7
skin=meta['skin'];skin_edges={c['edge'] for w in [model['faces'][skin]['outer']]+model['faces'][skin]['holes'] for c in model['loops'][w]['coedges']}
for side,chain in enumerate(meta['chains']):
    f=model['faces'][meta['supportFaces'][side]]
    expected={c['edge'] for w in [f['outer']]+f['holes'] for c in model['loops'][w]['coedges'] if c['edge'] in skin_edges}
    assert set(chain)==expected and len(chain)==len(set(chain))
    assert chain[0]==meta['sourceContacts'][side]
    for a,b in zip(chain,chain[1:]):
        assert model['edges'][a]['vertices'][1]==model['edges'][b]['vertices'][0]
position=normal=tensor=carrier=centers=radius_drift=0.;samples=0;per_piece=[]
for i,receipt in enumerate(meta['pieces']):
    edges=[meta['chains'][k][i] for k in range(2)];assert receipt['edges']==edges
    curves=[jets.curve(model['edges'][e]['curve']) for e in edges]
    domains=receipt.get('domains',[receipt['domain'],receipt['domain']])
    assert all([c.FirstParameter(),c.LastParameter()]==domains[side] for side,c in enumerate(curves))
    for side in range(2):
        if i==0:assert domains[side][0]==meta['coverageDomain'][0]
        else:
            previous=meta['pieces'][i-1].get('domains',[meta['pieces'][i-1]['domain']]*2)
            assert previous[side][1]==domains[side][0]
        if i==2:assert domains[side][1]==meta['coverageDomain'][1]
    piece_center=0.
    for j in range(65):
        fraction=j/64;balls=[]
        parameters=[c.FirstParameter()+(c.LastParameter()-c.FirstParameter())*fraction for c in curves]
        radii=[meta['lawRadii'][0]+(t-meta['lawDomain'][0])/(meta['lawDomain'][1]-meta['lawDomain'][0])*(meta['lawRadii'][1]-meta['lawRadii'][0]) for t in parameters]
        radius_drift=max(radius_drift,abs(radii[0]-radii[1]))
        # One common sphere radius, anchored to the first ORIGINAL clock.
        radius=radii[0];assert radius>0
        for side,(edge,curve) in enumerate(zip(edges,curves)):
            t=parameters[side]
            world=curve.Value(t).Coord();original=jets.curve(source['edges'][meta['sourceContacts'][side]]['curve'])
            carrier=max(carrier,math.dist(world,original.Value(t).Coord()))
            uses=[(fi,c) for fi,f in enumerate(model['faces']) for w in [f['outer']]+f['holes'] for c in model['loops'][w]['coedges'] if c['edge']==edge]
            assert len(uses)==2 and {fi for fi,_ in uses}=={skin,meta['supportFaces'][side]}
            data=[]
            for face in [meta['supportFaces'][side],skin]:
                c=next(c for f,c in uses if f==face);pc=jets.curve(c['pcurve']);q=1-fraction if c['reversed'] else fraction
                uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                p,n,k=jets.jets(jets.surface(model['faces'][face]['surface']),uv.X(),uv.Y());data.append((n,k));position=max(position,math.dist(p,world))
            (n,k),(m,l)=data;sign=1 if jets.dot(n,m)>=0 else -1
            normal=max(normal,math.dist(n,[sign*x for x in m]));tensor=max(tensor,max(abs(k[a][b]-sign*l[a][b]) for a in range(3) for b in range(3)))
            balls.append([world[k]+meta['sides'][side]*radius*n[k] for k in range(3)]);samples+=1
        piece_center=max(piece_center,math.dist(*balls))
    assert piece_center<=receipt['maximumCenterGapMm'][1]+1e-13
    centers=max(centers,piece_center);per_piece.append(piece_center)
assert position<1e-7 and carrier<1e-7 and normal<1e-5 and tensor<1e-4 and centers<1e-7 and radius_drift<1e-7
root_summary={}
if 'roots' in meta:
    assert meta['totalCells']<=100000 and meta['totalCells']==meta['operationCells']+meta['rootCells']
    assert len(meta['roots'])==2
    maximum_root_error=0.;maximum_clock_difference=0.;disjoint_bands=False
    for index,end in enumerate(meta['roots']):
        direction=meta['directions'][end['request']]
        cutter=json.loads((root/f"cutter-{end['request']}.json").read_text())
        face=cutter['faces'][0];assert not face['holes']
        uses=cutter['loops'][face['outer']]['coedges'];assert len(uses)==4
        corners={tuple(c['pcurve']['controlPoints'][0]) for c in uses}
        low=[min(p[k] for p in corners) for k in range(2)];high=[max(p[k] for p in corners) for k in range(2)]
        assert corners=={(u,v) for u in [low[0],high[0]] for v in [low[1],high[1]]}
        for side in range(2):
            parameter=jets.curve(model['edges'][meta['chains'][side][index]]['curve']).LastParameter()
            assert parameter==end['authoredParameters'][side]
            assert end['originalBands'][side][0]<=parameter<=end['originalBands'][side][1]
            point=model['vertices'][end['vertices'][side]]['point']
            observed=math.sqrt(sum(max(abs(x-band[0]),abs(x-band[1]))**2 for x,band in zip(point,end['positionBounds'][side])))
            assert observed<=end['errorUpperMm'][side]+16*math.ulp(end['errorUpperMm'][side])
            maximum_root_error=max(maximum_root_error,observed)
            assert end['errorUpperMm'][side]<=1e-7
            assert abs(point[0]*direction[1]-point[1]*direction[0])/math.hypot(*direction)<1e-7
            uv=end['uvBounds'][side]
            assert all(low[k]<uv[k][0]<=uv[k][1]<high[k] for k in range(2))
            world=jets.surface(face['surface']).Value(*[(a+b)/2 for a,b in uv]).Coord()
            assert math.dist(point,world)<1e-7
        maximum_clock_difference=max(maximum_clock_difference,abs(end['authoredParameters'][0]-end['authoredParameters'][1]))
        a,b=end['originalBands'];disjoint_bands|=a[1]<b[0] or b[1]<a[0]
    assert maximum_root_error<1e-7 and disjoint_bands
    root_summary=dict(rootBindingWithinTolerance=True,maxOriginalRootBoxErrorMm=maximum_root_error,maxAuthoredClockDifference=maximum_clock_difference,nonOverlappingOriginalRootBandsPresent=True)
report=dict(passed=True,topology=counts,volumeMm3=volumes[1],volumeErrorMm3=abs(volumes[1]-volumes[0]),samples=samples,maxPositionErrorMm=position,maxOriginalCarrierErrorMm=carrier,maxNormalChord=normal,maxShapeTensorDifference=tensor,maxBallCenterGapMm=centers,maxRadiusClockDriftMm=radius_drift,pieceBallCenterGapMm=per_piece,lawClockRestartsPerPiece=False,wholeBodyG2=False,globalEmbedding=False,sha256={n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in ['source.json','source.step','model.json','body.step','manifest.json']})
report.update(root_summary)
if 'roots' in meta:
    report['sha256'].update({f'cutter-{i}.json':hashlib.sha256((root/f'cutter-{i}.json').read_bytes()).hexdigest() for i in range(2)})
(root/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
