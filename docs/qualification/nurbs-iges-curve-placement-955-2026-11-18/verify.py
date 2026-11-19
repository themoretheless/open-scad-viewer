"""Read the IGES arc and the Rust-reconstructed STEP solid independently."""
import json
import math
import sys
from pathlib import Path
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader
from OCP.STEPControl import STEPControl_Reader

root = Path(sys.argv[1])
iges = IGESControl_Reader()
assert iges.ReadFile(str(root / "placed.igs")) == IFSelect_RetDone
model = iges.IGESModel()
arcs = [e for i in range(1, model.NbEntities() + 1)
        if (e := model.Entity(i)).TypeNumber() == 100]
assert len(arcs) == 1
arc = arcs[0]
assert abs(arc.Radius() - 3) < 1e-12 and abs(arc.Angle() - math.pi / 2) < 1e-12
center = arc.TransformedCenter().Coord()
start = arc.TransformedStartPoint().Coord()
end = arc.TransformedEndPoint().Coord()
assert center == (0., 0., 0.) and start == (3., 0., 0.) and end == (0., 3., 0.)
assert arc.Center().Coord() == (7., 4.)
step = STEPControl_Reader()
assert step.ReadFile(str(root / "restored.step")) == IFSelect_RetDone
assert step.TransferRoots() == 1 and BRepCheck_Analyzer(step.OneShape()).IsValid()
props = GProp_GProps()
BRepGProp.VolumeProperties_s(step.OneShape(), props, 1e-9, True)
expected = 5 * math.pi * arc.Radius() ** 2
assert abs(props.Mass() - expected) < 1e-6
print(json.dumps(dict(center=center, start=start, end=end, radius=arc.Radius(), angle=arc.Angle(), volume=props.Mass(), analytic=expected)))
