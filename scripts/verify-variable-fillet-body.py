#!/usr/bin/env python3
"""Independent closed STEP, actual stored G2 seams and ball-center law samples.

This does not claim circular G2 cross sections or global self-intersection proof.
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
spec=importlib.util.spec_from_file_location('radius',Path(__file__).with_name('verify-radius-transition-occt.py'))
helpers=importlib.util.module_from_spec(spec);spec.loader.exec_module(helpers)
root=Path(sys.argv[1]);reports=[]
for directory in sorted(root.glob('case-*')):
    m=json.loads((directory/'model.json').read_text());s=json.loads((directory/'source.json').read_text());receipt=json.loads((directory/'manifest.json').read_text())
    assert receipt['cells']<=100000 and receipt['exactWork']<=1000000 and receipt['radiusErrorUpperMm']<=1e-7
    assert receipt['wholeBodyG2'] is False and receipt['constantCircularCrossSectionRadius'] is False
    assert [f['surface'] for f in m['faces'][:len(s['faces'])]]==[f['surface'] for f in s['faces']]
    reader=STEPControl_Reader();assert reader.ReadFile(str(directory/'body.step'))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
    counts={name:len(helpers.entities(shape,kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    assert counts==dict(solids=1,faces=len(m['faces']),edges=len(m['edges']),vertices=len(m['vertices']))
    box=Bnd_Box();BRepBndLib.AddOptimal_s(shape,box,False,False)
    assert max(abs(a-b) for a,b in zip(box.Get(),[0.,2.,0.,10.,8.,10.]))<2e-7
    props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False)
    assert 0<props.Mass()<600
    for v in receipt['roots']:assert min(abs(m['vertices'][v]['point'][1]-y) for y in [2.,8.])<1e-7
    endpoint_mode='--endpoints' in sys.argv[2:]
    if '--window' in sys.argv[2:]:
        w=receipt['referenceWindow']
        assert 0<=w[0]<w[1]<=1
        if directory.name in ['case-3','case-4']:assert w!=[0.,1.]
    if endpoint_mode:
        assert 1<=receipt['attempts']<=8
        assert all(e<=1e-7 for pair in receipt['endpointErrorUpperMm'] for e in pair)
    maximum_position=maximum_normal=maximum_tensor=maximum_centers=0.;samples=0
    contacts=[]
    for side,edge in enumerate(receipt['contacts']):
        uses=[(fi,c) for fi,f in enumerate(m['faces']) for w in [f['outer']]+f['holes'] for c in m['loops'][w]['coedges'] if c['edge']==edge]
        assert len(uses)==2
        original=next(c for f,c in uses if f==receipt['supportFaces'][side]);saved=next(c for f,c in uses if f==receipt['skin'])
        contacts.append((jets.curve(m['edges'][edge]['curve']),original,saved,receipt['supportFaces'][side]))
    endpoint_error=0.
    if endpoint_mode:
        lo,hi=receipt['lawDomain'];r0,r1=receipt['lawEndpoints']
        for curve,_,_,_ in contacts:
            for end,t in enumerate([curve.FirstParameter(),curve.LastParameter()]):
                value=r0+(r1-r0)*(t-lo)/(hi-lo)
                desired=receipt['requestedEndRadiiMm'][receipt['endOrder'][end]]
                endpoint_error=max(endpoint_error,abs(value-desired))
        assert endpoint_error<=1e-7
    for i in range(65):
        fraction=i/64;centers=[]
        for curve,original,saved,fi in contacts:
            t=curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*fraction
            world=curve.Value(t).Coord();data=[]
            for face,c in [(fi,original),(receipt['skin'],saved)]:
                pc=jets.curve(c['pcurve']);q=1-fraction if c['reversed'] else fraction
                uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                p,n,k=jets.jets(jets.surface(m['faces'][face]['surface']),uv.X(),uv.Y());data.append((p,n,k))
                maximum_position=max(maximum_position,math.dist(p,world))
            (_,n,k),(_,n2,k2)=data;sign=1 if jets.dot(n,n2)>=0 else -1
            maximum_normal=max(maximum_normal,math.dist(n,[sign*x for x in n2]))
            maximum_tensor=max(maximum_tensor,max(abs(k[a][b]-sign*k2[a][b]) for a in range(3) for b in range(3)))
            lo,hi=receipt['lawDomain'];r0,r1=receipt['lawEndpoints'];radius=r0+(r1-r0)*(t-lo)/(hi-lo)
            centers.append([world[k]+radius*n[k] for k in range(3)]);samples+=1
        maximum_centers=max(maximum_centers,math.dist(*centers))
    assert maximum_position<=1e-7 and maximum_normal<=1e-5 and maximum_tensor<=1e-4 and maximum_centers<=1e-7
    report=dict(case=directory.name,passed=True,topology=counts,volumeMm3=props.Mass(),samples=samples,maxPositionErrorMm=maximum_position,maxNormalChord=maximum_normal,maxShapeTensorDifference=maximum_tensor,maxBallCenterGapMm=maximum_centers,endpointMode=endpoint_mode,maxRequestedEndpointErrorMm=endpoint_error if endpoint_mode else None,lawEndpoints=receipt['lawEndpoints'],retainedContactLawEndpoints=[[receipt['lawEndpoints'][0]+(receipt['lawEndpoints'][1]-receipt['lawEndpoints'][0])*(t-receipt['lawDomain'][0])/(receipt['lawDomain'][1]-receipt['lawDomain'][0]) for t in [c.FirstParameter(),c.LastParameter()]] for c,_,_,_ in contacts],wholeBodyG2=False,constantCircularCrossSectionRadius=False,globalEmbedding=False,sha256={name:hashlib.sha256((directory/name).read_bytes()).hexdigest() for name in ['model.json','source.json','body.step','manifest.json']})
    (directory/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');reports.append(report)
assert len(reports)==(5 if '--window' in sys.argv[2:] else 3)
(root/'independent-check.json').write_text(json.dumps(reports,indent=2)+'\n');print(json.dumps(reports,indent=2))
