#!/usr/bin/env python3
"""Independent STEP checks for original-body edge/root binding preparation."""
import hashlib,importlib.util,json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.TopExp import TopExp_Explorer
root=Path(sys.argv[1]);m=json.loads((root/'manifest.json').read_text())
for name,key in [('bound-model.json','boundModelSha256'),('bound-source.step','boundStepSha256')]:
    assert hashlib.sha256((root/name).read_bytes()).hexdigest()==m[key]
assert m['brepEndpointsBound'] and m['originalEdgesQueried']==2
r=STEPControl_Reader();assert r.ReadFile(str(root/'bound-source.step'))==IFSelect_RetDone and r.TransferRoots()
shape=r.OneShape();assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
def entities(shape,kind):
    result=[];e=TopExp_Explorer(shape,kind)
    while e.More():
        item=e.Current()
        if not any(item.IsSame(old) for old in result):result.append(item)
        e.Next()
    return result
topology={name:len(entities(shape,k)) for name,k in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert topology==m['boundTopology']==dict(solids=1,faces=6,edges=16,vertices=12)
g=GProp_GProps();BRepGProp.VolumeProperties_s(shape,g,1e-10,True,False)
assert abs(g.Mass()-m['expectedSourceVolumeMm3'])<1e-10
checks=[]
for pair in m['bindings']:
    for binding in pair:
        point=binding['pointMm'];matches=[]
        for v in entities(shape,TopAbs_VERTEX):
            p=BRep_Tool.Pnt_s(TopoDS.Vertex_s(v));distance=math.dist(point,[p.X(),p.Y(),p.Z()])
            if distance<1e-9:matches.append(v)
        assert len(matches)==1;v=matches[0]
        incident_edges=sum(any(w.IsSame(v) for w in entities(e,TopAbs_VERTEX)) for e in entities(shape,TopAbs_EDGE))
        incident_faces=sum(any(w.IsSame(v) for w in entities(f,TopAbs_VERTEX)) for f in entities(shape,TopAbs_FACE))
        assert incident_edges==2 and incident_faces==2
        checks.append(dict(sourceEdge=binding['sourceEdge'],ordinal=binding['ordinal'],pointMm=point,incidentEdges=incident_edges,incidentFaces=incident_faces))
assert len(checks)==4
report=dict(passed=True,topology=topology,volumeMm3=g.Mass(),volumeErrorMm3=abs(g.Mass()-m['expectedSourceVolumeMm3']),roots=checks,scope='unchanged source body with shared original-edge splits; no new fillet body')
(root/'independent-bound-source.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
