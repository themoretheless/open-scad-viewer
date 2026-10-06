import json,sys,math
from OCP.BRep import BRep_Tool
from OCP.TopoDS import TopoDS
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_FACE,TopAbs_EDGE,TopAbs_VERTEX
from OCP.TopExp import TopExp
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
r=STEPControl_Reader(); assert r.ReadFile(sys.argv[1])==IFSelect_RetDone
assert r.TransferRoots()>0
s=r.OneShape()
g=GProp_GProps(); BRepGProp.VolumeProperties_s(s,g)
b=Bnd_Box(); BRepBndLib.AddOptimal_s(s,b,False,False)
counts={}
for name,ty in [('solids',TopAbs_SOLID),('faces',TopAbs_FACE),('edges',TopAbs_EDGE),('vertices',TopAbs_VERTEX)]:
 explorer=TopExp_Explorer(s,ty); unique=[]
 while explorer.More():
  current=explorer.Current()
  if not any(current.IsSame(old) for old in unique): unique.append(current)
  explorer.Next()
 counts[name]=len(unique)
 if name=='vertices':
  coords=[]
  for vertex in unique:
   p=BRep_Tool.Pnt_s(TopoDS.Vertex(vertex));coords.append([p.X(),p.Y(),p.Z()])
result={'valid':BRepCheck_Analyzer(s).IsValid(),'counts':counts,'volume':g.Mass(),'vertices_mm':coords,'bounds':[b.CornerMin().X(),b.CornerMin().Y(),b.CornerMin().Z(),b.CornerMax().X(),b.CornerMax().Y(),b.CornerMax().Z()]}
print(json.dumps(result,indent=2));assert result['valid'];assert counts=={'solids':1,'faces':4,'edges':7,'vertices':5};assert abs(g.Mass()-1/6)<1e-7

r=math.sqrt(.5)
expected=[[0,0,0],[1,0,.25],[0,1,0],[0,0,1],[1-r,0,(1-r)**2/4]]
for point in expected:
 assert min(math.dist(point,q) for q in coords)<1e-7,(point,coords)
