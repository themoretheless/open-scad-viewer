import sys, math
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID,TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps

def shapes(shape,kind):
    it=TopExp_Explorer(shape,kind)
    while it.More():
        yield it.Current()
        it.Next()

def measure(path):
    r=STEPControl_Reader()
    assert r.ReadFile(path)==IFSelect_RetDone
    assert r.TransferRoots()==1
    parts=[]
    for solid in shapes(r.OneShape(),TopAbs_SOLID):
        pts=set()
        for v in shapes(solid,TopAbs_VERTEX):
            p=BRep_Tool.Pnt_s(TopoDS.Vertex(v))
            pts.add(tuple(round(a,7) for a in p.Coord()))
        mass=GProp_GProps()
        BRepGProp.VolumeProperties_s(solid,mass,1e-8,True)
        parts.append((pts,mass.Mass()))
    assert len(parts)==20
    return parts

a,b=map(measure,sys.argv[1:])
changes=[]
for (p,v),(q,w) in zip(a,b):
    if p==q:
        assert abs(v-w)<1e-5
        continue
    assert p-q and q-p
    for points,radius in ((p-q,21),(q-p,22)):
        assert all(abs(math.hypot(x,y)-radius)<1e-6 for x,y,z in points)
    changes.append(v-w)
assert len(changes)==1
print('Volume removed:',changes,'analytic:',430*math.pi,flush=True)
assert abs(changes[0]-430*math.pi)<0.001
