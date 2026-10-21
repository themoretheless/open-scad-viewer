#!/usr/bin/env python3
"""Independent STEP qualification of saved open intersection chains/partitions.

Sampling supplements the native whole-interval trace proofs; it does not prove
mixed-axis side construction, new fillet geometry or global self-intersections.
"""
import hashlib,json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAbs import GeomAbs_BSplineCurve
from OCP.TopExp import TopExp_Explorer
root=Path(sys.argv[1]);m=json.loads((root/'manifest.json').read_text())
assert m['changedClosedBodyClaim']
reference=json.loads((root/'expected-changed-volume.json').read_text())
for name,key in [('source.json','sourceSha256'),('cutter.json','cutterSha256')]:
    assert hashlib.sha256((root/name).read_bytes()).hexdigest()==reference[key]

expected=float(reference['expectedChangedVolumeMm3'])
assert m['cells']<=100000 and m['totalExactWork']<=1000000 and m['guideEvaluations']<=4096
r=STEPControl_Reader();assert r.ReadFile(str(root/'changed.step'))==IFSelect_RetDone and r.TransferRoots()
shape=r.OneShape();assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
def entities(shape,kind):
    result=[];e=TopExp_Explorer(shape,kind)
    while e.More():
        item=e.Current()
        if not any(item.IsSame(old) for old in result):result.append(item)
        e.Next()
    return result
topology={name:len(entities(shape,k)) for name,k in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
assert topology==dict(solids=1,**m['changedTopology'])==dict(solids=1,faces=8,edges=128,vertices=122)
g=GProp_GProps();BRepGProp.VolumeProperties_s(shape,g,1e-10,True,False)
assert abs(g.Mass()-expected)<1e-9
maximum=0.;curves=[];samples=0
for edge in entities(shape,TopAbs_EDGE):
    e=TopoDS.Edge_s(edge);curve=BRepAdaptor_Curve(e)
    if curve.GetType()!=GeomAbs_BSplineCurve or curve.Degree()!=3:continue
    curves.append(e);lo,hi=curve.FirstParameter(),curve.LastParameter()
    for i in range(65):
        p=curve.Value(lo+(hi-lo)*i/64)
        residual=max(min(abs(p.Z()-0.2),abs(p.X()-0.5),abs(p.X()+0.5)),abs(p.Z()-(p.Y()**2+0.005*p.X()**2-0.02)))
        maximum=max(maximum,residual);samples+=1
    uses=sum(any(e.IsSame(other) for other in entities(face,TopAbs_EDGE)) for face in entities(shape,TopAbs_FACE))
    assert uses==2
assert len(curves)==sum(len(b) for b in m['networkBranchEdges'])==114 and maximum<=1e-7
bound=json.loads((root/'bound-source.json').read_text())
root_vertices=set(v for side in m['sides'] for chain in side['savedChains'] for v in chain['vertices'])
assert len(root_vertices)==4
for index in root_vertices:
    point=bound['vertices'][index]['point']; matches=[]
    for v in entities(shape,TopAbs_VERTEX):
        p=BRep_Tool.Pnt_s(TopoDS.Vertex_s(v))
        if math.dist(point,[p.X(),p.Y(),p.Z()])<1e-9:matches.append(v)
    assert len(matches)==1
    v=matches[0]
    assert sum(any(w.IsSame(v) for w in entities(e,TopAbs_VERTEX)) for e in entities(shape,TopAbs_EDGE))==3
    assert sum(any(w.IsSame(v) for w in entities(f,TopAbs_VERTEX)) for f in entities(shape,TopAbs_FACE))==3
report=dict(passed=True,topology=topology,volumeMm3=g.Mass(),volumeErrorMm3=abs(g.Mass()-expected),
            sharedSavedCurveEdges=len(curves),curveSamples=samples,maximumIntersectionResidualMm=maximum,
            originalEndpointVerticesShared=True,scope='control connected top/side material region replaced; volume compared with actual-coefficient independent reference; not a general fillet or global embedding proof')
(root/'independent-changed-step.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
