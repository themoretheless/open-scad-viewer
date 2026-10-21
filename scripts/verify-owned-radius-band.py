#!/usr/bin/env python3
"""Independent original/result material probes for the owned quarter-band edit."""
import json, math, sys
import occt_surface_jets as jets
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.TopAbs import TopAbs_IN, TopAbs_OUT
from OCP.gp import gp_Pnt
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
import importlib.util
spec=importlib.util.spec_from_file_location("radius_check",Path(__file__).with_name("verify-radius-transition-occt.py"))
check=importlib.util.module_from_spec(spec);spec.loader.exec_module(check)
root=Path(sys.argv[1])
actual_owned='--actual-owned' in sys.argv[2:]
if actual_owned:
    manifest=json.loads((root/'manifest.json').read_text())
    assert manifest['actualStoredContactsG2'] is True
    assert manifest['coefficientWork']<=2000000 and manifest['cells']<=100000 and manifest['exactWork']<=1000000
    chains=manifest.get('smoothChains',[[e] for e in manifest['smoothEdges']])
    assert len(chains)==2 and all(1<=len(c)<=64 for c in chains)
    assert len({e for c in chains for e in c})==sum(map(len,chains))
    assert len(manifest['ownedContactDecisions'])==sum(map(len,chains))
    assert {p['edge'] for p in manifest['ownedContactDecisions']}=={e for c in chains for e in c}
    assert all(p['decision']=='WithinTolerance' and 0<p['cells']<=manifest['cells'] and p['positionUpperMm']<=1e-7 for p in manifest['ownedContactDecisions'])
check.main(root)
contact_samples=0;position=normal=tensor=0.
if actual_owned:
    source_model=json.loads((root/'source.json').read_text());model=json.loads((root/'model.json').read_text())
    assert model['vertices']==source_model['vertices']
    assert model['topologyIds']['vertices']==source_model['topologyIds']['vertices']
    for side,chain in enumerate(chains):
        actual=[c for c in model['loops'][model['faces'][3]['outer']]['coedges'] if all(p[1]==side for p in c['pcurve']['controlPoints'])]
        actual.sort(key=lambda c:min(p[0] for p in c['pcurve']['controlPoints']))
        assert [c['edge'] for c in actual]==chain
        bands=[sorted(p[0] for p in c['pcurve']['controlPoints']) for c in actual]
        assert bands[0][0]==0 and bands[-1][1]==1 and all(a[1]==b[0] for a,b in zip(bands,bands[1:]))
        for edge in chain:
            assert model['edges'][edge]==source_model['edges'][edge]
            assert model['topologyIds']['edges'][edge]==source_model['topologyIds']['edges'][edge]
            uses=[(fi,c) for fi,f in enumerate(model['faces']) for w in [f['outer']]+f['holes'] for c in model['loops'][w]['coedges'] if c['edge']==edge]
            assert len(uses)==2 and sum(fi==3 for fi,_ in uses)==1
            curve=jets.curve(model['edges'][edge]['curve'])
            for i in range(65):
                q=i/64;point=curve.Value(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*q).Coord();data=[]
                for fi,c in uses:
                    pc=jets.curve(c['pcurve']);t=1-q if c['reversed'] else q
                    uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*t)
                    p,n,k=jets.jets(jets.surface(model['faces'][fi]['surface']),uv.X(),uv.Y())
                    position=max(position,math.dist(p,point));data.append((n,k))
                (n,k),(m,l)=data;sign=1 if jets.dot(n,m)>=0 else -1
                normal=max(normal,math.dist(n,[sign*x for x in m]));tensor=max(tensor,max(abs(k[a][b]-sign*l[a][b]) for a in range(3) for b in range(3)))
                contact_samples+=1
    assert position<=1e-7 and normal<=1e-5 and tensor<=1e-4

def read(name):
    r=STEPControl_Reader();assert r.ReadFile(str(root/name))==IFSelect_RetDone and r.TransferRoots()
    s=r.OneShape();assert BRepCheck_Analyzer(s).IsValid();return s
source,result=read("source.step"),read("transition.step")
probes=[]
for point,expected in [((.14,.01,.025),(True,False)),((.27,.01,.075),(False,True))]:
    states=[]
    for shape in [source,result]:
        c=BRepClass3d_SolidClassifier(shape,gp_Pnt(*point),1e-8)
        assert c.State() in [TopAbs_IN,TopAbs_OUT];states.append(c.State()==TopAbs_IN)
    assert tuple(states)==expected;probes.append({"pointMm":point,"insideBeforeAfter":states})
g=GProp_GProps();BRepGProp.VolumeProperties_s(source,g,1e-10,True,False)
expected=math.pi/4*.1*(.1**2+.3**2+(.1**2+.1*.3+.3**2)/3)
assert abs(g.Mass()-expected)<1e-9
report={"passed":True,"actualOwnedMode":actual_owned,"sourceValid":True,"sourceVolumeMm3":g.Mass(),"expectedSourceVolumeMm3":expected,"materialProbes":probes,"contactSeamSamples":contact_samples,"maximumContactPositionErrorMm":position,"maximumContactNormalError":normal,"maximumContactTensorError":tensor,"scope":"owned coaxial quarter band; complete fragmented circular joins; sampled checks supplement native proofs"}
(root/"owned-material-check.json").write_text(json.dumps(report,indent=2)+"\n")
print(json.dumps(report,indent=2))
