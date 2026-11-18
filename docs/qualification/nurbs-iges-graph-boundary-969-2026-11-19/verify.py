"""Compare the restored saddle patch with an independent integral over 0 <= v <= u <= 1."""
import sys
from pathlib import Path
import numpy as np
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve, BRepAdaptor_Curve2d
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader
from OCP.STEPControl import STEPControl_Reader

path = Path(sys.argv[1])
source = IGESControl_Reader()
assert source.ReadFile(str(path)) == IFSelect_RetDone
pair = source.IGESModel().Entity(3)
assert pair.TypeNumber() == 142 and pair.CurveUV() is None
assert pair.Curve3D().Pole(5).Coord() == (2.,2.,0.)

reader = STEPControl_Reader()
assert reader.ReadFile(str(path.with_suffix(".step"))) == IFSelect_RetDone
assert reader.TransferRoots() == 1
face = reader.OneShape()
assert BRepCheck_Analyzer(face).IsValid()
edges = TopExp_Explorer(face, TopAbs_EDGE)
assert edges.More()
edge = TopoDS.Edge(edges.Current())
curve = BRepAdaptor_Curve(edge)
faces = TopExp_Explorer(face, TopAbs_FACE)
uv_curve = BRepAdaptor_Curve2d(edge, TopoDS.Face(faces.Current()))
assert (curve.FirstParameter(), curve.LastParameter()) == (0., 3.)
for t in np.linspace(0., 3., 193):
    if t <= 1.:
        expected = (4*t, 0., 0.)
    elif t <= 2.:
        expected = (4., 4*(t-1), 4*(t-1))
    else:
        x = 3-t
        expected = (4*x, 4*x, 4*x*x)
    assert np.max(abs(np.array(curve.Value(t).Coord()) - expected)) < 1e-12
    assert np.max(abs(np.array(uv_curve.Value(t).Coord()) - np.array(expected[:2])/4)) < 1e-12
edges.Next()
assert not edges.More()

props = GProp_GProps()
BRepGProp.SurfaceProperties_s(face, props, 1e-11)

nodes, weights = np.polynomial.legendre.leggauss(32)
u = (nodes[:,None] + 1) / 2
v = u * (nodes[None,:] + 1) / 2
d_area = weights[:,None]*weights[None,:]*u*4*np.sqrt(1+u*u+v*v)
area = d_area.sum()
centroid = np.array([(coord*d_area).sum()/area for coord in (4*u,4*v,4*u*v)])
print("area", props.Mass(), area, "centroid", props.CentreOfMass().Coord(), centroid)
assert abs(props.Mass()-area) < 1e-7
assert np.max(abs(np.array(props.CentreOfMass().Coord())-centroid)) < 1e-7
