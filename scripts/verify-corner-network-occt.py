#!/usr/bin/env python3
"""Independent STEP check of prepared curved sectors; no radius-law claim."""
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
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer


def count(shape, kind):
    explorer, found = TopExp_Explorer(shape, kind), []
    while explorer.More():
        entity = explorer.Current()
        if not any(entity.IsSame(old) for old in found):
            found.append(entity)
        explorer.Next()
    return len(found)


def main(directory):
    manifest = json.loads((directory/'manifest.json').read_text())
    for file, key in [('network.step', 'stepSha256'), ('model.json', 'modelSha256')]:
        if hashlib.sha256((directory/file).read_bytes()).hexdigest()!=manifest[key]:
            raise ValueError(f'{file} differs from the native receipt')
    reader = STEPControl_Reader()
    if reader.ReadFile(str(directory/'network.step'))!=IFSelect_RetDone or not reader.TransferRoots():
        raise ValueError('Independent STEP import failed')
    shape = reader.OneShape()
    valid = not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology = {name: count(shape, kind) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    bounds = [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]
    target = manifest['expected']
    bounds_error = max(abs(a-b) for row, wanted in zip(bounds, target['boundsMm']) for a, b in zip(row, wanted))
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props)
    volume_error = abs(props.Mass()-target['volumeMm3'])
    # Analytic control: integral(3+x²/4+y²/8), x,y in [-1,2] = 30.375 mm³.
    passed = (valid and topology=={k: target[k] for k in topology} and bounds_error<=1e-6
              and volume_error<=1e-6 and manifest['nativeClosureWithinTolerance'] and manifest['trimRegionsValid']
              and manifest['allSavedNewSeamsG2'] and manifest['allPatchChartsInjective'])
    report = dict(stepSha256=manifest['stepSha256'], valid=valid, topology=topology, boundsMm=bounds,
                  maximumBoundsErrorMm=bounds_error, volumeMm3=props.Mass(), volumeErrorMm3=volume_error,
                  prescribedRadiusClaim=False, generalCornerConstructionClaim=False,
                  wholeBodyG2Claim=False, passed=passed)
    (directory/'occt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if passed else 1


if __name__=='__main__':
    sys.exit(main(Path(sys.argv[1])))
