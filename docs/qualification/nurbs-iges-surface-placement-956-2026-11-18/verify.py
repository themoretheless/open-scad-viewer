"""Read the placed IGES surface and the Rust-reconstructed STEP solid independently."""
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
patches = [p for i in range(1, model.NbEntities() + 1)
           if (p := model.Entity(i)).TypeNumber() == 128 and p.HasTransf()]
assert len(patches) == 1
patch = patches[0]
points = []
for u in range(patch.NbPolesU()):
    for v in range(patch.NbPolesV()):
        x, y, z = patch.Pole(u, v).Coord()
        world = patch.TransformedPole(u, v).Coord()
        assert all(abs(a-b)<1e-12 for a,b in zip(world,(y+4,z-7,x+2)))
        points.append(world)
assert points == [(3.,0.,0.), (3.,0.,5.), (3.,3.,0.), (3.,3.,5.), (0.,3.,0.), (0.,3.,5.)]
weights = [patch.Weight(u,v) for u in range(3) for v in range(2)]
assert all(abs(a-b)<1e-14 for a,b in zip(weights, [1.,1.,math.sqrt(.5),math.sqrt(.5),1.,1.]))
step = STEPControl_Reader()
assert step.ReadFile(str(root / "restored.step")) == IFSelect_RetDone
assert step.TransferRoots() == 1 and BRepCheck_Analyzer(step.OneShape()).IsValid()
props = GProp_GProps()
BRepGProp.VolumeProperties_s(step.OneShape(), props, 1e-9, True)
expected = 5 * math.pi * 3 ** 2
assert abs(props.Mass() - expected) < 1e-6
print(json.dumps(dict(worldPoles=points, weights=weights, volume=props.Mass(), analytic=expected)))
