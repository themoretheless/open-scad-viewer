#!/usr/bin/env python3
"""Independent STEP check of [0,10]^3 minus [2,8]^3; no thickness claim."""
import hashlib,json,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_SHELL,TopAbs_FACE
root=Path(sys.argv[1]);path=root/'cavity.step'
reader=STEPControl_Reader()
if reader.ReadFile(str(path))!=IFSelect_RetDone or not reader.TransferRoots():raise RuntimeError('STEP read failed')
shape=reader.OneShape()
if shape.IsNull() or not BRepCheck_Analyzer(shape).IsValid():raise RuntimeError('Independent cavity shape invalid')
def count(kind):
 e=TopExp_Explorer(shape,kind);entities=[]
 while e.More():
  item=e.Current();e.Next()
  if not any(item.IsSame(old) for old in entities):entities.append(item)
 return len(entities)
solids,shells,faces=count(TopAbs_SOLID),count(TopAbs_SHELL),count(TopAbs_FACE)
if (solids,shells)!=(1,2):raise RuntimeError(f'Expected one solid/two shells, got {solids}/{shells}')
box=Bnd_Box();BRepBndLib.AddOptimal_s(shape,box,False,False);bounds=list(box.Get())
if any(abs(a-b)>2e-7 for a,b in zip(bounds,[0.,0.,0.,10.,10.,10.])):raise RuntimeError(f'Unexpected bounds {bounds}')
props=GProp_GProps();integration_error=BRepGProp.VolumeProperties_s(shape,props,1e-11,True)
volume=props.Mass();expected=10.**3-6.**3
if abs(volume-expected)>1e-7:raise RuntimeError(f'Unexpected cavity volume {volume}')
report=dict(valid=True,solids=solids,shells=shells,faces=faces,bounds=bounds,volumeMm3=volume,expectedVolumeMm3=expected,integrationErrorEstimate=integration_error,stepSha256=hashlib.sha256(path.read_bytes()).hexdigest(),wholeWallThicknessClaim=False)
(root/'independent-occt.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
