#!/usr/bin/env python3
"""Independent STEP and saved-seam checks for two original cutter roots.
The sampled checks supplement continuous native gates, not global embedding.
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
import occt_surface_jets as jets
spec=importlib.util.spec_from_file_location('reference',Path(__file__).with_name('verify-radius-transition-occt.py'))
reference=importlib.util.module_from_spec(spec);spec.loader.exec_module(reference)
root=Path(sys.argv[1]);reports=[]
for directory in sorted(root.glob('side-*')):
    source=json.loads((directory/'source.json').read_text())
    model=json.loads((directory/'model.json').read_text())
    receipt=json.loads((directory/'manifest.json').read_text())
    assert receipt['rootsIsolated'] and receipt['allFinalChildG2Proven']
    assert not receipt['wholeBodyG2'] and not receipt['globalEmbedding']
    assert receipt['cells']<=100000 and receipt['exactWork']<=1000000 and receipt['coefficientWork']<=2000000
    assert len(receipt['roots'])==2 and len(receipt['childEdges'])==3
    assert model['faces']==source['faces']
    shapes=[];volumes=[]
    for name in ['source.step','body.step']:
        reader=STEPControl_Reader();assert reader.ReadFile(str(directory/name))==IFSelect_RetDone and reader.TransferRoots()
        shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid();shapes.append(shape)
        props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False);volumes.append(props.Mass())
    counts={name:len(reference.entities(shapes[1],kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    assert counts==dict(solids=1,faces=len(source['faces']),edges=len(source['edges'])+2,vertices=len(source['vertices'])+2)
    assert abs(volumes[0]-volumes[1])<1e-9
    original=jets.curve(source['edges'][receipt['sourceEdge']]['curve'])
    max_root=max_carrier=position=normal=tensor=0.;samples=0
    for i,end in enumerate(receipt['roots']):
        assert end['children']==receipt['childEdges'][i:i+2]
        if i:assert receipt['roots'][i-1]['parameterBand'][1]<end['parameterBand'][0]
        point=model['vertices'][end['vertex']]['point']
        child=jets.curve(model['edges'][end['children'][0]]['curve']);t=child.LastParameter()
        assert end['parameterBand'][0]<=t<=end['parameterBand'][1]
        max_root=max(max_root,math.dist(point,original.Value(t).Coord()))
        graph=point[0]-receipt['cutterGraphCoefficient']*point[1]*point[2]
        assert abs(graph-receipt['cutterX'][end['request']])<1e-7
        assert end['errorUpperMm']<=1e-7
        for x,band in zip(point,end['positionBounds']):assert band[0]-1e-7<=x<=band[1]+1e-7
        for edge in end['children']:assert end['vertex'] in model['edges'][edge]['vertices']
    for edge in receipt['childEdges']:
        curve=jets.curve(model['edges'][edge]['curve'])
        uses=[(fi,c) for fi,f in enumerate(model['faces']) for w in [f['outer']]+f['holes'] for c in model['loops'][w]['coedges'] if c['edge']==edge]
        assert len(uses)==2 and {fi for fi,_ in uses}==set(receipt['faces'])
        for i in range(65):
            fraction=i/64;t=curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*fraction
            world=curve.Value(t).Coord();max_carrier=max(max_carrier,math.dist(world,original.Value(t).Coord()));data=[]
            for face,c in uses:
                pc=jets.curve(c['pcurve']);q=1-fraction if c['reversed'] else fraction
                uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                p,n,k=jets.jets(jets.surface(model['faces'][face]['surface']),uv.X(),uv.Y());data.append((n,k));position=max(position,math.dist(world,p))
            (n,k),(n2,k2)=data;sign=1 if jets.dot(n,n2)>=0 else -1
            normal=max(normal,math.dist(n,[sign*x for x in n2]))
            tensor=max(tensor,max(abs(k[a][b]-sign*k2[a][b]) for a in range(3) for b in range(3)))
            samples+=1
    assert max_root<1e-7 and max_carrier<1e-7 and position<1e-7 and normal<1e-5 and tensor<1e-4
    report=dict(passed=True,topology=counts,volumeMm3=volumes[1],volumeErrorMm3=abs(volumes[0]-volumes[1]),seamSamples=samples,maxRootCarrierErrorMm=max_root,maxCarrierErrorMm=max_carrier,maxPositionErrorMm=position,maxNormalChord=normal,maxShapeTensorDifference=tensor,globalEmbedding=False,sha256={n:hashlib.sha256((directory/n).read_bytes()).hexdigest() for n in ['source.json','source.step','model.json','body.step','manifest.json']})
    (directory/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');reports.append(report)
assert len(reports)==2
(root/'independent-check.json').write_text(json.dumps(reports,indent=2)+'\n');print(json.dumps(reports,indent=2))
