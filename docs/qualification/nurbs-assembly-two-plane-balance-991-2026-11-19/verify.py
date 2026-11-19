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
a = 0.018 / np.sqrt(2.)
cases = [
    ([5,4,0],[0,0,1],[0,12],[[-.324,-.432,0],[-.108,-.144,0]]),
    ([8,8,3],[1,1,0],[-2,2],[[-a,a,0],[a,-a,0]]),
]
for point, direction, planes, vectors in cases:
    point = np.array(point)
    axis = np.array(direction) / np.linalg.norm(direction)
    balanced = GProp_GProps()
    balanced.Add(aggregate)
    weights = GProp_PGProps()
    for plane, vector in zip(planes,vectors):
        magnitude = np.linalg.norm(vector)
        position = point + plane*axis + 12*np.array(vector)/magnitude
        weights.AddPoint(gp_Pnt(*position), magnitude/12)
    balanced.Add(weights)
    displacement = np.array(balanced.CentreOfMass().Coord()) - point
    tensor = balanced.MatrixOfInertia()
    action = np.array([[tensor.Value(i+1,j+1) for j in range(3)] for i in range(3)]) @ axis
    assert np.linalg.norm(np.cross(displacement,axis)) < 1e-10
    assert np.linalg.norm(np.cross(action,axis)) < 1e-10
    print("Corrected radial offset and transverse inertia:", np.linalg.norm(np.cross(displacement,axis)), np.linalg.norm(np.cross(action,axis)))
