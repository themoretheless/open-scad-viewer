#!/usr/bin/env python3
"""Qualify the exported assembly hierarchy and each current solid with OCCT XCAF."""
import hashlib
import json
from pathlib import Path
import sys
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.XCAFDoc import XCAFDoc_DocumentTool, XCAFDoc_ColorSurf, XCAFDoc_ColorGen
from OCP.Quantity import Quantity_Color, Quantity_TOC_sRGB
from OCP.collections import Sequence_TDF_Label
from OCP.TDF import TDF_Label
from OCP.TDataStd import TDataStd_Name
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp

directory = Path(sys.argv[1] if len(sys.argv) > 1 else 'output/qualification/cad-roadmap-assembly')
manifest = json.loads((directory / 'manifest.json').read_text())
reader = STEPCAFControl_Reader()
reader.SetNameMode(True)
reader.SetColorMode(True)
assert reader.ReadFile(str(directory / 'scene.step')) == IFSelect_RetDone
document = TDocStd_Document(TCollection_ExtendedString('MDTV-XCAF'))
assert reader.Transfer(document)
tool = XCAFDoc_DocumentTool.ShapeTool_s(document.Main())
colors = XCAFDoc_DocumentTool.ColorTool_s(document.Main())
roots = Sequence_TDF_Label()
tool.GetFreeShapes(roots)
assert roots.Length() == 1, 'Expected one assembly root'
leaves = []

def visit(label):
    attribute = TDataStd_Name()
    name = attribute.Get().ToExtString() if label.FindAttribute(TDataStd_Name.GetID_s(), attribute) else ''
    referred = TDF_Label()
    if tool.GetReferredShape_s(label, referred):
        label = referred
    children = Sequence_TDF_Label()
    tool.GetComponents_s(label, children)
    node = {'name': name, 'children': [visit(children.Value(i)) for i in range(1, children.Length()+1)]}
    if not node['children']:
        shape = tool.GetShape_s(label)
        assert BRepCheck_Analyzer(shape).IsValid(), name
        box, properties = Bnd_Box(), GProp_GProps()
        BRepBndLib.AddOptimal_s(shape, box, False, False)
        BRepGProp.VolumeProperties_s(shape, properties)
        node['boundsMm'] = [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]
        node['volumeMm3'] = properties.Mass()
        color = Quantity_Color()
        assert colors.GetColor(shape, XCAFDoc_ColorSurf, color) or colors.GetColor(shape, XCAFDoc_ColorGen, color), 'Missing color: '+name
        node['colorSrgb'] = list(color.Values(Quantity_TOC_sRGB))
        leaves.append(node)
    return node

tree = visit(roots.Value(1))
assert tree['name'] == 'Solid scene'
assert [g['name'] for g in tree['children']] == manifest['groups']
assert [len(g['children']) for g in tree['children']] == [2, 2]
assert len(leaves) == len(manifest['parts']) == 4
for actual, expected in zip(leaves, manifest['parts']):
    assert abs(actual['volumeMm3']-expected['volume']) <= max(1e-6, expected['volume']*1e-8), actual['name']
    assert max(abs(a-b) for row, ref in zip(actual['boundsMm'], expected['bounds']) for a,b in zip(row,ref)) <= 1e-6, actual['name']
    rgb = [int(expected['color'][i:i+2], 16)/255 for i in [1,3,5]]
    assert max(abs(a-b) for a,b in zip(actual['colorSrgb'],rgb)) < 1e-6, actual['name']
assert leaves[0]['name'] == "Кронштейн 'A' #1"
report = {'passed': True, 'oracle': 'OpenCascade XCAF / cadquery-ocp 8.0.1.0.0',
          'stepSha256': hashlib.sha256((directory/'scene.step').read_bytes()).hexdigest(), 'tree': tree}
(directory/'occt-report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2)+'\n')
print(json.dumps(report, ensure_ascii=False, indent=2))
