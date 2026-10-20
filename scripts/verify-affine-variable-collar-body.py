#!/usr/bin/env python3
"""Independent STEP and sampled saved-contact checks of variable end collars."""
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
spec=importlib.util.spec_from_file_location('radius',Path(__file__).with_name('verify-radius-transition-occt.py'));helpers=importlib.util.module_from_spec(spec);spec.loader.exec_module(helpers)
root=Path(sys.argv[1]);reports=[]
for directory in sorted(root.glob('case-*')):
    m=json.loads((directory/'model.json').read_text());s=json.loads((directory/'source.json').read_text());p=json.loads((directory/'manifest.json').read_text())
    assert p['nativeWholeSeamG2Proven'] and p['wholeBodyG2'] is False and p['cells']<=100000 and p['exactWork']<=1000000 and p['coefficientWork']<=2000000
    assert p['radiusErrorUpperMm']<=1e-7
    assert [f['surface'] for f in m['faces'][:len(s['faces'])]]==[f['surface'] for f in s['faces']]
    reader=STEPControl_Reader();assert reader.ReadFile(str(directory/'body.step'))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
    counts={name:len(helpers.entities(shape,kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    assert counts==dict(solids=1,faces=len(m['faces']),edges=len(m['edges']),vertices=len(m['vertices']))
    box=Bnd_Box();BRepBndLib.AddOptimal_s(shape,box,False,False)
    assert max(abs(a-b) for a,b in zip(box.Get(),[0.,0.,-0.2,1.,1.,0.4]))<2e-7
    props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False);assert 0<props.Mass()<math.pi*0.6/4
    contacts=[]
    for side,edge in enumerate(p['contacts']):
        uses=[(fi,c) for fi,f in enumerate(m['faces']) for w in [f['outer']]+f['holes'] for c in m['loops'][w]['coedges'] if c['edge']==edge];assert len(uses)==2
        original=next(c for f,c in uses if f==p['supportFaces'][side]);saved=next(c for f,c in uses if f==p['skin'])
        contacts.append((jets.curve(m['edges'][edge]['curve']),original,saved,p['supportFaces'][side]))
    r0,r1=p['radiiMm'];pad=p['padding'];full=[r0-pad*(r1-r0),r1+pad*(r1-r0)]
    position=normal=tensor=centers=end_error=0.;samples=0
    for i in range(65):
        fraction=i/64;ball=[]
        for side,(curve,original,saved,fi) in enumerate(contacts):
            t=curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*fraction
            world=curve.Value(t).Coord();data=[]
            for face,c in [(fi,original),(p['skin'],saved)]:
                pc=jets.curve(c['pcurve']);q=1-fraction if c['reversed'] else fraction;uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*q)
                a,n,k=jets.jets(jets.surface(m['faces'][face]['surface']),uv.X(),uv.Y());data.append((n,k));position=max(position,math.dist(a,world))
            (n,k),(n2,k2)=data;sign=1 if jets.dot(n,n2)>=0 else -1
            normal=max(normal,math.dist(n,[sign*x for x in n2]));tensor=max(tensor,max(abs(k[a][b]-sign*k2[a][b]) for a in range(3) for b in range(3)))
            radius=full[0]+t*(full[1]-full[0]);sense=1 if side==0 else -1;ball.append([world[a]+sense*radius*n[a] for a in range(3)])
            if i in [0,64]:end_error=max(end_error,abs(radius-[r0,r1][int(i==64)]))
            samples+=1
        centers=max(centers,math.dist(*ball))
    assert position<1e-7 and normal<1e-5 and tensor<1e-4 and centers<1e-7 and end_error<1e-7
    report=dict(passed=True,savedBodyAdmitted=True,nativeWholeContactG2Proven=True,wholeBodyG2=False,globalEmbedding=False,topology=counts,volumeMm3=props.Mass(),samples=samples,maxPositionErrorMm=position,maxNormalChord=normal,maxShapeTensorDifference=tensor,maxBallCenterGapMm=centers,maxEndpointRadiusErrorMm=end_error,sha256={name:hashlib.sha256((directory/name).read_bytes()).hexdigest() for name in ['model.json','source.json','body.step','manifest.json']})
    (directory/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');reports.append(report)
assert len(reports)==2
(root/'independent-check.json').write_text(json.dumps(reports,indent=2)+'\n');print(json.dumps(reports,indent=2))
