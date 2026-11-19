"""Read both boundary curves and transfer the trimmed face independently."""
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
assert pair.Curve3D().Pole(0).Coord() == (12., 1., 7.)
assert reader.TransferOne(5)
face = reader.OneShape()
assert BRepCheck_Analyzer(face).IsValid()
props = GProp_GProps()
BRepGProp.SurfaceProperties_s(face, props)
print("area", props.Mass())
assert abs(props.Mass() - 8.) < 1e-9
