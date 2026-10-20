#!/usr/bin/env python3
"""Independent STEP check of the documented 8 mm cube seedless plane-cut control."""
import json, sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool

root = Path(sys.argv[1])
cap = '--cap' in sys.argv[2:]
reader = STEPControl_Reader()
assert reader.ReadFile(str(root / ('body.step' if cap else 'partition.step'))) == IFSelect_RetDone
assert reader.TransferRoots()
shape = reader.OneShape()
assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()

def entities(value, kind):
    result = []
    iterator = TopExp_Explorer(value, kind)
    while iterator.More():
        item = iterator.Current()
        if not any(item.IsSame(old) for old in result):
            result.append(item)
        iterator.Next()
    return result

faces = entities(shape, TopAbs_FACE)
edges = entities(shape, TopAbs_EDGE)
topology = dict(solids=len(entities(shape, TopAbs_SOLID)), faces=len(faces),
                edges=len(edges), vertices=len(entities(shape, TopAbs_VERTEX)))
expected_topology = dict(solids=1, faces=6, edges=12, vertices=8) if cap else dict(solids=1, faces=7, edges=15, vertices=10)
assert topology == expected_topology, topology
assert all(sum(any(edge.IsSame(other) for other in entities(face, TopAbs_EDGE))
               for face in faces) == 2 for edge in edges)
props = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, props, 1e-10, True, False)
expected_volume = 256. if cap else 512.
assert abs(props.Mass() - expected_volume) < 1e-9
sectors = []
for face in faces:
    points = [BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)) for v in entities(face, TopAbs_VERTEX)]
    if not points or not all(abs(p.Z() - 8.) < 1e-9 for p in points):
        continue
    area = GProp_GProps()
    BRepGProp.SurfaceProperties_s(face, area)
    lo, hi = min(p.Y() for p in points), max(p.Y() for p in points)
    assert abs(area.Mass() - 32.) < 1e-9
    sectors.append([lo, hi])
assert sorted(sectors) == ([[4., 8.]] if cap else [[0., 4.], [4., 8.]]), sectors
if cap:
    points = [BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)) for v in entities(shape, TopAbs_VERTEX)]
    bounds = [[min(p.Coord(k) for p in points), max(p.Coord(k) for p in points)] for k in (1, 2, 3)]
    assert bounds == [[0., 8.], [4., 8.], [0., 8.]], bounds
    caps = []
    for face in faces:
        pts = [BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)) for v in entities(face, TopAbs_VERTEX)]
        if pts and all(abs(p.Y() - 4.) < 1e-9 for p in pts):
            area = GProp_GProps()
            BRepGProp.SurfaceProperties_s(face, area)
            caps.append(area.Mass())
    assert len(caps) == 1 and abs(caps[0] - 64.) < 1e-9, caps
report = dict(passed=True, topology=topology, volumeMm3=props.Mass(),
              expectedVolumeMm3=expected_volume, topSectorYBounds=sectors,
              scope=('Independent half-cube cap control; no arbitrary fillet or global embedding claim' if cap else 'Independent plane-cut control partition only; material has not been removed; no arbitrary fillet or closed replacement claim'))
(root / 'independent-step.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
