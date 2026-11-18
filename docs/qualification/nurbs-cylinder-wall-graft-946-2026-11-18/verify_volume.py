"""Compare an exchanged cylindrical wall replacement with an independent OCCT cut."""
import json
import math
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepGProp import BRepGProp
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut, BRepAlgoAPI_Common
from OCP.GProp import GProp_GProps

def read(path):
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(path)) == IFSelect_RetDone
    assert reader.TransferRoots() == 1
    return reader.OneShape()

def volume(shape):
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-9, True)
    return props.Mass()

directory = Path(sys.argv[1])
before = read(directory / "before.step")
after = read(directory / "after.step")
valid = BRepCheck_Analyzer(after).IsValid()
cutter_height = float(sys.argv[2]) if len(sys.argv) > 2 else 10.0
cut = BRepAlgoAPI_Cut(before, BRepPrimAPI_MakeCylinder(22., cutter_height).Shape())
assert cut.IsDone()
original, sewn, reference = map(volume, (before, after, cut.Shape()))
removed = original - sewn
assert original > sewn > 0
print(json.dumps(dict(valid=valid, original=original, sewn=sewn, reference=reference, removed=removed,
                      difference=sewn-reference, nominal=430*math.pi)), flush=True)
assert valid and abs(sewn-reference) < 0.01

if (directory / "sleeve.step").exists():
    sleeve = read(directory / "sleeve.step")
    common = BRepAlgoAPI_Common(before, BRepPrimAPI_MakeCylinder(22., cutter_height).Shape())
    assert common.IsDone() and BRepCheck_Analyzer(sleeve).IsValid()
    actual, expected = volume(sleeve), volume(common.Shape())
    assert actual > 0 and abs(actual - expected) < 0.01
    assert abs(actual + sewn - original) < 0.01
    print(json.dumps(dict(sleeve=actual, common=expected, balance=actual + sewn - original)))
