"""Independent OpenCascade audit of current packaged WASM/Rush loft exports."""
import json,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopAbs import TopAbs_FACE,TopAbs_EDGE,TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.gp import gp_Pnt,gp_Vec
root=Path(sys.argv[1]);manifest=json.loads((root/'wasm-rush-round-trip.json').read_text());reports=[]
for case in manifest['cases']:
 name=case['case'];reader=STEPControl_Reader()
 assert reader.ReadFile(str(root/(name+'.step')))==IFSelect_RetDone,name
 assert reader.TransferRoots()==1,name
 shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid(),name
 faces=[];exp=TopExp_Explorer(shape,TopAbs_FACE)
 while exp.More():faces.append(BRepAdaptor_Surface(TopoDS.Face(exp.Current())));exp.Next()
 assert len(faces)==case['faces'],(name,len(faces))
 solids=0;exp=TopExp_Explorer(shape,TopAbs_SOLID)
 while exp.More():solids+=1;exp.Next()
 assert solids==case['bodies'],(name,solids)
 # STEP face order is independent of source numbering. Require every source
 # face to match one unique imported carrier over all retained sample stations.
 samples=json.loads((root/(name+'.samples.json')).read_text());used=set();position_error=0.;derivative_error=0.
 for index in range(case['faces']):
  group=[s for s in samples if s['face']==index];matches=[]
  for imported,carrier in enumerate(faces):
   for reverse_u in [False,True]:
    error=max(abs(a-b) for s in group for a,b in zip(carrier.Value(1-s['u'] if reverse_u else s['u'],s['v']).Coord(),s['point']))
    if error<1e-9:matches.append((imported,carrier,error,reverse_u))
  assert len(matches)==1,(name,index,len(matches))
  imported,carrier,error,reverse_u=matches[0];assert imported not in used;used.add(imported);position_error=max(position_error,error)
  for sample in group:
   if sample.get('dv') is None:continue
   p,du,dv=gp_Pnt(),gp_Vec(),gp_Vec();carrier.D1(1-sample['u'] if reverse_u else sample['u'],sample['v'],p,du,dv)
   derivative_error=max(derivative_error,*(abs(a-b) for a,b in zip(dv.Coord(),sample['dv'])))
 assert derivative_error<1e-9,(name,derivative_error)
 volume=None;relative=None
 if case['expectedVolume'] is not None:
  props=GProp_GProps();BRepGProp.VolumeProperties_s(shape,props,1e-10,True);volume=props.Mass()
  relative=abs(volume-case['expectedVolume'])/case['expectedVolume'];assert volume>0 and relative<1e-9,(name,volume,relative)
 reports.append(dict(case=name,valid=True,faces=len(faces),solids=solids,samples=len(samples),maxPositionError=position_error,maxVDerivativeError=derivative_error,volume=volume,relativeVolumeError=relative))
assert len(reports)==6
result=dict(parser='OpenCascade/cadquery-ocp',scope='six finite source/STEP geometry, topology and nonplanar-cap volume cases; not global qualification',cases=reports)
(root/'opencascade-matrix.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
