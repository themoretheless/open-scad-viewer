"""Read both exports independently and check the three placed solids."""
from pathlib import Path
import sys
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

for name in ("original.step", "compressed.step"):
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(Path(sys.argv[1]) / name)) == IFSelect_RetDone
    assert reader.TransferRoots() == 1
    solids = TopExp_Explorer(reader.OneShape(), TopAbs_SOLID)
    measured = []
    while solids.More():
        solid = solids.Current()
        assert BRepCheck_Analyzer(solid).IsValid()
        props = GProp_GProps()
        BRepGProp.VolumeProperties_s(solid, props)
        measured.append((*props.CentreOfMass().Coord(), props.Mass()))
        solids.Next()
    expected = [(4, 2, 1, 64), (20.5, .5, .5, 1), (38, 4, 1, 64)]
    assert len(measured) == len(expected)
    for observed, target in zip(sorted(measured), expected):
        assert max(abs(a-b) for a,b in zip(observed,target)) < 1e-9
    print(name, sorted(measured))
