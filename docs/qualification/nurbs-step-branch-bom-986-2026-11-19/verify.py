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

root_parts = components(roots.Value(1))
group, = [p for p in root_parts if name(target(p)) == "Feed"]
direct_pin, = [p for p in root_parts if name(target(p)) == "Pin"]
plate, = [p for p in root_parts if name(target(p)) == "Plate"]
assert len(root_parts) == 3
nested = components(target(group))
assert len(nested) == 3 and all(name(target(p)) == "Pin" for p in nested)
linked = [p for p in nested if target(p).IsEqual(target(direct_pin))]
assert len(linked) == 2
assert all(tool.GetShape_s(p).Location().Transformation().TranslationPart().X() == 0 for p in linked)
print("Feed: shared Pin 2 inside, 1 outside; distinct Pin 1 inside, 0 outside; Plate outside only")
