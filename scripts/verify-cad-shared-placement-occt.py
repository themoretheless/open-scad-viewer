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
from OCP.TDataStd import TDataStd_Name
from OCP.TopLoc import TopLoc_Location
from OCP.StepShape import StepShape_ShapeDefinitionRepresentation
from OCP.StepRepr import StepRepr_NextAssemblyUsageOccurrence
from OCP.StepVisual import StepVisual_StyledItem, StepVisual_PresentationStyleByContext
from OCP.TDocStd import TDocStd_Document
from OCP.XCAFDoc import XCAFDoc_DocumentTool
from OCP.collections import Sequence_TDF_Label

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('fixture', type=Path)
parser.add_argument('--offset-x-mm', type=float, default=5)
parser.add_argument('--definition-volume-mm3', type=float, default=6)
parser.add_argument('--grouped-colours', action='store_true')
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
def label_name(label):
    attribute = TDataStd_Name()
    assert label.FindAttribute(TDataStd_Name.GetID_s(), attribute)
    return attribute.Get().ToExtString()


occurrences = []
def visit(label, world, groups):
    children = Sequence_TDF_Label()
    assert tool.GetComponents_s(label, children)
    for index in range(1, children.Length() + 1):
        child = children.Value(index)
        referred = TDF_Label()
        assert tool.GetReferredShape_s(child, referred)
        location = world.Multiplied(tool.GetLocation_s(child))
        if tool.IsAssembly_s(referred):
            visit(referred, location, [*groups, label_name(child)])
        else:
            transform = location.Transformation()
            matrix = [[transform.Value(row, column) for column in range(1, 5)]
                      for row in range(1, 4)]
            occurrences.append((matrix, referred, groups))


visit(roots.Value(1), TopLoc_Location(), [])
assert len(occurrences) == 2
occurrences.sort(key=lambda row: row[0][0][3])
placements, definitions, parents = zip(*occurrences)
assert definitions[0].IsEqual(definitions[1]), 'Occurrences must reference the same XCAF definition'
for placement, offset in zip(placements, sorted([0, args.offset_x_mm])):
    expected = [[1, 0, 0, offset], [0, 1, 0, 0], [0, 0, 1, 0]]
    assert max(abs(a - b) for row, ref in zip(placement, expected) for a, b in zip(row, ref)) < 1e-9

shape = tool.GetShape_s(definitions[0])
assert BRepCheck_Analyzer(shape).IsValid()
volume = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, volume)
assert abs(volume.Mass() - args.definition_volume_mm3) < 1e-9
colours = {}
if args.grouped_colours:
    assert parents == ([], ['Группа 1']), parents
    model = reader.Reader().StepModel()
    contexts = {}
    styles = []
    for index in range(1, model.NbEntities() + 1):
        entity = model.Value(index)
        if isinstance(entity, StepShape_ShapeDefinitionRepresentation):
            definition = entity.Definition().PropertyDefinition()
            if definition is not None:
                relationship = definition.Definition().ProductDefinitionRelationship()
                if isinstance(relationship, StepRepr_NextAssemblyUsageOccurrence):
                    contexts[entity.UsedRepresentation()] = relationship
        if isinstance(entity, StepVisual_StyledItem):
            styles.append(entity)
    targets = set()
    shared_definitions = set()
    occurrence_ids = set()
    for item in styles:
        assert item.NbStyles() == 1
        style = item.StylesValue(1)
        assert isinstance(style, StepVisual_PresentationStyleByContext)
        representation = style.StyleContext().Representation()
        relationship = contexts[representation]
        offset = representation.ItemsValue(1).Location().CoordinatesValue(1)
        assert offset not in colours
        shared_definitions.add(relationship.RelatedProductDefinition())
        occurrence_ids.add(relationship)
        fill = style.StylesValue(1).SurfaceStyleUsage().Style().StylesValue(1).SurfaceStyleFillArea().FillArea()
        rgb = fill.FillStylesValue(1).FillAreaStyleColour().FillColour()
        colours[offset] = [rgb.Red(), rgb.Green(), rgb.Blue()]
        targets.add(item.Item())
    assert len(occurrence_ids) == 2 and None not in occurrence_ids
    assert len(shared_definitions) == 1 and None not in shared_definitions
    assert len(targets) == 1 and None not in targets
    assert colours == {0: [1, 0, 0], args.offset_x_mm: [0, 0, 1]}, colours
else:
    assert parents == ([], []), parents

print(json.dumps(dict(
    passed=True, oracle='OpenCascade XCAF', version=importlib.metadata.version('cadquery-ocp'),
    stepSha256=hashlib.sha256(fixture.read_bytes()).hexdigest(), definitions=1, occurrences=2,
    definitionVolumeMm3=volume.Mass(), worldPlacementsMm=placements,
    **(dict(parentGroups=parents, occurrenceColours=colours) if args.grouped_colours else {}),
), indent=2))
