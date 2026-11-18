"""Check source product names and variant sharing in Open CASCADE XDE."""
import sys
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.XCAFDoc import XCAFDoc_DocumentTool
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
parts = Sequence_TDF_Label()
tool.GetComponents_s(roots.Value(1), parts, False)
assert parts.Length() == 4
targets = {}
for i in range(1, 5):
    part = parts.Value(i)
    target = TDF_Label()
    assert tool.GetReferredShape_s(part, target)
    x = tool.GetShape_s(part).Location().Transformation().TranslationPart().X()
    assert name(target) == ("Pin" if x == 20 else "Roller")
    targets[x] = target
assert set(targets) == {0, 20, 40, 60}
assert targets[0].IsEqual(targets[60]) and not targets[0].IsEqual(targets[40])
print("Drive:", [(x, name(targets[x])) for x in sorted(targets)])
