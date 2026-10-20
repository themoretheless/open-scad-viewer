#!/usr/bin/env python3
"""Independent STEP clearance for the fixed corner/floor qualification control."""
import hashlib
import json
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepExtrema import BRepExtrema_DistShapeShape


def main(directory):
    manifest = json.loads((directory / 'manifest.json').read_text())['parts'][0]
    native = json.loads((directory / 'wall-clearance.json').read_text())
    step = directory / manifest['file']
    if hashlib.sha256(step.read_bytes()).hexdigest() != manifest['sha256']:
        raise ValueError('STEP hash differs from manifest')
    if hashlib.sha256((directory / 'model.json').read_bytes()).hexdigest() != native['modelSha256']:
        raise ValueError('Clearance result is stale for this model')
    reader = STEPControl_Reader()
    if reader.ReadFile(str(step)) != IFSelect_RetDone or reader.TransferRoots() == 0:
        raise ValueError('STEP import failed')
    explorer = TopExp_Explorer(reader.OneShape(), TopAbs_FACE)
    corners, floor = [], []
    while explorer.More():
        face = TopoDS.Face(explorer.Current())
        box = Bnd_Box()
        BRepBndLib.AddOptimal_s(face, box, False, False)
        lo, hi = box.CornerMin(), box.CornerMax()
        bounds = [[lo.X(), lo.Y(), lo.Z()], [hi.X(), hi.Y(), hi.Z()]]
        if all(abs(bounds[j][2] + 3.) < 1e-5 for j in range(2)):
            floor.append(face)
        expected = [[0., 0., 0.], [1., 1., 0.375]]
        if max(abs(bounds[j][k] - expected[j][k]) for j in range(2) for k in range(3)) < 1e-5:
            corners.append(face)
        explorer.Next()
    if len(corners) != 1 or len(floor) != 9:
        raise ValueError('Independent geometric face identification failed')
    distances = []
    for face in floor:
        query = BRepExtrema_DistShapeShape(corners[0], face)
        query.Perform()
        if not query.IsDone() or query.NbSolution() == 0:
            raise ValueError('Independent face distance failed')
        distances.append(query.Value())
    distance = min(distances)
    tolerance = 1e-7  # Independent OCCT comparison tolerance, not a native bound.
    automatic_selection = (native.get('candidateDiscovery') == 'original-surface-normal-opposition'
                           and native.get('candidateSelectionComplete') is True
                           and len(set(native['targetFaces'])) == len(floor)
                           and len(native['targetFaces']) == len(floor))
    passed = (automatic_selection and native['coverageComplete'] and native['converged']
              and native['lowerMm'] <= 3. <= native['upperMm']
              and abs(distance - 3.) <= tolerance
              and native['lowerMm'] - tolerance <= distance <= native['upperMm'] + tolerance)
    report = dict(schema='corner-wall-occt/1', stepSha256=manifest['sha256'],
                  modelSha256=native['modelSha256'], sourceFaces=1, oppositeFaces=9,
                  selectedClearanceMm=distance, analyticDesignClearanceMm=3.,
                  nativeIntervalMm=[native['lowerMm'], native['upperMm']],
                  oracleComparisonToleranceMm=tolerance,
                  automaticSelectionComplete=automatic_selection, wholeWallClaim=False,
                  passed=passed)
    (directory / 'wall-clearance-occt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
