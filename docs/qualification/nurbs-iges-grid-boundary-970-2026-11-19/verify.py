"""Verify the intrinsic knot grid and the recovered UV and spatial trims."""
from pathlib import Path
import runpy
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS

previous = Path(__file__).resolve().parents[1] / "nurbs-iges-graph-boundary-969-2026-11-19/verify.py"
result = runpy.run_path(str(previous))
iges_surface = result["pair"].Surface()
assert iges_surface.NbPolesU() == iges_surface.NbPolesV() == 3
explorer = TopExp_Explorer(result["face"], TopAbs_FACE)
patch = BRepAdaptor_Surface(TopoDS.Face(explorer.Current())).BSpline()
assert [patch.UKnot(i) for i in range(1, patch.NbUKnots()+1)] == [0., 0.5, 1.]
assert [patch.VKnot(i) for i in range(1, patch.NbVKnots()+1)] == [0., 0.25, 1.]
print("retained grid: 2 x 2 cells")
