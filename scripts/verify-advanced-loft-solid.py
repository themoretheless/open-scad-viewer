"""Independent capped natural loft STEP topology, midsection and volume checks."""
import json, math, sys
from fractions import Fraction
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
root=Path(sys.argv[1]);reader=STEPControl_Reader()
assert reader.ReadFile(str(root/'natural-hollow-solid.step'))==IFSelect_RetDone
assert reader.TransferRoots()==1
shape=reader.OneShape();assert BRepCheck_Analyzer(shape).IsValid()
solids=TopExp_Explorer(shape,TopAbs_SOLID);assert solids.More();solids.Next();assert not solids.More()
faces=TopExp_Explorer(shape,TopAbs_FACE);count=0;curved=0;error=0.
while faces.More():
 s=BRepAdaptor_Surface(TopoDS.Face(faces.Current()));count+=1
 if s.GetType().name=='GeomAbs_BSplineSurface' and s.VDegree()==3:
  curved+=1
  for u in [.13,.5,.83]:
   x,y,z=s.Value(u,.5).Coord();r=math.hypot(x,y)
   error=max(error,abs(z-5),min(abs(r-6),abs(r-2)))
 faces.Next()
assert count==10 and curved==8 and error<1e-9,(count,curved,error)
# R(v)=3*s(v), inner radius=s(v); s(v)=1+3*v-4*v**3
# on [0,1/2], mirrored on [1/2,1]. z=10*v.
a=[Fraction(1),Fraction(3),Fraction(0),Fraction(-4)];square=[Fraction(0)]*7
for i,x in enumerate(a):
 for j,y in enumerate(a):square[i+j]+=x*y
integral=2*sum(c*Fraction(1,2)**(i+1)/Fraction(i+1) for i,c in enumerate(square))
expected=80*math.pi*float(integral)
props=GProp_GProps();integration_error=BRepGProp.VolumeProperties_s(shape,props,1e-10,True)
volume=props.Mass();relative=abs(volume-expected)/expected
assert volume>0 and relative<1e-8,(volume,expected,relative)
report=dict(parser='OpenCascade/cadquery-ocp',valid=True,solids=1,faces=count,curved_faces=curved,midsection_max_error=error,volume=volume,independent_analytic_volume=expected,volume_relative_error=relative,ocp_integration_error_estimate=integration_error,scope='one hollow cubic fixture; not global embedding certification')
(root/'opencascade-solid.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
