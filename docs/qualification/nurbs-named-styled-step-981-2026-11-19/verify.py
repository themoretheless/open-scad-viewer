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
assert roots.Length() == 1 and name(roots.Value(1)) == "Drive"

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
assert len(root_parts) == 2
group, = [label for label in root_parts if name(target(label)) == "Feed"]
direct, = [label for label in root_parts if name(target(label)) == "Roller"]
nested, = components(target(group))
assert target(nested).IsEqual(target(direct))
for label, expected_x, expected_rgb in [(nested,0,(1,0,0)),(direct,20,(0,0,1))]:
    shape = tool.GetShape_s(label)
    rgb = Quantity_Color()
    assert colours.GetInstanceColor(shape, XCAFDoc_ColorSurf, rgb) or colours.GetColor_s(label, XCAFDoc_ColorSurf, rgb)
    assert abs(shape.Location().Transformation().TranslationPart().X()-expected_x) < 1e-9
    assert (rgb.Red(),rgb.Green(),rgb.Blue()) == expected_rgb
print("Drive/Feed/Roller red at x=0 and Drive/Roller blue at x=20 share one product")
