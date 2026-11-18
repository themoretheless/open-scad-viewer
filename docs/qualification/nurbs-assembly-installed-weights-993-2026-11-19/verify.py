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
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
aggregate = GProp_GProps()
for label in nested:
    shape = tool.GetShape_s(label)
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props)
    density = {6:0.001, 9:0.002}[round(props.CentreOfMass().Y())]
    aggregate.Add(props, density)
import numpy as np
from OCP.GProp import GProp_PGProps
from OCP.gp import gp_Pnt
axis = np.array([0.,0.,1.])
origin = np.array([5.,4.,0.])
weights = GProp_PGProps()
for offset,position,mass in [(0,(-12,0,0),.030),(0,(0,-6,0),.070),
                             (12,(0,-12,0),.010),(12,(-6,0,0),.020)]:
    weights.AddPoint(gp_Pnt(*(origin+axis*offset+position)),mass)
aggregate.Add(weights)
offset = np.array(aggregate.CentreOfMass().Coord()) - origin
static = aggregate.Mass()*offset[:2]
tensor = aggregate.MatrixOfInertia()
couple = -np.array([tensor.Value(1,3),tensor.Value(2,3)]) + offset[2]*static
second = -couple/12
first = -static-second
assert np.allclose(first,[.036,-.012],rtol=0,atol=1e-10)
assert np.allclose(second,[.012,-.024],rtol=0,atol=1e-10)
print("Installed mass kg:", aggregate.Mass(), "remaining kg mm:",first,second)
