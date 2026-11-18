"""Independent headless OCCT evidence for the new original quintic-profile prism."""
from pathlib import Path
import hashlib,json,sys,io,math
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX,TopAbs_IN,TopAbs_OUT
from OCP.TopExp import TopExp_Explorer
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BOPAlgo import BOPAlgo_CheckerSI
from OCP.gp import gp_Pnt
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.GeomAbs import GeomAbs_BSplineSurface
root=Path(sys.argv[1]);path=root/'body.step'
# The test uses the original, untransformed source coordinates.
def point(x,y,z):return gp_Pnt(x,y,z)
reader=STEPControl_Reader();assert reader.ReadFile(str(path))==IFSelect_RetDone and reader.TransferRoots()==1
shape=reader.OneShape();valid=BRepCheck_Analyzer(shape).IsValid()
def count(kind):
 ex=TopExp_Explorer(shape,kind);items=[]
 while ex.More():
  s=ex.Current()
  if not any(s.IsSame(v)for v in items):items.append(s)
  ex.Next()
 return len(items)
diagnostic=GProp_GProps();standard_estimate=BRepGProp.VolumeProperties_s(shape,diagnostic,Eps=1e-10)
# High-order rational trims need the kernel's adaptive Gauss-Kronrod owner.
# Keep exactly the previous integration Eps and acceptance tolerance. The
# standard Gauss diagnostic is retained, including disagreement with the oracle.
# Acceptance keeps the former ACTUAL absolute error <=1e-7 against an exact
# source-volume enclosure. Neither kernel error estimate is an exact bound.
# Report requested-Eps misses explicitly; never promote them to convergence.
props=GProp_GProps();estimate=BRepGProp.VolumePropertiesGK_s(shape,props,1e-10,True,True)
# x=10q^5, y=10(q²+q/4), chord y=10+x/4, z in [0,2].
# Volume reference is an independent exactly enclosed rational Green integral.
expected=float(json.loads((root/'exact-oracle.json').read_text())['highPrecisionVolume']);volume=props.Mass();error=abs(volume-expected)
topology=dict(solids=count(TopAbs_SOLID),faces=count(TopAbs_FACE),edges=count(TopAbs_EDGE),vertices=count(TopAbs_VERTEX))
# Use the authored parameter correspondence, not a closest-point optimizer
# that can stop at another critical point on this high-order projection.
walls=[];ex=TopExp_Explorer(shape,TopAbs_FACE)
while ex.More():
 a=BRepAdaptor_Surface(TopoDS.Face(ex.Current()),True)
 if a.GetType()==GeomAbs_BSplineSurface and a.BSpline().UDegree()==5:
  walls.append(a)
 ex.Next()
assert len(walls)==2
walls.sort(key=lambda a:a.Value(0.,0.).Z())
points=[];parameter_errors=[];classification=[]
for q in [-1.,-.75,-.125,0.,.125,.5,1.]:
 W=(31-10*q*q-5*q**4)/16
 x=10*q**5/W;y=10*(q*q+q/4)/W
 for z in [0.,.25,.5,.75,1.,1.25,1.75,2.]:
  d=BRepExtrema_DistShapeShape(BRepBuilderAPI_MakeVertex(point(x,y,z)).Vertex(),shape)
  d.Perform();assert d.IsDone();points.append(d.Value())
  section=0 if z<=1 else 1
  u=(q+1)/2
  t=z-section
  v=t if walls[section].BSpline().VDegree()==1 else t/(1-t+math.sqrt(1-3*t+3*t*t))
  original=walls[section].Value(u,v)
  parameter_errors.append(max(abs(original.X()-point(x,y,z).X()),abs(original.Y()-point(x,y,z).Y()),abs(original.Z()-point(x,y,z).Z())))
 if abs(q)<1:
  top=10+x/4
  for yy,inside in [((y+top)/2,True),(top+1,False),(y-1,False)]:
   c=BRepClass3d_SolidClassifier(shape,point(x,yy,1.),1e-7)
   classification.append(c.State()==(TopAbs_IN if inside else TopAbs_OUT))
checker=BOPAlgo_CheckerSI();checker.AddArgument(shape);checker.SetLevelOfCheck(5);checker.Perform()
interferences=None if checker.HasErrors()else checker.DS().Interferences().Extent()
passed=estimate>=0 and valid and topology==dict(solids=1,faces=8,edges=15,vertices=9) and error<=1e-7 and max(parameter_errors)<=1e-7 and all(classification) and interferences==0 and not checker.HasWarnings()
result=dict(passed=passed,ocpVersion=OCP.__version__,valid=valid,topology=topology,volume=volume,analyticVolume=expected,absoluteVolumeError=error,absoluteTolerance=1e-7,integrationErrorEstimate=estimate,integrationMethod='OpenCascade VolumePropertiesGK, closed shells and original spans',integrationEps=1e-10,integrationRequestedEpsMet=estimate<=1e-10,integrationErrorEstimateControlsAcceptance=False,standardGaussDiagnosticVolume=diagnostic.Mass(),standardGaussDiagnosticErrorEstimate=standard_estimate,standardGaussDiagnosticAbsoluteError=abs(diagnostic.Mass()-expected),standardGaussDiagnosticControlsAcceptance=False,boundaryPointChecks=len(points),maxParameterPointError=max(parameter_errors),closestPointDiagnosticMax=max(points),closestPointDiagnosticControlsAcceptance=False,classificationChecks=len(classification),classificationsPassed=all(classification),interferencePairs=interferences,interferenceSearchCompleted=not checker.HasErrors(),stepSha256=hashlib.sha256(path.read_bytes()).hexdigest(),externalOpenCascade=True,nativeExactProof=False,scope='one non-Mobius rational two-interval quintic loft with three actual station seams; no all-mode claim')
(root/'independent-occt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));assert passed
