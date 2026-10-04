"""Independent OpenCascade check of the generated guided-loft STEP fixture.

Generate with LOFT_STEP_OUTPUT=/absolute/path cargo test --test guided_loft_step.
Requires cadquery-ocp in a separate Python environment.
"""
import json
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE, TopAbs_EDGE, TopAbs_SOLID
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.gp import gp_Pnt, gp_Vec

reader = STEPControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
assert reader.TransferRoots() == 1
shape = reader.OneShape()
assert not shape.IsNull()
assert BRepCheck_Analyzer(shape).IsValid()

def shapes(kind):
    ex = TopExp_Explorer(shape, kind)
    result = []
    while ex.More():
        result.append(ex.Current())
        ex.Next()
    return result

faces = shapes(TopAbs_FACE)
assert len(faces) == 1
assert len(shapes(TopAbs_EDGE)) == 4
assert len(shapes(TopAbs_SOLID)) == 0
surface = BRepAdaptor_Surface(TopoDS.Face(faces[0]))
position_error = derivative_error = 0.0
for u in [0, .13, .5, .87, 1]:
    for v in [0, .17, .5, .83, 1]:
        p, du, dv = gp_Pnt(), gp_Vec(), gp_Vec()
        surface.D1(u, v, p, du, dv)
        expected = (u, 2*v*(1-v), 2*v)
        position_error = max(position_error, *(abs(a-b) for a,b in zip(p.Coord(), expected)))
        derivative_error = max(derivative_error, *(abs(a-b) for a,b in zip(dv.Coord(), (0,2-4*v,2))))
assert position_error < 1e-9
assert derivative_error < 1e-9
report = dict(parser="OpenCascade/cadquery-ocp", faces=1, edges=4, solids=0,
              valid=True, sampled_positions=25, max_position_error=position_error,
              max_v_derivative_error=derivative_error,
              scope="one nonrational guided open shell; sampled geometry and jets, not continuous certification")
print(json.dumps(report, indent=2))
if len(sys.argv) > 2:
    Path(sys.argv[2]).write_text(json.dumps(report, indent=2)+"\n")
