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
direct, = [label for label in root_parts if name(target(label)) == "Bracket"]
nested = components(target(group))
assert len(nested) == 2
from OCP.Bnd import Bnd_Box
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_VERTEX
from OCP.BRep import BRep_Tool
from OCP.TopoDS import TopoDS
for labels, expected in [(nested,(6,5,0,10,10,6)),([direct,*nested],(6,0,0,22,10,6))]:
    box = Bnd_Box()
    for label in labels:
        vertices = TopExp_Explorer(tool.GetShape_s(label), TopAbs_VERTEX)
        while vertices.More():
            box.Add(BRep_Tool.Pnt_s(TopoDS.Vertex(vertices.Current())))
            vertices.Next()
    measured = (*box.CornerMin().Coord(), *box.CornerMax().Coord())
    assert max(abs(a-b) for a,b in zip(expected,measured)) < 1e-8, measured
    print(measured)
