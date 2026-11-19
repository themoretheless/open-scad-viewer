"""Verify the solid reconstructed from a natural IGES face boundary."""
import json
import sys

from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer


def count(shape, kind):
    explorer = TopExp_Explorer(shape, kind)
    total = 0
    while explorer.More():
        total += 1
        explorer.Next()
    return total


reader = STEPControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
assert reader.NbRootsForTransfer() == reader.TransferRoots() == 1
shape = reader.OneShape()
assert BRepCheck_Analyzer(shape).IsValid()
counts = [count(shape, kind) for kind in (TopAbs_SOLID, TopAbs_SHELL, TopAbs_FACE)]
assert counts == [1, 1, 6], counts
props = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, props, 1e-9, True)
assert abs(props.Mass() - 24) <= 1e-9, props.Mass()
print(json.dumps(dict(solids=counts[0], shells=counts[1], faces=counts[2], volume=props.Mass())))
