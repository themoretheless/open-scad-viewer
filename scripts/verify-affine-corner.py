#!/usr/bin/env python3
"""Independent samples of variable-frame corners plus STEP solid acceptance.
Continuous original-coefficient proofs remain native; no radius/embedding claim.
"""
import hashlib,importlib.util,json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface,BRepAdaptor_Curve2d
from OCP.GeomAbs import GeomAbs_BSplineSurface
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
import occt_surface_jets as jets
spec=importlib.util.spec_from_file_location('reference',Path(__file__).with_name('verify-mapped-corner-closure.py'))
reference=importlib.util.module_from_spec(spec);spec.loader.exec_module(reference)
root=Path(sys.argv[1]);reports=[]
def compare(a,b):
    p,n,k=a;q,m,l=b;sign=1 if jets.dot(n,m)>=0 else -1
    return [math.dist(p,q),math.dist(n,[sign*x for x in m]),max(abs(k[i][j]-sign*l[i][j]) for i in range(3) for j in range(3))]
for directory in sorted((root/'core').iterdir()):
    s=json.loads((directory/'source.json').read_text());c=json.loads((directory/'candidate.json').read_text());pcs=json.loads((directory/'pcurves.json').read_text());receipt=json.loads((directory/'manifest.json').read_text())
    assert receipt['directAdmitted'] and not receipt['prescribedRadiusLaw'] and not receipt['globalEmbedding']
    assert receipt['cells']<=100000 and receipt['coefficientWork']<=2000000
    assert all(all(0<=b<=t for b,t in zip(side,[1e-7,1e-5,1e-4])) for side in receipt['directUpper'])
    upper=[0.,0.,0.];samples=0
    for side,pc in enumerate(pcs):
        frame=receipt['frames'][side];ends=[]
        for point in pc['controlPoints']:
            end=[sum(frame['linear'][i][j]*point[j] for j in range(2))+frame['offset'][i] for i in range(2)]
            assert all(0<=x<=1 for x in end);ends.append([end[i]-point[i] for i in range(2)])
        assert math.dist(ends[0],ends[-1])>1e-4
        curve=jets.curve(pc)
        for i in range(65):
            t=i/64;uv=curve.Value(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*t)
            actual=[side,t] if side<2 else [t,side-2]
            errors=compare(jets.jets(jets.surface(s),uv.X(),uv.Y()),jets.jets(jets.surface(c),*actual))
            upper=[max(a,b) for a,b in zip(upper,errors)];samples+=1
    assert all(a<b for a,b in zip(upper,[1e-7,1e-5,1e-4]))
    report=dict(passed=True,kind=directory.name,seamSamples=samples,sampledUpper=upper,prescribedRadiusLaw=False,globalEmbedding=False,sha256={n:hashlib.sha256((directory/n).read_bytes()).hexdigest() for n in ['source.json','candidate.json','pcurves.json','manifest.json']})
    (directory/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');reports.append(report)
assert len(reports)==2
body=root/'body';model=json.loads((body/'model.json').read_text());source=json.loads((body/'source.json').read_text());receipt=json.loads((body/'manifest.json').read_text())
assert receipt['variableAffineFrames'] and receipt['allOriginalSeamsProven'] and not receipt['prescribedRadiusLaw'] and not receipt['globalEmbedding']
assert receipt['cells']<=100000 and receipt['coefficientWork']<=2000000 and receipt['exactWork']<=1000000
assert model['edges']==source['edges'] and model['vertices']==source['vertices']
reader=STEPControl_Reader();assert reader.ReadFile(str(body/'body.step'))==IFSelect_RetDone and reader.TransferRoots()
shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
counts={name:len(reference.entities(shape,kind)) for name,kind in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert counts==dict(solids=1,faces=10,edges=20,vertices=12)
props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False);assert abs(props.Mass()-1.)<1e-9
box=Bnd_Box();BRepBndLib.AddOptimal_s(shape,box,False,False)
assert max(abs(a-b) for a,b in zip(box.Get(),[0,0,-1,1,1,0]))<2e-7
faces=[TopoDS.Face_s(f) for f in reference.entities(shape,TopAbs_FACE)]
patches=[f for f in faces if BRepAdaptor_Surface(f).GetType()==GeomAbs_BSplineSurface and BRepAdaptor_Surface(f).UDegree()==5 and BRepAdaptor_Surface(f).VDegree()==5]
assert len(patches)==1;patch=patches[0];edges=reference.entities(patch,TopAbs_EDGE);assert len(edges)==4
upper=[0.,0.,0.];samples=0
for raw in edges:
    edge=TopoDS.Edge_s(raw)
    supports=[f for f in faces if not f.IsSame(patch) and any(edge.IsSame(e) for e in reference.entities(f,TopAbs_EDGE))];assert len(supports)==1
    a=BRepAdaptor_Curve2d(edge,patch);b=BRepAdaptor_Curve2d(edge,supports[0]);assert abs(a.FirstParameter()-b.FirstParameter())<1e-12 and abs(a.LastParameter()-b.LastParameter())<1e-12
    for i in range(65):
        t=a.FirstParameter()+(a.LastParameter()-a.FirstParameter())*i/64
        errors=compare(reference.properties(patch,a.Value(t)),reference.properties(supports[0],b.Value(t)))
        upper=[max(a,b) for a,b in zip(upper,errors)];samples+=1
assert all(a<b for a,b in zip(upper,[1e-7,1e-5,1e-4]))
report=dict(passed=True,kind='STEP body',topology=counts,volumeMm3=props.Mass(),bounds=box.Get(),seamSamples=samples,sampledUpper=upper,prescribedRadiusLaw=False,globalEmbedding=False,sha256={n:hashlib.sha256((body/n).read_bytes()).hexdigest() for n in ['model.json','source.json','body.step','manifest.json']})
(body/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');reports.append(report)
(root/'independent-check.json').write_text(json.dumps(reports,indent=2)+'\n');print(json.dumps(reports,indent=2))
