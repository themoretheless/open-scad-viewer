"""Read a UV-only boundary and transfer the trimmed face independently."""
import sys
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader

reader = IGESControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
model = reader.IGESModel()
pair = model.Entity(4)
assert pair.TypeNumber() == 142 and pair.Surface().TypeNumber() == 128
assert pair.CurveUV().Pole(0).Coord() == (0.25, 0.25, 0.)
assert pair.Curve3D() is None
assert reader.TransferOne(5)
face = reader.OneShape()
assert BRepCheck_Analyzer(face).IsValid()
props = GProp_GProps()
BRepGProp.SurfaceProperties_s(face, props)
print("area", props.Mass())
assert abs(props.Mass() - 8.) < 1e-9

from pathlib import Path
from OCP.STEPControl import STEPControl_Reader

restored = STEPControl_Reader()
assert restored.ReadFile(str(Path(sys.argv[1]).with_suffix(".step"))) == IFSelect_RetDone
assert restored.TransferRoots() == 1
shape = restored.OneShape()
assert BRepCheck_Analyzer(shape).IsValid()
properties = GProp_GProps()
BRepGProp.SurfaceProperties_s(shape, properties)
assert abs(properties.Mass() - 8.) < 1e-9
assert properties.CentreOfMass().Distance(props.CentreOfMass()) < 1e-9
print("restored area", properties.Mass(), "centroid", properties.CentreOfMass().Coord())
