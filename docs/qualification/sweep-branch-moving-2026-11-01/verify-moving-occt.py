"""One headless independent STEP case, no native geometry/mass helper."""
from pathlib import Path
import hashlib,json,sys,importlib.metadata
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
root=Path(sys.argv[1]);path=root/'moving-carrier.step'
reader=STEPControl_Reader();assert reader.ReadFile(str(path))==IFSelect_RetDone and reader.TransferRoots()>0
shape=reader.OneShape();valid=not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
def count(kind):
 ex=TopExp_Explorer(shape,kind);unique=[]
 while ex.More():
  current=ex.Current()
  if not any(current.IsSame(other)for other in unique):unique.append(current)
  ex.Next()
 return len(unique)
props=GProp_GProps();integration_error=BRepGProp.VolumeProperties_s(shape,props,Eps=1e-10)
volume=props.Mass();expected=json.loads((root/'reference.json').read_text())['volume'];error=abs(volume-expected)
topology={'solids':count(TopAbs_SOLID),'faces':count(TopAbs_FACE),'edges':count(TopAbs_EDGE),'vertices':count(TopAbs_VERTEX)}
passed=valid and topology=={'solids':1,'faces':16,'edges':32,'vertices':16} and error<=1e-7
result={'passed':passed,'valid':valid,'topology':topology,'volume':volume,'expectedVolume':expected,'absoluteVolumeError':error,'absoluteVolumeTolerance':1e-7,'integrationErrorEstimate':integration_error,'ocpVersion':importlib.metadata.version('cadquery-ocp'),'stepSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'referenceSha256':hashlib.sha256((root/'reference.json').read_bytes()).hexdigest(),'externalOpenCascade':True,'preview':False,'scope':'one new moving-matrix closed nonplanar carrier STEP case'}
(root/'independent-occt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));assert passed
