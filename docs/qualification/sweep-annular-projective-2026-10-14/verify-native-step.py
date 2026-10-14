from pathlib import Path
import json,math,hashlib,sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopAbs import TopAbs_FACE,TopAbs_SOLID,TopAbs_SHELL,TopAbs_EDGE,TopAbs_VERTEX,TopAbs_IN,TopAbs_OUT
from OCP.TopExp import TopExp_Explorer,TopExp
from OCP.TopoDS import TopoDS
from OCP.collections import IndexedDataMap_TopoDS_Shape_List_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.gp import gp_Pnt
root=Path(sys.argv[1]).resolve() if len(sys.argv)>1 else Path(__file__).resolve().parent
path=root/(sys.argv[2] if len(sys.argv)>2 else 'owned-original-annular.step')
reader=STEPControl_Reader()
assert reader.ReadFile(str(path))==IFSelect_RetDone
assert reader.TransferRoots()==1
shape=reader.OneShape()
valid=BRepCheck_Analyzer(shape).IsValid()
counts={}
for key,kind in [('faces',TopAbs_FACE),('solids',TopAbs_SOLID),('shells',TopAbs_SHELL)]:
    it=TopExp_Explorer(shape,kind);count=0
    while it.More():count+=1;it.Next()
    counts[key]=count
edge_map=ShapeMap();vertex_map=ShapeMap()
TopExp.MapShapesAndAncestors_s(shape,TopAbs_EDGE,TopAbs_FACE,edge_map)
TopExp.MapShapesAndAncestors_s(shape,TopAbs_VERTEX,TopAbs_EDGE,vertex_map)
counts.update(edges=edge_map.Extent(),vertices=vertex_map.Extent())
uses=[edge_map.FindFromIndex(i).Size() for i in range(1,edge_map.Extent()+1)]
volumes=[]
for eps in [1e-10,1e-12]:
    props=GProp_GProps();error=BRepGProp.VolumePropertiesGK_s(shape,props,eps,True,True)
    volumes.append({'eps':eps,'volume':props.Mass(),'estimatedError':error})
expected=math.pi*(0.5**2-0.2**2)*(20+math.sqrt(125))
relative=abs(volumes[-1]['volume']-expected)/expected
it=TopExp_Explorer(shape,TopAbs_SOLID)
classifier=BRepClass3d_SolidClassifier(TopoDS.Solid(it.Current()))
probes=[]
for point,inside in [([.35,0,5],True),([0,0,5],False),([.75,0,5],False),([5,.35,10],True),([5,0,10],False),([10.35,5,12.5],True),([10,5,12.5],False),([11,5,12.5],False)]:
    classifier.Perform(gp_Pnt(*point),1e-9)
    state=classifier.State()
    probes.append({'point':point,'expectedInside':inside,'matches':state==(TopAbs_IN if inside else TopAbs_OUT)})
euler=counts['vertices']-counts['edges']+counts['faces']
passed=(valid and counts=={'faces':32,'solids':1,'shells':1,'edges':64,'vertices':32}
    and set(uses)=={2} and euler==0 and relative<=1e-8
    and all(math.isfinite(v['volume']) and v['volume']>0 and math.isfinite(v['estimatedError']) and v['estimatedError']>=0 for v in volumes)
    and abs(volumes[0]['volume']-volumes[1]['volume'])<=1e-10*max(1,expected)
    and all(p['matches'] for p in probes))
report={'scope':'Independent OCCT check of one newly exported native original hollow; not the full STEP/UI matrix or a WASM qualification',
    'stepSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'valid':valid,'counts':counts,'edgeFaceUses':sorted(set(uses)),
    'eulerCharacteristic':euler,'volumes':volumes,'referenceMethod':'analytic circular annulus area times polyline length',
    'referenceVolume':expected,'relativeVolumeError':relative,'materialProbes':probes,'passed':passed}
(root/'source-owned-native-occt.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
raise SystemExit(0 if passed else 1)
