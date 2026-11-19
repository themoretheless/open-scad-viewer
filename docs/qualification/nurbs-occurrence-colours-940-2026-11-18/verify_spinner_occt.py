import json,sys,math
from pathlib import Path
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.XCAFDoc import XCAFDoc_DocumentTool, XCAFDoc_ColorSurf
from OCP.Quantity import Quantity_Color
from OCP.collections import Sequence_TDF_Label
from OCP.IFSelect import IFSelect_RetDone
from OCP.TDF import TDF_Label
from OCP.TDataStd import TDataStd_Name
from OCP.TopAbs import TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
step, manifest = map(Path,sys.argv[1:])
rows = json.loads(manifest.read_text())
reader = STEPCAFControl_Reader()
assert reader.ReadFile(str(step)) == IFSelect_RetDone
doc = TDocStd_Document(TCollection_ExtendedString('MDTV-XCAF'))
assert reader.Transfer(doc)
tool = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
colours = XCAFDoc_DocumentTool.ColorTool_s(doc.Main())
roots = Sequence_TDF_Label()
tool.GetFreeShapes(roots)
assert roots.Length() == 1
def components(label):
    seq = Sequence_TDF_Label()
    tool.GetComponents_s(label,seq,False)
    return [seq.Value(i) for i in range(1,seq.Length()+1)]
def definition(label):
    target = TDF_Label()
    assert tool.GetReferredShape_s(label,target)
    return target
def name(label):
    attr=TDataStd_Name()
    assert label.FindAttribute(TDataStd_Name.GetID_s(),attr)
    return attr.Get().ToExtString()
groups = components(roots.Value(1))
assert len(groups)==2
seen=set()
for group in groups:
    assert name(group) in {"Central gears", "Planets"}
    planets = name(group)=="Planets"
    leaves=components(definition(group))
    assert len(leaves)==(18 if planets else 2)
    if planets:
        assert all(definition(leaf).IsEqual(definition(leaves[0])) for leaf in leaves)
    for leaf in leaves:
        label=name(leaf)
        row=next(r for r in rows if r['name']==label and r['id'] not in seen)
        seen.add(row['id'])
        shape=tool.GetShape_s(leaf)
        colour=Quantity_Color()
        assert colours.GetInstanceColor(shape,XCAFDoc_ColorSurf,colour) or colours.GetColor_s(leaf,XCAFDoc_ColorSurf,colour)
        expected=(1.,0.,0.) if row['colour']=='red' else (0.,0.,1.)
        assert (colour.Red(),colour.Green(),colour.Blue())==expected
        vertices=TopExp_Explorer(shape,TopAbs_VERTEX)
        found = False
        minimum_radius = math.inf
        while vertices.More():
            point=BRep_Tool.Pnt_s(TopoDS.Vertex(vertices.Current()))
            found |= math.dist(row['xyz'],point.Coord()) < 1e-7
            minimum_radius = min(minimum_radius, math.hypot(point.X(), point.Y()))
            vertices.Next()
        assert found, (label,row['xyz'])
        if label == rows[0]["name"]:
            assert abs(2 * minimum_radius - 42.) < 1e-7, minimum_radius
assert len(seen)==20
print(json.dumps(dict(bytes=step.stat().st_size,groups=2,placements=20,sharedPlanets=18,coloursVerified=20,placedVerticesVerified=20,sunBoreMm=42)))
