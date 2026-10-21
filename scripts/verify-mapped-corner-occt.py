#!/usr/bin/env python3
"""Independent STEP check of the fixed mapped-corner paraboloid control."""
import hashlib
import json
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

root = Path(sys.argv[1])
path = root / 'mapped-corner.step'
reader = STEPControl_Reader()
if reader.ReadFile(str(path)) != IFSelect_RetDone or not reader.TransferRoots():
    raise RuntimeError('Independent STEP read failed')
shape = reader.OneShape()
if shape.IsNull() or not BRepCheck_Analyzer(shape).IsValid():
    raise RuntimeError('Independent STEP shape is invalid')
solids = TopExp_Explorer(shape, TopAbs_SOLID)
count = 0
while solids.More():
    count += 1
    solids.Next()
if count != 1:
    raise RuntimeError(f'Expected one solid, got {count}')
box = Bnd_Box()
BRepBndLib.AddOptimal_s(shape, box, False, False)
bounds = list(box.Get())
expected = [-1., -1., -3., 2., 2., 1.5]
if any(abs(a-b) > 2e-7 for a,b in zip(bounds, expected)):
    raise RuntimeError(f'Unexpected independent bounds: {bounds}')
properties = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, properties)
volume = properties.Mass()
# Integral on [-1,2]^2 of 3 + x²/4 + y²/8, independent of native data.
expected_volume = 243./8.
if abs(volume-expected_volume) > 1e-7:
    raise RuntimeError(f'Unexpected independent volume: {volume}')
report = dict(valid=True, solids=count, bounds=bounds, volume=volume,
              expectedVolume=expected_volume, stepSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
              radiusClaim=False, scope='fixed mapped-corner replacement control')
(root/'independent-occt.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps(report, indent=2))
