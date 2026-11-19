"""Check source product names and variant sharing in Open CASCADE XDE."""
import sys
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.Quantity import Quantity_Color
from OCP.XCAFDoc import XCAFDoc_DocumentTool, XCAFDoc_ColorSurf
from OCP.collections import Sequence_TDF_Label
from OCP.IFSelect import IFSelect_RetDone
from OCP.TDF import TDF_Label
from OCP.TDataStd import TDataStd_Name

def name(label):
    attr = TDataStd_Name()
    assert label.FindAttribute(TDataStd_Name.GetID_s(), attr)
    return attr.Get().ToExtString()

reader = STEPCAFControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
doc = TDocStd_Document(TCollection_ExtendedString("MDTV-XCAF"))
assert reader.Transfer(doc)
tool = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
roots = Sequence_TDF_Label()
tool.GetFreeShapes(roots)
assert roots.Length() == 1 and name(roots.Value(1)) == "Feed"

def components(label):
    seq = Sequence_TDF_Label()
    tool.GetComponents_s(label, seq, False)
    return [seq.Value(i) for i in range(1, seq.Length()+1)]

def target(label):
    result = TDF_Label()
    assert tool.GetReferredShape_s(label, result)
    return result

colours = XCAFDoc_DocumentTool.ColorTool_s(doc.Main())
root_parts = components(roots.Value(1))
leaf, = root_parts
assert name(target(leaf)) == "Roller"
shape = tool.GetShape_s(leaf)
rgb = Quantity_Color()
assert colours.GetInstanceColor(shape, XCAFDoc_ColorSurf, rgb) or colours.GetColor_s(leaf, XCAFDoc_ColorSurf, rgb)
assert (rgb.Red(), rgb.Green(), rgb.Blue()) == (1., 0., 0.)
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
import math
props = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, props, 1e-11, True)
assert math.isclose(props.Mass(), 288*math.pi, rel_tol=1e-9)
assert max(abs(a-b) for a,b in zip(props.CentreOfMass().Coord(), (0,0,-6))) < 1e-9
print("Feed contains one red Roller with the expected signed volume and centre")
