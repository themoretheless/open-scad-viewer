#!/usr/bin/env python3
"""Independent STEP controls for original interior-trim chain insertion/capping."""
import hashlib,json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAbs import GeomAbs_BSplineCurve
root=Path(sys.argv[1]);meta=json.loads((root/'manifest.json').read_text())
automatic='--automatic' in sys.argv[2:]
if automatic:
    assert meta.get('automaticGraphRegionReplacement') is True
    assert 0 < meta['replacementDomainCells'] <= 100000
reference=json.loads((root/'expected-changed-volume.json').read_text())
for name,key in [('source.json','sourceSha256'),('cutter.json','cutterSha256')]:
    assert hashlib.sha256((root/name).read_bytes()).hexdigest()==reference[key]
assert meta['networkCells']+meta['replacementCells']<=100000
assert meta['networkExactWork']+meta['replacementExactWork']<=1000000
assert meta['controlRegionReplacement'] and not meta['arbitraryBrepQualified']
trace_count=meta.get('savedTraceCount',72);assert 1<=trace_count<=1024
cutter=json.loads((root/'cutter.json').read_text())['faces'][0]['surface']
assert cutter['degreeU']==cutter['degreeV']==2 and cutter['weights']==[[1,1,1]]*3
cp=cutter['controlPoints'];assert all(p[:2]==[-1+i,-1+j] for i,row in enumerate(cp) for j,p in enumerate(row))
def graph_height(x,y):
    u=(x+1)/2;v=(y+1)/2;a=[(1-u)**2,2*u*(1-u),u*u];b=[(1-v)**2,2*v*(1-v),v*v]
    return sum(cp[i][j][2]*a[i]*b[j] for i in range(3) for j in range(3))
def entities(shape,kind):
    result=[];iterator=TopExp_Explorer(shape,kind)
    while iterator.More():
        value=iterator.Current()
        if not any(value.IsSame(old) for old in result):result.append(value)
        iterator.Next()
    return result
def read(name,model_name,expected):
    reader=STEPControl_Reader();assert reader.ReadFile(str(root/name))==IFSelect_RetDone and reader.TransferRoots()
    shape=reader.OneShape();assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    model=json.loads((root/model_name).read_text())
    topology={n:len(entities(shape,k)) for n,k in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]}
    assert topology==dict(solids=1,faces=len(model['faces']),edges=len(model['edges']),vertices=len(model['vertices']))
    props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True,False)
    error=abs(props.Mass()-expected);assert error<1e-9
    return shape,dict(topology=topology,volumeMm3=props.Mass(),referenceVolumeMm3=expected,volumeErrorMm3=error)
partition,partition_report=read('partition.step','partition.json',float(reference['sourceVolumeMm3']))
body,body_report=read('body.step','model.json',float(reference['expectedChangedVolumeMm3']))
assert partition_report['topology']==dict(solids=1,faces=10,edges=trace_count+16,vertices=trace_count+8)
assert body_report['topology']==dict(solids=1,faces=8,edges=trace_count+14,vertices=trace_count+8)
maximum=0.;saved=0;samples=0
for value in entities(body,TopAbs_EDGE):
    edge=TopoDS.Edge_s(value);curve=BRepAdaptor_Curve(edge)
    if curve.GetType()!=GeomAbs_BSplineCurve or curve.Degree()!=3:continue
    saved+=1;assert sum(any(edge.IsSame(other) for other in entities(face,TopAbs_EDGE)) for face in entities(body,TopAbs_FACE))==2
    for i in range(65):
        point=curve.Value(curve.FirstParameter()+(curve.LastParameter()-curve.FirstParameter())*i/64)
        residual=max(min(abs(point.Z()-0.2),abs(point.X()-0.5),abs(point.X()+0.5)),
            abs(point.Z()-graph_height(point.X(),point.Y())))
        maximum=max(maximum,residual);samples+=1
assert saved==trace_count and maximum<1e-7
bound=json.loads((root/'bound-source.json').read_text());roots=bound['vertices'][8:];assert len(roots)==4
for root_vertex in roots:
    point=root_vertex['point'];matches=[v for v in entities(body,TopAbs_VERTEX) if math.dist(point,BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)).Coord())<1e-9]
    assert len(matches)==1;vertex=matches[0]
    assert sum(any(vertex.IsSame(v) for v in entities(e,TopAbs_VERTEX)) for e in entities(body,TopAbs_EDGE))==3
    assert sum(any(vertex.IsSame(v) for v in entities(f,TopAbs_VERTEX)) for f in entities(body,TopAbs_FACE))==3
report=dict(passed=True,automaticMode=automatic,partition=partition_report,replacement=body_report,sharedSavedEdges=saved,curveSampleSites=samples,
    maximumSampledIntersectionResidualMm=maximum,originalRootVerticesShared=True,
    scope='control interior-trim chains inserted and selected region capped; independent volume quadrature/STEP controls; not arbitrary trimming, general fillet or global embedding qualification')
(root/'independent-step.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
