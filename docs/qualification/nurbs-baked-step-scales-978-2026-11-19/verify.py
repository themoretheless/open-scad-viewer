"""Verify independently the material orientation and extent of each placed solid."""
import math
import sys
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

def measure(path):
    reader = STEPControl_Reader()
    assert reader.ReadFile(path) == IFSelect_RetDone
    assert reader.TransferRoots() == 1
    it = TopExp_Explorer(reader.OneShape(), TopAbs_SOLID)
    solids = []
    while it.More():
        solid = it.Current()
        assert BRepCheck_Analyzer(solid).IsValid()
        props = GProp_GProps()
        BRepGProp.VolumeProperties_s(solid, props, 1e-11, True)
        solids.append((*props.CentreOfMass().Coord(), props.Mass()))
        it.Next()
    return sorted(solids)

solids = measure(sys.argv[1])
expected = [(5.5, 2, 8, 576), (15.5, 2, -8, 576), (35.5, 2, 8, 576)]
assert len(solids) == len(expected)
for measured, target in zip(sorted(solids), expected):
    assert max(abs(x-y) for x,y in zip(measured, target)) < 1e-9, measured
print(sorted(solids))
ellipse, = measure(sys.argv[2])
assert max(abs(x-y) for x,y in zip(ellipse[:3], (7,11,7))) < 1e-9, ellipse
assert math.isclose(ellipse[3], 288*math.pi, rel_tol=1e-9), ellipse
print("Elliptic cylinder:", ellipse)
