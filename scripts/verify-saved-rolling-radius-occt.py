#!/usr/bin/env python3
"""Independent STEP validation of constant/linear Y-axis rolling controls."""
import hashlib,json,math,sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepTools import BRepTools
from OCP.gp import gp_Pnt,gp_Vec
root=Path(sys.argv[1]);path=root/'rolling.step'
recipe=json.loads((root/'manifest.json').read_text())
field=recipe.get('radiusField',[0.,0.,0.,1.5])
if field[0]!=0. or field[2]!=0.:raise RuntimeError('Verifier requires the fixed Y-axis control')
radius_at=lambda y:field[1]*y+field[3]
r0,r1=radius_at(2.),radius_at(8.)
reader=STEPControl_Reader()
if reader.ReadFile(str(path))!=IFSelect_RetDone or not reader.TransferRoots():raise RuntimeError('STEP read failed')
shape=reader.OneShape()
if shape.IsNull() or not BRepCheck_Analyzer(shape).IsValid():raise RuntimeError('STEP shape invalid')
explorer=TopExp_Explorer(shape,TopAbs_SOLID);solids=0
while explorer.More():solids+=1;explorer.Next()
if solids!=1:raise RuntimeError('Expected one solid')
properties=GProp_GProps();integration_error=BRepGProp.VolumeProperties_s(shape,properties,1e-11,True)
volume=properties.Mass();expected=600.-2.*(r0*r0+r0*r1+r1*r1)*(1.-math.pi/4.)
if abs(volume-expected)>1e-7:raise RuntimeError(f'Volume {volume} differs from independent {expected}')
explorer=TopExp_Explorer(shape,TopAbs_FACE);checked=0;error=0.;normal_error=0.
while explorer.More():
 face=TopoDS.Face_s(explorer.Current());explorer.Next()
 box=Bnd_Box();BRepBndLib.AddOptimal_s(face,box,False,False)
 x0,y0,z0,x1,y1,z1=box.Get()
 if x1>1.500001 or z1>1.500001 or y1-y0<5.9:continue
 surface=BRep_Tool.Surface_s(face);u0,u1,v0,v1=BRepTools.UVBounds_s(face)
 for i in range(13):
  for j in range(13):
   p=surface.Value(u0+(u1-u0)*i/12.,v0+(v1-v0)*j/12.)
   r=radius_at(p.Y())
   error=max(error,abs(math.hypot(p.X()-r,p.Z()-r)-r))
 for u in [u0,u1]:
  for j in range(13):
   point=gp_Pnt();du=gp_Vec();dv=gp_Vec()
   surface.D1(u,v0+(v1-v0)*j/12.,point,du,dv)
   normal=du.Crossed(dv)
   if normal.Magnitude()<=0.:raise RuntimeError('Independent skin contact is singular')
   normal.Normalize();n=[normal.X(),normal.Y(),normal.Z()]
   if abs(point.Z())<1e-7:axis=2
   elif abs(point.X())<1e-7:axis=0
   else:raise RuntimeError('Contact is outside either original support plane')
   normal_error=max(normal_error,math.sqrt(sum(n[k]*n[k] for k in range(3) if k!=axis)+(abs(n[axis])-1.)**2))
 checked+=1
if checked!=1 or error>1e-7:raise RuntimeError(f'Independent radius samples failed: faces={checked}, error={error}')
if normal_error>1e-5:raise RuntimeError(f'Independent contact normal error: {normal_error}')
report=dict(sampledContactNormalChord=normal_error,valid=True,solids=solids,volumeMm3=volume,integrationErrorEstimate=integration_error,expectedVolumeMm3=expected,sampledSkinFaces=checked,sampledRadialErrorMm=error,stepSha256=hashlib.sha256(path.read_bytes()).hexdigest(),g2Claim=False,wholeRadiusProofSource='native original coefficients; OCCT samples are independent checks')
(root/'independent-occt.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,indent=2))
