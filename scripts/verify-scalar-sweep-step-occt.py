"""Independent finite STEP import/re-export, topology, surface and volume oracle."""
import hashlib,json,sys,math
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader,STEPControl_Writer,STEPControl_AsIs
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_FACE,TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.gp import gp_Pnt
root=Path(sys.argv[1]);manifest=json.loads((root/'manifest.json').read_text());reports=[]
def read(path):
 r=STEPControl_Reader();assert r.ReadFile(str(path))==IFSelect_RetDone;assert r.TransferRoots()>0;return r.OneShape()
def entities(shape,kind):
 e=TopExp_Explorer(shape,kind);out=[]
 while e.More():out.append(e.Current());e.Next()
 return out
def expected_volume(op):
 if op=='profile_sweep':return 140/3
 # Independent formula: 4*r(v)^2 dz/dv; Simpson, with convergence check.
 def integral(n):
  f=lambda v:40*((3-v)/(3-2*v))**2/(1+v)**2
  return (f(0)+f(1)+sum((4 if i%2 else 2)*f(i/n) for i in range(1,n)))/(3*n)
 a,b=integral(4096),integral(8192);assert abs(a-b)<1e-9;return b
for case in manifest['cases']:
 path=root/case['file'];assert hashlib.sha256(path.read_bytes()).hexdigest()==case['sha256']
 shape=read(path);writer=STEPControl_Writer();assert writer.Transfer(shape,STEPControl_AsIs)==IFSelect_RetDone
 peer=root/(case['op']+'-occt.step');assert writer.Write(str(peer))==IFSelect_RetDone
 stages=[]
 for solid in [shape,read(peer)]:
  assert BRepCheck_Analyzer(solid).IsValid();faces=entities(solid,TopAbs_FACE);assert len(faces)==6;assert len(entities(solid,TopAbs_SOLID))==1
  surfaces=[BRepAdaptor_Surface(TopoDS.Face(f)) for f in faces];worst=0
  for samples in case['wallSamples']:
   errors=[]
   for s in surfaces:
    errors.append(max(s.Value(p['u'],p['v']).Distance(gp_Pnt(*p['point'])) for p in samples))
   worst=max(worst,min(errors))
  assert worst<1e-8,worst
  props=GProp_GProps();BRepGProp.VolumeProperties_s(solid,props);volume=props.Mass();expected=expected_volume(case['op'])
  assert abs(volume-expected)<1e-7*expected,(volume,expected)
  stages.append({'valid':True,'faces':6,'solids':1,'volume':volume,'expectedVolume':expected,'maxSurfaceErrorMm':worst})
 reports.append({'op':case['op'],'nativeRoundTrip':case['nativeRoundTrip'],'occtRoundTrip':stages,'passed':True})
report={'schema':'scalar-sweep-step-occt/1','scope':'Two capped square sweep fixtures; finite geometry, topology and volume verification. No global sweep guarantee.','artifactProvenance':manifest['artifactProvenance'],'cases':reports,'passed':True}
(root/'occt.json').write_text(json.dumps(report,indent=2)+'\n');print('Two independent OCCT STEP round-trips passed',flush=True)
