"""Check the repeated definition, volume and exact translations of two STEP occurrences."""
import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path

from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TCollection import TCollection_ExtendedString
from OCP.TDF import TDF_Label
from OCP.TDocStd import TDocStd_Document
from OCP.XCAFDoc import XCAFDoc_DocumentTool
from OCP.collections import Sequence_TDF_Label

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('fixture', type=Path)
parser.add_argument('--offset-x-mm', type=float, default=5)
parser.add_argument('--definition-volume-mm3', type=float, default=6)
args = parser.parse_args()
fixture = args.fixture
reader = STEPCAFControl_Reader()
assert reader.ReadFile(str(fixture)) == IFSelect_RetDone
document = TDocStd_Document(TCollection_ExtendedString('MDTV-XCAF'))
assert reader.Transfer(document)
tool = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
roots = Sequence_TDF_Label()
tool.GetFreeShapes(roots)
assert roots.Length() == 1
children = Sequence_TDF_Label()
assert tool.GetComponents_s(roots.Value(1), children) and children.Length() == 2

placements, definitions = [], []
for index in range(1, children.Length() + 1):
    child = children.Value(index)
    referred = TDF_Label()
    assert tool.GetReferredShape_s(child, referred)
    definitions.append(referred)
    transform = tool.GetLocation_s(child).Transformation()
    placements.append([[transform.Value(row, column) for column in range(1, 5)]
                       for row in range(1, 4)])
assert definitions[0].IsEqual(definitions[1]), 'Occurrences must reference the same XCAF definition'
placements.sort(key=lambda matrix: matrix[0][3])
for placement, offset in zip(placements, sorted([0, args.offset_x_mm])):
    expected = [[1, 0, 0, offset], [0, 1, 0, 0], [0, 0, 1, 0]]
    assert max(abs(a - b) for row, ref in zip(placement, expected) for a, b in zip(row, ref)) < 1e-9

shape = tool.GetShape_s(definitions[0])
assert BRepCheck_Analyzer(shape).IsValid()
volume = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, volume)
assert abs(volume.Mass() - args.definition_volume_mm3) < 1e-9
print(json.dumps(dict(
    passed=True, oracle='OpenCascade XCAF', version=importlib.metadata.version('cadquery-ocp'),
    stepSha256=hashlib.sha256(fixture.read_bytes()).hexdigest(), definitions=1, occurrences=2,
    definitionVolumeMm3=volume.Mass(), worldPlacementsMm=placements,
), indent=2))
